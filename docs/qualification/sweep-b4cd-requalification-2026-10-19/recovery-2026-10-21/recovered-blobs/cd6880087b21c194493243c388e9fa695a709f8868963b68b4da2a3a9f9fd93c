//! Corner-aware raw closure by disjoint perimeter gaps, without rounded lengths.
use super::super::chart::{ChartEdge, Vertex};
use super::direction;
use crate::{Result, curve::Curve};
use std::cmp::Ordering;
#[derive(Clone, Copy)]
struct Key {
    side: usize,
    value: f64,
}
fn key(p: [f64; 2], chart: [[f64; 2]; 2]) -> Option<Key> {
    if p[1] == chart[1][0] && p[0] < chart[0][1] {
        Some(Key {
            side: 0,
            value: p[0],
        })
    } else if p[0] == chart[0][1] && p[1] < chart[1][1] {
        Some(Key {
            side: 1,
            value: p[1],
        })
    } else if p[1] == chart[1][1] && p[0] > chart[0][0] {
        Some(Key {
            side: 2,
            value: p[0],
        })
    } else if p[0] == chart[0][0] && p[1] > chart[1][0] {
        Some(Key {
            side: 3,
            value: p[1],
        })
    } else {
        None
    }
}
fn compare(a: Key, b: Key) -> Ordering {
    if a.side != b.side {
        return a.side.cmp(&b.side);
    }
    // Signed zero denotes the same geometric point.
    let order = if a.value == b.value {
        Ordering::Equal
    } else {
        a.value.total_cmp(&b.value)
    };
    if a.side >= 2 { order.reverse() } else { order }
}
fn contains(probe: Key, a: Key, b: Key) -> bool {
    if compare(a, b).is_lt() {
        !compare(probe, a).is_lt() && compare(probe, b).is_lt()
    } else {
        !compare(probe, a).is_lt() || compare(probe, b).is_lt()
    }
}
pub(super) enum CornerConnectors {
    Proven {
        edges: Vec<ChartEdge>,
        pairs: Vec<[usize; 2]>,
    },
    Unresolved(&'static str),
}
pub(super) fn build(
    points: &[Vertex],
    chart: [[f64; 2]; 2],
    available_edges: usize,
) -> Result<CornerConnectors> {
    let mut ports = Vec::new();
    for (i, v) in points.iter().enumerate() {
        match (v.starts.len(), v.ends.len()) {
            (1, 1) => continue,
            (1, 0) | (0, 1) => {}
            _ => return Ok(CornerConnectors::Unresolved("contact-vertex-unproven")),
        }
        let Some(k) = key(v.point, chart) else {
            return Ok(CornerConnectors::Unresolved("interior-endpoint-unproven"));
        };
        ports.push((i, k));
    }
    ports.sort_by(|a, b| compare(a.1, b.1));
    if ports.len() % 2 != 0 || ports.windows(2).any(|p| compare(p[0].1, p[1].1).is_eq()) {
        return Ok(CornerConnectors::Unresolved(
            "perimeter-port-alias-unproven",
        ));
    }
    if ports.is_empty() {
        return Ok(CornerConnectors::Proven {
            edges: Vec::new(),
            pairs: Vec::new(),
        });
    }
    let incoming = |i: usize| !points[i].ends.is_empty();
    let offset = (0..2).find(|offset| {
        (0..ports.len() / 2).all(|pair| {
            let a = ports[(offset + 2 * pair) % ports.len()].0;
            let b = ports[(offset + 2 * pair + 1) % ports.len()].0;
            incoming(a) != incoming(b)
        })
    });
    let Some(offset) = offset else {
        return Ok(CornerConnectors::Unresolved(
            "perimeter-port-order-unproven",
        ));
    };
    let gaps: Vec<_> = (0..ports.len() / 2)
        .map(|pair| {
            let a = ports[(offset + 2 * pair) % ports.len()];
            let b = ports[(offset + 2 * pair + 1) % ports.len()];
            (a.1, b.1, if incoming(a.0) { 1i32 } else { -1i32 })
        })
        .collect();
    let mut edges = Vec::new();
    let mut pairs = Vec::new();
    for axis in 0..2 {
        let trans = 1 - axis;
        let mut levels = vec![chart[trans][0], chart[trans][1]];
        for v in points {
            if v.point[axis] == chart[axis][0] || v.point[axis] == chart[axis][1] {
                levels.push(v.point[trans]);
            }
        }
        levels.sort_by(f64::total_cmp);
        levels.dedup();
        for range in levels.windows(2) {
            let mut coefficients = [0i32; 2];
            for side in 0..2 {
                let mut start = [0.; 2];
                start[axis] = chart[axis][side];
                start[trans] = if direction(axis, side) > 0 {
                    range[0]
                } else {
                    range[1]
                };
                let Some(probe) = key(start, chart) else {
                    return Ok(CornerConnectors::Unresolved("perimeter-probe-unproven"));
                };
                coefficients[side] = gaps
                    .iter()
                    .filter(|(a, b, _)| contains(probe, *a, *b))
                    .map(|(_, _, value)| *value)
                    .sum();
            }
            if coefficients[0] != coefficients[1] {
                return Ok(CornerConnectors::Unresolved("corner-periodicity-unproven"));
            }
            let value = coefficients[0];
            if value == 0 {
                continue;
            }
            if value != 1 && value != -1 {
                return Ok(CornerConnectors::Unresolved("corner-multiplicity-unproven"));
            }
            if edges.len() + 2 > available_edges {
                return Ok(CornerConnectors::Unresolved("edge-budget"));
            }
            let first = edges.len();
            for side in 0..2 {
                let mut a = [0.; 2];
                let mut b = [0.; 2];
                a[axis] = chart[axis][side];
                b[axis] = chart[axis][side];
                let increasing = direction(axis, side) * value > 0;
                a[trans] = if increasing { range[0] } else { range[1] };
                b[trans] = if increasing { range[1] } else { range[0] };
                edges.push(ChartEdge {
                    curve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()])?,
                    source: None,
                });
            }
            pairs.push([first, first + 1]);
        }
    }
    Ok(CornerConnectors::Proven { edges, pairs })
}
