//! Deterministic embroidery kernel.
//!
//! Geometry enters through the generators in this crate. Those generators are
//! the supported way to create [`Stitch`] records. A proposal from an external
//! model may choose object type, angle, density, and underlay. It does not
//! write stitch coordinates. File compilers live in `embroidery-dst`.

mod batch;
mod capabilities;
mod error;
pub mod generators;
pub mod geometry;
mod lettering;
mod lettering_layout;
mod lettering_v2;
pub mod model;
mod project;
mod sha256;
pub mod validate;

pub use batch::{
    generate_one, parse_batch_input, parse_name_csv, prepare_batch, run_batch, BatchCounts,
    BatchInput, BatchReport, BatchRow, ItemOutcome, PlacementMm, MAX_BATCH_ROWS,
    PROFILE_NATIVE_HOOP,
};
pub use capabilities::{
    capability_manifest, underlay_rules, Capability, CapabilityStatus, UnderlayRule,
};
pub use error::PlanError;
pub use generators::{
    generate_fill, generate_fill_with_holes, generate_running, generate_satin,
    generate_satin_variable, validate_fill_region, FillConfig, RunningConfig, SatinConfig,
};
pub use lettering::{
    font_content_sha256, font_recipe, generate_text, LineLayout, TextRequest, FONT_ID, FONT_NAME,
    FONT_VERSION, LINE_LAYOUT_POLICY, MAX_LINES, REPERTOIRE,
};

/// Checks an explicit line layout without sewing. Historical objects omit the field.
pub fn lettering_layout_error(object_id: u64, layout: &LineLayout) -> Result<(), String> {
    lettering_layout::validate_layout(object_id, layout)
        .map_err(|error| error.message().to_string())
}
pub use lettering_v2::{font_v2_content_sha256, font_v2_recipe, FONT_VERSION_2, REPERTOIRE_V2};
pub use model::{
    mm_to_units, units_to_mm, ColorStop, Point, Stitch, StitchKind, StitchPlan, ThreadSpec,
    DST_MAX_ABS_UNITS, DST_MAX_DELTA_UNITS, DST_MAX_STITCH_MM, DST_UNITS_PER_MM, DST_UNIT_MM,
};
pub use project::{
    generate_project, parse_project, readiness_for, revision_of, stamp_geometry_review,
    DesignObject, Generation, GeometryReview, ProjectDocument, Provenance, Readiness, TextIntent,
};
pub use sha256::sha256_hex;
pub use validate::{validate, ValidationReport};
