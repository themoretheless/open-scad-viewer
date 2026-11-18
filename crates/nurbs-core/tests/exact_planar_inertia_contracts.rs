use num_rational::BigRational as Q;
use nurbs_core::{
    curve::Curve,
    editing::exact_derivative::{ExactPlanarMassOutcome, Refusal, exact_planar_boundary_inertia},
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
fn integrates_a_translated_rational_rectangle_about_its_origin_and_centroid() {
    for reversed in [false, true] {
        let sign = if reversed { -1 } else { 1 };
        let report =
            exact_planar_boundary_inertia(&[rectangle([2., 2.], [5., 4.], reversed)], WORK)
                .unwrap();
        let moments = report
            .second_moments
            .unwrap_or_else(|| panic!("{:?}", report.mass));
        assert_eq!(
            moments.about_origin,
            [
                ratio(78 * sign, 1),
                ratio(63 * sign, 1),
                ratio(56 * sign, 1)
            ]
        );
        assert_eq!(
            moments.about_centroid,
            Some([ratio(9 * sign, 2), ratio(0, 1), ratio(2 * sign, 1)])
        );
    }
}

#[test]
fn integrates_the_second_moments_under_a_parabolic_arch() {
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
    let report = exact_planar_boundary_inertia(&[vec![arch, chord]], WORK).unwrap();
    let moments = report
        .second_moments
        .unwrap_or_else(|| panic!("{:?}", report.mass));
    assert_eq!(
        moments.about_origin,
        [ratio(-8, 5), ratio(-8, 15), ratio(-32, 105)]
    );
}

#[test]
fn opposite_winding_holes_subtract_and_zero_area_has_no_centroidal_tensor() {
    let outer = rectangle([0., 0.], [4., 4.], false);
    let hole = rectangle([1., 1.], [3., 3.], true);
    let report = exact_planar_boundary_inertia(&[outer.clone(), hole], WORK).unwrap();
    let inertia = report.second_moments.unwrap();
    assert_eq!(
        inertia.about_centroid,
        Some([ratio(20, 1), ratio(0, 1), ratio(20, 1)])
    );
    let inverse = rectangle([0., 0.], [4., 4.], true);
    let report = exact_planar_boundary_inertia(&[outer.clone(), inverse], WORK).unwrap();
    assert_eq!(report.second_moments.unwrap().about_centroid, None);
    let limited = exact_planar_boundary_inertia(&[outer], 1).unwrap();
    assert_eq!(
        limited.mass,
        ExactPlanarMassOutcome::Refused(Refusal::ResourceLimit)
    );
    assert!(limited.second_moments.is_none());
}
