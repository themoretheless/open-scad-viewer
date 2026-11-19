use super::*;
#[test]
fn closed_guided_frames_own_both_seams_independent_speed_and_joint_cover(){
    let path=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),
        control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],
            vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
    let mut guide=path.clone();guide.knots=[vec![-3.;8],vec![7.;8]].concat();
    for p in &mut guide.control_points {p[2]=1.;}
    let profile=crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=Curve {degree:5,knots:[vec![7.;6],vec![9.;6]].concat(),
        control_points:[0.,0.125,0.25,-0.25,-0.125,0.].into_iter().map(|x|vec![x,0.,0.]).collect(),weights:vec![1.;6],periodic:false};
    for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {for order in [1,2] {
        let opts=Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing,initial_sections:5,max_sections:17,max_deviation:1.};
        let audit=|g:&Curve,cells,work|Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_orientation_guide(g).unwrap()
            .certify_closed_guided_frame_smoothness(order,cells,work).unwrap();
        let proof=audit(&guide,10000,1000000);
        assert!(proof.closed_source_frame_smoothness_certified,"{spacing:?}/{order}/{proof:?}");
        assert!(proof.cells>0&&proof.exact_work>0);
        assert!(!audit(&guide,proof.cells-1,1000000).closed_source_frame_smoothness_certified);
        assert!(!audit(&guide,10000,proof.exact_work-1).closed_source_frame_smoothness_certified);
        assert!(!audit(&guide,0,1000000).closed_source_frame_smoothness_certified);
        assert!(!audit(&guide,10000,0).closed_source_frame_smoothness_certified);
        assert!(!audit(&path,10000,1000000).closed_source_frame_smoothness_certified);
        let mut damaged=guide.clone();damaged.control_points[2][2]=1_f64.next_up();
        assert_eq!(audit(&damaged,10000,1000000).closed_source_frame_smoothness_certified,order==1);
        let stopped=constant_vector_law([0.,0.,1.]).unwrap();
        let stopped_sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_orientation_guide(&stopped);
        let stopped_proof=stopped_sweep.and_then(|s|s.certify_closed_guided_frame_smoothness(order,10000,1000000));
        assert!(stopped_proof.is_err()||!stopped_proof.unwrap().closed_source_frame_smoothness_certified);
    }}
}
#[test]
#[cfg(feature="transport")]
fn closed_guided_frame_transport_preserves_scope_and_partial_budget_refusal(){
    let path=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),
        control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],
            vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
    let profile=crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let mut guide=path.clone();for p in &mut guide.control_points {p[2]=1.;}
    let request=value_codec::json!({"op":"surface_progressive_sweep_closed_guided_frame_smoothness",
        "profiles":[profile],"path":path,"orientation_guide":guide,"scale":scale,"twist":twist,"normal":[0.,0.,1.],
        "orientation":"rmf","spacing":"parameter","initial_sections":3,"max_sections":17,"max_deviation":1.,
        "order":2,"maxCells":10000,"maxExactWork":1000000});
    let proof=crate::transport::dispatch(request.clone()).unwrap();
    assert_eq!(proof["closedSourceFrameSmoothnessCertified"],true);
    assert_eq!(proof["scope"],"closed-original-guided-frame-only");
    for key in ["pathSeamCertified","retainedSeamsCertified","profileJoinsCertified","capJoinsCertified","continuousBound","solidCertified"] {
        assert_eq!(proof[key],false);
    }
    for (field,value) in [("maxCells",0),("maxExactWork",0),
        ("maxCells",proof["cells"].as_u64().unwrap()-1),("maxExactWork",proof["exactWork"].as_u64().unwrap()-1)] {
        let mut short=request.clone();short[field]=value_codec::json!(value);
        assert_eq!(crate::transport::dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],false);
    }
}
#[test]
#[cfg(feature="transport")]
fn closed_path_frame_transport_preserves_scope_and_partial_budget_refusal(){
    let path=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),
        control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],
            vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
    let profile=crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let request=value_codec::json!({"op":"surface_progressive_sweep_closed_path_frame_smoothness",
        "profiles":[profile],"path":path,"scale":scale,"twist":twist,"normal":[0.,0.,1.],
        "orientation":"rmf","spacing":"parameter","initial_sections":3,"max_sections":17,"max_deviation":1.,
        "order":2,"maxCells":10000,"maxExactWork":1000000});
    let proof=crate::transport::dispatch(request.clone()).unwrap();
    assert_eq!(proof["closedSourceFrameSmoothnessCertified"],true);
    assert_eq!(proof["scope"],"closed-original-path-frame-only");
    for key in ["pathSeamCertified","retainedSeamsCertified","profileJoinsCertified","capJoinsCertified","continuousBound","solidCertified"] {
        assert_eq!(proof[key],false);
    }
    for (field,value) in [("maxCells",0),("maxExactWork",0),
        ("maxCells",proof["cells"].as_u64().unwrap()-1),("maxExactWork",proof["exactWork"].as_u64().unwrap()-1)] {
        let mut short=request.clone();short[field]=value_codec::json!(value);
        assert_eq!(crate::transport::dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],false);
    }
}
#[test]
fn closed_path_frames_need_exact_c3_seam_full_cover_and_planar_transport(){
    let profile=crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=Curve {degree:5,knots:[vec![7.;6],vec![9.;6]].concat(),
        control_points:[0.,0.125,0.25,-0.25,-0.125,0.].into_iter().map(|x|vec![x,0.,0.]).collect(),
        weights:vec![1.;6],periodic:false};
    let path=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),
        control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],
            vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
    let rational=Curve {degree:9,knots:[vec![2.;10],vec![5.;10]].concat(),
        control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],vec![2.,4.,0.],
            vec![-2.,4.,0.],vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],
        weights:vec![1.,1.,1.,1.,2.,2.,1.,1.,1.,1.],periodic:false};
    for path in [&path,&rational] {
    for orientation in [Orientation::FixedNormal,Orientation::RotationMinimizing,Orientation::CorrectedFrenet] {
      for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {
        let opts=Options {normal:[0.,0.,1.],orientation,spacing,initial_sections:3,max_sections:17,max_deviation:1.};
        let audit=|p:&Curve,order,cells,work|Sweep::new(&profile,p,&scale,&twist,opts).unwrap().certify_closed_path_frame_smoothness(order,cells,work).unwrap();
        let proof=audit(path,2,10000,1000000);
        assert!(proof.closed_source_frame_smoothness_certified,"{orientation:?}/{spacing:?}/{proof:?}");
        assert!(proof.cells>0&&proof.exact_work>0);
        assert!(!audit(path,2,proof.cells-1,1000000).closed_source_frame_smoothness_certified);
        assert!(!audit(path,2,10000,proof.exact_work-1).closed_source_frame_smoothness_certified);
        assert!(!audit(path,2,0,1000000).closed_source_frame_smoothness_certified);
        assert!(!audit(&path,2,10000,0).closed_source_frame_smoothness_certified);
        let mut bad=path.clone();bad.control_points[3][1]=3_f64.next_up();
        assert!(!audit(&bad,2,10000,1000000).closed_source_frame_smoothness_certified);
        assert!(audit(&bad,1,10000,1000000).closed_source_frame_smoothness_certified);
        let mut spatial=path.clone();spatial.control_points[3][2]=1.;
        let opposite=spatial.control_points.len()-4;spatial.control_points[opposite][2]=-1.;
        assert_eq!(audit(&spatial,2,10000,1000000).closed_source_frame_smoothness_certified,orientation==Orientation::FixedNormal);
        let mut bad_twist=twist.clone();bad_twist.control_points[2][0]=0.25_f64.next_up();
        let damaged=Sweep::new(&profile,&path,&scale,&bad_twist,opts).unwrap();
        assert!(!damaged.certify_closed_path_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
        assert!(damaged.certify_closed_path_frame_smoothness(1,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
      }
    }
    }
}
#[test]
fn closed_cartesian_frame_jets_need_no_constant_homogeneous_seam_scale(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let normal=constant_vector_law([1.,0.,0.]).unwrap();
    let moving=Curve{degree:5,knots:[vec![0.;6],vec![1.;6]].concat(),
        control_points:vec![vec![0.,0.,1.],vec![0.125,0.,1.],vec![0.046875,0.125,1.],
            vec![-0.28125,0.125,1.],vec![-0.03125,0.,1.],vec![0.,0.,1.]],
        weights:vec![1.,0.5,1.,1.,2.,1.],periodic:false};
    let nonclamped=Curve{degree:2,knots:vec![-2.,-1.,0.,0.5,1.,2.,3.],
        control_points:vec![vec![0.,0.,1.];4],weights:vec![1.,2.,3.,4.],periodic:false};
    for axis in [&moving,&nonclamped]{
        for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}]{
            let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing,
                initial_sections:5,max_sections:17,max_deviation:2.};
            let audit=|law:&Curve,order,work|Sweep::new(&profile,&path,&scale,&twist,options).unwrap()
                .with_frame_laws(law,&normal).unwrap().certify_closed_authored_frame_smoothness(order,10000,work).unwrap();
            let proof=audit(axis,2,1000000);
            assert!(proof.closed_source_frame_smoothness_certified,"{spacing:?}: {proof:?}");
            assert!(proof.exact_work>0);
            assert!(!audit(axis,2,proof.exact_work-1).closed_source_frame_smoothness_certified);
            assert!(!audit(axis,2,0).closed_source_frame_smoothness_certified);
        }
    }
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,
        initial_sections:5,max_sections:17,max_deviation:2.};
    let mut bad=moving.clone();bad.control_points[3][0]=bad.control_points[3][0].next_up();
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&bad,&normal).unwrap();
    assert!(!sweep.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    assert!(sweep.certify_closed_authored_frame_smoothness(1,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
}
#[test]
fn closed_piecewise_bezier_frame_needs_internal_and_endpoint_jets(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let normal=constant_vector_law([1.,0.,0.]).unwrap();
    let axis=Curve{degree:5,knots:[vec![0.;6],vec![0.5;5],vec![1.;6]].concat(),
        control_points:vec![vec![0.,0.,1.],vec![0.0625,0.,1.],vec![0.125,0.03125,1.],
            vec![0.109375,0.0625,1.],vec![0.0546875,0.078125,1.],vec![0.,0.078125,1.],
            vec![-0.0546875,0.078125,1.],vec![-0.109375,0.0625,1.],vec![-0.125,0.03125,1.],
            vec![-0.0625,0.,1.],vec![0.,0.,1.]],weights:vec![1.;11],periodic:false};
    for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}]{
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing,
            initial_sections:5,max_sections:17,max_deviation:2.};
        let audit=|law:&Curve,order,work|Sweep::new(&profile,&path,&scale,&twist,options)
            .unwrap().with_frame_laws(law,&normal).unwrap()
            .certify_closed_authored_frame_smoothness(order,10000,work).unwrap();
        assert!(!basis_continuity(&axis,2));
        let proof=audit(&axis,2,1000000);
        assert!(proof.closed_source_frame_smoothness_certified,"{spacing:?}: {proof:?}");
        assert!(!audit(&axis,2,proof.exact_work-1).closed_source_frame_smoothness_certified);
        let mut bad=axis.clone();bad.control_points[7][0]=bad.control_points[7][0].next_up();
        assert!(!audit(&bad,2,1000000).closed_source_frame_smoothness_certified);
        assert!(audit(&bad,1,1000000).closed_source_frame_smoothness_certified);
        bad=axis.clone();bad.control_points[1][0]=bad.control_points[1][0].next_up();
        assert!(!audit(&bad,1,1000000).closed_source_frame_smoothness_certified);
    }
}
#[test]
fn moving_closed_authored_c2_needs_exact_seam_and_full_regular_cover(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let normal=constant_vector_law([1.,0.,0.]).unwrap();
    let axis=Curve {degree:5,knots:vec![0.,0.,0.,0.,0.,0.,1.,1.,1.,1.,1.,1.],
        control_points:vec![vec![0.,0.,1.],vec![0.125,0.,1.],vec![0.25,0.125,1.],
            vec![-0.25,0.125,1.],vec![-0.125,0.,1.],vec![0.,0.,1.]],
        weights:vec![1.;6],periodic:false};
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,
        initial_sections:5,max_sections:17,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&axis,&normal).unwrap();
    let proof=sweep.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap();
    assert!(proof.closed_source_frame_smoothness_certified,"{proof:?}");
    #[cfg(feature="transport")]
    {
        let request=value_codec::json!({"op":"surface_progressive_sweep_closed_frame_smoothness",
            "profiles":[profile],"path":path,"scale":scale,"twist":twist,
            "frame_axis":axis,"frame_normal":normal,"normal":[1.,0.,0.],
            "orientation":"authored","spacing":"parameter","initial_sections":5,
            "max_sections":17,"max_deviation":2.,"order":2,"maxCells":10000,"maxExactWork":1000000});
        let transported=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(transported["closedSourceFrameSmoothnessCertified"],true);
        assert_eq!(transported["scope"],"closed-original-authored-frame-only");
        for key in ["pathSeamCertified","retainedSeamsCertified","continuousBound","solidCertified"] {
            assert_eq!(transported[key],false);
        }
        let mut short=request;short["maxExactWork"]=value_codec::json!(proof.exact_work-1);
        assert_eq!(crate::transport::dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],false);
    }

    assert!(!sweep.certify_closed_authored_frame_smoothness(2,proof.cells-1,1000000).unwrap().closed_source_frame_smoothness_certified);
    assert!(!sweep.certify_closed_authored_frame_smoothness(2,10000,proof.exact_work-1).unwrap().closed_source_frame_smoothness_certified);
    let mut wrong=axis.clone();wrong.control_points[2][0]=0.25_f64.next_up();
    let bad=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&wrong,&normal).unwrap();
    assert!(!bad.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    assert!(bad.certify_closed_authored_frame_smoothness(1,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    assert!(!sweep.certify_original_frame_smoothness(2,10000,1000000).unwrap().source_frame_smoothness_certified);
    let multi=Curve {degree:3,knots:vec![0.,0.,0.,0.,0.25,0.5,0.75,1.,1.,1.,1.],
        control_points:vec![vec![0.,0.,1.],vec![0.125,0.,1.],vec![0.375,0.125,1.],
            vec![0.,0.25,1.],vec![-0.375,0.125,1.],vec![-0.125,0.,1.],vec![0.,0.,1.]],
        weights:vec![1.;7],periodic:false};
    let multi_sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&multi,&normal).unwrap();
    let multi_proof=multi_sweep.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap();
    assert!(multi_proof.closed_source_frame_smoothness_certified,"{multi_proof:?}");
    assert!(!multi_sweep.certify_closed_authored_frame_smoothness(2,10000,multi_proof.exact_work-1).unwrap().closed_source_frame_smoothness_certified);
    #[cfg(feature="transport")]
    {
        let request=value_codec::json!({"op":"surface_progressive_sweep_closed_frame_smoothness",
            "profiles":[profile],"path":path,"scale":scale,"twist":twist,
            "frame_axis":multi,"frame_normal":normal,"normal":[1.,0.,0.],
            "orientation":"authored","spacing":"parameter","initial_sections":5,
            "max_sections":17,"max_deviation":2.,"order":2,"maxCells":10000,"maxExactWork":1000000});
        let report=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(report["closedSourceFrameSmoothnessCertified"],true);
        assert_eq!(report["exactWork"],value_codec::json!(multi_proof.exact_work));
        assert_eq!(report["scope"],"closed-original-authored-frame-only");
        for key in ["pathSeamCertified","retainedSeamsCertified","continuousBound","solidCertified"] {
            assert_eq!(report[key],false);
        }
        let mut short=request;short["maxExactWork"]=value_codec::json!(multi_proof.exact_work-1);
        assert_eq!(crate::transport::dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],false);
    }
    let mut rational_multi=multi.clone();rational_multi.weights[3]=2.;
    for u in &mut rational_multi.knots {*u=-3.+8.* *u;}
    let rational_multi=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&rational_multi,&normal).unwrap();
    assert!(rational_multi.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let mut projective_multi=multi.clone();projective_multi.weights=vec![1.,1.,1.,1.5,2.,2.,2.];
    let projective=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&projective_multi,&normal).unwrap();
    let projective_proof=projective.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap();
    assert!(projective_proof.closed_source_frame_smoothness_certified,"{projective_proof:?}");
    assert!(!projective.certify_closed_authored_frame_smoothness(2,10000,projective_proof.exact_work-1).unwrap().closed_source_frame_smoothness_certified);
    projective_multi.control_points[2][0]=0.375_f64.next_up();
    let projective_bad=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&projective_multi,&normal).unwrap();
    assert!(!projective_bad.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    assert!(projective_bad.certify_closed_authored_frame_smoothness(1,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    // Matching endpoint jets do not promote an internal C0-only knot to C2.
    let mut c0_only=multi.clone();
    c0_only.knots.splice(6..6,[0.5,0.5]);
    c0_only.control_points.splice(3..3,[vec![0.,0.25,1.],vec![0.,0.25,1.]]);
    c0_only.weights.splice(3..3,[1.,1.]);
    assert!(super::super::endpoint_jets::certify(&c0_only,2,1000000).unwrap().certified);
    let c0_only=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&c0_only,&normal).unwrap();
    let refused=c0_only.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap();
    assert!(!refused.closed_source_frame_smoothness_certified);
    assert_eq!(refused.reason,Some("closed-authored-law-knot-continuity-unproved"));
    let mut damaged=multi.clone();damaged.control_points[2][0]=0.375_f64.next_up();
    let damaged=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&damaged,&normal).unwrap();
    assert!(!damaged.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    assert!(damaged.certify_closed_authored_frame_smoothness(1,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let mut projective_bezier=axis.clone();projective_bezier.weights=vec![1.,1.,1.,2.,2.,2.];
    let projective_bezier=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&projective_bezier,&normal).unwrap();
    assert!(projective_bezier.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let mut rational=axis.clone();
    rational.knots=vec![-3.,-3.,-3.,-3.,-3.,-3.,7.,7.,7.,7.,7.,7.];
    rational.weights=vec![1.,1.5,2.5,0.5,0.5,1.];
    rational.control_points=vec![vec![0.,0.,1.],vec![0.0625,0.,1.],vec![0.125,0.0625,1.],
        vec![-0.125,0.3125,1.],vec![-0.1875,0.,1.],vec![0.,0.,1.]];
    let arc_options=Options {spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},..options};
    let arc=Sweep::new(&profile,&path,&scale,&twist,arc_options).unwrap().with_frame_laws(&rational,&normal).unwrap();
    let arc_proof=arc.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap();
    assert!(arc_proof.closed_source_frame_smoothness_certified,"{arc_proof:?}");
    assert!(!arc.certify_closed_authored_frame_smoothness(2,10000,arc_proof.exact_work-1).unwrap().closed_source_frame_smoothness_certified);

}
