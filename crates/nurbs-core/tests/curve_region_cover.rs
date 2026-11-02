use nurbs_core::{
    curve::Curve,
    trim_domain::curve_cover::{self, Limits},
};
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
fn region() -> Vec<Vec<Curve>> {
    vec![vec![
        line([0., 0.], [4., 0.]),
        line([4., 0.], [4., 4.]),
        line([4., 4.], [0., 4.]),
        line([0., 4.], [0., 0.]),
    ]]
}
fn limits() -> Limits {
    Limits {
        cells: 100_000,
        classification_work: 1_000_000,
        witness_work: 1_000_000,
    }
}
#[test]
fn full_curve_cover_retains_original_identity_and_fresh_interior_proofs() {
    let loops = region();
    let r = curve_cover::inspect(&loops[0][0], &loops, 1e-7, limits()).unwrap();
    let c = r.certificate.unwrap();
    assert!(c.reverify().unwrap());
    assert_eq!(c.curve(), &loops[0][0]);
    let r = curve_cover::inspect(&line([1., 1.], [3., 3.]), &loops, 1e-7, limits()).unwrap();
    assert!(r.certificate.is_some());
    assert!(r.pending.is_empty());
}
#[test]
fn full_curve_cover_refuses_outside_hole_and_exhaustion() {
    let loops = region();
    let r = curve_cover::inspect(&line([5., 1.], [6., 2.]), &loops, 1e-7, limits()).unwrap();
    assert!(r.certificate.is_none());
    assert!(!r.pending.is_empty());
    let mut holes = loops.clone();
    holes.push(vec![
        line([1., 1.], [1., 3.]),
        line([1., 3.], [3., 3.]),
        line([3., 3.], [3., 1.]),
        line([3., 1.], [1., 1.]),
    ]);
    let r = curve_cover::inspect(&line([1.5, 1.5], [2.5, 2.5]), &holes, 1e-7, limits()).unwrap();
    assert!(r.certificate.is_none());
    let near = line([0., 0.], [1., 1.]);
    let r = curve_cover::inspect(
        &near,
        &loops,
        1e-7,
        Limits {
            cells: 1,
            ..limits()
        },
    )
    .unwrap();
    assert!(r.certificate.is_none());
}
