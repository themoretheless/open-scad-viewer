use super::*;
use crate::ss_face_membership::tests::{curves as midpoint_curves, model};
fn curves(r2: f64) -> ss::QualifiedGraphCurves {
    let c = midpoint_curves(r2);
    c.original()
        .construct_graph_curves_with_dyadic_proposals(
            ss::GraphCurveTolerances {
                world: 1e-7,
                uv: 1e-9,
            },
            Default::default(),
        )
        .unwrap()
}
use nurbs_core::ss_intersection as ss;
#[test]
fn complete_actual_sectors_get_opposite_height_roles_with_holes_and_swapped_sources() {
    for swapped in [false, true] {
        for hole in [
            None,
            Some([[0.4375, 0.5625], [0.4375, 0.5625]]),
            Some([[0.8125, 0.9375], [0.125, 0.25]]),
        ] {
            let original_curves = curves(0.0625);
            let original = original_curves.original();
            let source = if swapped {
                ss::FiberSources::new(original.source().second(), original.source().first())
            } else {
                ss::FiberSources::new(original.source().first(), original.source().second())
            }
            .unwrap();
            let original = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
            let curves = original
                .construct_graph_curves_with_dyadic_proposals(
                    ss::GraphCurveTolerances {
                        world: 1e-7,
                        uv: 1e-9,
                    },
                    Default::default(),
                )
                .unwrap();
            let m = [
                model(original.source().first(), hole),
                model(original.source().second(), None),
            ];
            let before = m.clone();
            let member =
                qualify(&curves, [(&m[0], 0), (&m[1], 0)], 1e-8, Default::default()).unwrap();
            let closed = partition_closed_branches(&member, Default::default()).unwrap();
            let sides = qualify_closed_graph_sides(&closed, Default::default()).unwrap();
            assert_eq!(sides.faces().len(), 4);
            assert_eq!(sides.faces().iter().filter(|f| f.above).count(), 2);
            for f in sides.faces() {
                assert_eq!(f.above, (!swapped) ^ f.inner ^ (f.side == 1));
                assert!(closed.models()[f.model].faces.get(f.face).is_some());
            }
            sides.reverify().unwrap();
            assert_eq!(m, before);
        }
    }
}
#[test]
fn bounded_approximate_zero_contours_do_not_get_exact_height_roles() {
    let curves = midpoint_curves(0.03125); // irrational cardinal radius, rounded saved poles
    let o = curves.original();
    let m = [
        model(o.source().first(), None),
        model(o.source().second(), None),
    ];
    let before = m.clone();
    let member = qualify(&curves, [(&m[0], 0), (&m[1], 0)], 1e-8, Default::default()).unwrap();
    let closed = partition_closed_branches(&member, Default::default()).unwrap();
    assert_eq!(closed.regions().len(), 2);
    assert!(qualify_closed_graph_sides(&closed, Default::default()).is_err());
    closed.reverify().unwrap();
    assert_eq!(m, before);
}
#[test]
fn exact_shared_work_and_sealed_actual_role_replay_are_mandatory() {
    let curves = curves(0.0625);
    let o = curves.original();
    let m = [
        model(o.source().first(), None),
        model(o.source().second(), None),
    ];
    let member = qualify(&curves, [(&m[0], 0), (&m[1], 0)], 1e-8, Default::default()).unwrap();
    let closed = partition_closed_branches(&member, Default::default()).unwrap();
    let mut sides = qualify_closed_graph_sides(&closed, Default::default()).unwrap();
    let exact = ClosedGraphSideLimits {
        operations: sides.operations(),
    };
    assert!(qualify_closed_graph_sides(&closed, exact).is_ok());
    assert!(
        qualify_closed_graph_sides(
            &closed,
            ClosedGraphSideLimits {
                operations: exact.operations - 1
            }
        )
        .is_err()
    );
    sides.faces[0].above = !sides.faces[0].above;
    assert!(sides.reverify().is_err());
    let mut snapshot = sides.faces().to_vec();
    snapshot.clear();
    assert_eq!(sides.faces().len(), 4);
}
#[test]
fn excluded_branch_cannot_publish_a_whole_face_role_from_empty_geometry() {
    let curves = curves(0.0625);
    let o = curves.original();
    let m = [
        model(o.source().first(), None),
        model(o.source().second(), Some([[0.125, 0.875], [0.125, 0.875]])),
    ];
    let member = qualify(&curves, [(&m[0], 0), (&m[1], 0)], 1e-8, Default::default()).unwrap();
    let closed = partition_closed_branches(&member, Default::default()).unwrap();
    assert!(closed.regions().is_empty());
    assert!(qualify_closed_graph_sides(&closed, Default::default()).is_err());
}
