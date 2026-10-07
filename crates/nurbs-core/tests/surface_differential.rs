use nurbs_core::{
    curve_differential::Side,
    surface::Surface,
    surface_differential::{self, Status},
};
fn graph(a: f64, b: f64) -> Surface {
    // x=2u-1, y=2v-1, z=a*x²+b*y² in quadratic Bernstein form.
    let xy = [-1., 0., 1.];
    let square = [1., -1., 1.];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![2., 2., 2., 7., 7., 7.],
        knots_v: vec![-3., -3., -3., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![xy[i], xy[j], a * square[i] + b * square[j]])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn contains(b: [f64; 2], x: f64) {
    assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
}
#[test]
fn quadratic_graph_has_analytic_curvatures_and_principal_axes() {
    let s = graph(1., 2.);
    let before = s.clone();
    let r = surface_differential::at(&s, [4.5, -1.], [Side::Automatic; 2]).unwrap();
    assert_eq!(r.status, Status::Available);
    let k = r.principal.unwrap();
    contains(k[0], 2.);
    contains(k[1], 4.);
    contains(r.gaussian.unwrap(), 8.);
    contains(r.mean.unwrap(), 3.);
    assert!(k[0][1] - k[0][0] < 1e-8 && k[1][1] - k[1][0] < 1e-8);
    let d = r.directions.unwrap();
    // Eigenvectors may have either sign, but their other components vanish.
    assert!(d[0][0][0] > 0.999 || d[0][0][1] < -0.999);
    assert!(d[1][1][0] > 0.999 || d[1][1][1] < -0.999);
    for (axis, v) in d.into_iter().enumerate() {
        for q in 0..3 {
            if q != axis {
                contains(v[q], 0.);
                assert!(v[q][1] - v[q][0] < 1e-8);
            }
        }
    }
    assert_eq!(s, before);
    let reversed = nurbs_core::surface_edit::reverse(&s, nurbs_core::surface::Axis::U).unwrap();
    let r = surface_differential::at(&reversed, [4.5, -1.], [Side::Automatic; 2]).unwrap();
    let k = r.principal.unwrap();
    contains(k[0], -4.);
    contains(k[1], -2.);
}
#[test]
fn umbilics_planes_and_singularities_are_explicit() {
    for a in [0., 1.] {
        let r = surface_differential::at(&graph(a, a), [4.5, -1.], [Side::Automatic; 2]).unwrap();
        assert_eq!(r.status, Status::Available);
        for k in r.principal.unwrap() {
            contains(k, 2. * a);
        }
        assert!(r.directions.is_none());
    }
    let mut s = graph(0., 0.);
    for row in &mut s.control_points {
        for p in row {
            p[1] = 0.;
        }
    }
    let r = surface_differential::at(&s, [4.5, -1.], [Side::Automatic; 2]).unwrap();
    assert_eq!(r.status, Status::RegularityNotProven);
    assert!(r.principal.is_none());
}
#[test]
fn sides_and_invalid_queries_are_checked() {
    let mut s = graph(1., 2.);
    assert!(surface_differential::at(&s, [f64::NAN, -1.], [Side::Automatic; 2]).is_err());
    assert!(surface_differential::at(&s, [1., -1.], [Side::Automatic; 2]).is_err());
    assert!(surface_differential::at(&s, [2., -1.], [Side::Left, Side::Automatic]).is_err());
    assert!(surface_differential::at(&s, [7., -1.], [Side::Right, Side::Automatic]).is_err());
    s = nurbs_core::surface_edit::insert(&s, nurbs_core::surface::Axis::U, 4.5, 1).unwrap();
    assert_eq!(
        surface_differential::at(&s, [4.5, -1.], [Side::Automatic; 2])
            .unwrap()
            .status,
        Status::ContinuityNotProven
    );
    for side in [Side::Left, Side::Right] {
        let r = surface_differential::at(&s, [4.5, -1.], [side, Side::Automatic]).unwrap();
        assert_eq!(r.status, Status::Available);
        let k = r.principal.unwrap();
        contains(k[0], 2.);
        contains(k[1], 4.);
    }
}

#[test]
fn rational_cylinder_and_parameter_scaling_preserve_curvatures() {
    let c = nurbs_core::curve::Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![vec![2., 0., 0.], vec![2., 2., 0.], vec![0., 2., 0.]],
        weights: vec![1., 0.5_f64.sqrt(), 1.],
        periodic: false,
    };
    let s = nurbs_core::surface::extrude(&c, [0., 0., 3.]).unwrap();
    for t in [0., 0.125, 0.5, 0.875, 1.] {
        let r = surface_differential::at(&s, [2. + 5. * t, 0.375], [Side::Automatic; 2]).unwrap();
        assert_eq!(r.status, Status::Available);
        let k = r.principal.unwrap();
        // The stored binary64 middle weight differs from ideal sqrt(1/2).
        // Bound and compare the resulting curvature with that analytic circle.
        assert!(k[0][0] >= -0.500000001 && k[0][1] <= -0.499999999);
        contains(k[1], 0.);
        let d = r.directions.unwrap();
        assert!(d[1][2][0] > 0.999 || d[1][2][1] < -0.999);
        contains(d[1][0], 0.);
        contains(d[1][1], 0.);
    }
    let saddle =
        surface_differential::at(&graph(1., -2.), [4.5, -1.], [Side::Automatic; 2]).unwrap();
    let k = saddle.principal.unwrap();
    contains(k[0], -4.);
    contains(k[1], 2.);
    contains(saddle.gaussian.unwrap(), -8.);
}
