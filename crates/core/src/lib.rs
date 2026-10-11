//! Deterministic embroidery kernel.
//!
//! Geometry enters through the generators in this crate. Those generators are
//! the supported way to create [`Stitch`] records. A proposal from an external
//! model may choose object type, angle, density, and underlay. It does not
//! write stitch coordinates. File compilers live in `embroidery-dst`.

mod capabilities;
mod error;
pub mod generators;
pub mod geometry;
pub mod model;
mod project;
mod sha256;
pub mod validate;

pub use capabilities::{
    capability_manifest, underlay_rules, Capability, CapabilityStatus, UnderlayRule,
};
pub use error::PlanError;
pub use generators::{
    generate_fill, generate_fill_with_holes, generate_running, generate_satin,
    validate_fill_region, FillConfig, RunningConfig, SatinConfig,
};
pub use model::{
    mm_to_units, units_to_mm, ColorStop, Point, Stitch, StitchKind, StitchPlan, ThreadSpec,
    DST_MAX_ABS_UNITS, DST_MAX_DELTA_UNITS, DST_MAX_STITCH_MM, DST_UNITS_PER_MM, DST_UNIT_MM,
};
pub use project::{
    generate_project, parse_project, readiness_for, revision_of, stamp_geometry_review,
    DesignObject, Generation, GeometryReview, ProjectDocument, Provenance, Readiness,
};
pub use sha256::sha256_hex;
pub use validate::{validate, ValidationReport};
