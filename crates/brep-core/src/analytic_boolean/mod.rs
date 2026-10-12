//! Analytic curved Boolean gated by aggregate `BooleanCertificate` (G5e close).
//!
//! Closed matrix: finite cylinders/tubes/frustums and spheres. Topology is
//! authored only by `imprint_pipeline` empty-algebra or sphere imprint —
//! **never** by `prismatic_boolean`, mesh, or Manifold.

use crate::Model;
use nurbs_intersect::analytic_ss::{
    AnalyticSsComponent, FiniteCone, FiniteCylinder, cone_cone, cylinder_cylinder, cylinder_sphere,
    plane_cylinder, torus_torus,
};
use crate::box_sphere_boolean;
use nurbs_intersect::coverage_verifier::verify_complete_report;
use crate::cylinder_sphere_boolean;
use crate::imprint_pipeline::{self, SpatialRelation};
use nurbs_intersect::{Coverage, Options, Plane, Report};
use crate::solid_audit::audit_solid;
use crate::sphere_boolean;
use crate::trim_sew::{
    CellLabel, ChartEvent, ChartKind, ClassificationCertificate, SewCertificate,
    classify_chart_events, classify_face_outer_loop, classify_imprint_circle_events,
};
use crate::uv_arrangement::{
    UvImprintCurve, arrange_imprint_curves, assert_missed_branch_detected,
};
use nurbs_core::{Error, Result};

mod boolean_certificate;
mod analytic_boolean;
#[cfg(test)]
mod tests;
pub use boolean_certificate::*;
pub use analytic_boolean::*;
