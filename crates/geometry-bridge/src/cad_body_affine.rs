//! Atomic affine operations on body records and their retained B-rep.
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;
/// Decode one source once and derive its placements in one transport call.
pub fn instances(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    let brep = v
        .get("brep")
        .map(|_| field::<brep_core::Model>(&v, "brep"))
        .transpose()?;
    let matrices: Vec<[[f64; 4]; 4]> = field(&v, "matrices")?;
    if matrices.len() > 1000 {
        return Err(input("At most 1000 instance placements per batch."));
    }
    let mut result = Vec::with_capacity(matrices.len());
    for matrix in matrices {
        let mut geometry = value_codec::Map::new();
        geometry.insert("mesh".into(), encode(mesh.transform(matrix)?)?);
        if let Some(model) = &brep {
            geometry.insert(
                "brep".into(),
                encode(brep_core::transform::affine(model, matrix)?)?,
            );
        }
        result.push(Value::Object(geometry));
    }
    // Entries already own encoded Values. Serializing this tree again copies
    // every B-rep control point and mesh buffer before binary transport.
    Ok(Value::Array(result))
}
/// Compose a world-space edit with an instance's source placement.
pub fn instance_transform(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    let placement: [[f64; 4]; 4] = field(&v, "matrix")?;
    let delta: [f64; 3] = field(&v, "delta")?;
    let axis: [f64; 3] = field(&v, "axis")?;
    let angle: f64 = field(&v, "angle")?;
    let scale: f64 = field(&v, "scale")?;
    if !placement
        .iter()
        .flatten()
        .chain(&delta)
        .chain(&axis)
        .chain([&angle, &scale])
        .all(|x| x.is_finite())
        || placement[3] != [0., 0., 0., 1.]
        || scale <= 0.
    {
        return Err(input("Invalid instance transform."));
    }
    let edit = matrix(&[mesh.positions], delta, axis, angle, scale)?;
    let composed: [[f64; 4]; 4] = std::array::from_fn(|i| {
        std::array::from_fn(|j| (0..4).map(|k| edit[i][k] * placement[k][j]).sum())
    });
    if !composed.iter().flatten().all(|x| x.is_finite()) {
        return Err(input("Instance transform exceeds finite numeric range."));
    }
    encode(composed)
}
pub fn joint(v: Value) -> Result<Value> {
    let bodies: Vec<Value> = field(&v, "bodies")?;
    let axis: [f64; 3] = field(&v, "axis")?;
    let origin: [f64; 3] = field(&v, "origin")?;
    let amount: f64 = field(&v, "amount")?;
    let mode: String = field(&v, "mode")?;
    if bodies.len() != 2 {
        return Err(input("Choose parent and moving component."));
    }
    if !axis
        .iter()
        .chain(&origin)
        .chain([&amount])
        .all(|x| x.is_finite())
    {
        return Err(input("Invalid joint parameters."));
    }
    let magnitude = axis.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. {
        return Err(input("Zero direction"));
    }
    let n = axis.map(|x| x / magnitude);
    let length = n[0].hypot(n[1]).hypot(n[2]);
    let n = n.map(|x| x / length);
    let delta = if mode == "slider" {
        n.map(|x| x * amount)
    } else {
        [0.; 3]
    };
    let angle = if mode == "slider" { 0. } else { amount };
    let transform = matrix(&[origin.to_vec()], delta, axis, angle, 1.)?;
    let mesh = field::<Mesh>(&bodies[1], "mesh")?;
    apply(vec![bodies[1].clone()], vec![mesh], transform)
}
pub fn arrange(v: Value) -> Result<Value> {
    let bodies: Vec<Value> = field(&v, "bodies")?;
    let axis: [f64; 3] = field(&v, "axis")?;
    let action: String = field(&v, "action")?;
    let mode: String = field(&v, "mode")?;
    if !["align", "distribute"].contains(&action.as_str()) {
        return Err(input("Invalid arrangement action."));
    }
    let magnitude = axis.iter().map(|x| x.abs()).fold(0., f64::max);
    if bodies.len() < 2 || magnitude == 0. || !axis.iter().all(|x| x.is_finite()) {
        return Err(input("Choose a world axis and at least two bodies."));
    }
    let n = axis.map(|x| x / magnitude);
    let length = n[0].hypot(n[1]).hypot(n[2]);
    let axis = n
        .iter()
        .position(|x| (x / length).abs() > 0.99)
        .ok_or_else(|| input("Choose a world axis and at least two bodies."))?;
    let meshes = bodies
        .iter()
        .map(|b| field::<Mesh>(b, "mesh"))
        .collect::<Result<Vec<_>>>()?;
    let keys = meshes
        .iter()
        .map(|m| {
            let (min, max) =
                polygon_core::scene_flatten::bounds(std::slice::from_ref(&m.positions))?;
            Ok(match mode.as_str() {
                "min" => min[axis],
                "max" => max[axis],
                _ => min[axis] * 0.5 + max[axis] * 0.5,
            })
        })
        .collect::<Result<Vec<f64>>>()?;
    let mut order = (0..bodies.len()).collect::<Vec<_>>();
    order.sort_by(|&a, &b| keys[a].total_cmp(&keys[b]));
    let first = keys[order[0]];
    let last = keys[*order.last().unwrap()];
    let mut offsets = vec![0.; bodies.len()];
    for (rank, &i) in order.iter().enumerate() {
        let t = rank as f64 / (order.len() - 1) as f64;
        let target = if action == "align" {
            keys[0]
        } else {
            first * (1. - t) + last * t
        };
        offsets[i] = target - keys[i];
    }
    let mut result = Vec::with_capacity(bodies.len());
    for (i, (body, mesh)) in bodies.into_iter().zip(meshes).enumerate() {
        let mut matrix = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];
        matrix[axis][3] = offsets[i];
        let moved = apply(vec![body], vec![mesh], matrix)?;
        result.extend(
            moved
                .as_array()
                .ok_or_else(|| input("Expected body array"))?
                .iter()
                .cloned(),
        );
    }
    encode(result)
}
pub fn transform(v: Value) -> Result<Value> {
    let bodies: Vec<Value> = field(&v, "bodies")?;
    let delta: [f64; 3] = field(&v, "delta")?;
    let axis: [f64; 3] = field(&v, "axis")?;
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
    let meshes = bodies
        .iter()
        .map(|b| field::<Mesh>(b, "mesh"))
        .collect::<Result<Vec<_>>>()?;
    let positions = meshes
        .iter()
        .map(|m| m.positions.clone())
        .collect::<Vec<_>>();
    if positions.iter().all(Vec::is_empty) {
        return encode(Vec::<Value>::new());
    }
    let matrix = matrix(&positions, delta, axis, angle, scale)?;
    apply(bodies, meshes, matrix)
}
pub(super) fn matrix(
    positions: &[Vec<f64>],
    delta: [f64; 3],
    axis: [f64; 3],
    angle: f64,
    scale: f64,
) -> Result<[[f64; 4]; 4]> {
    let (min, max) = polygon_core::scene_flatten::bounds(positions)?;
    let center: [f64; 3] = std::array::from_fn(|i| min[i] * 0.5 + max[i] * 0.5);
    let magnitude = axis.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. {
        return Err(input("Zero direction"));
    }
    let n = axis.map(|x| x / magnitude);
    let length = n[0].hypot(n[1]).hypot(n[2]);
    let n = n.map(|x| x / length);
    let (s, c) = angle.to_radians().sin_cos();
    let cross = [[0., -n[2], n[1]], [n[2], 0., -n[0]], [-n[1], n[0], 0.]];
    let mut matrix = [[0.; 4]; 4];
    matrix[3][3] = 1.;
    for i in 0..3 {
        for j in 0..3 {
            matrix[i][j] =
                scale * ((if i == j { c } else { 0. }) + s * cross[i][j] + (1. - c) * n[i] * n[j]);
        }
        matrix[i][3] = center[i] + delta[i] - (0..3).map(|j| matrix[i][j] * center[j]).sum::<f64>();
    }
    Ok(matrix)
}
pub fn resize(v: Value) -> Result<Value> {
    let bodies: Vec<Value> = field(&v, "bodies")?;
    let meshes = bodies
        .iter()
        .map(|b| field::<Mesh>(b, "mesh"))
        .collect::<Result<Vec<_>>>()?;
    let positions = meshes
        .iter()
        .map(|m| m.positions.clone())
        .collect::<Vec<_>>();
    let matrix = polygon_core::scene_flatten::resize_matrix(&positions, field(&v, "desired")?)?;
    apply(bodies, meshes, matrix)
}
pub fn mirror(v: Value) -> Result<Value> {
    let bodies: Vec<Value> = field(&v, "bodies")?;
    let meshes = bodies
        .iter()
        .map(|b| field::<Mesh>(b, "mesh"))
        .collect::<Result<Vec<_>>>()?;
    let matrix =
        polygon_core::scene_flatten::mirror_matrix(field(&v, "origin")?, field(&v, "axis")?)?;
    apply(bodies, meshes, matrix)
}
pub(super) fn apply(bodies: Vec<Value>, meshes: Vec<Mesh>, matrix: [[f64; 4]; 4]) -> Result<Value> {
    let mut result = Vec::with_capacity(bodies.len());
    for (body, mesh) in bodies.into_iter().zip(meshes) {
        let mut object = body
            .as_object()
            .ok_or_else(|| input("Expected body record"))?
            .clone();
        object.insert("mesh".into(), encode(mesh.transform(matrix)?)?);
        if body.get("brep").is_some() {
            let model = brep_core::transform::affine(&field(&body, "brep")?, matrix)?;
            object.insert("brep".into(), encode(model)?);
        }
        result.push(Value::Object(object));
    }
    encode(result)
}

#[cfg(test)]
mod instance_tests {
    use super::*;
    use value_codec::json;

    #[test]
    fn batch_placements_preserve_order_and_refuse_singular_matrix() {
        let identity = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];
        let mut moved = identity;
        moved[0][3] = 7.;
        let mut v = json!({"mesh":{"positions":[0.,0.,0.,1.,0.,0.,0.,1.,0.],"indices":[0,1,2]},"matrices":[identity,moved]});
        let result = instances(v.clone()).unwrap();
        // The direct Value tree must match the former codec round trip.
        assert_eq!(result, encode(result.clone()).unwrap());
        let meshes: Vec<Value> = value_codec::from_value(result).unwrap();
        assert_eq!(field::<Mesh>(&meshes[0], "mesh").unwrap().positions[0], 0.);
        assert_eq!(field::<Mesh>(&meshes[1], "mesh").unwrap().positions[0], 7.);
        moved[0][0] = 0.;
        v["matrices"] = json!([identity, moved]);
        assert!(instances(v).is_err());
    }

    #[test]
    fn instance_edit_composes_about_current_bounds_center() {
        let v = json!({
            "mesh":{"positions":[10.,0.,0.,11.,2.,3.],"indices":[]},
            "matrix":[[1.,0.,0.,10.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]],
            "delta":[1.,0.,0.],"axis":[0.,0.,1.],"angle":90.,"scale":2.
        });
        let result: [[f64; 4]; 4] =
            value_codec::from_value(instance_transform(v).unwrap()).unwrap();
        let expected = [
            [0., -2., 0., 13.5],
            [2., 0., 0., 0.],
            [0., 0., 2., -1.5],
            [0., 0., 0., 1.],
        ];
        for i in 0..4 {
            for j in 0..4 {
                assert!((result[i][j] - expected[i][j]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn instance_edit_rejects_invalid_affine_and_zero_scale() {
        let v = json!({
            "mesh":{"positions":[0.,0.,0.,1.,2.,3.],"indices":[]},
            "matrix":[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]],
            "delta":[0.,0.,0.],"axis":[0.,0.,1.],"angle":0.,"scale":0.
        });
        assert!(instance_transform(v.clone()).is_err());
        let mut invalid = v;
        invalid["scale"] = json!(1.);
        invalid["matrix"][3][0] = json!(1.);
        assert!(instance_transform(invalid).is_err());
    }
}
