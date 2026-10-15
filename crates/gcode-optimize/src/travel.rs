use gcode_core::PlannedPath;

#[inline]
pub fn dist_sq(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    dx * dx + dy * dy
}

#[inline]
pub fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    dist_sq(a, b).sqrt()
}

#[inline]
pub fn path_start(path: &PlannedPath) -> Option<[f64; 2]> {
    path.points.first().copied()
}

#[inline]
pub fn path_end(path: &PlannedPath) -> Option<[f64; 2]> {
    if path.closed {
        path.points.first().copied()
    } else {
        path.points.last().copied()
    }
}

#[inline]
pub fn travel_after_path_mm(from: &PlannedPath, to: &PlannedPath) -> f64 {
    match (path_end(from), path_start(to)) {
        (Some(a), Some(b)) => dist(a, b),
        _ => 0.0,
    }
}

pub fn travel_mm(paths: &[PlannedPath]) -> f64 {
    if paths.len() < 2 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..paths.len() - 1 {
        sum += travel_after_path_mm(&paths[i], &paths[i + 1]);
    }
    sum
}

pub fn count_paths(layers: &[gcode_core::PlannedLayer]) -> usize {
    layers.iter().map(|layer| layer.paths.len()).sum()
}

pub fn travel_layers_mm(layers: &[gcode_core::PlannedLayer]) -> f64 {
    layers.iter().map(|layer| travel_mm(&layer.paths)).sum()
}
