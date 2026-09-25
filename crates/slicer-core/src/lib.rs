//! Layered toolpaths from already-cut contours. Not a CAD kernel and not a
//! print process.
//!
//! The host (CAD) sections a mesh and passes rings. This crate offsets walls,
//! hatches infill, and can encode the plan through `gcode-core`. Coordinates
//! are millimeters.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]

use planar_geometry::{
    path::BezierPath,
    rings::{self, Rings},
    stroke::{self, StrokeOptions},
};

pub use gcode_core::{
    DIALECT as GCODE_DIALECT, DialectInfo, Flavor, GcodeBounds, GcodeMove, GcodePreview, Generator,
    JOB_DIALECT as GCODE_JOB_DIALECT, JobProfile, MeshBody,
};
pub use gcode_optimize::OptimizeSettings;
pub use math_core::{Error, Result};
pub use planar_geometry::{LayerSection, MAX_LAYERS};

/// Input contour vertices per section, before geometry normalization.
pub const MAX_SECTION_POINTS: usize = 4_096;
/// Total generated/intermediate vertices across one layer or scheduled plan.
pub const MAX_PLAN_POINTS: usize = 1_000_000;
/// Upper bound on scan lines per layer, including lines with no filled spans.
pub const MAX_HATCH_LINES: usize = 65_536;
/// Conservative bound on geometric pair tests and hatch edge/sort work.
pub const MAX_PLAN_WORK: usize = 64_000_000;

#[derive(Default)]
struct Budget {
    points: usize,
    work: usize,
}

impl Budget {
    fn points(&mut self, count: usize) -> Result<()> {
        self.points = self.points.checked_add(count).ok_or_else(budget_error)?;
        if self.points > MAX_PLAN_POINTS {
            return Err(budget_error());
        }
        Ok(())
    }

    fn work(&mut self, count: usize) -> Result<()> {
        self.work = self.work.checked_add(count).ok_or_else(budget_error)?;
        if self.work > MAX_PLAN_WORK {
            return Err(budget_error());
        }
        Ok(())
    }

    fn arrangement(&mut self, points: usize) -> Result<()> {
        self.work(points.checked_mul(points).ok_or_else(budget_error)?)
    }
}

fn budget_error() -> Error {
    invalid(
        "TOOLPATH_WORK_LIMIT",
        "Toolpath geometry/work budget exceeded",
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathRole {
    Outline,
    Inset,
    Hatch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toolpath {
    pub role: PathRole,
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolpathLayer {
    pub z_mm: f64,
    pub paths: Vec<Toolpath>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolpathSettings {
    pub layer_height_mm: f64,
    pub line_width_mm: f64,
    pub wall_count: usize,
    pub infill_spacing_mm: f64,
    pub feedrate_mm_s: f64,
    pub travel_feedrate_mm_s: f64,
    /// Used only when emitting extrusion-style G-code (`E`).
    pub filament_diameter_mm: f64,
}

impl Default for ToolpathSettings {
    fn default() -> Self {
        Self {
            layer_height_mm: 0.2,
            line_width_mm: 0.4,
            wall_count: 2,
            infill_spacing_mm: 2.0,
            feedrate_mm_s: 50.0,
            travel_feedrate_mm_s: 120.0,
            filament_diameter_mm: 1.75,
        }
    }
}

fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

fn require_settings(settings: &ToolpathSettings) -> Result<()> {
    machine_profile(settings)
        .validate()
        .map_err(|error| invalid("TOOLPATH_INVALID_SETTINGS", &error.message))?;
    if !settings.infill_spacing_mm.is_finite()
        || settings.infill_spacing_mm < gcode_core::COORDINATE_RESOLUTION_MM
        || settings.infill_spacing_mm > gcode_core::MAX_COORDINATE_MM
        || settings.wall_count == 0
        || settings.wall_count > 8
        || settings.wall_count as f64 * settings.line_width_mm > gcode_core::MAX_COORDINATE_MM
    {
        return Err(invalid(
            "TOOLPATH_INVALID_SETTINGS",
            "Toolpath settings are out of range",
        ));
    }
    Ok(())
}

fn require_section(section: &LayerSection, budget: &mut Budget) -> Result<()> {
    if !valid_coordinate(section.z_mm) {
        return Err(invalid(
            "TOOLPATH_INVALID_HEIGHT",
            "Layer Z must be finite and within +/-1000000 mm",
        ));
    }
    if section.contours.len() > MAX_SECTION_POINTS / 3 {
        return Err(invalid(
            "TOOLPATH_GEOMETRY_LIMIT",
            "Section exceeds 1365 contours",
        ));
    }
    let count = point_count(&section.contours);
    if count > MAX_SECTION_POINTS {
        return Err(invalid(
            "TOOLPATH_GEOMETRY_LIMIT",
            "Section exceeds 4096 contour vertices",
        ));
    }
    if section
        .contours
        .iter()
        .any(|ring| ring.len() < 3 || ring.iter().flatten().any(|&value| !valid_coordinate(value)))
    {
        return Err(invalid(
            "TOOLPATH_INVALID_GEOMETRY",
            "Section rings need at least 3 finite vertices within +/-1000000 mm",
        ));
    }
    budget.points(count)?;
    Ok(())
}

fn valid_coordinate(value: f64) -> bool {
    value.is_finite() && value.abs() <= gcode_core::MAX_COORDINATE_MM
}

fn point_count(source: &Rings) -> usize {
    source
        .iter()
        .fold(0_usize, |count, ring| count.saturating_add(ring.len()))
}

/// Subtract a band around every boundary. Direct vertex offsets can invert a
/// consumed inset and create phantom islands; region erosion cannot do that.
/// All input rings have already been normalized to material-left winding.
fn inset_rings(source: &Rings, distance: f64, budget: &mut Budget) -> Result<Rings> {
    if source.is_empty() {
        return Ok(Vec::new());
    }
    let mut bands = Vec::new();
    let tolerance = (distance * 0.01).min(0.01);
    let options = StrokeOptions {
        width: 2.0 * distance,
        ..Default::default()
    };
    for ring in source {
        budget.arrangement(ring.len().saturating_mul(6))?;
        let path = BezierPath::from_polyline(ring, true)?;
        for contour in stroke::outline_stroke_tol(&path, &options, tolerance)? {
            let band = contour.to_ring(tolerance)?;
            budget.points(band.len())?;
            bands.push(band);
        }
    }
    budget.arrangement(point_count(source).saturating_add(point_count(&bands)))?;
    let result = rings::planar(source, &bands, "difference")?;
    budget.points(point_count(&result))?;
    Ok(result)
}

fn path_from_ring(role: PathRole, ring: &[[f64; 2]], closed: bool) -> Toolpath {
    Toolpath {
        role,
        points: ring.to_vec(),
        closed,
    }
}

fn hatch(source: &Rings, spacing: f64, budget: &mut Budget) -> Result<Vec<Toolpath>> {
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in source.iter().flatten() {
        min_y = min_y.min(point[1]);
        max_y = max_y.max(point[1]);
    }
    if source.is_empty() || max_y - min_y < spacing * 0.25 {
        return Ok(Vec::new());
    }
    let start = min_y + spacing * 0.5;
    let end = max_y - spacing * 0.25;
    if start > end {
        return Ok(Vec::new());
    }
    let intervals = ((end - start) / spacing).floor();
    if !intervals.is_finite() || intervals >= MAX_HATCH_LINES as f64 {
        return Err(invalid(
            "TOOLPATH_HATCH_LIMIT",
            "Toolpath hatch exceeds 65536 scan lines",
        ));
    }
    let line_count = intervals as usize + 1;
    let edges = point_count(source);
    let row_work = edges
        .checked_mul(1 + edges.max(1).ilog2() as usize)
        .ok_or_else(budget_error)?;
    budget.work(line_count.checked_mul(row_work).ok_or_else(budget_error)?)?;
    // Scanline active edge table: index every non-horizontal edge once by its
    // lower endpoint, then rows only visit edges that can actually cross them.
    // The eligibility rule `(a[1] <= y && y < b[1]) || (b[1] <= y && y < a[1])`
    // is exactly `min_y <= y < max_y`, and the intersection formula, winding
    // sign and grouping below are unchanged, so spans stay bit-identical to
    // the naive per-row scan over all edges.
    struct HatchEdge {
        a: [f64; 2],
        b: [f64; 2],
        min_y: f64,
        max_y: f64,
        winding: i32,
    }
    let mut sorted_edges = Vec::new();
    for ring in source {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            if a[1] == b[1] {
                continue;
            }
            sorted_edges.push(HatchEdge {
                a,
                b,
                min_y: a[1].min(b[1]),
                max_y: a[1].max(b[1]),
                winding: if b[1] > a[1] { -1_i32 } else { 1_i32 },
            });
        }
    }
    sorted_edges.sort_by(|x, y| x.min_y.total_cmp(&y.min_y));
    let mut active: Vec<usize> = Vec::new();
    let mut paths = Vec::new();
    let mut previous_y = None;
    let mut next_edge = 0;
    for row in 0..line_count {
        // Index-based placement avoids accumulated roundoff and non-advancing
        // floating-point additions. The budget counts every attempted row.
        let y = spacing.mul_add(row as f64, start);
        if !y.is_finite() || previous_y.is_some_and(|previous| y <= previous) {
            return Err(invalid(
                "TOOLPATH_INVALID_SETTINGS",
                "Hatch spacing does not advance coordinates",
            ));
        }
        previous_y = Some(y);
        if y > end {
            break;
        }
        while next_edge < sorted_edges.len() && sorted_edges[next_edge].min_y <= y {
            active.push(next_edge);
            next_edge += 1;
        }
        active.retain(|&index| y < sorted_edges[index].max_y);
        let mut hits = Vec::new();
        for &index in &active {
            let edge = &sorted_edges[index];
            let (a, b) = (edge.a, edge.b);
            let t = (y - a[1]) / (b[1] - a[1]);
            let x = (b[0] - a[0]).mul_add(t, a[0]);
            if !valid_coordinate(x) {
                return Err(invalid(
                    "TOOLPATH_INVALID_GEOMETRY",
                    "Non-finite hatch intersection",
                ));
            }
            hits.push((x, edge.winding));
        }
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        // Sweep signed crossings so holes, disconnected islands and touching
        // boundaries share the same nonzero fill rule as the wall geometry.
        let mut winding = 0_i32;
        let mut span_start = 0.0;
        let mut index = 0;
        let row_start = paths.len();
        while index < hits.len() {
            let x = hits[index].0;
            let old_winding = winding;
            while index < hits.len() && hits[index].0 == x {
                winding += hits[index].1;
                index += 1;
            }
            if old_winding == 0 && winding != 0 {
                span_start = x;
            } else if old_winding != 0 && winding == 0 && x > span_start {
                budget.points(2)?;
                paths.push(Toolpath {
                    role: PathRole::Hatch,
                    points: vec![[span_start, y], [x, y]],
                    closed: false,
                });
            }
        }
        if winding != 0 {
            return Err(invalid(
                "TOOLPATH_INVALID_GEOMETRY",
                "Unbalanced hatch boundary",
            ));
        }
        if row % 2 == 1 {
            paths[row_start..].reverse();
            for path in &mut paths[row_start..] {
                path.points.reverse();
            }
        }
    }
    Ok(paths)
}

/// Offset contours and hatch one already-computed layer section. Contours use
/// nonzero winding (outer boundaries and holes have opposite orientations).
/// Coordinates remain in the host's model space; Z is a preview sample plane.
pub fn plan_layer(section: &LayerSection, settings: &ToolpathSettings) -> Result<ToolpathLayer> {
    require_settings(settings)?;
    plan_layer_with_budget(section, settings, &mut Budget::default())
}

fn plan_layer_with_budget(
    section: &LayerSection,
    settings: &ToolpathSettings,
    budget: &mut Budget,
) -> Result<ToolpathLayer> {
    require_section(section, budget)?;
    budget.arrangement(point_count(&section.contours))?;
    let source = rings::nonzero(&section.contours)?;
    budget.points(point_count(&source))?;
    let mut paths = Vec::new();
    for wall in 0..settings.wall_count {
        let inset = (wall as f64 + 0.5) * settings.line_width_mm;
        let offset = inset_rings(&source, inset, budget)?;
        let role = if wall == 0 {
            PathRole::Outline
        } else {
            PathRole::Inset
        };
        for ring in &offset {
            budget.points(ring.len())?;
            paths.push(path_from_ring(role, ring, true));
        }
        if offset.is_empty() {
            break;
        }
    }
    let remaining = inset_rings(
        &source,
        settings.wall_count as f64 * settings.line_width_mm,
        budget,
    )?;
    paths.extend(hatch(&remaining, settings.infill_spacing_mm, budget)?);
    Ok(ToolpathLayer {
        z_mm: section.z_mm,
        paths,
    })
}

/// Sample `z_min + i * layer_height_mm` in the half-open range `[z_min, z_max)`.
/// `section_at` must return the requested Z unchanged, including empty samples.
/// Every sample consumes the 2048-layer budget; empty planned layers are omitted.
/// This is model-space preview sampling, not first-layer/nozzle placement. The
/// emitter uses the nominal layer height even if the final interval is partial.
pub fn schedule_layers<F>(
    mut section_at: F,
    z_min: f64,
    z_max: f64,
    settings: &ToolpathSettings,
) -> Result<Vec<ToolpathLayer>>
where
    F: FnMut(f64) -> Result<LayerSection>,
{
    require_settings(settings)?;
    if !valid_coordinate(z_min) || !valid_coordinate(z_max) || z_max <= z_min {
        return Err(invalid(
            "TOOLPATH_INVALID_RANGE",
            "Layer range must be finite, increasing and within +/-1000000 mm",
        ));
    }
    // Preflight before calling the host, so empty sections cannot evade the cap.
    let mut heights = Vec::new();
    for index in 0..=MAX_LAYERS {
        let z = settings.layer_height_mm.mul_add(index as f64, z_min);
        if !z.is_finite() || heights.last().is_some_and(|&previous| z <= previous) {
            return Err(invalid(
                "TOOLPATH_INVALID_RANGE",
                "Layer height does not advance Z",
            ));
        }
        if z >= z_max {
            break;
        }
        if index == MAX_LAYERS {
            return Err(invalid(
                "TOOLPATH_LAYER_LIMIT",
                "Toolpath plan exceeded 2048 sampled layers",
            ));
        }
        heights.push(z);
    }
    let mut layers = Vec::new();
    let mut budget = Budget::default();
    for z in heights {
        let section = section_at(z)?;
        if section.z_mm != z {
            return Err(invalid(
                "TOOLPATH_INVALID_HEIGHT",
                "Section Z does not match the requested sample plane",
            ));
        }
        let layer = plan_layer_with_budget(&section, settings, &mut budget)?;
        if !layer.paths.is_empty() {
            layers.push(layer);
        }
    }
    Ok(layers)
}

fn machine_profile(settings: &ToolpathSettings) -> gcode_core::MachineProfile {
    gcode_core::MachineProfile {
        layer_height_mm: settings.layer_height_mm,
        line_width_mm: settings.line_width_mm,
        print_feedrate_mm_s: settings.feedrate_mm_s,
        travel_feedrate_mm_s: settings.travel_feedrate_mm_s,
        filament_diameter_mm: settings.filament_diameter_mm,
    }
}

fn planned_layers(layers: &[ToolpathLayer]) -> Result<Vec<gcode_core::PlannedLayerRef<'_>>> {
    if layers.len() > MAX_LAYERS {
        return Err(invalid(
            "TOOLPATH_LAYER_LIMIT",
            "Toolpath plan exceeded 2048 layers",
        ));
    }
    // Public callers can supply their own plans. Bound the compatibility view
    // before allocating; gcode-core performs the complete numeric validation.
    // The views borrow the toolpath points, so this no longer deep-copies
    // the plan for emit/volume; the optimizer path still owns its copy
    // because path optimization rewrites the geometry in place.
    let mut budget = Budget::default();
    for layer in layers {
        budget.points(layer.paths.len())?;
        for path in &layer.paths {
            budget.points(path.points.len())?;
        }
    }
    Ok(layers
        .iter()
        .map(|layer| gcode_core::PlannedLayerRef {
            z_mm: layer.z_mm,
            paths: layer
                .paths
                .iter()
                .map(|path| gcode_core::PlannedPathRef {
                    points: &path.points,
                    closed: path.closed,
                })
                .collect(),
        })
        .collect())
}

fn gcode_error(error: gcode_core::Error) -> Error {
    error
}

/// One encoding of a planned toolpath. Not the only possible machine dialect.
pub fn emit_gcode(layers: &[ToolpathLayer], settings: &ToolpathSettings) -> Result<String> {
    require_settings(settings)?;
    gcode_core::emit_ref(&planned_layers(layers)?, &machine_profile(settings)).map_err(gcode_error)
}

fn optimize_input(layers: &[ToolpathLayer]) -> Result<gcode_optimize::OptimizeInput> {
    Ok(gcode_optimize::from_toolpaths(layers.iter().map(|layer| {
        (
            layer.z_mm,
            layer
                .paths
                .iter()
                .map(|path| (path.points.clone(), path.closed))
                .collect::<Vec<_>>(),
        )
    })))
}

/// Preview dialect after path optimization (still not a printer job).
pub fn emit_optimized_gcode(
    layers: &[ToolpathLayer],
    settings: &ToolpathSettings,
    optimize: &OptimizeSettings,
) -> Result<String> {
    require_settings(settings)?;
    let input = optimize_input(layers)?;
    gcode_optimize::emit_optimized(input, &machine_profile(settings), optimize).map_err(gcode_error)
}

/// Machine job dialect: heat, retract, start/end after path optimization.
pub fn emit_job_gcode(
    layers: &[ToolpathLayer],
    job: &JobProfile,
    optimize: &OptimizeSettings,
) -> Result<String> {
    require_settings_from_job(job)?;
    let input = optimize_input(layers)?;
    gcode_optimize::emit_optimized_job(input, job, optimize).map_err(gcode_error)
}

/// Thick `.gcode.3mf` with job G-code and optional mesh body.
pub fn emit_job_gcode_3mf(
    layers: &[ToolpathLayer],
    job: &JobProfile,
    optimize: &OptimizeSettings,
    mesh: Option<&MeshBody>,
) -> Result<Vec<u8>> {
    require_settings_from_job(job)?;
    let input = optimize_input(layers)?;
    gcode_optimize::emit_optimized_gcode_3mf_job(input, job, optimize, mesh).map_err(gcode_error)
}

fn require_settings_from_job(job: &JobProfile) -> Result<()> {
    job.validate().map_err(gcode_error)
}

/// Build a [`JobProfile`] from toolpath settings and optional job overrides already validated.
pub fn job_profile(settings: &ToolpathSettings, job: JobProfile) -> Result<JobProfile> {
    require_settings(settings)?;
    let mut profile = job;
    profile.machine = machine_profile(settings);
    profile.validate().map_err(gcode_error)?;
    Ok(profile)
}

pub fn parse_gcode_preview(gcode: &str) -> Result<GcodePreview> {
    gcode_core::parse(gcode).map_err(gcode_error)
}

pub fn parse_gcode_job(gcode: &str) -> Result<GcodePreview> {
    gcode_core::parse_job(gcode).map_err(gcode_error)
}

/// Native dialects stay strict; other slicers' files use the tolerant reader.
pub fn parse_gcode_any(gcode: &str) -> Result<(GcodePreview, DialectInfo)> {
    gcode_core::parse_any(gcode).map_err(gcode_error)
}

/// Nominal volume estimate; returns NaN for invalid settings or oversized plans.
/// This does not model a printer's real bead shape or partial layer thickness.
pub fn deposited_volume_mm3(layers: &[ToolpathLayer], settings: &ToolpathSettings) -> f64 {
    if require_settings(settings).is_err() {
        return f64::NAN;
    }
    match planned_layers(layers) {
        Ok(layers) => gcode_core::deposited_volume_mm3_ref(&layers, &machine_profile(settings)),
        Err(_) => f64::NAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pre-AET reference implementation: every row scans every edge.
    /// Kept as the equivalence oracle for the active edge table in `hatch`.
    fn hatch_naive(source: &Rings, spacing: f64, budget: &mut Budget) -> Result<Vec<Toolpath>> {
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for point in source.iter().flatten() {
            min_y = min_y.min(point[1]);
            max_y = max_y.max(point[1]);
        }
        if source.is_empty() || max_y - min_y < spacing * 0.25 {
            return Ok(Vec::new());
        }
        let start = min_y + spacing * 0.5;
        let end = max_y - spacing * 0.25;
        if start > end {
            return Ok(Vec::new());
        }
        let intervals = ((end - start) / spacing).floor();
        if !intervals.is_finite() || intervals >= MAX_HATCH_LINES as f64 {
            return Err(invalid(
                "TOOLPATH_HATCH_LIMIT",
                "Toolpath hatch exceeds 65536 scan lines",
            ));
        }
        let line_count = intervals as usize + 1;
        let mut paths = Vec::new();
        let mut previous_y = None;
        for row in 0..line_count {
            let y = spacing.mul_add(row as f64, start);
            if !y.is_finite() || previous_y.is_some_and(|previous| y <= previous) {
                return Err(invalid(
                    "TOOLPATH_INVALID_SETTINGS",
                    "Hatch spacing does not advance coordinates",
                ));
            }
            previous_y = Some(y);
            if y > end {
                break;
            }
            let mut hits = Vec::new();
            for ring in source {
                for i in 0..ring.len() {
                    let a = ring[i];
                    let b = ring[(i + 1) % ring.len()];
                    if (a[1] <= y && y < b[1]) || (b[1] <= y && y < a[1]) {
                        let t = (y - a[1]) / (b[1] - a[1]);
                        let x = (b[0] - a[0]).mul_add(t, a[0]);
                        if !valid_coordinate(x) {
                            return Err(invalid(
                                "TOOLPATH_INVALID_GEOMETRY",
                                "Non-finite hatch intersection",
                            ));
                        }
                        hits.push((x, if b[1] > a[1] { -1_i32 } else { 1_i32 }));
                    }
                }
            }
            hits.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0_i32;
            let mut span_start = 0.0;
            let mut index = 0;
            let row_start = paths.len();
            while index < hits.len() {
                let x = hits[index].0;
                let old_winding = winding;
                while index < hits.len() && hits[index].0 == x {
                    winding += hits[index].1;
                    index += 1;
                }
                if old_winding == 0 && winding != 0 {
                    span_start = x;
                } else if old_winding != 0 && winding == 0 && x > span_start {
                    budget.points(2)?;
                    paths.push(Toolpath {
                        role: PathRole::Hatch,
                        points: vec![[span_start, y], [x, y]],
                        closed: false,
                    });
                }
            }
            if winding != 0 {
                return Err(invalid(
                    "TOOLPATH_INVALID_GEOMETRY",
                    "Unbalanced hatch boundary",
                ));
            }
            if row % 2 == 1 {
                paths[row_start..].reverse();
                for path in &mut paths[row_start..] {
                    path.points.reverse();
                }
            }
        }
        Ok(paths)
    }

    /// Deterministic xorshift64* so the equivalence test needs no rand crate.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        /// Coarse grid values make scan lines regularly hit exact vertex
        /// heights, stressing the half-open `min_y <= y < max_y` rule.
        fn coordinate(&mut self) -> f64 {
            (self.next() % 41) as f64 * 0.25 - 5.0
        }
    }

    /// Star-shaped (hence simple) ring: sorted angles, random radii, but with
    /// y coordinates snapped to the coarse grid for vertex-height hits.
    fn random_ring(rng: &mut Rng, center: [f64; 2], vertices: usize) -> Vec<[f64; 2]> {
        let mut ring = Vec::with_capacity(vertices);
        for i in 0..vertices {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / vertices as f64;
            let radius = 0.5 + (rng.next() % 20) as f64 * 0.25;
            let x = center[0] + radius * angle.cos();
            // Snap y to the grid; keep x raw so crossings are non-degenerate.
            let y = ((center[1] + radius * angle.sin()) * 4.0).round() / 4.0;
            ring.push([x, y]);
        }
        ring
    }

    #[test]
    fn hatch_active_edge_table_matches_naive_oracle() {
        let mut rng = Rng(0x1234_5678_9ABC_DEF1);
        for case in 0..200 {
            let mut rings: Rings = Vec::new();
            let ring_count = 1 + (rng.next() % 3) as usize;
            for _ in 0..ring_count {
                let center = [rng.coordinate(), rng.coordinate()];
                let vertices = 3 + (rng.next() % 10) as usize;
                rings.push(random_ring(&mut rng, center, vertices));
            }
            // Spacings that do and do not land on the 0.25 y grid.
            for spacing in [0.25, 0.5, 0.75, 1.0, 0.3, 2.0] {
                let expected = hatch_naive(&rings, spacing, &mut Budget::default());
                let actual = hatch(&rings, spacing, &mut Budget::default());
                match (expected, actual) {
                    (Ok(expected), Ok(actual)) => assert_eq!(
                        actual, expected,
                        "case {case} spacing {spacing}: AET hatch diverged from naive scan"
                    ),
                    (expected, actual) => assert_eq!(
                        actual.is_err(),
                        expected.is_err(),
                        "case {case} spacing {spacing}: error mismatch"
                    ),
                }
            }
        }
    }

    #[test]
    fn hatch_handles_hole_and_touching_rows() {
        // Square with a square hole, spacing lands exactly on vertex heights.
        let rings: Rings = vec![
            vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
            vec![[1., 1.], [1., 3.], [3., 3.], [3., 1.]],
        ];
        let expected = hatch_naive(&rings, 1.0, &mut Budget::default()).unwrap();
        let actual = hatch(&rings, 1.0, &mut Budget::default()).unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.is_empty());
        // Degenerate inputs keep working: empty rings and flat rings.
        assert!(hatch(&Rings::new(), 1.0, &mut Budget::default())
            .unwrap()
            .is_empty());
        let flat: Rings = vec![vec![[0., 1.], [2., 1.], [1., 1.5]]];
        assert_eq!(
            hatch(&flat, 1.0, &mut Budget::default()).unwrap(),
            hatch_naive(&flat, 1.0, &mut Budget::default()).unwrap()
        );
    }
}
