use brep_core::{
    face_cut::{self, RootEnd, RootedCut, RootedFace},
    watertight,
};

#[test]
fn cutter_roots_bind_face_cut_ends_to_shared_edge_vertices() {
    let model = brep_core::cuboid([0.; 3], [8.; 3]).unwrap();
    let cutter = brep_core::cuboid([-1., 4., -1.], [9., 5., 9.]).unwrap();
    let before = model.clone();
    let cutter_before = cutter.clone();
    let plane = cutter
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[1] == 4.)
        })
        .unwrap();
    let top = model
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 8.)
        })
        .unwrap();
    let side_edge = |x: f64| {
        model
            .edges
            .iter()
            .position(|e| {
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                a[0] == x && b[0] == x && a[2] == 8. && b[2] == 8. && a[1] != b[1]
            })
            .unwrap()
    };
    let supports = [&model.faces[top].surface, &cutter.faces[plane].surface];
    // Keep the existing strict SS coverage refusal for nonbinary parameter maps.
    let coverage = nurbs_core::ss_intersection::intersect_surface_surface_report(
        supports[0],
        supports[1],
        None,
    )
    .unwrap();
    assert!(!coverage.complete() && !coverage.topology_proven);
    let generated =
        face_cut::generate_intersection_curve(supports, None, model.tolerance_mm, 100000).unwrap();
    assert!(
        generated
            .agreements
            .iter()
            .all(|r| r.status == nurbs_core::curve_surface_agreement::Status::WithinTolerance)
    );
    assert!(generated.cells <= 100000);
    let generation_cells = generated.cells;
    let ends = [
        generated.curve.control_points.first().unwrap()[0],
        generated.curve.control_points.last().unwrap()[0],
    ];
    let edges = ends.map(side_edge);
    assert!(
        face_cut::generate_intersection_curve(
            supports,
            Some([[0., 4., 8.01], [8., 4., 8.]]),
            model.tolerance_mm,
            100000
        )
        .is_err()
    );
    assert!(face_cut::generate_intersection_curve(supports, None, model.tolerance_mm, 3).is_err());
    let mut bent = supports[0].clone();
    bent.control_points[0][0][2] += 0.1;
    assert!(
        face_cut::generate_intersection_curve(
            [&bent, supports[1]],
            None,
            model.tolerance_mm,
            100000
        )
        .is_err()
    );
    assert!(
        face_cut::generate_intersection_curve(
            [supports[0], supports[0]],
            None,
            model.tolerance_mm,
            100000
        )
        .is_err()
    );
    let cuts = [RootedCut {
        endpoints: edges.map(|edge| RootEnd::Crossing { edge, root: 0 }),
        curves: vec![generated.curve],
    }];
    let seeds = [[0.2, 0.3]];
    let requests = [RootedFace {
        face: top,
        cuts: &cuts,
        keep_uv: &seeds,
    }];
    let result = face_cut::plan_rooted_retention(
        &model,
        &cutter,
        plane,
        &requests,
        1e-8,
        100000 - generation_cells,
        1000000,
        100000,
    )
    .unwrap();
    assert_eq!(result.roots.len(), 2);
    assert_eq!(result.root_vertices.len(), 2);
    assert!(
        result
            .roots
            .iter()
            .all(|(_, r)| r.complete && r.source_is_owned)
    );
    assert_eq!(result.plan.regions[0].retained_faces.len(), 1);
    assert_eq!(result.plan.regions[0].discard_faces.len(), 1);
    assert!(result.cells <= 100000 && result.cells > result.plan.cells);
    assert!(result.cells + generation_cells <= 100000);
    let saved = &result.plan.model;
    for &(edge, _, vertex) in &result.root_vertices {
        assert_eq!(saved.vertices[vertex].point[1], 4.);
        // Two child definitions meet at the same vertex; each is used by both faces.
        let children = saved
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.vertices.contains(&vertex))
            .collect::<Vec<_>>();
        assert_eq!(children.len(), 3); // two original children and the new transverse cut
        assert!(
            saved
                .1
                .lineage
                .iter()
                .any(|l| l.parents == [model.1.edges[edge]])
        );
        for (id, _) in children {
            assert_eq!(
                saved
                    .loops
                    .iter()
                    .flat_map(|l| &l.coedges)
                    .filter(|c| c.edge == id)
                    .count(),
                2
            );
        }
    }
    assert_eq!(
        watertight::inspect(saved, 100000).unwrap().status,
        watertight::Closure::WithinTolerance
    );
    let bad = [RootedCut {
        endpoints: [
            RootEnd::Crossing {
                edge: edges[0],
                root: 1,
            },
            RootEnd::Crossing {
                edge: edges[1],
                root: 0,
            },
        ],
        curves: cuts[0].curves.clone(),
    }];
    assert!(
        face_cut::plan_rooted_retention(
            &model,
            &cutter,
            plane,
            &[RootedFace {
                face: top,
                cuts: &bad,
                keep_uv: &seeds
            }],
            1e-8,
            100000,
            1000000,
            100000
        )
        .is_err()
    );
    assert!(
        face_cut::plan_rooted_retention(
            &model,
            &cutter,
            plane,
            &requests,
            1e-8,
            result.cells - 1,
            1000000,
            100000
        )
        .is_err()
    );
    assert_eq!(model, before);
    assert_eq!(cutter, cutter_before);
}
