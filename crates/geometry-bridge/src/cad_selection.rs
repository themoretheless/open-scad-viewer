//! Transactional document transform; host code performs no geometry arithmetic.
use super::cad_body_affine;
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;
type V = [f64; 3];
fn finite(xs: impl IntoIterator<Item = f64>) -> Result<()> {
    if xs.into_iter().all(f64::is_finite) {
        Ok(())
    } else {
        Err(input("Selection transform exceeds finite numeric range."))
    }
}
fn dot(a: V, b: V) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn point(m: [[f64; 4]; 4], p: V) -> V {
    std::array::from_fn(|i| m[i][3] + (0..3).map(|j| m[i][j] * p[j]).sum::<f64>())
}
fn plane(s: &Value) -> Result<(V, V, V)> {
    match s.get("plane") {
        Some(p) => Ok((field(p, "origin")?, field(p, "u")?, field(p, "v")?)),
        None => Ok(([0.; 3], [1., 0., 0.], [0., 1., 0.])),
    }
}
fn world(p: [f64; 2], o: V, u: V, v: V) -> V {
    std::array::from_fn(|i| o[i] + p[0] * u[i] + p[1] * v[i])
}
pub fn transform(v: Value) -> Result<Value> {
    let mut document: Value = field(&v, "document")?;
    let ids: Vec<String> = field(&v, "ids")?;
    let delta: V = field(&v, "delta")?;
    let axis: V = field(&v, "axis")?;
    let angle: f64 = field(&v, "angle")?;
    let scale: f64 = field(&v, "scale")?;
    if !delta
        .iter()
        .chain(&axis)
        .chain([&angle, &scale])
        .all(|x| x.is_finite())
        || scale <= 0.
    {
        return Err(input("Invalid transform."));
    }
    let mut bodies: Vec<Value> = field(&document, "bodies")?;
    let mut sketches: Vec<Value> = field(&document, "sketches")?;
    let selected = |x: &Value| {
        x.get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| ids.iter().any(|i| i == id))
    };
    let body_ids: Vec<usize> = bodies
        .iter()
        .enumerate()
        .filter(|(_, b)| selected(b))
        .map(|(i, _)| i)
        .collect();
    let sketch_ids: Vec<usize> = sketches
        .iter()
        .enumerate()
        .filter(|(_, s)| selected(s))
        .map(|(i, _)| i)
        .collect();
    let meshes = body_ids
        .iter()
        .map(|&i| field::<Mesh>(&bodies[i], "mesh"))
        .collect::<Result<Vec<_>>>()?;
    let mut positions = meshes
        .iter()
        .map(|m| m.positions.clone())
        .collect::<Vec<_>>();
    for &i in &sketch_ids {
        let (o, u, w) = plane(&sketches[i])?;
        let points: Vec<[f64; 2]> = field(&sketches[i], "points")?;
        positions.push(points.into_iter().flat_map(|p| world(p, o, u, w)).collect());
    }
    if positions.iter().all(Vec::is_empty) {
        return Ok(document);
    }
    let matrix = cad_body_affine::matrix(&positions, delta, axis, angle, scale)?;
    let rotate =
        |p: V| -> V { std::array::from_fn(|i| (0..3).map(|j| matrix[i][j] / scale * p[j]).sum()) };
    let moved = cad_body_affine::apply(
        body_ids.iter().map(|&i| bodies[i].clone()).collect(),
        meshes,
        matrix,
    )?;
    for (&i, b) in body_ids.iter().zip(
        moved
            .as_array()
            .ok_or_else(|| input("Expected body array"))?,
    ) {
        bodies[i] = b.clone();
    }
    for i in sketch_ids {
        let sketch = &mut sketches[i];
        let (o, u, w) = plane(sketch)?;
        let moved_o = point(matrix, o);
        let moved_u = rotate(u);
        let moved_v = rotate(w);
        let normal = cross(u, w);
        let coplanar = dot(normal, cross(moved_u, moved_v)) > 1. - 1e-8
            && dot(sub(moved_o, o), normal).abs() < 1e-7;
        let local = |p: [f64; 2]| -> [f64; 2] {
            if coplanar {
                let q = sub(point(matrix, world(p, o, u, w)), o);
                [dot(q, u), dot(q, w)]
            } else {
                p.map(|x| x * scale)
            }
        };
        let points: Vec<[f64; 2]> = field(sketch, "points")?;
        let points = points.into_iter().map(local).collect::<Vec<_>>();
        finite(points.iter().flatten().copied())?;
        finite(moved_o.into_iter().chain(moved_u).chain(moved_v))?;
        sketch["points"] = encode(points)?;
        if !coplanar {
            sketch["plane"] = encode(std::collections::BTreeMap::from([
                ("origin".to_string(), moved_o),
                ("u".to_string(), moved_u),
                ("v".to_string(), moved_v),
            ]))?;
        }
        if let Some(analytic) = sketch.get_mut("analytic") {
            let center: [f64; 2] = field(analytic, "center")?;
            let radius: f64 = field(analytic, "radius")?;
            let center = local(center);
            finite(center.into_iter().chain([radius * scale]))?;
            analytic["center"] = encode(center)?;
            analytic["radius"] = encode(radius * scale)?;
            if coplanar {
                let start: f64 = field(analytic, "start")?;
                let start = start + dot(moved_u, w).atan2(dot(moved_u, u)).to_degrees();
                finite([start])?;
                analytic["start"] = encode(start)?;
            }
        }
    }
    document["bodies"] = encode(bodies)?;
    document["sketches"] = encode(sketches)?;
    Ok(document)
}
