use nurbs_core::{ss_intersection as ss, surface::Surface};
fn source(center: f64) -> ss::FiberSources {
    let a = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![3., 0., 0.], vec![3., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let mut b = a.clone();
    b.degree_u = 2;
    b.degree_v = 2;
    b.knots_u = vec![0., 0., 0., 1., 2., 3., 3., 3.];
    b.knots_v = vec![0., 0., 0., 1., 1., 1.];
    let x = [0., 0.5, 1.5, 2.5, 3.];
    let v = [0.25, -0.25, 0.25];
    b.control_points = (0..5)
        .map(|i| {
            (0..3)
                .map(|j| {
                    vec![
                        x[i],
                        j as f64 / 2.,
                        (b.knots_u[i + 1] - center) * (b.knots_u[i + 2] - center) + v[j] - 0.0625,
                    ]
                })
                .collect()
        })
        .collect();
    b.weights = vec![vec![3.; 3]; 5];
    ss::FiberSources::new(&a, &b).unwrap()
}
fn limits() -> ss::ClosedSSBuildLimits {
    ss::ClosedSSBuildLimits {
        seed_steps: 32,
        max_repairs: 32,
        coverage: ss::CoverageLimits {
            max_depth: 32,
            ..Default::default()
        },
        ..Default::default()
    }
}
#[test]
fn closed_branches_cross_exact_one_third_and_two_thirds_knots_without_rounding() {
    for center in [1., 1.5, 2.] {
        let source = source(center);
        let before = (source.first().clone(), source.second().clone());
        let owner = ss::construct_qualified_closed_ss_branches(&source, limits()).unwrap();
        assert_eq!(owner.branches().len(), 1);
        assert_eq!(owner.trims().len(), 2);
        assert_eq!(
            owner.owner().proof().coverage().report().reason,
            "complete-original-graph-reduction"
        );
        assert!(owner.owner().proof().coverage().covers_entire_surfaces());
        owner.reverify().unwrap();
        assert_eq!(before, (source.first().clone(), source.second().clone()));
    }
}
#[test]
fn generic_dispatch_owns_nondyadic_knots_and_cannot_omit_common_cells_on_exhaustion() {
    let source = source(1.);
    let owner = ss::construct_qualified_ss_branches(
        &source,
        ss::QualifiedSSBuildLimits {
            closed: limits(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        owner,
        ss::QualifiedSurfaceIntersection::RegularClosedLoops(_)
    ));
    assert!(owner.covers_entire_surfaces());
    owner.reverify().unwrap();
    let error = ss::construct_qualified_closed_ss_branches(
        &source,
        ss::ClosedSSBuildLimits {
            coverage: ss::CoverageLimits {
                max_cells: 1,
                ..Default::default()
            },
            ..limits()
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "NURBS_RESOURCE_LIMIT");
}
