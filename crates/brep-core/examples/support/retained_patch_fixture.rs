#[path = "corner_close_fixture.rs"]
mod fixture;
use brep_core::{Coedge, Model, face_cut, region_replacement as r};
use nurbs_core::{curve::Curve, curve_surface_agreement};

pub fn source() -> (Model, usize, face_cut::OwnedCut, [[f64; 2]; 1]) {
    let (open, supports, options) = fixture::fixture();
    let closed = brep_core::corner_close::close(
        &open,
        0,
        supports,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100_000,
        1_000_000,
    )
    .unwrap();
    let m = closed.model;
    let f = closed.face;
    let ring = &m.loops[m.faces[f].outer].coedges;
    assert_eq!(ring.len(), 4);
    let uv = ring
        .iter()
        .map(|c| c.pcurve.control_points[0].clone())
        .collect::<Vec<_>>();
    let endpoints = [0, 2].map(|i| m.edges[ring[i].edge].vertices[usize::from(ring[i].reversed)]);
    let pc = Curve::from_polyline(vec![uv[0].clone(), uv[2].clone()]).unwrap();
    let curve = curve_surface_agreement::lift(
        &pc,
        &m.faces[f].surface,
        Some(endpoints.map(|v| m.vertices[v].point)),
        1e-7,
        100_000,
    )
    .unwrap()
    .curve
    .unwrap();
    let seed = [std::array::from_fn(|k| {
        uv[0][k] * 0.25 + uv[1][k] * 0.5 + uv[2][k] * 0.25
    })];
    (
        m,
        f,
        face_cut::OwnedCut {
            endpoints,
            curves: vec![curve],
        },
        seed,
    )
}
pub fn fan(plan: &face_cut::RetentionNetwork) -> nurbs_core::Result<r::OwnedPatchSet> {
    let m = &plan.model;
    let region = &plan.regions[0];
    assert_eq!(region.discard_faces.len(), 1);
    let f = region.discard_faces[0];
    let surface = &m.faces[f].surface;
    let rim = &m.loops[m.faces[f].outer].coedges;
    let n = rim.len();
    assert!((3..=16).contains(&n));
    // Positive dyadic weights give an exact interior point of this convex rim.
    let w = 1. / n.next_power_of_two() as f64;
    let uv: [f64; 2] = std::array::from_fn(|k| {
        rim[..n - 1]
            .iter()
            .map(|c| c.pcurve.control_points[0][k] * w)
            .sum::<f64>()
            + rim[n - 1].pcurve.control_points[0][k] * (1. - (n - 1) as f64 * w)
    });
    let point: [f64; 3] = surface.evaluate(uv[0], uv[1])?.point.try_into().unwrap();
    let mut edges = Vec::new();
    let mut pcs = Vec::new();
    for co in rim {
        let start = m.edges[co.edge].vertices[usize::from(co.reversed)];
        let pc = Curve::from_polyline(vec![co.pcurve.control_points[0].clone(), uv.to_vec()])?;
        let curve = curve_surface_agreement::lift(
            &pc,
            surface,
            Some([m.vertices[start].point, point]),
            1e-7,
            100_000,
        )?
        .curve
        .unwrap();
        edges.push(brep_core::Edge {
            vertices: [start, m.vertices.len()],
            curve,
            degenerate: false,
        });
        pcs.push(pc);
    }
    let reversed = m
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .find(|u| u.face == f)
        .unwrap()
        .reversed;
    let patches = (0..n)
        .map(|i| r::OwnedSewPatch {
            surface: surface.clone(),
            reversed,
            loops: vec![vec![
                rim[i].clone(),
                Coedge::new(m.edges.len() + (i + 1) % n, false, pcs[(i + 1) % n].clone()),
                Coedge::new(m.edges.len() + i, true, pcs[i].reverse().unwrap()),
            ]],
        })
        .collect();
    Ok(r::OwnedPatchSet {
        vertices: vec![brep_core::Vertex { point }],
        edges,
        patches,
    })
}
