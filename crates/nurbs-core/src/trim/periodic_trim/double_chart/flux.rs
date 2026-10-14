//! Raw closure from integer endpoint flux on the complete chart perimeter.
use super::super::chart::{ChartEdge, Vertex};
use super::{corner::CornerConnectors, direction};
use crate::{Result, curve::Curve};
pub(super) fn build(
    points: &[Vertex],
    chart: [[f64; 2]; 2],
    available_edges: usize,
) -> Result<CornerConnectors> {
    for v in points {
        if v.starts.len() == v.ends.len() {
            continue;
        }
        if !(0..2).any(|a| (0..2).any(|s| v.point[a] == chart[a][s])) {
            return Ok(CornerConnectors::Unresolved("interior-endpoint-unproven"));
        }
    }
    let mut levels = [Vec::new(), Vec::new()];
    for axis in 0..2 {
        let trans = 1 - axis;
        levels[axis].extend(chart[trans]);
        for v in points {
            if (0..2).any(|s| v.point[axis] == chart[axis][s]) {
                levels[axis].push(v.point[trans]);
            }
        }
        levels[axis].sort_by(|a, b| {
            if a == b {
                std::cmp::Ordering::Equal
            } else {
                a.total_cmp(b)
            }
        });
        levels[axis].dedup();
    }
    let mut coefficients: [Vec<Vec<i32>>; 2] =
        std::array::from_fn(|a| vec![vec![0; levels[a].len() - 1]; 2]);
    let mut coefficient = 0i32;
    // One CCW circuit: bottom, right, top, left. Every corner is visited once,
    // so its total endpoint flux is distributed between its incident sides.
    // The free whole-perimeter constant is fixed later by winding calibration.
    for (axis, side) in [(1, 0), (0, 1), (1, 1), (0, 0)] {
        let trans = 1 - axis;
        let increasing = direction(axis, side) > 0;
        let mut indices: Vec<_> = (0..levels[axis].len() - 1).collect();
        if !increasing {
            indices.reverse();
        }
        for index in indices {
            let mut start = [0.; 2];
            start[axis] = chart[axis][side];
            start[trans] = levels[axis][index + usize::from(!increasing)];
            coefficient += points
                .iter()
                .filter(|v| v.point == start)
                .map(|v| v.ends.len() as i32 - v.starts.len() as i32)
                .sum::<i32>();
            coefficients[axis][side][index] = coefficient;
        }
    }
    if coefficient != 0 {
        return Ok(CornerConnectors::Unresolved("seam-flux-balance-unproven"));
    }
    for axis in 0..2 {
        if coefficients[axis][0] != coefficients[axis][1] {
            return Ok(CornerConnectors::Unresolved("opposite-seam-flux-unproven"));
        }
    }
    let mut edges = Vec::new();
    let mut pairs = Vec::new();
    for axis in 0..2 {
        let trans = 1 - axis;
        for (i, range) in levels[axis].windows(2).enumerate() {
            let value = coefficients[axis][0][i];
            let copies = value.unsigned_abs() as usize;
            if edges.len() + 2 * copies > available_edges {
                return Ok(CornerConnectors::Unresolved("edge-budget"));
            }
            for _ in 0..copies {
                let first = edges.len();
                for side in 0..2 {
                    let increasing = direction(axis, side) * value > 0;
                    let mut a = [0.; 2];
                    let mut b = [0.; 2];
                    a[axis] = chart[axis][side];
                    b[axis] = chart[axis][side];
                    a[trans] = range[usize::from(!increasing)];
                    b[trans] = range[usize::from(increasing)];
                    edges.push(ChartEdge {
                        curve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()])?,
                        source: None,
                    });
                }
                pairs.push([first, first + 1]);
            }
        }
    }
    Ok(CornerConnectors::Proven { edges, pairs })
}
