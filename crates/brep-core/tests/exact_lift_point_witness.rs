use brep_core::{boundary_agreement, cuboid};
use cad_predicates::BezierIdentity;
use nurbs_core::curve::Curve;
#[test]
fn unsupported_full_lift_keeps_a_located_exact_counterexample_and_shared_work() {
    let mut model=cuboid([0.;3],[1.;3]).unwrap();
    let face=0;
    let wire=model.faces[face].outer;
    let original=model.loops[wire].coedges[0].pcurve.clone();
    let a=original.control_points.first().unwrap();
    let b=original.control_points.last().unwrap();
    let fixed=(0..2).find(|&k|a[k]==b[k]).unwrap();
    let controls=(0..=9).map(|i| {
        let t=i as f64/9.;
        let mut p=(0..2).map(|k|a[k]*(1.-t)+b[k]*t).collect::<Vec<_>>();
        if i>0&&i<9 {p[fixed]+=if a[fixed]==0. {0.125}else{-0.125};}
        p
    }).collect();
    let domain=original.domain();
    model.loops[wire].coedges[0].pcurve=Curve {degree:9,knots:[vec![domain[0];10],vec![domain[1];10]].concat(),control_points:controls,weights:vec![1.;10],periodic:false};
    let before=model.clone();
    let report=boundary_agreement::verify_exact(&model,1_000_000).unwrap();
    assert!(!report.all_equal);assert!(report.has_exact_counterexample());assert!(report.work<=1_000_000);
    let use_report=report.uses.iter().find(|u|u.face==face&&u.wire==wire&&u.coedge==0).unwrap();
    assert!(use_report.decision.is_none());
    let witness=use_report.point_witness.as_ref().unwrap();
    assert_eq!(witness.normalized_parameter,0.5);
    assert_eq!(witness.result.point_equality,BezierIdentity::Different);
    assert!(witness.result.work_used>0&&witness.result.work_used<=report.work);
    let short=boundary_agreement::verify_exact(&model,report.work-1).unwrap();
    assert!(!short.all_equal&&short.work<=report.work-1);
    assert_eq!(model,before);
}
