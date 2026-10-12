//! SVG geometry import.
//!
//! The browser sends SVG text. Python converts a bounded subset into a draft
//! native project. This process checks that project with `parse_project` and
//! returns it with the source manifest. It does not generate stitches, encode
//! DST, or record a review.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use embroidery_core::{parse_project, sha256_hex, ProjectDocument};
use serde_json::{json, Value};

use crate::{json_error, json_ok, repo_root, HttpResponse};

const SVG_TEXT_LIMIT: usize = 400_000;
const NAME_LIMIT: usize = 80;

pub fn response(body: &[u8]) -> HttpResponse {
    let value: Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return json_error(
                400,
                "The SVG request is not JSON. Choose an SVG file and preview again. The design was not changed.",
            );
        }
    };
    let Some(svg_text) = value.get("svg_text").and_then(|item| item.as_str()) else {
        return json_error(
            400,
            "Choose an SVG file, then preview the import. The design was not changed.",
        );
    };
    if svg_text.trim().is_empty() {
        return json_error(
            400,
            "Choose an SVG file, then preview the import. The design was not changed.",
        );
    }
    if svg_text.len() > SVG_TEXT_LIMIT {
        return json_error(
            413,
            "This SVG is larger than 400000 bytes. Simplify it and preview again. The design was not changed.",
        );
    }
    let width_mm = match optional_width(&value) {
        Ok(width) => width,
        Err(message) => return json_error(400, &message),
    };
    let running_strokes = match optional_flag(&value, "running_strokes") {
        Ok(flag) => flag,
        Err(message) => return json_error(400, &message),
    };
    let name = match optional_name(&value) {
        Ok(name) => name,
        Err(message) => return json_error(400, &message),
    };
    let tolerance = match optional_tolerance(&value) {
        Ok(tolerance) => tolerance,
        Err(message) => return json_error(400, &message),
    };
    let mut payload = json!({
        "svg_text": svg_text,
        "running_strokes": running_strokes,
    });
    if let Some(width_mm) = width_mm {
        payload["width_mm"] = json!(width_mm);
    }
    if let Some(name) = name {
        payload["name"] = json!(name);
    }
    if let Some(tolerance) = tolerance {
        payload["flatten_tolerance_mm"] = json!(tolerance);
    }
    let answered = match run_converter(&payload) {
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
            .unwrap_or("The SVG was not accepted. Choose another file and preview again. The design was not changed.");
        return json_error(422, &public_message(message));
    }
    let Some(project_value) = answered.get("project") else {
        return json_error(
            422,
            "The SVG import did not include a design. Preview the file again. The design was not changed.",
        );
    };
    let project_json = match serde_json::to_string(project_value) {
        Ok(text) => text,
        Err(_) => {
            return json_error(
                422,
                "The SVG import could not be read. Preview the file again. The design was not changed.",
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
    let mut manifest = match answered.get("manifest").cloned() {
        Some(Value::Object(manifest)) => Value::Object(manifest),
        _ => {
            return json_error(
                422,
                "The SVG import did not include a source manifest. Preview the file again. The design was not changed.",
            );
        }
    };
    let expected_sha = sha256_hex(svg_text.as_bytes());
    let actual_sha = manifest
        .get("source_sha256")
        .and_then(|item| item.as_str())
        .unwrap_or("");
    if actual_sha != expected_sha {
        return json_error(
            422,
            "The source hash does not match the SVG that was sent. Preview the file again. The design was not changed.",
        );
    }
    if let Some(object) = manifest.as_object_mut() {
        object.insert("machine_release_allowed".to_string(), Value::Bool(false));
        object.insert("approval_allowed".to_string(), Value::Bool(false));
        object.insert("review".to_string(), Value::Null);
        object.insert(
            "not_interchangeable_with".to_string(),
            json!("twinlab-generic-flat-120-v1"),
        );
        object.insert("timing_model".to_string(), Value::Null);
    }
    let losses = match string_list(answered.get("losses")) {
        Ok(losses) => losses,
        Err(message) => return json_error(422, &message),
    };
    json_ok(
        accepted(&project, manifest, &losses)
            .to_string()
            .into_bytes(),
    )
}

fn accepted(project: &ProjectDocument, manifest: Value, losses: &[String]) -> Value {
    let curved = losses
        .iter()
        .any(|loss| loss.contains("Curves were flattened"));
    let curves = if curved {
        "Curves were flattened to polygons."
    } else {
        "No curves were flattened."
    };
    let noun = if project.objects.len() == 1 {
        "object"
    } else {
        "objects"
    };
    json!({
        "ok": true,
        "project": project,
        "manifest": manifest,
        "losses": losses,
        "summary": format!(
            "{} {} in a {} × {} mm hoop. {} No stitches were generated. Apply the geometry, then generate stitches. This import does not approve itself.",
            project.objects.len(),
            noun,
            mm_label(project.hoop_width_mm),
            mm_label(project.hoop_height_mm),
            curves,
        ),
    })
}

fn mm_label(value: f64) -> String {
    if (value - value.round()).abs() < 1e-6 {
        format!("{:.0}", value.round())
    } else {
        format!("{value:.2}")
    }
}

fn optional_width(value: &Value) -> Result<Option<f64>, String> {
    match value.get("width_mm") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => {
            let number = number.as_f64().ok_or_else(|| {
                "Artboard width must be a positive number of millimetres up to 10000. Enter the physical width and preview again. The design was not changed."
                    .to_string()
            })?;
            if !number.is_finite() || number <= 0.0 || number > 10_000.0 {
                return Err(
                    "Artboard width must be a positive number of millimetres up to 10000. Enter the physical width and preview again. The design was not changed."
                        .to_string(),
                );
            }
            Ok(Some(number))
        }
        Some(_) => Err(
            "Artboard width must be a positive number of millimetres up to 10000. Enter the physical width and preview again. The design was not changed."
                .to_string(),
        ),
    }
}

fn optional_tolerance(value: &Value) -> Result<Option<f64>, String> {
    match value.get("flatten_tolerance_mm") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => {
            let number = number.as_f64().ok_or_else(|| {
                "The flattening tolerance must be from 0.01 mm to 1 mm. Choose a tolerance in that range and preview again. The design was not changed."
                    .to_string()
            })?;
            if !number.is_finite() || !(0.01..=1.0).contains(&number) {
                return Err(
                    "The flattening tolerance must be from 0.01 mm to 1 mm. Choose a tolerance in that range and preview again. The design was not changed."
                        .to_string(),
                );
            }
            Ok(Some(number))
        }
        Some(_) => Err(
            "The flattening tolerance must be from 0.01 mm to 1 mm. Choose a tolerance in that range and preview again. The design was not changed."
                .to_string(),
        ),
    }
}

fn optional_flag(value: &Value, name: &str) -> Result<bool, String> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(_) => Err(
            "Stroke interpretation must be on or off. Preview the SVG again. The design was not changed."
                .to_string(),
        ),
    }
}

fn optional_name(value: &Value) -> Result<Option<String>, String> {
    match value.get("name") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) => {
            let cleaned = name.split_whitespace().collect::<Vec<_>>().join(" ");
            if cleaned.is_empty() {
                return Ok(None);
            }
            if cleaned.chars().count() > NAME_LIMIT
                || cleaned.chars().any(|symbol| symbol.is_ascii() && (symbol as u32) < 32)
            {
                return Err(
                    "The project name must be visible text up to 80 characters. Rename it and preview again. The design was not changed."
                        .to_string(),
                );
            }
            Ok(Some(cleaned))
        }
        Some(_) => Err(
            "The project name must be visible text up to 80 characters. Rename it and preview again. The design was not changed."
                .to_string(),
        ),
    }
}

fn string_list(value: Option<&Value>) -> Result<Vec<String>, String> {
    let Some(items) = value.and_then(|item| item.as_array()) else {
        return Err(
            "The SVG import did not list its losses. Preview the file again. The design was not changed."
                .to_string(),
        );
    };
    if items.is_empty() {
        return Err(
            "The SVG import did not list its losses. Preview the file again. The design was not changed."
                .to_string(),
        );
    }
    let mut losses = Vec::with_capacity(items.len());
    for item in items {
        let Some(text) = item.as_str() else {
            return Err(
                "The SVG import did not list its losses. Preview the file again. The design was not changed."
                    .to_string(),
            );
        };
        if text.chars().count() > 500 {
            return Err(
                "The SVG import reported a loss that is too long to show. Preview a simpler file. The design was not changed."
                    .to_string(),
            );
        }
        losses.push(text.to_string());
    }
    Ok(losses)
}

fn run_converter(payload: &Value) -> Result<Value, String> {
    let python = python_bin()?;
    let bytes = serde_json::to_vec(payload).map_err(|_| {
        "The SVG request could not be prepared. Preview the file again. The design was not changed."
            .to_string()
    })?;
    let mut child = Command::new(&python)
        .arg("-m")
        .arg("embroidery_ai.svg_geometry")
        .current_dir(repo_root())
        .env("PYTHONPATH", repo_root().join("python"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| {
            "The SVG helper could not start. Restore the project Python environment and preview again. The design was not changed."
                .to_string()
        })?;
    {
        let mut stdin = child.stdin.take().ok_or_else(|| {
            "The SVG helper could not take the file. Preview it again. The design was not changed."
                .to_string()
        })?;
        stdin.write_all(&bytes).map_err(|_| {
            "The SVG helper did not accept the file. Preview it again. The design was not changed."
                .to_string()
        })?;
    }
    let stdout = child.stdout.take().ok_or_else(|| {
        "The SVG helper produced no answer. Preview the file again. The design was not changed."
            .to_string()
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        "The SVG helper produced no answer. Preview the file again. The design was not changed."
            .to_string()
    })?;
    let out_thread = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout.take(2_000_000).read_to_end(&mut buffer);
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
            Ok(None) if started.elapsed() > Duration::from_secs(15) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out_thread.join();
                let _ = err_thread.join();
                return Err(
                    "The SVG import took longer than 15 seconds. Try a simpler file. The design was not changed."
                        .to_string(),
                );
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(
                    "The SVG helper stopped unexpectedly. Preview the file again. The design was not changed."
                        .to_string(),
                );
            }
        }
    }
    let out = out_thread.join().unwrap_or_default();
    let _err = err_thread.join().unwrap_or_default();
    serde_json::from_slice(&out).map_err(|_| {
        "The SVG helper returned an unreadable answer. Preview the file again. The design was not changed."
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
            "EMBROIDERY_PYTHON is not a Python program. Point it at the project .venv and preview the SVG again. The design was not changed."
                .to_string(),
        );
    }
    let path = repo_root().join(".venv/bin/python");
    if path.is_file() {
        Ok(path)
    } else {
        Err(
            "The project Python environment is missing. Restore .venv and preview the SVG again. The design was not changed."
                .to_string(),
        )
    }
}

fn public_message(message: &str) -> String {
    let key = std::env::var("XAI_API_KEY").unwrap_or_default();
    if message.trim().is_empty()
        || message.chars().count() > 500
        || (!key.is_empty() && message.contains(&key))
    {
        return "The SVG was not accepted. Choose another file and preview again. The design was not changed."
            .to_string();
    }
    message.to_string()
}
