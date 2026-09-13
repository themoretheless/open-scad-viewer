//! Boundary-preserving triangulation: duplicate bridge endpoints share coordinates.
//! No coordinate nudging is allowed: B-rep seams must retain their authored positions.
use crate::rings::area;
use crate::tessellation::FillMesh;
use crate::{Result, check};
use math_core::{cross2, sub2};

pub fn triangulate_profile(outer: &[[f64; 2]], holes: &[Vec<[f64; 2]>]) -> Result<FillMesh> {
    check(
        outer.len() >= 3 && holes.iter().all(|h| h.len() >= 3),
        "Profile rings need at least three vertices",
    )?;
    check(
        outer.len() + holes.iter().map(Vec::len).sum::<usize>() <= 2048,
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
        let mut options: Vec<_> = (0..ring.len()).collect();
        options.sort_by(|&i, &j| {
            let a = point(&m, ring[i]);
            let b = point(&m, ring[j]);
            (a[0] - hp[0])
                .hypot(a[1] - hp[1])
                .total_cmp(&(b[0] - hp[0]).hypot(b[1] - hp[1]))
        });
        let current_boundary: Vec<_> = ring.iter().map(|&i| point(&m, i)).collect();
        let index = options
            .into_iter()
            .find(|&i| {
                let p = point(&m, ring[i]);
                let d = sub2(hp, p);
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
            })
            .ok_or_else(|| crate::error("No visible bridge for profile hole"))?;
        let mut ids = Vec::new();
        for p in h {
            ids.push(m.positions.len());
            m.positions.push(p)
        }
        ids.push(ids[0]);
        ids.push(ring[index]);
        ring.splice(index + 1..index + 1, ids);
    }
    check(ring.len() <= 2048, "Profile triangulation budget exceeded")?;
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
