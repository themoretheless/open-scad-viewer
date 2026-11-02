use nurbs_core::{
    constrained_surface::{
        Constraint,
        adaptive::{self, Direction, Inversion, Method, Options, Step},
        network::{self, BoundaryLocks, CurvatureConstraint, CurveConstraint, SupportTrace},
        rational,
        unparameterized::{Continuity, Guide},
    },
    curve::Curve,
    patches,
    surface::{Axis, Surface},
    surface_join::Boundary,
};
fn plane(p: usize) -> Surface {
    patches::bezier(
        (0..=p)
            .map(|i| {
                (0..=p)
                    .map(|j| vec![i as f64 / p as f64, j as f64 / p as f64, 0.])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap()
}
fn pc(v: f64) -> Curve {
    Curve::from_polyline(vec![vec![0., v], vec![1., v]]).unwrap()
}
fn guide<'a>(c: &'a Curve, p: &'a Curve) -> CurveConstraint<'a> {
    CurveConstraint {
        spatial: c,
        parameters: p,
        reverse: false,
        tangent: None,
    }
}
fn quartic() -> Curve {
    Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: (0..5)
            .map(|i| vec![i as f64 / 4., 0.5, [0., 0.125, -1. / 6., 0.125, 0.][i]])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    }
}
fn check(q: &adaptive::Qualified) {
    let r = q.reverify(100000, 1000000).unwrap();
    assert!(r.admitted);
    assert!(q.work().0 <= 100000 && q.work().1 <= 1000000);
    for g in q.network().original_guides() {
        for i in 0..=40 {
            let t = i as f64 / 40.;
            let d = g.parameters.domain();
            let uv = g
                .parameters
                .evaluate(d[0] + t * (d[1] - d[0]))
                .unwrap()
                .point;
            let d = g.spatial.domain();
            let t = if g.reverse { 1. - t } else { t };
            let a = g.spatial.evaluate(d[0] + t * (d[1] - d[0])).unwrap().point;
            let b = q.surface().evaluate(uv[0], uv[1]).unwrap().point;
            assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-6));
        }
    }
}
#[test]
fn hidden_quartic_requires_degree_growth_and_complete_original_admission() {
    let seed = plane(1);
    let c = quartic();
    let p = pc(0.5);
    let before = (seed.clone(), c.clone(), p.clone());
    assert!(network::solve(&seed, &[guide(&c, &p)], &[], network::Options::default()).is_err());
    let out = adaptive::solve(&seed, &[guide(&c, &p)], &[], &[], None, Options::default()).unwrap();
    assert!(out.refinements().iter().any(|s| matches!(
        s,
        Step::Elevate {
            direction: Direction::U,
            to: 4,
            ..
        }
    )));
    assert_eq!(out.surface().degree_u, 4);
    assert_eq!(out.surface().degree_v, 1);
    assert_eq!(out.original(), &seed);
    check(&out);
    assert_eq!((seed, c, p), before);
}
#[test]
fn local_original_breaks_and_multiplicities_are_added_to_the_surface() {
    let seed = plane(3);
    let p = pc(0.5);
    for multiplicity in [1, 2] {
        let mut knots = vec![0.; 4];
        knots.extend(std::iter::repeat_n(0.37, multiplicity));
        knots.extend([1.; 4]);
        let n = knots.len() - 4;
        let c = Curve {
            degree: 3,
            control_points: (0..n)
                .map(|i| {
                    let x = knots[i + 1..=i + 3].iter().sum::<f64>() / 3.;
                    vec![x, 0.5, [0., 0.13, -0.21, 0.17, -0.11, 0.][i]]
                })
                .collect(),
            weights: vec![1.; n],
            knots,
            periodic: false,
        };
        let out =
            adaptive::solve(&seed, &[guide(&c, &p)], &[], &[], None, Options::default()).unwrap();
        assert_eq!(out.surface().degree_u, 3);
        assert!(
            out.surface()
                .knots_u
                .iter()
                .filter(|t| (**t - 0.37).abs() < 1e-12)
                .count()
                >= multiplicity
        );
        check(&out);
    }
}
#[test]
fn refined_boundary_representations_do_not_replace_original_g2_authority() {
    for (degree, rational) in [(1, false), (3, false), (3, true)] {
        let mut seed = plane(degree);
        if rational {
            for (i, row) in seed.weights.iter_mut().enumerate() {
                for (j, w) in row.iter_mut().enumerate() {
                    *w = [1., 0.9, 1.3, 1.][i] * [1., 1.1, 0.95, 1.][j];
                }
            }
        }
        let target = nurbs_core::surface_edit::elevate(&seed, Axis::U, 6).unwrap();
        let mut target = nurbs_core::surface_edit::elevate(&target, Axis::V, 6).unwrap();
        target.control_points[3][3][2] = 0.4;
        let c = target.iso(Axis::V, 0.5).unwrap();
        let p = pc(0.5);
        let sides = [
            Boundary::UMin,
            Boundary::UMax,
            Boundary::VMin,
            Boundary::VMax,
        ];
        let locks = BoundaryLocks {
            reference: &seed,
            boundaries: &sides,
            tolerances: [1e-7, 1e-6, 1e-6],
        };
        let out = adaptive::solve(
            &seed,
            &[guide(&c, &p)],
            &[],
            &[],
            Some(locks),
            Options::default(),
        )
        .unwrap();
        assert_eq!(out.surface().degree_u, 6);
        assert_eq!(out.surface().degree_v, 6);
        assert_eq!(out.original_seams().len(), 4);
        assert_eq!(out.original_boundary_reference(), Some(&seed));
        assert!(out.original_seams().iter().all(|(_, r)| r.positions.proven
            && (r.positions.sides[1].exact.is_some() || r.positions.sides[1].interval.is_some())));
        assert_eq!(
            out.reverify(100000, 1000000).unwrap().original_seams.len(),
            4
        );
        assert!(out.surface().evaluate(0.5, 0.5).unwrap().point[2] > 0.03);
        check(&out);
    }
}
#[test]
fn interior_world_g2_can_trigger_transverse_basis_growth() {
    let seed = plane(1);
    let mut support = plane(3);
    for row in &mut support.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = 0.1 * [0.25, -1. / 12., -1. / 12., 0.25][j];
        }
    }
    let c = support.iso(Axis::V, 0.5).unwrap();
    let p = pc(0.5);
    let curvature = [CurvatureConstraint {
        spatial: &c,
        parameters: &p,
        reverse: false,
        support: SupportTrace {
            surface: &support,
            parameters: &p,
            reverse: false,
        },
    }];
    let out = adaptive::solve(&seed, &[], &curvature, &[], None, Options::default()).unwrap();
    assert!(out.surface().degree_u >= 3 && out.surface().degree_v >= 3);
    assert!(
        out.surface()
            .evaluate(0.5, 0.5)
            .unwrap()
            .second_derivatives()
            .unwrap()
            .2[2]
            > 0.05
    );
    check(&out);
}
#[test]
fn unknown_uvs_and_positive_weights_share_the_refined_original_anchor() {
    let seed = plane(1);
    let c = quartic();
    let mut options = Options::default();
    let mut positive = rational::Options::default();
    positive.weight_bounds = [0.8, 1.2];
    options.method = Method::PositiveWeights(positive);
    let out = adaptive::solve_unparameterized(
        &seed,
        &[Guide {
            spatial: &c,
            continuity: Continuity::Position,
        }],
        &[],
        None,
        options,
        Inversion::default(),
    )
    .unwrap();
    assert_eq!(out.surface().degree_u, 4);
    assert!(!out.refinements().is_empty());
    assert!(
        out.surface()
            .weights
            .iter()
            .flatten()
            .all(|w| *w >= 0.8 && *w <= 1.2)
    );
    assert_eq!(out.original(), &seed);
    check(&out);
}
#[test]
fn caps_incompatible_data_and_old_control_indices_publish_no_refined_geometry() {
    let seed = plane(1);
    let c = quartic();
    let p = pc(0.5);
    let before = seed.clone();
    let mut options = Options::default();
    options.max_controls = 4;
    assert!(adaptive::solve(&seed, &[guide(&c, &p)], &[], &[], None, options).is_err());
    options = Options::default();
    options.max_refinements = 0;
    assert!(adaptive::solve(&seed, &[guide(&c, &p)], &[], &[], None, options).is_err());
    options = Options::default();
    let hard = [Constraint::ControlPoint {
        index: [0, 0],
        point: [0., 0., 0.],
    }];
    let e = adaptive::solve(&seed, &[guide(&c, &p)], &[], &hard, None, options)
        .err()
        .unwrap();
    assert!(e.to_string().contains("indices"));
    let other = Curve::from_polyline(vec![vec![0., 0.5, 0.3], vec![1., 0.5, 0.3]]).unwrap();
    assert!(
        adaptive::solve(
            &seed,
            &[guide(&c, &p), guide(&other, &p)],
            &[],
            &[],
            None,
            options
        )
        .is_err()
    );
    assert_eq!(seed, before);
    let mut broken = c.clone();
    broken.control_points.clear();
    broken.weights.clear();
    assert!(adaptive::solve(&seed, &[guide(&broken, &p)], &[], &[], None, options).is_err());
}

#[test]
fn original_break_mapping_preserves_reverse_and_independent_nonunit_clocks() {
    let mut seed = plane(3);
    for u in &mut seed.knots_u {
        *u = 10. + 10. * *u;
    }
    for v in &mut seed.knots_v {
        *v = -2. + 4. * *v;
    }
    let original_knots = [0., 0., 0., 0., 0.37, 1., 1., 1., 1.];
    let c = Curve {
        degree: 3,
        knots: original_knots.iter().map(|t| -3. + 8. * t).collect(),
        control_points: (0..5)
            .map(|i| {
                vec![
                    original_knots[i + 1..=i + 3].iter().sum::<f64>() / 3.,
                    0.5,
                    [0., 0.2, -0.3, 0.15, 0.][i],
                ]
            })
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let mut p = Curve::from_polyline(vec![vec![20., 0.], vec![10., 0.]]).unwrap();
    for t in &mut p.knots {
        *t = 2. + 2. * *t;
    }
    let mut g = guide(&c, &p);
    g.reverse = true;
    let out = adaptive::solve(&seed, &[g], &[], &[], None, Options::default()).unwrap();
    assert!(
        out.surface()
            .knots_u
            .iter()
            .any(|t| (*t - 13.7).abs() < 1e-10)
    );
    assert_eq!(out.surface().knots_u[0], 10.);
    assert_eq!(*out.surface().knots_u.last().unwrap(), 20.);
    check(&out);
}

#[test]
fn rounded_basis_edits_cannot_inherit_zero_error_original_boundary_identity() {
    let mut seed = plane(3);
    for (i, row) in seed.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            p[0] = [0., 0.2, 0.7, 1.][i];
            p[1] = [0., 0.3, 0.8, 1.][j];
        }
    }
    let target = nurbs_core::surface_edit::elevate(&seed, Axis::U, 6).unwrap();
    let mut target = nurbs_core::surface_edit::elevate(&target, Axis::V, 6).unwrap();
    target.control_points[3][3][2] = 0.4;
    let c = target.iso(Axis::V, 0.5).unwrap();
    let p = pc(0.5);
    let sides = [
        Boundary::UMin,
        Boundary::UMax,
        Boundary::VMin,
        Boundary::VMax,
    ];
    let mut options = Options::default();
    options.max_refinements = 2;
    let lock = BoundaryLocks {
        reference: &seed,
        boundaries: &sides,
        tolerances: [0., 1e-6, 1e-6],
    };
    assert!(adaptive::solve(&seed, &[guide(&c, &p)], &[], &[], Some(lock), options).is_err());
}
