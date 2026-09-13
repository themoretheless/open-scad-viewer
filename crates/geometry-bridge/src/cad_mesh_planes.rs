//! Numerical convex mesh editing. This does not infer authored B-rep geometry.
use super::{Result, Value, cad_mesh_topology, encode, field, input};
use polygon_core::{
    Mesh,
    solid::boolean::{Operation, Options, boolean},
};
type V = [f64; 3];
#[derive(Clone, Copy)]
struct Plane {
    normal: V,
    offset: f64,
}
fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn add(a: V, b: V) -> V {
    std::array::from_fn(|k| a[k] + b[k])
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|k| a[k] - b[k])
}
fn mul(a: V, s: f64) -> V {
    a.map(|x| x * s)
}
fn norm(a: V) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn unit(a: V) -> Result<V> {
    let n = norm(a);
    if !n.is_finite() || n < 1e-9 {
        return Err(input("Zero direction"));
    }
    Ok(mul(a, 1. / n))
}
fn finite(a: V) -> Result<()> {
    if a.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(input("Plane edit exceeds finite numeric range."))
    }
}
fn topology(mesh: &Mesh) -> Result<Value> {
    cad_mesh_topology::topology(value_codec::json!({"mesh":mesh}))
}
fn admit(mesh: &Mesh) -> Result<Vec<Plane>> {
    if !mesh.inspect()?.closed {
        return Err(input("This operation requires a closed solid."));
    }
    let faces: Vec<Value> = field(&topology(mesh)?, "faces")?;
    if faces.len() > 64 {
        return Err(input(
            "This operation supports convex solids with up to 64 planar faces.",
        ));
    }
    let planes = faces
        .iter()
        .map(|f| {
            Ok(Plane {
                normal: field(f, "normal")?,
                offset: field(f, "offset")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    for f in &planes {
        for p in mesh.positions.chunks_exact(3) {
            let d = dot(f.normal, [p[0], p[1], p[2]]);
            if !d.is_finite() || d > f.offset + 1e-5 {
                return Err(input(
                    "This operation currently requires a convex solid with planar faces.",
                ));
            }
        }
    }
    Ok(planes)
}
fn construct(planes: &[Plane]) -> Result<Mesh> {
    if planes.len() > 128 {
        return Err(input("Plane construction budget exceeded."));
    }
    for f in planes {
        finite(f.normal)?;
        if !f.offset.is_finite() {
            return Err(input("Plane edit exceeds finite numeric range."));
        }
    }
    let eps = 1e-6;
    let mut points = Vec::<V>::new();
    for i in 0..planes.len() {
        for j in i + 1..planes.len() {
            for k in j + 1..planes.len() {
                let (a, b, c) = (planes[i], planes[j], planes[k]);
                let bc = cross(b.normal, c.normal);
                let det = dot(a.normal, bc);
                if det.abs() < 1e-8 {
                    continue;
                }
                let p = mul(
                    add(
                        add(mul(bc, a.offset), mul(cross(c.normal, a.normal), b.offset)),
                        mul(cross(a.normal, b.normal), c.offset),
                    ),
                    1. / det,
                );
                finite(p)?;
                let mut outside = false;
                for f in planes {
                    let d = dot(f.normal, p);
                    if !d.is_finite() {
                        return Err(input("Plane edit exceeds finite numeric range."));
                    }
                    if d > f.offset + eps {
                        outside = true;
                        break;
                    }
                }
                if outside || points.iter().any(|&q| norm(sub(p, q)) < eps) {
                    continue;
                }
                if points.len() >= 4096 {
                    return Err(input("Plane vertex budget exceeded."));
                }
                points.push(p);
            }
        }
    }
    if points.len() < 4 {
        return Err(input(
            "The dimension collapses the solid. Use a smaller value.",
        ));
    }
    let mut indices = Vec::new();
    for face in planes {
        let mut ids = (0..points.len())
            .filter(|&i| (dot(face.normal, points[i]) - face.offset).abs() < eps)
            .collect::<Vec<_>>();
        if ids.len() < 3 {
            continue;
        }
        let center = ids.iter().fold([0.; 3], |a, &i| {
            add(a, mul(points[i], 1. / ids.len() as f64))
        });
        finite(center)?;
        let u = unit(sub(points[ids[0]], center))?;
        let v = cross(face.normal, u);
        let angle = |i: usize| {
            let d = sub(points[i], center);
            dot(d, v).atan2(dot(d, u))
        };
        ids.sort_by(|&a, &b| angle(a).total_cmp(&angle(b)));
        for i in 1..ids.len() - 1 {
            indices.extend([ids[0], ids[i], ids[i + 1]]);
        }
    }
    let mesh = Mesh {
        positions: points.into_iter().flatten().collect(),
        indices,
        uv: None,
    };
    let report = mesh.inspect()?;
    if !report.closed || !report.signed_volume_mm3.is_finite() || report.signed_volume_mm3 < 1e-8 {
        return Err(input("The operation produced an invalid solid."));
    }
    Ok(mesh)
}
fn finish(body: Value, mesh: Mesh) -> Result<Value> {
    let mut object = body
        .as_object()
        .ok_or_else(|| input("Expected body record"))?
        .clone();
    object.insert("mesh".into(), encode(mesh)?);
    Ok(Value::Object(object))
}
pub fn planar(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    let mesh: Mesh = field(&body, "mesh")?;
    let action: String = field(&v, "action")?;
    let amount: f64 = field(&v, "amount")?;
    let faces: Vec<usize> = field(&v, "faces")?;
    if !amount.is_finite() {
        return Err(input("Enter a finite distance."));
    }
    let mut planes = admit(&mesh)?;
    let result = match action.as_str() {
        "push" => {
            if faces.len() != 1 {
                return Err(input("Select exactly one face."));
            }
            planes
                .get_mut(faces[0])
                .ok_or_else(|| input("Select a face."))?
                .offset += amount;
            construct(&planes)?
        }
        "shell" => {
            if amount < 0.01 || faces.is_empty() {
                return Err(input(
                    "Select at least one opening and a positive wall thickness.",
                ));
            }
            let open = faces.into_iter().collect::<std::collections::BTreeSet<_>>();
            if open.len() >= planes.len() || open.iter().any(|&i| i >= planes.len()) {
                return Err(input("Keep at least one closed face."));
            }
            let (min, max) = polygon_core::scene_flatten::bounds(&[mesh.positions.clone()])?;
            let span = (0..3).map(|k| max[k] - min[k]).fold(0., f64::max) * 3. + amount;
            if !span.is_finite() {
                return Err(input("Shell extent exceeds finite numeric range."));
            }
            let cavity = planes
                .iter()
                .enumerate()
                .map(|(i, f)| Plane {
                    offset: f.offset - if open.contains(&i) { 0. } else { amount },
                    ..*f
                })
                .collect::<Vec<_>>();
            construct(&cavity)?;
            for (i, f) in planes.iter_mut().enumerate() {
                f.offset += if open.contains(&i) { span } else { -amount };
            }
            let cutter = construct(&planes)?;
            let built = boolean(&mesh, &cutter, Operation::Difference, &Options::default())?;
            if !built.report.closed || built.mesh.indices.is_empty() {
                return Err(input("The wall thickness collapses the body."));
            }
            built.mesh
        }
        _ => return Err(input("Invalid planar editing action.")),
    };
    finish(body, result)
}
pub fn edge(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    let mesh: Mesh = field(&body, "mesh")?;
    let selected: Vec<usize> = field(&v, "edges")?;
    let size: f64 = field(&v, "size")?;
    let kind: String = field(&v, "kind")?;
    let segments: usize = field(&v, "segments")?;
    if selected.len() != 1 {
        return Err(input("Select exactly one mesh edge."));
    }
    if !size.is_finite() || size < 0.01 {
        return Err(input("Size must be at least 0.01 mm."));
    }
    if !["chamfer", "fillet"].contains(&kind.as_str()) || !(2..=64).contains(&segments) {
        return Err(input("Invalid edge operation."));
    }
    let mut planes = admit(&mesh)?;
    let edges: Vec<Value> = field(&topology(&mesh)?, "edges")?;
    let edge = edges
        .get(selected[0])
        .ok_or_else(|| input("Select an edge."))?;
    let adjacent: [usize; 2] = field(edge, "faces")?;
    let vertex: usize = field(edge, "a")?;
    let (a, b) = (planes[adjacent[0]], planes[adjacent[1]]);
    let p = std::array::from_fn(|k| mesh.positions[vertex * 3 + k]);
    let alpha = dot(a.normal, b.normal).clamp(-1., 1.).acos();
    if alpha < 1e-4 || std::f64::consts::PI - alpha < 1e-4 {
        return Err(input("Select a convex edge."));
    }
    let bisector = unit(add(a.normal, b.normal))?;
    let center = sub(p, mul(bisector, size / (alpha / 2.).cos()));
    finite(center)?;
    if kind == "chamfer" {
        planes.push(Plane {
            normal: bisector,
            offset: dot(bisector, p) - size * (alpha / 2.).sin(),
        });
    } else {
        for i in 1..segments {
            let t = i as f64 / segments as f64;
            let normal = unit(add(
                mul(a.normal, ((1. - t) * alpha).sin()),
                mul(b.normal, (t * alpha).sin()),
            ))?;
            planes.push(Plane {
                normal,
                offset: dot(normal, center) + size,
            });
        }
    }
    let result = construct(&planes)?;
    let faces: Vec<Value> = field(&topology(&result)?, "faces")?;
    for f in [a, b] {
        let mut found = false;
        for g in &faces {
            let normal: V = field(g, "normal")?;
            let offset: f64 = field(g, "offset")?;
            if dot(normal, f.normal) > 1. - 1e-6 && (offset - f.offset).abs() < 1e-5 {
                found = true;
                break;
            }
        }
        if !found {
            return Err(input(
                "Size consumes an adjacent face. Use a smaller value.",
            ));
        }
    }
    finish(body, result)
}
