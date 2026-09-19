//! Handle-based application boundary for our Rust CAD algorithms.
use crate::{Result, encode, field, input};
use planar_geometry::rings::{self as planar, Rings};
use polygon_core::Mesh;
use polygon_core::solid::{boolean, modeling, primitives as solid, section};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use value_codec::{Value, json};
pub(crate) use crate::mesh_render::RenderMesh;
#[derive(Clone)]
enum Shape {
    Solid(Mesh),
    Profile(Rings),
}
#[derive(Default)]
struct Store {
    next: u32,
    values: HashMap<u32, Rc<Shape>>,
}
thread_local! {static STORE:RefCell<Store>=RefCell::new(Store::default());}
fn put(shape: Shape) -> Result<Value> {
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        if s.values.len() >= 20000 {
            return Err(input("CAD handle budget exceeded"));
        }
        s.next = s
            .next
            .checked_add(1)
            .ok_or_else(|| input("CAD handle ID exhausted"))?;
        let id = s.next;
        s.values.insert(id, Rc::new(shape));
        Ok(json!(id))
    })
}
fn get(id: u32) -> Result<Rc<Shape>> {
    STORE.with(|s| {
        s.borrow()
            .values
            .get(&id)
            .cloned()
            .ok_or_else(|| input("Unknown or deleted own CAD handle"))
    })
}
fn solid(s: &Shape) -> Result<&Mesh> {
    match s {
        Shape::Solid(m) => Ok(m),
        _ => Err(input("Expected solid")),
    }
}
fn profile(s: &Shape) -> Result<&Rings> {
    match s {
        Shape::Profile(r) => Ok(r),
        _ => Err(input("Expected profile")),
    }
}
fn boolean(a: &Mesh, b: &Mesh, op: &str) -> Result<Mesh> {
    if op == "union"
        && let Some(m) = solid::join_touching(a, b)?
    {
        return Ok(m);
    }
    // The prism arrangement is an exact shortcut, not the operation itself: when
    // its planar triangulation refuses a profile (many holes, bridge budget),
    // the general Boolean still owns the result.
    if let Ok(Some(m)) = solid::prism_boolean(a, b, op) {
        return Ok(m);
    }
    Ok(boolean::boolean(
        a,
        b,
        match op {
            "difference" => boolean::Operation::Difference,
            "intersection" => boolean::Operation::Intersection,
            _ => boolean::Operation::Union,
        },
        &Default::default(),
    )?
    .mesh)
}
/// Cutters subtracted per BSP step. Measured on a plate with 64 drilled holes:
/// one joined cutter fragments every plate face against every hole, while
/// small batches keep each clip local (see docs/design/csg-scaling-2026-09-19.md).
pub(crate) const DIFFERENCE_BATCH: usize = 8;
/// Ordered n-ary Boolean over solid operands. Union and difference route
/// through the bound-aware folds in `polygon_core::solid::boolean`, so
/// separated operands are joined without CSG; intersection folds pairwise.
pub(crate) fn combine_solids(meshes: &[Mesh], op: &str) -> Result<Mesh> {
    let mut pairwise = |a: &Mesh, b: &Mesh| boolean(a, b, op);
    match (op, meshes.split_first()) {
        (_, None) => Ok(solid::empty()),
        ("union", _) => boolean::union_many(meshes, &mut pairwise),
        ("difference", Some((base, cutters))) => {
            boolean::difference_many(base, cutters, &mut pairwise, DIFFERENCE_BATCH)
        }
        (_, Some((first, rest))) => {
            let mut m = first.clone();
            for b in rest {
                m = pairwise(&m, b)?;
            }
            Ok(m)
        }
    }
}
pub fn dispatch(v: Value) -> Result<Value> {
    let action = v["action"].as_str().unwrap_or("");
    if action == "delete" {
        let ids: Vec<u32> = field(&v, "ids")?;
        STORE.with(|s| {
            let mut s = s.borrow_mut();
            for id in ids {
                s.values.remove(&id);
            }
        });
        return Ok(Value::Null);
    }
    if action == "cube" {
        return put(Shape::Solid(solid::cube(
            field(&v, "size")?,
            field(&v, "center")?,
        )?));
    }
    if action == "sphere" {
        return put(Shape::Solid(solid::sphere(
            field(&v, "radius")?,
            field(&v, "segments")?,
        )?));
    }
    if action == "cylinder" {
        return put(Shape::Solid(solid::cylinder(
            field(&v, "height")?,
            field(&v, "bottom")?,
            field(&v, "top")?,
            field(&v, "segments")?,
            field(&v, "center")?,
        )?));
    }
    if action == "mesh" {
        return put(Shape::Solid(solid::clean(field(&v, "mesh")?)?));
    }
    if action == "profile" {
        let mut rings: Rings = field(&v, "rings")?;
        rings.retain(|r| r.len() >= 3);
        if v["fill"] == "EvenOdd" {
            let copy = rings.clone();
            for (i, r) in rings.iter_mut().enumerate() {
                let p = r.first().copied().ok_or_else(|| input("Empty polygon"))?;
                let mut depth = 0;
                for (j, other) in copy.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    let mut yes = false;
                    for k in 0..other.len() {
                        let a = other[k];
                        let b = other[(k + 1) % other.len()];
                        if (a[1] > p[1]) != (b[1] > p[1])
                            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
                        {
                            yes = !yes
                        }
                    }
                    if yes {
                        depth += 1
                    }
                }
                if (planar::area(r) > 0.) != (depth % 2 == 0) {
                    r.reverse()
                }
            }
        }
        return put(Shape::Profile(if v["fill"] == "NonZero" {
            planar::nonzero(&rings)?
        } else {
            planar::planar(&rings, &vec![], "union")?
        }));
    }
    if action == "combine" || action == "hull" {
        let ids: Vec<u32> = field(&v, "ids")?;
        let shapes: Vec<_> = ids.into_iter().map(get).collect::<Result<_>>()?;
        let dim = v["dimension"].as_u64().unwrap_or(3);
        let op = v["operation"].as_str().unwrap_or("union");
        if dim == 2 {
            let mut rings = vec![];
            if action == "hull" {
                let points = shapes
                    .iter()
                    .map(|s| profile(s))
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .flatten()
                    .copied()
                    .collect();
                let r = planar::hull2(points);
                if r.len() >= 3 {
                    rings.push(r)
                }
            } else {
                let planar_op = match op {
                    "xor" | "exclude" => "xor",
                    "intersection" => "intersection",
                    "difference" | "subtract" => "difference",
                    _ => "union",
                };
                for (i, s) in shapes.iter().enumerate() {
                    let r = profile(s)?;
                    rings = if i == 0 {
                        r.clone()
                    } else {
                        planar::planar(&rings, r, planar_op)?
                    };
                }
            }
            return put(Shape::Profile(rings));
        }
        let meshes = shapes
            .iter()
            .map(|s| solid(s).cloned())
            .collect::<Result<Vec<_>>>()?;
        let m = if action == "hull" {
            solid::hull3(&meshes)?
        } else if op == "compose" {
            solid::join(&meshes)?
        } else {
            combine_solids(&meshes, op)?
        };
        return put(Shape::Solid(m));
    }
    if matches!(
        action,
        "divide"
            | "crop"
            | "trim"
            | "minus_front"
            | "minus_back"
            | "shape_builder_extract"
            | "shape_builder_delete"
            | "make_compound"
    ) {
        let ids: Vec<u32> = field(&v, "ids")?;
        let shapes: Vec<_> = ids.into_iter().map(get).collect::<Result<_>>()?;
        let rings: Vec<Rings> = shapes
            .iter()
            .map(|s| profile(s).cloned())
            .collect::<Result<_>>()?;
        use planar_geometry::pathfinder;
        return match action {
            "divide" => put(Shape::Profile(pathfinder::divide(&rings)?)),
            "crop" => {
                let pieces = pathfinder::crop_to_front(&rings)?;
                let mut ids = vec![];
                for p in pieces {
                    ids.push(put(Shape::Profile(p))?);
                }
                Ok(json!(ids))
            }
            "trim" => {
                let pieces = pathfinder::trim_by_zorder(&rings)?;
                let mut ids = vec![];
                for p in pieces.into_iter().flatten() {
                    ids.push(put(Shape::Profile(p))?);
                }
                Ok(json!(ids))
            }
            "minus_front" => put(Shape::Profile(pathfinder::minus_front(&rings)?)),
            "minus_back" => put(Shape::Profile(pathfinder::minus_back(&rings)?)),
            "shape_builder_extract" => {
                let point: [f64; 2] = field(&v, "point")?;
                put(Shape::Profile(pathfinder::shape_builder_extract(
                    &rings, point,
                )?))
            }
            "shape_builder_delete" => {
                let point: [f64; 2] = field(&v, "point")?;
                put(Shape::Profile(pathfinder::shape_builder_delete(
                    &rings, point,
                )?))
            }
            "make_compound" => {
                let compounds = pathfinder::make_compound(&rings)?;
                let mut ids = vec![];
                for c in compounds {
                    ids.push(put(Shape::Profile(c.rings()))?);
                }
                Ok(json!(ids))
            }
            _ => unreachable!(),
        };
    }
    let shape = get(field(&v, "id")?)?;
    match action {
        "copy" => put((*shape).clone()),
        "split" => {
            let mesh = solid(&shape)?;
            let cutter = solid::halfspace(mesh, field(&v, "normal")?, field(&v, "offset")?)?;
            let left = boolean(mesh, &cutter, "intersection")?;
            let right = boolean(mesh, &cutter, "difference")?;
            let a = put(Shape::Solid(left))?;
            let b = put(Shape::Solid(right))?;
            Ok(json!([a, b]))
        }
        "decompose" => {
            let rings = profile(&shape)?;
            let mut ids = vec![];
            for outer in rings.iter().filter(|r| planar::area(r) > 0.) {
                let mut piece = vec![outer.clone()];
                piece.extend(
                    rings
                        .iter()
                        .filter(|r| planar::area(r) < 0. && planar::contains_point(r[0], outer))
                        .cloned(),
                );
                ids.push(put(Shape::Profile(piece))?)
            }
            Ok(json!(ids))
        }
        "transform" => {
            let m: [[f64; 4]; 4] = field(&v, "matrix")?;
            match &*shape {
                Shape::Solid(s) => {
                    if m.iter().flatten().any(|v| !v.is_finite()) {
                        return Err(input("Invalid affine matrix"));
                    }
                    let mut out = s.clone();
                    for p in out.positions.as_chunks_mut::<3>().0 {
                        let q = std::array::from_fn::<_, 3, _>(|i| {
                            m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2] + m[i][3]
                        });
                        p.copy_from_slice(&q)
                    }
                    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
                    if det < 0. {
                        out.reverse_winding()
                    }
                    put(Shape::Solid(out))
                }
                Shape::Profile(r) => {
                    let mut r = r.clone();
                    for p in r.iter_mut().flatten() {
                        *p = [
                            m[0][0] * p[0] + m[0][1] * p[1] + m[0][3],
                            m[1][0] * p[0] + m[1][1] * p[1] + m[1][3],
                        ]
                    }
                    if m[0][0] * m[1][1] - m[0][1] * m[1][0] < 0. {
                        for ring in &mut r {
                            ring.reverse()
                        }
                    }
                    put(Shape::Profile(r))
                }
            }
        }
        "offset" => put(Shape::Profile(planar::offset_join(
            profile(&shape)?,
            field(&v, "distance")?,
            v["join"].as_str().unwrap_or("Round"),
            field(&v, "segments")?,
        )?)),
        "extrude" => put(Shape::Solid(modeling::extrude_rings(
            profile(&shape)?,
            field(&v, "height")?,
            field(&v, "slices")?,
            field(&v, "twist")?,
            field(&v, "scale")?,
            field(&v, "center")?,
        )?)),
        "revolve" => {
            let rings = profile(&shape)?;
            let mut m = solid::empty();
            for r in rings {
                let mut ring = r.clone();
                if planar::area(&ring) < 0. {
                    ring.reverse()
                }
                let part =
                    modeling::revolve(&ring, field(&v, "angle")?, field(&v, "segments")?, true)?
                        .mesh;
                m = boolean(
                    &m,
                    &part,
                    if planar::area(r) < 0. {
                        "difference"
                    } else {
                        "union"
                    },
                )?;
            }
            put(Shape::Solid(m))
        }
        "project" => put(Shape::Profile(section::project(solid(&shape)?)?)),
        "slice" => put(Shape::Profile(section::slice(
            solid(&shape)?,
            field(&v, "height")?,
        )?)),
        "minkowski" => {
            let b = get(field(&v, "other")?)?;
            put(Shape::Solid(minkowski(solid(&shape)?, solid(&b)?)?))
        }
        "polygons" => encode(profile(&shape)?),
        "inspect" => match &*shape {
            Shape::Solid(m) => {
                let report = m.inspect()?;
                let mut area = 0.;
                for t in m.indices.as_chunks::<3>().0 {
                    let a = m.point(t[0])?;
                    let b = m.point(t[1])?;
                    let c = m.point(t[2])?;
                    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                    let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                    area += (u[1] * w[2] - u[2] * w[1])
                        .hypot(u[2] * w[0] - u[0] * w[2])
                        .hypot(u[0] * w[1] - u[1] * w[0])
                        / 2.;
                }
                let mut min = [f64::INFINITY; 3];
                let mut max = [f64::NEG_INFINITY; 3];
                for p in m.positions.as_chunks::<3>().0 {
                    for k in 0..3 {
                        min[k] = min[k].min(p[k]);
                        max[k] = max[k].max(p[k])
                    }
                }
                Ok(
                    json!({"empty":m.indices.is_empty(),"volume":report.signed_volume_mm3,"area":area,"min":min,"max":max,"report":report}),
                )
            }
            Shape::Profile(r) => {
                let mut min = [f64::INFINITY; 2];
                let mut max = [f64::NEG_INFINITY; 2];
                for p in r.iter().flatten() {
                    for k in 0..2 {
                        min[k] = min[k].min(p[k]);
                        max[k] = max[k].max(p[k])
                    }
                }
                Ok(
                    json!({"empty":r.is_empty(),"area":r.iter().map(|r|planar::area(r)).sum::<f64>(),"min":min,"max":max}),
                )
            }
        },
        "mesh" => encode(solid(&shape)?),
        "export_mesh" => encode(export_buffers(field(&v, "id")?)?),
        _ => Err(input(format!(
            "Own Rust CAD operation not implemented: {action}"
        ))),
    }
}

fn convex_parts(mesh: &Mesh, depth: usize, parts: &mut Vec<Mesh>) -> Result<()> {
    if mesh.indices.is_empty() {
        return Ok(());
    }
    if depth > 32 || parts.len() >= 64 {
        return Err(input("Minkowski convex decomposition budget exceeded"));
    }
    let mut split = None;
    for t in mesh.indices.as_chunks::<3>().0 {
        let a = mesh.point(t[0])?;
        let b = mesh.point(t[1])?;
        let c = mesh.point(t[2])?;
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let l = n[0].hypot(n[1]).hypot(n[2]);
        if l < 1e-16 {
            continue;
        }
        let n = n.map(|x| x / l);
        let d = n[0] * a[0] + n[1] * a[1] + n[2] * a[2];
        if mesh
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .any(|p| n[0] * p[0] + n[1] * p[1] + n[2] * p[2] - d > 1e-8)
        {
            split = Some((n, d));
            break;
        }
    }
    if let Some((n, d)) = split {
        let cutter = solid::halfspace(mesh, n, d)?;
        let a = boolean(mesh, &cutter, "intersection")?;
        let b = boolean(mesh, &cutter, "difference")?;
        if a.indices.is_empty() || b.indices.is_empty() {
            return Err(input("Minkowski decomposition reached numerical tolerance"));
        }
        convex_parts(&a, depth + 1, parts)?;
        convex_parts(&b, depth + 1, parts)?
    } else {
        parts.push(mesh.clone())
    }
    Ok(())
}
fn minkowski(a: &Mesh, b: &Mesh) -> Result<Mesh> {
    let mut left = vec![];
    let mut right = vec![];
    convex_parts(a, 0, &mut left)?;
    convex_parts(b, 0, &mut right)?;
    if left.len() * right.len() > 256 {
        return Err(input("Minkowski pair budget exceeded"));
    }
    let mut result = solid::empty();
    for a in &left {
        for b in &right {
            let sum = solid::minkowski(a, b)?;
            result = boolean(&result, &sum, "union")?;
        }
    }
    Ok(result)
}

/// A detached export snapshot. Its vectors remain valid until the JS lease is freed.
/// Commands may delete the source solid without invalidating this snapshot.
pub(crate) fn export_buffers(id: u32) -> Result<crate::CadMeshBuffer> {
    let shape = get(id)?;
    let m = solid(&shape)?;
    let mut groups = std::collections::BTreeMap::new();
    let mut ids = Vec::new();
    let scale = m.positions.iter().fold(1_f64, |a, b| a.max(b.abs()));
    for t in m.indices.as_chunks::<3>().0 {
        let a = m.point(t[0])?;
        let b = m.point(t[1])?;
        let c = m.point(t[2])?;
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let length = n[0].hypot(n[1]).hypot(n[2]);
        let n = n.map(|x| x / length);
        let d = n[0] * a[0] + n[1] * a[1] + n[2] * a[2];
        let key = [
            (n[0] * 1e7).round() as i64,
            (n[1] * 1e7).round() as i64,
            (n[2] * 1e7).round() as i64,
            (d / scale * 1e7).round() as i64,
        ];
        let next = groups.len();
        ids.push(*groups.entry(key).or_insert(next));
    }

    Ok(crate::CadMeshBuffer {
        positions: m
            .positions
            .iter()
            .map(|&v| if v.abs() < scale * 1e-14 { 0. } else { v })
            .collect(),
        indices: m.indices.iter().map(|&v| v as u32).collect(),
        face_ids: ids.into_iter().map(|v| v as u32).collect(),
    })
}
/// Detach the registry-owned solid before preparing its display buffers.
pub(crate) fn render_buffers(id: u32, crease_cosine: f64) -> Result<RenderMesh> {
    Ok(crate::mesh_render::render(export_buffers(id)?, crease_cosine))
}
pub(crate) fn import_buffers(stride: usize, vertices: &[f32], indices: &[u32]) -> Result<u32> {
    if !(3..=64).contains(&stride) || !vertices.len().is_multiple_of(stride) {
        return Err(input("Invalid mesh vertex stride"));
    }
    let mesh = Mesh {
        positions: vertices
            .chunks_exact(stride)
            .flat_map(|p| p[..3].iter().map(|&v| v as f64))
            .collect(),
        indices: indices.iter().map(|&i| i as usize).collect(),
        uv: None,
    };
    let id = put(Shape::Solid(solid::clean(mesh)?))?;
    Ok(id.as_u64().unwrap() as u32)
}
