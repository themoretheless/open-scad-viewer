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
pub(crate) struct Budget {
    pub(crate) points: usize,
    pub(crate) work: usize,
}

#[cfg(any())]
struct UnusedBudget {
    points: usize,
    work: usize,
}

impl Budget {
    pub(crate) fn points(&mut self, count: usize) -> Result<()> {
        self.points = self.points.checked_add(count).ok_or_else(budget_error)?;
        if self.points > MAX_PLAN_POINTS {
            return Err(budget_error());
        }
        Ok(())
    }

    pub(crate) fn work(&mut self, count: usize) -> Result<()> {
        self.work = self.work.checked_add(count).ok_or_else(budget_error)?;
        if self.work > MAX_PLAN_WORK {
            return Err(budget_error());
        }
        Ok(())
    }

    pub(crate) fn arrangement(&mut self, points: usize) -> Result<()> {
        self.work(points.checked_mul(points).ok_or_else(budget_error)?)
    }
}

pub(crate) fn budget_error() -> Error {
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
    SolidSkin,
    Bridge,
    Support,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InfillPattern {
    #[default]
    Rectilinear,
    Grid,
    Triangles,
    Concentric,
    Honeycomb,
    Gyroid,
    HilbertCurve,
    ArchimedeanChords,
    CrossHatch,
    TriHexagon,
    CubicSubdivision,
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

pub(crate) fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

pub(crate) fn require_settings(settings: &ToolpathSettings) -> Result<()> {
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

pub(crate) fn require_section(section: &LayerSection, budget: &mut Budget) -> Result<()> {
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

pub(crate) fn valid_coordinate(value: f64) -> bool {
    value.is_finite() && value.abs() <= gcode_core::MAX_COORDINATE_MM
}

pub(crate) fn point_count(source: &Rings) -> usize {
    source
        .iter()
        .fold(0_usize, |count, ring| count.saturating_add(ring.len()))
}

/// Subtract a band around every boundary. Direct vertex offsets can invert a
/// consumed inset and create phantom islands; region erosion cannot do that.
/// All input rings have already been normalized to material-left winding.
pub(crate) fn inset_rings(source: &Rings, distance: f64, budget: &mut Budget) -> Result<Rings> {
    if source.is_empty() {
        return Ok(Vec::new());
    }
    let mut bands = Vec::with_capacity(source.len());
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

pub(crate) fn path_from_ring(role: PathRole, ring: &[[f64; 2]], closed: bool) -> Toolpath {
    Toolpath {
        role,
        points: ring.to_vec(),
        closed,
    }
}


mod adhesion_and_support;
mod infill;

pub use adhesion_and_support::{
    BrimConfig, Contour, FuzzySkinConfig, RaftConfig, SkirtConfig, SupportConfig, TreeSupportNode,
    VariableLayerConfig, apply_fuzzy_skin_to_contour, generate_brim, generate_raft, generate_skirt,
    plan_layers_with_skins_and_supports, plan_tree_support_trunks, schedule_adaptive_heights,
};
use infill::{generate_cubic_infill, generate_infill, hatch, hatch_angled};

/// Offset contours and hatch one already-computed layer section. Contours use
/// nonzero winding (outer boundaries and holes have opposite orientations).
/// Coordinates remain in the host's model space; Z is a preview sample plane.
pub fn plan_layer(section: &LayerSection, settings: &ToolpathSettings) -> Result<ToolpathLayer> {
    require_settings(settings)?;
    plan_layer_with_budget(section, settings, &mut Budget::default())
}

/// Plan one layer with an explicit [`InfillPattern`] (`Grid`, `Triangles`, `Concentric`, or `Rectilinear`)
/// and optional solid top/bottom skin hatching at `line_width_mm` pitch.
pub fn plan_layer_with_pattern(
    section: &LayerSection,
    settings: &ToolpathSettings,
    pattern: InfillPattern,
    solid_skin: bool,
) -> Result<ToolpathLayer> {
    require_settings(settings)?;
    let mut budget = Budget::default();
    require_section(section, &mut budget)?;
    budget.arrangement(point_count(&section.contours))?;
    let source = rings::nonzero(&section.contours)?;
    budget.points(point_count(&source))?;
    let mut paths = Vec::new();
    for wall in 0..settings.wall_count {
        let inset = (wall as f64 + 0.5) * settings.line_width_mm;
        let offset = inset_rings(&source, inset, &mut budget)?;
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
        &mut budget,
    )?;
    if solid_skin {
        paths.extend(hatch_angled(
            &remaining,
            settings.line_width_mm,
            std::f64::consts::FRAC_PI_4,
            PathRole::SolidSkin,
            &mut budget,
        )?);
    } else if pattern == InfillPattern::CubicSubdivision {
        paths.extend(generate_cubic_infill(
            &remaining,
            settings.infill_spacing_mm,
            section.z_mm,
            PathRole::Hatch,
            &mut budget,
        )?);
    } else {
        paths.extend(generate_infill(
            &remaining,
            settings.infill_spacing_mm,
            pattern,
            PathRole::Hatch,
            &mut budget,
        )?);
    }
    Ok(ToolpathLayer {
        z_mm: section.z_mm,
        paths,
    })
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

/// Parallel counterpart to [`schedule_layers`] for thread-safe section providers (`F: Fn(f64) -> Result<LayerSection> + Sync`).
/// Evaluates sections and layer toolpaths across hardware threads while enforcing the exact same
/// cumulative `Budget` (`MAX_PLAN_POINTS`, `MAX_PLAN_WORK`) and layer ordering as [`schedule_layers`].
pub fn schedule_layers_parallel<F>(
    section_at: F,
    z_min: f64,
    z_max: f64,
    settings: &ToolpathSettings,
) -> Result<Vec<ToolpathLayer>>
where
    F: Fn(f64) -> Result<LayerSection> + Sync,
{
    require_settings(settings)?;
    if !valid_coordinate(z_min) || !valid_coordinate(z_max) || z_max <= z_min {
        return Err(invalid(
            "TOOLPATH_INVALID_RANGE",
            "Layer range must be finite, increasing and within +/-1000000 mm",
        ));
    }
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
    let per_layer = math_core::par_try_map(&heights, 2, |&z| {
        let section = section_at(z)?;
        if section.z_mm != z {
            return Err(invalid(
                "TOOLPATH_INVALID_HEIGHT",
                "Section Z does not match the requested sample plane",
            ));
        }
        let mut local_budget = Budget::default();
        let layer = plan_layer_with_budget(&section, settings, &mut local_budget)?;
        Ok((layer, local_budget))
    })?;
    let mut total_budget = Budget::default();
    let mut layers = Vec::new();
    for (layer, local_budget) in per_layer {
        total_budget.points(local_budget.points)?;
        total_budget.work(local_budget.work)?;
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

    #[test]
    fn infill_patterns_and_solid_skin_generate_valid_toolpaths() {
        let section = LayerSection {
            z_mm: 0.2,
            contours: vec![vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]]],
        };
        let settings = ToolpathSettings::default();
        let rect = plan_layer_with_pattern(&section, &settings, InfillPattern::Rectilinear, false)
            .unwrap();
        let grid = plan_layer_with_pattern(&section, &settings, InfillPattern::Grid, false).unwrap();
        let tri =
            plan_layer_with_pattern(&section, &settings, InfillPattern::Triangles, false).unwrap();
        let conc =
            plan_layer_with_pattern(&section, &settings, InfillPattern::Concentric, false).unwrap();
        let hex =
            plan_layer_with_pattern(&section, &settings, InfillPattern::Honeycomb, false).unwrap();
        let gyroid =
            plan_layer_with_pattern(&section, &settings, InfillPattern::Gyroid, false).unwrap();
        let hilbert =
            plan_layer_with_pattern(&section, &settings, InfillPattern::HilbertCurve, false)
                .unwrap();
        let arch =
            plan_layer_with_pattern(&section, &settings, InfillPattern::ArchimedeanChords, false)
                .unwrap();
        let skin =
            plan_layer_with_pattern(&section, &settings, InfillPattern::Rectilinear, true).unwrap();

        assert!(grid.paths.len() > rect.paths.len());
        assert!(tri.paths.len() > grid.paths.len());
        assert!(!hex.paths.is_empty());
        assert!(!gyroid.paths.is_empty());
        assert!(!hilbert.paths.is_empty());
        assert!(!arch.paths.is_empty());
        assert!(conc
            .paths
            .iter()
            .filter(|p| p.role == PathRole::Hatch)
            .all(|p| p.closed));
        assert!(skin
            .paths
            .iter()
            .any(|p| p.role == PathRole::SolidSkin && !p.closed));
    }

    #[test]
    fn adaptive_heights_and_skin_support_detection_work() {
        let cfg = VariableLayerConfig::default();
        // Steep vertical wall (|n_z| = 0) -> max_layer_height_mm, shallow dome (|n_z| -> 1) -> min_layer_height_mm
        let heights = schedule_adaptive_heights(0.0, 2.0, &cfg, |z| if z < 1.0 { 0.0 } else { 0.95 })
            .unwrap();
        assert!(heights.len() >= 8);
        let step_steep = heights[1] - heights[0];
        let step_shallow = heights[heights.len() - 1] - heights[heights.len() - 2];
        assert!(step_steep > step_shallow);

        // Two-layer overhanging L-shape: layer 0 is [0..10]x[0..10], layer 1 is [0..20]x[0..10]
        let sections = vec![
            LayerSection {
                z_mm: 0.2,
                contours: vec![vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]]],
            },
            LayerSection {
                z_mm: 0.4,
                contours: vec![vec![[0., 0.], [20., 0.], [20., 10.], [0., 10.]]],
            },
        ];
        let settings = ToolpathSettings::default();
        let support_cfg = SupportConfig {
            overhang_offset_mm: 1.0,
            support_spacing_mm: 2.0,
        };
        let planned = plan_layers_with_skins_and_supports(
            &sections,
            &settings,
            InfillPattern::Grid,
            Some(&support_cfg),
        )
        .unwrap();
        assert_eq!(planned.len(), 2);
        // Layer 0 should contain Support paths under the [10..20]x[0..10] overhang of layer 1
        assert!(planned[0].paths.iter().any(|p| p.role == PathRole::Support));
        // Layer 1 has an unsupported bridge region [10..20]x[0..10]
        assert!(planned[1].paths.iter().any(|p| p.role == PathRole::Bridge));
    }

    #[test]
    fn crosshatch_trihexagon_cubic_and_brim_generation_work() {
        let sec_z0 = LayerSection {
            z_mm: 0.2,
            contours: vec![vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]]],
        };
        let sec_z1 = LayerSection {
            z_mm: 1.4,
            contours: vec![vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]]],
        };
        let settings = ToolpathSettings::default();
        let ch =
            plan_layer_with_pattern(&sec_z0, &settings, InfillPattern::CrossHatch, false).unwrap();
        let th =
            plan_layer_with_pattern(&sec_z0, &settings, InfillPattern::TriHexagon, false).unwrap();
        let cub0 = plan_layer_with_pattern(
            &sec_z0,
            &settings,
            InfillPattern::CubicSubdivision,
            false,
        )
        .unwrap();
        let cub1 = plan_layer_with_pattern(
            &sec_z1,
            &settings,
            InfillPattern::CubicSubdivision,
            false,
        )
        .unwrap();
        assert!(!ch.paths.is_empty());
        assert!(!th.paths.is_empty());
        assert!(!cub0.paths.is_empty());
        // CubicSubdivision shifts phase with Z so layer Z=0.2 and Z=1.4 have distinct infill coordinates
        assert_ne!(cub0.paths, cub1.paths);

        let brim = generate_brim(&sec_z0.contours, 2.0, 0.5).unwrap();
        assert_eq!(brim.len(), 4);
        // Outermost brim loop extends outside [-0.2..20.2]
        assert!(brim.last().unwrap().iter().any(|p| p[0] > 21.0));

        let skirt = generate_skirt(&sec_z0.contours, 3.0, 2, 0.5).unwrap();
        assert_eq!(skirt.len(), 2);
        // First skirt loop starts 3.0 mm outside [0..20] -> beyond 23.0 mm
        assert!(skirt[0].iter().any(|p| p[0] >= 22.9));
        // Second skirt loop is spaced further out
        assert!(skirt[1].iter().any(|p| p[0] >= 23.4));

        // Tree support trunk coalescing from 4 overhang tips at z=20.0 down to bed_z=0.0
        let tips = vec![
            [10.0, 10.0, 20.0],
            [12.0, 10.0, 20.0],
            [30.0, 30.0, 20.0],
            [31.5, 30.0, 20.0],
        ];
        let trunks = plan_tree_support_trunks(&tips, 0.0, 45.0, 5.0);
        assert!(!trunks.is_empty());
        // Nearby tips ([10,10] & [12,10]) and ([30,30] & [31.5,30]) coalesce so fewer trunks hit bed_z=0.0 than 4
        let bed_contacts = trunks
            .iter()
            .filter(|(_, end, _)| (end[2] - 0.0).abs() < 1e-6)
            .count();
        assert!(bed_contacts >= 1 && bed_contacts <= 2, "bed_contacts={bed_contacts}");
        // Trunk radii increase toward the bed
        let max_radius = trunks.iter().map(|(_, _, r)| *r).fold(0.0_f64, f64::max);
        let min_radius = trunks.iter().map(|(_, _, r)| *r).fold(f64::INFINITY, f64::min);
        assert!(max_radius > min_radius);

        // Two-layer raft generation with orthogonal base + denser interface
        let raft_cfg = RaftConfig::default();
        let (raft_base, raft_interface) = generate_raft(
            &sec_z0.contours,
            raft_cfg.margin_mm,
            raft_cfg.base_spacing_mm,
            raft_cfg.interface_spacing_mm,
            raft_cfg.base_z_mm,
            raft_cfg.interface_z_mm,
        )
        .unwrap();
        assert!(!raft_base.is_empty());
        assert!(raft_interface.len() > raft_base.len() * 2);
        assert!(raft_base.iter().all(|s| (s[0][2] - raft_cfg.base_z_mm).abs() < 1e-9));
        assert!(raft_interface.iter().all(|s| (s[0][2] - raft_cfg.interface_z_mm).abs() < 1e-9));

        // Fuzzy skin outer contour subdivision and deterministic normal jitter
        let f_cfg = FuzzySkinConfig::default();
        let fuzzy = apply_fuzzy_skin_to_contour(
            &sec_z0.contours[0],
            f_cfg.thickness_mm,
            f_cfg.point_dist_mm,
            f_cfg.seed,
        );
        // 80 mm perimeter at 0.8 mm point distance -> ~100 subdivided vertices
        assert!(fuzzy.len() >= 80, "expected >= 80 vertices, got {}", fuzzy.len());
        // Deterministic: calling with same seed yields identical contour
        let fuzzy_same = apply_fuzzy_skin_to_contour(
            &sec_z0.contours[0],
            f_cfg.thickness_mm,
            f_cfg.point_dist_mm,
            f_cfg.seed,
        );
        assert_eq!(fuzzy, fuzzy_same);
        // Outward displacement: some vertices lie strictly outside [0..20]x[0..20] up to thickness_mm
        assert!(fuzzy.iter().any(|p| p[0] > 20.05 || p[1] > 20.05 || p[0] < -0.05 || p[1] < -0.05));
        assert!(fuzzy.iter().all(|p| p[0] >= -f_cfg.thickness_mm - 1e-6 && p[0] <= 20.0 + f_cfg.thickness_mm + 1e-6));
    }
}

