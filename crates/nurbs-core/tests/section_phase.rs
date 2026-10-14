use nurbs_core::{
    curve::Curve,
    periodic_seam,
    section_phase::{self, Status},
};
fn square(z: f64) -> Curve {
    Curve::from_polyline(vec![
        vec![0., 0., z],
        vec![2., 0., z],
        vec![2., 2., z],
        vec![0., 2., z],
        vec![0., 0., z],
    ])
    .unwrap()
}
#[test]
fn landmarks_align_closed_sections_at_an_interior_edge_point() {
    let curves = vec![square(0.), square(2.)];
    let before = curves.clone();
    let anchors = vec![vec![2., 1., -1.], vec![2., 1., 3.]];
    let r = section_phase::align(&curves, &anchors, 0.01, 0.1, 10000).unwrap();
    assert!(r.phases.iter().all(|p| p.status == Status::Aligned));
    let aligned = r.curves.unwrap();
    for (c, phase) in aligned.iter().zip(&r.phases) {
        assert!((phase.parameter - 1.5).abs() < 1e-8);
        assert!(
            phase.nearest.distance_interval[0] <= 1. && phase.nearest.distance_interval[1] >= 1.
        );
        assert!(phase.shape.as_ref().unwrap().accepted);
        let final_distance = phase.final_distance_interval.unwrap();
        assert!(final_distance[0] <= 1. && final_distance[1] >= 1.);
        assert!(phase.final_minimum_gap_upper.unwrap() <= 0.11);
        let p = c.evaluate(c.domain()[0]).unwrap().point;
        assert!((p[0] - 2.).abs() < 1e-10 && (p[1] - 1.).abs() < 1e-10);
        assert_eq!(c.control_points.first(), c.control_points.last());
        assert!(!c.periodic);
    }
    assert_eq!(curves, before);
}
#[test]
fn closed_rational_curves_receive_continuous_phase_bounds() {
    let source = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 2.).unwrap();
    assert!(!source.periodic);
    assert_eq!(source.control_points.first(), source.control_points.last());
    for seam in [0.125, 0.333, 0.875] {
        let q = section_phase::curve_candidate(&source, seam).unwrap();
        let proof = periodic_seam::verify_curve(&source, &q, 0.1, 10000).unwrap();
        assert!(proof.accepted, "{proof:?}");
        for i in 0..=240 {
            let t = q.domain()[0] + (q.domain()[1] - q.domain()[0]) * i as f64 / 240.;
            let p = q.evaluate(t).unwrap().point;
            assert!((p[0].hypot(p[1]) - 2.).abs() < 1e-10);
            assert!(p[2].abs() < 1e-10);
        }
    }
}
#[test]
fn unresolved_results_are_atomic_and_invalid_inputs_are_rejected() {
    let curves = vec![square(0.), square(2.)];
    let before = curves.clone();
    let anchors = vec![vec![2., 1., -1.], vec![2., 1., 3.]];
    let r = section_phase::align(&curves, &anchors, 0.01, 1e-12, 5).unwrap();
    assert!(r.curves.is_none());
    assert!(
        r.phases
            .iter()
            .all(|p| p.final_distance_interval.is_none() && p.final_minimum_gap_upper.is_none())
    );
    assert!(r.phases.iter().any(|p| p.status != Status::Aligned));
    assert_eq!(curves, before);
    assert!(section_phase::align(&curves, &anchors[..1], 0.1, 0.1, 100).is_err());
    assert!(section_phase::align(&curves, &anchors, 0., 0.1, 100).is_err());
    assert!(section_phase::align(&curves, &anchors, 0.1, 0.1, 0).is_err());
    let open = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
    assert!(section_phase::align(&[curves[0].clone(), open], &anchors, 0.1, 0.1, 100).is_err());
    let mut bad = anchors;
    bad[0][0] = f64::NAN;
    assert!(section_phase::align(&curves, &bad, 0.1, 0.1, 100).is_err());
}

#[test]
fn periodic_and_clamped_sections_share_landmarks_without_claiming_unique_phase() {
    let periodic = Curve {
        degree: 1,
        knots: vec![0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![2., 0., 0.],
            vec![0., 2., 0.],
            vec![0., 0., 0.],
        ],
        weights: vec![1.; 4],
        periodic: true,
    };
    let r = section_phase::align(
        &[periodic.clone(), square(0.)],
        &[vec![1., 0., -1.], vec![1., 0., -1.]],
        0.01,
        0.1,
        10000,
    )
    .unwrap();
    let out = r.curves.unwrap();
    assert!(out[0].periodic);
    assert!(!out[1].periodic);
    for c in out {
        let p = c.evaluate(c.domain()[0]).unwrap().point;
        assert!((p[0] - 1.).abs() < 1e-10 && p[1].abs() < 1e-10);
    }
    let mut constant = periodic;
    for p in &mut constant.control_points {
        *p = vec![0.; 3];
    }
    let r = section_phase::align(
        &[constant.clone(), constant],
        &[vec![0., 0., 1.], vec![0., 0., 1.]],
        0.01,
        0.1,
        100,
    )
    .unwrap();
    assert!(r.curves.is_some());
    // Every parameter is an equally valid nearest witness in this fixture.
    assert!(
        r.phases
            .iter()
            .all(|p| p.nearest.converged && p.status == Status::Aligned)
    );
}
