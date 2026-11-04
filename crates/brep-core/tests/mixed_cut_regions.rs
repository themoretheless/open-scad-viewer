use brep_core::{closed_solid as solid, mixed_root_model as mixed};
use nurbs_core::{ss_intersection as ss, trim_domain::Location};
#[path = "../../nurbs-core/tests/support/ss_root_circle.rs"]
mod fixture;
#[path = "support/closed_solid_policy.rs"]
mod policy;
fn source() -> solid::QualifiedClosedSolid {
    solid::qualify(
        brep_core::cuboid([0., 0., -1.], [1., 1., 0.]).unwrap(),
        policy::policy(),
    )
    .unwrap()
    .into_solid()
    .unwrap()
}
#[test]
fn whole_original_boxes_belong_to_exactly_the_proved_finite_piece() {
    let source = source();
    let wire = fixture::wire();
    let cut =
        mixed::split_closed_contour_candidate(&source, 1, &wire, 0, [0.5; 2], Default::default())
            .unwrap();
    let inner = cut
        .classify_rectangle([[0.45, 0.55]; 2], Default::default())
        .unwrap();
    assert_eq!(inner.source_location(), Location::Inside);
    assert_eq!(inner.inside_piece(), Location::Inside);
    assert_eq!(inner.outside_piece(), Location::Outside);
    assert_eq!(inner.winding(), Some(1));
    inner.reverify().unwrap();
    let outer = cut
        .classify_rectangle([[0.025, 0.10]; 2], Default::default())
        .unwrap();
    assert_eq!(outer.inside_piece(), Location::Outside);
    assert_eq!(outer.outside_piece(), Location::Inside);
    outer.reverify().unwrap();
    let boundary = cut
        .classify_rectangle(
            [[0.49, 0.51], [0.80, 0.83]],
            mixed::CutRectangleLimits {
                winding: ss::OriginalSsWindingLimits {
                    max_segments: 8,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(boundary.inside_piece(), Location::Unresolved);
    assert_eq!(boundary.outside_piece(), Location::Unresolved);
    assert!(boundary.root_refusal().is_some());
    assert_eq!(boundary.root_segments(), None);
    boundary.reverify().unwrap();
    assert!(
        cut.classify_rectangle([[-0.1, 0.1], [0.4, 0.5]], Default::default())
            .is_err()
    );
    assert!(
        cut.classify_rectangle(
            [[0.4, 0.5]; 2],
            mixed::CutRectangleLimits {
                domain_cells: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn inherited_hole_is_outside_both_pieces_and_winding_reversal_preserves_membership() {
    let outer = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let hole = vec![
        [0.4375, 0.4375],
        [0.4375, 0.5625],
        [0.5625, 0.5625],
        [0.5625, 0.4375],
    ];
    let model = brep_core::extrude_polygon_with_holes(&outer, &[hole], -1., 0.).unwrap();
    let face = model
        .faces
        .iter()
        .position(|f| {
            !f.holes.is_empty()
                && f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[2] == 0.)
        })
        .unwrap();
    let wire = fixture::wire_on(model.faces[face].surface.clone());
    let body = solid::qualify(model, policy::policy())
        .unwrap()
        .into_solid()
        .unwrap();
    let reversed = ss::OriginalSsRootWire::new(
        wire.edges()
            .iter()
            .rev()
            .map(|e| ss::OriginalSsRootEdge::new(e.end().clone(), e.start().clone()).unwrap())
            .collect(),
    )
    .unwrap();
    for contour in [&wire, &reversed] {
        let cut = mixed::split_closed_contour_candidate(
            &body,
            face,
            contour,
            0,
            [0.5, 0.6875],
            Default::default(),
        )
        .unwrap();
        let hole = cut
            .classify_rectangle([[0.49, 0.51]; 2], Default::default())
            .unwrap();
        assert_eq!(hole.source_location(), Location::Outside);
        assert_eq!(hole.inside_piece(), Location::Outside);
        assert_eq!(hole.outside_piece(), Location::Outside);
        assert_eq!(hole.root_segments(), Some(0));
        hole.reverify().unwrap();
        let material = cut
            .classify_rectangle([[0.48, 0.52], [0.66, 0.70]], Default::default())
            .unwrap();
        assert_eq!(material.inside_piece(), Location::Inside);
        assert_eq!(material.outside_piece(), Location::Outside);
    }
}
#[path = "support/quadratic_graph_body.rs"]
mod graph;
#[test]
fn actual_intersecting_nurbs_bodies_select_material_on_both_finite_cut_pieces() {
    use solid::{BooleanMaterialOp as Op, BooleanSourceSide as Side};
    let a = source();
    let wire = fixture::wire();
    let b = graph::bowl();
    assert_eq!(
        &b.faces[1].surface,
        wire.edges()[0].curve().source().second()
    );
    let mut curved_policy = policy::policy();
    curved_policy.geometry.boundary.contacts.cells_per_pair = 256;
    curved_policy
        .geometry
        .boundary
        .contacts
        .domain_cells_per_pair = 10000;
    let admission = solid::qualify(b, curved_policy).unwrap();
    assert!(
        admission.solid().is_some(),
        "Original curved input lacks complete continuous material admission"
    );
    let b = admission.into_solid().unwrap();
    let cut = mixed::split_closed_contour_candidate(&a, 1, &wire, 0, [0.5; 2], Default::default())
        .unwrap();
    let directions = [[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]];
    for (operation, inside_keep, outside_keep, outside_reverse) in [
        (Op::Union, true, false, false),
        (Op::Intersection, false, true, false),
        (Op::Difference, true, false, false),
        (Op::Xor, true, true, true),
    ] {
        let inside = cut
            .qualify_boolean_rectangle(
                &b,
                operation,
                Side::First,
                [[0.48, 0.52], [0.66, 0.70]],
                &directions,
                Default::default(),
            )
            .unwrap();
        assert_eq!(inside.piece_face(), cut.inside_face());
        assert_eq!(inside.retain(), inside_keep);
        assert!(!inside.reverse_material_normal());
        let outside = cut
            .qualify_boolean_rectangle(
                &b,
                operation,
                Side::First,
                [[0.035, 0.065]; 2],
                &directions,
                Default::default(),
            )
            .unwrap();
        assert_eq!(outside.piece_face(), cut.source_face());
        assert_eq!(outside.retain(), outside_keep);
        assert_eq!(outside.reverse_material_normal(), outside_reverse);
    }
    let verified = cut
        .qualify_boolean_rectangle(
            &b,
            Op::Union,
            Side::First,
            [[0.48, 0.52], [0.66, 0.70]],
            &directions,
            Default::default(),
        )
        .unwrap();
    verified.reverify().unwrap();
    let boundary_limits = mixed::CutMaterialLimits {
        region: mixed::CutRectangleLimits {
            winding: ss::OriginalSsWindingLimits {
                max_segments: 8,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(
        cut.qualify_boolean_rectangle(
            &b,
            Op::Union,
            Side::First,
            [[0.49, 0.51], [0.80, 0.83]],
            &directions,
            boundary_limits
        )
        .is_err()
    );
    let second_cut =
        mixed::split_closed_contour_candidate(&b, 1, &wire, 1, [0.5; 2], Default::default())
            .unwrap();
    let subtraction = second_cut
        .qualify_boolean_rectangle(
            &a,
            Op::Difference,
            Side::Second,
            [[0.48, 0.52], [0.66, 0.70]],
            &directions,
            Default::default(),
        )
        .unwrap();
    assert_eq!(subtraction.piece_face(), second_cut.inside_face());
    assert!(subtraction.retain() && subtraction.reverse_material_normal());
    subtraction.reverify().unwrap();
    assert!(!cut.material_classified() && !cut.geometry_admitted());
}
