//! Interior witnesses adjacent to a represented counterclockwise boundary walk.
use crate::{
    Result, check, chord_arrangement::Arrangement, chord_winding, curve_offset::Segment,
    curve_offset_diagnostics::orientation, distance_bounds::Interval,
    foundation::guards::Budget, numeric,
};
fn box_apart(line: [[f64; 2]; 2], box_: [[f64; 2]; 2]) -> Result<bool> {
    if (0..2)
        .any(|k| line[0][k].max(line[1][k]) < box_[0][k] || line[0][k].min(line[1][k]) > box_[1][k])
    {
        return Ok(true);
    }
    let corners = [
        [box_[0][0], box_[0][1]],
        [box_[0][0], box_[1][1]],
        [box_[1][0], box_[0][1]],
        [box_[1][0], box_[1][1]],
    ];
    let mut side = None;
    for p in corners {
        let current = orientation(line[0], line[1], p)?;
        if !matches!(current, Some(-1) | Some(1)) {
            return Ok(false);
        }
        if let Some(previous) = side {
            if current != Some(previous) {
                return Ok(false);
            }
        } else {
            side = current
        }
    }
    Ok(true)
}
pub fn interior(graph: &Arrangement, cycle: &[usize], max_checks: usize) -> Result<[f64; 2]> {
    witness(graph, cycle, max_checks, true)
}
/// Prove a point adjacent to the left side, including clockwise hole walks.
pub fn left(graph: &Arrangement, cycle: &[usize], max_checks: usize) -> Result<[f64; 2]> {
    witness(graph, cycle, max_checks, false)
}
fn witness(graph: &Arrangement, cycle: &[usize], max_checks: usize, require_inside: bool) -> Result<[f64; 2]> {
    check(
        cycle.len() >= 3
            && cycle.len() <= 65536
            && (1..=1000000).contains(&max_checks)
            && cycle.iter().all(|i| *i < graph.vertices.len()),
        "Use a bounded boundary walk and witness budget.",
    )?;
    check(
        !graph.edges.is_empty()
            && graph.edges.len() <= 131072
            && graph
                .edges
                .iter()
                .all(|e| e.vertices.iter().all(|i| *i < graph.vertices.len()))
            && graph
                .vertices
                .iter()
                .all(|v| v.point.iter().all(|x| x.is_finite() && x.abs() <= 1e9)),
        "Invalid represented witness graph.",
    )?;
    let edges: std::collections::BTreeSet<_> = graph
        .edges
        .iter()
        .map(|e| {
            let [a, b] = e.vertices;
            if a < b { [a, b] } else { [b, a] }
        })
        .collect();
    check(
        cycle.iter().enumerate().all(|(i, a)| {
            let b = cycle[(i + 1) % cycle.len()];
            edges.contains(&if *a < b { [*a, b] } else { [b, *a] })
        }),
        "Boundary walk contains a missing graph edge.",
    )?;
    let boundary: Vec<_> = cycle
        .iter()
        .enumerate()
        .map(|(i, a)| Segment {
            points: [
                graph.vertices[*a].point,
                graph.vertices[cycle[(i + 1) % cycle.len()]].point,
            ],
            domain: [i as f64, (i + 1) as f64],
            error_upper_mm: 0.,
        })
        .collect();
    let mut checks = 0;
    // Unified guard as a typed backstop for the explicit `max_checks` budget.
    let mut guard = Budget::with_iterations(max_checks + 1)?.guard("chord_witness");
    for pair in &boundary {
        let [a, b] = pair.points;
        let direction = [b[0] - a[0], b[1] - a[1]];
        let length = direction[0].hypot(direction[1]);
        if length == 0. {
            continue;
        }
        let middle = [
            Interval::point(a[0])
                .add(Interval::point(b[0]))?
                .mul(Interval::point(0.5))?,
            Interval::point(a[1])
                .add(Interval::point(b[1]))?
                .mul(Interval::point(0.5))?,
        ];
        let center = middle.map(|i| i.lo * 0.5 + i.hi * 0.5);
        let mut step = length / 8.;
        for _ in 0..64 {
            let point = [
                center[0] - direction[1] / length * step,
                center[1] + direction[0] / length * step,
            ];
            step *= 0.5;
            if !point.iter().all(|v| v.is_finite() && v.abs() <= 1e9)
                || orientation(a, b, point)? != Some(1)
            {
                continue;
            }
            let box_ = [
                [middle[0].lo.min(point[0]), middle[1].lo.min(point[1])],
                [middle[0].hi.max(point[0]), middle[1].hi.max(point[1])],
            ];
            let mut clear = true;
            for edge in &graph.edges {
                let line = edge.vertices.map(|i| graph.vertices[i].point);
                if line == [a, b] || line == [b, a] {
                    continue;
                }
                check(checks < max_checks, "Interior witness budget exceeded.")?;
                guard.tick()?;
                checks += 1;
                if !box_apart(line, box_)? {
                    clear = false;
                    break;
                }
            }
            if clear && (!require_inside || matches!(chord_winding::at(&boundary, point), Ok(1))) {
                return Ok(point);
            }
        }
    }
    numeric(false, "No separated interior witness was proved.")?;
    unreachable!()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chord_arrangement, chord_faces};
    fn fixture(points: &[[f64; 2]]) -> Arrangement {
        let chain: Vec<_> = points
            .windows(2)
            .enumerate()
            .map(|(i, p)| Segment {
                points: [p[0], p[1]],
                domain: [i as f64, (i + 1) as f64],
                error_upper_mm: 0.,
            })
            .collect();
        chord_arrangement::split(&chain, true, 1e-6, 100).unwrap()
    }
    #[test]
    fn concave_and_crossed_contours_have_proved_interior_witnesses() {
        for points in [
            vec![
                [0., 0.],
                [8., 0.],
                [8., 1.],
                [1., 1.],
                [1., 8.],
                [0., 8.],
                [0., 0.],
            ],
            vec![[0., 0.], [4., 4.], [0., 4.], [4., 0.], [0., 0.]],
        ] {
            let graph = fixture(&points);
            for cycle in chord_faces::walk(&graph).unwrap().counterclockwise {
                let point = interior(&graph, &cycle, 10000).unwrap();
                assert!(point.iter().all(|v| v.is_finite()));
            }
        }
    }
    #[test]
    fn witness_budget_never_returns_an_unchecked_point() {
        let graph = fixture(&[[0., 0.], [4., 0.], [4., 4.], [0., 4.], [0., 0.]]);
        let cycle = chord_faces::walk(&graph)
            .unwrap()
            .counterclockwise
            .remove(0);
        assert!(interior(&graph, &cycle, 1).is_err());
    }
}
