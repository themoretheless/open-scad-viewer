use brep_core::profile_certificate as p;
use num_rational::BigRational as Q;
use nurbs_core::curve::Curve;
fn wire(points: &[[f64; 2]]) -> Vec<Curve> {
    vec![
        Curve::from_polyline(
            points
                .iter()
                .chain(points.first())
                .map(|p| p.to_vec())
                .collect(),
        )
        .unwrap(),
    ]
}
fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Vec<Curve> {
    wire(&[[x, y], [x + w, y], [x + w, y + h], [x, y + h]])
}
fn area(rings: p::ExactRegionRings) -> Q {
    rings
        .into_iter()
        .map(|r| {
            let r = r
                .into_iter()
                .map(|p| p.map(|q| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap())))
                .collect::<Vec<_>>();
            (0..r.len())
                .map(|i| &r[i][0] * &r[(i + 1) % r.len()][1] - &r[i][1] * &r[(i + 1) % r.len()][0])
                .sum::<Q>()
                / Q::from_integer(2.into())
        })
        .sum()
}
fn check(source: &[Vec<Curve>], coefficients: &[i32]) -> p::LinearWindingArrangement {
    let r = p::regularize_linear_winding_arrangement(source, coefficients).unwrap();
    r.reverify_source().unwrap();
    r
}
#[test]
fn crossing_overlap_and_signed_nonzero_selection_are_source_owned() {
    let source = vec![rectangle(0., 0., 4., 4.), rectangle(2., 0., 4., 4.)];
    let r = check(&source, &[1, 1]);
    assert_eq!(r.groups().len(), 1);
    assert_eq!(area(r.exact_regions()), Q::from_integer(24.into()));
    let r = check(&source, &[1, -1]);
    assert_eq!(r.groups().len(), 2);
    assert_eq!(area(r.exact_regions()), Q::from_integer(16.into()));
    let r = check(&source, &[2, -1]);
    assert_eq!(area(r.exact_regions()), Q::from_integer(24.into()));
    let r = check(&source, &[0, 0]);
    assert!(r.exact_regions().is_empty());
}
#[test]
fn negative_islands_holes_and_self_crossing_lobes_keep_actual_winding() {
    let r = check(
        &[rectangle(0., 0., 4., 4.), rectangle(10., 0., 2., 2.)],
        &[1, -1],
    );
    assert_eq!(r.groups().len(), 2);
    assert_eq!(area(r.exact_regions()), Q::from_integer(20.into()));
    let hole = rectangle(1., 1., 2., 2.)
        .into_iter()
        .rev()
        .map(|c| c.reverse().unwrap())
        .collect::<Vec<_>>();
    let source = vec![rectangle(0., 0., 4., 4.), hole];
    let r = check(&source, &[1, 1]);
    assert_eq!(r.groups()[0].len(), 2);
    assert_eq!(area(r.exact_regions()), Q::from_integer(12.into()));
    let r = check(&source, &[1, -1]);
    assert_eq!(r.exact_regions().len(), 1);
    assert_eq!(area(r.exact_regions()), Q::from_integer(16.into()));
    let bow = wire(&[[0., 0.], [4., 4.], [0., 4.], [4., 0.]]);
    let r = check(&[bow], &[1]);
    assert_eq!(r.groups().len(), 2);
    assert_eq!(area(r.exact_regions()), Q::from_integer(8.into()));
}
#[test]
fn contact_fans_shared_borders_and_coincident_cancellation_are_exact() {
    let a = rectangle(0., 0., 2., 2.);
    let r = check(&[a.clone(), rectangle(2., 2., 2., 2.)], &[1, 1]);
    assert_eq!(r.groups().len(), 2);
    let r = check(&[a.clone(), rectangle(2., 0., 2., 2.)], &[1, 1]);
    assert_eq!(r.groups().len(), 1);
    assert_eq!(area(r.exact_regions()), Q::from_integer(8.into()));
    let r = check(&[a.clone(), a], &[1, -1]);
    assert!(r.exact_regions().is_empty());
}
#[test]
fn nonbinary_intersection_vertices_remain_exact_rational_values() {
    let a = wire(&[[0., 0.], [4., 0.], [0., 4.]]);
    let b = wire(&[[1., -1.], [2., 2.], [3., -1.]]);
    let r = check(&[a, b], &[1, -1]);
    assert_eq!(area(r.exact_regions()), Q::new(29.into(), 3.into()));
    assert!(
        r.exact_regions()
            .iter()
            .flatten()
            .flatten()
            .any(|q| q[1] == "3")
    );
}
#[test]
fn nonlinear_rational_line_parameterization_is_proved_not_chord_fitted() {
    let points = [[0., 0.], [2., 0.], [2., 2.], [0., 2.]];
    let wire = (0..4)
        .map(|i| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                points[i].to_vec(),
                points[i].to_vec(),
                points[(i + 1) % 4].to_vec(),
            ],
            weights: vec![1., 2., 1.],
            periodic: false,
        })
        .collect::<Vec<_>>();
    let r = check(&[wire.clone()], &[1]);
    assert_eq!(area(r.exact_regions()), Q::from_integer(4.into()));
    let mut curved = wire;
    curved[0].control_points[1][1] = f64::from_bits(1);
    assert!(p::regularize_linear_winding_arrangement(&[curved.clone()], &[1]).is_err());
    assert!(p::regularize_linear_winding_arrangement(&[curved], &[0]).is_err());
}
