//! Boundary-preserving triangulation: duplicate bridge endpoints share coordinates.
//! No coordinate nudging is allowed: B-rep seams must retain their authored positions.
use crate::rings::area;
use crate::tessellation::FillMesh;
use crate::{check, Result};
use math_core::{cross2, sub2};

const MAX_PROFILE_VERTICES: usize = 4096;

pub fn triangulate_profile(outer: &[[f64; 2]], holes: &[Vec<[f64; 2]>]) -> Result<FillMesh> {
    check(
        outer.len() >= 3 && holes.iter().all(|h| h.len() >= 3),
        "Profile rings need at least three vertices",
    )?;
    check(
        outer.len() + holes.iter().map(Vec::len).sum::<usize>() <= MAX_PROFILE_VERTICES,
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
    for (ring_index, ring) in std::iter::once(outer)
        .chain(holes.iter().map(Vec::as_slice))
        .enumerate()
    {
        for i in 0..ring.len() {
            for (other_index, other) in std::iter::once(outer)
                .chain(holes.iter().map(Vec::as_slice))
                .enumerate()
                .skip(ring_index)
            {
                for j in 0..other.len() {
                    if ring_index == other_index
                        && (i == j || (i + 1) % ring.len() == j || (j + 1) % other.len() == i)
                    {
                        continue;
                    }
                    if segments_intersect(
                        ring[i],
                        ring[(i + 1) % ring.len()],
                        other[j],
                        other[(j + 1) % other.len()],
                    ) {
                        return Err(crate::error("Profile boundary is self-intersecting"));
                    }
                }
            }
        }
    }
    let mut m = FillMesh::default();
    let mut ring: Vec<usize> = vec![];
    for p in outer {
        ring.push(m.positions.len());
        m.positions.push(*p);
    }

    fn segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
        let orient = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| cross2(sub2(q, p), sub2(r, p));
        let (o1, o2, o3, o4) = (
            orient(a, b, c),
            orient(a, b, d),
            orient(c, d, a),
            orient(c, d, b),
        );
        ((o1 > 0.) != (o2 > 0.) && (o3 > 0.) != (o4 > 0.))
            || (o1 == 0.
                && c[0] >= a[0].min(b[0])
                && c[0] <= a[0].max(b[0])
                && c[1] >= a[1].min(b[1])
                && c[1] <= a[1].max(b[1]))
            || (o2 == 0.
                && d[0] >= a[0].min(b[0])
                && d[0] <= a[0].max(b[0])
                && d[1] >= a[1].min(b[1])
                && d[1] <= a[1].max(b[1]))
            || (o3 == 0.
                && a[0] >= c[0].min(d[0])
                && a[0] <= c[0].max(d[0])
                && a[1] >= c[1].min(d[1])
                && a[1] <= c[1].max(d[1]))
            || (o4 == 0.
                && b[0] >= c[0].min(d[0])
                && b[0] <= c[0].max(d[0])
                && b[1] >= c[1].min(d[1])
                && b[1] <= c[1].max(d[1]))
    }
    if area(outer) < 0. {
        ring.reverse()
    }
    let point = |m: &FillMesh, i: usize| m.positions[i];
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
        let mut options: Vec<(usize, [f64; 2])> =
            (0..ring.len()).map(|i| (i, point(&m, ring[i]))).collect();
        for i in 0..ring.len() {
            let a = point(&m, ring[i]);
            let b = point(&m, ring[(i + 1) % ring.len()]);
            if (a[1] - hp[1]) * (b[1] - hp[1]) <= 0. && a[1] != b[1] {
                let u = (hp[1] - a[1]) / (b[1] - a[1]);
                let x = a[0] + u * (b[0] - a[0]);
                if x < hp[0] {
                    options.push((ring.len() + i, [x, hp[1]]));
                }
            }
        }
        options.sort_by(|(_, a), (_, b)| {
            (a[0] - hp[0])
                .hypot(a[1] - hp[1])
                .total_cmp(&(b[0] - hp[0]).hypot(b[1] - hp[1]))
                .then(a[0].total_cmp(&b[0]))
                .then(a[1].total_cmp(&b[1]))
        });
        let current_boundary: Vec<_> = ring.iter().map(|&i| point(&m, i)).collect();
        let (index, target) = options
            .into_iter()
            .find_map(|(i, p)| {
                let d = sub2(hp, p);
                let midpoint = [(p[0] + hp[0]) / 2., (p[1] + hp[1]) / 2.];
                if !crate::rings::contains_point(midpoint, outer)
                    || holes
                        .iter()
                        .any(|hole| crate::rings::contains_point(midpoint, hole))
                {
                    return None;
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
                            if t > 1e-10 && t < 1. - 1e-10 && u >= -1e-10 && u <= 1. + 1e-10 {
                                return None;
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
                                return None;
                            }
                        }
                    }
                }
                Some((i, p))
            })
            .ok_or_else(|| crate::error("No visible bridge for profile hole"))?;
        let index = if index < ring.len() {
            index
        } else {
            let edge = index - ring.len();
            let id = m.positions.len();
            m.positions.push(target);
            ring.insert(edge + 1, id);
            edge + 1
        };
        let mut ids = Vec::new();
        for p in h {
            ids.push(m.positions.len());
            m.positions.push(p)
        }
        ids.push(ids[0]);
        ids.push(ring[index]);
        ring.splice(index + 1..index + 1, ids);
    }
    check(
        ring.len() <= MAX_PROFILE_VERTICES,
        "Profile triangulation budget exceeded",
    )?;
    let mut remaining = ring;
    let eps = 1e-12;
    while remaining.len() > 3 {
        let n = remaining.len();
        let mut ear = None;
        for i in 0..n {
            let a = remaining[(i + n - 1) % n];
            let b = remaining[i];
            let c = remaining[(i + 1) % n];
            let pa = point(&m, a);
            let pb = point(&m, b);
            let pc = point(&m, c);
            if cross2(sub2(pb, pa), sub2(pc, pb)) <= eps {
                continue;
            }
            let blocked = remaining.iter().any(|&v| {
                let p = point(&m, v);
                if p == pa || p == pb || p == pc {
                    return false;
                }
                cross2(sub2(pb, pa), sub2(p, pa)) >= -eps
                    && cross2(sub2(pc, pb), sub2(p, pb)) >= -eps
                    && cross2(sub2(pa, pc), sub2(p, pc)) >= -eps
            });
            if !blocked {
                ear = Some((i, [a, b, c]));
                break;
            }
        }
        if let Some((i, t)) = ear {
            m.indices.extend(t.map(|i| i as u32));
            remaining.remove(i);
        } else {
            return Err(crate::error(
                "Profile cannot be triangulated without crossing its boundary",
            ));
        }
    }
    if remaining.len() == 3 {
        m.indices.extend(remaining.into_iter().map(|i| i as u32))
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn square(x: f64, y: f64, size: f64) -> Vec<[f64; 2]> {
        vec![[x, y], [x + size, y], [x + size, y + size], [x, y + size]]
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
                        .chunks_exact(3)
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

    #[test]
    fn aligned_hole_grids_are_triangulated() {
        for side in 2..=10 {
            let holes = (0..side * side)
                .map(|i| {
                    square(
                        (i % side) as f64 * 3.0 + 1.0,
                        (i / side) as f64 * 3.0 + 1.0,
                        1.0,
                    )
                })
                .collect::<Vec<_>>();
            let mesh = triangulate_profile(&square(0.0, 0.0, side as f64 * 3.0), &holes).unwrap();
            let area = mesh
                .indices
                .chunks_exact(3)
                .map(|t| {
                    let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
                    cross2(sub2(b, a), sub2(c, a)) / 2.0
                })
                .sum::<f64>();
            assert!((area - (side * side * 8) as f64).abs() < 1e-9);
        }
    }

    #[test]
    fn self_intersecting_profiles_are_rejected() {
        let outer = vec![[0.0, 0.0], [4.0, 4.0], [0.0, 4.0], [4.0, 0.0]];
        assert!(triangulate_profile(&outer, &[]).is_err());
    }

    #[test]
    fn oversized_profiles_refuse_before_visibility_work() {
        let holes = (0..1024)
            .map(|i| {
                square(
                    (i % 32) as f64 * 2.0 + 1.0,
                    (i / 32) as f64 * 2.0 + 1.0,
                    0.5,
                )
            })
            .collect::<Vec<_>>();
        let error = triangulate_profile(&square(0.0, 0.0, 64.0), &holes).unwrap_err();
        assert_eq!(error.message, "Profile triangulation budget exceeded");
    }
}
