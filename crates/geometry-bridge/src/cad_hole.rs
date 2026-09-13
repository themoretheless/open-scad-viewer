//! Retained-body hole cutters and native Boolean. No mesh fallback.
use super::{Result, Value, brep, encode, field, input};
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(v: [f64; 3]) -> Result<[f64; 3]> {
    let max = v.iter().map(|x| x.abs()).fold(0., f64::max);
    if max == 0. || !v.iter().all(|x| x.is_finite()) {
        return Err(input("Invalid hole direction."));
    }
    let n = v.map(|x| x / max);
    let length = n[0].hypot(n[1]).hypot(n[2]);
    Ok(n.map(|x| x / length))
}
pub fn hole(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    if body.get("brep").is_none() {
        return mesh_hole(v);
    }
    let model: brep_core::Model = field(&body, "brep")?;
    model.validate()?;
    let width: f64 = field(&v, "width")?;
    let depth: f64 = field(&v, "depth")?;
    let height: f64 = field(&v, "height")?;
    let secondary: f64 = field(&v, "secondary")?;
    let mode: String = field(&v, "mode")?;
    let origin: [f64; 3] = field(&v, "origin")?;
    let n = unit(field(&v, "axis")?)?;
    if !origin
        .iter()
        .chain([&width, &depth, &height, &secondary])
        .all(|x| x.is_finite())
        || width <= 0.
        || depth <= 0.
    {
        return Err(input("Diameter and depth must be positive and finite."));
    }
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
    let mut cutter = brep_core::cylinder(width / 2., depth)?;
    if mode == "counterbore" || mode == "countersink" {
        if secondary <= width || height <= 0. || height > depth {
            return Err(input(
                "Counterbore diameter/depth must exceed the hole diameter and fit its depth.",
            ));
        }
        let extra = if mode == "counterbore" {
            brep_core::cylinder(secondary / 2., height)?
        } else {
            brep_core::frustum(secondary / 2., width / 2., height)?
        };
        cutter = brep_core::boolean(&cutter, &extra, "union")?;
    } else if mode != "plain" {
        return Err(input("Invalid hole mode."));
    }
    let cutter = brep_core::transform::affine(&cutter, matrix)?;
    let result = brep_core::boolean(&model, &cutter, "difference")?;
    if result.bodies.is_empty() {
        return Err(input("Hole consumed the entire body."));
    }
    let mesh = brep::nurbs(&result, 4)?.built.mesh;
    let mut next = body
        .as_object()
        .ok_or_else(|| input("Expected body record"))?
        .clone();
    next.insert("brep".into(), encode(result)?);
    next.insert("mesh".into(), encode(mesh)?);
    Ok(Value::Object(next))
}

fn mesh_hole(v: Value) -> Result<Value> {
    use polygon_core::{
        Mesh,
        solid::{
            boolean::{Operation, Options, boolean},
            modeling::{Profile, extrude, loft},
        },
    };
    let body: Value = field(&v, "body")?;
    let mesh: Mesh = field(&body, "mesh")?;
    mesh.validate()?;
    let width: f64 = field(&v, "width")?;
    let depth: f64 = field(&v, "depth")?;
    let height: f64 = field(&v, "height")?;
    let secondary: f64 = field(&v, "secondary")?;
    let mode: String = field(&v, "mode")?;
    let origin: [f64; 3] = field(&v, "origin")?;
    let n = unit(field(&v, "axis")?)?;
    if !origin
        .iter()
        .chain([&width, &depth, &height, &secondary])
        .all(|x| x.is_finite())
        || width <= 0.
        || depth <= 0.
    {
        return Err(input("Diameter and depth must be positive and finite."));
    }
    if !["plain", "counterbore", "countersink"].contains(&mode.as_str()) {
        return Err(input("Invalid hole mode."));
    }
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
    let circle = |radius: f64| -> Vec<[f64; 2]> {
        (0..48)
            .map(|i| {
                let angle = i as f64 * std::f64::consts::PI / 24.;
                [radius * angle.cos(), radius * angle.sin()]
            })
            .collect()
    };
    let mut cutter = extrude(
        &Profile {
            outer: circle(width / 2.),
            holes: vec![],
        },
        [0., 0., depth],
    )?
    .mesh
    .transform(matrix)?;
    if mode != "plain" {
        if secondary <= width || height <= 0. || height > depth {
            return Err(input(
                "Counterbore diameter/depth must exceed the hole diameter and fit its depth.",
            ));
        }
        let sections = vec![
            circle(secondary / 2.)
                .iter()
                .map(|p| [p[0], p[1], 0.])
                .collect(),
            circle(if mode == "countersink" {
                width / 2.
            } else {
                secondary / 2.
            })
            .iter()
            .map(|p| [p[0], p[1], height])
            .collect(),
        ];
        let extra = loft(&sections, true)?.mesh.transform(matrix)?;
        cutter = boolean(&cutter, &extra, Operation::Union, &Options::default())?.mesh;
    }
    let built = boolean(&mesh, &cutter, Operation::Difference, &Options::default())?;
    if !built.report.closed || built.mesh.indices.is_empty() {
        return Err(input("Hole did not produce a closed body."));
    }
    let mut result = body
        .as_object()
        .ok_or_else(|| input("Expected body record"))?
        .clone();
    result.insert("mesh".into(), encode(built.mesh)?);
    Ok(Value::Object(result))
}
