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





#[cfg(test)]
#[path="lib/tests/arc_and_zhop_tests.rs"]
mod arc_and_zhop_tests;

pub use gcode_core::GcodeMove;



/// Formats an Input Shaper resonance compensation command for Klipper (`SET_INPUT_SHAPER`) or Marlin (`M593`).
pub fn format_input_shaper_command(
    flavor: Flavor,
    shaper_type: &str,
    freq_x_hz: f64,
    freq_y_hz: f64,
) -> Result<String> {
    if !freq_x_hz.is_finite()
        || !(10.0..=200.0).contains(&freq_x_hz)
        || !freq_y_hz.is_finite()
        || !(10.0..=200.0).contains(&freq_y_hz)
    {
        return Err(invalid(
            "OPT_INVALID_SETTINGS",
            "Input shaper frequencies must be finite and within 10..=200 Hz",
        ));
    }
    let shaper_upper = shaper_type.trim().to_ascii_uppercase();
    if !["ZV", "MZV", "ZVD", "EI", "2HUMP_EI", "3HUMP_EI"].contains(&shaper_upper.as_str()) {
        return Err(invalid(
            "OPT_INVALID_SETTINGS",
            "Unsupported input shaper type",
        ));
    }
    Ok(match flavor {
        Flavor::Klipper => format!(
            "SET_INPUT_SHAPER SHAPER_TYPE={shaper_upper} SHAPER_FREQ_X={freq_x_hz:.2} SHAPER_FREQ_Y={freq_y_hz:.2}"
        ),
        _ => format!("M593 X F{freq_x_hz:.2}\nM593 Y F{freq_y_hz:.2}"),
    })
}



/// Generates a layer-indexed progressive part-cooling fan schedule (`(layer_index, gcode_command)`):
/// layer `0` turns the fan off (`M107`) for bed adhesion unless `full_fan_layer == 0`,
/// intermediate layers linearly ramp PWM from `min_fan_pwm` up to `max_fan_pwm` (`M106 S...`),
/// and layers at or above `full_fan_layer` maintain `max_fan_pwm`.
pub fn inject_ramping_fan_control(
    layers_count: usize,
    min_fan_pwm: u8,
    max_fan_pwm: u8,
    full_fan_layer: usize,
) -> Vec<(usize, String)> {
    let mut schedule = Vec::with_capacity(layers_count);
    let low = min_fan_pwm.min(max_fan_pwm) as f64;
    let high = max_fan_pwm.max(min_fan_pwm) as f64;
    for layer in 0..layers_count {
        let cmd = if full_fan_layer == 0 || layer >= full_fan_layer {
            if max_fan_pwm == 0 {
                "M107".to_string()
            } else {
                format!("M106 S{}", high.round() as u8)
            }
        } else if layer == 0 {
            "M107".to_string()
        } else {
            let t = if full_fan_layer <= 1 {
                1.0
            } else {
                (layer - 1) as f64 / (full_fan_layer - 1) as f64
            };
            let pwm = (low + t * (high - low)).round().clamp(0.0, 255.0) as u8;
            if pwm == 0 {
                "M107".to_string()
            } else {
                format!("M106 S{pwm}")
            }
        };
        schedule.push((layer, cmd));
    }
    schedule
}

/// Computes the maximum allowable cornering / junction velocity (in $\text{mm/s}$) between two
/// consecutive 3D move vectors `prev_vec` and `next_vec` using the classic Marlin / Grbl
/// junction deviation model ($v_{\text{corner}} = \sqrt{a \cdot \text{JD} \cdot \sin(\theta/2) / (1 - \sin(\theta/2))}$),
/// clamped to `[0.0, max_feedrate_mm_s]`.
pub fn optimize_corner_velocity_limits(
    prev_vec: [f64; 3],
    next_vec: [f64; 3],
    accel_mm_s2: f64,
    junction_deviation_mm: f64,
    max_feedrate_mm_s: f64,
) -> f64 {
    if !accel_mm_s2.is_finite()
        || accel_mm_s2 <= 0.0
        || !junction_deviation_mm.is_finite()
        || junction_deviation_mm <= 0.0
        || !max_feedrate_mm_s.is_finite()
        || max_feedrate_mm_s <= 0.0
    {
        return 0.0;
    }
    let len_a = (prev_vec[0] * prev_vec[0] + prev_vec[1] * prev_vec[1] + prev_vec[2] * prev_vec[2]).sqrt();
    let len_b = (next_vec[0] * next_vec[0] + next_vec[1] * next_vec[1] + next_vec[2] * next_vec[2]).sqrt();
    if len_a <= 1e-9 || len_b <= 1e-9 {
        return 0.0;
    }
    // Cosine of angle between unit move directions u_a and u_b:
    // Note that in Grbl/Marlin junction deviation, cos_theta = -(u_a . u_b) is the interior corner angle,
    // so sin(theta_interior / 2) = sqrt(0.5 * (1 - cos_theta_interior)) = sqrt(0.5 * (1 - (u_a . u_b))) when u_a == u_b -> sin=0?
    // Wait: when moves are collinear (u_a == u_b), the interior angle between (-u_a) and u_b is 180 deg, so sin(180/2) = 1 -> denominator (1 - 1) = 0 -> v = max_feedrate!
    let dot = ((prev_vec[0] * next_vec[0] + prev_vec[1] * next_vec[1] + prev_vec[2] * next_vec[2])
        / (len_a * len_b))
        .clamp(-1.0, 1.0);
    // Let cos_interior = -dot (angle between -u_a and u_b)
    // Then sin(interior / 2) = sqrt(0.5 * (1.0 - cos_interior)) = sqrt(0.5 * (1.0 + dot))
    let sin_half_interior = (0.5 * (1.0 + dot)).max(0.0).sqrt();
    if sin_half_interior >= 1.0 - 1e-6 {
        return max_feedrate_mm_s;
    }
    if sin_half_interior <= 1e-6 {
        return 0.0;
    }
    let v_sq = accel_mm_s2 * junction_deviation_mm * sin_half_interior / (1.0 - sin_half_interior);
    v_sq.max(0.0).sqrt().min(max_feedrate_mm_s)
}

/// Closed 2D contour loop (`Vec<[f64; 2]>`) in millimeter coordinates.
pub type Contour = Vec<[f64; 2]>;

/// Generates a continuous single-wall helical Z-interpolated ("spiral vase mode") 3D toolpath
/// from a sequence of `(z_mm, outer_contour)` slices. Each layer's contour is seam-rotated to
/// start at the vertex nearest to the previous layer's end, and $Z$ rises linearly along cumulative
/// XY perimeter arc length by `dz` (either the slice-to-slice height delta or `layer_height_mm`).
pub fn plan_spiral_vase_mode(
    contours_by_layer: &[(f64, Contour)],
    layer_height_mm: f64,
) -> Vec<[f64; 3]> {
    if contours_by_layer.is_empty() || !layer_height_mm.is_finite() || layer_height_mm <= 0.0 {
        return Vec::new();
    }
    let total_cap: usize = contours_by_layer
        .iter()
        .map(|(_, r)| r.len() + 1)
        .sum();
    let max_ring_len = contours_by_layer
        .iter()
        .map(|(_, r)| r.len())
        .max()
        .unwrap_or(0);
    let mut spiral = Vec::with_capacity(total_cap);
    let mut cum_len = Vec::with_capacity(max_ring_len + 1);
    let mut last_xy: Option<[f64; 2]> = None;

    for (layer_idx, (z_base, ring)) in contours_by_layer.iter().enumerate() {
        let n = ring.len();
        if n < 3 || !z_base.is_finite() {
            continue;
        }
        let dz = if layer_idx + 1 < contours_by_layer.len() {
            let next_z = contours_by_layer[layer_idx + 1].0;
            if next_z > *z_base && next_z.is_finite() {
                next_z - *z_base
            } else {
                layer_height_mm
            }
        } else {
            layer_height_mm
        };

        // Rotate contour so vertex 0 is nearest to `last_xy` for a seamless helical transition
        let start_idx = if let Some(prev) = last_xy {
            let mut best_idx = 0usize;
            let mut best_d2 = f64::INFINITY;
            for (idx, a) in ring.iter().enumerate() {
                let dx = a[0] - prev[0];
                let dy = a[1] - prev[1];
                let d2 = dx * dx + dy * dy;
                if d2 < best_d2 {
                    best_d2 = d2;
                    best_idx = idx;
                }
            }
            best_idx
        } else {
            0
        };

        cum_len.clear();
        cum_len.push(0.0);
        let mut total_perim = 0.0;
        for k in 0..n {
            let a = ring[(start_idx + k) % n];
            let b = ring[(start_idx + k + 1) % n];
            let seg = (b[0] - a[0]).hypot(b[1] - a[1]);
            total_perim += seg;
            cum_len.push(total_perim);
        }
        if total_perim <= 1e-9 {
            continue;
        }
        let first_pt = ring[start_idx];
        let skip_first = spiral.last().is_some_and(|prev: &[f64; 3]| {
            let dx = prev[0] - first_pt[0];
            let dy = prev[1] - first_pt[1];
            dx * dx + dy * dy < 1e-12
        });
        let inv_perim = 1.0 / total_perim;
        for (k, &cl) in cum_len.iter().enumerate() {
            if k == 0 && skip_first {
                continue;
            }
            let pt = ring[(start_idx + k) % n];
            let t = cl * inv_perim;
            let z = *z_base + t * dz;
            spiral.push([pt[0], pt[1], z]);
        }
        last_xy = Some(first_pt);
    }
    spiral
}

/// Injects back-trace nozzle wipe moves along the end of each extruding path during filament
/// retraction (`retract_length_mm` over `wipe_distance_mm`) to eliminate stringing and seam blobs.
/// Input and output moves are `[x, y, z, e]` in millimeters (where `e` is cumulative extrusion).
pub fn inject_wiper_retraction(
    moves: &[[f64; 4]],
    retract_length_mm: f64,
    wipe_distance_mm: f64,
) -> Vec<[f64; 4]> {
    if moves.len() < 2
        || !retract_length_mm.is_finite()
        || retract_length_mm <= 0.0
        || !wipe_distance_mm.is_finite()
        || wipe_distance_mm <= 0.0
    {
        return moves.to_vec();
    }
    let mut out = Vec::with_capacity(moves.len() + 16);
    let mut run_start = 0usize;
    let mut in_extrusion = false;

    let emit_wipe = |out: &mut Vec<[f64; 4]>, slice: &[[f64; 4]]| {
        if slice.len() < 2 {
            return;
        }
        // Compute total extruding path length available for back-tracing
        let mut total_avail = 0.0_f64;
        for w in slice.windows(2).rev() {
            let a = w[0];
            let b = w[1];
            let seg = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
            total_avail += seg;
            if total_avail >= wipe_distance_mm {
                break;
            }
        }
        let target_wipe = wipe_distance_mm.min(total_avail);
        if target_wipe <= 1e-6 {
            return;
        }
        let e_end = slice.last().unwrap()[3];
        let mut wiped_so_far = 0.0_f64;
        for w in slice.windows(2).rev() {
            let prev_pt = w[0];
            let curr_pt = w[1];
            let dx = prev_pt[0] - curr_pt[0];
            let dy = prev_pt[1] - curr_pt[1];
            let dz = prev_pt[2] - curr_pt[2];
            let seg = (dx * dx + dy * dy + dz * dz).sqrt();
            if seg <= 1e-9 {
                continue;
            }
            let rem = target_wipe - wiped_so_far;
            if seg >= rem {
                let frac = rem / seg;
                let wx = curr_pt[0] + frac * dx;
                let wy = curr_pt[1] + frac * dy;
                let wz = curr_pt[2] + frac * dz;
                out.push([wx, wy, wz, e_end - retract_length_mm]);
                break;
            } else {
                wiped_so_far += seg;
                let progress = wiped_so_far / target_wipe;
                let e_wipe = e_end - progress * retract_length_mm;
                out.push([prev_pt[0], prev_pt[1], prev_pt[2], e_wipe]);
            }
        }
    };

    out.push(moves[0]);
    for i in 1..moves.len() {
        let prev = moves[i - 1];
        let curr = moves[i];
        let de = curr[3] - prev[3];
        let dist = ((curr[0] - prev[0]).powi(2)
            + (curr[1] - prev[1]).powi(2)
            + (curr[2] - prev[2]).powi(2))
        .sqrt();
        let is_extruding = de > 1e-7 && dist > 1e-7;
        if is_extruding {
            if !in_extrusion {
                run_start = i - 1;
                in_extrusion = true;
            }
            out.push(curr);
        } else {
            if in_extrusion {
                // Transition from extruding run to travel move: inject back-trace wipe + retract
                emit_wipe(&mut out, &moves[run_start..i]);
                // Travel to `curr` while retracted, then unretract at destination
                out.push([curr[0], curr[1], curr[2], prev[3] - retract_length_mm]);
                out.push(curr);
                in_extrusion = false;
            } else {
                out.push(curr);
            }
        }
    }
    if in_extrusion {
        emit_wipe(&mut out, &moves[run_start..moves.len()]);
    }
    out
}

/// Fits a circle `(center, radius, is_clockwise)` to a sequence of planar polyline points
/// if all vertices lie within `tolerance_mm` of the circumcircle of the start, middle, and end points
/// and maintain a consistent turning orientation (`G2` clockwise when `is_clockwise == true`, `G3` CCW otherwise).
pub fn detect_circular_arc_fit(
    points: &[[f64; 2]],
    tolerance_mm: f64,
) -> Option<([f64; 2], f64, bool)> {
    if points.len() < 3 || !tolerance_mm.is_finite() || tolerance_mm < 0.0 {
        return None;
    }
    let p_start = points[0];
    let p_mid = points[points.len() / 2];
    let p_end = *points.last()?;
    if (p_end[0] - p_start[0]).hypot(p_end[1] - p_start[1]) < 1e-6 {
        return None;
    }
    let (center, radius, is_clockwise) = circumcircle(p_start, p_mid, p_end)?;
    if radius > 10_000.0 {
        return None;
    }
    for (idx, pt) in points.iter().enumerate() {
        let dist = (pt[0] - center[0]).hypot(pt[1] - center[1]);
        if (dist - radius).abs() > tolerance_mm {
            return None;
        }
        if idx > 0 && idx + 1 < points.len() {
            let prev = points[idx - 1];
            let next = points[idx + 1];
            let c_step =
                (pt[0] - prev[0]) * (next[1] - pt[1]) - (pt[1] - prev[1]) * (next[0] - pt[0]);
            if c_step.abs() > 1e-12 && (c_step < 0.0) != is_clockwise {
                return None;
            }
        }
    }
    Some((center, radius, is_clockwise))
}

/// Compresses consecutive collinear extrusion moves `[x, y, z, e]` (where `e` is cumulative extrusion)
/// whose intermediate vertices lie within perpendicular spatial tolerance `tol_mm` of the chord
/// and share a consistent extrusion rate $dE / ds$.
pub fn compress_collinear_extrusion_moves(moves: &[[f64; 4]], tol_mm: f64) -> Vec<[f64; 4]> {
    if moves.len() <= 2 || !tol_mm.is_finite() || tol_mm < 0.0 {
        return moves.to_vec();
    }
    let mut out = Vec::with_capacity(moves.len());
    let mut anchor_idx = 0usize;
    out.push(moves[0]);

    let mut i = 1usize;
    while i < moves.len() {
        if i + 1 >= moves.len() {
            out.push(moves[i]);
            break;
        }
        let a = moves[anchor_idx];
        let c = moves[i + 1];
        let dx = c[0] - a[0];
        let dy = c[1] - a[1];
        let dz = c[2] - a[2];
        let chord_len = (dx * dx + dy * dy + dz * dz).sqrt();
        let total_de = c[3] - a[3];

        let mut can_merge = chord_len > 1e-7 && total_de > 1e-7;
        if can_merge {
            let inv_chord = 1.0 / chord_len;
            let ux = dx * inv_chord;
            let uy = dy * inv_chord;
            let uz = dz * inv_chord;
            let rate = total_de * inv_chord;

            for k in (anchor_idx + 1)..=i {
                let p = moves[k];
                let prev = moves[k - 1];
                let seg_dx = p[0] - prev[0];
                let seg_dy = p[1] - prev[1];
                let seg_dz = p[2] - prev[2];
                let seg_len = (seg_dx * seg_dx + seg_dy * seg_dy + seg_dz * seg_dz).sqrt();
                let seg_de = p[3] - prev[3];
                if seg_len <= 1e-7 || seg_de <= 1e-7 {
                    can_merge = false;
                    break;
                }
                let seg_rate = seg_de / seg_len;
                if (seg_rate - rate).abs() > 0.05 * rate.abs().max(1e-6) {
                    can_merge = false;
                    break;
                }
                let apx = p[0] - a[0];
                let apy = p[1] - a[1];
                let apz = p[2] - a[2];
                let proj = apx * ux + apy * uy + apz * uz;
                if proj <= 0.0 || proj >= chord_len {
                    can_merge = false;
                    break;
                }
                let perp_x = apx - proj * ux;
                let perp_y = apy - proj * uy;
                let perp_z = apz - proj * uz;
                let perp_dist = (perp_x * perp_x + perp_y * perp_y + perp_z * perp_z).sqrt();
                if perp_dist > tol_mm {
                    can_merge = false;
                    break;
                }
            }
        }
        if can_merge {
            i += 1;
        } else {
            out.push(moves[i]);
            anchor_idx = i;
            i += 1;
        }
    }
    out
}





#[path="lib/algorithm_arc_fit_polyline.rs"]
mod algorithm_arc_fit_polyline;
pub use algorithm_arc_fit_polyline::arc_fit_polyline;
#[path="lib/algorithm_emit_arc_gcode.rs"]
mod algorithm_emit_arc_gcode;
pub use algorithm_emit_arc_gcode::emit_arc_gcode;
#[path="lib/algorithm_inject_pressure_advance.rs"]
mod algorithm_inject_pressure_advance;
pub use algorithm_inject_pressure_advance::inject_pressure_advance;
#[path="lib/algorithm_estimate_klipper_print_time.rs"]
mod algorithm_estimate_klipper_print_time;
pub use algorithm_estimate_klipper_print_time::estimate_klipper_print_time;
