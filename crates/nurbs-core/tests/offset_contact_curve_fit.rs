use nurbs_core::{
    offset_contact_pcurve::{self as uv, FitLimits as L, FitStop as Stop},
    surface::Surface,
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn independent_nonunit_source_parameters_get_both_fresh_whole_interval_proofs() {
    let mut a = plane();
    a.knots_u = vec![2., 2., 7., 7.];
    let mut b = plane();
    b.knots_u = vec![-3., -3., 4., 4.];
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    let saved = (a.clone(), b.clone());
    let work = L::default();
    let r = uv::fit_branch(
        [&a, &b],
        [0.2, -0.2],
        0,
        [3., 6.],
        [0.1, 0.3],
        [[-0.6, 1.6], [0.1, 0.3]],
        1e-8,
        work,
    )
    .unwrap();
    assert_eq!(r.stop, Stop::Certified);
    let curves = r.curves.as_ref().unwrap();
    assert!(curves.iter().all(|c| c.domain() == [3., 6.]));
    assert_eq!(r.attempts.last().unwrap().reports.len(), 2);
    assert!(
        r.attempts
            .last()
            .unwrap()
            .reports
            .iter()
            .all(|p| p.correspondence_proven)
    );
    assert!(
        r.root_queries <= work.root_queries
            && r.coefficient_work <= work.coefficient_work
            && r.proof_cells <= work.proof_cells
    );
    let bands: usize = r
        .attempts
        .iter()
        .flat_map(|a| &a.reports)
        .map(|p| p.band_queries)
        .sum();
    assert!(bands <= (work.root_refinements + 2) * r.proof_cells);
    assert_eq!((a, b), saved);
}
fn curved() -> [Surface; 2] {
    let mut a = plane();
    a.degree_u = 2;
    a.knots_u = vec![0., 0., 0., 1., 1., 1.];
    a.control_points = [[1., 0.], [1., 1.], [0., 1.]]
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
        .to_vec();
    a.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
        .map(|w| vec![w; 2])
        .to_vec();
    let mut b = plane();
    b.control_points = vec![
        vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        vec![vec![0., 2., 0.], vec![0.5, 2., 1.]],
    ];
    [a, b]
}
#[test]
fn nonlinear_original_offset_contact_refines_curves_without_world_point_authority() {
    let s = curved();
    let r = uv::fit_branch(
        s.each_ref(),
        [0.1; 2],
        1,
        [0.35, 0.55],
        [0.88, 0.96],
        [[0.07, 0.22], [0.3, 0.6]],
        1e-7,
        L::default(),
    )
    .unwrap();
    assert_eq!(r.stop, Stop::Certified);
    let curves = r.curves.as_ref().unwrap();
    assert!(curves[0].degree > 1);
    for side in 0..2 {
        let proof = uv::certify_refined(
            &curves[side],
            side,
            s.each_ref(),
            [0.1; 2],
            1,
            [0.35, 0.55],
            [0.88, 0.96],
            [[0.07, 0.22], [0.3, 0.6]],
            128,
            1e-7,
            100000,
            8,
            8,
        )
        .unwrap();
        assert!(proof.correspondence_proven, "{}", proof.reason);
        assert!(proof.band_queries > proof.visited);
        assert!(proof.band_queries <= 10 * proof.visited);
    }
}
#[test]
fn budget_and_degree_stops_never_publish_one_successful_curve() {
    let s = curved();
    for (work, stop) in [
        (
            L {
                root_queries: 1,
                ..L::default()
            },
            Stop::QueryLimit,
        ),
        (
            L {
                coefficient_work: 1,
                ..L::default()
            },
            Stop::CoefficientLimit,
        ),
        (
            L {
                proof_cells: 1,
                ..L::default()
            },
            Stop::ProofLimit,
        ),
        (
            L {
                max_degree: 1,
                ..L::default()
            },
            Stop::DegreeLimit,
        ),
    ] {
        let r = uv::fit_branch(
            s.each_ref(),
            [0.1; 2],
            1,
            [0.35, 0.55],
            [0.88, 0.96],
            [[0.07, 0.22], [0.3, 0.6]],
            1e-7,
            work,
        )
        .unwrap();
        assert!(r.curves.is_none());
        assert_eq!(r.stop, stop);
        assert!(
            r.root_queries <= work.root_queries
                && r.coefficient_work <= work.coefficient_work
                && r.proof_cells <= work.proof_cells
        );
    }
}
#[test]
fn the_wrong_uv_sheet_refuses_even_when_its_world_points_coincide() {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        let z = p[1];
        p[1] = 0.5;
        p[2] = z;
    }
    b.control_points.push(b.control_points[0].clone());
    b.control_points.push(b.control_points[1].clone());
    b.weights.push(vec![1.; 2]);
    b.weights.push(vec![1.; 2]);
    b.knots_u = vec![0., 0., 1., 2., 3., 3.];
    let r = uv::fit_branch(
        [&a, &b],
        [0.2; 2],
        0,
        [0.35, 0.39],
        [0.25, 0.35],
        [[0.30, 0.44], [0.15, 0.25]],
        1e-8,
        L::default(),
    )
    .unwrap();
    assert_eq!(r.stop, Stop::Certified);
    let curves = r.curves.unwrap();
    let mut wrong = curves[1].clone();
    for p in &mut wrong.control_points {
        p[0] += 2.;
    }
    for t in [0.35, 0.37, 0.39] {
        let p = curves[1].evaluate(t).unwrap().point;
        let q = wrong.evaluate(t).unwrap().point;
        let p = b.evaluate(p[0], p[1]).unwrap().point;
        let q = b.evaluate(q[0], q[1]).unwrap().point;
        assert!(p.iter().zip(q).all(|(a, b)| (*a - b).abs() < 1e-13));
        let p = curves[1].evaluate(t).unwrap().point;
        let q = wrong.evaluate(t).unwrap().point;
        let p = nurbs_core::surface_offset::evaluate(&b, [p[0], p[1]], 0.2)
            .unwrap()
            .point;
        let q = nurbs_core::surface_offset::evaluate(&b, [q[0], q[1]], 0.2)
            .unwrap()
            .point;
        assert!(p.iter().zip(q).all(|(a, b)| (*a - b).abs() < 1e-13));
    }
    let proof = uv::certify(
        &wrong,
        1,
        [&a, &b],
        [0.2; 2],
        0,
        [0.35, 0.39],
        [0.25, 0.35],
        [[0.30, 0.44], [0.15, 0.25]],
        128,
        1e-8,
        100000,
        8,
    )
    .unwrap();
    assert!(!proof.correspondence_proven);
    assert_eq!(proof.band_queries, proof.visited);
}
#[test]
fn excluded_and_unresolved_bands_and_invalid_work_never_authorize_curves() {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 3.;
    }
    let absent = uv::fit_branch(
        [&a, &b],
        [0.2; 2],
        0,
        [0.2, 0.8],
        [0.1, 0.9],
        [[0.1, 0.9]; 2],
        1e-6,
        L::default(),
    )
    .unwrap();
    assert_eq!(absent.stop, Stop::NoContact);
    assert!(absent.curves.is_none());
    assert_eq!(absent.root_queries, 0);
    let same = uv::fit_branch(
        [&a, &a],
        [0.2; 2],
        0,
        [0.2, 0.8],
        [0.1, 0.9],
        [[0.1, 0.9]; 2],
        1e-6,
        L::default(),
    )
    .unwrap();
    assert_eq!(same.stop, Stop::BandUnresolved);
    assert!(same.curves.is_none());
    assert!(
        uv::fit_branch(
            [&a, &b],
            [0.2; 2],
            0,
            [0.2, 0.8],
            [0.1, 0.9],
            [[0.1, 0.9]; 2],
            1e-6,
            L {
                max_degree: 0,
                ..L::default()
            }
        )
        .is_err()
    );
}
