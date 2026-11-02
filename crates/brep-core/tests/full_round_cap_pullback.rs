use brep_core::profile_certificate::original_restriction::plane::pullback;
use nurbs_core::{curve::Curve, surface::Surface};
fn surface() -> Surface {
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
fn curve(points: Vec<Vec<f64>>) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    }
}

fn rational(r: &[String; 2]) -> num_rational::BigRational {
    num_rational::BigRational::new(r[0].parse().unwrap(), r[1].parse().unwrap())
}
fn ratio(n: &str, d: &str) -> [String; 2] {
    [n.into(), d.into()]
}
#[test]
fn affine_pullback_keeps_exact_nonbinary_controls() {
    let mut s = surface();
    for row in &mut s.control_points {
        for p in row {
            p[0] = -1. + 6. * p[0];
            p[1] *= 8.;
        }
    }
    let world = curve(vec![vec![4., 0.], vec![5., 4.], vec![4., 8.]]);
    let proof = pullback::qualify(&s, &world).unwrap();
    assert_eq!(proof.world_curve(), &world);
    assert_eq!(proof.chart().surface(), &s);
    let spans = proof.affine_spans().unwrap().unwrap();
    assert_eq!(spans[0].homogeneous_controls()[0][0], ratio("5", "6"));
    let b = proof
        .inverse_box(ratio("1", "2"), ratio("1", "1000000000000"), 0)
        .unwrap();
    assert_eq!(b.bounds()[1], [ratio("1", "2"), ratio("1", "2")]);
    b.reverify().unwrap();
}
#[test]
fn bilinear_irrational_inverse_is_enclosed_and_budgeted() {
    let mut s = surface();
    s.control_points[1][1] = vec![1.125, 1.25, 0.];
    let world = curve(vec![vec![0., 0.], vec![1.125, 1.25]]);
    let proof = pullback::qualify(&s, &world).unwrap();
    assert!(proof.affine_spans().unwrap().is_none());
    assert!(proof
        .inverse_box(ratio("1", "2"), ratio("1", "1000000000000"), 0)
        .is_err());
    let b = proof
        .inverse_box(ratio("1", "2"), ratio("1", "1000000000000"), 128)
        .unwrap();
    let bounds = b.bounds();
    let limit = rational(&ratio("1", "1000000000000"));
    let zero = rational(&ratio("0", "1"));
    let one = rational(&ratio("1", "1"));
    for axis in &bounds {
        let lo = rational(&axis[0]);
        let hi = rational(&axis[1]);
        assert!(lo >= zero && hi <= one && lo <= hi);
        assert!(hi - lo <= limit);
    }
    let f = |u: num_rational::BigRational| {
        rational(&ratio("9", "1"))
            - rational(&ratio("15", "1")) * &u
            - rational(&ratio("4", "1")) * &u * u
    };
    assert!(f(rational(&bounds[0][0])) > zero);
    assert!(f(rational(&bounds[0][1])) < zero);
    b.reverify().unwrap();
    assert_eq!(proof.world_curve(), &world);
}
#[test]
fn actual_curve_containment_accepts_tangency_and_refuses_excursion() {
    let inside = curve(vec![vec![0.25, 0.5], vec![1.75, 0.5], vec![0.25, 0.5]]);
    pullback::qualify(&surface(), &inside)
        .unwrap()
        .reverify()
        .unwrap();
    let outside = curve(vec![vec![0.25, 0.5], vec![1.875, 0.5], vec![0.25, 0.5]]);
    assert!(pullback::qualify(&surface(), &outside).is_err());
}
