//! Shared STEP writer / parse helpers for freeform NURBS interchange (A1–A4).

use nurbs_core::{Error, Result, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_STEP_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;

mod refuse;
mod validate_curve_pcurve_correspondence;
mod invert_uv;
pub(crate) use refuse::*;
pub(crate) use validate_curve_pcurve_correspondence::*;
pub(crate) use invert_uv::*;
