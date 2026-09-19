use gcode_core::PlannedPath;

use crate::SeamPrefer;
use crate::travel::dist;

pub fn rotate_seam(path: &mut PlannedPath, prefer: SeamPrefer, previous: Option<[f64; 2]>) -> bool {
    if !path.closed || path.points.len() < 3 {
        return false;
    }
    let index = match prefer {
        SeamPrefer::Nearest => {
            let from = previous.unwrap_or(path.points[0]);
            path.points
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| dist(**a, from).total_cmp(&dist(**b, from)))
                .map(|(i, _)| i)
                .unwrap_or(0)
        }
        SeamPrefer::MinX => path
            .points
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])))
            .map(|(i, _)| i)
            .unwrap_or(0),
        SeamPrefer::MinY => path
            .points
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a[1].total_cmp(&b[1]).then(a[0].total_cmp(&b[0])))
            .map(|(i, _)| i)
            .unwrap_or(0),
    };
    if index == 0 {
        return false;
    }
    path.points.rotate_left(index);
    true
}
