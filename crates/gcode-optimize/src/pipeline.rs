use gcode_core::{JobProfile, MachineProfile, MeshBody, PlannedLayer, PlannedPath};

use crate::comb::comb_layer;
use crate::retract::virtual_retracts;
use crate::seam::rotate_seam;
use crate::simplify::simplify_path;
use crate::travel::{count_paths, path_end, travel_layers_mm};
use crate::{Budget, MAX_LAYERS, OptimizeInput, OptimizeReport, OptimizeSettings, Result, invalid};

pub fn from_planned(layers: &[PlannedLayer]) -> OptimizeInput {
    OptimizeInput {
        layers: layers.to_vec(),
        avoid: Vec::new(),
    }
}

/// Drop slicer roles: keep Z, vertices, and closed flags.
pub fn from_toolpaths(
    layers: impl IntoIterator<Item = (f64, Vec<(Vec<[f64; 2]>, bool)>)>,
) -> OptimizeInput {
    OptimizeInput {
        layers: layers
            .into_iter()
            .map(|(z_mm, paths)| PlannedLayer {
                z_mm,
                paths: paths
                    .into_iter()
                    .map(|(points, closed)| PlannedPath { points, closed })
                    .collect(),
            })
            .collect(),
        avoid: Vec::new(),
    }
}

pub fn optimize(
    input: OptimizeInput,
    settings: &OptimizeSettings,
) -> Result<(Vec<PlannedLayer>, OptimizeReport)> {
    settings.validate()?;
    if input.layers.len() > MAX_LAYERS {
        return Err(invalid(
            "OPT_LAYER_LIMIT",
            "Optimization exceeded 2048 layers",
        ));
    }
    let mut report = OptimizeReport {
        travel_before_mm: travel_layers_mm(&input.layers),
        paths_before: count_paths(&input.layers),
        ..OptimizeReport::default()
    };
    let mut budget = Budget::new();
    let mut layers = input.layers;
    for (index, layer) in layers.iter_mut().enumerate() {
        let mut simplified = Vec::with_capacity(layer.paths.len());
        for path in &layer.paths {
            let before = path.points.len();
            let next = simplify_path(path, settings.simplify_tolerance_mm, &mut budget)?;
            report.simplified_vertices += before.saturating_sub(next.points.len());
            simplified.push(next);
        }
        layer.paths = simplified;

        let mut previous = None;
        for path in &mut layer.paths {
            if rotate_seam(path, settings.seam, previous) {
                report.seam_rotations += 1;
            }
            previous = path_end(path);
        }

        let (flips, swaps) =
            crate::order::order_layer(&mut layer.paths, settings.max_2opt_swaps, &mut budget)?;
        report.order_flips += flips;
        report.two_opt_swaps += swaps;

        let contours = input.avoid.get(index).map(Vec::as_slice).unwrap_or(&[]);
        if !contours.is_empty() {
            let comb = comb_layer(&layer.paths, contours, &mut budget)?;
            report.comb_waypoints += comb.waypoints;
            report.uncombed_travels += comb.uncombed;
            layer.paths = comb.paths;
        } else {
            report.uncombed_travels += layer.paths.len().saturating_sub(1);
        }

        report.virtual_retracts += virtual_retracts(&layer.paths, settings.retract_min_travel_mm);
    }
    report.travel_after_mm = travel_layers_mm(&layers);
    report.paths_after = count_paths(&layers);
    Ok((layers, report))
}

pub fn emit_optimized(
    input: OptimizeInput,
    machine: &MachineProfile,
    settings: &OptimizeSettings,
) -> Result<String> {
    let (layers, _) = optimize(input, settings)?;
    gcode_core::emit(&layers, machine)
}

pub fn emit_optimized_3mf(
    input: OptimizeInput,
    machine: &MachineProfile,
    settings: &OptimizeSettings,
) -> Result<Vec<u8>> {
    let (layers, _) = optimize(input, settings)?;
    gcode_core::emit_3mf(&layers, machine)
}

/// Optimize then emit the machine job dialect (heat + retract).
pub fn emit_optimized_job(
    input: OptimizeInput,
    job: &JobProfile,
    settings: &OptimizeSettings,
) -> Result<String> {
    let (layers, _) = optimize(input, settings)?;
    gcode_core::emit_job(&layers, job)
}

/// Optimize then package a thick `.gcode.3mf` job with optional mesh body.
pub fn emit_optimized_gcode_3mf_job(
    input: OptimizeInput,
    job: &JobProfile,
    settings: &OptimizeSettings,
    mesh: Option<&MeshBody>,
) -> Result<Vec<u8>> {
    let (layers, _) = optimize(input, settings)?;
    gcode_core::emit_gcode_3mf_job(&layers, job, mesh)
}
