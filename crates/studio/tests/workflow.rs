use std::fs;
use std::io::Cursor;

use embroidery_core::{generate_project, mm_to_units, parse_project};
use embroidery_dst::read_dst;
use embroidery_ffi::generate_project_json;
use embroidery_studio::{decode_hex, handle_http, repo_root, route};
use serde_json::Value;

#[test]
fn twelve_fixtures_cover_generation_and_dst_readback() {
    let root = repo_root().join("fixtures/projects");
    let manifest: Vec<Value> =
        serde_json::from_str(&fs::read_to_string(root.join("manifest.json")).unwrap()).unwrap();
    assert!(
        manifest.len() >= 12,
        "the milestone needs at least 12 declared fixtures"
    );
    let mut kinds = Vec::new();
    for entry in &manifest {
        let name = entry["file"].as_str().unwrap();
        let text = fs::read_to_string(root.join(name)).unwrap();
        let expect = entry["expect"].as_str().unwrap();
        if expect == "ok" {
            let project = parse_project(&text).unwrap();
            kinds.push(project.objects[0].kind.clone());
            let generation = generate_project(&project).unwrap();
            assert!(
                generation.validation.ok(),
                "{name} {}",
                generation.validation.errors.join("; ")
            );
            assert!(generation.readiness.engineering_export_allowed);
            assert!(!generation.readiness.machine_release_allowed);
            let response: Value =
                serde_json::from_str(&generate_project_json(&text).unwrap()).unwrap();
            let bytes = decode_hex(response["dst_hex"].as_str().unwrap()).unwrap();
            assert!(bytes.starts_with(b"LA:"));
            let read = read_dst(Cursor::new(bytes)).unwrap();
            assert!(read.sewing_stitch_count() > 0, "{name}");
            let expected = generation.plan.stitches.last().unwrap().to;
            let actual = read.stitches.last().unwrap().to;
            assert_eq!(
                mm_to_units(expected.x).unwrap(),
                mm_to_units(actual.x).unwrap()
            );
            assert_eq!(
                mm_to_units(expected.y).unwrap(),
                mm_to_units(actual.y).unwrap()
            );
        } else {
            let error = generate_project_json(&text).unwrap_err();
            let needle = entry["contains"].as_str().unwrap();
            assert!(
                error.contains(needle),
                "{name} error {error} did not contain {needle}"
            );
        }
    }
    assert!(kinds.iter().any(|kind| kind == "running"));
    assert!(kinds.iter().any(|kind| kind == "satin"));
    assert!(kinds.iter().any(|kind| kind == "fill"));
}

#[test]
fn club_mark_changes_colour_and_http_round_trips() {
    let path = repo_root().join("fixtures/projects/08_club_mark.json");
    let text = fs::read_to_string(path).unwrap();
    let project = parse_project(&text).unwrap();
    let generation = generate_project(&project).unwrap();
    assert!(generation
        .plan
        .stitches
        .iter()
        .any(|stitch| stitch.kind == embroidery_core::StitchKind::ColorChange));

    let request = format!(
        "POST /api/generate HTTP/1.1\r\nContent-Length: {}\r\n\r\n{text}",
        text.len()
    );
    let response = String::from_utf8(handle_http(request.as_bytes())).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("\"machine_release_allowed\":false"));
}

#[test]
fn library_lists_alpine_and_refuses_other_paths() {
    let listed = route("GET", "/api/library", b"");
    assert_eq!(listed.status, 200);
    let listed = String::from_utf8(listed.body).unwrap();
    assert!(listed.contains("13_alpine_club.json"));
    assert!(listed.contains("Alpine Club"));
    let opened = route("GET", "/api/projects/13_alpine_club.json", b"");
    assert_eq!(opened.status, 200);
    let opened = String::from_utf8(opened.body).unwrap();
    assert!(
        opened.contains("\"name\": \"Alpine Club\"") || opened.contains("\"name\":\"Alpine Club\"")
    );
    for blocked in [
        "/api/projects/../Cargo.toml",
        "/api/projects/..%2FCargo.toml",
        "/api/projects/not-a-design.json",
    ] {
        let response = route("GET", blocked, b"");
        assert_eq!(response.status, 404, "{blocked}");
    }
}

#[test]
fn unknown_page_is_a_branded_404() {
    let response = route("GET", "/missing", b"");
    assert_eq!(response.status, 404);
    let body = String::from_utf8(response.body).unwrap();
    assert!(body.contains("Page not found"));
    assert!(body.contains("Back to the design studio"));
    let home = route("GET", "/", b"");
    assert_eq!(home.status, 200);
    let home = String::from_utf8(home.body).unwrap();
    assert!(home.contains("Design studio"));
    assert!(home.contains("EmbroideryOS"));
}

#[test]
fn a_geometry_proposal_shape_generates_without_release() {
    let text = r#"{"format":"embroidery-project","format_version":"1","name":"Trial square","hoop_width_mm":80.0,"hoop_height_mm":80.0,"objects":[{"object_id":1,"label":"Field","kind":"fill","points":[{"x":10.0,"y":10.0},{"x":60.0,"y":10.0},{"x":60.0,"y":60.0},{"x":10.0,"y":60.0}],"angle_degrees":15.0,"density_mm":1.2,"column_width_mm":2.0,"max_stitch_mm":3.0,"underlay_spacing_mm":0.0,"underlay":[],"pull_compensation_mm":0.0,"color_index":0}],"review":null}"#;
    let project = parse_project(text).unwrap();
    assert!(project.review.is_none());
    let generation = generate_project(&project).unwrap();
    assert!(
        generation.validation.ok(),
        "{}",
        generation.validation.errors.join("; ")
    );
    assert!(!generation.readiness.machine_release_allowed);
    assert!(generation.plan.sewing_stitch_count() > 0);
}

#[test]
fn an_empty_proposal_does_not_call_the_model() {
    let response = route(
        "POST",
        "/api/propose",
        br#"{"prompt":"   ","image_data_url":""}"#,
    );
    assert_eq!(response.status, 400);
    let body = String::from_utf8(response.body).unwrap();
    assert!(body.contains("The design was not changed"));
    assert!(!body.contains("dst_hex"));
    assert!(!body.contains("stitches"));

    let image = route(
        "POST",
        "/api/propose",
        br#"{"prompt":"a badge","image_data_url":"data:image/gif;base64,AAAA"}"#,
    );
    assert_eq!(image.status, 400);
    let image = String::from_utf8(image.body).unwrap();
    assert!(image.contains("JPEG or PNG"));
}

#[test]
fn svg_import_previews_geometry_and_rejects_text() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="30mm" viewBox="0 0 40 30"><rect x="5" y="5" width="20" height="10" fill="#1e4d3c"/></svg>"##;
    let payload = serde_json::json!({
        "svg_text": svg,
        "name": "Trial rect",
        "width_mm": null,
        "running_strokes": false,
    });
    let bytes = serde_json::to_vec(&payload).unwrap();
    let response = route("POST", "/api/import-svg", &bytes);
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["ok"], true);
    assert!(body.get("dst_hex").is_none());
    assert!(body.get("stitches").is_none());
    assert!(body["project"].get("machine_release_allowed").is_none());
    assert_eq!(body["project"]["review"], Value::Null);
    assert_eq!(body["project"]["name"], "Trial rect");
    assert_eq!(body["manifest"]["machine_release_allowed"], false);
    assert_eq!(body["manifest"]["approval_allowed"], false);
    assert_eq!(
        body["manifest"]["not_interchangeable_with"],
        "twinlab-generic-flat-120-v1"
    );
    assert_eq!(body["manifest"]["timing_model"], Value::Null);
    assert_eq!(
        body["manifest"]["coordinate_frame"],
        "native-hoop-nonnegative-v1"
    );
    let summary = body["summary"].as_str().unwrap();
    assert!(summary.contains("No stitches were generated"));
    assert!(summary.contains("does not approve itself"));
    assert!(summary.contains("No curves were flattened"));
    let project = parse_project(&body["project"].to_string()).unwrap();
    assert!(project.review.is_none());
    assert_eq!(project.objects.len(), 1);
    assert_eq!(project.objects[0].kind, "fill");
    assert!(project.objects[0].holes.is_empty());
    let home = String::from_utf8(route("GET", "/", b"").body).unwrap();
    assert!(home.contains("id=\"svg-form\""));
    assert!(home.contains("Preview import"));

    let text = serde_json::json!({
        "svg_text": r#"<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="40mm" viewBox="0 0 40 40"><text x="4" y="20">Hi</text></svg>"#,
    });
    let rejected = route(
        "POST",
        "/api/import-svg",
        &serde_json::to_vec(&text).unwrap(),
    );
    assert_eq!(rejected.status, 422);
    let rejected = String::from_utf8(rejected.body).unwrap();
    assert!(rejected.contains("text"));
    assert!(rejected.contains("The design was not changed"));
    assert!(!rejected.contains("dst_hex"));

    let empty = route("POST", "/api/import-svg", br#"{"svg_text":"   "}"#);
    assert_eq!(empty.status, 400);
    let bad_width = route(
        "POST",
        "/api/import-svg",
        br#"{"svg_text":"<svg></svg>","width_mm":"wide"}"#,
    );
    assert_eq!(bad_width.status, 400);
}

#[test]
fn the_browser_does_not_plan_stitches() {
    let source = fs::read_to_string(repo_root().join("web/studio.js")).unwrap();
    let batch = fs::read_to_string(repo_root().join("web/batch.js")).unwrap();
    let lettering = fs::read_to_string(repo_root().join("web/lettering.js")).unwrap();
    let creative = fs::read_to_string(repo_root().join("web/creative.js")).unwrap();
    assert!(source.contains("/api/generate"));
    assert!(batch.contains("/api/batch/item"));
    assert!(
        source.len() <= 80 * 1024,
        "studio.js is {} bytes",
        source.len()
    );
    assert!(
        batch.len() <= 48 * 1024,
        "batch.js is {} bytes",
        batch.len()
    );
    for forbidden in [
        "generate_satin",
        "generate_fill",
        "resample_polyline",
        "encode_record",
        "scanline",
        "console.log",
        "console.debug",
    ] {
        assert!(
            !source.contains(forbidden),
            "{forbidden} found in studio.js"
        );
        assert!(!batch.contains(forbidden), "{forbidden} found in batch.js");
        assert!(
            !lettering.contains(forbidden),
            "{forbidden} found in lettering.js"
        );
        assert!(
            !creative.contains(forbidden),
            "{forbidden} found in creative.js"
        );
    }
    assert!(
        creative.len() <= 32 * 1024,
        "creative.js is {} bytes",
        creative.len()
    );
    assert!(creative.contains("/api/creative/jobs"));
    assert!(!creative.contains("/api/propose"));
}

#[test]
fn twinlab_replays_the_compiled_dst() {
    let path = repo_root().join("fixtures/projects/08_club_mark.json");
    let text = fs::read_to_string(path).unwrap();
    let request = format!(
        "POST /api/rehearse HTTP/1.1\r\nContent-Length: {}\r\n\r\n{text}",
        text.len()
    );
    let response = String::from_utf8(handle_http(request.as_bytes())).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let body = response.split("\r\n\r\n").nth(1).unwrap();
    let value: Value = serde_json::from_str(body).unwrap();
    assert_eq!(
        value["qualification"],
        "Virtual demo · Generic machine · No hardware connected · Physical quality not validated"
    );
    assert_eq!(value["machine_release_allowed"], false);
    assert_eq!(value["hardware_connected"], false);
    assert_eq!(value["physical_quality_validated"], false);
    assert!(value["trace"]["needle_down_count"].as_u64().unwrap() > 0);
    assert_eq!(value["trace"]["dst_sha256"].as_str().unwrap().len(), 64);
    assert!(value["pose_samples"].as_array().unwrap().len() >= 3);

    let page = route("GET", "/twinlab", b"");
    assert_eq!(page.status, 200);
    let html = String::from_utf8(page.body).unwrap();
    assert!(html.contains("Machine TwinLab"));
    assert!(html.contains("No hardware connected"));

    let source = fs::read_to_string(repo_root().join("web/twinlab.js")).unwrap();
    assert!(source.contains("/api/rehearse"));
    for forbidden in [
        "generate_satin",
        "generate_fill",
        "encode_record",
        "scanline",
        "console.log",
        "console.debug",
    ] {
        assert!(
            !source.contains(forbidden),
            "{forbidden} found in twinlab.js"
        );
    }
}
