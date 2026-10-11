//! Geometry proposal route.
//!
//! The browser sends a prompt and an optional image. This process asks the
//! model for region geometry and checks it with `parse_project`. It does not
//! generate stitches, encode DST, or record a review.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use embroidery_core::{parse_project, ProjectDocument};
use serde_json::{json, Value};

use crate::{json_error, json_ok, repo_root, HttpResponse};

const PROMPT_LIMIT: usize = 2000;
const IMAGE_LIMIT: usize = 900_000;

pub fn response(body: &[u8]) -> HttpResponse {
    let value: Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return json_error(
                400,
                "The proposal body is not JSON. Send a prompt or an image and ask again. The design was not changed.",
            );
        }
    };
    let prompt = value
        .get("prompt")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let image = value
        .get("image_data_url")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if prompt.is_empty() && image.is_empty() {
        return json_error(
            400,
            "Enter a prompt or attach an image, then ask again. The design was not changed.",
        );
    }
    if prompt.chars().count() > PROMPT_LIMIT {
        return json_error(
            400,
            "The prompt is longer than 2000 characters. Shorten it and ask again. The design was not changed.",
        );
    }
    if !image.is_empty() && !allowed_image(&image) {
        return json_error(
            400,
            "Attach a JPEG or PNG image. Other file types are not sent. The design was not changed.",
        );
    }
    if std::env::var("XAI_API_KEY")
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return json_error(
            422,
            "The studio has no XAI_API_KEY. Set that key for the studio process and ask again. The design was not changed.",
        );
    }
    let payload = json!({
        "prompt": prompt,
        "image_data_url": image,
    });
    let answered = match run_proposer(&payload) {
        Ok(answered) => answered,
        Err(message) => return json_error(422, &public_message(&message)),
    };
    let ok = answered
        .get("ok")
        .and_then(|item| item.as_bool())
        .unwrap_or(false);
    if !ok {
        let message = answered
            .get("error")
            .and_then(|item| item.as_str())
            .unwrap_or("The proposal was not accepted. Ask again. The design was not changed.");
        return json_error(422, &public_message(message));
    }
    let Some(project_value) = answered.get("project") else {
        return json_error(
            422,
            "The proposal did not include a design. Ask again. The design was not changed.",
        );
    };
    let project_json = match serde_json::to_string(project_value) {
        Ok(text) => text,
        Err(_) => {
            return json_error(
                422,
                "The proposal could not be read. Ask again. The design was not changed.",
            );
        }
    };
    let mut project = match parse_project(&project_json) {
        Ok(project) => project,
        Err(error) => {
            let message = format!("{} The design was not changed.", error.message());
            return json_error(422, &public_message(&message));
        }
    };
    project.review = None;
    json_ok(accepted(&project).to_string().into_bytes())
}

fn accepted(project: &ProjectDocument) -> Value {
    let objects: Vec<Value> = project
        .objects
        .iter()
        .map(|object| {
            json!({
                "label": object.label,
                "kind": object.kind,
                "points": object.points.len(),
                "color_index": object.color_index,
            })
        })
        .collect();
    let noun = if project.objects.len() == 1 {
        "object"
    } else {
        "objects"
    };
    json!({
        "ok": true,
        "project": project,
        "summary": format!(
            "{} {} in a {:.0} × {:.0} mm hoop. No stitches were generated. Apply the geometry, then generate stitches. This proposal does not approve itself.",
            project.objects.len(),
            noun,
            project.hoop_width_mm,
            project.hoop_height_mm,
        ),
        "objects": objects,
        "note": "This is a geometry proposal. It is not a stitch plan and it is not a machine release.",
    })
}

fn run_proposer(payload: &Value) -> Result<Value, String> {
    let python = python_bin()?;
    let bytes = serde_json::to_vec(payload).map_err(|_| {
        "The proposal could not be prepared. Ask again. The design was not changed.".to_string()
    })?;
    let mut child = Command::new(&python)
        .arg("-m")
        .arg("embroidery_ai.propose")
        .current_dir(repo_root())
        .env("PYTHONPATH", repo_root().join("python"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| {
            "The proposal helper could not start. Restore the project Python environment and ask again. The design was not changed."
                .to_string()
        })?;
    {
        let mut stdin = child.stdin.take().ok_or_else(|| {
            "The proposal helper could not take the request. Ask again. The design was not changed."
                .to_string()
        })?;
        stdin.write_all(&bytes).map_err(|_| {
            "The proposal helper did not accept the request. Ask again. The design was not changed."
                .to_string()
        })?;
    }
    let stdout = child.stdout.take().ok_or_else(|| {
        "The proposal helper produced no answer. Ask again. The design was not changed.".to_string()
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        "The proposal helper produced no answer. Ask again. The design was not changed.".to_string()
    })?;
    let out_thread = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout.take(300_000).read_to_end(&mut buffer);
        buffer
    });
    let err_thread = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stderr.take(16_384).read_to_end(&mut buffer);
        buffer
    });
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() > Duration::from_secs(90) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out_thread.join();
                let _ = err_thread.join();
                return Err(
                    "The proposal took longer than 90 seconds. Shorten the prompt and ask again. The design was not changed."
                        .to_string(),
                );
            }
            Ok(None) => thread::sleep(Duration::from_millis(40)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(
                    "The proposal helper stopped unexpectedly. Ask again. The design was not changed."
                        .to_string(),
                );
            }
        }
    }
    let out = out_thread.join().unwrap_or_default();
    let _err = err_thread.join().unwrap_or_default();
    serde_json::from_slice(&out).map_err(|_| {
        "The proposal helper returned an unreadable answer. Ask again. The design was not changed."
            .to_string()
    })
}

fn python_bin() -> Result<PathBuf, String> {
    if let Ok(value) = std::env::var("EMBROIDERY_PYTHON") {
        let path = PathBuf::from(value.trim());
        if path.is_file() {
            return Ok(path);
        }
        return Err(
            "EMBROIDERY_PYTHON is not a Python program. Point it at the project .venv and ask again. The design was not changed."
                .to_string(),
        );
    }
    let path = repo_root().join(".venv/bin/python");
    if path.is_file() {
        Ok(path)
    } else {
        Err(
            "The project Python environment is missing. Restore .venv and ask again. The design was not changed."
                .to_string(),
        )
    }
}

fn allowed_image(value: &str) -> bool {
    let data = [
        "data:image/jpeg;base64,",
        "data:image/jpg;base64,",
        "data:image/png;base64,",
    ]
    .iter()
    .find_map(|prefix| value.strip_prefix(prefix));
    let Some(data) = data else {
        return false;
    };
    !data.is_empty()
        && data.len() <= IMAGE_LIMIT
        && data
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
}

fn public_message(message: &str) -> String {
    let key = std::env::var("XAI_API_KEY").unwrap_or_default();
    if message.trim().is_empty()
        || message.chars().count() > 500
        || (!key.is_empty() && message.contains(&key))
    {
        return "The proposal was not accepted. Ask again. The design was not changed.".to_string();
    }
    message.to_string()
}
