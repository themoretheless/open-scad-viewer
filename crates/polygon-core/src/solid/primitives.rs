//! Mesh primitives: cube / cylinder / sphere, join, hull, clean.
//! Closed-ring boolean lives in `planar_geometry::rings`.
use crate::{Mesh, Result, check, cross, dot, norm, sub};
use planar_geometry::rings::{Rings, area, cross2, inside, planar, sub2};
use std::collections::{BTreeSet, HashMap};

pub fn empty() -> Mesh {
    Mesh {
        positions: vec![],
        indices: vec![],
        uv: None,
    }
}
pub fn join(meshes: &[Mesh]) -> Result<Mesh> {
    let mut out = empty();
    for m in meshes {
        m.validate()?;
        let base = out.positions.len() / 3;
        out.positions.extend(&m.positions);
        out.indices.extend(m.indices.iter().map(|i| i + base));
    }
    out.validate()?;
    Ok(out)
}
pub fn cube(size: [f64; 3], center: bool) -> Result<Mesh> {
    if size.contains(&0.) {
        return Ok(empty());
    }
    let (x, y, z) = (size[0], size[1], size[2]);
    let o = if center {
        [-x / 2., -y / 2., -z / 2.]
    } else {
        [0., 0., 0.]
    };
    let p = [
        [o[0], o[1], o[2]],
        [o[0] + x, o[1], o[2]],
        [o[0] + x, o[1] + y, o[2]],
        [o[0], o[1] + y, o[2]],
        [o[0], o[1], o[2] + z],
        [o[0] + x, o[1], o[2] + z],
        [o[0] + x, o[1] + y, o[2] + z],
        [o[0], o[1] + y, o[2] + z],
    ];
    let mut m = empty();
    m.positions = p.into_iter().flatten().collect();
    m.indices = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6,
        3, 0, 4, 3, 4, 7,
    ];
    Ok(m)
}
pub fn cylinder(height: f64, r1: f64, r2: f64, n: usize, center: bool) -> Result<Mesh> {
    check(
        (3..=512).contains(&n) && height > 0. && r1 >= 0. && r2 >= 0.,
        "Invalid cylinder",
    )?;
    let mut m = empty();
    for (r, z) in [(r1, 0.), (r2, height)] {
        for i in 0..n {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            m.positions.extend([
                r * a.cos(),
                r * a.sin(),
                z - if center { height / 2. } else { 0. },
            ]);
        }
    }
    for i in 0..n {
        let j = (i + 1) % n;
        m.indices.extend([i, j, j + n, i, j + n, i + n]);
    }
    for i in 1..n - 1 {
        m.indices.extend([0, i + 1, i, n, n + i, n + i + 1]);
    }
    clean(m)
}
pub fn sphere(radius: f64, n: usize) -> Result<Mesh> {
    check(radius > 0. && (4..=512).contains(&n), "Invalid sphere")?;
    let rows = (n / 2).max(2);
    let mut m = empty();
    for j in 0..=rows {
        let a = std::f64::consts::PI * j as f64 / rows as f64;
        for i in 0..n {
            let b = std::f64::consts::TAU * i as f64 / n as f64;
            let r = if j == 0 || j == rows {
                0.
            } else {
                radius * a.sin()
            };
            m.positions
                .extend([r * b.cos(), r * b.sin(), radius * a.cos()]);
        }
    }
    for j in 0..rows {
        for i in 0..n {
            let a = j * n + i;
            let b = j * n + (i + 1) % n;
            m.indices.extend([a, a + n, b + n, a, b + n, b]);
        }
    }
    clean(m)
}
pub fn clean(mut m: Mesh) -> Result<Mesh> {
    m = crate::solid::proximity::weld_exact(&m)?;
    let mut indices = Vec::new();
    for t in m.indices.as_chunks::<3>().0 {
        let a = m.point(t[0])?;
        let b = m.point(t[1])?;
        let c = m.point(t[2])?;
        if norm(cross(sub(b, a), sub(c, a))) > 1e-16 {
            indices.extend(t)
        }
    }
    m.indices = indices;
    if !m.indices.is_empty() && m.inspect()?.signed_volume_mm3 < 0. {
        m.reverse_winding()
    }
    Ok(m)
}
pub fn hull3(meshes: &[Mesh]) -> Result<Mesh> {
    let mut p: Vec<[f64; 3]> = meshes
        .iter()
        .flat_map(|m| {
            m.positions
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| [p[0], p[1], p[2]])
        })
        .collect();
    p.sort_by(|a, b| {
        a[0].total_cmp(&b[0])
            .then(a[1].total_cmp(&b[1]))
            .then(a[2].total_cmp(&b[2]))
    });
    p.dedup();
    check(p.len() <= 20000, "Hull point budget exceeded")?;
    if p.len() < 4 {
        return Ok(empty());
    }
    let a = 0;
    let b = p.len() - 1;
    let c = (0..p.len())
        .max_by(|&i, &j| {
            norm(cross(sub(p[b], p[a]), sub(p[i], p[a])))
                .total_cmp(&norm(cross(sub(p[b], p[a]), sub(p[j], p[a]))))
        })
        .unwrap();
    let n = cross(sub(p[b], p[a]), sub(p[c], p[a]));
    let d = (0..p.len())
        .max_by(|&i, &j| {
            dot(n, sub(p[i], p[a]))
                .abs()
                .total_cmp(&dot(n, sub(p[j], p[a])).abs())
        })
        .unwrap();
    if dot(n, sub(p[d], p[a])).abs() < 1e-15 {
        return Ok(empty());
    }
    let center = std::array::from_fn(|k| (p[a][k] + p[b][k] + p[c][k] + p[d][k]) / 4.);
    let orient = |mut f: [usize; 3]| {
        if dot(
            cross(sub(p[f[1]], p[f[0]]), sub(p[f[2]], p[f[0]])),
            sub(center, p[f[0]]),
        ) > 0.
        {
            f.swap(1, 2)
        }
        f
    };
    let mut faces = vec![
        orient([a, b, c]),
        orient([a, d, b]),
        orient([a, c, d]),
        orient([b, d, c]),
    ];
    let eps = norm(sub(p[b], p[a])) * 1e-10;
    for i in 0..p.len() {
        let mut edge = BTreeSet::new();
        faces.retain(|f| {
            let normal = cross(sub(p[f[1]], p[f[0]]), sub(p[f[2]], p[f[0]]));
            if dot(normal, sub(p[i], p[f[0]])) > eps * norm(normal) {
                for j in 0..3 {
                    let x = f[j];
                    let y = f[(j + 1) % 3];
                    if !edge.remove(&(y, x)) {
                        edge.insert((x, y));
                    }
                }
                false
            } else {
                true
            }
        });
        for (x, y) in edge {
            faces.push(orient([x, y, i]));
        }
        check(faces.len() <= 20000, "Hull face budget exceeded")?;
    }
    clean(Mesh {
        positions: p.into_iter().flatten().collect(),
        indices: faces.into_iter().flatten().collect(),
        uv: None,
    })
}

pub fn minkowski(a: &Mesh, b: &Mesh) -> Result<Mesh> {
    fn vertices(m: &Mesh) -> Vec<[f64; 3]> {
        let ids: BTreeSet<_> = m.indices.iter().copied().collect();
        ids.into_iter().map(|i| m.point(i).unwrap()).collect()
    }
    let a_points = vertices(a);
    let b_points = vertices(b);
    check(
        a_points.len().saturating_mul(b_points.len()) <= 20000,
        "Minkowski vertex budget exceeded",
    )?;
    for (m, p) in [(a, &a_points), (b, &b_points)] {
        for t in m.indices.as_chunks::<3>().0 {
            let u = m.point(t[0])?;
            let n = cross(sub(m.point(t[1])?, u), sub(m.point(t[2])?, u));
            check(
                p.iter().all(|&v| dot(n, sub(v, u)) <= norm(n) * 1e-8),
                "Own 3D Minkowski currently requires convex operands",
            )?;
        }
    }
    let mut mesh = empty();
    for a in a_points {
        for b in &b_points {
            mesh.positions
                .extend([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
        }
    }
    hull3(&[mesh])
}

/// Collapse coplanar triangulation introduced by successive CSG operations.
/// Boundary rings (including holes) remain authoritative; no shape approximation.
pub fn simplify(mesh: &Mesh) -> Result<Mesh> {
    let mut groups: Vec<([f64; 3], f64, Vec<usize>)> = vec![];
    let extent = mesh.positions.iter().fold(1_f64, |a, b| a.max(b.abs()));
    let eps = extent * 1e-9;
    for t in mesh.indices.as_chunks::<3>().0 {
        let a = mesh.point(t[0])?;
        let normal = cross(sub(mesh.point(t[1])?, a), sub(mesh.point(t[2])?, a));
        let len = norm(normal);
        if len < 1e-16 {
            continue;
        }
        let n = normal.map(|v| v / len);
        let d = dot(n, a);
        if let Some(g) = groups
            .iter_mut()
            .find(|(m, k, _)| dot(n, *m) > 1. - 1e-12 && (d - k).abs() < eps)
        {
            g.2.extend(t)
        } else {
            groups.push((n, d, t.to_vec()))
        }
    }
    let mut pieces = vec![];
    for (n, _, indices) in groups {
        let part = Mesh {
            positions: mesh.positions.clone(),
            indices,
            uv: None,
        };
        let loops = match part.boundary_loops() {
            Ok(loops) => loops,
            Err(_) => {
                pieces.push(part);
                continue;
            }
        };
        if loops.is_empty() {
            continue;
        }
        let origin = part.point(loops[0][0])?;
        let delta = sub(part.point(loops[0][1])?, origin);
        let len = norm(delta);
        let u = delta.map(|v| v / len);
        let v = cross(n, u);
        let mut rings: Rings = loops
            .into_iter()
            .map(|ring| {
                ring[..ring.len() - 1]
                    .iter()
                    .map(|&i| {
                        let p = sub(part.point(i).unwrap(), origin);
                        [dot(p, u), dot(p, v)]
                    })
                    .collect()
            })
            .collect();
        for r in &mut rings {
            loop {
                if r.len() <= 3 {
                    break;
                }
                let count = r.len();
                let at = (0..count).find(|&i| {
                    let a = sub2(r[i], r[(i + count - 1) % count]);
                    let b = sub2(r[(i + 1) % count], r[i]);
                    cross2(a, b).abs() < eps * (a[0].hypot(a[1]) + b[0].hypot(b[1]))
                });
                if let Some(i) = at {
                    r.remove(i);
                } else {
                    break;
                }
            }
        }
        for outer in rings.iter().filter(|r| area(r) > 0.) {
            let holes = rings
                .iter()
                .filter(|r| area(r) < 0. && inside(r[0], &vec![outer.clone()]))
                .cloned()
                .collect();
            let mut cap = triangulate(&crate::solid::modeling::Profile {
                outer: outer.clone(),
                holes,
            })?;
            for p in cap.positions.as_chunks_mut::<3>().0 {
                let point =
                    std::array::from_fn::<_, 3, _>(|k| origin[k] + p[0] * u[k] + p[1] * v[k]);
                p.copy_from_slice(&point)
            }
            pieces.push(cap);
        }
    }
    let mut out = join(&pieces)?;
    let mut cells = HashMap::<[i64; 3], Vec<usize>>::new();
    let mut positions = vec![];
    let mut remap = vec![];
    for p in out.positions.as_chunks::<3>().0 {
        let p = [p[0], p[1], p[2]];
        let cell = p.map(|x| (x / eps).floor() as i64);
        let mut found = None;
        'near: for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(ids) = cells.get(&[cell[0] + x, cell[1] + y, cell[2] + z]) {
                        for &i in ids {
                            let q = [positions[3 * i], positions[3 * i + 1], positions[3 * i + 2]];
                            if norm(sub(p, q)) < eps {
                                found = Some(i);
                                break 'near;
                            }
                        }
                    }
                }
            }
        }
        let i = found.unwrap_or_else(|| {
            let i = positions.len() / 3;
            positions.extend(p);
            cells.entry(cell).or_default().push(i);
            i
        });
        remap.push(i)
    }
    for i in &mut out.indices {
        *i = remap[*i]
    }
    out.positions = positions;
    if out.inspect()?.closed {
        Ok(out)
    } else {
        crate::solid::boolean::stitch_mesh(&out, eps)
    }
}

/// Constrained ear clipping with visible bridges for holes. It reuses all input
/// boundary coordinates, so neighboring solid faces acquire no extra T junctions.
pub fn triangulate(profile: &crate::solid::modeling::Profile) -> Result<Mesh> {
    let fill = planar_geometry::triangulation::triangulate_profile(&profile.outer, &profile.holes)?;
    let mesh = Mesh {
        positions: fill
            .positions
            .iter()
            .flat_map(|p| [p[0], p[1], 0.])
            .collect(),
        indices: fill.indices.iter().map(|&i| i as usize).collect(),
        uv: None,
    };
    mesh.validate()?;
    Ok(mesh)
}

pub fn halfspace(mesh: &Mesh, normal: [f64; 3], offset: f64) -> Result<Mesh> {
    let length = norm(normal);
    check(length > 0. && offset.is_finite(), "Invalid cutting plane")?;
    let n = normal.map(|x| x / length);
    let a = if n[0].abs() < 0.8 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let u = cross(n, a);
    let ul = norm(u);
    let u = u.map(|x| x / ul);
    let v = cross(n, u);
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in mesh.positions.as_chunks::<3>().0 {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k])
        }
    }
    let c = std::array::from_fn(|k| (min[k] + max[k]) / 2.);
    let distance = offset - dot(c, n);
    let origin = std::array::from_fn::<_, 3, _>(|k| c[k] + distance * n[k]);
    let extent = norm(sub(max, min)) + distance.abs() + 1.;
    let mut box_mesh = cube([2. * extent, 2. * extent, extent], false)?;
    for p in box_mesh.positions.as_chunks_mut::<3>().0 {
        let q = std::array::from_fn::<_, 3, _>(|k| {
            origin[k] + (p[0] - extent) * u[k] + (p[1] - extent) * v[k] + p[2] * n[k]
        });
        p.copy_from_slice(&q)
    }
    Ok(box_mesh)
}

/// Exact specialization for vertical prisms with coincident end planes. Reduces
/// repeated drilled-hole CSG to a planar arrangement and one triangulation.
pub fn prism_boolean(a: &Mesh, b: &Mesh, operation: &str) -> Result<Option<Mesh>> {
    fn bounds(m: &Mesh) -> Option<(f64, f64)> {
        if m.indices.is_empty() {
            return None;
        }
        let lo = m
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| p[2])
            .fold(f64::INFINITY, f64::min);
        let hi = m
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| p[2])
            .fold(f64::NEG_INFINITY, f64::max);
        let eps = (hi - lo).abs().max(1.) * 1e-9;
        if hi - lo <= eps {
            return None;
        }
        for t in m.indices.as_chunks::<3>().0 {
            let a = m.point(t[0]).ok()?;
            let b = m.point(t[1]).ok()?;
            let c = m.point(t[2]).ok()?;
            let n = cross(sub(b, a), sub(c, a));
            if n[2].abs() > norm(n) * 1e-9
                && (![a, b, c].iter().all(|p| (p[2] - lo).abs() < eps)
                    && ![a, b, c].iter().all(|p| (p[2] - hi).abs() < eps))
            {
                return None;
            }
        }
        Some((lo, hi))
    }
    let (Some((lo, hi)), Some((other_lo, other_hi))) = (bounds(a), bounds(b)) else {
        return Ok(None);
    };
    if (lo - other_lo).abs() > 1e-9 || (hi - other_hi).abs() > 1e-9 {
        return Ok(None);
    }
    let pa = crate::solid::section::slice(a, (lo + hi) / 2.)?;
    let pb = crate::solid::section::slice(b, (lo + hi) / 2.)?;
    let p = planar(&pa, &pb, operation)?;
    let mut result = crate::solid::modeling::extrude_rings(&p, hi - lo, 1, 0., [1., 1.], false)?;
    for v in result.positions.as_chunks_mut::<3>().0 {
        v[2] += lo
    }
    Ok(Some(result))
}

/// Join solids stacked on a shared horizontal cap. Boundary equivalence is
/// checked explicitly; overlapping bounds alone never select this path.
pub fn join_touching(a: &Mesh, b: &Mesh) -> Result<Option<Mesh>> {
    if a.indices.is_empty() || b.indices.is_empty() {
        return Ok(None);
    }
    let range = |m: &Mesh| {
        (
            m.positions
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| p[2])
                .fold(f64::INFINITY, f64::min),
            m.positions
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| p[2])
                .fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let (al, ah) = range(a);
    let (bl, bh) = range(b);
    let eps = 1e-8;
    let z = if (ah - bl).abs() < eps {
        ah
    } else if (bh - al).abs() < eps {
        al
    } else {
        return Ok(None);
    };
    type CapBoundary = BTreeSet<([i64; 2], [i64; 2])>;
    let cap = |m: &Mesh| -> Result<Option<(Mesh, CapBoundary)>> {
        let mut rest = m.clone();
        let mut top = empty();
        top.positions = m.positions.clone();
        rest.indices.clear();
        for t in m.indices.as_chunks::<3>().0 {
            if t.iter().all(|&i| (m.positions[i * 3 + 2] - z).abs() < eps) {
                top.indices.extend(t)
            } else {
                rest.indices.extend(t)
            }
        }
        if top.indices.is_empty() {
            return Ok(None);
        }
        let mut edges = BTreeSet::new();
        for ring in top.boundary_loops()? {
            for [i, j] in ring.array_windows() {
                let key = |i: usize| {
                    [
                        (m.positions[i * 3] / eps).round() as i64,
                        (m.positions[i * 3 + 1] / eps).round() as i64,
                    ]
                };
                let x = key(*i);
                let y = key(*j);
                edges.insert(if x < y { (x, y) } else { (y, x) });
            }
        }
        Ok(Some((rest, edges)))
    };
    let (Some((left, le)), Some((right, re))) = (cap(a)?, cap(b)?) else {
        return Ok(None);
    };
    if le != re {
        return Ok(None);
    }
    if !a.inspect()?.closed || !b.inspect()?.closed {
        return Ok(None);
    }
    let mut out = join(&[left, right])?;
    let mut shared = HashMap::new();
    let mut remap: Vec<_> = (0..out.positions.len() / 3).collect();
    for (i, p) in out.positions.as_chunks::<3>().0.iter().enumerate() {
        if (p[2] - z).abs() < eps {
            let key = [(p[0] / eps).round() as i64, (p[1] / eps).round() as i64];
            remap[i] = *shared.entry(key).or_insert(i);
        }
    }
    for i in &mut out.indices {
        *i = remap[*i]
    }
    if out.inspect()?.closed {
        Ok(Some(out))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_closed() {
        for m in [
            cube([2., 3., 4.], false).unwrap(),
            sphere(3., 24).unwrap(),
            cylinder(4., 2., 0., 24, false).unwrap(),
        ] {
            let r = m.inspect().unwrap();
            assert!(r.closed, "{r:?}");
            assert_eq!(r.degenerate_triangles, 0);
            assert!(r.signed_volume_mm3 > 0.)
        }
    }

    #[test]
    fn hull_box() {
        let m = cube([2., 3., 4.], false).unwrap();
        let h = hull3(&[m]).unwrap();
        let r = h.inspect().unwrap();
        assert!(r.closed);
        assert!((r.signed_volume_mm3 - 24.).abs() < 1e-8)
    }
}
