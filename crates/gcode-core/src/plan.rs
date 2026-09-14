use crate::{invalid, MachineProfile, Result, MAX_LAYERS};

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedPath {
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedLayer {
    pub z_mm: f64,
    pub paths: Vec<PlannedPath>,
}

pub fn require_layers(layers: &[PlannedLayer]) -> Result<()> {
    if layers.len() > MAX_LAYERS {
        return Err(invalid(
            "GCODE_LAYER_LIMIT",
            "G-code preview exceeded 2048 layers",
        ));
    }
    if layers.iter().any(|layer| !layer.z_mm.is_finite()) {
        return Err(invalid(
            "GCODE_INVALID_HEIGHT",
            "Layer height must be finite",
        ));
    }
    Ok(())
}

pub fn path_vertices(path: &PlannedPath) -> Vec<[f64; 2]> {
    if path.closed && path.points.len() >= 3 {
        let mut points = path.points.clone();
        points.push(path.points[0]);
        points
    } else {
        path.points.clone()
    }
}

pub fn path_length_mm(path: &PlannedPath) -> f64 {
    let points = path_vertices(path);
    if points.len() < 2 {
        return 0.0;
    }
    points
        .array_windows()
        .map(|[a, b]| (b[0] - a[0]).hypot(b[1] - a[1]))
        .sum()
}

pub fn deposited_volume_mm3(layers: &[PlannedLayer], machine: &MachineProfile) -> f64 {
    let area = machine.bead_area_mm2();
    layers
        .iter()
        .flat_map(|layer| layer.paths.iter())
        .map(|path| path_length_mm(path) * area)
        .sum()
}
