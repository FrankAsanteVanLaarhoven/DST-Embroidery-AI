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
fn the_browser_does_not_plan_stitches() {
    let source = fs::read_to_string(repo_root().join("web/studio.js")).unwrap();
    assert!(source.contains("/api/generate"));
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
    }
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
