use super::*;
fn contains(v: VectorEnclosure, p: [f64; 3]) {
    for (range, x) in v.into_iter().zip(p) {
        assert!(range[0] <= x && x <= range[1], "{x} outside {range:?}");
    }
}
#[test]
fn authored_remainder_covers_rotation_and_axial_miter_scale_product() {
    let constant=|v:[f64;3]|crate::paths::bezier(vec![v.to_vec(),v.to_vec()],None).unwrap();
    let scale=constant([1.,0.,0.]);let longitudinal=constant([0.,0.,1.]);let transverse=constant([1.,0.,0.]);
    let twist=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.]],None).unwrap();
    let straight=certify(&[[0.,0.,0.],[0.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
    let bound=straight.interpolation_upper_authored_frame(0,[0.,1.],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,8).unwrap().unwrap();
    let actual=((0.5_f64.cos()-(1.+1_f64.cos())/2.).powi(2)+(0.5_f64.sin()-1_f64.sin()/2.).powi(2)).sqrt();
    assert!(bound>=actual && bound<0.18);
    let refined=straight.interpolation_upper_authored_frame(0,[0.,0.5],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,8).unwrap().unwrap();
    assert!(refined<bound*0.3);
    let kinked=crate::curve::Curve {degree:1,knots:vec![0.,0.,0.5,1.,1.],control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;3],periodic:false};
    let frame=super::super::authored_frame_certificate::certify_twisted(&longitudinal,&transverse,&kinked,[0.,1.],8).unwrap();
    assert!(!frame.single_span);
    let kink_bound=straight.interpolation_upper_authored_frame(0,[0.,1.],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&kinked,9).unwrap().unwrap();
    // The midpoint has angle1 while both endpoints have angle0; crossing
    // the derivative jump requires the first-derivative remainder.
    assert!(kink_bound>=2.*0.5_f64.sin());
    let corner=certify(&[[0.,0.,0.],[0.,0.,10.],[10.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
    let longitudinal=constant([1.,0.,0.]);let transverse=constant([0.,0.,1.]);let twist=constant([0.,0.,0.]);
    let scale=crate::paths::bezier(vec![vec![1.,0.,0.],vec![2.,0.,0.]],None).unwrap();
    let bound=corner.interpolation_upper_authored_frame(0,[0.,1.],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,8).unwrap().unwrap();
    assert!(bound>=0.125 && bound<(2_f64.sqrt()/8.+1e-10),"{bound}");
    assert!(corner.interpolation_upper_authored_frame(0,[0.,1.],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,7).unwrap().is_none());
}
#[test]
fn authored_station_keeps_path_projection_and_one_law_budget() {
    let r=certify(&[[0.,0.,0.],[0.,0.,10.],[10.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
    let constant=|v:[f64;3]|crate::paths::bezier(vec![v.to_vec(),v.to_vec()],None).unwrap();
    let longitudinal=constant([1.,0.,0.]);
    let transverse=constant([0.,0.,1.]);
    let scale=constant([1.,0.,0.]);let twist=constant([0.,0.,0.]);
    let p=r.station_local_authored_frame(0,[0.5,0.5],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,8).unwrap().unwrap();
    // q is axial to the path but transverse to the authored frame.
    // At the halfway station the end-plane miter subtracts half of q.
    contains(p,[0.,0.,5.5]);
    assert!(r.station_local_authored_frame(0,[0.5,0.5],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,7).unwrap().is_none());
    assert!(r.station_local_authored_frame(0,[0.5,0.5],[[1.,1.],[0.,0.],[0.1,0.1]],&longitudinal,&transverse,&scale,&twist,8).is_err());
}
#[test]
fn independent_right_angle_minimum_rotation() {
    let r = certify(
        &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.]],
        [1., 0., 0.],
        false,
        2.,
        10,
    )
    .unwrap();
    assert_eq!(r.status, Status::Certified);
    contains(r.tangents.as_ref().unwrap()[0], [0., 0., 1.]);
    contains(r.tangents.as_ref().unwrap()[1], [1., 0., 0.]);
    contains(r.normals.as_ref().unwrap()[0], [1., 0., 0.]);
    contains(r.normals.as_ref().unwrap()[1], [0., 0., -1.]);
    contains(
        r.planes.as_ref().unwrap()[1].unwrap(),
        [
            std::f64::consts::FRAC_1_SQRT_2,
            0.,
            std::f64::consts::FRAC_1_SQRT_2,
        ],
    );
}
#[test]
fn skew_closed_transport_encloses_holonomy_angle() {
    let points = [
        [0., 0., 0.],
        [10., 0., 0.],
        [10., 10., 4.],
        [0., 10., 1.],
        [0., 5., -2.],
    ];
    let r = certify(&points, [0., 0., 1.], true, 4., 32).unwrap();
    assert_eq!(r.status, Status::Certified);
    assert!(r.closing_normal.is_some());
    let angle = r.holonomy_angle.unwrap();
    assert!(angle[0].abs() > 1e-3 && angle[1] - angle[0] < 1e-8);
    assert_eq!(r.tangents.unwrap().len(), 5);
    assert_eq!(r.normals.unwrap().len(), 5);
    assert!(r.planes.unwrap()[0].is_some());
    let exhausted = certify(&points, [0., 0., 1.], true, 4., 11).unwrap();
    assert_eq!(exhausted.status, Status::Unresolved);
    assert_eq!(exhausted.reason, Some("cell-budget-exhausted"));
    assert!(exhausted.holonomy_angle.is_none() && exhausted.closing_normal.is_none());
}
#[test]
fn degeneracy_limits_and_budget_discard_partial_frames() {
    for (points, normal, limit, budget) in [
        (vec![[0., 0., 0.], [0., 0., 0.]], [1., 0., 0.], 4., 10),
        (vec![[0., 0., 0.], [0., 0., 1.]], [0., 0., 1.], 4., 10),
        (
            vec![[0., 0., 0.], [0., 0., 1.], [0., 0., 0.]],
            [1., 0., 0.],
            4.,
            10,
        ),
        (
            vec![[0., 0., 0.], [0., 0., 1.], [1., 0., 1.]],
            [1., 0., 0.],
            1.1,
            10,
        ),
        (vec![[0., 0., 0.], [0., 0., 1.]], [1., 0., 0.], 4., 0),
    ] {
        let r = certify(&points, normal, false, limit, budget).unwrap();
        assert_eq!(r.status, Status::Unresolved);
        assert!(r.tangents.is_none() && r.normals.is_none() && r.planes.is_none());
    }
}
#[test]
fn traversal_encloses_exact_rational_chord_ratios() {
    let r = certify(
        &[[0., 0., 0.], [3., 0., 0.], [3., 4., 0.]],
        [0., 0., 1.],
        false,
        2.,
        10,
    )
    .unwrap();
    let t = r.traversal(1, [0.5, 0.5]).unwrap().unwrap();
    assert!(t[0] <= 5. / 7. && t[1] >= 5. / 7. && t[1] - t[0] < 1e-12);
    let c = r.correction_at(1, [0., 1.]).unwrap().unwrap();
    assert!(c[0] <= 0. && c[1] >= 0.);
    assert!(r.traversal(2, [0., 1.]).is_err());
    assert!(r.traversal(usize::MAX, [0., 1.]).is_err());
    assert!(r.traversal(0, [-0.1, 1.]).is_err());
    let u = certify(&[[0., 0., 0.], [3., 0., 0.]], [0., 0., 1.], false, 2., 0).unwrap();
    assert!(u.correction_at(0, [0., 1.]).unwrap().is_none());
}
#[test]
fn rotated_frame_encloses_exact_axes_and_continuous_twist_range() {
    let r = certify(&[[0., 0., 0.], [0., 0., 10.]], [1., 0., 0.], false, 4., 10).unwrap();
    let (n, b) = r.rotated_frame(0, [0.25, 0.75], [0., 0.]).unwrap().unwrap();
    contains(n, [1., 0., 0.]);
    contains(b, [0., 1., 0.]);
    let (n, b) = r.rotated_frame(0, [0., 1.], [-0.2, 0.3]).unwrap().unwrap();
    for i in 0..101 {
        let a: f64 = -0.2 + 0.5 * i as f64 / 100.;
        contains(n, [a.cos(), a.sin(), 0.]);
        contains(b, [-a.sin(), a.cos(), 0.]);
    }
    let u = certify(&[[0., 0., 0.], [0., 0., 10.]], [1., 0., 0.], false, 4., 0).unwrap();
    assert!(u.rotated_frame(0, [0., 1.], [0., 1.]).unwrap().is_none());
}
#[test]
fn miter_shear_encloses_exact_corner_plane_and_open_endpoint() {
    let r = certify(
        &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.]],
        [1., 0., 0.],
        false,
        2.,
        10,
    )
    .unwrap();
    let q = [[1., 1.], [0., 0.], [0., 0.]];
    contains(
        r.miter_offset(0, [0., 0.], q).unwrap().unwrap(),
        [1., 0., 0.],
    );
    contains(
        r.miter_offset(0, [1., 1.], q).unwrap().unwrap(),
        [1., 0., -1.],
    );
    let range = r.miter_offset(0, [0.2, 0.8], q).unwrap().unwrap();
    for i in 0..101 {
        let f = 0.2 + 0.6 * i as f64 / 100.;
        contains(range, [1., 0., -f]);
    }
    // The outgoing transported offset reaches exactly the same plane.
    contains(
        r.miter_offset(1, [0., 0.], [[0., 0.], [0., 0.], [-1., -1.]])
            .unwrap()
            .unwrap(),
        [1., 0., -1.],
    );
    let u = certify(&[[0., 0., 0.], [0., 0., 10.]], [1., 0., 0.], false, 2., 0).unwrap();
    assert!(u.miter_offset(0, [0., 1.], q).unwrap().is_none());
}
#[test]
fn station_composition_preserves_translated_profile_and_corner() {
    let r = certify(
        &[[100., 200., 300.], [100., 200., 310.], [110., 200., 310.]],
        [1., 0., 0.],
        false,
        2.,
        10,
    )
    .unwrap();
    contains(
        r.station_point(0, [1., 1.], [101., 202., 300.], [2., 2.], [0., 0.])
            .unwrap()
            .unwrap(),
        [102., 204., 308.],
    );
    let box_range = r
        .station_point(0, [0.2, 0.8], [101., 202., 300.], [1., 2.], [0., 0.])
        .unwrap()
        .unwrap();
    for f in [0.2, 0.5, 0.8] {
        for scale in [1., 1.5, 2.] {
            contains(
                box_range,
                [100. + scale, 200. + 2. * scale, 300. + 10. * f - f * scale],
            );
        }
    }
    assert!(
        r.station_point(0, [0., 0.], [101., 202., 300.], [0., 1.], [0., 0.])
            .is_err()
    );
}
#[test]
fn stored_error_bound_covers_ideal_box_and_displaced_point() {
    let ideal = [[0., 0.], [0., 0.], [0., 0.]];
    let bound = stored_point_error_upper([3., 4., 0.], ideal).unwrap();
    assert!(bound >= 5. && bound < 5.000000000001);
    let bound =
        stored_point_error_upper([0., 0., 0.], [[-3., 2.], [-4., 1.], [0., 0.]]).unwrap();
    assert!(bound >= 5. && bound < 5.000000000001);
    assert!(stored_point_error_upper([f64::NAN, 0., 0.], ideal).is_err());
}
#[test]
fn affine_station_projects_the_same_axis_scale_and_center_into_corner_plane(){
    let r=certify(&[[0.,0.,0.],[0.,0.,10.],[10.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
    let axes=[[2.,2.],[3.,3.],[1.,1.]];
    let center=[[0.5,0.5],[-1.,-1.],[2.,2.]];
    // q=(4.5,11,2), projection onto x+z=10 removes q.x+q.z.
    contains(r.station_point_affine(0,[1.,1.],[1.,2.,0.],[2.,2.],[0.,0.],axes,center).unwrap().unwrap(),[4.5,11.,5.5]);
    // At the open endpoint there is no miter projection.
    contains(r.station_point_affine(0,[0.,0.],[1.,2.,0.],[2.,2.],[0.,0.],axes,center).unwrap().unwrap(),[4.5,11.,2.]);
    assert!(r.station_point_affine(0,[0.,0.],[1.,2.,0.],[2.,2.],[0.,0.],[[0.,1.];3],center).is_err());
    let vector=|value:[f64;3]|crate::curve::Curve {degree:1,knots:vec![2.,2.,5.,5.],control_points:vec![value.to_vec();2],weights:vec![1.;2],periodic:false};
    let scalar=|value:f64|vector([value,0.,0.]);
    let axes=vector([2.,3.,1.]);let center=vector([0.5,-1.,2.]);
    contains(r.station_with_affine_laws(0,[1.,1.],[1.,2.,0.],&scalar(2.),&scalar(0.),Some((&axes,&center)),6).unwrap().unwrap(),[4.5,11.,5.5]);
    assert!(r.station_with_affine_laws(0,[1.,1.],[1.,2.,0.],&scalar(2.),&scalar(0.),Some((&axes,&center)),5).unwrap().is_none());
}
#[test]
fn ordinary_affine_endpoint_uses_values_without_zero_width_jets() {
    let scalar=|v|crate::curve::Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![v,0.,0.];2],weights:vec![1.;2],periodic:false};
    let axes=crate::curve::Curve{control_points:vec![vec![2.,1.,1.],vec![3.,2.,1.]],..scalar(1.)};
    let center=crate::curve::Curve{control_points:vec![vec![0.125,0.,0.],vec![0.5,0.25,0.125]],..scalar(0.)};
    let frame=certify(&[[0.,0.,0.],[0.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
    for (t,expected) in [(0.,[2.125,2.,0.]),(1.,[3.5,4.25,10.125])] {
        contains(frame.station_with_affine_laws(0,[t,t],[1.,2.,0.],&scalar(1.),&scalar(0.),Some((&axes,&center)),6).unwrap().unwrap(),expected);
        assert!(frame.station_with_affine_laws(0,[t,t],[1.,2.,0.],&scalar(1.),&scalar(0.),Some((&axes,&center)),5).unwrap().is_none());
    }
}
#[test]
fn affine_remainder_covers_center_curvature_and_axial_projection_derivative(){
    let scalar=|value:f64|crate::curve::Curve {degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![value,0.,0.];2],weights:vec![1.;2],periodic:false};
    let axes=crate::curve::Curve {control_points:vec![vec![1.,1.,1.];2],..scalar(1.)};
    let center=crate::curve::Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],control_points:vec![vec![0.,0.,0.],vec![0.,0.,0.],vec![0.,0.,2.]],weights:vec![1.;3],periodic:false};
    let r=certify(&[[0.,0.,0.],[0.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
    let bound=r.interpolation_upper_affine(0,[0.,1.],[1.,0.,0.],&scalar(1.),&scalar(0.),Some((&axes,&center)),6).unwrap().unwrap();
    assert!(bound>=0.5 && bound<0.50000000001);
    for i in 0..=100 {let f=i as f64/100.;assert!((2.*f*f-2.*f).abs()<=bound);}
    assert!(r.interpolation_upper_affine(0,[0.,1.],[1.,0.,0.],&scalar(1.),&scalar(0.),Some((&axes,&center)),5).unwrap().is_none());
    // Collinear interior station still has a normal miter plane. A varying
    // axial center is projected away there but remains at the open site.
    let r=certify(&[[0.,0.,0.],[0.,0.,10.],[0.,0.,20.]],[1.,0.,0.],false,2.,10).unwrap();
    let center=crate::curve::Curve {control_points:vec![vec![0.,0.,0.],vec![0.,0.,4.]],..scalar(0.)};
    let bound=r.interpolation_upper_affine(0,[0.,1.],[1.,0.,0.],&scalar(1.),&scalar(0.),Some((&axes,&center)),6).unwrap().unwrap();
    assert!(bound>=0.5 && bound<0.50000000001);
    for i in 0..=100 {let f=i as f64/100.;assert!((2.*f*(1.-f)).abs()<=bound);}
}
#[test]
fn integrated_station_laws_preserve_budget_refusal() {
    let r = certify(&[[0., 0., 0.], [0., 0., 10.]], [1., 0., 0.], false, 4., 10).unwrap();
    let scale = crate::paths::bezier(vec![vec![1., 0., 0.], vec![2., 0., 0.]], None).unwrap();
    let twist = crate::paths::bezier(vec![vec![0., 0., 0.], vec![0., 0., 0.]], None).unwrap();
    let point = r
        .station_with_laws(0, [0.5, 0.5], [1., 2., 0.], &scale, &twist, 1)
        .unwrap()
        .unwrap();
    contains(point, [1.5, 3., 5.]);
    assert!(
        r.station_with_laws(0, [0.5, 0.5], [1., 2., 0.], &scale, &twist, 0)
            .unwrap()
            .is_none()
    );
}
#[test]
fn interpolation_certificate_bounds_sinusoidal_midpoint_error() {
    let r = certify(&[[0., 0., 0.], [0., 0., 10.]], [1., 0., 0.], false, 4., 10).unwrap();
    let scale = crate::paths::bezier(vec![vec![1., 0., 0.], vec![1., 0., 0.]], None).unwrap();
    let twist = crate::paths::bezier(vec![vec![0., 0., 0.], vec![1., 0., 0.]], None).unwrap();
    let e = r
        .interpolation_upper(0, [0., 1.], [1., 0., 0.], &scale, &twist, 1)
        .unwrap()
        .unwrap();
    let actual = ((0.5_f64.cos() - (1. + 1_f64.cos()) / 2.).powi(2)
        + (0.5_f64.sin() - 1_f64.sin() / 2.).powi(2))
    .sqrt();
    assert!(e >= actual && e < 0.126);
    assert!(
        r.interpolation_upper(0, [0., 1.], [1., 0., 0.], &scale, &twist, 0)
            .unwrap()
            .is_none()
    );
}
