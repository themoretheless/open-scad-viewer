#[path = "../examples/support/curved_trim_fixture.rs"]
mod fixture;
use brep_core::{edge_partition, face_cut, face_domain::FaceDomain, watertight};
#[test]
fn sections_are_clipped_to_owned_side_face_roots_before_lifting() {
    let source = fixture::build();
    let before = source.clone();
    let cutter = brep_core::cuboid([-1., -1., 0.5], [2., 2., 0.75]).unwrap();
    let plane = 0;
    for (face, x, minimum_y) in [(3, 1., 0.25), (5, 0., 0.5)] {
        assert!(
            face_cut::generate_ruled_plane_cut(
                [&source.faces[face].surface, &cutter.faces[plane].surface],
                None,
                source.tolerance_mm,
                100000
            )
            .is_err()
        );
        let result =
            face_cut::partition_ruled_at_face(&source, face, &cutter, plane, 1e-8, 100000, 1000000)
                .unwrap();
        assert!(result.cells <= 100000);
        let model = &result.partition.model;
        let mut y = result.endpoints.map(|v| {
            let p = model.vertices[v].point;
            assert!((p[0] - x).abs() < 1e-10 && (p[2] - 0.5).abs() < 1e-7);
            p[1]
        });
        y.sort_by(f64::total_cmp);
        assert!((y[0] - minimum_y).abs() < 1e-7 && (y[1] - 1.).abs() < 1e-10);
        assert_eq!(
            watertight::inspect(model, 100000).unwrap().status,
            watertight::Closure::WithinTolerance
        );
    }
    assert_eq!(source, before);
}
#[test]
fn generated_nonlinear_section_splits_a_closed_body_with_shared_root_ends() {
    let source = fixture::build();
    let before = source.clone();
    assert_eq!(
        watertight::inspect(&source, 100000).unwrap().status,
        watertight::Closure::WithinTolerance
    );
    let top = 1;
    let cutter = brep_core::cuboid([-1., -1., 0.5], [2., 2., 0.75]).unwrap();
    let plane = cutter
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 0.5)
        })
        .unwrap();
    let automatic =
        face_cut::partition_ruled_at_face(&source, top, &cutter, plane, 1e-8, 100000, 1000000)
            .unwrap();
    assert_eq!(automatic.boundary_coverage.len(), 4);
    assert!(automatic.boundary_coverage.iter().all(|(_, r)| r.complete));
    assert!(automatic.cells <= 100000);
    assert_eq!(automatic.original_face_identity, source.1.faces[top]);
    assert!(automatic.partition.model.persistent_naming_complete());
    #[cfg(feature = "codec")]
    {
        let restored: brep_core::Model =
            value_codec::from_str(&value_codec::to_string(&automatic.partition.model).unwrap())
                .unwrap();
        assert_eq!(restored, automatic.partition.model);
        assert_eq!(
            watertight::inspect(&restored, 100000).unwrap().status,
            watertight::Closure::WithinTolerance
        );
    }
    assert_eq!(
        watertight::inspect(&automatic.partition.model, 100000)
            .unwrap()
            .status,
        watertight::Closure::WithinTolerance
    );
    assert!(
        face_cut::partition_ruled_at_face(
            &source,
            top,
            &cutter,
            plane,
            1e-8,
            automatic.cells - 1,
            1000000
        )
        .is_err()
    );
    let without_exact =
        face_cut::partition_ruled_at_face(&source, top, &cutter, plane, 1e-8, 100000, 0).unwrap();
    assert_eq!(without_exact.partition.exact_work, 0);
    let generated = face_cut::generate_ruled_plane_cut(
        [&source.faces[top].surface, &cutter.faces[plane].surface],
        None,
        source.tolerance_mm,
        100000,
    )
    .unwrap();
    let mut holed = cutter.clone();
    let center = generated.pcurves[1].evaluate(0.5).unwrap().point;
    let h = 1. / 32.;
    let points = [
        [center[0] - h, center[1] - h],
        [center[0] - h, center[1] + h],
        [center[0] + h, center[1] + h],
        [center[0] + h, center[1] - h],
    ];
    let base = holed.vertices.len();
    for uv in points {
        let point = holed.faces[plane]
            .surface
            .evaluate(uv[0], uv[1])
            .unwrap()
            .point
            .try_into()
            .unwrap();
        holed.vertices.push(brep_core::Vertex { point });
    }
    let mut coedges = Vec::new();
    for i in 0..4 {
        let next = (i + 1) % 4;
        let edge = holed.edges.len();
        let curve = nurbs_core::curve::Curve::from_polyline(vec![
            holed.vertices[base + i].point.to_vec(),
            holed.vertices[base + next].point.to_vec(),
        ])
        .unwrap();
        holed.edges.push(brep_core::Edge {
            curve,
            vertices: [base + i, base + next],
            degenerate: false,
        });
        coedges.push(brep_core::Coedge {
            edge,
            reversed: false,
            pcurve: nurbs_core::curve::Curve::from_polyline(vec![
                points[i].to_vec(),
                points[next].to_vec(),
            ])
            .unwrap(),
        });
    }
    let wire = holed.loops.len();
    holed.loops.push(brep_core::Loop { coedges });
    holed.faces[plane].holes.push(wire);
    // The cutter is an authored open shell with a holed face, not a solid.
    holed.bodies.clear();
    holed.shells[0].closed = false;
    holed.rebuild_topology_ids();
    holed.validate().unwrap();
    let failure =
        face_cut::partition_ruled_at_face(&source, top, &holed, plane, 1e-8, 100000, 1000000)
            .err()
            .unwrap();
    assert!(
        failure.message.contains("actual cutter trim"),
        "{}",
        failure.message
    );
    let edges = [0., 1.].map(|x| {
        source.loops[source.faces[top].outer]
            .coedges
            .iter()
            .find(|c| {
                source.edges[c.edge]
                    .vertices
                    .iter()
                    .all(|&v| source.vertices[v].point[0] == x)
            })
            .unwrap()
            .edge
    });
    let mut model = source.clone();
    let mut vertices = [0; 2];
    let mut cells = generated.cells;
    for (i, edge) in edges.into_iter().enumerate() {
        let split = edge_partition::partition_all_at_face(
            &model,
            edge,
            &cutter,
            plane,
            1e-8,
            100000 - cells,
        )
        .unwrap();
        assert!(split.coverage.complete);
        assert_eq!(split.vertices.len(), 1);
        vertices[i] = split.vertices[0];
        cells += split.cells;
        model = split.model;
    }
    let bound = face_cut::bind_intersection_ends(
        &model,
        top,
        vertices,
        &generated,
        &cutter.faces[plane].surface,
        100000 - cells,
    )
    .unwrap();
    let candidate_before = (generated.curve.clone(), generated.pcurves.clone());
    assert!(
        face_cut::bind_intersection_ends(
            &model,
            top,
            [4, vertices[1]],
            &generated,
            &cutter.faces[plane].surface,
            100000
        )
        .is_err()
    );
    assert_eq!(
        (generated.curve.clone(), generated.pcurves.clone()),
        candidate_before
    );
    cells += bound.cells;
    let split = face_cut::partition(
        &model,
        top,
        vertices,
        &[face_cut::Segment {
            curve: bound.curve,
            pcurve: bound.pcurves[0].clone(),
        }],
        1e-8,
        100000 - cells,
    )
    .unwrap();
    cells += split.cells;
    assert!(cells <= 100000);
    for v in vertices {
        let incident = split
            .model
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.vertices.contains(&v))
            .collect::<Vec<_>>();
        assert_eq!(incident.len(), 3);
        for (edge, _) in incident {
            assert_eq!(
                split
                    .model
                    .loops
                    .iter()
                    .flat_map(|l| &l.coedges)
                    .filter(|c| c.edge == edge)
                    .count(),
                2
            );
        }
    }
    let closure = watertight::inspect(&split.model, 100000).unwrap();
    assert_eq!(closure.status, watertight::Closure::WithinTolerance);
    for seed in [[0.5, 0.1], [0.5, 0.8]] {
        let mut inside = 0;
        for f in split.faces {
            let r = FaceDomain::new(&split.model, f, 1e-8)
                .unwrap()
                .classify(seed.map(|x| [x, x]), 100000)
                .unwrap();
            if r.location == nurbs_core::trim_domain::Location::Inside {
                inside += 1;
            }
        }
        assert_eq!(inside, 1);
    }
    assert_eq!(source, before);
    #[cfg(feature = "codec")]
    {
        let reloaded: brep_core::Model =
            value_codec::from_str(&value_codec::to_string(&split.model).unwrap()).unwrap();
        assert_eq!(reloaded, split.model);
        assert_eq!(
            watertight::inspect(&reloaded, 100000).unwrap().status,
            watertight::Closure::WithinTolerance
        );
    }
}
