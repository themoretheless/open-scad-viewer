//! Directed boundary walks of a represented chord construction graph.
//! No winding selection or original-offset topology claim is made here.
use crate::{Result, check, chord_arrangement::Arrangement, distance_bounds::Interval, numeric};
use std::cmp::Ordering;
#[derive(Debug)]
pub struct Walks {
    pub counterclockwise: Vec<Vec<usize>>,
    pub clockwise: Vec<Vec<usize>>,
}
fn ends(graph: &Arrangement, half: usize) -> [usize; 2] {
    let e = graph.edges[half / 2].vertices;
    if half % 2 == 0 { e } else { [e[1], e[0]] }
}
fn upper(a: [f64; 2], b: [f64; 2]) -> bool {
    b[1] > a[1] || (b[1] == a[1] && b[0] > a[0])
}
fn angle_order(origin: [f64; 2], a: [f64; 2], b: [f64; 2]) -> Result<Ordering> {
    let ua = upper(origin, a);
    let ub = upper(origin, b);
    if ua != ub {
        return Ok(if ua {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let sign=crate::curve_offset_diagnostics::orientation(origin,a,b)?;
    numeric(matches!(sign,Some(1)|Some(-1)),"Graph has coincident rays or unresolved angular order.")?;
    Ok(if sign==Some(1) {Ordering::Less} else {Ordering::Greater})
}
pub fn walk(graph: &Arrangement) -> Result<Walks> {
    crate::chord_embedding::admit(graph,1_000_000)?;
    check(
        !graph.vertices.is_empty()
            && graph.vertices.len() <= 65536
            && !graph.edges.is_empty()
            && graph.edges.len() <= 131072,
        "Use a nonempty graph within vertex and edge limits.",
    )?;
    check(
        graph
            .vertices
            .iter()
            .all(|v| v.point.iter().all(|x| x.is_finite() && x.abs() <= 1e9)),
        "Graph coordinates exceed bounds.",
    )?;
    let mut outgoing = vec![Vec::new(); graph.vertices.len()];
    for half in 0..graph.edges.len() * 2 {
        let [a, b] = ends(graph, half);
        check(
            a < outgoing.len() && b < outgoing.len() && a != b,
            "Invalid graph edge vertices.",
        )?;
        numeric(
            graph.vertices[a].point != graph.vertices[b].point,
            "Graph has a collapsed represented edge.",
        )?;
        outgoing[a].push(half);
    }
    let mut comparisons = 0;
    for (vertex, row) in outgoing.iter_mut().enumerate() {
        // Canceled source edges may leave unused construction vertices.
        if row.is_empty() { continue; }
        check(
            row.len() >= 2 && row.len() <= 64,
            "Closed boundary graph needs vertex degree 2..64.",
        )?;
        // Insertion sorting propagates every uncertain predicate as a failure.
        for i in 1..row.len() {
            let mut j = i;
            while j > 0 {
                comparisons += 1;
                check(
                    comparisons <= 1000000,
                    "Angular comparison budget exceeded.",
                )?;
                let a = graph.vertices[ends(graph, row[j - 1])[1]].point;
                let b = graph.vertices[ends(graph, row[j])[1]].point;
                if angle_order(graph.vertices[vertex].point, a, b)? != Ordering::Greater {
                    break;
                }
                row.swap(j - 1, j);
                j -= 1;
            }
        }
        for i in 0..row.len() - 1 {
            angle_order(
                graph.vertices[vertex].point,
                graph.vertices[ends(graph, row[i])[1]].point,
                graph.vertices[ends(graph, row[i + 1])[1]].point,
            )?;
        }
    }
    let mut next = vec![0; graph.edges.len() * 2];
    for half in 0..next.len() {
        let dest = ends(graph, half)[1];
        let row = &outgoing[dest];
        let reverse = half ^ 1;
        let at = row.iter().position(|h| *h == reverse).unwrap();
        next[half] = row[(at + row.len() - 1) % row.len()];
    }
    let mut seen = vec![false; next.len()];
    let mut result = Walks {
        counterclockwise: Vec::new(),
        clockwise: Vec::new(),
    };
    for start in 0..next.len() {
        if seen[start] {
            continue;
        }
        let mut cycle = Vec::new();
        let mut half = start;
        loop {
            numeric(!seen[half], "Boundary walk merged into another cycle.")?;
            seen[half] = true;
            cycle.push(ends(graph, half)[0]);
            half = next[half];
            if half == start {
                break;
            }
        }
        check(
            cycle.len() >= 3,
            "Boundary cycle needs at least three edges.",
        )?;
        let origin = graph.vertices[cycle[0]].point;
        let mut area = Interval::point(0.);
        for i in 0..cycle.len() {
            let a = graph.vertices[cycle[i]].point;
            let b = graph.vertices[cycle[(i + 1) % cycle.len()]].point;
            let u = [
                Interval::point(a[0]).sub(Interval::point(origin[0]))?,
                Interval::point(a[1]).sub(Interval::point(origin[1]))?,
            ];
            let v = [
                Interval::point(b[0]).sub(Interval::point(origin[0]))?,
                Interval::point(b[1]).sub(Interval::point(origin[1]))?,
            ];
            area = area.add(u[0].mul(v[1])?.sub(u[1].mul(v[0])?)?)?;
        }
        numeric(
            area.lo > 0. || area.hi < 0.,
            "Boundary area sign is unresolved.",
        )?;
        if area.lo > 0. {
            result.counterclockwise.push(cycle)
        } else {
            result.clockwise.push(cycle)
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chord_arrangement, curve_offset::Segment};
    fn graph(points: &[[f64; 2]]) -> Arrangement {
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
    fn adjacent_binary64_rays_have_exact_angular_order() {
        for origin in [[0f64,0.],[1e6,-1e6]] {
            let a=[origin[0]+1.,origin[1]+1.];
            let b=[a[0],f64::from_bits(if a[1]>0. {a[1].to_bits()+1} else {a[1].to_bits()-1})];
            assert_eq!(angle_order(origin,a,b).unwrap(),Ordering::Less);
            assert_eq!(angle_order(origin,b,a).unwrap(),Ordering::Greater);assert!(angle_order(origin,a,a).is_err());
        }
    }
    #[test]
    fn square_and_reversed_square_have_one_counterclockwise_walk() {
        let points = [[0., 0.], [2., 0.], [2., 2.], [0., 2.], [0., 0.]];
        for p in [points.to_vec(), points.into_iter().rev().collect()] {
            let r = walk(&graph(&p)).unwrap();
            assert_eq!(r.counterclockwise.len(), 1);
            assert_eq!(r.clockwise.len(), 1);
            assert_eq!(r.counterclockwise[0].len(), 4);
        }
    }
    #[test]
    fn bowtie_has_two_counterclockwise_triangle_walks() {
        let r = walk(&graph(&[[0., 0.], [2., 2.], [0., 2.], [2., 0.], [0., 0.]])).unwrap();
        assert_eq!(r.counterclockwise.len(), 2);
        assert_eq!(r.clockwise.len(), 1);
        assert!(r.counterclockwise.iter().all(|c| c.len() == 3));
    }
    #[test]
    fn duplicated_rays_and_collapsed_edges_are_refused() {
        let points = [[0., 0.], [2., 0.], [2., 2.], [0., 2.], [0., 0.]];
        let mut duplicate = graph(&points);
        duplicate.edges.push(crate::chord_arrangement::Edge {
            vertices: [0, 1],
            source_edge: 0,
            source_domain: [0., 1.],
            source_parameters: [[0., 0.], [1., 1.]],
            error_upper_mm: 0.,
        });
        assert!(walk(&duplicate).is_err());
        let mut collapsed = graph(&points);
        collapsed.vertices[1].point = collapsed.vertices[0].point;
        assert!(walk(&collapsed).is_err());
    }
}
