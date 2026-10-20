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
    Flavor, JOB_DIALECT, JobProfile, MAX_LAYERS, MAX_MOVES, MachineProfile, MeshBody, PlannedLayer,
    PlannedPath, emit, emit_3mf, emit_gcode_3mf_job, emit_job, parse, parse_job,
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
        self.work = self.work.checked_add(count).ok_or_else(work_limit)?;
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

/// Configuration for circular arc fitting (`G2` / `G3` ArcWelder).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcWelderConfig {
    /// Maximum radial deviation (in mm) allowed between intermediate polyline vertices and the fitted circle.
    pub tolerance_mm: f64,
    /// Minimum number of consecutive points required to form a circular arc.
    pub min_arc_points: usize,
    /// Maximum allowed circle radius in mm (prevents near-collinear segments from producing huge radii).
    pub max_radius_mm: f64,
}

impl Default for ArcWelderConfig {
    fn default() -> Self {
        Self {
            tolerance_mm: 0.05,
            min_arc_points: 4,
            max_radius_mm: 1_000.0,
        }
    }
}

/// A 2D toolpath segment after `G1` / `G2` / `G3` arc welding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FittedSegment {
    /// Linear move (`G1`) from `start` to `end`.
    Linear { start: [f64; 2], end: [f64; 2] },
    /// Circular arc (`G2` if `clockwise`, `G3` if counter-clockwise) from `start` to `end`
    /// with center offset `ij_offset = [cx - start_x, cy - start_y]` (`I`, `J` parameters).
    Arc {
        start: [f64; 2],
        end: [f64; 2],
        ij_offset: [f64; 2],
        radius: f64,
        clockwise: bool,
    },
}

/// Configuration for retraction and Z-hop vertical lift across long non-extruding travel moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetractZHopConfig {
    pub min_travel_mm: f64,
    pub retract_length_mm: f64,
    pub retract_speed_mm_s: f64,
    pub z_hop_mm: f64,
}

impl Default for RetractZHopConfig {
    fn default() -> Self {
        Self {
            min_travel_mm: 2.0,
            retract_length_mm: 0.8,
            retract_speed_mm_s: 35.0,
            z_hop_mm: 0.2,
        }
    }
}

fn circumcircle(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Option<([f64; 2], f64, bool)> {
    let d = 2.0 * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-9 {
        return None;
    }
    let a2 = a[0] * a[0] + a[1] * a[1];
    let b2 = b[0] * b[0] + b[1] * b[1];
    let c2 = c[0] * c[0] + c[1] * c[1];
    let ux = (a2 * (b[1] - c[1]) + b2 * (c[1] - a[1]) + c2 * (a[1] - b[1])) / d;
    let uy = (a2 * (c[0] - b[0]) + b2 * (a[0] - c[0]) + c2 * (b[0] - a[0])) / d;
    let r = (a[0] - ux).hypot(a[1] - uy);
    if !r.is_finite() || r <= 1e-6 {
        return None;
    }
    // Cross product of (b - a) x (c - b): negative => clockwise (G2), positive => CCW (G3)
    let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
    Some(([ux, uy], r, cross < 0.0))
}

/// Welds chains of short `G1` polyline segments into smooth circular `G2` (CW) and `G3` (CCW)
/// arcs wherever consecutive vertices lie within `config.tolerance_mm` of a common circle.
pub fn arc_fit_polyline(points: &[[f64; 2]], closed: bool, config: &ArcWelderConfig) -> Vec<FittedSegment> {
    if points.len() < 2 {
        return Vec::new();
    }
    let mut seq = points.to_vec();
    if closed && seq.first() != seq.last() {
        seq.push(seq[0]);
    }
    let n = seq.len();
    let min_pts = config.min_arc_points.max(3);
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < n {
        if i + min_pts <= n {
            let mut best_end = None;
            let mut best_circle = None;
            for end in (i + min_pts - 1)..n {
                let mid = i + (end - i) / 2;
                let p_start = seq[i];
                let p_mid = seq[mid];
                let p_end = seq[end];
                if (p_end[0] - p_start[0]).hypot(p_end[1] - p_start[1]) < 1e-6 {
                    break;
                }
                let Some((center, radius, cw)) = circumcircle(p_start, p_mid, p_end) else {
                    break;
                };
                if radius > config.max_radius_mm {
                    break;
                }
                let mut valid = true;
                for w in i..=end {
                    let dist = (seq[w][0] - center[0]).hypot(seq[w][1] - center[1]);
                    if (dist - radius).abs() > config.tolerance_mm {
                        valid = false;
                        break;
                    }
                    if w > i && w < end {
                        let c_step = (seq[w][0] - seq[w - 1][0]) * (seq[w + 1][1] - seq[w][1])
                            - (seq[w][1] - seq[w - 1][1]) * (seq[w + 1][0] - seq[w][0]);
                        if (c_step < 0.0) != cw {
                            valid = false;
                            break;
                        }
                    }
                }
                if valid {
                    best_end = Some(end);
                    best_circle = Some((center, radius, cw));
                } else {
                    break;
                }
            }
            if let (Some(end), Some((center, radius, clockwise))) = (best_end, best_circle) {
                let start = seq[i];
                let end_pt = seq[end];
                out.push(FittedSegment::Arc {
                    start,
                    end: end_pt,
                    ij_offset: [center[0] - start[0], center[1] - start[1]],
                    radius,
                    clockwise,
                });
                i = end;
                continue;
            }
        }
        out.push(FittedSegment::Linear {
            start: seq[i],
            end: seq[i + 1],
        });
        i += 1;
    }
    out
}

/// Emits G-code with `G2` / `G3` circular arcs (`I` / `J` center offsets) and optional
/// `Retract` + `Z-hop` travel protection between paths.
pub fn emit_arc_gcode(
    layers: &[PlannedLayer],
    machine: &MachineProfile,
    arc_config: &ArcWelderConfig,
    retract_zhop: Option<&RetractZHopConfig>,
) -> Result<String> {
    machine.validate()?;
    let filament_area =
        std::f64::consts::PI * (0.5 * machine.filament_diameter_mm).powi(2);
    let extrusion_per_mm =
        (machine.layer_height_mm * machine.line_width_mm) / filament_area.max(1e-9);
    let print_f = machine.print_feedrate_mm_s * 60.0;
    let travel_f = machine.travel_feedrate_mm_s * 60.0;

    let mut out = String::from("G90\nM82\nG92 E0\n");
    let mut e_pos = 0.0_f64;
    let mut current_xy: Option<[f64; 2]> = None;

    for layer in layers {
        let z = layer.z_mm;
        for path in &layer.paths {
            let segments = arc_fit_polyline(&path.points, path.closed, arc_config);
            let Some(first) = segments.first() else {
                continue;
            };
            let start = match *first {
                FittedSegment::Linear { start, .. } | FittedSegment::Arc { start, .. } => start,
            };
            let travel_dist = current_xy
                .map(|prev| (start[0] - prev[0]).hypot(start[1] - prev[1]))
                .unwrap_or(0.0);
            if let Some(rz) = retract_zhop
                && current_xy.is_some()
                && travel_dist >= rz.min_travel_mm
            {
                let retract_f = rz.retract_speed_mm_s * 60.0;
                if rz.retract_length_mm > 0.0 {
                    e_pos -= rz.retract_length_mm;
                    out.push_str(&format!("G1 E{e_pos:.5} F{retract_f:.1}\n"));
                }
                if rz.z_hop_mm > 0.0 {
                    let z_lift = z + rz.z_hop_mm;
                    out.push_str(&format!("G0 Z{z_lift:.3} F{travel_f:.1}\n"));
                }
                out.push_str(&format!(
                    "G0 X{:.3} Y{:.3} F{travel_f:.1}\n",
                    start[0], start[1]
                ));
                out.push_str(&format!("G0 Z{z:.3} F{travel_f:.1}\n"));
                if rz.retract_length_mm > 0.0 {
                    e_pos += rz.retract_length_mm;
                    out.push_str(&format!("G1 E{e_pos:.5} F{retract_f:.1}\n"));
                }
            } else {
                out.push_str(&format!(
                    "G0 X{:.3} Y{:.3} Z{z:.3} F{travel_f:.1}\n",
                    start[0], start[1]
                ));
            }
            for seg in segments {
                match seg {
                    FittedSegment::Linear { start: a, end: b } => {
                        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
                        if len > 1e-9 {
                            e_pos += len * extrusion_per_mm;
                            out.push_str(&format!(
                                "G1 X{:.3} Y{:.3} E{e_pos:.5} F{print_f:.1}\n",
                                b[0], b[1]
                            ));
                        }
                        current_xy = Some(b);
                    }
                    FittedSegment::Arc {
                        start: a,
                        end: b,
                        ij_offset: [i_off, j_off],
                        radius,
                        clockwise,
                    } => {
                        let chord = (b[0] - a[0]).hypot(b[1] - a[1]);
                        let angle = 2.0 * (0.5 * chord / radius.max(1e-6)).clamp(-1.0, 1.0).asin();
                        let arc_len = radius * angle;
                        e_pos += arc_len * extrusion_per_mm;
                        let cmd = if clockwise { "G2" } else { "G3" };
                        out.push_str(&format!(
                            "{cmd} X{:.3} Y{:.3} I{i_off:.3} J{j_off:.3} E{e_pos:.5} F{print_f:.1}\n",
                            b[0], b[1]
                        ));
                        current_xy = Some(b);
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod arc_and_zhop_tests {
    use super::*;

    #[test]
    fn arc_fit_welds_discretized_circle_into_g2_or_g3_arcs() {
        let mut pts = Vec::new();
        let r = 10.0;
        for k in 0..=16 {
            let theta = std::f64::consts::FRAC_PI_2 * (k as f64 / 16.0);
            pts.push([r * theta.cos(), r * theta.sin()]);
        }
        let fitted = arc_fit_polyline(&pts, false, &ArcWelderConfig::default());
        assert_eq!(fitted.len(), 1);
        match fitted[0] {
            FittedSegment::Arc {
                start,
                end,
                ij_offset,
                radius,
                clockwise,
            } => {
                assert!(!clockwise); // CCW quarter circle -> G3
                assert!((radius - 10.0).abs() < 1e-3);
                assert!((start[0] - 10.0).abs() < 1e-6);
                assert!((end[1] - 10.0).abs() < 1e-6);
                assert!((ij_offset[0] + 10.0).abs() < 1e-3);
                assert!(ij_offset[1].abs() < 1e-3);
            }
            _ => panic!("expected FittedSegment::Arc"),
        }
    }

    #[test]
    fn emit_arc_gcode_writes_g3_and_zhop_retracts() {
        let mut arc_pts = Vec::new();
        for k in 0..=12 {
            let theta = std::f64::consts::FRAC_PI_2 * (k as f64 / 12.0);
            arc_pts.push([10.0 * theta.cos(), 10.0 * theta.sin()]);
        }
        let layers = vec![PlannedLayer {
            z_mm: 0.2,
            paths: vec![
                PlannedPath {
                    points: arc_pts,
                    closed: false,
                },
                PlannedPath {
                    points: vec![[30.0, 30.0], [40.0, 30.0]],
                    closed: false,
                },
            ],
        }];
        let machine = MachineProfile {
            layer_height_mm: 0.2,
            line_width_mm: 0.4,
            print_feedrate_mm_s: 50.0,
            travel_feedrate_mm_s: 120.0,
            filament_diameter_mm: 1.75,
        };
        let gcode = emit_arc_gcode(
            &layers,
            &machine,
            &ArcWelderConfig::default(),
            Some(&RetractZHopConfig::default()),
        )
        .unwrap();
        assert!(gcode.contains("G3 X0.000 Y10.000 I-10.000 J0.000"));
        assert!(gcode.contains("G0 Z0.400")); // Z-hop lift of 0.2mm above z=0.2mm
    }
}
