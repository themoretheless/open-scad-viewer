use brep_core::profile_certificate::original_restriction::{self as exact, wire};
use nurbs_core::curve::Curve;
fn q(n: &str, d: &str) -> [String; 2] {
    [n.into(), d.into()]
}
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
fn sources(parallel: bool) -> Vec<exact::Restriction> {
    let arc = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![2., 0.125], vec![2., 1.]],
        weights: vec![1., 0.7, 1.],
        periodic: false,
    };
    let (end, corner) = if parallel {
        ([3., 1.], [3., 2.])
    } else {
        ([2., 2.], [1., 2.])
    };
    let curves = vec![
        line([0., 0.], [1., 0.]),
        arc,
        line([2., 1.], end),
        line(end, corner),
        line(corner, [0., 2.]),
        line([0., 2.], [0., 0.]),
    ];
    curves
        .iter()
        .map(|c| exact::restrict_binary64(c, c.domain()).unwrap())
        .collect()
}
#[test]
fn exact_tangent_intersection_bounds_middle_control_and_preserves_all_weights() {
    let original = sources(false);
    let mask = vec![false, true, false, false, false, false];
    let g0 = wire::sew(original.clone(), mask.clone(), q("0", "1")).unwrap();
    assert_eq!(g0.displacement_squared_bound(), q("0", "1"));
    assert!(g0.qualify_smooth_edits(q("0", "1")).is_err());
    assert!(wire::sew_tangent_repaired(original.clone(), mask.clone(), q("1", "16")).is_err());
    let repaired = wire::sew_tangent_repaired(original.clone(), mask, q("1", "4")).unwrap();
    assert!(repaired.tangent_repaired());
    assert_eq!(repaired.displacement_squared_bound(), q("1", "64"));
    for (i, (before, after)) in original.iter().zip(repaired.pieces()).enumerate() {
        for (a, b) in before.spans().iter().zip(after) {
            let a = a.homogeneous_controls();
            let b = b.homogeneous_controls();
            for (a, b) in a.iter().zip(&b) {
                assert_eq!(a[2], b[2]);
            }
            if i != 1 {
                assert_eq!(a, b);
            }
        }
    }
    let proof = repaired.qualify_smooth_edits(q("0", "1")).unwrap();
    assert!(proof
        .junctions()
        .iter()
        .all(|j| j.sine_squared == q("0", "1")));
    proof.reverify().unwrap();
    repaired.reverify().unwrap();
}
#[test]
fn parallel_lines_do_not_create_a_fabricated_quadratic_intersection() {
    assert!(wire::sew_tangent_repaired(
        sources(true),
        vec![false, true, false, false, false, false],
        q("1", "1")
    )
    .is_err());
}
