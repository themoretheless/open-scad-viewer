//! Gcode emission: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn machine_profile(settings: &ToolpathSettings) -> gcode_core::MachineProfile {
    gcode_core::MachineProfile {
        layer_height_mm: settings.layer_height_mm,
        line_width_mm: settings.line_width_mm,
        print_feedrate_mm_s: settings.feedrate_mm_s,
        travel_feedrate_mm_s: settings.travel_feedrate_mm_s,
        filament_diameter_mm: settings.filament_diameter_mm,
    }
}

pub(super) fn planned_layers(layers: &[ToolpathLayer]) -> Result<Vec<gcode_core::PlannedLayerRef<'_>>> {
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

pub(super) fn gcode_error(error: gcode_core::Error) -> Error {
    error
}

/// One encoding of a planned toolpath. Not the only possible machine dialect.
pub fn emit_gcode(layers: &[ToolpathLayer], settings: &ToolpathSettings) -> Result<String> {
    require_settings(settings)?;
    gcode_core::emit_ref(&planned_layers(layers)?, &machine_profile(settings)).map_err(gcode_error)
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

pub(super) fn require_settings_from_job(job: &JobProfile) -> Result<()> {
    job.validate().map_err(gcode_error)
}

pub(super) fn optimize_input(layers: &[ToolpathLayer]) -> Result<gcode_optimize::OptimizeInput> {
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
