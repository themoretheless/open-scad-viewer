use nurbs_core::{
    curve::Curve,
    offset_contact_pcurve as uv,
    surface::Surface,
    surface_offset::contact_curve::{self as world, FitLimits as L, FitStop as Stop, Limits},
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
fn proof() -> Limits {
    Limits {
        cells: 100000,
        spans: 128,
        section_refinements: 8,
        band_refinements: 8,
    }
}
#[test]
fn nonunit_contact_clock_has_an_independent_complete_world_error_bound() {
    let mut a = plane();
    a.knots_u = vec![2., 2., 7., 7.];
    let mut b = plane();
    b.knots_u = vec![-3., -3., 4., 4.];
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    let saved = (a.clone(), b.clone());
    let limits = L::default();
    let r = world::fit(
        [&a, &b],
        [0.2, -0.2],
        0,
        [3., 6.],
        [0.1, 0.3],
        [[-0.6, 1.6], [0.1, 0.3]],
        1e-8,
        limits,
    )
    .unwrap();
    assert_eq!(r.stop, Stop::Certified);
    let c = r.curve.as_ref().unwrap();
    assert_eq!(c.domain(), [3., 6.]);
    assert!(
        r.root_queries <= limits.root_queries
            && r.proof_cells <= limits.proof_cells
            && r.coefficient_work <= limits.coefficient_work
    );
    let fresh = world::certify(
        c,
        nurbs_core::offset::contact_pcurve::OffsetFaces { surfaces: [&a, &b], distances: [0.2, -0.2] },
        world::ContactDomain { axis: 0, drive: [3., 6.], free: [0.1, 0.3], second: [[-0.6, 1.6], [0.1, 0.3]] },
        1e-8,
        proof(),
    )
    .unwrap();
    assert!(fresh.world_correspondence_proven && fresh.error_upper_mm.unwrap() <= 1e-8);
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
fn nonlinear_world_contact_refines_degree_and_covers_the_literal_rounded_curve() {
    let s = curved();
    let r = world::fit(
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
    let c = r.curve.as_ref().unwrap();
    assert!(c.degree > 1);
    let fresh = world::certify(
        c,
        nurbs_core::offset::contact_pcurve::OffsetFaces { surfaces: s.each_ref(), distances: [0.1; 2] },
        world::ContactDomain { axis: 1, drive: [0.35, 0.55], free: [0.88, 0.96], second: [[0.07, 0.22], [0.3, 0.6]] },
        1e-7,
        proof(),
    )
    .unwrap();
    assert!(fresh.world_correspondence_proven && fresh.error_upper_mm.unwrap() <= 1e-7);
    assert!(fresh.band_queries <= 10 * fresh.visited);
}
#[test]
fn matching_endpoints_cannot_admit_a_world_curve_with_interior_drift() {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.2, 0.2], vec![0.5, 0.2, 0.28], vec![1., 0.2, 0.2]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = world::certify(
        &c,
        nurbs_core::offset::contact_pcurve::OffsetFaces { surfaces: [&a, &b], distances: [0.2, -0.2] },
        world::ContactDomain { axis: 0, drive: [0., 1.], free: [0.1, 0.3], second: [[0.2, 0.8], [0.1, 0.3]] },
        1e-7,
        proof(),
    )
    .unwrap();
    assert!(!r.world_correspondence_proven && r.error_upper_mm.is_none());
    assert_eq!(r.reason, "world-contact-mismatch");
}
#[test]
fn each_work_limit_and_insufficient_degree_publish_no_world_curve() {
    let s = curved();
    for (limits, stop) in [
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
        let r = world::fit(
            s.each_ref(),
            [0.1; 2],
            1,
            [0.35, 0.55],
            [0.88, 0.96],
            [[0.07, 0.22], [0.3, 0.6]],
            1e-7,
            limits,
        )
        .unwrap();
        assert!(r.curve.is_none());
        assert_eq!(r.stop, stop);
        assert!(
            r.root_queries <= limits.root_queries
                && r.proof_cells <= limits.proof_cells
                && r.coefficient_work <= limits.coefficient_work
        );
    }
}
#[test]
fn whole_world_coincidence_does_not_authorize_the_wrong_original_uv_sheet() {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        let z = p[1];
        p[1] = 0.5;
        p[2] = z;
    }
    b.control_points.push(b.control_points[0].clone());
    b.control_points.push(b.control_points[1].clone());
    b.weights.extend([vec![1.; 2], vec![1.; 2]]);
    b.knots_u = vec![0., 0., 1., 2., 3., 3.];
    let r = world::fit(
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
    let pair = uv::fit_branch(
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
    let mut wrong = pair.curves.unwrap()[1].clone();
    for p in &mut wrong.control_points {
        p[0] += 2.;
    }
    let mut offset = b.clone();
    for p in offset.control_points.iter_mut().flatten() {
        p[1] = 0.3;
    }
    let world_agreement = nurbs_core::curve_surface_agreement::verify(
        r.curve.as_ref().unwrap(),
        &wrong,
        &offset,
        false,
        1e-8,
        100000,
    )
    .unwrap();
    assert_eq!(
        world_agreement.status,
        nurbs_core::curve_surface_agreement::Status::WithinTolerance
    );
    let parameter = uv::certify(
        &wrong,
        1,
        uv::OffsetFaces { surfaces: [&a, &b], distances: [0.2; 2] },
        uv::ContactParameters { axis: 0, drive: [0.35, 0.39], free: [0.25, 0.35], second: [[0.30, 0.44], [0.15, 0.25]], spans: 128 },
        1e-8,
        100000,
        8,
    )
    .unwrap();
    assert!(!parameter.correspondence_proven);
}
#[test]
fn absent_nonisolated_and_wrong_clock_inputs_never_provide_world_authority() {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 3.;
    }
    let absent = world::fit(
        [&a, &b],
        [0.1; 2],
        0,
        [0.2, 0.8],
        [0.1, 0.9],
        [[0.1, 0.9]; 2],
        1e-7,
        L::default(),
    )
    .unwrap();
    assert_eq!(absent.stop, Stop::NoContact);
    assert!(absent.curve.is_none());
    let same = world::fit(
        [&a, &a],
        [0.1; 2],
        0,
        [0.2, 0.8],
        [0.1, 0.9],
        [[0.1, 0.9]; 2],
        1e-7,
        L::default(),
    )
    .unwrap();
    assert_eq!(same.stop, Stop::BandUnresolved);
    assert!(same.curve.is_none());
    let c = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0.; 3], vec![1.; 3]],
        weights: vec![1.; 2],
        periodic: false,
    };
    assert!(
        world::certify(
            &c,
            nurbs_core::offset::contact_pcurve::OffsetFaces { surfaces: [&a, &b], distances: [0.1; 2] },
            world::ContactDomain { axis: 0, drive: [0.2, 0.8], free: [0.1, 0.9], second: [[0.1, 0.9]; 2] },
            1e-7,
            proof(),
        )
        .is_err()
    );
}
