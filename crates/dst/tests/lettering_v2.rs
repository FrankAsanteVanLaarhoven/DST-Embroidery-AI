//! Version 2 canonical pairs share DST bytes and keep distinct source text.
use embroidery_core::{
    font_v2_content_sha256, generate_text, sha256_hex, LineLayout, TextRequest, FONT_ID,
    FONT_VERSION_2, LINE_LAYOUT_POLICY,
};
use embroidery_dst::compile_to_vec;

fn sew(content: &str) -> Vec<u8> {
    let hash = font_v2_content_sha256();
    let item = TextRequest {
        object_id: 3,
        content,
        font_id: FONT_ID,
        font_version: FONT_VERSION_2,
        font_sha256: &hash,
        origin_x_mm: 8.0,
        origin_y_mm: 8.0,
        cap_height_mm: 12.0,
        letter_spacing_mm: 1.0,
        word_spacing_mm: 4.0,
        density_mm: 1.5,
        column_width_mm: 1.92,
        pull_compensation_mm: 0.0,
        max_stitch_mm: 3.0,
        centre_run_mm: None,
        edge_run_mm: None,
        color_index: 0,
        min_x: 0.0,
        min_y: 0.0,
        max_x: 220.0,
        max_y: 80.0,
        layout: None,
    };
    let mut plan = embroidery_core::StitchPlan::new("letter");
    generate_text(&mut plan, &item).unwrap_or_else(|error| panic!("{content}: {error}"));
    assert_eq!(
        plan.metadata["lettering_text"],
        format!("object 3 {content}")
    );
    compile_to_vec(&plan, 12.1, 120.0).unwrap_or_else(|error| panic!("{content}: {error}"))
}

#[test]
fn canonical_pairs_share_dst_bytes_and_apostrophes_do_not() {
    let pairs = [
        ("Á", "A\u{0301}"),
        ("á", "a\u{0301}"),
        ("É", "E\u{0301}"),
        ("é", "e\u{0301}"),
        ("Í", "I\u{0301}"),
        ("í", "i\u{0301}"),
        ("Ó", "O\u{0301}"),
        ("ó", "o\u{0301}"),
        ("Ú", "U\u{0301}"),
        ("ú", "u\u{0301}"),
        ("Ä", "A\u{0308}"),
        ("ä", "a\u{0308}"),
        ("Ë", "E\u{0308}"),
        ("ë", "e\u{0308}"),
        ("Ï", "I\u{0308}"),
        ("ï", "i\u{0308}"),
        ("Ö", "O\u{0308}"),
        ("ö", "o\u{0308}"),
        ("Ü", "U\u{0308}"),
        ("ü", "u\u{0308}"),
        ("Ñ", "N\u{0303}"),
        ("ñ", "n\u{0303}"),
        ("Ç", "C\u{0327}"),
        ("ç", "c\u{0327}"),
        ("Café", "Cafe\u{0301}"),
    ];
    assert!(pairs.len() >= 12);
    for (composed, decomposed) in pairs {
        let left = sew(composed);
        let right = sew(decomposed);
        assert_eq!(left, right, "{composed}");
        assert_eq!(sha256_hex(&left), sha256_hex(&right));
    }
    assert_ne!(sew("O'Neil"), sew("O\u{2019}Neil"));
}

fn sew_lines(content: &str, alignment: &str) -> Vec<u8> {
    let hash = font_v2_content_sha256();
    let layout = LineLayout {
        policy: LINE_LAYOUT_POLICY.to_string(),
        alignment: alignment.to_string(),
        block_width_mm: 80.0,
        first_baseline_mm: 8.0 + 92.0 * 12.0 / 100.0,
        baseline_spacing_mm: 24.0,
    };
    let item = TextRequest {
        object_id: 3,
        content,
        font_id: FONT_ID,
        font_version: FONT_VERSION_2,
        font_sha256: &hash,
        origin_x_mm: 8.0,
        origin_y_mm: 8.0,
        cap_height_mm: 12.0,
        letter_spacing_mm: 1.0,
        word_spacing_mm: 4.0,
        density_mm: 1.5,
        column_width_mm: 1.92,
        pull_compensation_mm: 0.0,
        max_stitch_mm: 3.0,
        centre_run_mm: None,
        edge_run_mm: None,
        color_index: 0,
        min_x: 0.0,
        min_y: 0.0,
        max_x: 220.0,
        max_y: 160.0,
        layout: Some(&layout),
    };
    let mut plan = embroidery_core::StitchPlan::new("letter");
    generate_text(&mut plan, &item).unwrap_or_else(|error| panic!("{content}: {error}"));
    compile_to_vec(&plan, 12.1, 120.0).unwrap_or_else(|error| panic!("{content}: {error}"))
}

#[test]
fn repeated_multiline_layouts_produce_identical_dst_bytes() {
    let texts = [
        "Café\nAlpine 08",
        "AB",
        "AB\nA",
        "AB\nA\nABC",
        "AB\n\nA\nABC",
    ];
    assert_eq!(texts.len() * 3, 15);
    for text in texts {
        for alignment in ["left", "centre", "right"] {
            assert_eq!(
                sew_lines(text, alignment),
                sew_lines(text, alignment),
                "{alignment} {text}"
            );
        }
    }
    assert_eq!(sew("AB"), sew_lines("AB", "left"));
    assert_eq!(sew("Café"), sew_lines("Café", "left"));
    assert_ne!(sew("Café"), sew_lines("Café\nAlpine 08", "centre"));
}
