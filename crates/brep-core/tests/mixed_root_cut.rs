use brep_core::{closed_solid as solid, mixed_root_model as mixed};
#[path = "../../nurbs-core/tests/support/ss_root_circle.rs"]
mod fixture;
#[path = "support/closed_solid_policy.rs"]
mod policy;
fn body(model: brep_core::Model) -> solid::QualifiedClosedSolid {
    solid::qualify(model, policy::policy())
        .unwrap()
        .into_solid()
        .unwrap()
}
#[test]
fn finite_face_cut_keeps_original_rims_and_exact_shared_intersection_edges() {
    let original = brep_core::cuboid([0., 0., -1.], [1., 1., 0.]).unwrap();
    let before = original.clone();
    let source = body(original);
    let wire = fixture::wire();
    let cut =
        mixed::split_closed_contour_candidate(&source, 1, &wire, 0, [0.5; 2], Default::default())
            .unwrap();
    assert!(!cut.geometry_admitted() && !cut.material_classified());
    cut.reverify().unwrap();
    let topology = cut.model().topology();
    assert_eq!(topology.faces.len(), 7);
    assert_eq!(topology.edges.len(), 20);
    assert_eq!(topology.vertices.len(), 16);
    assert_eq!(topology.faces[1].outer, before.faces[1].outer);
    assert_eq!(topology.faces[1].holes.len(), 1);
    assert!(topology.faces[cut.inside_face()].holes.is_empty());
    assert_eq!(topology.euler_characteristic(), 2);
    assert!(topology.boundary_edges().is_empty());
    for i in 0..12 {
        assert!(
            matches!(&topology.edges[i].curve,mixed::MixedEdge::Rational(c) if c==&before.edges[i].curve)
        );
    }
    for e in &topology.edges[12..] {
        assert!(matches!(e.curve, mixed::MixedEdge::Root(_)));
    }
    assert_eq!(source.model(), &before);
    assert!(
        mixed::split_closed_contour_candidate(&source, 0, &wire, 0, [0.5; 2], Default::default())
            .is_err()
    );
    assert!(
        mixed::split_closed_contour_candidate(
            &source,
            1,
            &wire,
            0,
            [0.5; 2],
            mixed::ClosedCutLimits {
                placement_queries: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn finite_holes_are_transferred_to_their_true_side_of_the_implicit_cut() {
    let profile = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let holes = vec![
        vec![
            [0.4375, 0.4375],
            [0.4375, 0.5625],
            [0.5625, 0.5625],
            [0.5625, 0.4375],
        ],
        vec![
            [0.03125, 0.03125],
            [0.03125, 0.09375],
            [0.09375, 0.09375],
            [0.09375, 0.03125],
        ],
    ];
    let original = brep_core::extrude_polygon_with_holes(&profile, &holes, -1., 0.).unwrap();
    let face = original
        .faces
        .iter()
        .position(|f| {
            f.holes.len() == 2
                && f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[2] == 0.)
        })
        .unwrap();
    let wire = fixture::wire_on(original.faces[face].surface.clone());
    let old_holes = original.faces[face].holes.clone();
    let source = body(original);
    let cut = mixed::split_closed_contour_candidate(
        &source,
        face,
        &wire,
        0,
        [0.5, 0.6875],
        Default::default(),
    )
    .unwrap();
    let t = cut.model().topology();
    assert_eq!(cut.inside_inherited_holes().len(), 1);
    assert!(old_holes.contains(&cut.inside_inherited_holes()[0]));
    assert_eq!(t.faces[face].holes.len(), 2);
    assert_eq!(t.faces[cut.inside_face()].holes.len(), 1);
    let used: Vec<_> = t
        .faces
        .iter()
        .flat_map(|f| std::iter::once(f.outer).chain(f.holes.iter().copied()))
        .collect();
    for h in old_holes {
        assert_eq!(used.iter().filter(|&&i| i == h).count(), 1);
    }
    assert_eq!(
        t.euler_characteristic(),
        source.model().euler_characteristic()
    );
    cut.reverify().unwrap();
}
#[test]
fn contour_crossing_a_finite_rim_or_existing_hole_cannot_publish_a_split() {
    let profiles = vec![
        (
            vec![
                [0., 0.],
                [1., 0.],
                [1., 0.375],
                [0.375, 0.375],
                [0.375, 1.],
                [0., 1.],
            ],
            vec![],
        ),
        (
            vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
            vec![vec![[0.70, 0.45], [0.70, 0.55], [0.90, 0.55], [0.90, 0.45]]],
        ),
    ];
    for (outer, holes) in profiles {
        let original = brep_core::extrude_polygon_with_holes(&outer, &holes, -1., 0.).unwrap();
        let face = original
            .faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[2] == 0.)
            })
            .unwrap();
        let wire = fixture::wire_on(original.faces[face].surface.clone());
        let before = original.clone();
        let source = body(original);
        assert!(
            mixed::split_closed_contour_candidate(
                &source,
                face,
                &wire,
                0,
                [0.5; 2],
                Default::default()
            )
            .is_err()
        );
        assert_eq!(source.model(), &before);
    }
}
