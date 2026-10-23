//! Straight-prism recognition, prism/sphere shell construction, and prism longitudinal beveling.
use crate::{
    Mesh, Result, check, dot, error, mesh_error, norm,
    profile_tools::SketchPlane,
    solid::{
        boolean::{Operation, Options, boolean},
        modeling::{Profile, extrude},
    },
    sub,
};
use planar_geometry::{
    sampled_corner::{self, Kind},
    sketch_offset,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct StraightPrism {
    pub points: Vec<[f64; 2]>,
    pub plane: SketchPlane,
    pub height: f64,
    pub top: usize,
    pub bottom: usize,
}

impl StraightPrism {
    pub fn local(&self, q: [f64; 3]) -> [f64; 2] {
        let v = sub(q, self.plane.origin);
        [dot(v, self.plane.u), dot(v, self.plane.v)]
    }
}

fn unit(a: [f64; 3]) -> Result<[f64; 3]> {
    let l = norm(a);
    check(l.is_finite() && l >= 1e-12, "Zero direction")?;
    Ok(a.map(|v| v / l))
}

fn extrude_on_plane(
    points: &[[f64; 2]],
    plane: &SketchPlane,
    height: f64,
    base: f64,
) -> Result<Mesh> {
    let mut mesh = extrude(
        &Profile {
            outer: points.to_vec(),
            holes: Vec::new(),
        },
        [0., 0., height],
    )?
    .mesh;
    let local: Vec<[f64; 3]> = mesh
        .positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| [p[0], p[1], p[2] + base])
        .collect();
    mesh.positions = plane.place(&local)?.into_iter().flatten().collect();
    Ok(mesh)
}

/// Recognize an actual straight prism along `direction` before using a 2D offset/corner construction.
pub fn recognize_prism(mesh: &Mesh, direction: [f64; 3]) -> Result<StraightPrism> {
    let n = unit(direction)?;
    mesh.validate()?;
    let p = mesh.positions.as_chunks::<3>().0;
    let topology = mesh_topology::planar::topology(mesh.view()).map_err(mesh_error)?;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &q in p {
        let v = dot(q, n);
        min = min.min(v);
        max = max.max(v);
    }
    if !min.is_finite()
        || !max.is_finite()
        || max - min < 1e-7
        || p.iter().any(|&q| {
            let v = dot(q, n);
            (v - min).abs() > 1e-5 && (v - max).abs() > 1e-5
        })
    {
        return Err(error(
            "This extension requires a straight prism; general curved shells are not supported.",
        ));
    }
    let bottom = topology
        .faces
        .iter()
        .position(|f| dot(f.normal, n) < -1. + 1e-6);
    let top = topology
        .faces
        .iter()
        .position(|f| dot(f.normal, n) > 1. - 1e-6);
    let (Some(bottom), Some(top)) = (bottom, top) else {
        return Err(error("The prism must have two planar end caps."));
    };
    let f = &topology.faces[top];
    let mut plane =
        mesh_topology::planar::face_plane(mesh.view(), &f.vertices, f.normal).map_err(mesh_error)?;
    plane.origin = std::array::from_fn(|k| plane.origin[k] - n[k] * (max - min));
    let sketch_plane = SketchPlane {
        origin: plane.origin,
        u: plane.u,
        v: plane.v,
    };
    let mut bottom_indices = Vec::with_capacity(topology.faces[bottom].triangles.len() * 3);
    for &t in &topology.faces[bottom].triangles {
        bottom_indices.extend_from_slice(&mesh.indices[t * 3..t * 3 + 3]);
    }
    let bottom_mesh = Mesh {
        positions: mesh.positions.clone(),
        indices: bottom_indices,
        uv: None,
    };
    let loops = bottom_mesh.boundary_loops()?;
    if loops.len() != 1 {
        return Err(error(
            "Prisms with holes require a topology-aware offset.",
        ));
    }
    let local = |q: [f64; 3]| -> [f64; 2] {
        let v = sub(q, sketch_plane.origin);
        [dot(v, sketch_plane.u), dot(v, sketch_plane.v)]
    };
    let mut points = Vec::with_capacity(loops[0].len());
    points.extend(loops[0].iter().map(|&i| local(p[i])));
    if points.len() > 1 {
        let last = *points.last().unwrap();
        if (points[0][0] - last[0]).hypot(points[0][1] - last[1]) < 1e-7 {
            points.pop();
        }
    }
    let mut top_points = Vec::with_capacity(topology.faces[top].vertices.len());
    top_points.extend(topology.faces[top].vertices.iter().map(|&i| local(p[i])));
    if points.iter().any(|a| {
        !top_points
            .iter()
            .any(|b| (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-5)
    }) {
        return Err(error(
            "End profiles differ; this is not a straight prism.",
        ));
    }
    Ok(StraightPrism {
        points,
        plane: sketch_plane,
        height: max - min,
        top,
        bottom,
    })
}

/// Concentric inner skin for a tessellated sphere; bridge only the requested openings.
pub fn sphere_shell(mesh: &Mesh, openings: &[usize], thickness: f64) -> Result<Mesh> {
    mesh.validate()?;
    let points = mesh.positions.as_chunks::<3>().0;
    if points.is_empty() {
        return Err(error("Keep at least one closed face."));
    }
    let mut pmin = [f64::INFINITY; 3];
    let mut pmax = [f64::NEG_INFINITY; 3];
    for p in points {
        for k in 0..3 {
            pmin[k] = pmin[k].min(p[k]);
            pmax[k] = pmax[k].max(p[k]);
        }
    }
    let center: [f64; 3] = std::array::from_fn(|k| (pmin[k] + pmax[k]) / 2.);
    let radius = points.iter().map(|&p| norm(sub(p, center))).sum::<f64>() / points.len() as f64;
    let r_tol = 1e-5_f64.max(radius * 1e-5);
    if points
        .iter()
        .any(|&p| (norm(sub(p, center)) - radius).abs() > r_tol)
    {
        return Err(error(
            "Shell supports convex planar solids, straight prisms and tessellated spheres; this surface needs a general offset kernel.",
        ));
    }
    if thickness >= radius {
        return Err(error("Wall thickness collapses the sphere."));
    }
    let topology = mesh_topology::planar::topology(mesh.view()).map_err(mesh_error)?;
    let mut removed = BTreeSet::new();
    for &i in openings {
        if let Some(f) = topology.faces.get(i) {
            removed.extend(f.triangles.iter().copied());
        }
    }
    let count = points.len();
    let tri_count = mesh.indices.len() / 3;
    if removed.is_empty() || removed.len() == tri_count {
        return Err(error("Keep at least one closed face."));
    }
    let mut positions = Vec::with_capacity(mesh.positions.len() * 2);
    positions.extend_from_slice(&mesh.positions);
    let scale = (radius - thickness) / radius;
    for p in points {
        for k in 0..3 {
            positions.push(center[k] + (p[k] - center[k]) * scale);
        }
    }
    let mut indices = Vec::with_capacity((tri_count - removed.len()) * 6);
    let mut boundary_key_to_order = BTreeMap::<(usize, usize), usize>::new();
    let mut boundary_in_order = BTreeMap::<usize, [usize; 2]>::new();
    let mut order_seq = 0usize;
    for t in 0..tri_count {
        if removed.contains(&t) {
            continue;
        }
        let a = mesh.indices[t * 3];
        let b = mesh.indices[t * 3 + 1];
        let c = mesh.indices[t * 3 + 2];
        indices.extend([a, b, c, c + count, b + count, a + count]);
        for [u, v] in [[a, b], [b, c], [c, a]] {
            let key = (u.min(v), u.max(v));
            if let Some(ord) = boundary_key_to_order.remove(&key) {
                boundary_in_order.remove(&ord);
            } else {
                let ord = order_seq;
                order_seq += 1;
                boundary_key_to_order.insert(key, ord);
                boundary_in_order.insert(ord, [u, v]);
            }
        }
    }
    for [a, b] in boundary_in_order.into_values() {
        indices.extend([b, a, a + count, b, a + count, b + count]);
    }
    let built = Mesh {
        positions,
        indices,
        uv: None,
    };
    let report = built.inspect()?;
    if !report.closed || report.signed_volume_mm3 <= 0. {
        return Err(error("Sphere shell produced invalid topology."));
    }
    Ok(built)
}

/// Nonconvex/round straight-prism or tessellated-sphere shell fallback when convex planar shell refuses.
pub fn specialized_shell_fallback(
    mesh: &Mesh,
    openings: &[usize],
    thickness: f64,
) -> Result<Mesh> {
    if !thickness.is_finite() || thickness < 0.01 || openings.is_empty() {
        return Err(error(
            "Select at least one opening and a positive wall thickness.",
        ));
    }
    mesh.validate()?;
    let topology = mesh_topology::planar::topology(mesh.view()).map_err(mesh_error)?;
    let first = topology
        .faces
        .get(openings[0])
        .ok_or_else(|| error("Keep at least one closed face."))?;
    let p = match recognize_prism(mesh, first.normal) {
        Ok(p) => p,
        Err(_) => return sphere_shell(mesh, openings, thickness),
    };
    if openings.iter().any(|&i| i != p.top && i != p.bottom) {
        return Err(error(
            "For a nonconvex/round prism, open one or both end caps.",
        ));
    }
    let inner = sketch_offset::offset(&p.points, -thickness)?;
    let base = if openings.contains(&p.bottom) {
        -thickness
    } else {
        thickness
    };
    let end = if openings.contains(&p.top) {
        p.height + thickness
    } else {
        p.height - thickness
    };
    if end <= base {
        return Err(error("Wall thickness collapses the cavity."));
    }
    let cavity = extrude_on_plane(&inner, &p.plane, end - base, base)?;
    let built = boolean(mesh, &cavity, Operation::Difference, &Options::default())?;
    if !built.report.closed || built.mesh.indices.is_empty() {
        return Err(error("Shell produced an invalid solid."));
    }
    Ok(built.mesh)
}

/// Straight-prism longitudinal edge fillet or chamfer when convex planar edge bevel refuses.
pub fn prism_bevel(mesh: &Mesh, edge_index: usize, size: f64, fillet: bool) -> Result<Mesh> {
    if !size.is_finite() || size < 0.01 {
        return Err(error("Size must be at least 0.01 mm."));
    }
    mesh.validate()?;
    let topology = mesh_topology::planar::topology(mesh.view()).map_err(mesh_error)?;
    let e = topology
        .edges
        .get(edge_index)
        .ok_or_else(|| error("Select an edge."))?;
    let pa = mesh.point(e.a)?;
    let pb = mesh.point(e.b)?;
    let p = recognize_prism(mesh, unit(sub(pb, pa))?)?;
    let q = p.local(pa);
    let vertex = p
        .points
        .iter()
        .position(|v| (v[0] - q[0]).hypot(v[1] - q[1]) < 1e-5)
        .ok_or_else(|| error("Select a longitudinal prism edge."))?;
    let profile = if fillet {
        sampled_corner::corner(&p.points, vertex, size, Kind::Fillet)?
    } else {
        let n = p.points.len();
        let a = p.points[(vertex + n - 1) % n];
        let b = p.points[vertex];
        let c = p.points[(vertex + 1) % n];
        let la = (a[0] - b[0]).hypot(a[1] - b[1]);
        let lc = (c[0] - b[0]).hypot(c[1] - b[1]);
        if size >= la.min(lc) / 2. {
            return Err(error("Chamfer consumes an adjacent edge."));
        }
        let mut next = Vec::with_capacity(n + 1);
        next.extend_from_slice(&p.points[..vertex]);
        next.push([
            b[0] + (a[0] - b[0]) * size / la,
            b[1] + (a[1] - b[1]) * size / la,
        ]);
        next.push([
            b[0] + (c[0] - b[0]) * size / lc,
            b[1] + (c[1] - b[1]) * size / lc,
        ]);
        next.extend_from_slice(&p.points[vertex + 1..]);
        next
    };
    let out = extrude_on_plane(&profile, &p.plane, p.height, 0.)?;
    if !out.inspect()?.closed {
        return Err(error("Invalid beveled prism."));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonconvex_prism_shell_and_bevel_succeed_natively() {
        let l_profile = Profile {
            outer: vec![
                [0., 0.],
                [20., 0.],
                [20., 10.],
                [10., 10.],
                [10., 20.],
                [0., 20.],
            ],
            holes: Vec::new(),
        };
        let mesh = extrude(&l_profile, [0., 0., 10.]).unwrap().mesh;
        let topology = mesh_topology::planar::topology(mesh.view()).unwrap();
        let top = topology
            .faces
            .iter()
            .position(|f| f.normal[2] > 0.99)
            .unwrap();
        let bottom = topology
            .faces
            .iter()
            .position(|f| f.normal[2] < -0.99)
            .unwrap();
        for openings in [vec![top], vec![top, bottom]] {
            let shelled = specialized_shell_fallback(&mesh, &openings, 1.).unwrap();
            let report = shelled.inspect().unwrap();
            assert!(report.closed);
            assert!(report.signed_volume_mm3 > 0. && report.signed_volume_mm3 < 3000.);
        }
        let edge = topology
            .edges
            .iter()
            .position(|e| {
                (mesh.positions[e.a * 3 + 2] - mesh.positions[e.b * 3 + 2]).abs() > 9.
            })
            .unwrap();
        let filleted = prism_bevel(&mesh, edge, 1., true).unwrap();
        assert!(filleted.inspect().unwrap().closed);
        let chamfered = prism_bevel(&mesh, edge, 1., false).unwrap();
        assert!(chamfered.inspect().unwrap().closed);
    }

    #[test]
    fn tessellated_sphere_shell_succeeds_natively() {
        let profile: Vec<[f64; 2]> = (0..13)
            .map(|i| {
                if i == 0 {
                    [0., -10.]
                } else if i == 12 {
                    [0., 10.]
                } else {
                    let a = i as f64 * std::f64::consts::PI / 12.;
                    [10. * a.sin(), -10. * a.cos()]
                }
            })
            .collect();
        let mesh = crate::solid::modeling::revolve(&profile, 360., 24, true)
            .unwrap()
            .mesh;
        let shelled = sphere_shell(&mesh, &[0], 1.).unwrap();
        let report = shelled.inspect().unwrap();
        assert!(report.closed);
        assert!(report.signed_volume_mm3 > 0.);
    }
}
