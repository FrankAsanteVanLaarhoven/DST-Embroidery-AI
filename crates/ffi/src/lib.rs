//! C ABI for the embroidery engine.
//!
//! Callers pass a JSON [`embroidery_core::StitchPlan`] and receive DST bytes
//! or a JSON report. The same entry points are exported to WASM when the
//! `wasm` feature is enabled. Python research code should call this library.
//! It should not plan stitches itself.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use embroidery_core::{
    capability_manifest, generate_project, parse_project, underlay_rules, validate, StitchPlan,
};
use embroidery_dst::compile_to_vec;
use embroidery_simulate::score;

const VERSION: &[u8] = b"0.1.0\0";
const MAX_STITCH_MM: f64 = 12.1;
const MAX_JUMP_MM: f64 = 1000.0;

pub fn compile_plan_json(plan_json: &str) -> Result<Vec<u8>, String> {
    let plan = parse_plan(plan_json)?;
    compile_to_vec(&plan, MAX_STITCH_MM, MAX_JUMP_MM).map_err(|error| error.to_string())
}

pub fn validate_plan_json(plan_json: &str) -> Result<String, String> {
    let plan = parse_plan(plan_json)?;
    let report = validate(&plan, MAX_STITCH_MM, MAX_JUMP_MM);
    Ok(serde_json::json!({
        "ok": report.ok(),
        "errors": report.errors,
        "warnings": report.warnings,
    })
    .to_string())
}

pub fn score_plan_json(plan_json: &str) -> Result<String, String> {
    let plan = parse_plan(plan_json)?;
    serde_json::to_string(&score(&plan))
        .map_err(|error| format!("Could not encode the risk report ({error})."))
}

pub fn generate_project_json(project_json: &str) -> Result<String, String> {
    let project = parse_project(project_json).map_err(|error| error.to_string())?;
    let generation = generate_project(&project).map_err(|error| error.to_string())?;
    let dst_hex = if generation.readiness.engineering_export_allowed {
        let bytes = compile_to_vec(&generation.plan, MAX_STITCH_MM, MAX_JUMP_MM)
            .map_err(|error| error.to_string())?;
        Some(encode_hex(&bytes))
    } else {
        None
    };
    let risk = score(&generation.plan);
    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "revision": generation.revision,
        "readiness": generation.readiness,
        "validation": generation.validation,
        "provenance": generation.provenance,
        "plan": generation.plan,
        "dst_hex": dst_hex,
        "risk": risk,
    }))
    .map_err(|error| format!("Could not encode the generation result ({error})."))
}

pub fn capabilities_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "engine": "embroidery-core",
        "engine_version": "0.1.0",
        "capabilities": capability_manifest(),
        "underlay": underlay_rules(),
    }))
    .expect("capability manifest")
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn parse_plan(plan_json: &str) -> Result<StitchPlan, String> {
    serde_json::from_str(plan_json).map_err(|error| {
        format!(
            "Could not read the stitch plan JSON ({error}). Build the plan with the Rust generators from a region proposal; do not hand-write stitch records."
        )
    })
}

#[no_mangle]
pub extern "C" fn embroidery_engine_version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

/// # Safety
///
/// `plan_json` is either null or a readable NUL-terminated string that remains
/// valid for this call. `out_ptr`, `out_len`, and `out_cap` are either null or
/// writable. On status 0 or 1 the three outputs describe a buffer that the
/// caller owns and must release with [`embroidery_free`]. On status 2 they are
/// left untouched.
#[no_mangle]
pub unsafe extern "C" fn embroidery_compile_dst_json(
    plan_json: *const c_char,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    out_cap: *mut usize,
) -> c_int {
    dispatch(plan_json, out_ptr, out_len, out_cap, compile_plan_json)
}

/// # Safety
///
/// Same pointer contract as [`embroidery_compile_dst_json`]. A successful call
/// returns a JSON report in the output buffer.
#[no_mangle]
pub unsafe extern "C" fn embroidery_validate_json(
    plan_json: *const c_char,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    out_cap: *mut usize,
) -> c_int {
    dispatch(plan_json, out_ptr, out_len, out_cap, |json| {
        validate_plan_json(json).map(String::into_bytes)
    })
}

/// # Safety
///
/// Same pointer contract as [`embroidery_compile_dst_json`]. A successful call
/// returns a JSON risk report in the output buffer.
#[no_mangle]
pub unsafe extern "C" fn embroidery_score_json(
    plan_json: *const c_char,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    out_cap: *mut usize,
) -> c_int {
    dispatch(plan_json, out_ptr, out_len, out_cap, |json| {
        score_plan_json(json).map(String::into_bytes)
    })
}

/// # Safety
///
/// `project_json` is either null or a readable NUL-terminated string that
/// remains valid for this call. The output pointers follow the same contract
/// as [`embroidery_compile_dst_json`]. Success is a JSON generation result.
/// Failure is a UTF-8 message with no trailing NUL.
#[no_mangle]
pub unsafe extern "C" fn embroidery_generate_project_json(
    project_json: *const c_char,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    out_cap: *mut usize,
) -> c_int {
    dispatch(project_json, out_ptr, out_len, out_cap, |json| {
        generate_project_json(json).map(String::into_bytes)
    })
}

/// # Safety
///
/// `out_ptr`, `out_len`, and `out_cap` are either null or writable. On status
/// 0 they describe a JSON capability manifest the caller must release with
/// [`embroidery_free`]. On status 2 they are left untouched.
#[no_mangle]
pub unsafe extern "C" fn embroidery_capabilities_json(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    out_cap: *mut usize,
) -> c_int {
    if out_ptr.is_null() || out_len.is_null() || out_cap.is_null() {
        return 2;
    }
    write_out(out_ptr, out_len, out_cap, capabilities_json().into_bytes());
    0
}

/// # Safety
///
/// `ptr` is null, which does nothing, or a pointer previously returned through
/// `out_ptr` by one of the JSON entry points. `len` and `cap` are the values
/// returned with that pointer. The pointer is freed once and must not be used
/// again.
#[no_mangle]
pub unsafe extern "C" fn embroidery_free(ptr: *mut u8, len: usize, cap: usize) {
    if ptr.is_null() {
        return;
    }
    drop(Vec::from_raw_parts(ptr, len, cap));
}

unsafe fn dispatch(
    plan_json: *const c_char,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    out_cap: *mut usize,
    call: impl FnOnce(&str) -> Result<Vec<u8>, String>,
) -> c_int {
    if plan_json.is_null() || out_ptr.is_null() || out_len.is_null() || out_cap.is_null() {
        return 2;
    }
    let text = match CStr::from_ptr(plan_json).to_str() {
        Ok(text) => text,
        Err(_) => {
            write_out(
                out_ptr,
                out_len,
                out_cap,
                b"Plan JSON must be UTF-8.".to_vec(),
            );
            return 1;
        }
    };
    match call(text) {
        Ok(bytes) => {
            write_out(out_ptr, out_len, out_cap, bytes);
            0
        }
        Err(message) => {
            write_out(out_ptr, out_len, out_cap, message.into_bytes());
            1
        }
    }
}

unsafe fn write_out(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    out_cap: *mut usize,
    bytes: Vec<u8>,
) {
    let mut bytes = std::mem::ManuallyDrop::new(bytes);
    *out_ptr = bytes.as_mut_ptr();
    *out_len = bytes.len();
    *out_cap = bytes.capacity();
}

#[cfg(feature = "wasm")]
mod wasm_api {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(js_name = compileDst)]
    pub fn compile_dst(plan_json: &str) -> Result<Vec<u8>, JsValue> {
        super::compile_plan_json(plan_json).map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = validatePlan)]
    pub fn validate_plan(plan_json: &str) -> Result<String, JsValue> {
        super::validate_plan_json(plan_json).map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = scorePlan)]
    pub fn score_plan(plan_json: &str) -> Result<String, JsValue> {
        super::score_plan_json(plan_json).map_err(|error| JsValue::from_str(&error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    const SQUARE: &str = r#"{
        "name": "Square",
        "stitches": [
            {
                "kind": "running",
                "from": {"x": 0.0, "y": 0.0},
                "to": {"x": 10.0, "y": 0.0},
                "color_index": 0,
                "object_id": 1
            }
        ],
        "colors": [],
        "metadata": {}
    }"#;

    #[test]
    fn c_api_compiles_and_rejects_bad_json() {
        let json = CString::new(SQUARE).unwrap();
        let mut ptr = std::ptr::null_mut();
        let mut len = 0usize;
        let mut cap = 0usize;
        let code =
            unsafe { embroidery_compile_dst_json(json.as_ptr(), &mut ptr, &mut len, &mut cap) };
        assert_eq!(code, 0);
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        assert!(bytes.starts_with(b"LA:Square"));
        assert_eq!(bytes.len(), 512 + 6);
        unsafe { embroidery_free(ptr, len, cap) };

        let bad = CString::new("{").unwrap();
        let code =
            unsafe { embroidery_compile_dst_json(bad.as_ptr(), &mut ptr, &mut len, &mut cap) };
        assert_eq!(code, 1);
        let message = unsafe { std::slice::from_raw_parts(ptr, len) };
        let message = std::str::from_utf8(message).unwrap();
        assert!(message.contains("stitch plan JSON"));
        unsafe { embroidery_free(ptr, len, cap) };
    }

    #[test]
    fn c_api_validate_reports_an_empty_plan() {
        let json =
            CString::new(r#"{"name":"Empty","stitches":[],"colors":[],"metadata":{}}"#).unwrap();
        let mut ptr = std::ptr::null_mut();
        let mut len = 0usize;
        let mut cap = 0usize;
        let code = unsafe { embroidery_validate_json(json.as_ptr(), &mut ptr, &mut len, &mut cap) };
        assert_eq!(code, 0);
        let report: serde_json::Value =
            serde_json::from_slice(unsafe { std::slice::from_raw_parts(ptr, len) }).unwrap();
        assert_eq!(report["ok"], false);
        unsafe { embroidery_free(ptr, len, cap) };
    }

    #[test]
    fn generate_project_exports_dst_and_rejects_motif() {
        let project = r#"{
            "format": "embroidery-project",
            "format_version": "1",
            "name": "Line",
            "hoop_width_mm": 120.0,
            "hoop_height_mm": 100.0,
            "objects": [{
                "object_id": 1,
                "label": "Outline",
                "kind": "running",
                "points": [{"x": 10.0, "y": 20.0}, {"x": 40.0, "y": 20.0}],
                "angle_degrees": 0.0,
                "density_mm": 2.5,
                "column_width_mm": 2.0,
                "max_stitch_mm": 2.5,
                "underlay_spacing_mm": 0.0,
                "underlay": [],
                "pull_compensation_mm": 0.0,
                "color_index": 0
            }],
            "review": null
        }"#;
        let json = CString::new(project).unwrap();
        let mut ptr = std::ptr::null_mut();
        let mut len = 0usize;
        let mut cap = 0usize;
        let code = unsafe {
            embroidery_generate_project_json(json.as_ptr(), &mut ptr, &mut len, &mut cap)
        };
        assert_eq!(code, 0);
        let body = unsafe { std::slice::from_raw_parts(ptr, len) };
        let value: serde_json::Value = serde_json::from_slice(body).unwrap();
        assert_eq!(value["readiness"]["machine_release_allowed"], false);
        assert!(value["dst_hex"].as_str().unwrap().starts_with("4c41"));
        unsafe { embroidery_free(ptr, len, cap) };

        let motif = project.replace("\"running\"", "\"motif\"");
        let json = CString::new(motif).unwrap();
        let code = unsafe {
            embroidery_generate_project_json(json.as_ptr(), &mut ptr, &mut len, &mut cap)
        };
        assert_eq!(code, 1);
        let message = unsafe { std::slice::from_raw_parts(ptr, len) };
        let message = std::str::from_utf8(message).unwrap();
        assert!(message.contains("unavailable"));
        unsafe { embroidery_free(ptr, len, cap) };
    }

    #[test]
    fn version_is_readable() {
        let version = unsafe { std::ffi::CStr::from_ptr(embroidery_engine_version()) };
        assert_eq!(version.to_str().unwrap(), "0.1.0");
    }
}
