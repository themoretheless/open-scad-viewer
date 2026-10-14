//! Local sampled fillet/chamfer construction on a selected display seam.
use crate::{
    BuiltMesh, Mesh, Result,
    solid::{boolean, modeling},
};
use math_core::{cross, dot, norm, sub};
type P = [f64; 3];
fn unit(p: P) -> Result<P> {
    let length = norm(p);
    if length < 1e-9 {
        return Err(crate::error("Zero direction"));
    }
    Ok(p.map(|x| x / length))
}
pub fn blend(
    mesh: &Mesh,
    target: &Mesh,
    edge_id: usize,
    radius: f64,
    end_radius: f64,
    fillet: bool,
) -> Result<BuiltMesh> {
    if !radius.is_finite() || radius < 0.01 || !end_radius.is_finite() || end_radius < 0.01 {
        return Err(crate::error("Radius must be at least 0.01 mm."));
    }
    let report = mesh.inspect()?;
    if !report.closed || report.signed_volume_mm3 <= 0. {
        return Err(crate::error("Select a closed outward-oriented body."));
    }
    let topology = mesh_topology::planar::topology(mesh.view())?;
    let edges = topology.edges;
    let faces = topology.faces;
    let edge = edges
        .get(edge_id)
        .ok_or_else(|| crate::error("Select a manifold edge."))?;
    let a_id = edge.a;
    let b_id = edge.b;
    let face_ids = &edge.faces;
    if face_ids.len() != 2 {
        return Err(crate::error("Select a manifold edge."));
    }
    let a = mesh.view().point(a_id)?;
    let b = mesh.view().point(b_id)?;
    let length = norm(sub(b, a));
    let axis = unit(sub(b, a))?;
    let middle = std::array::from_fn::<_, 3, _>(|k| (a[k] + b[k]) / 2.);
    let mut adjacent = Vec::new();
    let mut sides = Vec::new();
    for i in face_ids {
        let face = &faces[*i];
        let mut point = face.center;
        for &t in &face.triangles {
            let ids = &mesh.indices[t * 3..t * 3 + 3];
            if ids.contains(&a_id) && ids.contains(&b_id) {
                let id = ids
                    .iter()
                    .find(|j| **j != a_id && **j != b_id)
                    .ok_or_else(|| crate::error("Degenerate edge"))?;
                point = mesh.view().point(*id)?;
                break;
            }
        }
        adjacent.push(point);
        let q = sub(point, middle);
        let along = dot(q, axis);
        sides.push(unit(std::array::from_fn(|k| q[k] - axis[k] * along))?);
    }
    let u = sides[0];
    let w = cross(axis, u);
    let second = [dot(sides[1], u), dot(sides[1], w)];
    let theta = second[0].clamp(-1., 1.).acos();
    if theta < 0.01 || std::f64::consts::PI - theta < 0.01 {
        return Err(crate::error(
            "The selected edge has no usable corner angle.",
        ));
    }
    let tangent = if fillet {
        radius.max(end_radius) / (theta / 2.).tan()
    } else {
        radius.max(end_radius)
    };
    let mut width = f64::INFINITY;
    for (i, f) in face_ids.iter().enumerate() {
        let mut max = f64::NEG_INFINITY;
        for &j in &faces[*f].vertices {
            max = max.max(dot(sub(mesh.view().point(j)?, middle), sides[i]));
        }
        width = width.min(max);
    }
    if tangent >= width * 0.95 || tangent >= length / 2. {
        return Err(crate::error(
            "Radius consumes an adjacent face or endpoint; reduce it.",
        ));
    }
    let reach = tangent * 2. + radius;
    let triangle = [
        [0., 0.],
        [reach, 0.],
        [second[0] * reach, second[1] * reach],
    ];
    let arc = if fillet {
        let mut arc = planar_geometry::sampled_corner::corner(
            &triangle,
            0,
            radius,
            planar_geometry::sampled_corner::Kind::Fillet,
        )?;
        arc.truncate(arc.len() - 2);
        arc
    } else {
        vec![[second[0] * radius, second[1] * radius], [radius, 0.]]
    };
    let mut profile = vec![[0., 0.]];
    profile.extend(arc);
    let sections = [
        profile
            .iter()
            .map(|p| std::array::from_fn(|k| a[k] + p[0] * u[k] + p[1] * w[k]))
            .collect::<Vec<P>>(),
        profile
            .iter()
            .map(|p| {
                std::array::from_fn(|k| {
                    a[k] + p[0] * end_radius / radius * u[k]
                        + p[1] * end_radius / radius * w[k]
                        + length * cross(u, w)[k]
                })
            })
            .collect(),
    ];
    let cutter = modeling::loft(&sections, true)?.mesh;
    let concave = dot(faces[face_ids[0]].normal, adjacent[1]) > faces[face_ids[0]].offset + 1e-6;
    let built = boolean::boolean(
        target,
        &cutter,
        if concave {
            boolean::Operation::Union
        } else {
            boolean::Operation::Difference
        },
        &boolean::Options::default(),
    )?;
    if !built.report.closed
        || built.report.signed_volume_mm3 <= 0.
        || (built.report.signed_volume_mm3 - target.inspect()?.signed_volume_mm3).abs() < 1e-8
    {
        return Err(crate::error(
            "Local edge blend did not produce a valid changed solid.",
        ));
    }
    Ok(built)
}
