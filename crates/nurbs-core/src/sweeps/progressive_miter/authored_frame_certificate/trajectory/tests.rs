use super::*;
#[test]
fn orthonormal_pose_norm_keeps_affine_center_fit_and_translation_separate() {
    let law=crate::sweeps::progressive_sweep::constant_vector_law;
    let path=law([10.,20.,30.]).unwrap();let scale=law([2.,0.,0.]).unwrap();
    let axes=law([0.5,1.,0.25]).unwrap();let center=law([1.,-2.,4.]).unwrap();
    let q=[[[3.,3.],[4.,4.],[12.,12.]]];
    // Deliberately loose enclosures contain every Z rotation. Their
    // independent norms say nothing useful about the exact frame norm.
    let frame=ValuesReport {status:Status::Certified,cells:7,
        longitudinal:Some([[-100.,100.];3]),transverse:Some([[-100.,100.];3]),
        binormal:Some([[-100.,100.];3])};
    let report=control_values_with_fit(&path,&scale,Some((&axes,&center)),&q,
        [0.,1.],100,frame.clone(),Some(I::point(1.5))).unwrap();
    assert_eq!(report.status,Status::Certified);
    let value=report.values.unwrap()[0];let radius=172_f64.sqrt();
    // Original coefficients after axis/center/fit are exactly [6,6,10].
    for theta in [-std::f64::consts::PI,-1.,0.,0.7,std::f64::consts::PI] {
        let expected=[10.+6.*theta.cos()-6.*theta.sin(),
            20.+6.*theta.sin()+6.*theta.cos(),40.];
        for k in 0..3 {
            assert!(value[k][0]<=expected[k]&&expected[k]<=value[k][1]);
            assert!(value[k][1]-value[k][0]<2.*radius+1e-10);
        }
    }
    for max in [7,report.cells-1] {
        let short=control_values_with_fit(&path,&scale,Some((&axes,&center)),&q,
            [0.,1.],max,frame.clone(),Some(I::point(1.5))).unwrap();
        assert_eq!(short.status,Status::Unresolved);assert!(short.values.is_none());
        assert!(short.cells<=max);
    }
    let unresolved=ValuesReport {status:Status::Unresolved,..frame};
    let short=control_values_with_fit(&path,&scale,None,&q,[0.,1.],100,unresolved,None).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.values.is_none());
}
fn line(a: [f64; 3], b: [f64; 3], domain: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![domain[0], domain[0], domain[1], domain[1]],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}
#[test]
fn simultaneous_path_scale_affine_and_frame_jets_use_independent_domains() {
    let path = line([0.; 3], [0., 0., 10.], [2., 5.]);
    let axis = line([0., 0., 1.], [0., 0., 1.], [7., 9.]);
    let normal = line([1., 0., 0.], [1., 0., 0.], [31., 41.]);
    let twist = line([0.; 3], [0.; 3], [17., 19.]);
    let scale = line([1., 0., 0.], [2., 0., 0.], [23., 29.]);
    let axes = line([1.; 3], [2., 1., 1.], [43., 47.]);
    let center = line([0.; 3], [0.5, 0., 1.], [59., 61.]);
    let q = [[2., 2.], [3., 3.], [4., 4.]];
    let report = certify_control_trajectory(
        &path,
        &scale,
        &twist,
        &axis,
        &normal,
        Some((&axes, &center)),
        q,
        [0., 1.],
        100,
    )
    .unwrap();
    assert_eq!(report.status, Status::Certified);
    assert_eq!(report.cells, 17);
    assert_eq!(report.traversal, [0., 1.]);
    let remainder = report.linear_remainder_upper().unwrap().unwrap();
    assert!(remainder >= 0.5 && remainder < 0.5 + 1e-10);
    let combined = report.linear_error_upper([0., 0.125]).unwrap().unwrap();
    assert!(combined >= 0.625 && combined < 0.625 + 1e-10);
    assert!(report.linear_error_upper([-1., 0.]).is_err());
    let mut unproved_knots = report.clone();
    unproved_knots.single_span = false;
    assert!(unproved_knots.linear_remainder_upper().unwrap().is_none());
    // Here the independently specified original polynomial is C2 on
    // the open interval even if another source span touches an endpoint.
    let conditional=unproved_knots.linear_second_error_upper_on_open_span(
        [0.,0.125]).unwrap().unwrap();
    assert!(conditional>=0.625 && conditional<0.625+1e-10);
    assert!(unproved_knots.linear_remainder_upper().unwrap().is_none());
    // Piecewise-linear path has zero second derivative on each piece,
    // but its derivative jump gives a nonzero interpolation error. The
    // span premise prevents treating those separate jets as global C2.
    let mut kinked_path = path.clone();
    kinked_path.knots = vec![2., 2., 3.5, 5., 5.];
    kinked_path.control_points = vec![vec![0., 0., 0.], vec![0., 0., 4.], vec![0., 0., 10.]];
    kinked_path.weights = vec![1.; 3];
    let kinked = certify_control_trajectory(
        &kinked_path,
        &scale,
        &twist,
        &axis,
        &normal,
        Some((&axes, &center)),
        q,
        [0., 1.],
        100,
    )
    .unwrap();
    assert_eq!(kinked.status, Status::Certified);
    assert!(!kinked.single_span);
    assert!(kinked.linear_remainder_upper().unwrap().is_none());
    // Independent polynomial trajectory: Qx=2+4.5t+2t².
    // Retained endpoints are displaced in opposite directions; verify
    // the first-derivative bound against the actual chord error.
    let endpoints=[[2.125,3.,4.],[8.25,6.,19.]];
    let first=report.linear_first_error_upper_on_continuous_span(
        endpoints,[0.125,0.25]).unwrap().unwrap();
    for i in 0..=128 {
        let t=i as f64/128.;
        let original=[2.+4.5*t+2.*t*t,3.+3.*t,4.+15.*t];
        let error:[f64;3]=std::array::from_fn(|k|
            original[k]-((1.-t)*endpoints[0][k]+t*endpoints[1][k]));
        assert!(error.iter().map(|v|v*v).sum::<f64>().sqrt()<=first);
    }
    let mut refused=report.clone();
    refused.status=Status::Unresolved;
    assert!(refused.linear_first_error_upper_on_continuous_span(
        endpoints,[0.125,0.25]).unwrap().is_none());
    assert!(report.linear_first_error_upper_on_continuous_span(
        endpoints,[-1.,0.]).is_err());
    let jet = report.jet.unwrap();
    for t in [0., 0.375, 1.] {
        let endpoint = certify_control_value(
            &path,
            &scale,
            &twist,
            &axis,
            &normal,
            Some((&axes, &center)),
            q,
            [t, t],
            100,
        )
        .unwrap();
        assert_eq!(endpoint.status, Status::Certified);
        assert_eq!(endpoint.cells, 17);
        let value = endpoint.value.unwrap();
        let expected = [2. + 4.5 * t + 2. * t * t, 3. + 3. * t, 4. + 15. * t];
        for k in 0..3 {
            assert!(value[k][0] <= expected[k] && expected[k] <= value[k][1]);
            assert!(value[k][1] - value[k][0] < 1e-10);
        }
        assert!(
            endpoint
                .retained_displacement_upper(expected)
                .unwrap()
                .unwrap()
                < 1e-10
        );
        let mut damaged = expected;
        damaged[0] += 0.125;
        let upper = endpoint
            .retained_displacement_upper(damaged)
            .unwrap()
            .unwrap();
        assert!(upper >= 0.125 && upper < 0.125 + 1e-10);
    }
    for i in 0..=16 {
        let t = i as f64 / 16.;
        for (bounds, point) in [
            (
                jet.value,
                [2. + 4.5 * t + 2. * t * t, 3. + 3. * t, 4. + 15. * t],
            ),
            (jet.first, [4.5 + 4. * t, 3., 15.]),
            (jet.second, [4., 0., 0.]),
        ] {
            for k in 0..3 {
                assert!(bounds[k][0] <= point[k] && point[k] <= bounds[k][1]);
            }
        }
    }
    for budget in [0, 16] {
        let report = certify_control_trajectory(
            &path,
            &scale,
            &twist,
            &axis,
            &normal,
            Some((&axes, &center)),
            q,
            [0., 1.],
            budget,
        )
        .unwrap();
        assert_eq!(report.status, Status::Unresolved);
        assert!(report.jet.is_none() && report.cells <= budget);
        assert!(report.linear_remainder_upper().unwrap().is_none());
        let endpoint = certify_control_value(
            &path,
            &scale,
            &twist,
            &axis,
            &normal,
            Some((&axes, &center)),
            q,
            [1., 1.],
            budget,
        )
        .unwrap();
        assert_eq!(endpoint.status, Status::Unresolved);
        assert!(endpoint.value.is_none() && endpoint.cells <= budget);
        assert!(
            endpoint
                .retained_displacement_upper([0.; 3])
                .unwrap()
                .is_none()
        );
    }
}
