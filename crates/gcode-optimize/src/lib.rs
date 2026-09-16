//! Toolpath optimization for preview G-code. Not a slicer and not printer I/O.
//!
//! Passes run in a fixed order: simplify, seam, order/flip, combing, retract
//! policy. Emit stays on the print-preview dialect: travel is `G0`, never a
//! negative E.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]

mod comb;
mod from_gcode;
mod order;
mod pipeline;
mod retract;
mod seam;
mod simplify;
mod travel;

pub use from_gcode::{from_gcode, from_preview};
pub use pipeline::{
    emit_optimized, emit_optimized_3mf, emit_optimized_gcode_3mf_job, emit_optimized_job,
    from_planned, from_toolpaths, optimize,
};
pub use travel::{path_end, path_start, travel_after_path_mm, travel_mm};

pub use gcode_core::{
    emit, emit_3mf, emit_gcode_3mf_job, emit_job, parse, parse_job, Flavor, JobProfile,
    MachineProfile, MeshBody, PlannedLayer, PlannedPath, JOB_DIALECT, MAX_LAYERS, MAX_MOVES,
};
pub use math_core::{Error, Result};

pub const MAX_PLAN_WORK: usize = 64_000_000;
pub const MAX_COMB_WAYPOINTS: usize = 4_096;

pub(crate) fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SeamPrefer {
    #[default]
    Nearest,
    MinX,
    MinY,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptimizeSettings {
    pub simplify_tolerance_mm: f64,
    pub seam: SeamPrefer,
    pub max_2opt_swaps: usize,
    pub retract_min_travel_mm: f64,
}

impl Default for OptimizeSettings {
    fn default() -> Self {
        Self {
            simplify_tolerance_mm: 0.0,
            seam: SeamPrefer::Nearest,
            max_2opt_swaps: 256,
            retract_min_travel_mm: 2.0,
        }
    }
}

impl OptimizeSettings {
    pub fn validate(&self) -> Result<()> {
        if !self.simplify_tolerance_mm.is_finite() || self.simplify_tolerance_mm < 0.0 {
            return Err(invalid(
                "OPT_INVALID_SETTINGS",
                "simplify_tolerance_mm must be finite and non-negative",
            ));
        }
        if !self.retract_min_travel_mm.is_finite() || self.retract_min_travel_mm < 0.0 {
            return Err(invalid(
                "OPT_INVALID_SETTINGS",
                "retract_min_travel_mm must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OptimizeReport {
    pub travel_before_mm: f64,
    pub travel_after_mm: f64,
    pub paths_before: usize,
    pub paths_after: usize,
    pub simplified_vertices: usize,
    pub seam_rotations: usize,
    pub order_flips: usize,
    pub two_opt_swaps: usize,
    pub comb_waypoints: usize,
    pub uncombed_travels: usize,
    pub virtual_retracts: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptimizeInput {
    pub layers: Vec<PlannedLayer>,
    /// Optional closed contours per layer index, used only by combing.
    pub avoid: Vec<Vec<Vec<[f64; 2]>>>,
}

impl OptimizeInput {
    pub fn with_avoid(mut self, layer_index: usize, contours: Vec<Vec<[f64; 2]>>) -> Self {
        if self.avoid.len() <= layer_index {
            self.avoid.resize(layer_index + 1, Vec::new());
        }
        self.avoid[layer_index] = contours;
        self
    }
}

pub(crate) struct Budget {
    work: usize,
}

impl Budget {
    fn new() -> Self {
        Self { work: 0 }
    }

    fn add(&mut self, count: usize) -> Result<()> {
        self.work = self
            .work
            .checked_add(count)
            .ok_or_else(work_limit)?;
        if self.work > MAX_PLAN_WORK {
            return Err(work_limit());
        }
        Ok(())
    }
}

fn work_limit() -> Error {
    invalid(
        "OPT_WORK_LIMIT",
        "Toolpath optimization work budget exceeded",
    )
}
