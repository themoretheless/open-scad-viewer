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
                        if t > 1e-10 && t < 1. - 1e-10 && u >= -1e-10 && u <= 1. + 1e-10 {
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
    let eps = 1e-12;
    while remaining.len() > 3 {
        let n = remaining.len();
        let mut selected = None;
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
            if cross2(sub2(pb, pa), sub2(pc, pb)) <= eps {
                continue;
            }
            let min = [pa[0].min(pb[0]).min(pc[0]), pa[1].min(pb[1]).min(pc[1])];
            let max = [pa[0].max(pb[0]).max(pc[0]), pa[1].max(pb[1]).max(pc[1])];
            let blocked = remaining.iter().any(|&v| {
                *work += 1;
                let p = positions[v];
                p != pa
                    && p != pb
                    && p != pc
                    && p[0] >= min[0]
                    && p[0] <= max[0]
                    && p[1] >= min[1]
                    && p[1] <= max[1]
                    && cross2(sub2(pb, pa), sub2(p, pa)) >= -eps
                    && cross2(sub2(pc, pb), sub2(p, pb)) >= -eps
                    && cross2(sub2(pa, pc), sub2(p, pc)) >= -eps
            });
            check(
                *work <= MAX_WORK,
                "Profile triangulation work budget exceeded",
            )?;
            if !blocked {
                selected = Some((i, [a as u32, b as u32, c as u32]));
                break;
            }
        }
        let (index, triangle) = selected.ok_or_else(|| {
            crate::error("Profile cannot be triangulated without crossing its boundary")
        })?;
        indices.extend(triangle);
        remaining.remove(index);
        cursor = index % remaining.len();
    }
    let (a, b, c) = (
        positions[remaining[0]],
        positions[remaining[1]],
        positions[remaining[2]],
    );
    let final_area = cross2(sub2(b, a), sub2(c, a));
    check(
        final_area.is_finite() && final_area > 0.,
        "Profile has a degenerate final triangle",
    )?;
    indices.extend(remaining.into_iter().map(|i| i as u32));
    Ok(indices)
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
