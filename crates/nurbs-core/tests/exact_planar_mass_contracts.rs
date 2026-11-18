use num_rational::BigRational as Q;
use nurbs_core::{
    curve::Curve,
    editing::exact_derivative::{
        ExactPlanarMass, ExactPlanarMassOutcome, Refusal, exact_planar_boundary_mass,
    },
};
const WORK: u64 = 1_000_000;

fn rectangle(a: [f64; 2], b: [f64; 2], reversed: bool) -> Vec<Curve> {
    let mut poles = vec![a, [b[0], a[1]], b, [a[0], b[1]], a];
    if reversed {
        poles.reverse();
    }
    vec![Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 4., 4.],
        control_points: poles.into_iter().map(Vec::from).collect(),
        weights: vec![1., 2., 4., 8., 1.],
        periodic: false,
    }]
}

fn ratio(a: i64, b: i64) -> Q {
    Q::new(a.into(), b.into())
}

#[test]
fn subtracts_an_off_center_hole_and_preserves_winding_sign() {
    for reversed in [false, true] {
        let sign = if reversed { -1 } else { 1 };
        let loops = [
            rectangle([0., 0.], [4., 4.], reversed),
            rectangle([2., 1.], [3., 2.], !reversed),
        ];
        assert_eq!(
            exact_planar_boundary_mass(&loops, WORK).unwrap().outcome,
            ExactPlanarMassOutcome::Mass(ExactPlanarMass {
                signed_area: ratio(15 * sign, 1),
                first_moments: [ratio(59 * sign, 2), ratio(61 * sign, 2)],
                centroid: Some([ratio(59, 30), ratio(61, 30)]),
            })
        );
    }
}

#[test]
fn integrates_a_polynomial_arch_and_rational_segment_consistently() {
    let arch = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 2.], vec![2., 0.]],
        weights: vec![2.; 3],
        periodic: false,
    };
    let chord = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![2., 0.], vec![0., 0.]],
        weights: vec![4., 1.],
        periodic: false,
    };
    assert_eq!(
        exact_planar_boundary_mass(&[vec![arch, chord]], WORK)
            .unwrap()
            .outcome,
        ExactPlanarMassOutcome::Mass(ExactPlanarMass {
            signed_area: ratio(-4, 3),
            first_moments: [ratio(-4, 3), ratio(-8, 15)],
            centroid: Some([ratio(1, 1), ratio(2, 5)]),
        })
    );
}

#[test]
fn cancels_opposite_contours_and_refuses_exhausted_arithmetic() {
    let loops = [
        rectangle([0., 0.], [2., 1.], false),
        rectangle([0., 0.], [2., 1.], true),
    ];
    assert_eq!(
        exact_planar_boundary_mass(&loops, WORK).unwrap().outcome,
        ExactPlanarMassOutcome::Mass(ExactPlanarMass {
            signed_area: ratio(0, 1),
            first_moments: [ratio(0, 1), ratio(0, 1)],
            centroid: None,
        })
    );
    let report = exact_planar_boundary_mass(&loops, 1).unwrap();
    assert_eq!(
        report.outcome,
        ExactPlanarMassOutcome::Refused(Refusal::ResourceLimit)
    );
    assert!(report.work_used <= 1);
}
