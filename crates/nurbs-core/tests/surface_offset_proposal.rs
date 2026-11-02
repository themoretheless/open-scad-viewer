use nurbs_core::{
    surface::Surface,
    surface_offset::{self, CandidateStatus as S, ProposalLimits as L, ProposalStop as Stop},
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
fn map() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![-4., -4., 2., 2.],
        knots_v: vec![10., 10., 10., 14., 14., 14.],
        control_points: [0.1, 0.8]
            .map(|u| vec![vec![u, 0.1, 0.], vec![u + 0.06, 0.5, 0.], vec![u, 0.9, 0.]])
            .to_vec(),
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn polynomial_map_refines_degree_and_recertifies_actual_rounded_poles() {
    let s = plane();
    let m = map();
    let before = (s.clone(), m.clone());
    let limits = L::default();
    let r = surface_offset::propose_parametric_candidate(&s, &m, 0.1, 0.0005, limits).unwrap();
    assert_eq!(r.stop, Stop::Certified);
    let cap = r.cap.as_ref().unwrap();
    assert!(cap.degree_u > 1);
    assert_eq!(r.attempts.first().unwrap().degree, 1);
    assert_ne!(r.attempts.first().unwrap().certificate.status, S::Within);
    assert_eq!(r.attempts.last().unwrap().certificate.status, S::Within);
    assert!(
        r.evaluations <= limits.evaluations
            && r.coefficient_work <= limits.coefficient_work
            && r.proof_cells <= limits.proof_cells
    );
    assert_eq!(
        r.proof_cells,
        r.attempts
            .iter()
            .map(|a| a.certificate.cells)
            .sum::<usize>()
    );
    assert_eq!(
        surface_offset::certify_parametric_candidate(&s, cap, &m, 0.1, 0.0005, 100000)
            .unwrap()
            .status,
        S::Within
    );
    assert_eq!((s, m), before);
}
#[test]
fn curved_original_normal_expression_gets_an_independently_admitted_cap() {
    let mut s = plane();
    s.degree_u = 2;
    s.knots_u = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = [[1., 0.], [1., 1.], [0., 1.]]
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
        .to_vec();
    s.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
        .map(|w| vec![w; 2])
        .to_vec();
    let mut m = plane();
    m.control_points = [0.1, 0.9]
        .map(|u| [0.1, 0.9].map(|v| vec![u, v, 0.]).to_vec())
        .to_vec();
    let r = surface_offset::propose_parametric_candidate(&s, &m, 0.1, 0.002, L::default()).unwrap();
    assert_eq!(r.stop, Stop::Certified);
    let c = r.cap.unwrap();
    assert!(c.degree_u > 1);
    assert_eq!(
        surface_offset::certify_parametric_candidate(&s, &c, &m, 0.1, 0.002, 100000)
            .unwrap()
            .status,
        S::Within
    );
}
#[test]
fn independent_work_limits_and_low_degree_never_publish_an_unproven_cap() {
    let s = plane();
    let m = map();
    for (limits, stop) in [
        (
            L {
                evaluations: 1,
                ..L::default()
            },
            Stop::EvaluationLimit,
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
        let r = surface_offset::propose_parametric_candidate(&s, &m, 0.1, 1e-12, limits).unwrap();
        assert!(r.cap.is_none());
        assert_eq!(r.stop, stop);
        assert!(
            r.evaluations <= limits.evaluations
                && r.coefficient_work <= limits.coefficient_work
                && r.proof_cells <= limits.proof_cells
        );
        assert!(r.attempts.iter().all(|a| a.certificate.status != S::Within));
    }
}
#[test]
fn invalid_maps_and_budgets_refuse_before_station_work() {
    let s = plane();
    let mut m = map();
    m.control_points.reverse();
    assert!(
        surface_offset::propose_parametric_candidate(&s, &m, 0.1, 0.001, L::default()).is_err()
    );
    assert!(
        surface_offset::propose_parametric_candidate(
            &s,
            &map(),
            0.1,
            0.001,
            L {
                max_degree: 0,
                ..L::default()
            }
        )
        .is_err()
    );
}
