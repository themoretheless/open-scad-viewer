use brep_core::{Model, Shell, watertight};
use std::collections::BTreeMap;
fn cube_patches() -> Vec<Model> {
    let cube = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    cube.shells[0]
        .faces
        .iter()
        .map(|fu| {
            let mut m = Model::empty(cube.tolerance_mm).unwrap();
            let mut vertices = BTreeMap::new();
            let f = &cube.faces[fu.face];
            let mut face = f.clone();
            let mut wire = cube.loops[f.outer].clone();
            for c in &mut wire.coedges {
                let mut edge = cube.edges[c.edge].clone();
                edge.vertices = edge.vertices.map(|v| {
                    *vertices.entry(v).or_insert_with(|| {
                        let id = m.vertices.len();
                        m.vertices.push(cube.vertices[v].clone());
                        id
                    })
                });
                c.edge = m.edges.len();
                m.edges.push(edge);
            }
            face.outer = 0;
            face.holes.clear();
            m.faces.push(face);
            m.loops.push(wire);
            m.shells.push(Shell {
                faces: vec![brep_core::FaceUse {
                    face: 0,
                    reversed: fu.reversed,
                }],
                closed: false,
            });
            m.rebuild_topology_ids();
            m.validate().unwrap();
            m
        })
        .collect()
}
fn subdivide(m: &mut Model) {
    let i = m.loops[0].coedges.iter().position(|c| !c.reversed).unwrap();
    let c = m.loops[0].coedges[i].clone();
    let old = m.edges[c.edge].clone();
    let parts = old.curve.split(0.5).unwrap();
    let uv = c.pcurve.split(0.5).unwrap();
    let point = old.curve.evaluate(0.5).unwrap().point;
    let mid = m.vertices.len();
    m.vertices.push(brep_core::Vertex {
        point: [point[0], point[1], point[2]],
    });
    m.edges[c.edge].curve = parts[0].clone();
    m.edges[c.edge].vertices[1] = mid;
    let next = m.edges.len();
    m.edges.push(brep_core::Edge {
        vertices: [mid, old.vertices[1]],
        curve: parts[1].clone(),
        degenerate: false,
    });
    m.loops[0].coedges[i].pcurve = uv[0].clone();
    m.loops[0].coedges.insert(
        i + 1,
        brep_core::Coedge::new(
            next,
            false,
            uv[1].clone(),
        ),
    );
    m.rebuild_topology_ids();
    m.validate().unwrap();
}
fn nonlinear_at(c: &mut nurbs_core::curve::Curve, h: f64) {
    let a = c.control_points[0].clone();
    let b = c.control_points[1].clone();
    c.degree = 2;
    c.knots = vec![0., 0., 0., 1., 1., 1.];
    c.control_points = vec![
        a.clone(),
        a.iter().zip(&b).map(|(x, y)| x + h * (y - x)).collect(),
        b,
    ];
    c.weights = vec![1.; 3];
}
#[test]
fn independent_cube_patch_images_are_closed_with_unique_opposite_uses() {
    let models = cube_patches();
    let before = models.clone();
    let r = watertight::inspect_linear_patch_set(&models, 100, 10000).unwrap();
    assert!(r.all_boundaries_paired, "{r:?}");
    assert_eq!(r.matches.len(), 12);
    assert_eq!(r.boundaries.len(), 24);
    assert_eq!(models, before);
}
#[test]
fn automatic_partial_partition_and_nonlinear_parameterization_cover_all_original_images() {
    let mut models = cube_patches();
    subdivide(&mut models[0]);
    for c in &mut models[1].loops[0].coedges {
        nonlinear_at(&mut c.pcurve, if c.reversed { 0.875 } else { 0.125 });
        c.pcurve.weights = if c.reversed {
            vec![2., 1.5, 1.]
        } else {
            vec![1., 1.5, 2.]
        };
    }
    for e in &mut models[1].edges {
        nonlinear_at(&mut e.curve, 0.125);
        e.curve.weights = vec![1., 1.5, 2.];
    }
    models[1].rebuild_topology_ids();
    let r = watertight::inspect_linear_patch_set(&models, 100, 10000).unwrap();
    assert!(r.all_boundaries_paired, "{r:?}");
    assert_eq!(r.matches.len(), 13);
    assert_eq!(r.boundaries.len(), 25);
    assert!(r.matches.iter().any(|m| m.range == [0., 0.5]));
    assert!(r.matches.iter().any(|m| m.range == [0.5, 1.]));
}
#[test]
fn holes_duplicates_interior_bulges_and_budget_exhaustion_do_not_pass() {
    let mut models = cube_patches();
    models.pop();
    assert!(
        !watertight::inspect_linear_patch_set(&models, 100, 10000)
            .unwrap()
            .all_boundaries_paired
    );
    let mut models = cube_patches();
    models.push(models[0].clone());
    let r = watertight::inspect_linear_patch_set(&models, 100, 10000).unwrap();
    assert!(!r.all_boundaries_paired && r.ambiguous_intervals > 0);
    let mut models = cube_patches();
    let curve = &mut models[0].edges[0].curve;
    nonlinear_at(curve, 0.125);
    curve.control_points[1][0] += 0.1;
    assert!(
        !watertight::inspect_linear_patch_set(&models, 100, 10000)
            .unwrap()
            .all_boundaries_paired
    );
    let models = cube_patches();
    assert!(
        !watertight::inspect_linear_patch_set(&models, 100, 1)
            .unwrap()
            .all_boundaries_paired
    );
    assert!(watertight::inspect_linear_patch_set(&models, 1, 10000).is_err());
    assert!(watertight::inspect_linear_patch_set(&[], 100, 10000).is_err());
}
