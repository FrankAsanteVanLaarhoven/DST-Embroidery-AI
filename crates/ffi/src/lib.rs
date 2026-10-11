//! C ABI for the embroidery engine.
//!
//! Callers pass a JSON [`embroidery_core::StitchPlan`] and receive DST bytes
//! or a JSON report. The same entry points are exported to WASM when the
//! `wasm` feature is enabled. Python research code should call this library.
//! It should not plan stitches itself.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use embroidery_core::{validate, StitchPlan};
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
    fn version_is_readable() {
        let version = unsafe { std::ffi::CStr::from_ptr(embroidery_engine_version()) };
        assert_eq!(version.to_str().unwrap(), "0.1.0");
    }
}
