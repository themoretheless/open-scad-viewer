//! Analytic STEP AP214/AP242 interchange for constructor solids.
//!
//! Export writes a linked ADVANCED_FACE / MANIFOLD_SOLID_BREP graph with real
//! AXIS2 placements and CIRCLE ring edges. Import rebuilds via surface+AXIS2
//! recognition → constructors (never proprietary OSCAD_SOLID, never AABB
//! fallback for curved solids, never silent height defaults). FACETED / STL /
//! OBJ / incomplete graphs refuse.

use brep_core::analytic::{cylinder, frustum, sphere, torus, tube};
use brep_core::analytic_features::FeatureCertificate;
use brep_core::intersections::{recognize_cone, recognize_cylinder, recognize_sphere, recognize_torus};
use brep_core::transform;
use brep_core::{Model, TopoId, TopoKind, cuboid};
use nurbs_core::{Error, Result};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

mod step_identity_report;
mod step_writer;
mod export_step;
mod import_step_v2;
#[cfg(test)]
mod tests;
pub use step_identity_report::*;
use step_writer::*;
pub use export_step::*;
pub use import_step_v2::*;
