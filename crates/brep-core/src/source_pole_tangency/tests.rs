use super::*;
use crate::source_boundary_fragment::{Endpoint, Fragment};
use nurbs_core::{curve::Curve, surface::Surface};
fn pole() -> CollapsedBoundary {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0.; 3]; 2],
            vec![vec![0.5, 0., 0.], vec![0.5, 0.5, 0.]],
            vec![vec![1., 0., 1.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1., 0.5]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let uv = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
    let f = Fragment::new(&s, &uv, Endpoint::Parameter(0.), Endpoint::Parameter(1.))
        .unwrap()
        .split_at_parameter(0.375)
        .unwrap()[1]
        .clone();
    crate::source_collapsed_boundary::qualify(&f, [0.; 3], 1000000)
        .unwrap()
        .boundary
        .unwrap()
}
#[test]
fn limit_token_keeps_the_actual_original_collapsed_restriction() {
    let p = pole();
    let before = p.source().definition();
    let report = qualify(&p, 100000000).unwrap();
    let proof = report.certificate.expect(report.reason);
    assert!(std::ptr::eq(proof.pole(), &p));
    assert!(std::ptr::eq(proof.limit().surface(), p.source().surface()));
    assert_eq!(proof.pole().source().definition(), before);
    assert_eq!(proof.limit().factor_order(), 1);
    let short = qualify(&p, report.work - 1).unwrap();
    assert!(short.certificate.is_none());
    assert!(short.work < report.work);
    assert!(qualify(&p, 0).is_err());
}
#[test]
fn collapse_identity_alone_cannot_supply_a_common_plane() {
    let s = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![0.; 3]; 3],
            vec![vec![1., 0., 1.], vec![1., 1., 1.], vec![0., 1., 1.]],
        ],
        weights: vec![vec![1., 0.5, 1.]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let uv = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
    let f = Fragment::new(&s, &uv, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
    let p = crate::source_collapsed_boundary::qualify(&f, [0.; 3], 1000000)
        .unwrap()
        .boundary
        .unwrap();
    assert!(qualify(&p, 100000000).unwrap().certificate.is_none());
}
