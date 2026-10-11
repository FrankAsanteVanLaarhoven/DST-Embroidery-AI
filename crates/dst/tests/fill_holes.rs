//! Locked no-hole export, plus repeated DST bytes for one holed fill.
use embroidery_core::{
    generate_fill, generate_fill_with_holes, sha256_hex, FillConfig, Point, StitchPlan,
};
use embroidery_dst::compile_to_vec;

fn square() -> Vec<Point> {
    vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
    ]
}

fn no_hole_config() -> FillConfig {
    FillConfig {
        angle_degrees: 0.0,
        row_spacing_mm: 2.0,
        max_stitch_length_mm: 5.0,
        underlay_spacing_mm: 0.0,
    }
}

#[test]
fn no_hole_square_dst_matches_the_pre_change_bytes() {
    let mut plan = StitchPlan::new("fill");
    generate_fill(&mut plan, 4, 2, &square(), &no_hole_config()).unwrap();
    let first = compile_to_vec(&plan, 12.1, 120.0).unwrap();
    let second = compile_to_vec(&plan, 12.1, 120.0).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 560);
    assert_eq!(
        sha256_hex(&first),
        "5d74efde4d1d1d74210bab965640c72291136761a9fc5d14f867182f0865a171"
    );
}

#[test]
fn holed_fill_dst_is_repeatable_and_differs_from_the_solid_square() {
    let mut solid = StitchPlan::new("solid");
    let mut holed = StitchPlan::new("holed");
    let hole = vec![
        Point::new(3.0, 3.0),
        Point::new(7.0, 3.0),
        Point::new(7.0, 7.0),
        Point::new(3.0, 7.0),
    ];
    generate_fill(&mut solid, 1, 0, &square(), &no_hole_config()).unwrap();
    generate_fill_with_holes(&mut holed, 1, 0, &square(), &[hole], &no_hole_config()).unwrap();
    let solid_bytes = compile_to_vec(&solid, 12.1, 120.0).unwrap();
    let first = compile_to_vec(&holed, 12.1, 120.0).unwrap();
    let second = compile_to_vec(&holed, 12.1, 120.0).unwrap();
    assert_eq!(first, second);
    assert_ne!(first, solid_bytes);
    println!(
        "HOLE_DST_SHA256 {} bytes {}",
        sha256_hex(&first),
        first.len()
    );
}
