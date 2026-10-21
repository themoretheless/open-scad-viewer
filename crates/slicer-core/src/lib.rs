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

fn path_from_ring(role: PathRole, ring: &[[f64; 2]], closed: bool) -> Toolpath {
    Toolpath {
        role,
        points: ring.to_vec(),
        closed,
    }
}

















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
#[path = "tests/tests_integration.rs"]
mod tests;

/// Configuration for curvature / surface-normal adaptive layer thickness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VariableLayerConfig {
    pub min_layer_height_mm: f64,
    pub max_layer_height_mm: f64,
}

impl Default for VariableLayerConfig {
    fn default() -> Self {
        Self {
            min_layer_height_mm: 0.08,
            max_layer_height_mm: 0.28,
        }
    }
}

/// Computes an adaptive sequence of Z sample heights in `[z_min, z_max)` driven by
/// `max_abs_normal_z(z)` ($\max |n_z| \in [0, 1]$ on the slice contour):
/// vertical walls ($|n_z| \approx 0$) use `max_layer_height_mm`, while shallow / horizontal
/// slopes ($|n_z| \to 1$) smoothly step down to `min_layer_height_mm`.
pub fn schedule_adaptive_heights<F>(
    z_min: f64,
    z_max: f64,
    config: &VariableLayerConfig,
    mut max_abs_normal_z: F,
) -> Result<Vec<f64>>
where
    F: FnMut(f64) -> f64,
{
    if !valid_coordinate(z_min)
        || !valid_coordinate(z_max)
        || z_max <= z_min
        || !config.min_layer_height_mm.is_finite()
        || !config.max_layer_height_mm.is_finite()
        || config.min_layer_height_mm < gcode_core::COORDINATE_RESOLUTION_MM
        || config.max_layer_height_mm < config.min_layer_height_mm
    {
        return Err(invalid(
            "TOOLPATH_INVALID_RANGE",
            "Invalid adaptive layer height range or settings",
        ));
    }
    let mut heights = Vec::with_capacity(
        (((z_max - z_min) / config.min_layer_height_mm).ceil() as usize + 1).min(MAX_LAYERS),
    );
    let mut z = z_min;
    for _ in 0..MAX_LAYERS {
        if z >= z_max {
            break;
        }
        heights.push(z);
        let nz = max_abs_normal_z(z).abs().clamp(0.0, 1.0);
        let dz = config.max_layer_height_mm
            - nz * (config.max_layer_height_mm - config.min_layer_height_mm);
        let next_z = z + dz.max(config.min_layer_height_mm);
        if !next_z.is_finite() || next_z <= z {
            return Err(invalid(
                "TOOLPATH_INVALID_RANGE",
                "Adaptive layer height did not advance Z",
            ));
        }
        z = next_z;
    }
    if z < z_max && heights.len() >= MAX_LAYERS {
        return Err(invalid(
            "TOOLPATH_LAYER_LIMIT",
            "Adaptive layer schedule exceeded 2048 layers",
        ));
    }
    Ok(heights)
}

/// Configuration for automatic overhang and support toolpath generation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportConfig {
    /// Horizontal expansion threshold beyond which upper-layer material is considered an overhang.
    pub overhang_offset_mm: f64,
    /// Line spacing for support hatching.
    pub support_spacing_mm: f64,
}

impl Default for SupportConfig {
    fn default() -> Self {
        Self {
            overhang_offset_mm: 0.4,
            support_spacing_mm: 2.5,
        }
    }
}



/// Closed 2D contour loop (`Vec<[f64; 2]>`) in millimeter coordinates.
pub type Contour = Vec<[f64; 2]>;

/// Bed-adhesion brim configuration for the first print layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrimConfig {
    pub width_mm: f64,
    pub line_spacing_mm: f64,
}

impl Default for BrimConfig {
    fn default() -> Self {
        Self {
            width_mm: 4.0,
            line_spacing_mm: 0.4,
        }
    }
}

/// Generates concentric outward-offset bed-adhesion brim loops around `outer_loops`
/// spanning `width_mm` at pitch `line_spacing_mm`.
pub fn generate_brim(
    outer_loops: &[Contour],
    width_mm: f64,
    line_spacing_mm: f64,
) -> Result<Vec<Contour>> {
    if !width_mm.is_finite()
        || width_mm <= 0.0
        || !line_spacing_mm.is_finite()
        || line_spacing_mm < gcode_core::COORDINATE_RESOLUTION_MM
    {
        return Err(invalid(
            "TOOLPATH_INVALID_SETTINGS",
            "Brim width_mm and line_spacing_mm must be finite and positive",
        ));
    }
    if outer_loops.is_empty() {
        return Ok(Vec::new());
    }
    let mut budget = Budget::default();
    let norm = rings::nonzero(&outer_loops.to_vec())?;
    let ring_count = ((width_mm / line_spacing_mm).round() as usize).clamp(1, 64);
    let mut brim_loops = Vec::new();
    for step in 1..=ring_count {
        let distance = (step as f64 - 0.5) * line_spacing_mm;
        let expanded = outset_rings(&norm, distance, &mut budget)?;
        for ring in expanded {
            let mut area2 = 0.0;
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                area2 += a[0] * b[1] - b[0] * a[1];
            }
            if area2 > 0.0 {
                brim_loops.push(ring);
            }
        }
    }
    Ok(brim_loops)
}

fn outset_rings(source: &Rings, distance: f64, budget: &mut Budget) -> Result<Rings> {
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
    let result = rings::planar(source, &bands, "union")?;
    budget.points(point_count(&result))?;
    Ok(result)
}

#[path = "infill_planning.rs"]
mod infill_planning;
use infill_planning::generate_infill;
use infill_planning::hatch;
pub use infill_planning::plan_layers_with_skins_and_supports;

#[path = "infill_curves.rs"]
mod infill_curves;
use infill_curves::hatch_angled;
use infill_curves::generate_cubic_infill;
use infill_curves::point_in_rings;
use infill_curves::hilbert_d2xy;
use infill_curves::generate_hilbert_infill;
use infill_curves::generate_archimedean_infill;

#[path = "gcode_emission.rs"]
mod gcode_emission;
use gcode_emission::machine_profile;
use gcode_emission::planned_layers;
use gcode_emission::gcode_error;
pub use gcode_emission::emit_gcode;
pub use gcode_emission::emit_optimized_gcode;
pub use gcode_emission::emit_job_gcode;
pub use gcode_emission::emit_job_gcode_3mf;
use gcode_emission::require_settings_from_job;
use gcode_emission::optimize_input;
