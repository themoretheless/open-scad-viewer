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
        if mesh.positions.is_empty() {
            return Err(input("Empty geometry"));
        }
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for p in mesh.positions.as_chunks::<3>().0 {
            min[0] = min[0].min(p[0]);
            min[1] = min[1].min(p[1]);
            min[2] = min[2].min(p[2]);
            max[0] = max[0].max(p[0]);
            max[1] = max[1].max(p[1]);
            max[2] = max[2].max(p[2]);
        }
        let center: [f64; 3] = std::array::from_fn(|k| min[k] * 0.5 + max[k] * 0.5);
        let span = (max[0] * 0.5 - min[0] * 0.5)
            .max(max[1] * 0.5 - min[1] * 0.5)
            .max(max[2] * 0.5 - min[2] * 0.5);
        if !span.is_finite() || span <= 0. {
            return Err(input("Draft requires a nonzero body extent."));
        }
        for p in mesh.positions.as_chunks_mut::<3>().0 {
            let h = (p[0] - origin[0]) * n[0]
                + (p[1] - origin[1]) * n[1]
                + (p[2] - origin[2]) * n[2];
            let axial = (p[0] - center[0]) * n[0]
                + (p[1] - center[1]) * n[1]
                + (p[2] - center[2]) * n[2];
            let factor = 1. + h * tangent / span;
            if !factor.is_finite() || factor <= 0.01 {
                return Err(input(
                    "Draft collapses the section or exceeds finite numeric range.",
                ));
            }
            let scale_f = factor - 1.;
            p[0] += (p[0] - center[0] - axial * n[0]) * scale_f;
            p[1] += (p[1] - center[1] - axial * n[1]) * scale_f;
            p[2] += (p[2] - center[2] - axial * n[2]) * scale_f;
        }
        if !mesh.inspect()?.closed {
            return Err(input("Invalid draft result."));
        }
        let Value::Object(mut object) = body else {
            return Err(input("Expected body record"));
        };
        object.insert("mesh".into(), encode(mesh)?);
        result.push(Value::Object(object));
    }
    encode(result)
}
