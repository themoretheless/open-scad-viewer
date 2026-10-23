//! Native mesh editor operations. UI identity/history and transport stay outside.
use crate::{Mesh, Result, error};
use std::collections::{BTreeMap, BTreeSet};
type Point = [f64; 3];
fn finite(x: f64) -> bool {
    x.is_finite() && x.abs() <= 1e6
}
fn vector(v: Point, message: &str) -> Result<()> {
    if v.iter().all(|x| finite(*x)) {
        Ok(())
    } else {
        Err(error(message))
    }
}
fn output(positions: Vec<f64>, indices: Vec<usize>) -> Mesh {
    Mesh {
        positions,
        indices,
        uv: None,
    }
}
fn ids(mesh: &Mesh, selected: &[usize], faces: bool) -> Result<()> {
    let limit = if faces {
        mesh.indices.len() / 3
    } else {
        mesh.positions.len() / 3
    };
    if selected.iter().any(|i| *i >= limit) {
        return Err(error(if faces {
            "Invalid face."
        } else {
            "Invalid vertex."
        }));
    }
    Ok(())
}
fn budget(mesh: &Mesh, message: &str) -> Result<()> {
    if mesh.positions.len() > 300_000 || mesh.indices.len() > 300_000 {
        Err(error(message))
    } else {
        Ok(())
    }
}
fn normal(mesh: &Mesh, face: usize) -> Point {
    let p = mesh.view();
    let t = &mesh.indices[face * 3..face * 3 + 3];
    let a = p.point(t[0]).unwrap();
    let b = p.point(t[1]).unwrap();
    let c = p.point(t[2]).unwrap();
    let n = math_core::cross(math_core::sub(b, a), math_core::sub(c, a));
    let len = math_core::norm(n);
    n.map(|x| x / if len == 0. { 1. } else { len })
}
pub fn centroid(mesh: &Mesh, selected: &[usize]) -> Result<Point> {
    mesh.validate()?;
    ids(mesh, selected, false)?;
    let count = if selected.is_empty() {
        mesh.positions.len() / 3
    } else {
        selected.len()
    };
    let mut p = [0.; 3];
    if count == 0 {
        return Ok([f64::NAN; 3]);
    }
    if selected.is_empty() {
        for q in mesh.positions.as_chunks::<3>().0 {
            for k in 0..3 {
                p[k] += q[k];
            }
        }
    } else {
        for i in selected {
            for k in 0..3 {
                p[k] += mesh.positions[i * 3 + k];
            }
        }
    }
    Ok(p.map(|x| x / count as f64))
}
pub fn transform(mesh: &Mesh, delta: Point, angle: f64, scale: f64) -> Result<Mesh> {
    vector(delta, "Invalid transform.")?;
    if !finite(angle) || !finite(scale) || scale <= 0. {
        return Err(error("Invalid transform."));
    }
    let center = centroid(mesh, &[])?;
    let a = angle * std::f64::consts::PI / 180.;
    let (s, c) = a.sin_cos();
    let mut out = output(mesh.positions.clone(), mesh.indices.clone());
    for p in out.positions.as_chunks_mut::<3>().0 {
        let x = (p[0] - center[0]) * scale;
        let y = (p[1] - center[1]) * scale;
        let z = (p[2] - center[2]) * scale;
        p[0] = center[0] + c * x - s * y + delta[0];
        p[1] = center[1] + s * x + c * y + delta[1];
        p[2] = center[2] + z + delta[2];
    }
    Ok(out)
}
pub fn move_vertices(
    mesh: &Mesh,
    selected: &[usize],
    delta: Point,
    radius: Option<f64>,
) -> Result<Mesh> {
    mesh.validate()?;
    ids(mesh, selected, false)?;
    vector(delta, "Invalid delta.")?;
    let mut out = output(mesh.positions.clone(), mesh.indices.clone());
    if let Some(radius) = radius {
        if !finite(radius) || radius <= 0. {
            return Err(error("Invalid proportional transform."));
        }
        let selected: BTreeSet<_> = selected.iter().copied().collect();
        if selected.is_empty() {
            return Err(error("Select vertices for proportional editing."));
        }
        for (i, p) in out.positions.as_chunks_mut::<3>().0.iter_mut().enumerate() {
            let q = mesh.view().point(i)?;
            let mut distance = f64::INFINITY;
            for j in &selected {
                distance = distance.min(math_core::norm(math_core::sub(q, mesh.view().point(*j)?)));
            }
            if distance >= radius {
                continue;
            }
            let linear = 1. - distance / radius;
            let weight = linear * linear * (3. - 2. * linear);
            for k in 0..3 {
                p[k] += delta[k] * weight;
            }
        }
    } else {
        for i in selected {
            for k in 0..3 {
                out.positions[i * 3 + k] += delta[k];
            }
        }
    }
    Ok(out)
}
pub fn compact(mesh: &Mesh) -> Result<Mesh> {
    mesh.validate()?;
    let used: BTreeSet<_> = mesh.indices.iter().copied().collect();
    let mut remap = BTreeMap::new();
    let mut positions = Vec::new();
    for i in used {
        remap.insert(i, positions.len() / 3);
        positions.extend_from_slice(&mesh.positions[i * 3..i * 3 + 3]);
    }
    Ok(output(
        positions,
        mesh.indices.iter().map(|i| remap[i]).collect(),
    ))
}
pub fn delete_faces(mesh: &Mesh, selected: &[usize]) -> Result<Mesh> {
    mesh.validate()?;
    let remove: BTreeSet<_> = selected.iter().copied().collect();
    let indices = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .enumerate()
        .filter(|(i, _)| !remove.contains(i))
        .flat_map(|(_, t)| *t)
        .collect::<Vec<_>>();
    if indices.is_empty() {
        return Err(error("Cannot delete all faces."));
    }
    compact(&output(mesh.positions.clone(), indices))
}
pub fn flip_faces(mesh: &Mesh, selected: Option<&[usize]>) -> Result<Mesh> {
    mesh.validate()?;
    let mut out = output(mesh.positions.clone(), mesh.indices.clone());
    if let Some(selected) = selected {
        ids(mesh, selected, true)?;
        for i in selected {
            out.indices.swap(i * 3 + 1, i * 3 + 2);
        }
    } else {
        for t in out.indices.as_chunks_mut::<3>().0 {
            t.swap(1, 2);
        }
    }
    Ok(out)
}
pub fn subdivide(mesh: &Mesh, selected: Option<&[usize]>) -> Result<Mesh> {
    mesh.validate()?;
    let selected = selected.map(|s| s.iter().copied().collect::<BTreeSet<_>>());
    let mut positions = mesh.positions.clone();
    let mut indices = Vec::new();
    let mut midpoints = BTreeMap::new();
    for (i, [a, b, c]) in mesh.indices.as_chunks::<3>().0.iter().copied().enumerate() {
        if selected.as_ref().is_some_and(|s| !s.contains(&i)) {
            indices.extend([a, b, c]);
            continue;
        }
        let mut mid = |a: usize, b: usize| {
            let key = (a.min(b), a.max(b));
            *midpoints.entry(key).or_insert_with(|| {
                let id = positions.len() / 3;
                for k in 0..3 {
                    positions.push((mesh.positions[a * 3 + k] + mesh.positions[b * 3 + k]) / 2.);
                }
                id
            })
        };
        let ab = mid(a, b);
        let bc = mid(b, c);
        let ca = mid(c, a);
        indices.extend([a, ab, ca, ab, b, bc, ca, bc, c, ab, bc, ca]);
        if positions.len() > 300_000 || indices.len() > 300_000 {
            return Err(error("Subdivision exceeds mesh budget."));
        }
    }
    Ok(output(positions, indices))
}
pub fn extrude(mesh: &Mesh, selected: &[usize], distance: f64) -> Result<Mesh> {
    mesh.validate()?;
    ids(mesh, selected, true)?;
    if !finite(distance) || selected.is_empty() {
        return Err(error("Choose faces and a finite distance."));
    }
    let mut sum = [0.; 3];
    for i in selected {
        let n = normal(mesh, *i);
        for k in 0..3 {
            sum[k] += n[k];
        }
    }
    let len = math_core::norm(sum);
    let direction = sum.map(|x| x / if len == 0. { 1. } else { len } * distance);
    Ok(crate::solid::edit::extrude_faces(mesh, selected, direction)?.mesh)
}
pub fn inset(mesh: &Mesh, selected: &[usize], amount: f64) -> Result<Mesh> {
    mesh.validate()?;
    ids(mesh, selected, true)?;
    if !finite(amount) || amount <= 0. || selected.is_empty() {
        return Err(error("Inset requires positive amount and faces."));
    }
    let mut out = output(mesh.positions.clone(), mesh.indices.clone());
    let mut moved = BTreeSet::new();
    for i in selected {
        let t = &mesh.indices[i * 3..i * 3 + 3];
        let center = std::array::from_fn::<_, 3, _>(|k| {
            (out.positions[t[0] * 3 + k]
                + out.positions[t[1] * 3 + k]
                + out.positions[t[2] * 3 + k])
                / 3.
        });
        for id in t {
            if !moved.insert(*id) {
                continue;
            }
            let v = std::array::from_fn::<_, 3, _>(|k| out.positions[id * 3 + k] - center[k]);
            let len = math_core::norm(v);
            let weight = (amount / if len == 0. { 1. } else { len }).min(0.95);
            for (k, x) in v.iter().enumerate() {
                out.positions[id * 3 + k] -= x * weight;
            }
        }
    }
    Ok(out)
}
pub fn merge(mesh: &Mesh, distance: f64) -> Result<Mesh> {
    mesh.validate()?;
    if !finite(distance) || distance <= 0. {
        return Err(error("Merge distance must be positive."));
    }
    let s = 1. / distance;
    let mut map = BTreeMap::new();
    let mut positions = Vec::new();
    let mut remap = Vec::new();
    for p in mesh.positions.as_chunks::<3>().0 {
        let key = p.map(|x| {
            let value = x * s;
            let low = value.floor();
            let rounded = if value - low >= 0.5 { low + 1. } else { low };
            if rounded == 0. { 0 } else { rounded.to_bits() }
        });
        let id = *map.entry(key).or_insert_with(|| {
            let id = positions.len() / 3;
            positions.extend(p);
            id
        });
        remap.push(id);
    }
    let indices = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter_map(|t| {
            let [a, b, c] = t.map(|i| remap[i]);
            (a != b && b != c && c != a).then_some([a, b, c])
        })
        .flatten()
        .collect::<Vec<_>>();
    if indices.is_empty() {
        return Err(error("Merge removed all faces."));
    }
    Ok(output(positions, indices))
}
pub fn edges(mesh: &Mesh) -> Result<Vec<[usize; 2]>> {
    mesh.validate()?;
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for [a, b, c] in mesh.indices.as_chunks::<3>().0 {
        for [u, v] in [[*a, *b], [*b, *c], [*c, *a]] {
            let e = [u.min(v), u.max(v)];
            if seen.insert(e) {
                result.push(e);
            }
        }
    }
    Ok(result)
}
pub fn knife(mesh: &Mesh, selected: &[usize]) -> Result<Mesh> {
    if selected.is_empty() {
        return Err(error("Select edges to cut."));
    }
    let edges = edges(mesh)?;
    let mut seen = BTreeSet::new();
    let mut out = output(mesh.positions.clone(), mesh.indices.clone());
    for i in selected {
        if !seen.insert(*i) {
            continue;
        }
        let [a, b] = *edges
            .get(*i)
            .ok_or_else(|| error("Selected edge is out of range."))?;
        let midpoint = out.positions.len() / 3;
        for k in 0..3 {
            out.positions
                .push((out.positions[a * 3 + k] + out.positions[b * 3 + k]) / 2.);
        }
        let mut indices = Vec::new();
        let mut adjacent = 0;
        for t in out.indices.as_chunks::<3>().0 {
            let ai = t.iter().position(|i| *i == a);
            let bi = t.iter().position(|i| *i == b);
            if let (Some(ai), Some(bi)) = (ai, bi) {
                adjacent += 1;
                let c = *t
                    .iter()
                    .find(|i| **i != a && **i != b)
                    .ok_or_else(|| error("Degenerate selected edge."))?;
                if (ai + 1) % 3 == bi {
                    indices.extend([a, midpoint, c, midpoint, b, c]);
                } else {
                    indices.extend([b, midpoint, c, midpoint, a, c]);
                }
            } else {
                indices.extend(t);
            }
        }
        if adjacent == 0 {
            return Err(error("Selected edge has no adjacent faces."));
        }
        out.indices = indices;
        budget(&out, "Knife cut exceeds mesh budget.")?;
    }
    Ok(out)
}
pub fn separate(mesh: &Mesh, selected: &[usize]) -> Result<(Mesh, Mesh)> {
    mesh.validate()?;
    if selected.is_empty() {
        return Err(error("Select faces to separate."));
    }
    let selected: BTreeSet<_> = selected.iter().copied().collect();
    let mut kept = Vec::new();
    let mut removed = Vec::new();
    for (i, t) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
        if selected.contains(&i) {
            removed.extend(t);
        } else {
            kept.extend(t);
        }
    }
    if kept.is_empty() || removed.is_empty() {
        return Err(error("Separation must leave two non-empty meshes."));
    }
    Ok((
        compact(&output(mesh.positions.clone(), kept))?,
        compact(&output(mesh.positions.clone(), removed))?,
    ))
}
pub fn join(meshes: &[Mesh]) -> Result<Mesh> {
    if meshes.len() < 2 {
        return Err(error("Join needs at least two meshes."));
    }
    let mut out = output(Vec::new(), Vec::new());
    for mesh in meshes {
        mesh.validate()?;
        let base = out.positions.len() / 3;
        out.positions.extend(&mesh.positions);
        out.indices.extend(mesh.indices.iter().map(|i| i + base));
        budget(&out, "Joined mesh exceeds budget.")?;
    }
    merge(&out, 1e-5)
}
pub fn symmetrize(mesh: &Mesh, axis: usize) -> Result<Mesh> {
    mesh.validate()?;
    if axis >= 3 {
        return Err(error("Invalid symmetry axis."));
    }
    let mut mirrored = flip_faces(mesh, None)?;
    for p in mirrored.positions.as_chunks_mut::<3>().0 {
        p[axis] *= -1.;
    }
    join(&[mesh.clone(), mirrored])
}
pub fn uv_sphere(radius: f64, segments: usize, rings: usize) -> Result<Mesh> {
    if !finite(radius)
        || radius <= 0.
        || segments < 3
        || rings < 2
        || segments > 512
        || rings > 512
        || (segments + 1) * (rings + 1) * 3 > 300_000
        || segments * rings * 6 > 300_000
    {
        return Err(error("Invalid sphere/budget."));
    }
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for y in 0..=rings {
        let phi = y as f64 / rings as f64 * std::f64::consts::PI;
        for x in 0..=segments {
            let theta = x as f64 / segments as f64 * std::f64::consts::PI * 2.;
            positions.extend([
                radius * phi.sin() * theta.cos(),
                radius * phi.cos(),
                radius * phi.sin() * theta.sin(),
            ]);
        }
    }
    for y in 0..rings {
        for x in 0..segments {
            let a = y * (segments + 1) + x;
            let b = a + segments + 1;
            indices.extend([a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    Ok(output(positions, indices))
}

/// Orthographic face projection. SVG formatting stays in the UI adapter.
pub fn project_faces(
    mesh: &Mesh,
    yaw: f64,
    pitch: f64,
    orientation: f64,
) -> Result<Vec<([Point; 3], usize, f64)>> {
    mesh.validate()?;
    if !yaw.is_finite() || !pitch.is_finite() || ![-1., 0., 1.].contains(&orientation) {
        return Err(error("Invalid projection."));
    }
    let (cy, sy, cp, sp) = (yaw.cos(), yaw.sin(), pitch.cos(), pitch.sin());
    let points: Vec<Point> = mesh
        .positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| {
            let h = p[0] * sy + p[1] * cy;
            [
                p[0] * cy - p[1] * sy,
                h * sp - p[2] * cp,
                h * cp + p[2] * sp,
            ]
        })
        .collect();
    let mut faces = Vec::new();
    for (i, t) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
        let [a, b, c] = t.map(|i| points[i]);
        let facing = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        if orientation != 0. && facing * orientation <= 0. {
            continue;
        }
        faces.push(([a, b, c], i, (a[2] + b[2] + c[2]) / 3.));
    }
    Ok(faces)
}
