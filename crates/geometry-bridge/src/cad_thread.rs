//! Atomic mesh threading; analytic retained B-rep threading is not yet supported.
use super::{Result, Value, encode, field, input};
use polygon_core::{
    Mesh,
    solid::boolean::{Operation, Options, boolean},
};
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(v: [f64; 3]) -> Result<[f64; 3]> {
    let magnitude = v.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. || !v.iter().all(|x| x.is_finite()) {
        return Err(input("Invalid thread direction."));
    }
    let v = v.map(|x| x / magnitude);
    let length = v[0].hypot(v[1]).hypot(v[2]);
    Ok(v.map(|x| x / length))
}
pub fn apply(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    if body.get("brep").is_some() {
        return Err(input(
            "Threading retained B-rep is not implemented; geometry was not changed.",
        ));
    }
    let mesh: Mesh = field(&body, "mesh")?;
    mesh.validate()?;
    let width: f64 = field(&v, "width")?;
    let depth: f64 = field(&v, "depth")?;
    let pitch: f64 = field(&v, "pitch")?;
    let origin: [f64; 3] = field(&v, "origin")?;
    let n = unit(field(&v, "axis")?)?;
    if !origin.iter().all(|x| x.is_finite()) {
        return Err(input("Thread origin must be finite."));
    }
    let mode: String = field(&v, "mode")?;
    let generated=modelgraph_runtime::thread_geometry(&value_codec::json!({"diameter":width,"pitch":pitch,"length":depth,"internal":false,"wall":1.,"clearance":0.,"starts":1.,"left_handed":false,"segments_per_turn":16.})).map_err(|e|input(e.message))?;
    let cutter: Mesh = field(&generated, "mesh")?;
    let u = unit(cross(
        n,
        if n[0].abs() < 0.8 {
            [1., 0., 0.]
        } else {
            [0., 1., 0.]
        },
    ))?;
    let w = cross(n, u);
    let matrix = std::array::from_fn(|i| {
        if i == 3 {
            [0., 0., 0., 1.]
        } else {
            [u[i], w[i], n[i], origin[i]]
        }
    });
    let cutter = cutter.transform(matrix)?;
    let operation = if mode == "external" {
        Operation::Union
    } else {
        Operation::Difference
    };
    let built = boolean(&mesh, &cutter, operation, &Options::default())?;
    if !built.report.closed || built.mesh.indices.is_empty() {
        return Err(input("Thread did not produce a closed body."));
    }
    let mut result = body
        .as_object()
        .ok_or_else(|| input("Expected body record"))?
        .clone();
    result.insert("mesh".into(), encode(built.mesh)?);
    Ok(Value::Object(result))
}
