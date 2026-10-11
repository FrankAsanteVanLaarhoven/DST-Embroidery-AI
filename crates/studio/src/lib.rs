//! Loopback HTTP routes for the design studio.
//!
//! The process binds to 127.0.0.1 only. It forwards project documents to the
//! Rust generators and returns their stitch plans. It does not plan stitches
//! itself, and it never marks a file as released to a machine.

use std::io::Cursor;
use std::path::PathBuf;

use embroidery_core::{parse_project, stamp_geometry_review};
use embroidery_dst::read_dst;
use embroidery_ffi::{capabilities_json, generate_project_json};
use embroidery_motion::rehearse_project_json;
use serde_json::json;

const NOT_FOUND_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<meta name="description" content="That address is not part of the local EmbroideryOS design studio.">
<title>Page not found — EmbroideryOS</title>
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="stylesheet" href="/studio.css">
</head>
<body>
<main class="missing">
<h1>Page not found</h1>
<p>That address is not part of the local design studio.</p>
<p><a href="/">Back to the design studio</a></p>
</main>
</body>
</html>
"#;

pub struct HttpResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn handle_http(raw: &[u8]) -> Vec<u8> {
    encode(match split_request(raw) {
        Ok((method, path, body)) => route(&method, &path, body),
        Err(response) => response,
    })
}

pub fn route(method: &str, path: &str, body: &[u8]) -> HttpResponse {
    let path = path.split('?').next().unwrap_or(path);
    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => {
            file_response("text/html; charset=utf-8", "index.html")
        }
        ("GET", "/studio.css") => file_response("text/css; charset=utf-8", "studio.css"),
        ("GET", "/studio.js") => file_response("text/javascript; charset=utf-8", "studio.js"),
        ("GET", "/favicon.svg") => file_response("image/svg+xml", "favicon.svg"),
        ("GET", "/twinlab") | ("GET", "/twinlab.html") => {
            file_response("text/html; charset=utf-8", "twinlab.html")
        }
        ("GET", "/twinlab.css") => file_response("text/css; charset=utf-8", "twinlab.css"),
        ("GET", "/twinlab.js") => file_response("text/javascript; charset=utf-8", "twinlab.js"),
        ("GET", "/api/capabilities") => json_ok(capabilities_json().into_bytes()),
        ("GET", "/api/sample") => sample_response(),
        ("POST", "/api/generate") => generate_response(body),
        ("POST", "/api/review") => review_response(body),
        ("POST", "/api/rehearse") => rehearse_response(body),
        ("POST", "/api/verify") => verify_response(body),
        ("GET", _) => html(404, NOT_FOUND_HTML.as_bytes()),
        _ => json_error(
            404,
            "That API route does not exist. Use the design studio at /.",
        ),
    }
}

fn generate_response(body: &[u8]) -> HttpResponse {
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(_) => {
            return json_error(
                400,
                "The project body is not UTF-8. Save the project as JSON and generate again.",
            );
        }
    };
    match generate_project_json(text) {
        Ok(json) => json_ok(json.into_bytes()),
        Err(message) => json_error(422, &message),
    }
}

fn rehearse_response(body: &[u8]) -> HttpResponse {
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(_) => {
            return json_error(
                400,
                "The rehearsal body is not UTF-8. Save the project as JSON and rehearse again.",
            );
        }
    };
    match rehearse_project_json(text) {
        Ok(json) => json_ok(json.into_bytes()),
        Err(message) => json_error(422, &message),
    }
}

fn review_response(body: &[u8]) -> HttpResponse {
    let value: serde_json::Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return json_error(
                400,
                "The review body is not JSON. Send the current project and a reviewer name.",
            );
        }
    };
    let Some(project_value) = value.get("project") else {
        return json_error(
            400,
            "The review request needs a project. Generate the design, then record the geometry review.",
        );
    };
    let project_json = match serde_json::to_string(project_value) {
        Ok(json) => json,
        Err(_) => {
            return json_error(
                400,
                "The project could not be read. Reload the studio and record the review again.",
            );
        }
    };
    let mut project = match parse_project(&project_json) {
        Ok(project) => project,
        Err(error) => return json_error(422, error.message()),
    };
    let reviewer = value
        .get("reviewer")
        .and_then(|item| item.as_str())
        .unwrap_or("");
    let revision = match stamp_geometry_review(&mut project, reviewer) {
        Ok(revision) => revision,
        Err(error) => return json_error(422, error.message()),
    };
    json_ok(
        json!({
            "ok": true,
            "revision": revision,
            "project": project,
        })
        .to_string()
        .into_bytes(),
    )
}

fn verify_response(body: &[u8]) -> HttpResponse {
    let value: serde_json::Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return json_error(
                400,
                "The verify body is not JSON. Export the design again and verify that file.",
            );
        }
    };
    let Some(hex) = value.get("dst_hex").and_then(|item| item.as_str()) else {
        return json_error(
            400,
            "The verify request needs dst_hex. Export the design again and verify that file.",
        );
    };
    let bytes = match decode_hex(hex) {
        Ok(bytes) => bytes,
        Err(message) => return json_error(400, &message),
    };
    let plan = match read_dst(Cursor::new(bytes)) {
        Ok(plan) => plan,
        Err(error) => return json_error(422, &error.to_string()),
    };
    let last = plan.stitches.last().map(|stitch| {
        json!({
            "kind": stitch.kind,
            "x": stitch.to.x,
            "y": stitch.to.y,
        })
    });
    json_ok(
        json!({
            "ok": true,
            "name": plan.name,
            "stitch_count": plan.stitches.len(),
            "sewing_stitch_count": plan.sewing_stitch_count(),
            "last": last,
            "note": "DST does not store satin or fill. Needle-down records are read back as running stitches. This check does not mean a machine accepted the file.",
        })
        .to_string()
        .into_bytes(),
    )
}

fn sample_response() -> HttpResponse {
    match std::fs::read(repo_root().join("fixtures/projects/08_club_mark.json")) {
        Ok(bytes) => json_ok(bytes),
        Err(_) => json_error(
            500,
            "The sample project is missing from fixtures/projects. Restore 08_club_mark.json and start the studio again.",
        ),
    }
}

fn file_response(content_type: &'static str, name: &str) -> HttpResponse {
    let path = repo_root().join("web").join(name);
    match std::fs::read(&path) {
        Ok(bytes) => HttpResponse {
            status: 200,
            content_type,
            body: bytes,
        },
        Err(_) => html(404, NOT_FOUND_HTML.as_bytes()),
    }
}

fn split_request(raw: &[u8]) -> Result<(String, String, &[u8]), HttpResponse> {
    let header_end = raw.windows(4).position(|window| window == b"\r\n\r\n");
    let Some(header_end) = header_end else {
        return Err(json_error(
            400,
            "The request did not include a complete header. Send it again.",
        ));
    };
    let header = match std::str::from_utf8(&raw[..header_end]) {
        Ok(header) => header,
        Err(_) => {
            return Err(json_error(
                400,
                "The request header is not UTF-8. Send the project as JSON.",
            ));
        }
    };
    let mut lines = header.lines();
    let Some(start) = lines.next() else {
        return Err(json_error(
            400,
            "The request has no start line. Send it again.",
        ));
    };
    let mut parts = start.split_whitespace();
    let (Some(method), Some(path)) = (parts.next(), parts.next()) else {
        return Err(json_error(
            400,
            "The request start line is not a method and path. Send it again.",
        ));
    };
    let mut length = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            length = match value.trim().parse() {
                Ok(length) => length,
                Err(_) => {
                    return Err(json_error(
                        400,
                        "Content-Length is not a number. Send the project again.",
                    ));
                }
            };
        }
    }
    if length > 1_048_576 {
        return Err(json_error(
            413,
            "The project is larger than 1 MB. Split it and generate again.",
        ));
    }
    let body = &raw[header_end + 4..];
    if body.len() < length {
        return Err(json_error(
            400,
            "The request ended before the body arrived. Send the project again.",
        ));
    }
    Ok((method.to_string(), path.to_string(), &body[..length]))
}

pub fn encode(response: HttpResponse) -> Vec<u8> {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        500 => "Internal Server Error",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
    );
    let mut out = head.into_bytes();
    out.extend_from_slice(&response.body);
    out
}

fn json_ok(body: Vec<u8>) -> HttpResponse {
    HttpResponse {
        status: 200,
        content_type: "application/json; charset=utf-8",
        body,
    }
}

fn json_error(status: u16, message: &str) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json; charset=utf-8",
        body: json!({ "ok": false, "error": message })
            .to_string()
            .into_bytes(),
    }
}

fn html(status: u16, body: &[u8]) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "text/html; charset=utf-8",
        body: body.to_vec(),
    }
}

pub fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    let text = text.trim();
    if text.is_empty() || text.len() % 2 != 0 {
        return Err(
            "DST hex is incomplete. Export the design again and verify that file.".to_string(),
        );
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut index = 0;
    while index < bytes.len() {
        let high = hex_value(bytes[index])?;
        let low = hex_value(bytes[index + 1])?;
        out.push((high << 4) | low);
        index += 2;
    }
    Ok(out)
}

fn hex_value(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(
            "DST hex contains a character that is not hexadecimal. Export the design again and verify that file."
                .to_string(),
        ),
    }
}
