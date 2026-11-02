use brep_core::profile_certificate::original_restriction::{self as r, wire};
use nurbs_core::curve::Curve;
fn q(n: i64, d: i64) -> [String; 2] {
    [n.to_string(), d.to_string()]
}
fn line(a: [f64; 2], b: [f64; 2]) -> r::Restriction {
    let c = Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    r::restrict_binary64(&c, c.domain()).unwrap()
}
#[test]
fn exact_wire_preserves_fixed_source_and_bounds_entire_editable_piece() {
    let fixed = line([0., 0.], [1., 0.]);
    let bridge = line([1., 0.125], [0., 0.]);
    let sources = vec![fixed.clone(), bridge.clone()];
    let sewn = wire::sew(sources.clone(), vec![false, true], q(1, 8)).unwrap();
    sewn.reverify().unwrap();
    assert_eq!(sewn.pieces()[0], fixed.spans());
    assert_eq!(sewn.displacement_squared_bound(), q(1, 64));
    assert_eq!(sewn.sources()[1].source(), bridge.source());
    assert_eq!(
        sewn.pieces()[1][0].homogeneous_controls()[0],
        [q(1, 1), q(0, 1), q(1, 1)]
    );
    assert!(wire::sew(sources.clone(), vec![false, true], q(1, 16)).is_err());
    assert!(wire::sew(sources, vec![false, false], q(1, 1)).is_err());
}
#[test]
fn rational_contact_coordinate_is_shared_without_rounding_original_support() {
    let c = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let fixed = r::restrict(&c, [q(0, 1), q(1, 3)]).unwrap();
    let bridge = line([0.6, 0.], [0., 0.]);
    let sewn = wire::sew(
        vec![fixed.clone(), bridge],
        vec![false, true],
        q(1, 1_000_000),
    )
    .unwrap();
    assert_eq!(sewn.pieces()[0], fixed.spans());
    assert_eq!(
        sewn.pieces()[1][0].homogeneous_controls()[0],
        [q(3, 5), q(0, 1), q(1, 1)]
    );
    assert_ne!(sewn.displacement_squared_bound(), q(0, 1));
    sewn.reverify().unwrap();
}
