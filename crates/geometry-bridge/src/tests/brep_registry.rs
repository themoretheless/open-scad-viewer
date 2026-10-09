use super::*;
use std::collections::BTreeSet;

#[test]
fn herringbone_gear_tessellation_preserves_manifold_seams() {
    let model = brep_core::gear(&brep_core::GearSpec {
        module: 2.,
        teeth: 12,
        height: 5.,
        helix_angle_deg: 20.,
        herringbone: true,
        bore: 6.,
        ..Default::default()
    })
    .unwrap();
    let built = nurbs(&model, 7).unwrap();
    assert!(built.built.report.closed);
    assert_eq!(built.built.report.degenerate_triangles, 0);
}

#[test]
fn spinner_gear_tessellation_preserves_manifold_seams() {
    let model = brep_core::gear(&brep_core::GearSpec {
        module: 1.5,
        teeth: 32,
        height: 10.,
        helix_angle_deg: -35.,
        herringbone: true,
        bore: 44.,
        clearance: 0.1,
        backlash: 0.05,
        ..Default::default()
    })
    .unwrap();
    let built = nurbs(&model, 4).unwrap();
    assert!(built.built.report.closed);
    assert_eq!(built.built.report.degenerate_triangles, 0);
}

fn empty_model() -> brep_core::Model {
    brep_core::Model(
        brep_topology::Model {
            vertices: vec![],
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        brep_core::TopologyIds::default(),
    )
}
#[test]
fn corrected_fixed_normal_hollow_body_tessellates_original_boundary() {
    use nurbs_core::{
        primitives::line,
        progressive_sweep::{Options, Orientation, Spacing, constant_vector_law},
    };
    let ring = |points: [[f64; 3]; 4]| {
        (0..4)
            .map(|i| line(points[i], points[(i + 1) % 4]).unwrap())
            .collect()
    };
    let loops = vec![
        ring([[0., 0., 0.], [0.1, 0., 0.], [0.1, 0.1, 0.], [0., 0.1, 0.]]),
        ring([
            [0.025, 0.025, 0.],
            [0.025, 0.075, 0.],
            [0.075, 0.075, 0.],
            [0.075, 0.025, 0.],
        ]),
    ];
    let path = nurbs_core::core::curve::Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0.; 3], vec![0., 0., 0.5], vec![0., 1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axes = constant_vector_law([2., 3., 1.]).unwrap();
    let center = constant_vector_law([0.; 3]).unwrap();
    let body = brep_core::analytic::progressive_profile_body_with_evidence_and_correction(
        &loops,
        &path,
        &scale,
        &twist,
        Some((&axes, &center)),
        None,
        None,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::FixedNormal,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 3,
            max_sections: 17,
            max_deviation: 2.,
        },
        Some(brep_core::analytic::EndpointCapCorrection {
            quantum: 2_f64.powi(-40),
            tolerance: 1e-9,
            max_work: 1000000,
        }),
    )
    .unwrap();
    assert_eq!(body.boundary_error_within_budget, Some(true));
    for detail in [1, 2, 4, 8] {
        let built = nurbs(&body.model, detail).unwrap_or_else(|e| panic!("detail={detail}: {e:?}"));
        assert!(built.built.report.closed, "detail={detail}");
    }
}
#[test]
fn periodic_step_sphere_tessellates_with_shared_seam_and_poles() {
    let source = brep_core::sphere(2.).unwrap();
    let text = brep_core::export_step_v6(&source).unwrap().0;
    let model = brep_core::import_step_v6(&text).unwrap().0;
    nurbs(&model, 8).unwrap();
}
fn append_model(target: &mut brep_core::Model, mut source: brep_core::Model) {
    let (vertices, edges, loops, faces, shells) = (
        target.vertices.len(),
        target.edges.len(),
        target.loops.len(),
        target.faces.len(),
        target.shells.len(),
    );
    for edge in &mut source.edges {
        edge.vertices = edge.vertices.map(|id| id + vertices);
    }
    for wire in &mut source.loops {
        for coedge in &mut wire.coedges {
            coedge.edge += edges;
        }
    }
    for face in &mut source.faces {
        face.outer += loops;
        for wire in &mut face.holes {
            *wire += loops;
        }
    }
    for shell in &mut source.shells {
        for face in &mut shell.faces {
            face.face += faces;
        }
    }
    for body in &mut source.bodies {
        body.outer_shell += shells;
        for shell in &mut body.inner_shells {
            *shell += shells;
        }
    }
    target.vertices.append(&mut source.vertices);
    target.edges.append(&mut source.edges);
    target.loops.append(&mut source.loops);
    target.faces.append(&mut source.faces);
    target.shells.append(&mut source.shells);
    target.bodies.append(&mut source.bodies);
    target.rebuild_topology_ids();
}
fn components(mesh: &Mesh) -> usize {
    let mut adjacency = vec![Vec::new(); mesh.positions.len() / 3];
    for triangle in mesh.indices.as_chunks::<3>().0 {
        for i in 0..3 {
            adjacency[triangle[i]].push(triangle[(i + 1) % 3]);
            adjacency[triangle[(i + 1) % 3]].push(triangle[i]);
        }
    }
    let mut visited = BTreeSet::new();
    let mut components = 0;
    for i in 0..adjacency.len() {
        if adjacency[i].is_empty() || visited.contains(&i) {
            continue;
        }
        components += 1;
        let mut pending = vec![i];
        while let Some(vertex) = pending.pop() {
            if visited.insert(vertex) {
                pending.extend(&adjacency[vertex]);
            }
        }
    }
    components
}
#[test]
fn nearby_disconnected_shells_keep_distinct_authored_vertices() {
    let gap = 1e-8;
    let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    append_model(
        &mut model,
        brep_core::cuboid([1. + gap, 0., 0.], [2. + gap, 1., 1.]).unwrap(),
    );
    model.validate().unwrap();
    assert!(gap < model.tolerance_mm);
    let built = nurbs(&model, 4).unwrap();
    assert!(built.built.report.closed);
    assert_eq!(built.built.report.vertex_count, 16);
    assert_eq!(components(&built.built.mesh), 2);
    assert!((built.built.report.signed_volume_mm3 - 2.).abs() < 1e-12);
    for (vertex, expected) in model.vertices.iter().enumerate() {
        assert_eq!(built.built.mesh.point(vertex).unwrap(), expected.point);
    }
}
#[test]
fn poles_round_holes_and_partial_turns_share_edge_indices_at_each_detail() {
    let models = [
        brep_core::sphere(3.).unwrap(),
        brep_core::frustum(3., 0., 5.).unwrap(),
        brep_core::frustum(0., 3., 5.).unwrap(),
        brep_core::tube(3., 2., 5.).unwrap(),
        brep_core::revolve_angle(&[[0., 0.], [2., 0.], [2., 3.], [0., 3.]], 125.).unwrap(),
    ];
    for (kind, model) in models.iter().enumerate() {
        for detail in [1, 2, 4, 8, 16, 32] {
            let result = nurbs(model, detail)
                .unwrap_or_else(|e| panic!("model {kind}, detail {detail}: {e}"));
            assert!(result.built.mesh.indices.len() / 3 <= 20_000);
            assert!(result.built.report.closed, "model {kind}, detail {detail}");
            assert_eq!(result.built.report.boundary_edges, 0);
            assert_eq!(result.built.report.non_manifold_edges, 0);
            assert_eq!(result.built.report.orientation_conflicts, 0);
            assert_eq!(result.built.report.degenerate_triangles, 0);
            assert!(result.built.report.signed_volume_mm3 > 0.);
            assert_eq!(components(&result.built.mesh), 1);
            assert_eq!(
                result.topology_face_ids.unwrap(),
                result
                    .face_ids
                    .iter()
                    .map(|&face| model.1.faces[face].to_string())
                    .collect::<Vec<_>>()
            );
        }
    }
}
#[test]
fn torus_periodic_seams_preserve_genus_and_obey_triangle_budget() {
    let model = brep_core::torus(5., 2.).unwrap();
    for detail in [1, 2, 4, 8, 16] {
        let built = nurbs(&model, detail).unwrap();
        assert!(built.built.report.closed);
        assert_eq!(components(&built.built.mesh), 1);
        let edges: BTreeSet<_> = built
            .built
            .mesh
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|triangle| {
                (0..3).map(|i| {
                    let a = triangle[i];
                    let b = triangle[(i + 1) % 3];
                    [a.min(b), a.max(b)]
                })
            })
            .collect();
        assert_eq!(
            built.built.report.vertex_count + built.built.report.triangle_count,
            edges.len()
        );
    }
    assert!(
        nurbs(&model, 32)
            .err()
            .unwrap()
            .message
            .contains("20000 triangles")
    );
}
#[test]
fn curved_prism_xor_keeps_contacting_components_and_trim_samples_separate() {
    let a = brep_core::cylinder(2., 3.).unwrap();
    let mut b = a.clone();
    for vertex in &mut b.vertices {
        vertex.point[0] += 2.;
    }
    for edge in &mut b.edges {
        for point in &mut edge.curve.control_points {
            point[0] += 2.;
        }
    }
    for face in &mut b.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            point[0] += 2.;
        }
    }
    b.validate().unwrap();
    let model = brep_core::boolean(&a, &b, "xor").unwrap();
    model.validate().unwrap();
    assert_eq!(model.bodies.len(), 2);
    let exact = (8. * std::f64::consts::PI / 3. + 4. * 3_f64.sqrt()) * 3.;
    for detail in [1, 2, 4, 8, 16] {
        let built =
            nurbs(&model, detail).unwrap_or_else(|error| panic!("XOR detail {detail}: {error}"));
        assert!(built.built.report.closed);
        assert_eq!(components(&built.built.mesh), 2);
        assert_eq!(built.built.report.orientation_conflicts, 0);
        assert_eq!(built.built.report.non_manifold_edges, 0);
        assert_eq!(built.built.report.degenerate_triangles, 0);
        if detail == 16 {
            assert!((built.built.report.signed_volume_mm3 - exact).abs() / exact < 0.003);
        }
    }
}
fn holed_curved_patch(touching: bool) -> brep_core::Model {
    let surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            j as f64 / 2.,
                            (if i == 2 { 1. } else { 0. }) + (if j == 2 { 1. } else { 0. }),
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let mut model = empty_model();
    let hole = if touching {
        vec![[0., 0.], [0., 0.5], [0.5, 0.5], [0.5, 0.]]
    } else {
        vec![[0.25, 0.25], [0.25, 0.75], [0.75, 0.75], [0.75, 0.25]]
    };
    for ring in [vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]], hole] {
        let start = model.vertices.len();
        for p in &ring {
            model.vertices.push(brep_core::Vertex {
                point: surface.evaluate(p[0], p[1]).unwrap().point,
            });
        }
        let mut coedges = Vec::new();
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let pa = surface.evaluate(a[0], a[1]).unwrap().point;
            let pb = surface.evaluate(b[0], b[1]).unwrap().point;
            let mid = surface
                .evaluate((a[0] + b[0]) / 2., (a[1] + b[1]) / 2.)
                .unwrap()
                .point;
            let control = (0..3)
                .map(|axis| 2. * mid[axis] - (pa[axis] + pb[axis]) / 2.)
                .collect();
            let curve = Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![pa.to_vec(), control, pb.to_vec()],
                weights: vec![1.; 3],
                periodic: false,
            };
            coedges.push(brep_core::Coedge {
                edge: model.edges.len(),
                reversed: false,
                pcurve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap(),
            });
            model.edges.push(brep_core::Edge {
                vertices: [start + i, start + (i + 1) % ring.len()],
                curve,
                degenerate: false,
            });
        }
        model.loops.push(brep_core::Loop { coedges });
    }
    model.faces.push(brep_core::Face {
        surface,
        outer: 0,
        holes: vec![1],
    });
    model.shells.push(brep_core::Shell {
        faces: vec![brep_core::FaceUse {
            face: 0,
            reversed: false,
        }],
        closed: false,
    });
    model.rebuild_topology_ids();
    if touching {
        let id = brep_core::TopoId::derive(
            brep_core::TopoKind::Vertex,
            "bridge-test",
            "separate-touching-vertex",
            "vertex",
            b"separate-touching-vertex",
        );
        model.1.vertices[4] = id;
        model
            .1
            .change_set
            .nodes
            .insert(id, brep_core::TopoKind::Vertex);
        model.1.change_set.changes.push(brep_core::TopologyChange {
            kind: brep_core::ChangeKind::Generated,
            topo_kind: brep_core::TopoKind::Vertex,
            parents: vec![],
            children: vec![id],
            provenance: brep_core::ChangeProvenance {
                operation: "bridge-test".into(),
                operand: None,
                occurrence: "separate-touching-vertex".into(),
            },
            role: "vertex".into(),
            anchor: None,
        });
    }
    model.validate().unwrap();
    model
}
#[test]
fn curved_trim_holes_preserve_registry_ownership_through_bridge_refinement() {
    let model = holed_curved_patch(false);
    for detail in [1, 2, 4, 8, 16] {
        let built =
            nurbs(&model, detail).unwrap_or_else(|error| panic!("detail {detail}: {error}"));
        assert!(!built.built.report.closed);
        assert_eq!(built.built.report.boundary_edges, 8 * detail);
        assert_eq!(built.built.report.non_manifold_edges, 0);
        assert_eq!(built.built.report.orientation_conflicts, 0);
        assert_eq!(built.built.report.degenerate_triangles, 0);
        assert_eq!(components(&built.built.mesh), 1);
    }
}
#[test]
fn coincident_uv_samples_from_different_topology_are_rejected() {
    let model = holed_curved_patch(true);
    let error = match nurbs(&model, 4) {
        Ok(_) => panic!("ambiguous topology accepted"),
        Err(error) => error,
    };
    assert!(error.message.contains("ambiguous shared UV"));
}
#[test]
fn empty_brep_returns_an_empty_mesh_without_claiming_a_closed_shell() {
    let model = empty_model();
    let built = nurbs(&model, 4).unwrap();
    assert!(built.built.mesh.positions.is_empty());
    assert!(built.built.mesh.indices.is_empty());
    assert!(!built.built.report.closed);
    assert_eq!(built.built.report.signed_volume_mm3, 0.);
    assert!(built.face_ids.is_empty());
    assert!(built.topology_face_ids.unwrap().is_empty());
}

#[test]
fn shared_edge_registry_gives_identical_indices_to_adjacent_faces() {
    let model = brep_core::cylinder(2., 4.).unwrap();
    let built = nurbs(&model, 8).unwrap();
    assert!(
        !built.built.report.closed || built.built.report.non_manifold_edges == 0,
        "shared-edge tess must remain manifold: {:?}",
        built.built.report
    );
    // Adjacent wall/cap triangles must share exact vertex indices along the
    // circular rim — not merely coincident coordinates.
    let positions = &built.built.mesh.positions;
    let indices = &built.built.mesh.indices;
    let mut edge_uses: std::collections::BTreeMap<(usize, usize), usize> =
        std::collections::BTreeMap::new();
    for tri in indices.as_chunks::<3>().0 {
        for [a, b] in [[tri[0], tri[1]], [tri[1], tri[2]], [tri[2], tri[0]]] {
            let key = if a < b { (a, b) } else { (b, a) };
            *edge_uses.entry(key).or_default() += 1;
        }
    }
    let shared = edge_uses.values().filter(|&&n| n == 2).count();
    let boundary = edge_uses.values().filter(|&&n| n == 1).count();
    assert!(shared > 0, "expected dual-face shared mesh edges");
    assert_eq!(
        boundary, 0,
        "closed cylinder tess must not leave singleton mesh edges"
    );
    // Spot-check: every shared edge endpoint has finite coords (no NaN weld).
    for &(a, b) in edge_uses.iter().filter(|(_, n)| **n == 2).map(|(k, _)| k) {
        for i in [a, b] {
            for c in 0..3 {
                assert!(positions[i * 3 + c].is_finite());
            }
        }
    }
    let _ = positions;
}
