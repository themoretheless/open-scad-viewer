use nurbs_core::{surface::Surface,geodesic::{self,GeodesicStop}};
fn plane()->Surface {Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],control_points:vec![vec![vec![0.,0.,0.],vec![0.,3.,0.]],vec![vec![2.,0.,0.],vec![2.,3.,0.]]],weights:vec![vec![1.,1.];2],periodic_u:false,periodic_v:false}}
#[test]
fn direction_scale_does_not_change_the_world_geodesic() {
    let s=plane();
    for scale in [1.,1e300,1e-300] {
        let r=geodesic::trace_geodesic_report(&s,[0.25,0.25],[scale,scale],0.5,4096).unwrap();
        assert_eq!(r.stop,GeodesicStop::LengthReached);
        let uv=*r.uv_polyline.last().unwrap();let p=s.evaluate(uv[0],uv[1]).unwrap().point;
        let expected=[0.5+1./13_f64.sqrt(),0.75+1.5/13_f64.sqrt(),0.];
        for k in 0..3 {assert!((p[k]-expected[k]).abs()<1e-10,"{p:?} vs {expected:?}");}
        assert!((r.length_achieved-0.5).abs()<1e-12);
    }
}
#[test]
fn boundary_stop_reports_retained_length_not_full_rk_step() {
    let s=plane();
    for (start,dir,expected,end) in [([0.25,0.5],[1.,0.],1.5,[1.,0.5]),([0.75,0.5],[-1.,0.],1.5,[0.,0.5]),([0.5,0.25],[0.,1.],2.25,[0.5,1.])] {
        let r=geodesic::trace_geodesic_report(&s,start,dir,4.,4096).unwrap();
        assert_eq!(r.stop,GeodesicStop::BoundaryHit);
        assert!((r.length_achieved-expected).abs()<1e-10,"{} vs {expected}",r.length_achieved);
        let uv=*r.uv_polyline.last().unwrap();for k in 0..2 {assert!((uv[k]-end[k]).abs()<1e-12);}
        assert!(r.uv_polyline.iter().all(|p|p.iter().all(|v|*v>=0.&&*v<=1.)));
    }
}
#[test]
fn budgets_and_invalid_requests_keep_explicit_outcomes() {
    let s=plane();let r=geodesic::trace_geodesic_report(&s,[0.5;2],[1.,0.],0.5,1).unwrap();
    assert_eq!(r.stop,GeodesicStop::BudgetExhausted);assert!(r.length_achieved<0.5);
    for dir in [[0.;2],[f64::NAN,1.]] {assert!(geodesic::trace_geodesic_report(&s,[0.5;2],dir,1.,100).is_err());}
    for len in [0.,-1.,f64::INFINITY,f64::from_bits(1)] {assert!(geodesic::trace_geodesic_report(&s,[0.5;2],[1.,0.],len,100).is_err());}
    for b in [0,4097] {assert!(geodesic::trace_geodesic_report(&s,[0.5;2],[1.,0.],1.,b).is_err());}
    assert!(geodesic::trace_geodesic_report(&s,[-0.1,0.5],[1.,0.],1.,100).is_err());
}

#[test]
fn very_long_request_keeps_a_representable_short_boundary_path() {
    let r=geodesic::trace_geodesic_report(&plane(),[0.25,0.5],[1.,0.],1e100,4096).unwrap();
    assert_eq!(r.stop,GeodesicStop::BoundaryHit);
    assert!((r.length_achieved-1.5).abs()<1e-10,"{r:?}");
    let end=*r.uv_polyline.last().unwrap();assert!((end[0]-1.).abs()<1e-12);
}
