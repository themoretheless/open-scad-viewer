//! Boundary-preserving triangulation: duplicate bridge endpoints share coordinates.
//! No coordinate nudging is allowed: B-rep seams must retain their authored positions.
use crate::rings::area;
use crate::tessellation::FillMesh;
use crate::{Result, check};
use math_core::{cross2, sub2};

const MAX_VERTICES: usize = 4096;
const MAX_WORK: usize = 8_000_000;

pub fn triangulate_profile(outer: &[[f64; 2]], holes: &[Vec<[f64; 2]>]) -> Result<FillMesh> {
    check(
        outer.len() >= 3 && holes.iter().all(|h| h.len() >= 3),
        "Profile rings need at least three vertices",
    )?;
    let vertex_count = outer.len() + holes.iter().map(Vec::len).sum::<usize>();
    check(
        vertex_count.saturating_add(holes.len().saturating_mul(2)) <= MAX_VERTICES,
        "Profile triangulation budget exceeded",
    )?;
    check(
        outer
            .iter()
            .chain(holes.iter().flatten())
            .flatten()
            .all(|v| v.is_finite()),
        "Non-finite profile coordinate",
    )?;
    let mut m = FillMesh::default();
    let mut ring: Vec<usize> = vec![];
    for p in outer {
        ring.push(m.positions.len());
        m.positions.push(*p);
    }
    if area(outer) < 0. {
        ring.reverse()
    }
    let point = |m: &FillMesh, i: usize| m.positions[i];
    let mut work = 0;
    let mut ordered_holes = holes.to_vec();
    ordered_holes.sort_by(|a, b| {
        a.iter()
            .map(|p| p[0])
            .fold(f64::INFINITY, f64::min)
            .total_cmp(&b.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min))
    });
    for mut h in ordered_holes {
        if area(&h) > 0. {
            h.reverse()
        }
        let start = (0..h.len())
            .min_by(|&a, &b| {
                h[a][0]
                    .total_cmp(&h[b][0])
                    .then(h[a][1].total_cmp(&h[b][1]))
            })
            .unwrap();
        h.rotate_left(start);
        let hp = h[0];
        let mut options: Vec<_> = ring
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let p = point(&m, id);
                ((p[0] - hp[0]).hypot(p[1] - hp[1]), i)
            })
            .collect();
        options.sort_by(|a, b| a.0.total_cmp(&b.0));
        let current_boundary: Vec<_> = ring.iter().map(|&i| point(&m, i)).collect();
        let index = options.into_iter().map(|(_, i)| i).find(|&i| {
            let p = point(&m, ring[i]);
            let d = sub2(hp, p);
            // A bridge endpoint may occur in several sectors of the stitched
            // boundary. Visibility alone cannot select the correct occurrence.
            let prev = sub2(point(&m, ring[(i + ring.len() - 1) % ring.len()]), p);
            let next = sub2(point(&m, ring[(i + 1) % ring.len()]), p);
            let after_next = cross2(next, d);
            let before_prev = cross2(d, prev);
            let inside = if cross2(next, prev) > 0. {
                after_next >= 0. && before_prev >= 0.
            } else {
                after_next > 0. || before_prev > 0.
            };
            if !inside {
                return false;
            }
            // Charge both point containment and boundary visibility scans
            // conservatively before traversing them, including early exits.
            work += 2 * vertex_count + current_boundary.len();
            if work > MAX_WORK {
                return false;
            }
            let midpoint = [(p[0] + hp[0]) / 2., (p[1] + hp[1]) / 2.];
            if !crate::rings::contains_point(midpoint, outer)
                || holes
                    .iter()
                    .any(|hole| crate::rings::contains_point(midpoint, hole))
            {
                return false;
            }
            for boundary in std::iter::once(outer)
                .chain(holes.iter().map(Vec::as_slice))
                .chain(std::iter::once(current_boundary.as_slice()))
            {
                for k in 0..boundary.len() {
                    let a = boundary[k];
                    let b = boundary[(k + 1) % boundary.len()];
                    let e = sub2(b, a);
                    let den = cross2(d, e);
                    if den.abs() > 1e-14 {
                        let t = cross2(sub2(a, p), e) / den;
                        let u = cross2(sub2(a, p), d) / den;
                        if t > 1e-10 && t < 1. - 1e-10 && (-1e-10..=1. + 1e-10).contains(&u) {
                            return false;
                        }
                    } else if cross2(sub2(a, p), d).abs() <= 1e-14 {
                        // A bridge may touch its endpoints, but must not run
                        // along a hole edge or an earlier bridge corridor.
                        let length_squared = d[0] * d[0] + d[1] * d[1];
                        let parameter = |q: [f64; 2]| {
                            ((q[0] - p[0]) * d[0] + (q[1] - p[1]) * d[1]) / length_squared
                        };
                        let (ta, tb) = (parameter(a), parameter(b));
                        if ta.min(tb).max(0.) < ta.max(tb).min(1.) - 1e-10 {
                            return false;
                        }
                    }
                }
            }
            true
        });
        check(
            work <= MAX_WORK,
            "Profile triangulation work budget exceeded",
        )?;
        let index = index.ok_or_else(|| crate::error("No visible bridge for profile hole"))?;
        let mut ids = Vec::new();
        for p in h {
            ids.push(m.positions.len());
            m.positions.push(p)
        }
        ids.push(ids[0]);
        ids.push(ring[index]);
        ring.splice(index + 1..index + 1, ids);
    }
    m.indices = clip_ears(&m.positions, &ring, &mut work)?;
    Ok(m)
}

fn clip_ears(positions: &[[f64; 2]], ring: &[usize], work: &mut usize) -> Result<Vec<u32>> {
    let mut remaining = ring.to_vec();
    let mut indices = Vec::with_capacity((ring.len() - 2) * 3);
    // Keep scans contiguous, but do not revisit the same rejected prefix after
    // every removal. Benchmarked against indexed linked-list traversal.
    let mut cursor = 0;
    // These are the exact binary64 coordinates of the display boundary. This
    // context proves mesh decisions only, not original CAD/source geometry.
    let source = cad_predicates::SourceArena::authored(
        "display-triangulation-boundary",
        1,
        positions
            .iter()
            .flatten()
            .map(|v| cad_predicates::AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::error("Invalid triangulation predicate source"))?;
    let tolerance = cad_predicates::ToleranceContext::default_valid();
    let refs: Vec<_> = (0..positions.len())
        .map(|i| [source.leaf(2 * i).unwrap(), source.leaf(2 * i + 1).unwrap()])
        .collect();
    let orientation =
        |a: usize, b: usize, c: usize, work: &mut usize| -> Result<cad_predicates::Sign> {
            // Equal-coordinate axes establish exact collinearity directly from
            // source bits; avoid expansion construction for sampled straight edges.
            *work = work.saturating_add(1);
            check(
                *work <= MAX_WORK,
                "Profile triangulation work budget exceeded",
            )?;
            let [pa, pb, pc] = [positions[a], positions[b], positions[c]];
            if pa == pb
                || pb == pc
                || pa == pc
                || (pa[0] == pb[0] && pb[0] == pc[0])
                || (pa[1] == pb[1] && pb[1] == pc[1])
            {
                return Ok(cad_predicates::Sign::Zero);
            }
            // For normal differences/products the standard binary64 orient2d
            // forward error is less than 8*EPSILON*(|left|+|right|).
            // This deliberately loose bound includes subtraction/product/sum
            // rounding; underflow, overflow and cancellation use exact fallback.
            *work = work.saturating_add(16);
            check(
                *work <= MAX_WORK,
                "Profile triangulation work budget exceeded",
            )?;
            let dx = pa[0] - pc[0];
            let dy = pa[1] - pc[1];
            let ex = pb[0] - pc[0];
            let ey = pb[1] - pc[1];
            let left = dx * ey;
            let right = dy * ex;
            let determinant = left - right;
            let regular = |v: f64| v == 0. || v.is_normal();
            let product_regular =
                |p: f64, a: f64, b: f64| p.is_normal() || (p == 0. && (a == 0. || b == 0.));
            let magnitude = left.abs() + right.abs();
            let error = 8. * f64::EPSILON * magnitude;
            if [dx, dy, ex, ey].into_iter().all(regular)
                && product_regular(left, dx, ey)
                && product_regular(right, dy, ex)
                && determinant.is_normal()
                && magnitude.is_finite()
                && error.is_normal()
                && determinant.abs() > error
            {
                return Ok(if determinant > 0. {
                    cad_predicates::Sign::Positive
                } else {
                    cad_predicates::Sign::Negative
                });
            }
            let mut context = cad_predicates::PredicateContext::new(
                &source,
                &tolerance,
                cad_predicates::Limits {
                    max_work: (MAX_WORK.saturating_sub(*work) as u64).min(cad_predicates::MAX_WORK),
                    ..Default::default()
                },
                None,
            );
            let decision = cad_predicates::orient2d(&mut context, refs[a], refs[b], refs[c])
                .map_err(|_| crate::error("Invalid triangulation predicate reference"))?;
            *work = work.saturating_add(context.work_used() as usize);
            check(
                *work <= MAX_WORK,
                "Profile triangulation work budget exceeded",
            )?;
            match decision.outcome {
                cad_predicates::Outcome::Sign(sign) => Ok(sign),
                cad_predicates::Outcome::Indeterminate(_) => Err(crate::error(
                    "Profile triangulation predicate could not be proved",
                )),
            }
        };
    let conditioned = |pa: [f64; 2], pb: [f64; 2], pc: [f64; 2]| {
        let ab = sub2(pb, pa);
        let ac = sub2(pc, pa);
        let ab_len = ab[0].hypot(ab[1]);
        let ac_len = ac[0].hypot(ac[1]);
        // Include cancellation in coordinate subtraction. A tiny ear at
        // UV coordinates near one has larger uncertainty than the same
        // ear whose coordinates and edges are both near zero.
        let coordinate_scale = pa
            .into_iter()
            .chain(pb)
            .chain(pc)
            .map(f64::abs)
            .fold(0., f64::max);
        cross2(ab, ac).abs()
            > 64. * f64::EPSILON * (ab_len * ac_len + coordinate_scale * (ab_len + ac_len))
    };
    while remaining.len() > 3 {
        let n = remaining.len();
        let mut selected = None;
        let mut fallback = None;
        for offset in 0..n {
            let i = (cursor + offset) % n;
            *work += 1;
            check(
                *work <= MAX_WORK,
                "Profile triangulation work budget exceeded",
            )?;
            let (a, b, c) = (
                remaining[(i + n - 1) % n],
                remaining[i],
                remaining[(i + 1) % n],
            );
            let (pa, pb, pc) = (positions[a], positions[b], positions[c]);
            if orientation(a, b, c, work)? != cad_predicates::Sign::Positive {
                continue;
            }
            // Exact positive orientation can still describe a rounding sliver
            // between samples of one straight authored edge. Keep its vertex
            // in the boundary and clip a better-conditioned ear instead.
            let stable = conditioned(pa, pb, pc);
            let min = [pa[0].min(pb[0]).min(pc[0]), pa[1].min(pb[1]).min(pc[1])];
            let max = [pa[0].max(pb[0]).max(pc[0]), pa[1].max(pb[1]).max(pc[1])];
            let mut blocked = false;
            for &v in &remaining {
                *work += 1;
                let p = positions[v];
                if p != pa
                    && p != pb
                    && p != pc
                    && p[0] >= min[0]
                    && p[0] <= max[0]
                    && p[1] >= min[1]
                    && p[1] <= max[1]
                    && orientation(a, b, v, work)? != cad_predicates::Sign::Negative
                    && orientation(b, c, v, work)? != cad_predicates::Sign::Negative
                    && orientation(c, a, v, work)? != cad_predicates::Sign::Negative
                {
                    blocked = true;
                    break;
                }
            }
            check(
                *work <= MAX_WORK,
                "Profile triangulation work budget exceeded",
            )?;
            if !blocked {
                let ear = (i, [a as u32, b as u32, c as u32]);
                if stable {
                    selected = Some(ear);
                    break;
                }
                if fallback.is_none() {
                    fallback = Some(ear);
                }
            }
        }
        let (index, triangle) = selected.or(fallback).ok_or_else(|| {
            crate::error("Profile cannot be triangulated without crossing its boundary")
        })?;
        indices.extend(triangle);
        remaining.remove(index);
        cursor = index % remaining.len();
    }
    check(
        orientation(remaining[0], remaining[1], remaining[2], work)?
            == cad_predicates::Sign::Positive,
        "Profile has a degenerate final triangle",
    )?;
    indices.extend(remaining.into_iter().map(|i| i as u32));
    // Recondition rounding slivers by flipping internal diagonals only.
    // Every replacement retains the original four vertices and boundary
    // segments; exact orientations prevent crossing or inverted triangles.
    loop {
        let mut bad = false;
        let mut repair = None;
        for index in (0..indices.len()).step_by(3) {
            *work = work.saturating_add(1);
            check(
                *work <= MAX_WORK,
                "Profile triangulation work budget exceeded",
            )?;
            let last = [
                indices[index] as usize,
                indices[index + 1] as usize,
                indices[index + 2] as usize,
            ];
            if conditioned(positions[last[0]], positions[last[1]], positions[last[2]]) {
                continue;
            }
            bad = true;
            for k in 0..3 {
                let [a, b, c] = [last[k], last[(k + 1) % 3], last[(k + 2) % 3]];
                for other in (0..indices.len()).step_by(3) {
                    if other == index {
                        continue;
                    }
                    *work = work.saturating_add(1);
                    check(
                        *work <= MAX_WORK,
                        "Profile triangulation work budget exceeded",
                    )?;
                    let t = [
                        indices[other] as usize,
                        indices[other + 1] as usize,
                        indices[other + 2] as usize,
                    ];
                    let Some(j) = (0..3).find(|&j| t[j] == b && t[(j + 1) % 3] == a) else {
                        continue;
                    };
                    let d = t[(j + 2) % 3];
                    if conditioned(positions[c], positions[d], positions[b])
                        && conditioned(positions[d], positions[c], positions[a])
                        && orientation(c, d, b, work)? == cad_predicates::Sign::Positive
                        && orientation(d, c, a, work)? == cad_predicates::Sign::Positive
                    {
                        repair = Some((index, other, [c, d, b], [d, c, a]));
                        break;
                    }
                }
                if repair.is_some() {
                    break;
                }
            }
            if repair.is_some() {
                break;
            }
        }
        if !bad {
            break;
        }
        let (index, other, first, second) = repair.ok_or_else(|| {
            crate::error("Boundary triangles cannot be conditioned without changing their boundary")
        })?;
        for k in 0..3 {
            indices[index + k] = first[k] as u32;
            indices[other + k] = second[k] as u32;
        }
    }
    Ok(indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotated_sampled_straight_edges_keep_every_boundary_segment_without_slivers() {
        let corners = [[-10., -4.], [10., -4.], [10., 4.], [-10., 4.]];
        let (sin, cos) = 0.37_f64.sin_cos();
        let mut boundary = Vec::new();
        for edge in 0..4 {
            for step in 0..7 {
                let t = step as f64 / 7.;
                let a = corners[edge];
                let b = corners[(edge + 1) % 4];
                let x = a[0] + t * (b[0] - a[0]);
                let y = a[1] + t * (b[1] - a[1]);
                boundary.push([cos * x - sin * y, sin * x + cos * y]);
            }
        }
        let mesh = triangulate_profile(&boundary, &[]).unwrap();
        assert_eq!(mesh.positions, boundary);
        let mut uses = std::collections::BTreeMap::new();
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = triangle.map(|i| mesh.positions[i as usize]);
            assert!(cross2(sub2(b, a), sub2(c, a)) > 64. * f64::EPSILON);
            for k in 0..3 {
                let a = triangle[k];
                let b = triangle[(k + 1) % 3];
                *uses.entry([a.min(b), a.max(b)]).or_insert(0) += 1;
            }
        }
        for a in 0..boundary.len() as u32 {
            let b = (a + 1) % boundary.len() as u32;
            assert_eq!(uses[&[a.min(b), a.max(b)]], 1);
        }
    }
    #[test]
    fn ear_clipping_stops_at_the_work_budget() {
        let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let mut work = MAX_WORK;
        assert!(
            clip_ears(&points, &[0, 1, 2, 3], &mut work)
                .unwrap_err()
                .contains("work budget")
        );
    }
    #[test]
    fn concave_caps_with_holes_preserve_area_in_all_orientations() {
        let outer = [[0., 0.], [5., 0.], [5., 2.], [3., 2.], [3., 5.], [0., 5.]];
        let hole = [[1., 1.], [1., 2.], [2., 2.], [2., 1.]];
        for swap in [false, true] {
            for sx in [-1., 1.] {
                for sy in [-1., 1.] {
                    let map = |p: &[f64; 2]| {
                        if swap {
                            [sx * p[1] / 5., sy * p[0] / 5.]
                        } else {
                            [sx * p[0] / 5., sy * p[1] / 5.]
                        }
                    };
                    let a: Vec<_> = outer.iter().map(map).collect();
                    let h: Vec<_> = hole.iter().map(map).collect();
                    let mesh = triangulate_profile(&a, &[h])
                        .unwrap_or_else(|e| panic!("{swap} {sx} {sy}: {e:?}"));
                    let area = mesh
                        .indices
                        .as_chunks::<3>()
                        .0
                        .iter()
                        .map(|t| {
                            let [a, b, c] = [
                                mesh.positions[t[0] as usize],
                                mesh.positions[t[1] as usize],
                                mesh.positions[t[2] as usize],
                            ];
                            let area = cross2(sub2(b, a), sub2(c, a)) / 2.;
                            assert!(area > 0.);
                            area
                        })
                        .sum::<f64>();
                    assert!((area - 18. / 25.).abs() < 1e-12);
                }
            }
        }
    }
}
