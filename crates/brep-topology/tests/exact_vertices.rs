use brep_topology::*;
use cad_predicates::*;

#[test]
fn intersection_contour_retains_exact_vertices_in_indexed_topology() {
    let values = [
        0., 0., 0., 6., 0., 0., 3., 6., 0., 0., 4., 0., 6., 4., 0., 3., -2., 0.,
    ];
    let arena = SourceArena::authored(
        "topology-source",
        1,
        values
            .into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let input =
        |n: usize| Point3Input::Authored(std::array::from_fn(|i| arena.leaf(n * 3 + i).unwrap()));
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let vertices = match intersect_triangles3d(&mut ctx, [0, 1, 2].map(input), [3, 4, 5].map(input))
        .unwrap()
        .outcome
    {
        TriangleTriangleIntersection3::Polygon { vertices, .. } => vertices,
        other => panic!("{other:?}"),
    };
    let count = vertices.len();
    let mut model: Model<(), (), (), ConstructedPoint3> = Model {
        vertices: vertices.into_iter().map(|point| Vertex { point }).collect(),
        edges: (0..count)
            .map(|i| Edge {
                vertices: [i, (i + 1) % count],
                curve: (),
                degenerate: false,
            })
            .collect(),
        loops: vec![Loop {
            coedges: (0..count)
                .map(|edge| Coedge {
                    edge,
                    reversed: false,
                    pcurve: (),
                })
                .collect(),
        }],
        faces: vec![Face {
            surface: (),
            outer: 0,
            holes: vec![],
        }],
        shells: vec![Shell {
            faces: vec![FaceUse {
                face: 0,
                reversed: false,
            }],
            closed: false,
        }],
        bodies: vec![],
        tolerance_mm: 1e-6,
    };
    let validate = |point: &ConstructedPoint3| {
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        match classify_point_triangle3d(
            &mut ctx,
            Point3Input::Constructed(point),
            [0, 1, 2].map(input),
        )
        .map_err(|_| math_core::Error::new("EXACT_VERTEX_CONTEXT", "Foreign vertex context"))?
        .outcome
        {
            PointTriangleLocation3::OnPlane(
                TriangleLocation3::Interior
                | TriangleLocation3::Edge(_)
                | TriangleLocation3::Vertex(_),
            ) => Ok(()),
            _ => Err(math_core::Error::new(
                "EXACT_VERTEX_INVALID",
                "Invalid retained vertex",
            )),
        }
    };
    model.validate_topology_with_vertices(validate).unwrap();
    // The geometry owner chooses a portable payload explicitly. ConstructedPoint3
    // itself has no Serialize implementation that could round away its recipe.
    let roots: Vec<&ConstructedPoint3> = model.vertices.iter().map(|v| &v.point).collect();
    let mut save_ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let records = export_point3d_batch(&mut save_ctx, &roots)
        .unwrap()
        .outcome
        .unwrap();
    let wire: Model<(), (), (), Vec<u8>> = Model {
        vertices: records.into_iter().map(|point| Vertex { point }).collect(),
        edges: model.edges.clone(),
        loops: model.loops.clone(),
        faces: model.faces.clone(),
        shells: model.shells.clone(),
        bodies: model.bodies.clone(),
        tolerance_mm: model.tolerance_mm,
    };
    let json = value_codec::to_string(&wire).unwrap();
    let decoded: Model<(), (), (), Vec<u8>> = value_codec::from_str_strict(&json).unwrap();
    assert_eq!(json, value_codec::to_string(&decoded).unwrap());
    let records: Vec<&[u8]> = decoded
        .vertices
        .iter()
        .map(|v| v.point.as_slice())
        .collect();
    let mut load_ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let points = replay_point3d_batch(&mut load_ctx, &records)
        .unwrap()
        .outcome
        .unwrap();
    let restored = Model {
        vertices: points.into_iter().map(|point| Vertex { point }).collect(),
        edges: decoded.edges,
        loops: decoded.loops,
        faces: decoded.faces,
        shells: decoded.shells,
        bodies: decoded.bodies,
        tolerance_mm: decoded.tolerance_mm,
    };
    restored.validate_topology_with_vertices(validate).unwrap();
    assert_eq!(restored.vertices.len(), 6);
    for (original, loaded) in model.vertices.iter().zip(&restored.vertices) {
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        assert_eq!(
            compare_points3d(
                &mut ctx,
                Point3Input::Constructed(&original.point),
                Point3Input::Constructed(&loaded.point)
            )
            .unwrap()
            .outcome,
            Outcome::Sign(Sign::Zero)
        );
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        assert!(matches!(
            classify_point_triangle3d(
                &mut ctx,
                Point3Input::Constructed(&loaded.point),
                [3, 4, 5].map(input)
            )
            .unwrap()
            .outcome,
            PointTriangleLocation3::OnPlane(
                TriangleLocation3::Interior
                    | TriangleLocation3::Edge(_)
                    | TriangleLocation3::Vertex(_)
            )
        ));
    }
    let snapshot = TopologySnapshot::new(model.clone(), validate).unwrap();
    let sibling = snapshot.clone();
    let mut store = TopologyStore::new(snapshot.clone());
    let base = store.checkout();
    let stale = base
        .prepare(|m| {
            m.tolerance_mm = 3e-6;
            Ok(())
        })
        .unwrap();
    let accepted = base
        .prepare(|m| {
            m.tolerance_mm = 2e-6;
            Ok(())
        })
        .unwrap();
    assert_eq!(store.commit(accepted).unwrap(), 1);
    assert_eq!(
        store.commit(stale).unwrap_err().code,
        "BREP_STALE_TRANSACTION"
    );
    assert_eq!(store.snapshot().model().tolerance_mm, 2e-6);
    let aba = base.prepare(|_| Ok(())).unwrap();
    let restore = store
        .checkout()
        .prepare(|m| {
            m.tolerance_mm = 1e-6;
            Ok(())
        })
        .unwrap();
    assert_eq!(store.commit(restore).unwrap(), 2);
    assert_eq!(
        store.commit(aba).unwrap_err().code,
        "BREP_STALE_TRANSACTION"
    );
    let foreign_store = TopologyStore::new(snapshot.clone());
    let foreign = foreign_store.checkout().prepare(|_| Ok(())).unwrap();
    assert_eq!(
        store.commit(foreign).unwrap_err().code,
        "BREP_FOREIGN_TRANSACTION"
    );
    assert!(
        store
            .checkout()
            .prepare(|m| {
                m.vertices.pop();
                Ok(())
            })
            .is_err()
    );
    assert_eq!(store.revision(), 2);
    assert_eq!(base.revision(), 0);
    let before_undo = store.checkout();
    assert!(store.undo().unwrap());
    assert_eq!(store.snapshot().model().tolerance_mm, 2e-6);
    assert!(store.redo().unwrap());
    assert_eq!(store.snapshot().model().tolerance_mm, 1e-6);
    assert_eq!(
        store
            .commit(before_undo.prepare(|_| Ok(())).unwrap())
            .unwrap_err()
            .code,
        "BREP_STALE_TRANSACTION"
    );
    store
        .snapshot()
        .model()
        .validate_topology_with_vertices(validate)
        .unwrap();
    assert!(
        store
            .snapshot()
            .model()
            .vertices
            .iter()
            .all(|v| v.point.recipe_depth() > 0)
    );

    base.snapshot()
        .model()
        .validate_topology_with_vertices(validate)
        .unwrap();

    let edited = snapshot
        .try_edit(|candidate| {
            candidate.tolerance_mm = 2e-6;
            Ok(())
        })
        .unwrap();
    assert_eq!(snapshot.model().tolerance_mm, 1e-6);
    assert_eq!(sibling.model().tolerance_mm, 1e-6);
    assert_eq!(edited.model().tolerance_mm, 2e-6);
    assert!(
        snapshot
            .try_edit(|candidate| {
                candidate.vertices.pop();
                Ok(())
            })
            .is_err()
    );
    assert!(
        snapshot
            .try_edit(|candidate| {
                candidate.tolerance_mm = 1e-4;
                Err(math_core::Error::new(
                    "EDIT_FAILED",
                    "Deliberate interruption",
                ))
            })
            .is_err()
    );
    assert_eq!(snapshot.model().vertices.len(), count);
    assert_eq!(snapshot.model().tolerance_mm, 1e-6);
    let foreign_arena = SourceArena::authored(
        "foreign",
        1,
        values
            .into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let foreign_input = |n: usize| {
        Point3Input::Authored(std::array::from_fn(|i| {
            foreign_arena.leaf(n * 3 + i).unwrap()
        }))
    };
    let mut foreign_ctx =
        PredicateContext::new(&foreign_arena, &tolerance, Limits::default(), None);
    let TriangleTriangleIntersection3::Polygon { mut vertices, .. } = intersect_triangles3d(
        &mut foreign_ctx,
        [0, 1, 2].map(foreign_input),
        [3, 4, 5].map(foreign_input),
    )
    .unwrap()
    .outcome
    else {
        panic!("Expected foreign polygon");
    };
    let foreign = vertices.pop().unwrap();
    assert!(
        snapshot
            .try_edit(|candidate| {
                candidate.vertices[0].point = foreign;
                Ok(())
            })
            .is_err()
    );
    snapshot
        .model()
        .validate_topology_with_vertices(validate)
        .unwrap();
    drop(snapshot);
    sibling
        .model()
        .validate_topology_with_vertices(validate)
        .unwrap();
    let mut check = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    assert_eq!(
        compare_points3d(
            &mut check,
            Point3Input::Constructed(&sibling.model().vertices[0].point),
            Point3Input::Constructed(&edited.model().vertices[0].point)
        )
        .unwrap()
        .outcome,
        Outcome::Sign(Sign::Zero)
    );

    let cloned = model.clone();
    drop(model.vertices.pop());
    assert!(model.validate_topology_with_vertices(validate).is_err());
    cloned.validate_topology_with_vertices(validate).unwrap();
    assert!(cloned.vertices.iter().all(|v| v.point.recipe_depth() > 0));
    assert!(
        cloned
            .validate_topology_with_vertices(|_| Err(math_core::Error::new(
                "VERTEX_REJECTED",
                "Rejected by owning context"
            )))
            .is_err()
    );
}
