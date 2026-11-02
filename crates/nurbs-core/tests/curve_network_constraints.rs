use nurbs_core::{
    constrained_surface::{
        Constraint,
        network::{self, BoundaryLocks, CurveConstraint, Options, SupportTrace},
    },
    curve::Curve,
    patches,
    surface::{Axis, Surface},
    surface_join::{Boundary, Decision},
};
fn plane(degree: usize) -> Surface {
    patches::bezier(
        (0..=degree)
            .map(|i| {
                (0..=degree)
                    .map(|j| vec![i as f64 / degree as f64, j as f64 / degree as f64, 0.])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap()
}
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
fn guide<'a>(c: &'a Curve, p: &'a Curve) -> CurveConstraint<'a> {
    CurveConstraint {
        spatial: c,
        parameters: p,
        reverse: false,
        tangent: None,
    }
}
#[test]
fn hidden_between_site_bump_requires_refinement_and_whole_curve_evidence() {
    let seed = plane(4);
    let before = seed.clone();
    // Quartic Bernstein height with zeros at the initial 0, 1/2, 1 sites.
    let c = Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: (0..5)
            .map(|i| vec![i as f64 / 4., 0.5, [0., 0.125, -1. / 6., 0.125, 0.][i]])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let p = line([0., 0.5], [1., 0.5]);
    assert!(c.evaluate(0.5).unwrap().point[2].abs() < 1e-15);
    assert!(c.evaluate(0.25).unwrap().point[2].abs() > 1e-3);
    let mut scarce = Options::default();
    scarce.max_iterations = 1;
    assert!(network::solve(&seed, &[guide(&c, &p)], &[], scarce).is_err());
    let result = network::solve(&seed, &[guide(&c, &p)], &[], Options::default()).unwrap();
    assert_eq!(seed, before);
    assert!(result.iterations() > 1);
    assert!(result.sites()[0].len() > 3);
    assert!(result.evidence().admitted);
    assert!(result.reverify(100000, 1000000).unwrap().admitted);
    assert_eq!(result.surface().weights, seed.weights);
    assert_eq!(result.surface().knots_u, seed.knots_u);
    for t in [0., 0.031, 0.25, 0.37, 0.5, 0.83, 1.] {
        let a = c.evaluate(t).unwrap().point;
        let b = result.surface().evaluate(t, 0.5).unwrap().point;
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-8));
    }
    assert!(result.work().0 <= 100000 && result.work().1 <= 1000000);
}
#[test]
fn rational_curved_guide_is_admitted_on_the_complete_original_clock() {
    let seed = plane(3);
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0.5, 0., 0.25],
            vec![0.5, 0.5, 0.25],
            vec![0., 0.5, 0.25],
        ],
        weights: vec![1., 0.5f64.sqrt(), 1.],
        periodic: false,
    };
    let mut p = c.clone();
    for q in &mut p.control_points {
        q.pop();
    }
    let result = network::solve(
        &seed,
        &[guide(&c, &p)],
        &[Constraint::Point {
            uv: [0., 0.],
            point: [0., 0., 0.],
        }],
        Options::default(),
    )
    .unwrap();
    assert!(result.evidence().admitted && result.reverify(100000, 0).unwrap().admitted);
    assert_eq!(result.original_guides()[0].spatial, &c);
    assert_eq!(result.original_guides()[0].parameters, &p);
    assert!(result.surface().evaluate(0., 0.).unwrap().point[2].abs() < 1e-8);
}
#[test]
fn whole_support_tangent_planes_use_original_support_curves() {
    let seed = plane(3);
    let mut support = plane(3);
    for p in support.control_points.iter_mut().flatten() {
        p[2] = 0.125 * p[0] + 0.25 * p[1];
    }
    let c = support.iso(Axis::V, 0.5).unwrap();
    let p = line([0., 0.5], [1., 0.5]);
    let g = CurveConstraint {
        spatial: &c,
        parameters: &p,
        reverse: false,
        tangent: Some(SupportTrace {
            surface: &support,
            parameters: &p,
            reverse: false,
        }),
    };
    let before = (seed.clone(), support.clone(), c.clone(), p.clone());
    let result = network::solve(&seed, &[g], &[], Options::default()).unwrap();
    assert!(result.evidence().admitted);
    let network::Evidence::TangentPlane(report) = &result.evidence().evidence[0] else {
        panic!("missing G1 evidence")
    };
    assert_eq!(report.decision, Decision::WithinTolerance);
    assert!(report.positions.proven && report.regularity_proven);
    assert!(result.reverify(100000, 1000000).unwrap().admitted);
    assert_eq!((seed, support, c, p), before);
}
#[test]
fn full_g2_boundary_jets_and_interior_curve_network_are_admitted_together() {
    let seed = plane(6);
    let mut desired = seed.clone();
    desired.control_points[3][3][2] = 0.25;
    let c = desired.iso(Axis::V, 0.5).unwrap();
    let p = line([0., 0.5], [1., 0.5]);
    let sides = [
        Boundary::UMin,
        Boundary::UMax,
        Boundary::VMin,
        Boundary::VMax,
    ];
    let result = network::solve_preserving_g2(
        &seed,
        &[guide(&c, &p)],
        &[],
        BoundaryLocks {
            reference: &seed,
            boundaries: &sides,
            tolerances: [1e-8, 1e-6, 1e-6],
        },
        Options::default(),
    )
    .unwrap();
    assert!(result.evidence().admitted);
    let boundary = result.boundary_evidence().unwrap();
    assert_eq!(boundary.surface(), result.surface());
    assert_eq!(boundary.seams().len(), 4);
    let replay = result.reverify(100000, 1000000).unwrap();
    assert!(replay.admitted);
    assert_eq!(replay.boundary_seams.len(), 4);
    assert!(
        boundary
            .seams()
            .iter()
            .all(|(_, r)| r.decision == Decision::WithinTolerance && r.regularity_proven)
    );
    for t in [0., 0.17, 0.5, 0.91, 1.] {
        for uv in [[0., t], [1., t], [t, 0.], [t, 1.]] {
            let a = seed.evaluate(uv[0], uv[1]).unwrap();
            let b = result.surface().evaluate(uv[0], uv[1]).unwrap();
            assert_eq!(a.point, b.point);
            assert_eq!(a.first_derivatives(), b.first_derivatives());
            assert_eq!(a.second_derivatives(), b.second_derivatives());
        }
    }
}
#[test]
fn reversed_clocks_and_nonunit_surface_domains_are_retained() {
    let mut seed = plane(3);
    for u in &mut seed.knots_u {
        *u = 10. + *u * 10.;
    }
    for v in &mut seed.knots_v {
        *v = -3. + *v * 10.;
    }
    let c = Curve::from_polyline(vec![vec![0., 0.5, 0.125], vec![1., 0.5, 0.25]])
        .unwrap()
        .reverse()
        .unwrap();
    let mut p = line([10., 2.], [20., 2.]);
    for t in &mut p.knots {
        *t = 2. + 3. * *t;
    }
    let result = network::solve(
        &seed,
        &[CurveConstraint {
            spatial: &c,
            parameters: &p,
            reverse: true,
            tangent: None,
        }],
        &[],
        Options::default(),
    )
    .unwrap();
    assert!(result.evidence().admitted && result.reverify(100000, 0).unwrap().admitted);
    assert_eq!(result.surface().knots_u, seed.knots_u);
    assert_eq!(result.original_guides()[0].parameters.knots, p.knots);
    for t in [0., 0.13, 0.5, 0.81, 1.] {
        let a = c.evaluate(1. - t).unwrap().point;
        let b = result.surface().evaluate(10. + 10. * t, 2.).unwrap().point;
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-8));
    }
}
#[test]
fn inconsistent_crossing_and_resource_limits_do_not_publish_a_sampled_fit() {
    let seed = plane(3);
    let before = seed.clone();
    let a = Curve::from_polyline(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![0.5, 0., 1.], vec![0.5, 1., 1.]]).unwrap();
    let pa = line([0., 0.5], [1., 0.5]);
    let pb = line([0.5, 0.], [0.5, 1.]);
    assert!(
        network::solve(
            &seed,
            &[guide(&a, &pa), guide(&b, &pb)],
            &[],
            Options::default()
        )
        .is_err()
    );
    let mut budget = Options::default();
    budget.max_cells = 1;
    assert!(network::solve(&seed, &[guide(&a, &pa)], &[], budget).is_err());
    budget = Options::default();
    budget.max_iterations = 0;
    assert!(network::solve(&seed, &[guide(&a, &pa)], &[], budget).is_err());
    let outside = line([-0.01, 0.5], [1., 0.5]);
    assert!(network::solve(&seed, &[guide(&a, &outside)], &[], Options::default()).is_err());
    assert_eq!(seed, before);
}

#[test]
fn crossing_original_rational_guides_share_the_authored_weight_basis() {
    let mut seed = plane(3);
    seed.weights[1][1] = 2.;
    seed.weights[2][1] = 0.5;
    let mut desired = seed.clone();
    desired.control_points[1][1][2] = 0.25;
    desired.control_points[2][2][2] = -0.125;
    let a = desired.iso(Axis::V, 0.25).unwrap();
    let b = desired.iso(Axis::U, 0.75).unwrap();
    let pa = line([0., 0.25], [1., 0.25]);
    let pb = line([0.75, 0.], [0.75, 1.]);
    let before = (seed.clone(), desired.clone(), a.clone(), b.clone());
    let result = network::solve(
        &seed,
        &[guide(&a, &pa), guide(&b, &pb)],
        &[],
        Options::default(),
    )
    .unwrap();
    assert!(result.evidence().admitted && result.reverify(100000, 0).unwrap().admitted);
    assert_eq!(result.surface().weights, seed.weights);
    assert_eq!(result.original_guides().len(), 2);
    let p = result.surface().evaluate(0.75, 0.25).unwrap().point;
    let expected = desired.evaluate(0.75, 0.25).unwrap().point;
    assert!(p.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-8));
    assert_eq!((seed, desired, a, b), before);
}

#[test]
fn curved_support_normals_and_all_clock_orientations_receive_whole_g1_proof() {
    let seed = plane(3);
    let mut support = plane(3);
    for (i, row) in support.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] = 0.001 * (i * (i.saturating_sub(1))) as f64 / 6.;
        }
    }
    let a = support.evaluate(0., 0.5).unwrap().unit_normal().unwrap();
    let b = support.evaluate(1., 0.5).unwrap().unit_normal().unwrap();
    assert!((a[0] - b[0]).abs() > 0.001);
    let c = support.iso(Axis::V, 0.5).unwrap();
    let p = line([0., 0.5], [1., 0.5]);
    for reverse in [false, true] {
        for support_reverse in [false, true] {
            let pc = if reverse {
                p.reverse().unwrap()
            } else {
                p.clone()
            };
            let sp = if support_reverse {
                p.reverse().unwrap()
            } else {
                p.clone()
            };
            let result = network::solve(
                &seed,
                &[CurveConstraint {
                    spatial: &c,
                    parameters: &pc,
                    reverse,
                    tangent: Some(SupportTrace {
                        surface: &support,
                        parameters: &sp,
                        reverse: support_reverse,
                    }),
                }],
                &[],
                Options::default(),
            )
            .unwrap();
            assert!(result.evidence().admitted);
            let network::Evidence::TangentPlane(report) = &result.evidence().evidence[0] else {
                panic!("missing G1")
            };
            assert_eq!(report.decision, Decision::WithinTolerance);
            assert!(report.normal_chord_upper.unwrap() <= 1e-5);
            assert!(result.work().0 <= 100000 && result.work().1 <= 1000000);
        }
    }
}
