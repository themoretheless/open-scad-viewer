//! Numerical unique-support admission for legacy display-face selection.
//! This is not persistent topological naming or certified correspondence.
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;
pub fn select(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    let model: brep_core::Model = field(&body, "brep")?;
    model.validate()?;
    let mesh: Mesh = field(&body, "mesh")?;
    mesh.validate()?;
    let triangles: Vec<usize> = field(&v, "triangles")?;
    let first = *triangles
        .first()
        .ok_or_else(|| input("Select a planar face on a B-rep body."))?;
    if triangles.iter().any(|&t| t >= mesh.indices.len() / 3) {
        return Err(input("Invalid displayed face triangle."));
    }
    let point =
        |i: usize| -> [f64; 3] { std::array::from_fn(|k| mesh.positions[mesh.indices[i] * 3 + k]) };
    let origin = point(first * 3);
    let b = point(first * 3 + 1);
    let c = point(first * 3 + 2);
    let u: [f64; 3] = std::array::from_fn(|i| b[i] - origin[i]);
    let w: [f64; 3] = std::array::from_fn(|i| c[i] - origin[i]);
    let normal = [
        u[1] * w[2] - u[2] * w[1],
        u[2] * w[0] - u[0] * w[2],
        u[0] * w[1] - u[1] * w[0],
    ];
    let magnitude = normal.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. || !magnitude.is_finite() {
        return Err(input("Degenerate displayed face."));
    }
    let normal = normal.map(|x| x / magnitude);
    let length = normal[0].hypot(normal[1]).hypot(normal[2]);
    let normal = normal.map(|x| x / length);
    let on_plane = |p: [f64; 3]| {
        let d = (0..3).map(|k| (p[k] - origin[k]) * normal[k]).sum::<f64>();
        d.is_finite() && d.abs() <= model.tolerance_mm * 8.
    };
    if triangles
        .iter()
        .any(|&t| (0..3).any(|k| !on_plane(point(t * 3 + k))))
    {
        return Err(input("Displayed selection is not planar."));
    }
    let matches = model
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| on_plane([p[0], p[1], p[2]]))
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(input(
            "Displayed face does not identify one authored planar B-rep support.",
        ));
    }
    encode(matches[0])
}

/// Admit only an existing mesh edge matching one straight authored edge.
/// Curved edge chords and endpoint-only coincidences are not correspondence.
pub fn edge(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    let model: brep_core::Model = field(&body, "brep")?;
    model.validate()?;
    let mesh: Mesh = field(&body, "mesh")?;
    mesh.validate()?;
    let vertices: [usize; 2] = field(&v, "vertices")?;
    if vertices[0] == vertices[1] || vertices.iter().any(|&i| i >= mesh.positions.len() / 3) {
        return Err(input("Invalid displayed edge vertices."));
    }
    let exists = mesh.indices.as_chunks::<3>().0.iter().any(|t| {
        (0..3).any(|i| {
            let a = t[i];
            let b = t[(i + 1) % 3];
            [a, b] == vertices || [b, a] == vertices
        })
    });
    if !exists {
        return Err(input("Selected vertices do not form a displayed edge."));
    }
    let a: [f64; 3] = std::array::from_fn(|k| mesh.positions[vertices[0] * 3 + k]);
    let b: [f64; 3] = std::array::from_fn(|k| mesh.positions[vertices[1] * 3 + k]);
    let same = |p: [f64; 3], q: [f64; 3]| {
        (p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - q[2]) <= model.tolerance_mm
    };
    let d: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
    let length = d[0].hypot(d[1]).hypot(d[2]);
    if !length.is_finite() || length <= model.tolerance_mm {
        return Err(input("Degenerate displayed edge."));
    }
    let n = d.map(|x| x / length);
    let mut matches = Vec::new();
    for (i, edge) in model.edges.iter().enumerate() {
        let p = model.vertices[edge.vertices[0]].point;
        let q = model.vertices[edge.vertices[1]].point;
        if !(same(a, p) && same(b, q) || same(a, q) && same(b, p)) {
            continue;
        }
        let straight = edge.curve.control_points.iter().all(|p| {
            let v: [f64; 3] = std::array::from_fn(|k| p[k] - a[k]);
            let t = (0..3).map(|k| v[k] * n[k]).sum::<f64>();
            let residual: [f64; 3] = std::array::from_fn(|k| v[k] - t * n[k]);
            t.is_finite()
                && t >= -model.tolerance_mm
                && t <= length + model.tolerance_mm
                && residual[0].hypot(residual[1]).hypot(residual[2]) <= model.tolerance_mm
        });
        if straight {
            matches.push(i);
        }
    }
    if matches.len() != 1 {
        return Err(input(
            "The displayed edge does not identify one straight authored B-rep edge.",
        ));
    }
    encode(matches[0])
}
