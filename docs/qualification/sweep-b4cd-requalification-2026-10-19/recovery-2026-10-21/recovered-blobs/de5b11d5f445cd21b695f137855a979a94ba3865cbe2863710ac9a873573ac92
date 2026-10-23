use nurbs_core::{
    curve::Curve,
    curve_surface_agreement::{self, LiftMethod, Status},
    surface::Surface,
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn quintic_tensor_diagonal_lifts_with_owned_ends_and_whole_original_agreement() {
    let choose=|n:usize,k:usize| {let mut v=1.;for i in 0..k {v*= (n-i) as f64/(i+1) as f64;}v};
    let graph=|degree:usize|Surface {
        degree_u:degree,degree_v:degree,
        knots_u:[vec![0.;degree+1],vec![1.;degree+1]].concat(),
        knots_v:[vec![0.;degree+1],vec![1.;degree+1]].concat(),
        control_points:(0..=degree).map(|i|(0..=degree).map(|j|vec![i as f64/degree as f64,j as f64/degree as f64,
            if i<2 {0.}else{choose(i,2)/choose(degree,2)} + if j<3 {0.}else{choose(j,3)/choose(degree,3)*0.25}]).collect()).collect(),
        weights:vec![vec![1.;degree+1];degree+1],periodic_u:false,periodic_v:false,
    };
    let s=graph(5);let p=Curve::from_polyline(vec![vec![0.,0.],vec![1.,1.]]).unwrap();
    let before=(s.clone(),p.clone());
    let r=curve_surface_agreement::lift(&p,&s,Some([[0.;3],[1.,1.,1.25]]),1e-7,100000).unwrap();
    assert_eq!(r.method,Some(LiftMethod::TensorComposition));
    assert_eq!(r.evidence.unwrap().status,Status::WithinTolerance);
    let c=r.curve.unwrap();assert_eq!(c.degree,10);
    assert_eq!(c.control_points[0],vec![0.;3]);assert_eq!(c.control_points.last().unwrap(),&vec![1.,1.,1.25]);
    let bad=curve_surface_agreement::lift(&p,&s,Some([[0.,0.,0.01],[1.,1.,1.25]]),1e-7,100000).unwrap();
    assert!(bad.curve.is_none());
    let empty=curve_surface_agreement::lift(&p,&s,None,1e-7,0).unwrap();assert!(empty.curve.is_none());
    assert!(curve_surface_agreement::lift(&p,&graph(9),None,1e-7,100000).unwrap().curve.is_none());
    assert_eq!((s,p),before);
}
#[test]
fn affine_original_chart_lifts_general_rational_quintics_with_fresh_endpoint_checks() {
    let s=Surface {degree_u:1,degree_v:1,knots_u:vec![-2.,-2.,3.,3.],knots_v:vec![5.,5.,9.,9.],
        control_points:vec![vec![vec![10.,20.,30.],vec![10.,26.,32.]],vec![vec![18.,22.,30.],vec![18.,28.,32.]]],
        weights:vec![vec![2.;2];2],periodic_u:false,periodic_v:false};
    let p=Curve {degree:5,knots:vec![0.;6].into_iter().chain(vec![1.;6]).collect(),
        control_points:vec![vec![-2.,5.],vec![-1.,5.5],vec![0.,6.],vec![1.,8.],vec![2.,8.5],vec![3.,9.]],
        weights:vec![1.,0.7,0.9,1.2,1.,1.],periodic:false};
    let result=curve_surface_agreement::lift(&p,&s,None,1e-8,10000).unwrap();
    assert_eq!(result.method,Some(LiftMethod::AffineProposal));
    assert_eq!(result.evidence.unwrap().status,Status::WithinTolerance);
    let c=result.curve.unwrap();
    assert_eq!(c.weights,p.weights);assert_eq!(c.knots,p.knots);assert_eq!(c.degree,5);
    for q in [0.,0.17,0.5,0.83,1.] {
        let uv=p.evaluate(q).unwrap().point;let actual=c.evaluate(q).unwrap().point;
        let expected=s.evaluate(uv[0],uv[1]).unwrap().point;
        assert!((0..3).all(|k|(actual[k]-expected[k]).abs()<1e-10));
    }
    let bad=curve_surface_agreement::lift(&p,&s,Some([[10.,20.,30.1],[18.,28.,32.1]]),1e-8,10000).unwrap();
    assert!(bad.curve.is_none());
}
#[test]
fn rational_chart_trace_and_actual_endpoint_edits_require_continuous_admission() {
    let s = plane();
    let mut p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
    p.weights[1] = 0.5;
    let before = (s.clone(), p.clone());
    let r = curve_surface_agreement::lift(&p, &s, None, 1e-8, 1000).unwrap();
    assert!(r.curve.is_some());
    assert_eq!(r.evidence.unwrap().status, Status::WithinTolerance);
    let bad =
        curve_surface_agreement::lift(&p, &s, Some([[0., 0., 0.1], [1., 1., 0.1]]), 1e-8, 1000)
            .unwrap();
    assert!(bad.curve.is_none());
    let empty = curve_surface_agreement::lift(&p, &s, None, 1e-8, 0).unwrap();
    assert!(empty.curve.is_none() && empty.cells == 0);
    assert_eq!((s, p), before);
}
#[test]
fn rational_multiple_span_iso_proposal_is_checked_against_the_original_trace() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 0.5, 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.25], vec![0.5, 1., 0.25]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1., 0.5], vec![0.75, 1.], vec![1., 0.5]],
        periodic_u: false,
        periodic_v: false,
    };
    let p = Curve::from_polyline(vec![vec![0., 0.3], vec![1., 0.3]]).unwrap();
    let original = s.clone();
    let r = curve_surface_agreement::lift(&p, &s, None, 1e-8, 10000).unwrap();
    assert_eq!(r.method, Some(LiftMethod::IsoparametricProposal));
    assert!(r.curve.is_some());
    assert_eq!(r.evidence.unwrap().status, Status::WithinTolerance);
    assert_eq!(s, original);
}
