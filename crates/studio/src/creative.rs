//! Creative image jobs.
//!
//! One Creative Director builds a brief and calls one configured image
//! operation. The result is a raster artifact. It is not embroidery geometry,
//! and saving it does not change the open embroidery project.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use embroidery_core::sha256_hex;
use serde_json::{json, Value};

use crate::{json_error, json_ok, repo_root, HttpResponse};

const IMAGE_MODEL: &str = "grok-imagine-image-2.0";
const OPENROUTER_MODEL: &str = "x-ai/grok-imagine-image-2.0";
const GEOMETRY_MODEL: &str = "grok-4.7";
const GENERATE_ENDPOINT: &str = "https://api.x.ai/v1/images/generations";
const OPENROUTER_ENDPOINT: &str = "https://openrouter.ai/api/v1/images";
const MAX_STORED: usize = 8_388_608;
const MAX_REFERENCE: usize = 700_000;

struct Options {
    credential_present: bool,
    openrouter_credential_present: bool,
    network_enabled: bool,
    python: PathBuf,
    provider_env: Option<String>,
    model_env: Option<String>,
}

struct Inner {
    root: PathBuf,
    options: Options,
    jobs: HashMap<String, Value>,
    clients: HashMap<String, String>,
    provider_calls: u32,
}

#[derive(Clone)]
struct Service {
    inner: Arc<Mutex<Inner>>,
}

enum Stage {
    Error(Value),
    Finished(Value),
    Ready(String),
}

enum Outcome {
    Image(Vec<u8>),
    Failed(String),
    Uncertain(String),
    Stopped,
}

static GLOBAL: Mutex<Option<Service>> = Mutex::new(None);

pub fn dispatch(method: &str, path: &str, body: &[u8]) -> Option<HttpResponse> {
    if path != "/creative.js" && !path.starts_with("/api/creative") {
        return None;
    }
    if method == "GET" && path == "/creative.js" {
        return None;
    }
    Some(api(method, path, body))
}

fn api(method: &str, path: &str, body: &[u8]) -> HttpResponse {
    let service = match global_service() {
        Ok(service) => service,
        Err(message) => return json_error(500, &message),
    };
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let response = match (method, parts.as_slice()) {
        ("GET", ["api", "creative", "policy"]) => service.policy_view(),
        ("POST", ["api", "creative", "jobs"]) => service.submit_async(body),
        ("GET", ["api", "creative", "jobs", id]) => service.job_view(id),
        ("POST", ["api", "creative", "jobs", id, "cancel"]) => service.cancel(id),
        ("POST", ["api", "creative", "jobs", id, "select"]) => service.select(id, body),
        ("POST", ["api", "creative", "masters", "apply"]) => service.apply(body),
        ("GET", ["api", "creative", "artifacts", id]) => return service.artifact_bytes(id),
        ("GET", ["api", "creative", "artifacts", id, "meta"]) => service.artifact_meta(id),
        _ => json!({
            "ok": false,
            "error": "That creative route does not exist. Use Creative Director in the design studio."
        }),
    };
    let status = if response.get("ok").and_then(|item| item.as_bool()) == Some(false)
        && response.get("error").is_some()
        && response.get("execution").and_then(|item| item.as_str()) != Some("submitted")
        && response.get("execution").and_then(|item| item.as_str()) != Some("queued")
    {
        if response.get("code").and_then(|item| item.as_str()) == Some("not_found") {
            404
        } else if response.get("job_id").is_some() || response.get("execution").is_some() {
            422
        } else {
            400
        }
    } else {
        200
    };
    if status == 200 {
        json_ok(response.to_string().into_bytes())
    } else {
        HttpResponse {
            status,
            content_type: "application/json; charset=utf-8",
            body: response.to_string().into_bytes(),
        }
    }
}

fn global_service() -> Result<Service, String> {
    let mut slot = GLOBAL.lock().unwrap_or_else(|poison| poison.into_inner());
    if slot.is_none() {
        let path = std::env::var("EMBROIDERY_CREATIVE_STORE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| repo_root().join("var/creative"));
        *slot = Some(Service::open(&path, options_from_env())?);
    }
    Ok(slot.as_ref().unwrap().clone())
}

fn env_text(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn options_from_env() -> Options {
    Options {
        credential_present: env_text("XAI_API_KEY").is_some(),
        openrouter_credential_present: env_text("OPENROUTER_API_KEY").is_some(),
        network_enabled: std::env::var("EMBROIDERY_CREATIVE_LIVE")
            .ok()
            .map(|value| value.trim() == "1")
            .unwrap_or(false),
        python: python_bin(),
        provider_env: env_text("EMBROIDERY_CREATIVE_PROVIDER"),
        model_env: env_text("EMBROIDERY_CREATIVE_MODEL"),
    }
}

fn python_bin() -> PathBuf {
    if let Ok(value) = std::env::var("EMBROIDERY_PYTHON") {
        if !value.trim().is_empty() {
            return PathBuf::from(value);
        }
    }
    let venv = repo_root().join(".venv/bin/python");
    if venv.exists() {
        return venv;
    }
    PathBuf::from("python3")
}

impl Service {
    fn open(root: &Path, options: Options) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|_| {
            "The creative store could not be created. Choose a writable folder and start the studio again.".to_string()
        })?;
        let policy_path = root.join("policy.json");
        if !policy_path.exists() {
            write_json(&policy_path, &default_policy())?;
        }
        let service = Service {
            inner: Arc::new(Mutex::new(Inner {
                root: root.to_path_buf(),
                options,
                jobs: HashMap::new(),
                clients: HashMap::new(),
                provider_calls: 0,
            })),
        };
        service.lock().load()?;
        Ok(service)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    fn policy_view(&self) -> Value {
        let inner = self.lock();
        let policy = inner.policy();
        let provider = text(&policy, "provider");
        let model = text(&policy, "model");
        let remaining = policy.get("remaining_calls").and_then(|item| item.as_u64()).unwrap_or(0);
        let transmit = policy.get("transmit_permitted").and_then(|item| item.as_bool()).unwrap_or(false);
        let key_configured = provider_key_configured(&provider, &inner.options);
        let model_ok = (provider == "xai_images" && model == IMAGE_MODEL)
            || (provider == "openrouter_images" && model == OPENROUTER_MODEL);
        let env_ok = inner.options.provider_env.as_ref().map(|value| value == &provider).unwrap_or(true)
            && inner.options.model_env.as_ref().map(|value| value == &model).unwrap_or(true);
        let estimate_ok = provider != "openrouter_images" || estimated_output_cost_usd(&policy).is_some();
        let live_ready = (provider == "xai_images" || provider == "openrouter_images")
            && model_ok
            && text(&policy, "operation") == "generate"
            && transmit
            && remaining > 0
            && key_configured
            && inner.options.network_enabled
            && env_ok
            && estimate_ok
            && spend_ceiling_usd(&policy).is_some();
        json!({
            "ok": true,
            "schema": "embroideryos-creative-policy/0.1",
            "provider": provider,
            "model": model,
            "operation": text(&policy, "operation"),
            "endpoint": provider_endpoint(&provider),
            "route": provider_route(&provider),
            "supported_model": if provider == "openrouter_images" { OPENROUTER_MODEL } else { IMAGE_MODEL },
            "geometry_model_rejected": GEOMETRY_MODEL,
            "remaining_calls": remaining,
            "transmit_permitted": transmit,
            "spend_ceiling_usd": policy.get("spend_ceiling_usd").cloned().unwrap_or(json!(null)),
            "estimated_output_cost_usd": policy.get("estimated_output_cost_usd").cloned().unwrap_or(json!(null)),
            "direct_xai_authorisation": policy.get("direct_xai_authorisation").cloned().unwrap_or(json!(null)),
            "authorisation_id": policy.get("authorisation_id").cloned().unwrap_or(json!(null)),
            "key_configured": key_configured,
            "live_process_enabled": inner.options.network_enabled,
            "live_switch": "EMBROIDERY_CREATIVE_LIVE=1",
            "live_ready": live_ready,
            "local_test": provider == "test_image",
            "test_fallback_enabled": provider == "test_image",
            "allow_provider_fallbacks": false,
            "machine_release_allowed": false,
            "note": "A key alone does not send a prompt. Live generation needs a remaining call allowance, transmit permission, a positive spend_ceiling_usd, and EMBROIDERY_CREATIVE_LIVE set to exactly 1. OpenRouter uses OPENROUTER_API_KEY and does not read XAI_API_KEY. Image editing is a separate operation. Startup does not submit a job.",
        })
    }

    fn submit_async(&self, body: &[u8]) -> Value {
        match self.prepare(body) {
            Stage::Error(value) | Stage::Finished(value) => value,
            Stage::Ready(id) => {
                let service = self.clone();
                let job_id = id.clone();
                thread::spawn(move || service.execute(&job_id));
                self.public_job(&id)
            }
        }
    }

    #[cfg(test)]
    fn submit_sync(&self, body: &[u8]) -> Value {
        match self.prepare(body) {
            Stage::Error(value) | Stage::Finished(value) => value,
            Stage::Ready(id) => {
                self.execute(&id);
                self.public_job(&id)
            }
        }
    }

    fn prepare(&self, body: &[u8]) -> Stage {
        let value: Value = match serde_json::from_slice(body) {
            Ok(value) => value,
            Err(_) => {
                return Stage::Error(json!({
                    "ok": false,
                    "code": "invalid_request",
                    "error": "The creative request is not JSON. Send the brief again. Nothing was generated.",
                }));
            }
        };
        if value.as_object().is_none() {
            return Stage::Error(json!({
                "ok": false,
                "code": "invalid_request",
                "error": "The creative request must be an object. Nothing was generated.",
            }));
        }
        for key in [
            "project",
            "items",
            "name_drop",
            "batch",
            "template",
            "dst",
            "dst_hex",
            "proof",
            "customers",
            "rows",
        ] {
            if value.get(key).is_some() {
                return Stage::Error(json!({
                    "ok": false,
                    "code": "private_material",
                    "error": "The creative request included an embroidery project or a name-drop table. That material was not sent. Send only the artwork brief and one chosen reference.",
                }));
            }
        }
        let mut inner = self.lock();
        let client = match token(text(&value, "client_request_id"), 8, 80) {
            Ok(token) => token,
            Err(_) => {
                return Stage::Error(json!({
                    "ok": false,
                    "code": "invalid_request",
                    "error": "Add a client request id so a repeated submit is not sent twice. Nothing was generated.",
                }));
            }
        };
        if let Some(existing) = inner.clients.get(&client) {
            let mut view = inner.public(existing);
            view["duplicate"] = json!(true);
            view["ok"] = json!(false);
            view["error"] = json!("This request id was already submitted. The provider was not called again. Submit a new job only if another attempt is intended.");
            return Stage::Finished(view);
        }
        let request = text(&value, "request").trim().to_string();
        if request.is_empty() || request.chars().count() > 2000 {
            return Stage::Error(json!({
                "ok": false,
                "code": "invalid_request",
                "error": "Enter an artwork description of at most 2000 characters. Nothing was generated.",
            }));
        }
        let revision = match token(text(&value, "source_revision"), 1, 200) {
            Ok(token) => token,
            Err(_) => {
                return Stage::Error(json!({
                    "ok": false,
                    "code": "invalid_request",
                    "error": "The artwork needs the current embroidery revision token. Nothing was generated and the open design was not changed.",
                }));
            }
        };
        let profile = text(&value, "art_direction");
        let Some(instruction) = profile_instruction(&profile) else {
            return Stage::Error(json!({
                "ok": false,
                "code": "invalid_request",
                "error": "Choose an art-direction profile. Nothing was generated.",
            }));
        };
        let width = value.get("width").and_then(|item| item.as_u64()).unwrap_or(0);
        let height = value.get("height").and_then(|item| item.as_u64()).unwrap_or(0);
        if !(16..=2048).contains(&width) || !(16..=2048).contains(&height) {
            return Stage::Error(json!({
                "ok": false,
                "code": "invalid_request",
                "error": "Choose a width and height from 16 to 2048 pixels. Nothing was generated.",
            }));
        }
        let references = match reference_bytes(&value) {
            Ok(bytes) => bytes,
            Err(message) => {
                return Stage::Error(json!({
                    "ok": false,
                    "code": "invalid_request",
                    "error": message,
                }));
            }
        };
        let mut reference_hashes = Vec::new();
        for bytes in &references {
            let hash = sha256_hex(bytes);
            if let Err(message) = store_bytes(&inner.root, &hash, bytes) {
                return Stage::Error(json!({"ok": false, "code": "store", "error": message}));
            }
            let artifact_id = fresh_id("art", &hash);
            let record = json!({
                "schema": "embroideryos-creative-artifact/0.1",
                "artifact_id": artifact_id,
                "role": "reference_input",
                "media_type": "application/octet-stream",
                "representation": "reference",
                "sha256": hash,
                "byte_length": bytes.len(),
                "immutable": true,
                "editable_vector": false,
                "layered_art": false,
                "dimensional_cad": false,
                "embroidery_geometry": false,
            });
            if write_json(&inner.root.join("artifacts").join(format!("{artifact_id}.json")), &record).is_err() {
                return Stage::Error(json!({
                    "ok": false,
                    "code": "store",
                    "error": "The reference could not be stored. Nothing was sent.",
                }));
            }
            reference_hashes.push(hash);
        }
        let policy = inner.policy();
        let provider = text(&policy, "provider");
        let model = text(&policy, "model");
        let operation = text(&policy, "operation");
        let script = if provider == "test_image" {
            let name = text(&policy, "test_script");
            if name.is_empty() { "png".to_string() } else { name }
        } else {
            String::new()
        };
        let live = provider == "xai_images" || provider == "openrouter_images";
        let prompt = provider_prompt(instruction, &request, width, height);
        let job_id = fresh_id("job", &format!("{client}:{request}"));
        let mut job = json!({
            "schema": "embroideryos-creative-job/0.1",
            "job_id": job_id,
            "client_request_id": client,
            "source_revision": revision,
            "exact_request": request,
            "brief": {
                "art_direction": profile,
                "profile_instruction": instruction,
                "provider_prompt": prompt,
                "width": width,
                "height": height,
                "reference_hashes": reference_hashes,
                "preserves_subject": true,
                "embroidery_geometry": false,
                "agents": ["creative-director"],
            },
            "provider": {
                "id": provider,
                "model": model,
                "operation": operation,
                "endpoint": if live { provider_endpoint(&provider) } else { "" },
                "route": provider_route(&provider),
                "allow_fallbacks": false,
                "authorisation_id": policy.get("authorisation_id").cloned().unwrap_or(json!(null)),
                "live": live,
                "script": script,
            },
            "transmit_requested": value.get("transmit").and_then(|item| item.as_bool()).unwrap_or(false),
            "execution": "queued",
            "application": "none",
            "cancel_requested": false,
            "provider_stop_confirmed": false,
            "billable": false,
            "billable_uncertain": false,
            "attempts": 0,
            "artifact_ids": [],
            "stale_artifact_ids": [],
            "error": null,
            "code": null,
            "prompt_transmitted": false,
            "provider_request_id": null,
            "provider_usage": null,
            "machine_release_allowed": false,
            "embroidery_project_changed": false,
        });
        if let Some(message) = gate_failure(&inner, &job, &policy) {
            job["execution"] = json!("failed");
            job["code"] = json!(message.0);
            job["error"] = json!(message.1);
            inner.insert_job(job);
            return Stage::Finished(inner.public(&job_id));
        }
        inner.insert_job(job);
        Stage::Ready(job_id)
    }

    fn execute(&self, job_id: &str) {
        let call = {
            let mut inner = self.lock();
            if !inner.begin(job_id) {
                return;
            }
            inner.provider_calls += 1;
            inner.call_spec(job_id)
        };
        let Some(call) = call else {
            return;
        };
        let (outcome, receipt) = produce(&call);
        let outcome = match outcome {
            Outcome::Image(bytes) => match inspect_image(&call.python, &bytes) {
                Ok(info) => {
                    if info.width < 1 || info.height < 1 || info.width > 4096 || info.height > 4096 {
                        Outcome::Failed(
                            "The image dimensions are outside the permitted range. It was not stored and no fixture was substituted.".to_string(),
                        )
                    } else if bytes.len() > MAX_STORED {
                        Outcome::Failed(
                            "The image is larger than 8 MiB. It was not stored and no fixture was substituted.".to_string(),
                        )
                    } else {
                        Outcome::Image(bytes)
                    }
                }
                Err(_) => Outcome::Failed(
                    "The provider returned data that is not a usable PNG or JPEG. It was not stored as artwork and it was not replaced with a fixture. Submit a new job if another attempt is acceptable.".to_string(),
                ),
            },
            other => other,
        };
        let info = if let Outcome::Image(bytes) = &outcome {
            inspect_image(&call.python, bytes).ok()
        } else {
            None
        };
        if !receipt.is_null() {
            let mut inner = self.lock();
            if let Some(job) = inner.jobs.get_mut(job_id) {
                job["provider_request_id"] = receipt.get("request_id").cloned().unwrap_or(json!(null));
                job["provider_usage"] = receipt.get("usage").cloned().unwrap_or(json!(null));
            }
            inner.persist(job_id);
        }
        self.lock().deliver(job_id, outcome, info, false);
    }

    fn public_job(&self, id: &str) -> Value {
        self.lock().public(id)
    }

    fn job_view(&self, id: &str) -> Value {
        let inner = self.lock();
        if !valid_id(id) || !inner.jobs.contains_key(id) {
            return json!({
                "ok": false,
                "code": "not_found",
                "error": "That creative job is not in the local store. Generate the artwork again.",
            });
        }
        inner.public(id)
    }

    fn cancel(&self, id: &str) -> Value {
        let mut inner = self.lock();
        if !valid_id(id) || !inner.jobs.contains_key(id) {
            return json!({
                "ok": false,
                "code": "not_found",
                "error": "That creative job is not in the local store. Nothing was cancelled.",
            });
        }
        let execution = text(inner.jobs.get(id).unwrap(), "execution");
        if execution == "queued" {
            let job = inner.jobs.get_mut(id).unwrap();
            job["execution"] = json!("cancelled");
            job["cancel_requested"] = json!(true);
            job["provider_stop_confirmed"] = json!(true);
            job["code"] = json!("cancelled");
            job["error"] = json!("The job was cancelled before it was submitted. The provider was not called.");
            inner.persist(id);
        } else if execution == "submitted" {
            let job = inner.jobs.get_mut(id).unwrap();
            job["cancel_requested"] = json!(true);
            job["error"] = json!("Cancellation was recorded after submission. The provider has not been confirmed to have stopped.");
            inner.persist(id);
        }
        let mut view = inner.public(id);
        view["ok"] = json!(false);
        view
    }

    fn select(&self, id: &str, body: &[u8]) -> Value {
        let value: Value = match serde_json::from_slice(body) {
            Ok(value) => value,
            Err(_) => {
                return json!({
                    "ok": false,
                    "code": "invalid_request",
                    "error": "The selection is not JSON. The artwork was not saved and the open design was not changed.",
                });
            }
        };
        let revision = text(&value, "current_revision");
        let mut inner = self.lock();
        if !valid_id(id) || !inner.jobs.contains_key(id) {
            return json!({
                "ok": false,
                "code": "not_found",
                "error": "That creative job is not in the local store. The open design was not changed.",
            });
        }
        let job = inner.jobs.get(id).unwrap().clone();
        if text(&job, "execution") != "succeeded" || text(&job, "application") == "none" {
            return json!({
                "ok": false,
                "code": "not_candidate",
                "job_id": id,
                "error": "Only a successful candidate can be saved as an artwork master. The open design was not changed.",
            });
        }
        if text(&job, "source_revision") != revision {
            return json!({
                "ok": false,
                "code": "revision_mismatch",
                "job_id": id,
                "error": "The embroidery revision changed after this artwork was requested. The candidate was not saved. The open design was not changed.",
            });
        }
        if text(&job, "application") == "candidate" {
            let artifact_id = job["artifact_ids"][0].as_str().unwrap_or("").to_string();
            let meta = read_json(&inner.root.join("artifacts").join(format!("{artifact_id}.json"))).unwrap_or(json!({}));
            let master_id = fresh_id("mas", &artifact_id);
            let master = json!({
                "schema": "embroideryos-artwork-master/0.1",
                "master_id": master_id,
                "artifact_id": artifact_id,
                "sha256": meta.get("sha256").cloned().unwrap_or(json!(null)),
                "job_id": id,
                "source_revision": revision,
                "application": "selected",
                "embroidery_project_changed": false,
                "machine_release_allowed": false,
            });
            if write_json(&inner.root.join("masters").join(format!("{master_id}.json")), &master).is_err()
                || write_json(&inner.root.join("masters").join("current.json"), &master).is_err()
            {
                return json!({
                    "ok": false,
                    "code": "store",
                    "error": "The artwork master could not be saved. The open design was not changed.",
                });
            }
            let job = inner.jobs.get_mut(id).unwrap();
            job["application"] = json!("selected");
            job["master_id"] = json!(master_id);
            inner.persist(id);
        }
        let mut view = inner.public(id);
        view["ok"] = json!(true);
        view["error"] = json!(null);
        view["note"] = json!("Artwork master saved. The open embroidery design was not changed. Font pins and proofs were not changed. Prepare for embroidery is a separate action.");
        view
    }

    fn apply(&self, body: &[u8]) -> Value {
        let value: Value = match serde_json::from_slice(body) {
            Ok(value) => value,
            Err(_) => {
                return json!({
                    "ok": false,
                    "code": "invalid_request",
                    "error": "The apply request is not JSON. The open design was not changed.",
                });
            }
        };
        let master_id = text(&value, "master_id");
        let revision = text(&value, "current_revision");
        let mut inner = self.lock();
        let path = inner.root.join("masters").join(format!("{master_id}.json"));
        let Some(mut master) = read_json(&path) else {
            return json!({
                "ok": false,
                "code": "not_found",
                "error": "That artwork master is not in the local store. The open design was not changed.",
            });
        };
        if text(&master, "source_revision") != revision {
            return json!({
                "ok": false,
                "code": "revision_mismatch",
                "error": "The embroidery revision changed after this master was saved. It was not applied. The open design was not changed.",
            });
        }
        let job_id = text(&master, "job_id");
        let Some(job) = inner.jobs.get(&job_id) else {
            return json!({
                "ok": false,
                "code": "not_found",
                "error": "The job for that master is missing. It was not applied. The open design was not changed.",
            });
        };
        if text(job, "application") != "selected" && text(job, "application") != "applied" {
            return json!({
                "ok": false,
                "code": "not_candidate",
                "error": "Select the artwork master before applying it. The open design was not changed.",
            });
        }
        master["application"] = json!("applied");
        master["embroidery_project_changed"] = json!(false);
        master["machine_release_allowed"] = json!(false);
        if write_json(&path, &master).is_err()
            || write_json(&inner.root.join("masters").join("current.json"), &master).is_err()
        {
            return json!({
                "ok": false,
                "code": "store",
                "error": "The artwork master could not be updated. The open design was not changed.",
            });
        }
        if let Some(job) = inner.jobs.get_mut(&job_id) {
            job["application"] = json!("applied");
            inner.persist(&job_id);
        }
        json!({
            "ok": true,
            "master": master,
            "machine_release_allowed": false,
            "embroidery_project_changed": false,
            "note": "The creative project records this artwork master as applied. The open embroidery design, font pins, and proofs were not changed. Prepare for embroidery remains a separate action.",
        })
    }

    fn artifact_meta(&self, id: &str) -> Value {
        if !valid_id(id) {
            return json!({
                "ok": false,
                "code": "not_found",
                "error": "That artwork id is not valid. Reopen the job and choose the stored image.",
            });
        }
        let inner = self.lock();
        match read_json(&inner.root.join("artifacts").join(format!("{id}.json"))) {
            Some(value) => {
                let mut value = value;
                value["ok"] = json!(true);
                value
            }
            None => json!({
                "ok": false,
                "code": "not_found",
                "error": "That stored artwork could not be found. Generate or save it again.",
            }),
        }
    }

    fn artifact_bytes(&self, id: &str) -> HttpResponse {
        if !valid_id(id) {
            return json_error(404, "That artwork id is not valid. Reopen the job and download the stored image.");
        }
        let inner = self.lock();
        let Some(meta) = read_json(&inner.root.join("artifacts").join(format!("{id}.json"))) else {
            return json_error(404, "That stored artwork could not be found. Generate or save it again.");
        };
        let hash = text(&meta, "sha256");
        let path = inner.root.join("blobs").join(&hash);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                return json_error(404, "The stored artwork file is missing. It was not replaced with a fixture.");
            }
        };
        if sha256_hex(&bytes) != hash {
            return json_error(
                500,
                "The stored artwork no longer matches its recorded hash. It was not replaced.",
            );
        }
        let content_type = match text(&meta, "media_type").as_str() {
            "image/png" => "image/png",
            "image/jpeg" => "image/jpeg",
            _ => "application/octet-stream",
        };
        HttpResponse {
            status: 200,
            content_type,
            body: bytes,
        }
    }
}

impl Inner {
    fn policy(&self) -> Value {
        read_json(&self.root.join("policy.json")).unwrap_or_else(default_policy)
    }

    fn load(&mut self) -> Result<(), String> {
        let jobs = self.root.join("jobs");
        if !jobs.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&jobs).map_err(|_| "The creative jobs could not be read.".to_string())? {
            let entry = entry.map_err(|_| "The creative jobs could not be read.".to_string())?;
            let Some(mut job) = read_json(&entry.path()) else {
                continue;
            };
            let execution = text(&job, "execution");
            if execution == "queued" || execution == "submitted" {
                job["execution"] = json!("uncertain");
                job["billable_uncertain"] = json!(job.get("billable").and_then(|item| item.as_bool()).unwrap_or(false));
                job["code"] = json!("interrupted");
                job["error"] = json!("The studio stopped while this image job was in progress. It was not retried. A late result will not replace current artwork.");
                job["provider_stop_confirmed"] = json!(false);
            }
            let id = text(&job, "job_id");
            if id.is_empty() {
                continue;
            }
            write_json(&self.root.join("jobs").join(format!("{id}.json")), &job)?;
            let client = text(&job, "client_request_id");
            if !client.is_empty() {
                self.clients.insert(client, id.clone());
            }
            self.jobs.insert(id, job);
        }
        Ok(())
    }

    fn insert_job(&mut self, job: Value) {
        let id = text(&job, "job_id");
        let client = text(&job, "client_request_id");
        let _ = write_json(&self.root.join("jobs").join(format!("{id}.json")), &job);
        self.clients.insert(client, id.clone());
        self.jobs.insert(id, job);
    }

    fn begin(&mut self, id: &str) -> bool {
        let Some(job) = self.jobs.get(id) else {
            return false;
        };
        if text(job, "execution") != "queued" {
            return false;
        }
        let cancel = job.get("cancel_requested").and_then(|item| item.as_bool()) == Some(true);
        let live = job["provider"]["live"].as_bool().unwrap_or(false);
        if cancel {
            let job = self.jobs.get_mut(id).unwrap();
            job["execution"] = json!("cancelled");
            job["provider_stop_confirmed"] = json!(true);
            job["code"] = json!("cancelled");
            job["error"] = json!("The job was cancelled before it was submitted. The provider was not called.");
            let _ = write_json(&self.root.join("jobs").join(format!("{id}.json")), job);
            return false;
        }
        if live {
            let mut policy = self.policy();
            let remaining = policy.get("remaining_calls").and_then(|item| item.as_u64()).unwrap_or(0);
            if remaining == 0 {
                let job = self.jobs.get_mut(id).unwrap();
                job["execution"] = json!("failed");
                job["code"] = json!("budget_exhausted");
                job["error"] = json!("The creative image budget has no remaining calls. No image was requested. Raise the configured budget, then submit a new job.");
                let _ = write_json(&self.root.join("jobs").join(format!("{id}.json")), job);
                return false;
            }
            policy["remaining_calls"] = json!(remaining - 1);
            if write_json(&self.root.join("policy.json"), &policy).is_err() {
                let job = self.jobs.get_mut(id).unwrap();
                job["execution"] = json!("failed");
                job["code"] = json!("store");
                job["error"] = json!("The creative budget could not be recorded. No image was requested.");
                let _ = write_json(&self.root.join("jobs").join(format!("{id}.json")), job);
                return false;
            }
        }
        let job = self.jobs.get_mut(id).unwrap();
        if live {
            job["billable"] = json!(true);
        }
        job["execution"] = json!("submitted");
        job["attempts"] = json!(1);
        let _ = write_json(&self.root.join("jobs").join(format!("{id}.json")), job);
        true
    }

    fn call_spec(&self, id: &str) -> Option<Call> {
        let job = self.jobs.get(id)?;
        Some(Call {
            python: self.options.python.clone(),
            provider: text(&job["provider"], "id"),
            model: text(&job["provider"], "model"),
            operation: text(&job["provider"], "operation"),
            script: text(&job["provider"], "script"),
            prompt: text(&job["brief"], "provider_prompt"),
            width: job["brief"]["width"].as_u64().unwrap_or(64) as u32,
            height: job["brief"]["height"].as_u64().unwrap_or(64) as u32,
            network_enabled: self.options.network_enabled,
            credential_present: self.options.credential_present,
            openrouter_credential_present: self.options.openrouter_credential_present,
        })
    }

    fn deliver(&mut self, id: &str, outcome: Outcome, info: Option<ImageInfo>, late: bool) {
        let Some(job) = self.jobs.get(id) else {
            return;
        };
        let execution = text(job, "execution");
        let cancel = job.get("cancel_requested").and_then(|item| item.as_bool()).unwrap_or(false);
        if execution == "succeeded" || execution == "failed" {
            return;
        }
        if late || execution == "uncertain" || execution == "cancelled" || (execution == "submitted" && cancel) {
            if let Outcome::Image(bytes) = outcome {
                if let Some(info) = info {
                    let _ = self.store_artwork(id, &bytes, &info, "stale_provider_output", true);
                }
            }
            if execution == "submitted" {
                let job = self.jobs.get_mut(id).unwrap();
                job["execution"] = json!("uncertain");
                job["provider_stop_confirmed"] = json!(false);
                job["billable_uncertain"] = json!(job.get("billable").and_then(|item| item.as_bool()).unwrap_or(false));
                job["code"] = json!(if cancel { "cancelled_after_submit" } else { "uncertain" });
                job["error"] = json!(if cancel {
                    "The job was cancelled after it was submitted. The provider was not confirmed to have stopped. A late image is kept separately and was not selected."
                } else {
                    "The image request is uncertain. It was not retried. A late image is kept separately and was not selected."
                });
                job["application"] = json!("none");
                self.persist(id);
            }
            return;
        }
        if execution != "submitted" {
            return;
        }
        match outcome {
            Outcome::Image(bytes) => {
                let Some(info) = info else {
                    self.fail(id, "malformed_output", "The provider image could not be inspected. It was not stored and no fixture was substituted.");
                    return;
                };
                if self.store_artwork(id, &bytes, &info, "provider_output", false).is_err() {
                    self.fail(id, "store", "The artwork could not be stored. No fixture was substituted.");
                    return;
                }
                let job = self.jobs.get_mut(id).unwrap();
                job["execution"] = json!("succeeded");
                job["application"] = json!("candidate");
                job["code"] = json!(null);
                job["error"] = json!(null);
                self.persist(id);
            }
            Outcome::Failed(message) => self.fail(id, "malformed_output", &message),
            Outcome::Uncertain(message) => {
                let job = self.jobs.get_mut(id).unwrap();
                let billable = job.get("billable").and_then(|item| item.as_bool()).unwrap_or(false);
                job["execution"] = json!("uncertain");
                job["billable_uncertain"] = json!(billable);
                job["provider_stop_confirmed"] = json!(false);
                job["code"] = json!("uncertain");
                job["error"] = json!(message);
                job["application"] = json!("none");
                self.persist(id);
            }
            Outcome::Stopped => {
                let job = self.jobs.get_mut(id).unwrap();
                job["execution"] = json!("cancelled");
                job["provider_stop_confirmed"] = json!(true);
                job["code"] = json!("cancelled");
                job["error"] = json!("The provider confirmed that it did not continue this request. No artwork was selected.");
                self.persist(id);
            }
        }
    }

    fn fail(&mut self, id: &str, code: &str, message: &str) {
        let Some(job) = self.jobs.get_mut(id) else {
            return;
        };
        job["execution"] = json!("failed");
        job["code"] = json!(code);
        job["error"] = json!(message);
        job["application"] = json!("none");
        self.persist(id);
    }

    fn store_artwork(
        &mut self,
        job_id: &str,
        bytes: &[u8],
        info: &ImageInfo,
        role: &str,
        stale: bool,
    ) -> Result<String, ()> {
        let hash = sha256_hex(bytes);
        store_bytes(&self.root, &hash, bytes).map_err(|_| ())?;
        let artifact_id = fresh_id("art", &format!("{role}:{hash}:{job_id}"));
        let record = json!({
            "schema": "embroideryos-creative-artifact/0.1",
            "artifact_id": artifact_id,
            "job_id": job_id,
            "role": role,
            "media_type": info.media_type,
            "representation": "raster",
            "editable_vector": false,
            "layered_art": false,
            "dimensional_cad": false,
            "embroidery_geometry": false,
            "sha256": hash,
            "byte_length": bytes.len(),
            "width": info.width,
            "height": info.height,
            "has_alpha": info.has_alpha,
            "alpha_inspected": true,
            "immutable": true,
        });
        write_json(&self.root.join("artifacts").join(format!("{artifact_id}.json")), &record).map_err(|_| ())?;
        let job = self.jobs.get_mut(job_id).ok_or(())?;
        let key = if stale { "stale_artifact_ids" } else { "artifact_ids" };
        job[key].as_array_mut().ok_or(())?.push(json!(artifact_id));
        Ok(artifact_id)
    }

    fn persist(&self, id: &str) {
        if let Some(job) = self.jobs.get(id) {
            let _ = write_json(&self.root.join("jobs").join(format!("{id}.json")), job);
        }
    }

    fn public(&self, id: &str) -> Value {
        let Some(job) = self.jobs.get(id) else {
            return json!({"ok": false, "code": "not_found"});
        };
        let mut artifacts = Vec::new();
        for artifact_id in job["artifact_ids"].as_array().cloned().unwrap_or_default() {
            if let Some(id) = artifact_id.as_str() {
                if let Some(meta) = read_json(&self.root.join("artifacts").join(format!("{id}.json"))) {
                    artifacts.push(meta);
                }
            }
        }
        let execution = text(job, "execution");
        json!({
            "ok": execution == "succeeded",
            "schema": job.get("schema").cloned().unwrap_or(json!(null)),
            "job_id": job.get("job_id").cloned().unwrap_or(json!(null)),
            "client_request_id": job.get("client_request_id").cloned().unwrap_or(json!(null)),
            "source_revision": job.get("source_revision").cloned().unwrap_or(json!(null)),
            "exact_request": job.get("exact_request").cloned().unwrap_or(json!(null)),
            "brief": {
                "art_direction": job["brief"].get("art_direction").cloned().unwrap_or(json!(null)),
                "profile_instruction": job["brief"].get("profile_instruction").cloned().unwrap_or(json!(null)),
                "provider_prompt": job["brief"].get("provider_prompt").cloned().unwrap_or(json!(null)),
                "width": job["brief"].get("width").cloned().unwrap_or(json!(null)),
                "height": job["brief"].get("height").cloned().unwrap_or(json!(null)),
                "reference_hashes": job["brief"].get("reference_hashes").cloned().unwrap_or(json!([])),
                "preserves_subject": true,
                "embroidery_geometry": false,
                "agents": ["creative-director"],
            },
            "provider": {
                "id": job["provider"].get("id").cloned().unwrap_or(json!(null)),
                "model": job["provider"].get("model").cloned().unwrap_or(json!(null)),
                "operation": job["provider"].get("operation").cloned().unwrap_or(json!(null)),
                "live": job["provider"].get("live").cloned().unwrap_or(json!(false)),
                "route": job["provider"].get("route").cloned().unwrap_or(json!(null)),
                "endpoint": job["provider"].get("endpoint").cloned().unwrap_or(json!(null)),
                "allow_fallbacks": job["provider"].get("allow_fallbacks").cloned().unwrap_or(json!(false)),
                "authorisation_id": job["provider"].get("authorisation_id").cloned().unwrap_or(json!(null)),
            },
            "execution": execution,
            "application": job.get("application").cloned().unwrap_or(json!("none")),
            "cancel_requested": job.get("cancel_requested").cloned().unwrap_or(json!(false)),
            "provider_stop_confirmed": job.get("provider_stop_confirmed").cloned().unwrap_or(json!(false)),
            "billable": job.get("billable").cloned().unwrap_or(json!(false)),
            "billable_uncertain": job.get("billable_uncertain").cloned().unwrap_or(json!(false)),
            "attempts": job.get("attempts").cloned().unwrap_or(json!(0)),
            "artifact_ids": job.get("artifact_ids").cloned().unwrap_or(json!([])),
            "stale_artifact_ids": job.get("stale_artifact_ids").cloned().unwrap_or(json!([])),
            "artifacts": artifacts,
            "master_id": job.get("master_id").cloned().unwrap_or(json!(null)),
            "error": job.get("error").cloned().unwrap_or(json!(null)),
            "code": job.get("code").cloned().unwrap_or(json!(null)),
            "prompt_transmitted": job.get("prompt_transmitted").cloned().unwrap_or(json!(false)),
            "provider_request_id": job.get("provider_request_id").cloned().unwrap_or(json!(null)),
            "provider_usage": job.get("provider_usage").cloned().unwrap_or(json!(null)),
            "machine_release_allowed": false,
            "embroidery_project_changed": false,
            "provider_calls": self.provider_calls,
        })
    }
}

struct Call {
    python: PathBuf,
    provider: String,
    model: String,
    operation: String,
    script: String,
    prompt: String,
    width: u32,
    height: u32,
    network_enabled: bool,
    credential_present: bool,
    openrouter_credential_present: bool,
}

struct ImageInfo {
    media_type: String,
    width: u32,
    height: u32,
    has_alpha: bool,
}

fn produce(call: &Call) -> (Outcome, Value) {
    if call.provider == "test_image" {
        return (test_image(call), json!(null));
    }
    if call.provider == "openrouter_images" {
        if !call.network_enabled || !call.openrouter_credential_present {
            return (
                Outcome::Failed(
                    "Live OpenRouter image calls are not enabled for this studio process. Nothing was sent.".to_string(),
                ),
                json!(null),
            );
        }
        return match openrouter_generate(call) {
            Ok(image) => (
                Outcome::Image(image.bytes),
                json!({
                    "request_id": image.request_id,
                    "usage": image.usage,
                }),
            ),
            Err(error) => {
                let outcome = if error.uncertain {
                    Outcome::Uncertain(error.message)
                } else {
                    Outcome::Failed(error.message)
                };
                (outcome, json!(null))
            }
        };
    }
    if call.provider != "xai_images" {
        return (
            Outcome::Failed("The creative provider is not configured. Nothing was sent.".to_string()),
            json!(null),
        );
    }
    if !call.network_enabled || !call.credential_present {
        return (
            Outcome::Failed(
                "Live image calls are not enabled for this studio process. Nothing was sent.".to_string(),
            ),
            json!(null),
        );
    }
    match xai_generate(call) {
        Ok(image) => (
            Outcome::Image(image.bytes),
            json!({
                "request_id": image.request_id,
                "usage": image.usage,
            }),
        ),
        Err(error) => {
            let outcome = if error.uncertain {
                Outcome::Uncertain(error.message)
            } else {
                Outcome::Failed(error.message)
            };
            (outcome, json!(null))
        }
    }
}

fn test_image(call: &Call) -> Outcome {
    // The counter lives on the service. The caller increments before produce for test jobs.
    match call.script.as_str() {
        "malformed" => Outcome::Image(b"{\"objects\":[]}".to_vec()),
        "timeout" => Outcome::Uncertain(
            "The image request timed out. The outcome is uncertain and it was not retried. A late image will not replace current artwork.".to_string(),
        ),
        "stopped" => Outcome::Stopped,
        "png-alpha" => Outcome::Image(encode_png(call.width.min(128), call.height.min(128), true, &call.prompt, &call.script)),
        _ => Outcome::Image(encode_png(call.width.min(128), call.height.min(128), false, &call.prompt, &call.script)),
    }
}

struct ProviderError {
    message: String,
    uncertain: bool,
}

struct LiveImage {
    bytes: Vec<u8>,
    request_id: Option<String>,
    usage: Value,
}

fn openrouter_generate(call: &Call) -> Result<LiveImage, ProviderError> {
    let payload = json!({
        "operation": call.operation,
        "model": call.model,
        "prompt": call.prompt,
        "width": call.width,
        "height": call.height,
        "timeout_secs": 90,
    });
    decode_live_image(invoke_image_cli(call, "generate-openrouter", &payload)?)
}

fn xai_generate(call: &Call) -> Result<LiveImage, ProviderError> {
    let payload = json!({
        "operation": call.operation,
        "model": call.model,
        "prompt": call.prompt,
        "width": call.width,
        "height": call.height,
        "timeout_secs": 90,
        "references": [],
    });
    decode_live_image(invoke_image_cli(call, "generate", &payload)?)
}

fn invoke_image_cli(call: &Call, command: &str, payload: &Value) -> Result<Value, ProviderError> {
    let bytes = serde_json::to_vec(payload).map_err(|_| ProviderError {
        message: "The image request could not be prepared. Nothing was sent.".to_string(),
        uncertain: false,
    })?;
    let mut child = Command::new(&call.python)
        .arg("-m")
        .arg("embroidery_ai.images")
        .arg(command)
        .current_dir(repo_root())
        .env("PYTHONPATH", repo_root().join("python"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ProviderError {
            message: "The image adapter could not start. Nothing was stored.".to_string(),
            uncertain: false,
        })?;
    {
        let mut stdin = child.stdin.take().ok_or_else(|| ProviderError {
            message: "The image adapter did not accept the request. Nothing was stored.".to_string(),
            uncertain: true,
        })?;
        stdin.write_all(&bytes).map_err(|_| ProviderError {
            message: "The image adapter did not accept the request. The outcome is uncertain and it was not retried.".to_string(),
            uncertain: true,
        })?;
    }
    let stdout = child.stdout.take().ok_or_else(|| ProviderError {
        message: "The image adapter produced no answer. The outcome is uncertain and it was not retried.".to_string(),
        uncertain: true,
    })?;
    let handle = thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout.take(12_000_000).read_to_end(&mut buffer);
        buffer
    });
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() > Duration::from_secs(100) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = handle.join();
                return Err(ProviderError {
                    message: "The image request timed out. The outcome is uncertain and it was not retried.".to_string(),
                    uncertain: true,
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(30)),
            Err(_) => {
                let _ = child.kill();
                return Err(ProviderError {
                    message: "The image adapter stopped unexpectedly. The outcome is uncertain and it was not retried.".to_string(),
                    uncertain: true,
                });
            }
        }
    }
    let output = handle.join().unwrap_or_default();
    let parsed: Value = serde_json::from_slice(&output).map_err(|_| ProviderError {
        message: "The image adapter returned an unreadable answer. It was not retried.".to_string(),
        uncertain: true,
    })?;
    if parsed.get("ok").and_then(|item| item.as_bool()) != Some(true) {
        let uncertain = parsed.get("uncertain").and_then(|item| item.as_bool()).unwrap_or(false);
        let message = parsed
            .get("error")
            .and_then(|item| item.as_str())
            .unwrap_or("The image service did not return artwork. Nothing was stored.")
            .to_string();
        return Err(ProviderError { message, uncertain });
    }
    Ok(parsed)
}

fn decode_live_image(parsed: Value) -> Result<LiveImage, ProviderError> {
    let encoded = parsed.get("image_b64").and_then(|item| item.as_str()).unwrap_or("");
    let bytes = decode_base64(encoded).map_err(|_| ProviderError {
        message: "The provider image bytes could not be decoded. They were not stored.".to_string(),
        uncertain: true,
    })?;
    let request_id = parsed
        .get("provider_request_id")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.chars().take(128).collect());
    Ok(LiveImage {
        bytes,
        request_id,
        usage: parsed.get("usage").cloned().unwrap_or(json!(null)),
    })
}

fn gate_failure(inner: &Inner, job: &Value, policy: &Value) -> Option<(&'static str, String)> {
    let provider = text(&job["provider"], "id");
    let profile = text(&job["brief"], "art_direction");
    let references = job["brief"]["reference_hashes"].as_array().map(|items| items.len()).unwrap_or(0);
    if profile == "portrait" && references == 0 {
        return Some((
            "reference_required",
            "A portrait treatment needs one supplied reference image. None was attached. No likeness was invented and nothing was sent.".to_string(),
        ));
    }
    if references > 0 {
        return Some((
            "reference_not_supported",
            "This generation adapter does not accept reference images. Reference editing was not called. No likeness was invented and nothing was sent.".to_string(),
        ));
    }
    if provider == "test_image" {
        return None;
    }
    if text(&policy, "direct_xai_authorisation") == "superseded_unused" && provider == "xai_images" {
        return Some((
            "route_superseded",
            "The direct xAI image allowance was superseded before it was used. This store has one OpenRouter submission and no second allowance. Nothing was sent.".to_string(),
        ));
    }
    if let Some(expected) = inner.options.provider_env.as_deref() {
        if expected != provider {
            return Some((
                "provider_mismatch",
                format!("EMBROIDERY_CREATIVE_PROVIDER is \"{expected}\" and the job policy provider is \"{provider}\". Nothing was sent."),
            ));
        }
    }
    let model = text(&job["provider"], "model");
    if let Some(expected) = inner.options.model_env.as_deref() {
        if expected != model {
            return Some((
                "model_mismatch",
                format!("EMBROIDERY_CREATIVE_MODEL is \"{expected}\" and the job policy model is \"{model}\". Nothing was sent."),
            ));
        }
    }
    if provider != "xai_images" && provider != "openrouter_images" {
        return Some((
            "unconfigured",
            "Creative image generation has no configured provider. Set the creative policy provider, model, budget, and transmit permission. Nothing was sent.".to_string(),
        ));
    }
    if !text(policy, "test_script").is_empty() {
        return Some((
            "test_fallback_disabled",
            "This live policy still names a test script. Test-provider fallback is disabled, and nothing was sent. Clear test_script and submit a new job.".to_string(),
        ));
    }
    if text(&job["provider"], "operation") != "generate" {
        return Some((
            "unsupported_operation",
            "The creative policy operation is not image generation. Image editing is a separate operation and was not called.".to_string(),
        ));
    }
    let model_ok = if provider == "openrouter_images" {
        model == OPENROUTER_MODEL
    } else {
        model == IMAGE_MODEL
    };
    if model == GEOMETRY_MODEL || !model_ok {
        let expected = if provider == "openrouter_images" { OPENROUTER_MODEL } else { IMAGE_MODEL };
        return Some((
            "unsupported_model",
            format!("The model \"{model}\" is not a supported image-generation model for this adapter. The geometry model was not called. Choose {expected} in the creative policy and submit a new job."),
        ));
    }
    let key_ready = if provider == "openrouter_images" {
        inner.options.openrouter_credential_present
    } else {
        inner.options.credential_present
    };
    if !key_ready {
        let message = if provider == "openrouter_images" {
            "The studio has no OPENROUTER_API_KEY. The direct XAI_API_KEY was not used. No image was requested.".to_string()
        } else {
            "The studio has no XAI_API_KEY. No image was requested. Set that key for the studio process and submit a new job after the budget and transmit permission are configured.".to_string()
        };
        return Some(("missing_credentials", message));
    }
    if !policy.get("transmit_permitted").and_then(|item| item.as_bool()).unwrap_or(false)
        || !job.get("transmit_requested").and_then(|item| item.as_bool()).unwrap_or(false)
    {
        return Some((
            "transmit_refused",
            "This job does not have permission to send the prompt. Nothing was sent. Turn on transmit permission for this request and submit a new job.".to_string(),
        ));
    }
    if policy.get("remaining_calls").and_then(|item| item.as_u64()).unwrap_or(0) == 0 {
        return Some((
            "budget_exhausted",
            "The creative image budget has no remaining calls. No image was requested. Raise the configured budget, then submit a new job.".to_string(),
        ));
    }
    if !inner.options.network_enabled {
        return Some((
            "live_disabled",
            "Live image calls are not enabled for this studio process. Start it with EMBROIDERY_CREATIVE_LIVE=1 only for a budgeted call. Nothing was sent.".to_string(),
        ));
    }
    let ceiling = match spend_ceiling_usd(policy) {
        Some(value) => value,
        None => {
            return Some((
                "spend_ceiling_required",
                "This live image call has no positive spend_ceiling_usd. No image was requested. Set the monetary ceiling, then submit one new job.".to_string(),
            ));
        }
    };
    if provider == "openrouter_images" {
        if policy.get("allow_provider_fallbacks").and_then(|item| item.as_bool()) != Some(false) {
            return Some((
                "fallback_disabled_required",
                "OpenRouter provider fallback must be explicitly disabled for this one-call test. Nothing was sent.".to_string(),
            ));
        }
        match estimated_output_cost_usd(policy) {
            Some(estimate) if estimate <= ceiling => {}
            Some(_) => {
                return Some((
                    "cost_exceeds_ceiling",
                    "The selected image parameters cost more than the authorised spend ceiling. Nothing was sent.".to_string(),
                ));
            }
            None => {
                return Some((
                    "cost_estimate_required",
                    "This OpenRouter image call has no estimated output cost. Nothing was sent.".to_string(),
                ));
            }
        }
    }
    None
}

fn provider_key_configured(provider: &str, options: &Options) -> bool {
    if provider == "openrouter_images" {
        options.openrouter_credential_present
    } else if provider == "xai_images" {
        options.credential_present
    } else {
        false
    }
}

fn provider_endpoint(provider: &str) -> &'static str {
    if provider == "openrouter_images" {
        OPENROUTER_ENDPOINT
    } else if provider == "xai_images" {
        GENERATE_ENDPOINT
    } else {
        ""
    }
}

fn provider_route(provider: &str) -> &'static str {
    if provider == "openrouter_images" {
        "openrouter"
    } else if provider == "xai_images" {
        "xai-direct"
    } else {
        "local"
    }
}

fn estimated_output_cost_usd(policy: &Value) -> Option<f64> {
    let value = policy.get("estimated_output_cost_usd")?;
    let number = value.as_f64().or_else(|| value.as_u64().map(|item| item as f64))?;
    if number.is_finite() && number > 0.0 {
        Some(number)
    } else {
        None
    }
}

fn spend_ceiling_usd(policy: &Value) -> Option<f64> {
    let value = policy.get("spend_ceiling_usd")?;
    let number = value.as_f64().or_else(|| value.as_u64().map(|item| item as f64))?;
    if number.is_finite() && number > 0.0 {
        Some(number)
    } else {
        None
    }
}

fn profile_instruction(profile: &str) -> Option<&'static str> {
    match profile {
        "cartoon" => Some("Illustrate the subject as a colourful cartoon character. Use a clean background and keep the silhouette readable."),
        "portrait" => Some("Treat the subject as a portrait of the supplied reference. Keep that identity. Do not invent a different person."),
        "renaissance-drawing" => Some("Render the subject as a Renaissance-style drawing. Keep the requested subject and setting."),
        "painterly-landscape" => Some("Render the subject as a painterly landscape. Keep the requested place, light, and season."),
        "none" => Some("Follow the request without adding a new subject."),
        _ => None,
    }
}

fn provider_prompt(instruction: &str, request: &str, width: u64, height: u64) -> String {
    format!("{instruction}\n\nSubject and direction, kept as requested:\n{request}\n\nOutput a single {width} by {height} pixel raster image. Preserve the requested subject, artistic direction, and output dimensions. Do not convert the image into embroidery geometry, a stitch plan, vector paths, or a simplified mark unless the request itself asks for that.")
}

fn default_policy() -> Value {
    json!({
        "schema": "embroideryos-creative-policy/0.1",
        "provider": "unconfigured",
        "model": "",
        "operation": "generate",
        "remaining_calls": 0,
        "transmit_permitted": false,
        "spend_ceiling_usd": null,
        "test_script": "",
        "timeout_ms": 90000,
        "max_stored_bytes": MAX_STORED,
    })
}

fn text(value: &Value, key: &str) -> String {
    value.get(key).and_then(|item| item.as_str()).unwrap_or("").trim().to_string()
}

fn token(value: String, min: usize, max: usize) -> Result<String, ()> {
    if value.len() < min || value.len() > max {
        return Err(());
    }
    if !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b':' || byte == b'.') {
        return Err(());
    }
    Ok(value)
}

fn valid_id(id: &str) -> bool {
    let Some((prefix, rest)) = id.split_once('-') else {
        return false;
    };
    matches!(prefix, "job" | "art" | "mas")
        && rest.len() == 32
        && rest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn fresh_id(kind: &str, material: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tick = SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.as_nanos()).unwrap_or(0);
    let digest = sha256_hex(format!("{kind}:{count}:{tick}:{material}").as_bytes());
    format!("{kind}-{}", &digest[..32])
}

fn reference_bytes(value: &Value) -> Result<Vec<Vec<u8>>, String> {
    let Some(items) = value.get("references").and_then(|item| item.as_array()) else {
        return Ok(Vec::new());
    };
    if items.len() > 1 {
        return Err("Send at most one reference image. The name-drop table was not requested and nothing was sent.".to_string());
    }
    let mut out = Vec::new();
    for item in items {
        let encoded = item.get("data_base64").and_then(|value| value.as_str()).unwrap_or("");
        let payload = encoded.split_once(',').map(|(_, data)| data).unwrap_or(encoded);
        let bytes = decode_base64(payload).map_err(|_| {
            "The reference image is not valid base64. Nothing was sent.".to_string()
        })?;
        if bytes.len() > MAX_REFERENCE {
            return Err("The reference image is too large for this request. Choose a smaller image. Nothing was sent.".to_string());
        }
        if !(bytes.starts_with(b"\x89PNG\r\n\x1a\n") || bytes.starts_with(&[0xff, 0xd8, 0xff])) {
            return Err("Attach a PNG or JPEG reference. Nothing was sent.".to_string());
        }
        out.push(bytes);
    }
    Ok(out)
}

fn store_bytes(root: &Path, hash: &str, bytes: &[u8]) -> Result<(), String> {
    let path = root.join("blobs").join(hash);
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| "The creative store could not be created.".to_string())?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(|_| "The artwork bytes could not be written.".to_string())?;
    fs::rename(&temporary, &path).map_err(|_| "The artwork bytes could not be saved.".to_string())?;
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| "The creative store could not be created.".to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| "The creative record could not be encoded.".to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(|_| "The creative record could not be written.".to_string())?;
    fs::rename(&temporary, path).map_err(|_| "The creative record could not be saved.".to_string())?;
    Ok(())
}

fn read_json(path: &Path) -> Option<Value> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn inspect_image(python: &Path, bytes: &[u8]) -> Result<ImageInfo, String> {
    let mut child = Command::new(python)
        .arg("-m")
        .arg("embroidery_ai.images")
        .arg("inspect")
        .current_dir(repo_root())
        .env("PYTHONPATH", repo_root().join("python"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "The image inspector could not start.".to_string())?;
    {
        let mut stdin = child.stdin.take().ok_or_else(|| "The image inspector did not accept the file.".to_string())?;
        stdin.write_all(bytes).map_err(|_| "The image inspector did not accept the file.".to_string())?;
    }
    let output = child.wait_with_output().map_err(|_| "The image inspector stopped.".to_string())?;
    let parsed: Value = serde_json::from_slice(&output.stdout).unwrap_or(json!({"ok": false}));
    if parsed.get("ok").and_then(|item| item.as_bool()) != Some(true) {
        return Err("The returned data is not a usable PNG or JPEG.".to_string());
    }
    Ok(ImageInfo {
        media_type: text(&parsed, "media_type"),
        width: parsed.get("width").and_then(|item| item.as_u64()).unwrap_or(0) as u32,
        height: parsed.get("height").and_then(|item| item.as_u64()).unwrap_or(0) as u32,
        has_alpha: parsed.get("has_alpha").and_then(|item| item.as_bool()).unwrap_or(true),
    })
}

fn encode_png(width: u32, height: u32, alpha: bool, request: &str, script: &str) -> Vec<u8> {
    let width = width.max(1);
    let height = height.max(1);
    let channels: usize = if alpha { 4 } else { 3 };
    let marker = sha256_hex(format!("{script}:{request}").as_bytes());
    let mark = u8::from_str_radix(&marker[..2], 16).unwrap_or(0);
    let (base, alt) = match script {
        "renaissance-drawing" => ([92, 64, 36], [214, 196, 160]),
        "painterly-landscape" => ([36, 92, 64], [168, 196, 214]),
        "portrait" => ([72, 84, 120], [180, 176, 168]),
        _ => ([32, 112, 196], [232, 176, 48]),
    };
    let mut raw = Vec::with_capacity((width as usize * channels + 1) * height as usize);
    for y in 0..height {
        raw.push(0);
        for x in 0..width {
            let mut pixel = if ((x / 8) + (y / 8)) % 2 == 0 { base } else { alt };
            if x == 0 && y == 0 {
                pixel[0] = mark;
            }
            raw.extend_from_slice(&pixel);
            if alpha {
                raw.push(if (x + y) % 2 == 0 { 0 } else { 255 });
            }
        }
    }
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, if alpha { 6 } else { 2 }, 0, 0, 0]);
    let mut png = vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    png.extend(chunk(b"IHDR", &ihdr));
    png.extend(chunk(b"IDAT", &zlib_store(&raw)));
    png.extend(chunk(b"IEND", &[]));
    png
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(4 + data.len());
    body.extend_from_slice(kind);
    body.extend_from_slice(data);
    let crc = crc32(&body);
    let mut out = Vec::new();
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc.to_be_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut offset = 0;
    loop {
        let end = (offset + 65535).min(data.len());
        let last = end == data.len();
        let block = &data[offset..end];
        out.push(if last { 1 } else { 0 });
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
        offset = end;
        if last {
            break;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn decode_base64(text: &str) -> Result<Vec<u8>, ()> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() % 4 != 0 {
        return Err(());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut index = 0;
    while index < bytes.len() {
        let third = bytes[index + 2];
        let fourth = bytes[index + 3];
        let first = value(bytes[index]).ok_or(())?;
        let second = value(bytes[index + 1]).ok_or(())?;
        let third_value = if third == b'=' { 0 } else { value(third).ok_or(())? };
        let fourth_value = if fourth == b'=' { 0 } else { value(fourth).ok_or(())? };
        let packed = (u32::from(first) << 18)
            | (u32::from(second) << 12)
            | (u32::from(third_value) << 6)
            | u32::from(fourth_value);
        out.push((packed >> 16) as u8);
        if third != b'=' {
            out.push((packed >> 8) as u8);
        }
        if fourth != b'=' {
            out.push(packed as u8);
        }
        index += 4;
    }
    Ok(out)
}

#[cfg(test)]
fn encode_base64(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut index = 0;
    while index + 3 <= data.len() {
        let packed = (u32::from(data[index]) << 16) | (u32::from(data[index + 1]) << 8) | u32::from(data[index + 2]);
        out.push(TABLE[((packed >> 18) & 63) as usize] as char);
        out.push(TABLE[((packed >> 12) & 63) as usize] as char);
        out.push(TABLE[((packed >> 6) & 63) as usize] as char);
        out.push(TABLE[(packed & 63) as usize] as char);
        index += 3;
    }
    let rest = data.len() - index;
    if rest == 1 {
        let packed = u32::from(data[index]) << 16;
        out.push(TABLE[((packed >> 18) & 63) as usize] as char);
        out.push(TABLE[((packed >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rest == 2 {
        let packed = (u32::from(data[index]) << 16) | (u32::from(data[index + 1]) << 8);
        out.push(TABLE[((packed >> 18) & 63) as usize] as char);
        out.push(TABLE[((packed >> 12) & 63) as usize] as char);
        out.push(TABLE[((packed >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}

#[cfg(test)]
fn test_options() -> Options {
    Options {
        credential_present: false,
        openrouter_credential_present: false,
        network_enabled: false,
        python: python_bin(),
        provider_env: None,
        model_env: None,
    }
}

#[cfg(test)]
fn test_store(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("embroidery-creative-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[cfg(test)]
fn write_policy(root: &Path, value: Value) {
    write_json(&root.join("policy.json"), &value).unwrap();
}

#[cfg(test)]
fn local_policy(script: &str) -> Value {
    json!({
        "schema": "embroideryos-creative-policy/0.1",
        "provider": "test_image",
        "model": "test-raster",
        "operation": "generate",
        "remaining_calls": 0,
        "transmit_permitted": false,
        "test_script": script,
    })
}

#[cfg(test)]
fn openrouter_policy(estimate: f64, ceiling: f64) -> Value {
    json!({
        "schema": "embroideryos-creative-policy/0.1",
        "provider": "openrouter_images",
        "model": OPENROUTER_MODEL,
        "operation": "generate",
        "remaining_calls": 1,
        "transmit_permitted": true,
        "spend_ceiling_usd": ceiling,
        "estimated_output_cost_usd": estimate,
        "allow_provider_fallbacks": false,
        "test_script": "",
        "direct_xai_authorisation": "superseded_unused",
        "authorisation_id": "original-robot-one-call",
    })
}

#[cfg(test)]
fn xai_policy(model: &str, remaining: u64, transmit: bool) -> Value {
    json!({
        "schema": "embroideryos-creative-policy/0.1",
        "provider": "xai_images",
        "model": model,
        "operation": "generate",
        "remaining_calls": remaining,
        "transmit_permitted": transmit,
        "test_script": "",
    })
}

#[cfg(test)]
fn request(id: &str, revision: &str, text: &str, profile: &str, references: Value, transmit: bool) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "client_request_id": id,
        "source_revision": revision,
        "request": text,
        "art_direction": profile,
        "width": 32,
        "height": 32,
        "transmit": transmit,
        "references": references,
    }))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn creative_jobs_isolate_revision_cancellation_and_failures() {
        let root = test_store("cases");
        write_policy(&root, local_policy("png"));
        let service = Service::open(&root, test_options()).unwrap();
        let mut failures = Vec::new();
        let robot = "Create an original friendly exploration robot, illustrated as a colourful cartoon character. Use a clean background and keep the silhouette readable.";

        let mut run = |name: &str, body: &dyn Fn()| {
            let result = catch_unwind(AssertUnwindSafe(|| body()));
            if let Err(payload) = result {
                let message = payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("assertion failed");
                failures.push(format!("{name}: {message}"));
            }
        };

        run("artwork without dst", &|| {
            let job = service.submit_sync(&request("case-cartoon", "rev-1", robot, "cartoon", json!([]), false));
            assert_eq!(job["execution"], "succeeded", "{job}");
            assert_eq!(job["application"], "candidate");
            assert_eq!(job["provider"]["live"], false);
            assert_eq!(job["provider"]["id"], "test_image");
            assert_eq!(job["embroidery_project_changed"], false);
            assert_eq!(job["machine_release_allowed"], false);
            assert!(job["brief"]["provider_prompt"].as_str().unwrap().contains(robot));
            assert!(!job["brief"]["provider_prompt"].as_str().unwrap().contains("rev-1"));
            assert_eq!(job["brief"]["embroidery_geometry"], false);
            let artifact = &job["artifacts"][0];
            assert_eq!(artifact["representation"], "raster");
            assert_eq!(artifact["editable_vector"], false);
            assert_eq!(artifact["has_alpha"], false);
            assert_eq!(artifact["alpha_inspected"], true);
            let bytes = fs::read(root.join("blobs").join(artifact["sha256"].as_str().unwrap())).unwrap();
            assert_eq!(sha256_hex(&bytes), artifact["sha256"].as_str().unwrap());
            assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        });

        run("candidate does not apply itself", &|| {
            let job = service.submit_sync(&request("case-hold", "rev-1", robot, "cartoon", json!([]), false));
            assert_eq!(job["application"], "candidate");
            assert!(job.get("master_id").is_none() || job["master_id"].is_null());
            assert!(!root.join("masters").join("current.json").exists() || {
                let current = read_json(&root.join("masters").join("current.json")).unwrap();
                current["job_id"] != job["job_id"]
            });
        });

        run("duplicate submission", &|| {
            let before = service.lock().provider_calls;
            let again = service.submit_sync(&request("case-cartoon", "rev-1", robot, "cartoon", json!([]), false));
            assert_eq!(again["duplicate"], true, "{again}");
            assert_eq!(service.lock().provider_calls, before);
            assert_eq!(again["attempts"], 1);
        });

        run("select and apply keep embroidery untouched", &|| {
            let sample = fs::read(repo_root().join("fixtures/projects/08_club_mark.json")).unwrap();
            let sample_hash = sha256_hex(&sample);
            let job = service.submit_sync(&request("case-save", "rev-save", robot, "cartoon", json!([]), false));
            let id = job["job_id"].as_str().unwrap().to_string();
            let saved = service.select(&id, &serde_json::to_vec(&json!({"current_revision": "rev-save"})).unwrap());
            assert_eq!(saved["ok"], true, "{saved}");
            assert_eq!(saved["application"], "selected");
            let master = saved["master_id"].as_str().unwrap().to_string();
            let applied = service.apply(&serde_json::to_vec(&json!({"master_id": master, "current_revision": "rev-save"})).unwrap());
            assert_eq!(applied["ok"], true, "{applied}");
            assert_eq!(applied["embroidery_project_changed"], false);
            assert_eq!(sha256_hex(&fs::read(repo_root().join("fixtures/projects/08_club_mark.json")).unwrap()), sample_hash);
        });

        run("revision change blocks select", &|| {
            let job = service.submit_sync(&request("case-stale-rev", "rev-old", robot, "cartoon", json!([]), false));
            let id = job["job_id"].as_str().unwrap();
            let refused = service.select(id, br#"{"current_revision":"rev-new"}"#);
            assert_eq!(refused["code"], "revision_mismatch", "{refused}");
            assert_eq!(service.job_view(id)["application"], "candidate");
        });

        run("second success does not replace the master", &|| {
            let first = service.submit_sync(&request("case-first-master", "rev-m", "First landscape", "painterly-landscape", json!([]), false));
            let first_id = first["job_id"].as_str().unwrap().to_string();
            service.select(&first_id, br#"{"current_revision":"rev-m"}"#);
            let current = read_json(&root.join("masters").join("current.json")).unwrap();
            let second = service.submit_sync(&request("case-second-master", "rev-m", "Second landscape", "painterly-landscape", json!([]), false));
            assert_eq!(second["application"], "candidate");
            let still = read_json(&root.join("masters").join("current.json")).unwrap();
            assert_eq!(still["master_id"], current["master_id"]);
            assert_ne!(second["artifacts"][0]["sha256"], first["artifacts"][0]["sha256"]);
        });

        run("changed references do not overwrite", &|| {
            let png = encode_png(8, 8, false, "reference-a", "png");
            let other = encode_png(8, 8, false, "reference-b", "png");
            let left = service.submit_sync(&request(
                "case-ref-a",
                "rev-r",
                "A renaissance drawing of a bridge",
                "renaissance-drawing",
                json!([{"data_base64": encode_base64(&png)}]),
                false,
            ));
            let right = service.submit_sync(&request(
                "case-ref-b",
                "rev-r",
                "A renaissance drawing of a bridge",
                "renaissance-drawing",
                json!([{"data_base64": encode_base64(&other)}]),
                false,
            ));
            assert_eq!(left["code"], "reference_not_supported", "{left}");
            assert_eq!(right["code"], "reference_not_supported");
            assert_ne!(left["brief"]["reference_hashes"][0], right["brief"]["reference_hashes"][0]);
            assert_eq!(left["provider_calls"], right["provider_calls"]);
            assert!(left["artifact_ids"].as_array().unwrap().is_empty());
        });

        run("portrait without reference is blocked", &|| {
            let job = service.submit_sync(&request("case-portrait", "rev-p", "Portrait of a person", "portrait", json!([]), false));
            assert_eq!(job["code"], "reference_required", "{job}");
            assert_eq!(job["execution"], "failed");
            assert_eq!(job["attempts"], 0);
        });

        run("name-drop material is refused", &|| {
            let job = service.submit_sync(&serde_json::to_vec(&json!({
                "client_request_id": "case-private",
                "source_revision": "rev-1",
                "request": robot,
                "art_direction": "cartoon",
                "width": 32,
                "height": 32,
                "items": [{"name": "customer"}],
            })).unwrap());
            assert_eq!(job["code"], "private_material", "{job}");
        });

        run("cancel before submit", &|| {
            write_policy(&root, local_policy("png"));
            let body = request("case-cancel-before", "rev-c", robot, "cartoon", json!([]), false);
            let value: Value = serde_json::from_slice(&body).unwrap();
            let staged = service.prepare(&body);
            let Stage::Ready(id) = staged else {
                panic!("expected a queued job");
            };
            let _ = value;
            let calls = service.lock().provider_calls;
            let cancelled = service.cancel(&id);
            assert_eq!(cancelled["execution"], "cancelled", "{cancelled}");
            assert_eq!(cancelled["provider_stop_confirmed"], true);
            service.execute(&id);
            assert_eq!(service.lock().provider_calls, calls);
            assert_eq!(service.job_view(&id)["execution"], "cancelled");
        });

        run("cancel after submit keeps a late image unselected", &|| {
            let mut inner = service.lock();
            let id = fresh_id("job", "late-cancel");
            inner.insert_job(json!({
                "schema": "embroideryos-creative-job/0.1",
                "job_id": id,
                "client_request_id": "case-cancel-after",
                "source_revision": "rev-c2",
                "exact_request": robot,
                "brief": {"art_direction": "cartoon", "provider_prompt": robot, "width": 8, "height": 8, "reference_hashes": [], "embroidery_geometry": false, "agents": ["creative-director"]},
                "provider": {"id": "test_image", "model": "test-raster", "operation": "generate", "live": false, "script": "png"},
                "execution": "submitted",
                "application": "none",
                "cancel_requested": true,
                "provider_stop_confirmed": false,
                "billable": false,
                "billable_uncertain": false,
                "attempts": 1,
                "artifact_ids": [],
                "stale_artifact_ids": [],
                "prompt_transmitted": false,
                "machine_release_allowed": false,
                "embroidery_project_changed": false,
            }));
            let bytes = encode_png(8, 8, false, robot, "png");
            let info = inspect_image(&python_bin(), &bytes).unwrap();
            inner.deliver(&id, Outcome::Image(bytes), Some(info), true);
            let view = inner.public(&id);
            assert_eq!(view["execution"], "uncertain", "{view}");
            assert_eq!(view["application"], "none");
            assert_eq!(view["provider_stop_confirmed"], false);
            assert_eq!(view["artifact_ids"].as_array().unwrap().len(), 0);
            assert_eq!(view["stale_artifact_ids"].as_array().unwrap().len(), 1);
            drop(inner);
            let selected = service.select(&id, br#"{"current_revision":"rev-c2"}"#);
            assert_eq!(selected["code"], "not_candidate");
        });

        run("timeout is uncertain and a late image does not succeed", &|| {
            let mut inner = service.lock();
            let id = fresh_id("job", "timeout");
            inner.insert_job(json!({
                "schema": "embroideryos-creative-job/0.1",
                "job_id": id,
                "client_request_id": "case-timeout",
                "source_revision": "rev-t",
                "exact_request": "A painterly landscape",
                "brief": {"art_direction": "painterly-landscape", "provider_prompt": "A painterly landscape", "width": 8, "height": 8, "reference_hashes": [], "embroidery_geometry": false, "agents": ["creative-director"]},
                "provider": {"id": "test_image", "model": "test-raster", "operation": "generate", "live": false},
                "execution": "submitted",
                "application": "none",
                "cancel_requested": false,
                "billable": true,
                "billable_uncertain": false,
                "attempts": 1,
                "artifact_ids": [],
                "stale_artifact_ids": [],
                "machine_release_allowed": false,
                "embroidery_project_changed": false,
            }));
            inner.deliver(&id, Outcome::Uncertain("timed out".to_string()), None, false);
            let bytes = encode_png(8, 8, false, "late", "png");
            let info = inspect_image(&python_bin(), &bytes).unwrap();
            inner.deliver(&id, Outcome::Image(bytes), Some(info), true);
            let view = inner.public(&id);
            assert_eq!(view["execution"], "uncertain", "{view}");
            assert_eq!(view["billable_uncertain"], true);
            assert_eq!(view["application"], "none");
            assert_eq!(view["artifact_ids"].as_array().unwrap().len(), 0);
            assert_eq!(view["stale_artifact_ids"].as_array().unwrap().len(), 1);
            drop(inner);
            let again = service.submit_sync(&request("case-timeout", "rev-t", "A painterly landscape", "painterly-landscape", json!([]), false));
            assert_eq!(again["duplicate"], true, "{again}");
            assert_eq!(again["execution"], "uncertain");
            assert_eq!(again["attempts"], 1);
        });

        run("malformed output is not replaced", &|| {
            write_policy(&root, local_policy("malformed"));
            let job = service.submit_sync(&request("case-bad", "rev-b", robot, "cartoon", json!([]), false));
            assert_eq!(job["execution"], "failed", "{job}");
            assert_eq!(job["code"], "malformed_output");
            assert!(job["artifact_ids"].as_array().unwrap().is_empty());
            assert!(job["error"].as_str().unwrap_or("").contains("was not stored"));
            assert!(job["error"].as_str().unwrap_or("").contains("not replaced with a fixture"));
            let again = service.submit_sync(&request("case-bad", "rev-b", robot, "cartoon", json!([]), false));
            assert_eq!(again["duplicate"], true);
            assert_eq!(again["execution"], "failed");
        });

        run("checkerboard is not treated as transparent", &|| {
            write_policy(&root, local_policy("png"));
            let job = service.submit_sync(&request(
                "case-checker",
                "rev-k",
                "A transparent checkerboard background",
                "cartoon",
                json!([]),
                false,
            ));
            assert_eq!(job["artifacts"][0]["has_alpha"], false, "{job}");
        });

        run("alpha comes from the file", &|| {
            write_policy(&root, local_policy("png-alpha"));
            let job = service.submit_sync(&request("case-alpha", "rev-a", "An opaque badge", "cartoon", json!([]), false));
            assert_eq!(job["execution"], "succeeded", "{job}");
            assert_eq!(job["artifacts"][0]["has_alpha"], true);
            assert_eq!(job["artifacts"][0]["representation"], "raster");
        });

        run("missing credentials", &|| {
            write_policy(&root, xai_policy(IMAGE_MODEL, 2, true));
            let calls = service.lock().provider_calls;
            let job = service.submit_sync(&request("case-key", "rev-key", robot, "cartoon", json!([]), true));
            assert_eq!(job["code"], "missing_credentials", "{job}");
            assert_eq!(service.lock().provider_calls, calls);
            assert_eq!(service.lock().policy()["remaining_calls"], 2);
        });

        run("unsupported geometry model", &|| {
            write_policy(&root, xai_policy(GEOMETRY_MODEL, 2, true));
            let mut options = test_options();
            options.credential_present = true;
            options.network_enabled = true;
            let isolated = test_store("geometry-model");
            write_policy(&isolated, xai_policy(GEOMETRY_MODEL, 2, true));
            let local = Service::open(&isolated, options).unwrap();
            let job = local.submit_sync(&request("case-geometry-model", "rev-g", robot, "cartoon", json!([]), true));
            assert_eq!(job["code"], "unsupported_model", "{job}");
            assert_eq!(job["attempts"], 0);
            assert_eq!(local.lock().provider_calls, 0);
        });

        run("exhausted budget", &|| {
            let isolated = test_store("budget");
            write_policy(&isolated, xai_policy(IMAGE_MODEL, 0, true));
            let mut options = test_options();
            options.credential_present = true;
            options.network_enabled = true;
            let local = Service::open(&isolated, options).unwrap();
            let job = local.submit_sync(&request("case-budget", "rev-budget", robot, "cartoon", json!([]), true));
            assert_eq!(job["code"], "budget_exhausted", "{job}");
            assert_eq!(local.lock().provider_calls, 0);
        });

        run("transmit refused", &|| {
            let isolated = test_store("transmit");
            write_policy(&isolated, xai_policy(IMAGE_MODEL, 3, false));
            let mut options = test_options();
            options.credential_present = true;
            options.network_enabled = true;
            let local = Service::open(&isolated, options).unwrap();
            let job = local.submit_sync(&request("case-transmit", "rev-x", robot, "cartoon", json!([]), true));
            assert_eq!(job["code"], "transmit_refused", "{job}");
            assert_eq!(local.lock().policy()["remaining_calls"], 3);
        });

        run("openrouter does not accept the direct xai key", &|| {
            let isolated = test_store("openrouter-key");
            write_policy(&isolated, openrouter_policy(0.04, 1.0));
            let mut options = test_options();
            options.credential_present = true;
            options.openrouter_credential_present = false;
            options.network_enabled = true;
            let local = Service::open(&isolated, options).unwrap();
            let job = local.submit_sync(&request("case-openrouter-key", "rev-or", robot, "cartoon", json!([]), true));
            assert_eq!(job["code"], "missing_credentials", "{job}");
            assert!(job["error"].as_str().unwrap_or("").contains("OPENROUTER_API_KEY"), "{job}");
            assert_eq!(job["provider"]["route"], "openrouter");
            assert_eq!(job["attempts"], 0);
            assert_eq!(local.lock().provider_calls, 0);
            assert_eq!(local.lock().policy()["remaining_calls"], 1);
        });

        run("the unused direct xai allowance cannot be spent", &|| {
            let isolated = test_store("superseded-xai");
            let mut policy = xai_policy(IMAGE_MODEL, 1, true);
            policy["direct_xai_authorisation"] = json!("superseded_unused");
            policy["spend_ceiling_usd"] = json!(1.0);
            write_policy(&isolated, policy);
            let mut options = test_options();
            options.credential_present = true;
            options.openrouter_credential_present = true;
            options.network_enabled = true;
            let local = Service::open(&isolated, options).unwrap();
            let job = local.submit_sync(&request("case-superseded", "rev-sup", robot, "cartoon", json!([]), true));
            assert_eq!(job["code"], "route_superseded", "{job}");
            assert_eq!(local.lock().policy()["remaining_calls"], 1);
            assert_eq!(local.lock().provider_calls, 0);
        });

        run("an openrouter estimate above the ceiling is refused", &|| {
            let isolated = test_store("openrouter-cost");
            write_policy(&isolated, openrouter_policy(2.0, 1.0));
            let mut options = test_options();
            options.openrouter_credential_present = true;
            options.network_enabled = true;
            let local = Service::open(&isolated, options).unwrap();
            let job = local.submit_sync(&request("case-cost", "rev-cost", robot, "cartoon", json!([]), true));
            assert_eq!(job["code"], "cost_exceeds_ceiling", "{job}");
            assert_eq!(local.lock().policy()["remaining_calls"], 1);
            assert_eq!(local.lock().provider_calls, 0);
        });

        run("the process provider must match the job policy", &|| {
            let isolated = test_store("openrouter-env");
            write_policy(&isolated, openrouter_policy(0.04, 1.0));
            let mut options = test_options();
            options.openrouter_credential_present = true;
            options.network_enabled = true;
            options.provider_env = Some("xai_images".to_string());
            let local = Service::open(&isolated, options).unwrap();
            let job = local.submit_sync(&request("case-env", "rev-env", robot, "cartoon", json!([]), true));
            assert_eq!(job["code"], "provider_mismatch", "{job}");
            assert_eq!(local.lock().policy()["remaining_calls"], 1);
        });

        run("a live call needs an explicit spend ceiling", &|| {
            let isolated = test_store("ceiling");
            write_policy(&isolated, xai_policy(IMAGE_MODEL, 1, true));
            let mut options = test_options();
            options.credential_present = true;
            options.network_enabled = true;
            let local = Service::open(&isolated, options).unwrap();
            let job = local.submit_sync(&request("case-ceiling", "rev-ceiling", robot, "cartoon", json!([]), true));
            assert_eq!(job["code"], "spend_ceiling_required", "{job}");
            assert_eq!(job["attempts"], 0);
            assert_eq!(local.lock().provider_calls, 0);
            assert_eq!(local.lock().policy()["remaining_calls"], 1);
            assert!(job["provider_request_id"].is_null());
        });

        run("key and budget still need the live switch", &|| {
            let isolated = test_store("switch");
            write_policy(&isolated, xai_policy(IMAGE_MODEL, 3, true));
            let mut options = test_options();
            options.credential_present = true;
            options.network_enabled = false;
            let local = Service::open(&isolated, options).unwrap();
            let job = local.submit_sync(&request("case-switch", "rev-s", robot, "cartoon", json!([]), true));
            assert_eq!(job["code"], "live_disabled", "{job}");
            assert_eq!(local.lock().provider_calls, 0);
            assert_eq!(local.lock().policy()["remaining_calls"], 3);
        });

        run("unconfigured provider", &|| {
            let isolated = test_store("none");
            write_policy(&isolated, default_policy());
            let local = Service::open(&isolated, test_options()).unwrap();
            let job = local.submit_sync(&request("case-none", "rev-n", robot, "cartoon", json!([]), false));
            assert_eq!(job["code"], "unconfigured", "{job}");
        });

        run("restart reopens the same bytes and does not retry an in-flight job", &|| {
            write_policy(&root, local_policy("png"));
            let job = service.submit_sync(&request("case-restart", "rev-restart", "A renaissance drawing of a tower", "renaissance-drawing", json!([]), false));
            let hash = job["artifacts"][0]["sha256"].as_str().unwrap().to_string();
            let artifact = job["artifacts"][0]["artifact_id"].as_str().unwrap().to_string();
            let bytes = fs::read(root.join("blobs").join(&hash)).unwrap();
            let orphan = fresh_id("job", "orphan");
            write_json(&root.join("jobs").join(format!("{orphan}.json")), &json!({
                "schema": "embroideryos-creative-job/0.1",
                "job_id": orphan,
                "client_request_id": "case-orphan",
                "source_revision": "rev-o",
                "exact_request": "orphaned",
                "brief": {"reference_hashes": []},
                "provider": {"id": "xai_images", "live": true},
                "execution": "submitted",
                "application": "none",
                "billable": true,
                "attempts": 1,
                "artifact_ids": [],
                "stale_artifact_ids": [],
            })).unwrap();
            let reopened = Service::open(&root, test_options()).unwrap();
            let response = reopened.artifact_bytes(&artifact);
            assert_eq!(response.status, 200);
            assert_eq!(sha256_hex(&response.body), hash);
            assert_eq!(response.body, bytes);
            let orphan_view = reopened.job_view(&orphan);
            assert_eq!(orphan_view["execution"], "uncertain", "{orphan_view}");
            assert_eq!(orphan_view["billable_uncertain"], true);
            assert_eq!(orphan_view["attempts"], 1);
            assert_eq!(reopened.lock().provider_calls, 0);
        });

        run("distinct local styles stay distinct rasters", &|| {
            write_policy(&root, local_policy("png"));
            let styles = [
                ("case-style-cartoon", "cartoon", "original cartoon robot"),
                ("case-style-drawing", "renaissance-drawing", "A renaissance drawing of a courtyard"),
                ("case-style-land", "painterly-landscape", "A painterly landscape of hills at dusk"),
            ];
            let mut hashes = Vec::new();
            for (id, profile, text) in styles {
                let job = service.submit_sync(&request(id, "rev-style", text, profile, json!([]), false));
                assert_eq!(job["execution"], "succeeded", "{id} {job}");
                assert_eq!(job["provider"]["live"], false);
                hashes.push(job["artifacts"][0]["sha256"].as_str().unwrap().to_string());
            }
            assert_eq!(hashes.len(), 3);
            assert_ne!(hashes[0], hashes[1]);
            assert_ne!(hashes[1], hashes[2]);
        });

        if !failures.is_empty() {
            panic!("creative cases failed: {}", failures.join(", "));
        }
    }

    #[test]
    fn creative_http_reopens_the_same_artwork_bytes() {
        let root = test_store("http");
        write_policy(&root, local_policy("png"));
        {
            let mut slot = GLOBAL.lock().unwrap();
            *slot = Some(Service::open(&root, test_options()).unwrap());
        }
        let sample = fs::read(repo_root().join("fixtures/projects/08_club_mark.json")).unwrap();
        let body = request(
            "http-robot",
            "rev-http",
            "Create an original friendly exploration robot, illustrated as a colourful cartoon character. Use a clean background and keep the silhouette readable.",
            "cartoon",
            json!([]),
            false,
        );
        let created = crate::route("POST", "/api/creative/jobs", &body);
        assert_eq!(created.status, 200, "{}", String::from_utf8_lossy(&created.body));
        let mut job: Value = serde_json::from_slice(&created.body).unwrap();
        let id = job["job_id"].as_str().unwrap().to_string();
        for _ in 0..50 {
            if job["execution"] == "succeeded" || job["execution"] == "failed" {
                break;
            }
            thread::sleep(Duration::from_millis(20));
            let polled = crate::route("GET", &format!("/api/creative/jobs/{id}"), b"");
            job = serde_json::from_slice(&polled.body).unwrap();
        }
        assert_eq!(job["execution"], "succeeded", "{job}");
        let artifact = job["artifacts"][0]["artifact_id"].as_str().unwrap().to_string();
        let hash = job["artifacts"][0]["sha256"].as_str().unwrap().to_string();
        let downloaded = crate::route("GET", &format!("/api/creative/artifacts/{artifact}"), b"");
        assert_eq!(downloaded.status, 200);
        assert_eq!(downloaded.content_type, "image/png");
        assert_eq!(sha256_hex(&downloaded.body), hash);
        let selected = crate::route(
            "POST",
            &format!("/api/creative/jobs/{id}/select"),
            br#"{"current_revision":"rev-http"}"#,
        );
        assert_eq!(selected.status, 200, "{}", String::from_utf8_lossy(&selected.body));
        {
            let mut slot = GLOBAL.lock().unwrap();
            *slot = Some(Service::open(&root, test_options()).unwrap());
        }
        let again = crate::route("GET", &format!("/api/creative/artifacts/{artifact}"), b"");
        assert_eq!(again.body, downloaded.body);
        assert_eq!(fs::read(repo_root().join("fixtures/projects/08_club_mark.json")).unwrap(), sample);
        let proposal = crate::route("POST", "/api/propose", b"{}");
        assert_ne!(proposal.status, 404);
    }
}
