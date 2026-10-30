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
pub use infill::generate_gyroid_infill_lines;
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
#[path = "lib/tests/mod.rs"]
mod tests;

