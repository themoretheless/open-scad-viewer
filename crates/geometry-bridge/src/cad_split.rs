//! Atomic plane split for retained planar B-rep and polygon bodies.
use super::{Result, Value, brep, encode, field, input};
pub fn split(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    let normal: [f64; 3] = field(&v, "normal")?;
    let offset: f64 = field(&v, "offset")?;
    if !normal.iter().chain([&offset]).all(|x| x.is_finite()) {
        return Err(input("Invalid split plane."));
    }
    let magnitude = normal.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. {
        return Err(input("Zero direction"));
    }
    let normal = normal.map(|x| x / magnitude);
    let length = normal[0].hypot(normal[1]).hypot(normal[2]);
    let normal = normal.map(|x| x / length);
    if body.get("brep").is_none() {
        return split_mesh(body, normal, offset);
    }
    let model: brep_core::Model = field(&body, "brep")?;
    // The editor offset is signed distance along a normalized direction.
    // The core equation API instead scales both normal and offset.
    let [negative, positive] = brep_core::operations::split_planar(&model, normal, offset)?;
    let mut result = Vec::with_capacity(2);
    for part in [positive, negative] {
        let mesh = brep::nurbs(&part, 1)?.built.mesh;
        let mut next = body
            .as_object()
            .ok_or_else(|| input("Expected body record"))?
            .clone();
        next.insert("brep".into(), encode(part)?);
        next.insert("mesh".into(), encode(mesh)?);
        result.push(Value::Object(next));
    }
    encode(result)
}

fn split_mesh(body: Value, n: [f64; 3], offset: f64) -> Result<Value> {
    use polygon_core::{
        Mesh,
        solid::{
            boolean::{Operation, Options, boolean},
            modeling::{Profile, extrude},
        },
    };
    let mesh: Mesh = field(&body, "mesh")?;
    mesh.validate()?;
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let origin = n.map(|x| x * offset);
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut radius = 0_f64;
    for p in mesh.positions.as_chunks::<3>().0 {
        let projection = dot(n, [p[0], p[1], p[2]]);
        let distance = (p[0] - origin[0])
            .hypot(p[1] - origin[1])
            .hypot(p[2] - origin[2]);
        if !projection.is_finite() || !distance.is_finite() {
            return Err(input("Split geometry exceeds finite numeric range."));
        }
        min = min.min(projection);
        max = max.max(projection);
        radius = radius.max(distance);
    }
    if offset <= min + 1e-6 || offset >= max - 1e-6 {
        return Err(input("The plane must intersect the body."));
    }
    let size = radius * 2. + 1.;
    if !size.is_finite() {
        return Err(input("Split cutter exceeds finite numeric range."));
    }
    let u = cross(
        n,
        if n[0].abs() < 0.8 {
            [1., 0., 0.]
        } else {
            [0., 1., 0.]
        },
    );
    let length = u[0].hypot(u[1]).hypot(u[2]);
    let u = u.map(|x| x / length);
    let w = cross(n, u);
    let matrix = std::array::from_fn(|i| {
        if i == 3 {
            [0., 0., 0., 1.]
        } else {
            [u[i], w[i], n[i], origin[i]]
        }
    });
    let cutter = extrude(
        &Profile {
            outer: vec![[-size, -size], [size, -size], [size, size], [-size, size]],
            holes: vec![],
        },
        [0., 0., size],
    )?
    .mesh
    .transform(matrix)?;
    let mut result = Vec::with_capacity(2);
    for operation in [Operation::Intersection, Operation::Difference] {
        let built = boolean(&mesh, &cutter, operation, &Options::default())?;
        if !built.report.closed || built.mesh.indices.is_empty() {
            return Err(input("Could not create two closed halves."));
        }
        let mut next = body
            .as_object()
            .ok_or_else(|| input("Expected body record"))?
            .clone();
        next.insert("mesh".into(), encode(built.mesh)?);
        result.push(Value::Object(next));
    }
    encode(result)
}
