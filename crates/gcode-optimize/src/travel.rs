use gcode_core::PlannedPath;

pub fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}

pub fn path_start(path: &PlannedPath) -> Option<[f64; 2]> {
    path.points.first().copied()
}

pub fn path_end(path: &PlannedPath) -> Option<[f64; 2]> {
    if path.closed {
        path.points.first().copied()
    } else {
        path.points.last().copied()
    }
}

pub fn travel_after_path_mm(from: &PlannedPath, to: &PlannedPath) -> f64 {
    match (path_end(from), path_start(to)) {
        (Some(a), Some(b)) => dist(a, b),
        _ => 0.0,
    }
}

pub fn travel_mm(paths: &[PlannedPath]) -> f64 {
    paths
        .windows(2)
        .map(|pair| travel_after_path_mm(&pair[0], &pair[1]))
        .sum()
}

pub fn count_paths(layers: &[gcode_core::PlannedLayer]) -> usize {
    layers.iter().map(|layer| layer.paths.len()).sum()
}

pub fn travel_layers_mm(layers: &[gcode_core::PlannedLayer]) -> f64 {
    layers.iter().map(|layer| travel_mm(&layer.paths)).sum()
}
