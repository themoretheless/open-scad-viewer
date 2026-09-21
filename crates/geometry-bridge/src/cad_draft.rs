//! Atomic draft of meshes and retained convex planar prisms.
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;

pub fn draft(v: Value) -> Result<Value> {
    let bodies: Vec<Value> = field(&v, "bodies")?;
    let axis: [f64; 3] = field(&v, "axis")?;
    let origin: [f64; 3] = field(&v, "origin")?;
    let angle: f64 = field(&v, "amount")?;
    if bodies.is_empty() {
        return Err(input("Select bodies."));
    }
    if !axis
        .iter()
        .chain(&origin)
        .chain([&angle])
        .all(|x| x.is_finite())
        || angle.abs() > 60.
    {
        return Err(input(
            "Draft requires finite parameters and an angle between -60 and 60 degrees.",
        ));
    }
    let magnitude = axis.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. {
        return Err(input("Zero direction"));
    }
    let n = axis.map(|x| x / magnitude);
    let length = n[0].hypot(n[1]).hypot(n[2]);
    let n = n.map(|x| x / length);
    let tangent = angle.to_radians().tan();
    let mut result = Vec::with_capacity(bodies.len());
    for body in bodies {
        if body.get("brep").is_some() {
            let model = brep_core::operations::draft_planar_prism(
                &field(&body, "brep")?,
                axis,
                origin,
                angle,
            )?;
            let mesh = super::brep::nurbs(&model, 1)?.built.mesh;
            let mut object = body
                .as_object()
                .ok_or_else(|| input("Expected body record"))?
                .clone();
            object.insert("brep".into(), encode(model)?);
            object.insert("mesh".into(), encode(mesh)?);
            result.push(Value::Object(object));
            continue;
        }
        let mut mesh: Mesh = field(&body, "mesh")?;
        mesh.validate()?;
        let (min, max) = polygon_core::scene_flatten::bounds(&[mesh.positions.clone()])?;
        let center: [f64; 3] = std::array::from_fn(|k| min[k] * 0.5 + max[k] * 0.5);
        let span = (0..3)
            .map(|k| max[k] * 0.5 - min[k] * 0.5)
            .fold(0., f64::max);
        if !span.is_finite() || span <= 0. {
            return Err(input("Draft requires a nonzero body extent."));
        }
        for p in mesh.positions.as_chunks_mut::<3>().0 {
            let h: f64 = (0..3).map(|k| (p[k] - origin[k]) * n[k]).sum();
            let axial: f64 = (0..3).map(|k| (p[k] - center[k]) * n[k]).sum();
            let factor = 1. + h * tangent / span;
            if !factor.is_finite() || factor <= 0.01 {
                return Err(input(
                    "Draft collapses the section or exceeds finite numeric range.",
                ));
            }
            for k in 0..3 {
                p[k] += (p[k] - center[k] - axial * n[k]) * (factor - 1.);
            }
        }
        if !mesh.inspect()?.closed {
            return Err(input("Invalid draft result."));
        }
        let mut object = body
            .as_object()
            .ok_or_else(|| input("Expected body record"))?
            .clone();
        object.insert("mesh".into(), encode(mesh)?);
        result.push(Value::Object(object));
    }
    encode(result)
}
