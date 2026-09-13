//! Certified offset ribbons. Ambiguous, folded or intersecting boundaries fall
//! back to the general strip union; no tolerance-based topology repair is used.
use super::*;
use crate::rings::{Rings, area, cross2};
use crate::tessellation::FillMesh;
use std::ops::Range;

struct Ribbon {
    rings: Rings,
    sides: [Vec<[f64; 2]>; 2],
    joins: [Vec<Range<usize>>; 2],
    caps: Vec<Vec<[f64; 2]>>,
    closed: bool,
}

pub(super) fn outline(
    pts: &[[f64; 2]],
    closed: bool,
    opts: &StrokeOptions,
    tolerance: f64,
) -> Result<Option<Rings>> {
    Ok(build(pts, closed, opts, tolerance)?.map(|r| r.rings))
}

pub(super) fn mesh(
    pts: &[[f64; 2]],
    closed: bool,
    opts: &StrokeOptions,
    tolerance: f64,
) -> Result<Option<FillMesh>> {
    Ok(build(pts, closed, opts, tolerance)?.and_then(Ribbon::mesh))
}

fn build(
    pts: &[[f64; 2]],
    closed: bool,
    opts: &StrokeOptions,
    tolerance: f64,
) -> Result<Option<Ribbon>> {
    let n = pts.len();
    let count = if closed { n } else { n - 1 };
    let half = opts.width * 0.5;
    let mut directions = Vec::with_capacity(count);
    let mut lengths = Vec::with_capacity(count);
    for i in 0..count {
        let d = sub2(pts[(i + 1) % n], pts[i]);
        let length = norm2(d);
        if !length.is_finite() || length <= 1e-12 {
            return Ok(None);
        }
        lengths.push(length);
        directions.push(scale2(d, 1. / length));
    }
    let mut sides = [Vec::with_capacity(n * 2), Vec::with_capacity(n * 2)];
    let mut joins = [Vec::with_capacity(n), Vec::with_capacity(n)];
    let (mut min_turn, mut max_turn, mut total_turn) = (0_f64, 0_f64, 0_f64);
    for i in 0..n {
        if !closed && (i == 0 || i == n - 1) {
            let d = directions[if i == 0 { 0 } else { count - 1 }];
            let extension = if opts.cap == LineCap::Square {
                if i == 0 { -half } else { half }
            } else {
                0.
            };
            let p = add2(pts[i], scale2(d, extension));
            sides[0].push(add2(p, scale2(perp(d), half)));
            sides[1].push(sub2(p, scale2(perp(d), half)));
            for side in 0..2 {
                joins[side].push(sides[side].len() - 1..sides[side].len());
            }
            continue;
        }
        let previous = (i + count - 1) % count;
        let next = i % count;
        let d0 = directions[previous];
        let d1 = directions[next];
        let cross = cross2(d0, d1);
        let dot = d0[0] * d1[0] + d0[1] * d1[1];
        if closed {
            min_turn = min_turn.min(cross);
            max_turn = max_turn.max(cross);
            total_turn += cross.atan2(dot);
        }
        if dot <= -0.99 {
            return Ok(None);
        }
        // Neither inward support intersection may consume its incident edge.
        // This conservative local certificate excludes short-edge folds before
        // the global boundary intersection check.
        let trim = half * cross.abs() / (1. + dot);
        if trim >= 0.45 * lengths[previous].min(lengths[next]) {
            return Ok(None);
        }
        for (side, chain) in sides.iter_mut().enumerate() {
            let first = chain.len();
            let signed_half = if side == 0 { half } else { -half };
            let p = pts[i];
            let q0 = add2(p, scale2(perp(d0), signed_half));
            let q1 = add2(p, scale2(perp(d1), signed_half));
            let miter = add2(
                p,
                scale2(add2(perp(d0), perp(d1)), signed_half / (1. + dot)),
            );
            let inward = cross * signed_half >= 0.;
            if inward || (opts.join == LineJoin::Miter && dist(p, miter) <= half * opts.miter_limit)
            {
                chain.push(miter);
            } else if opts.join == LineJoin::Round {
                let angle = (q0[1] - p[1]).atan2(q0[0] - p[0]);
                let mut arc = arc_sector(p, half, angle, cross.atan2(dot), tolerance)?;
                arc[1] = q0;
                *arc.last_mut().unwrap() = q1;
                chain.extend(arc.into_iter().skip(1));
            } else {
                chain.extend([q0, q1]);
            }
            joins[side].push(first..chain.len());
        }
    }
    let [mut left, right] = sides.clone();
    let mut caps = Vec::new();
    let mut rings = if closed {
        left.reverse();
        vec![left, right]
    } else {
        let start_left = left[0];
        let start_right = right[0];
        if opts.cap == LineCap::Round {
            let d = directions[count - 1];
            let mut arc = arc_sector(
                pts[n - 1],
                half,
                d[1].atan2(d[0]) + std::f64::consts::FRAC_PI_2,
                -std::f64::consts::PI,
                tolerance,
            )?;
            arc[1] = *left.last().unwrap();
            *arc.last_mut().unwrap() = *right.last().unwrap();
            caps.push(arc[1..].to_vec());
            left.extend(arc.into_iter().skip(1));
        }
        left.extend(right.into_iter().rev());
        if opts.cap == LineCap::Round {
            let d = directions[0];
            let mut arc = arc_sector(
                pts[0],
                half,
                d[1].atan2(d[0]) - std::f64::consts::FRAC_PI_2,
                -std::f64::consts::PI,
                tolerance,
            )?;
            arc[1] = start_right;
            *arc.last_mut().unwrap() = start_left;
            caps.push(arc[1..].to_vec());
            left.extend(arc.into_iter().skip(1));
        }
        left.reverse();
        vec![left]
    };
    for ring in &mut rings {
        ring.dedup();
        if ring.first() == ring.last() {
            ring.pop();
        }
    }
    // A single complete turn with one curvature sign certifies a convex input.
    // The local trim bound above preserves every inward support edge, proving
    // both offset boundaries are simple and disjoint without a pairwise sweep.
    let convex = closed
        && (min_turn >= 0. || max_turn <= 0.)
        && (total_turn.abs() - std::f64::consts::TAU).abs() < 1e-7;
    crate::rings::coordinate_metrics(rings.iter().flatten())?;
    if rings.iter().map(Vec::len).sum::<usize>() > crate::limits::MESH_VERTICES
        || (!convex && !simple_boundaries(&rings)?)
    {
        return Ok(None);
    }
    if closed {
        let a = area(&rings[0]);
        let b = area(&rings[1]);
        if a * b >= 0. || a + b <= 0. {
            return Ok(None);
        }
    } else if area(&rings[0]) <= 0. {
        return Ok(None);
    }
    Ok(Some(Ribbon {
        rings,
        sides,
        joins,
        caps,
        closed,
    }))
}

impl Ribbon {
    fn mesh(self) -> Option<FillMesh> {
        let right_base = self.sides[0].len();
        let mut positions = self.sides[0].clone();
        positions.extend_from_slice(&self.sides[1]);
        let mut mesh = FillMesh {
            positions,
            indices: Vec::new(),
        };
        let n = self.joins[0].len();
        let edge_count = if self.closed { n } else { n - 1 };
        for i in 0..edge_count {
            let j = (i + 1) % n;
            let a = self.joins[0][i].end - 1;
            let b = right_base + self.joins[1][i].end - 1;
            let c = right_base + self.joins[1][j].start;
            let d = self.joins[0][j].start;
            triangle(&mut mesh, a, b, c)?;
            triangle(&mut mesh, a, c, d)?;
        }
        for i in 0..n {
            for side in 0..2 {
                let range = &self.joins[side][i];
                let base = if side == 0 { 0 } else { right_base };
                let opposite_base = if side == 0 { right_base } else { 0 };
                let pivot = opposite_base + self.joins[1 - side][i].start;
                for j in range.start..range.end - 1 {
                    let (a, b) = if side == 0 { (j + 1, j) } else { (j, j + 1) };
                    triangle(&mut mesh, pivot, base + a, base + b)?;
                }
            }
        }
        for cap in self.caps {
            let base = mesh.positions.len();
            mesh.positions.extend_from_slice(&cap);
            for i in 1..cap.len() - 1 {
                triangle(&mut mesh, base, base + i + 1, base + i)?;
            }
        }
        if mesh.positions.len() > crate::limits::MESH_VERTICES
            || mesh.indices.len() / 3 > crate::limits::MESH_TRIANGLES
        {
            return None;
        }
        Some(mesh)
    }
}

fn triangle(mesh: &mut FillMesh, a: usize, b: usize, c: usize) -> Option<()> {
    let area = cross2(
        sub2(mesh.positions[b], mesh.positions[a]),
        sub2(mesh.positions[c], mesh.positions[a]),
    );
    if !area.is_finite() || area < 0. {
        return None;
    }
    if area > 0. {
        mesh.indices.extend([a as u32, b as u32, c as u32]);
    }
    Some(())
}

/// Sort segment bounds, then reject every nonadjacent contact, including
/// collinear overlap. A bounded candidate count makes pathological inputs fall
/// back rather than adding an unbounded quadratic pass to the general solver.
fn simple_boundaries(rings: &Rings) -> Result<bool> {
    let (scale, extent) = crate::rings::coordinate_metrics(rings.iter().flatten())?;
    let epsilon = (scale * f64::EPSILON * 64.).max(extent * 1e-12);
    let mut edges = Vec::new();
    for (r, ring) in rings.iter().enumerate() {
        if ring.len() < 3 {
            return Ok(false);
        }
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let c = ring[(i + 2) % ring.len()];
            let ab = sub2(b, a);
            let bc = sub2(c, b);
            if norm2(ab) <= epsilon {
                return Ok(false);
            }
            if cross2(ab, bc).abs() <= epsilon * (norm2(ab) + norm2(bc))
                && ab[0] * bc[0] + ab[1] * bc[1] < 0.
            {
                return Ok(false);
            }
            edges.push((a, b, r, i));
        }
    }
    if edges.len() > crate::limits::MESH_VERTICES {
        return Ok(false);
    }
    edges.sort_unstable_by(|a, b| a.0[0].min(a.1[0]).total_cmp(&b.0[0].min(b.1[0])));
    let mut candidates = 0;
    for i in 0..edges.len() {
        let (a, b, ring_a, id_a) = edges[i];
        let max_x = a[0].max(b[0]) + epsilon;
        for &(c, d, ring_b, id_b) in &edges[i + 1..] {
            if c[0].min(d[0]) > max_x {
                break;
            }
            candidates += 1;
            if candidates > 1_000_000 {
                return Ok(false);
            }
            if ring_a == ring_b
                && ((id_a + 1) % rings[ring_a].len() == id_b
                    || (id_b + 1) % rings[ring_a].len() == id_a)
            {
                continue;
            }
            if a[1].min(b[1]) > c[1].max(d[1]) + epsilon
                || c[1].min(d[1]) > a[1].max(b[1]) + epsilon
            {
                continue;
            }
            let separated = |a, b, c, d| {
                let ab = sub2(b, a);
                let first = cross2(ab, sub2(c, a));
                let second = cross2(ab, sub2(d, a));
                let e = epsilon * norm2(ab);
                (first > e && second > e) || (first < -e && second < -e)
            };
            if !separated(a, b, c, d) && !separated(c, d, a, b) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certified_ribbons_match_strip_union_for_open_closed_and_reversed_paths() {
        let mut accepted = 0;
        for seed in 0..8 {
            for closed in [false, true] {
                let mut pts: Vec<_> = (0..24)
                    .map(|i| {
                        let t = i as f64 / 24.;
                        if closed {
                            let angle = t * std::f64::consts::TAU;
                            let radius = 10. + (angle * 3. + seed as f64).sin();
                            [angle.cos() * radius, angle.sin() * radius]
                        } else {
                            [t * 30., (t * 7. + seed as f64).sin() * 3.]
                        }
                    })
                    .collect();
                if seed % 2 == 1 {
                    pts.reverse();
                }
                for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
                    for join in [LineJoin::Miter, LineJoin::Bevel, LineJoin::Round] {
                        for width in [0.1, 0.5, 2.] {
                            let opts = StrokeOptions {
                                width,
                                cap,
                                join,
                                ..Default::default()
                            };
                            let Some(actual) = outline(&pts, closed, &opts, 0.005).unwrap() else {
                                continue;
                            };
                            accepted += 1;
                            let mesh = mesh(&pts, closed, &opts, 0.005)
                                .unwrap()
                                .expect("certified mesh");
                            let triangles: Vec<_> = mesh
                                .indices
                                .chunks_exact(3)
                                .map(|ids| {
                                    [
                                        mesh.positions[ids[0] as usize],
                                        mesh.positions[ids[1] as usize],
                                        mesh.positions[ids[2] as usize],
                                    ]
                                })
                                .collect();
                            let mut pieces = vec![];
                            stroke_pieces(&pts, closed, &opts, 0.005, &mut pieces).unwrap();
                            let expected = crate::rings::nonzero(&pieces).unwrap();
                            let a: f64 = actual.iter().map(|r| area(r)).sum();
                            let e: f64 = expected.iter().map(|r| area(r)).sum();
                            let mesh_area: f64 = triangles
                                .iter()
                                .map(|t| cross2(sub2(t[1], t[0]), sub2(t[2], t[0])) * 0.5)
                                .sum();
                            assert!(
                                (mesh_area - e).abs() < 1e-7 * e.max(1.),
                                "mesh area {mesh_area} != {e}"
                            );
                            assert!(
                                (a - e).abs() < 1e-7 * e.max(1.),
                                "seed={seed} closed={closed} {cap:?}/{join:?} width={width}: {a} != {e}"
                            );
                            for x in (-13..33).step_by(3) {
                                for y in (-13..13).step_by(3) {
                                    let p = [x as f64 + 0.317, y as f64 + 0.271];
                                    assert_eq!(
                                        crate::rings::inside(p, &actual),
                                        crate::rings::inside(p, &expected),
                                        "seed={seed} closed={closed} {cap:?}/{join:?} width={width} p={p:?}"
                                    );
                                    let covers = triangles
                                        .iter()
                                        .filter(|t| {
                                            (0..3).all(|i| {
                                                cross2(sub2(t[(i + 1) % 3], t[i]), sub2(p, t[i]))
                                                    > 0.
                                            })
                                        })
                                        .count();
                                    assert_eq!(
                                        covers,
                                        usize::from(crate::rings::inside(p, &expected)),
                                        "overlap/gap: seed={seed} closed={closed} {cap:?}/{join:?} width={width} p={p:?}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(
            accepted >= 350,
            "only {accepted} cases exercised the fast path"
        );
    }

    #[test]
    fn convex_mesh_has_two_triangles_per_edge_and_preserves_translated_area() {
        let n = 4096;
        for shift in [0., 1e7] {
            let pts: Vec<_> = (0..n)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / n as f64;
                    [shift + a.cos() * 10., shift + a.sin() * 10.]
                })
                .collect();
            let opts = StrokeOptions {
                width: 0.4,
                ..Default::default()
            };
            let mesh = mesh(&pts, true, &opts, 0.01).unwrap().unwrap();
            assert_eq!(mesh.positions.len(), n * 2);
            assert_eq!(mesh.triangle_count(), n * 2);
            let actual: f64 = mesh
                .indices
                .chunks_exact(3)
                .map(|ids| {
                    let a = mesh.positions[ids[0] as usize];
                    cross2(
                        sub2(mesh.positions[ids[1] as usize], a),
                        sub2(mesh.positions[ids[2] as usize], a),
                    ) * 0.5
                })
                .sum();
            let expected = opts.width * (0..n).map(|i| dist(pts[i], pts[(i + 1) % n])).sum::<f64>();
            assert!(
                (actual - expected).abs() < expected * 2e-6,
                "{actual} != {expected}"
            );
        }
        let star: Vec<_> = (0..5)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU * 2. / 5.;
                [a.cos() * 10., a.sin() * 10.]
            })
            .collect();
        assert!(
            outline(
                &star,
                true,
                &StrokeOptions {
                    width: 0.1,
                    ..Default::default()
                },
                0.01
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn crossing_touching_retraced_and_consumed_strokes_use_general_solver() {
        for (pts, closed, width) in [
            (vec![[0., 0.], [10., 10.], [0., 10.], [10., 0.]], false, 1.),
            (vec![[0., 0.], [10., 0.], [0., 0.]], false, 1.),
            (vec![[0., 0.], [10., 0.], [10., 1.], [0., 1.]], false, 2.),
            (vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]], true, 20.),
            (vec![[0., 0.], [10., 10.], [0., 10.], [10., 0.]], true, 0.1),
        ] {
            for join in [LineJoin::Miter, LineJoin::Bevel, LineJoin::Round] {
                assert!(
                    outline(
                        &pts,
                        closed,
                        &StrokeOptions {
                            width,
                            join,
                            ..Default::default()
                        },
                        0.01
                    )
                    .unwrap()
                    .is_none()
                );
            }
        }
        assert!(
            !simple_boundaries(&vec![vec![
                [0., 0.],
                [4., 0.],
                [2., 0.],
                [2., 2.],
                [0., 2.]
            ]])
            .unwrap()
        );
        assert!(
            !simple_boundaries(&vec![
                vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
                vec![[4., 1.], [5., 1.], [5., 2.], [4., 2.]]
            ])
            .unwrap()
        );
    }
}
