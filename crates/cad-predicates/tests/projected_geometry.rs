use cad_predicates::*;
fn input(points: &[[f64; 3]], normal: [f64; 3]) -> (SourceArena, Vec<[LeafRef; 3]>, [LeafRef; 3]) {
    let s = SourceArena::authored(
        "original-projected-test",
        0,
        points
            .iter()
            .flatten()
            .chain(normal.iter())
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let p = (0..points.len())
        .map(|i| {
            [
                s.leaf(3 * i).unwrap(),
                s.leaf(3 * i + 1).unwrap(),
                s.leaf(3 * i + 2).unwrap(),
            ]
        })
        .collect();
    let k = points.len() * 3;
    let n = [
        s.leaf(k).unwrap(),
        s.leaf(k + 1).unwrap(),
        s.leaf(k + 2).unwrap(),
    ];
    (s, p, n)
}
#[test]
fn original_coordinates_keep_orientation_erased_by_rounded_projection() {
    let big = 2f64.powi(53);
    let (s, p, n) = input(
        &[[big, 0., 0.], [big, 1., 0.], [big, 0., 1.]],
        [-1., 0., 1.],
    );
    let t = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(&s, &t, Limits::default(), None);
    let r = projected_orient3d(&mut c, p[0], p[1], p[2], n).unwrap();
    // Projecting to (x+z,y) in binary64 would erase the one-unit difference.
    assert_eq!(big + 1., big);
    assert_eq!(r.outcome, Outcome::Sign(Sign::Negative));
    assert_eq!(r.stage, Stage::ExactExpansion);
}
#[test]
fn projected_between_ignores_unrelated_height_and_preserves_area_orientation() {
    let (s, p, n) = input(
        &[[0., 0., 20.], [0.5, 0., -10.], [1., 0., 5.]],
        [0., 0., 1.],
    );
    let t = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(&s, &t, Limits::default(), None);
    assert_eq!(
        projected_orient3d(&mut c, p[0], p[1], p[2], n)
            .unwrap()
            .outcome,
        Outcome::Sign(Sign::Zero)
    );
    assert_eq!(
        projected_direction_dot3d(&mut c, p[1], p[0], p[1], p[2], n)
            .unwrap()
            .outcome,
        Outcome::Sign(Sign::Negative)
    );
    let (s, p, n) = input(
        &[[0., 0., 0.], [1., 0., 1.], [1., 1., 3.], [0., 1., 2.]],
        [0., 0., 1.],
    );
    let mut c = PredicateContext::new(&s, &t, Limits::default(), None);
    assert_eq!(
        projected_polygon_orientation(&mut c, &p, n)
            .unwrap()
            .outcome,
        Outcome::Sign(Sign::Positive)
    );
    let mut reverse = p.clone();
    reverse.reverse();
    assert_eq!(
        projected_polygon_orientation(&mut c, &reverse, n)
            .unwrap()
            .outcome,
        Outcome::Sign(Sign::Negative)
    );
}
#[test]
fn exhausted_or_foreign_sources_do_not_become_projection_signs() {
    let (s, p, n) = input(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], [0., 0., 1.]);
    let t = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(
        &s,
        &t,
        Limits {
            max_work: 1,
            ..Limits::default()
        },
        None,
    );
    assert!(matches!(
        projected_orient3d(&mut c, p[0], p[1], p[2], n)
            .unwrap()
            .outcome,
        Outcome::Indeterminate(_)
    ));
    let (_, _, foreign) = input(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], [0., 0., 1.]);
    let mut c = PredicateContext::new(&s, &t, Limits::default(), None);
    assert_eq!(
        projected_orient3d(&mut c, p[0], p[1], p[2], foreign),
        Err(InputError::ProvenanceMismatch)
    );
}
