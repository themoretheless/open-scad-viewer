use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn q(a: i64, b: i64) -> [String; 2] {
    [a.to_string(), b.to_string()]
}
fn square(lo: f64, hi: f64) -> Vec<Curve> {
    let p = [[lo, lo], [hi, lo], [hi, hi], [lo, hi], [lo, lo]];
    p.windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect()
}
fn owner(loops: &[Vec<Curve>]) -> Arc<p::OwnedOriginalCarriers> {
    let sources = loops.iter().flatten().cloned().collect::<Vec<_>>();
    Arc::new(
        p::own_original_carriers(Arc::new(
            p::assemble_original_contact_atlas(&sources, 1e-12, p::ContactAtlasLimits::default())
                .unwrap(),
        ))
        .unwrap(),
    )
}
#[test]
fn left_right_winding_is_attached_to_the_actual_original_carrier() {
    let loops = vec![square(0., 1.)];
    let original = owner(&loops);
    let sides =
        p::own_original_carrier_sides(original.clone(), &loops, 0, q(1, 2), q(1, 8), 192).unwrap();
    sides.reverify_source().unwrap();
    assert_eq!(sides.certificates()[0].windings, [1]);
    assert_eq!(sides.certificates()[1].windings, [0]);
    assert!(
        p::own_original_carrier_sides(original.clone(), &loops, 0, q(1, 2), q(2, 1), 192).is_err()
    );
    assert!(
        p::own_original_carrier_sides(original.clone(), &loops, 0, q(0, 1), q(1, 8), 192).is_err()
    );
    assert!(p::own_original_carrier_sides(original, &loops, 0, q(1, 1), q(1, 8), 192).is_err());
}
#[test]
fn nearby_nested_branch_cannot_be_crossed_by_a_side_query() {
    let loops = vec![square(0., 1.), square(0.0625, 0.9375)];
    let original = owner(&loops);
    assert!(
        p::own_original_carrier_sides(original.clone(), &loops, 0, q(1, 2), q(1, 8), 192).is_err()
    );
    let sides =
        p::own_original_carrier_sides(original.clone(), &loops, 0, q(1, 2), q(1, 32), 192).unwrap();
    sides.reverify_source().unwrap();
    assert_eq!(sides.certificates()[0].windings, [1, 0]);
    assert_eq!(sides.certificates()[1].windings, [0, 0]);
    let mut wrong = loops.clone();
    wrong.swap(0, 1);
    assert!(p::own_original_carrier_sides(original, &wrong, 0, q(1, 2), q(1, 32), 192).is_err());
}
#[test]
fn curved_original_carrier_proves_normal_sides_without_polygon_replacement() {
    let arch = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![1., 2., 1.],
        control_points: vec![vec![-2., 0.], vec![0., 4.], vec![2., 0.]],
        periodic: false,
    };
    let base = Curve::from_polyline(vec![vec![2., 0.], vec![-2., 0.]]).unwrap();
    let loops = vec![vec![arch, base]];
    let original = owner(&loops);
    let sides = p::own_original_carrier_sides(original, &loops, 0, q(1, 2), q(1, 32), 192).unwrap();
    sides.reverify_source().unwrap();
    assert_eq!(sides.certificates()[0].windings, [0]);
    assert_eq!(sides.certificates()[1].windings, [-1]);
}
