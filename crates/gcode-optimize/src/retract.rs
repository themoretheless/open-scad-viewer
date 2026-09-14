use gcode_core::PlannedPath;

use crate::travel::travel_after_path_mm;

pub fn virtual_retracts(paths: &[PlannedPath], min_travel_mm: f64) -> usize {
    if min_travel_mm == 0.0 {
        return 0;
    }
    paths
        .windows(2)
        .filter(|pair| {
            !is_waypoint(&pair[0])
                && !is_waypoint(&pair[1])
                && travel_after_path_mm(&pair[0], &pair[1]) >= min_travel_mm
        })
        .count()
}

fn is_waypoint(path: &PlannedPath) -> bool {
    !path.closed && path.points.len() == 1
}
