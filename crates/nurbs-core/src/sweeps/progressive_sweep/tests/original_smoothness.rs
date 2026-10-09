use super::*;
#[test]
fn forward_collinear_frames_are_c2_across_speed_knots_without_positional_claim(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let cubic=Curve {degree:3,knots:vec![2.,2.,2.,2.,5.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![0.,0.,1.],vec![0.,0.,7.],vec![0.,0.,10.]],weights:vec![1.,2.,2.,1.],periodic:false};
    let multispan=Curve {degree:1,knots:vec![2.,2.,3.,5.,5.],
        control_points:vec![vec![0.;3],vec![0.,0.,1.],vec![0.,0.,10.]],weights:vec![1.;3],periodic:false};
    assert!(!basis_continuity(&multispan,1));
    for path in [&cubic,&multispan] {for orientation in [Orientation::RotationMinimizing,Orientation::CorrectedFrenet] {
      for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {for order in [1,2] {
        let options=Options {normal:[1.,0.,1.],orientation,spacing,initial_sections:3,max_sections:9,max_deviation:1.};
        let audit=|source:&Curve,cells,work|Sweep::new(&profile,source,&scale,&twist,options).unwrap().certify_original_frame_smoothness(order,cells,work).unwrap();
        let proof=audit(path,10000,1000000);
        assert!(proof.source_frame_smoothness_certified,"{orientation:?}/{spacing:?}/{order}/{proof:?}");
        assert!(proof.cells>0&&proof.exact_work>0);
        assert!(!audit(path,proof.cells-1,1000000).source_frame_smoothness_certified);
        assert!(!audit(path,10000,proof.exact_work-1).source_frame_smoothness_certified);
        assert!(!audit(path,0,1000000).source_frame_smoothness_certified);
        assert!(!audit(path,10000,0).source_frame_smoothness_certified);
        let mut backtrack=multispan.clone();backtrack.control_points[1][2]=11.;
        assert!(Sweep::new(&profile,&backtrack,&scale,&twist,options).is_err());
      }}
    }}
}
#[test]
fn corrected_planar_source_frames_extend_through_inflections_with_shared_limits() {
    let profile=crate::primitives::line([0.,0.,1.],[0.,0.,2.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let path=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,1.,0.],
        vec![2.,-1.,0.],vec![3.,0.,0.]],None).unwrap();
    for rational in [false,true] {
        let mut source=path.clone();
        if rational {source.weights=vec![1.,2.,2.,1.];}
        let before=source.clone();
        assert!(source.evaluate(0.5).unwrap().d2.unwrap().iter().all(|x|*x==0.));
        for order in [1,2] {
            for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}] {
                let options=Options{normal:[0.,0.,1.],orientation:Orientation::CorrectedFrenet,
                    spacing,initial_sections:3,max_sections:9,max_deviation:1.};
                let audit=|curve:&Curve,cells,work|Sweep::new(&profile,curve,&scale,&twist,options).unwrap()
                    .certify_original_frame_smoothness(order,cells,work).unwrap();
                let proof=audit(&source,10000,1000000);
                assert!(proof.source_frame_smoothness_certified,"{rational}/{order}/{spacing:?}: {proof:?}");
                assert!(proof.cells>0&&proof.exact_work>0);
                assert!(!audit(&source,proof.cells-1,1000000).source_frame_smoothness_certified);
                assert!(!audit(&source,10000,proof.exact_work-1).source_frame_smoothness_certified);
                assert!(!audit(&source,0,1000000).source_frame_smoothness_certified);
                assert!(!audit(&source,10000,0).source_frame_smoothness_certified);
                let mut off_plane=source.clone();off_plane.control_points[1][2]=f64::from_bits(1);
                assert!(!audit(&off_plane,10000,1000000).source_frame_smoothness_certified);
            }
        }
        assert_eq!(source.control_points,before.control_points);
        assert_eq!(source.weights,before.weights);assert_eq!(source.knots,before.knots);
    }
    let stationary=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![0.,0.,0.]],None).unwrap();
    // This path is closed; choose an open source with an interior zero speed.
    let singular=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],
        vec![0.,0.,0.],vec![1.,0.,0.]],None).unwrap();
    let options=Options{normal:[0.,0.,1.],orientation:Orientation::CorrectedFrenet,
        spacing:Spacing::Parameter,initial_sections:3,max_sections:9,max_deviation:1.};
    for curve in [&stationary,&singular] {
        assert!(!Sweep::new(&profile,curve,&scale,&twist,options).unwrap()
            .certify_original_frame_smoothness(2,10000,1000000).unwrap().source_frame_smoothness_certified);
    }
    let straight=crate::primitives::line([0.;3],[1.,2.,3.]).unwrap();
    assert!(Sweep::new(&profile,&straight,&scale,&twist,options).unwrap()
        .certify_original_frame_smoothness(2,10000,0).unwrap().source_frame_smoothness_certified);
}
#[test]
fn mixed_path_knots_preserve_all_c2_frame_modes_and_shared_budget(){
    let path=Curve{degree:2,knots:vec![0.,0.,0.,0.25,0.5,0.5,1.,1.,1.],
        control_points:vec![vec![0.,0.,0.],vec![0.,0.,0.125],vec![0.,0.125,0.375],
            vec![0.,0.25,0.5],vec![0.,0.5,0.75],vec![0.,1.,1.]],weights:vec![1.;6],periodic:false};
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    for orientation in [Orientation::FixedNormal,Orientation::Frenet,Orientation::RotationMinimizing,Orientation::CorrectedFrenet]{
        for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}]{
            let options=Options{normal:[1.,0.,0.],orientation,spacing,initial_sections:3,max_sections:9,max_deviation:1.};
            let audit=|curve:&Curve,work|Sweep::new(&profile,curve,&scale,&twist,options).unwrap()
                .certify_original_frame_smoothness(2,10000,work).unwrap();
            let proof=audit(&path,1000000);
            assert!(proof.source_frame_smoothness_certified,"{orientation:?} {spacing:?}: {proof:?}");
            assert!(!audit(&path,proof.exact_work-1).source_frame_smoothness_certified);
            let mut bad=path.clone();bad.control_points[3][1]=bad.control_points[3][1].next_up();
            assert!(!audit(&bad,1000000).source_frame_smoothness_certified);
        }
    }
}
#[test]
fn original_curved_path_high_jets_cover_simple_knots_for_c2_frames(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    for rational in [false,true] {
        let path=Curve{degree:2,knots:vec![0.,0.,0.,0.5,1.,1.,1.],
            control_points:if rational{vec![vec![0.,0.,0.],vec![0.,0.,0.375],vec![0.,0.25,0.625],vec![0.,1.,1.]]}
                else{vec![vec![0.,0.,0.],vec![0.,0.,0.25],vec![0.,0.5,0.75],vec![0.,1.,1.]]},
            weights:if rational{vec![1.,2.,2.,1.]}else{vec![1.;4]},periodic:false};
        assert!(!basis_continuity(&path,3));
        for orientation in [Orientation::FixedNormal,Orientation::Frenet,Orientation::CorrectedFrenet] {
            for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}] {
                let options=Options{normal:[1.,0.,0.],orientation,spacing,initial_sections:3,max_sections:9,max_deviation:1.};
                let audit=|curve:&Curve,work,cells|Sweep::new(&profile,curve,&scale,&twist,options).unwrap()
                    .certify_original_frame_smoothness(2,cells,work).unwrap();
                let proof=audit(&path,1000000,10000);
                assert!(proof.source_frame_smoothness_certified,"{rational} {orientation:?} {spacing:?}: {proof:?}");
                assert!(proof.exact_work>0);
                assert!(!audit(&path,proof.exact_work-1,10000).source_frame_smoothness_certified);
                assert!(!audit(&path,1000000,0).source_frame_smoothness_certified);
                let mut bad=path.clone();bad.control_points[2][1]=bad.control_points[2][1].next_up();
                assert!(!audit(&bad,1000000,10000).source_frame_smoothness_certified);
            }
        }
    }
}
#[test]
fn original_multispan_frame_jets_override_basis_continuity_only_when_exact(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=crate::primitives::line([0.;3],[0.,0.,4.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let normal=constant_vector_law([1.,0.,0.]).unwrap();
    let axis=Curve {degree:2,knots:vec![0.,0.,0.,0.25,0.25,1.,1.,1.],
        control_points:vec![vec![0.,0.,1.],vec![0.125,0.,1.],vec![0.25,0.,1.],vec![0.625,0.,1.],vec![1.,0.,1.]],
        weights:vec![1.;5],periodic:false};
    let before=axis.clone();
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,
        initial_sections:3,max_sections:9,max_deviation:1.};
    let audit=|law:&Curve,order,work|Sweep::new(&profile,&path,&scale,&twist,options)
        .unwrap().with_frame_laws(law,&normal).unwrap()
        .certify_original_frame_smoothness(order,10000,work).unwrap();
    assert!(!basis_continuity(&axis,2));
    let proof=audit(&axis,2,1000000);
    assert!(proof.source_frame_smoothness_certified,"{proof:?}");
    assert!(proof.exact_work>0);
    assert!(!audit(&axis,2,proof.exact_work-1).source_frame_smoothness_certified);
    assert!(!audit(&axis,2,0).source_frame_smoothness_certified);
    let mut wrong=axis.clone();wrong.control_points[4][0]=1_f64.next_up();
    assert!(!audit(&wrong,2,1000000).source_frame_smoothness_certified);
    assert!(audit(&wrong,1,1000000).source_frame_smoothness_certified);
    wrong=axis.clone();wrong.knots[3]=0.5;wrong.knots[4]=0.5;
    assert!(!audit(&wrong,1,1000000).source_frame_smoothness_certified);
    let rational=Curve{knots:vec![0.,0.,0.,0.5,0.5,1.,1.,1.],
        control_points:vec![vec![0.,0.,1.];5],weights:vec![1.,0.5,1.,1.5,3.],..axis.clone()};
    assert!(audit(&rational,2,1000000).source_frame_smoothness_certified);
    assert_eq!(axis.control_points,before.control_points);assert_eq!(axis.knots,before.knots);
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&axis,&normal).unwrap();
    assert!(!sweep.certify_original_frame_smoothness(2,0,1000000).unwrap().source_frame_smoothness_certified);
}
#[test]
fn original_nonaxial_planar_frame_c2_requires_plane_regular_cover_and_budget() {
    let path = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0.; 3], vec![0.5, -0.5, 0.], vec![1., -1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let profile = crate::primitives::line([1., 1., 0.], [2., 2., 0.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = crate::primitives::line([0.; 3], [0.125, 0., 0.]).unwrap();
    for spacing in [
        Spacing::Parameter,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    ] {
        let options = Options {
            normal: [1., 1., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing,
            initial_sections: 3,
            max_sections: 17,
            max_deviation: 2.,
        };
        let sweep = Sweep::new(&profile, &path, &scale, &twist, options).unwrap();
        let proof = sweep
            .certify_original_frame_smoothness(2, 10000, 1000000)
            .unwrap();
        assert!(proof.source_frame_smoothness_certified, "{proof:?}");
        assert!(proof.cells > 0 && proof.exact_work > 0);
        assert!(
            !sweep
                .certify_original_frame_smoothness(2, proof.cells - 1, 1000000)
                .unwrap()
                .source_frame_smoothness_certified
        );
        assert!(
            !sweep
                .certify_original_frame_smoothness(2, 10000, proof.exact_work - 1)
                .unwrap()
                .source_frame_smoothness_certified
        );
        let mut wrong = path.clone();
        wrong.control_points[1][1] = (-0.5_f64).next_up();
        assert!(
            !Sweep::new(&profile, &wrong, &scale, &twist, options)
                .unwrap()
                .certify_original_frame_smoothness(2, 10000, 1000000)
                .unwrap()
                .source_frame_smoothness_certified
        );
    }
}
#[test]
fn guided_arc_curved_independent_domains_prove_c2_without_inverse_jets() {
    let profile=crate::primitives::line([0.1,0.,0.],[0.2,0.,0.]).unwrap();
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],
        control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,2.,1.]],weights:vec![1.;3],periodic:false};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:17,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_orientation_guide(&guide).unwrap();
    let proof=sweep.certify_original_frame_smoothness(2,10000,1000000).unwrap();
    assert!(proof.source_frame_smoothness_certified,"{proof:?}");
    assert!(!sweep.certify_original_frame_smoothness(2,proof.cells-1,1000000).unwrap().source_frame_smoothness_certified);
    #[cfg(feature="transport")]
    {
        let request=value_codec::json!({"op":"surface_progressive_sweep_frame_smoothness",
            "profiles":[profile],"path":path,"scale":scale,"twist":twist,"orientation_guide":guide,
            "normal":[1.,0.,0.],"orientation":"rmf","spacing":"arc_length",
            "length_tolerance":0.001,"length_max_cells":100000,
            "initial_sections":3,"max_sections":17,"max_deviation":2.,
            "order":2,"maxCells":10000,"maxExactWork":1000000});
        let transported=crate::transport::dispatch(request).unwrap();
        assert_eq!(transported["sourceFrameSmoothnessCertified"],true);
        assert_eq!(transported["continuousBound"],false);
        assert_eq!(transported["solidCertified"],false);
    }
    let mut rational=guide.clone();rational.weights[1]=0.5;
    let rational_sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_orientation_guide(&rational).unwrap();
    let rational_proof=rational_sweep.certify_original_frame_smoothness(2,10000,1000000).unwrap();
    assert!(rational_proof.source_frame_smoothness_certified,"{rational_proof:?}");
    assert!(!rational_sweep.certify_original_frame_smoothness(2,rational_proof.cells-1,1000000).unwrap().source_frame_smoothness_certified);
    let mut crossing=guide.clone();crossing.control_points.iter_mut().for_each(|p|p[0]=0.);
    let unresolved=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_orientation_guide(&crossing).unwrap()
        .certify_original_frame_smoothness(2,200,1000000).unwrap();
    assert!(!unresolved.source_frame_smoothness_certified);
}
#[test]
fn guided_arc_c2_requires_both_speed_covers_and_joint_frame_budget() {
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=crate::primitives::line([0.;3],[0.,0.,4.]).unwrap();
    let guide=crate::primitives::line([1.,0.,0.],[1.,0.,4.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:10000},
        initial_sections:3,max_sections:5,max_deviation:0.01};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_orientation_guide(&guide).unwrap();
    let proof=sweep.certify_original_frame_smoothness(2,10000,1000000).unwrap();
    assert!(proof.source_frame_smoothness_certified,"{proof:?}");
    assert!(proof.cells>2);
    assert!(!sweep.certify_original_frame_smoothness(2,proof.cells-1,1000000).unwrap().source_frame_smoothness_certified);
    let kink=Curve {degree:1,knots:vec![0.,0.,0.5,1.,1.],
        control_points:vec![vec![1.,0.,0.],vec![1.,0.,1.],vec![2.,1.,4.]],
        weights:vec![1.;3],periodic:false};
    for (source,rail) in [(&path,&kink)] {
        let refused=Sweep::new(&profile,source,&scale,&twist,options).unwrap().with_orientation_guide(rail).unwrap()
            .certify_original_frame_smoothness(2,10000,1000000).unwrap();
        assert!(!refused.source_frame_smoothness_certified);
        assert_eq!(refused.reason,Some("original-frame-path-knot-continuity-unproved"));
    }
    assert!(Sweep::new(&profile,&kink,&scale,&twist,options).is_err());
    let stopped=constant_vector_law([1.,0.,0.]).unwrap();
    let bad=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_orientation_guide(&stopped).unwrap()
        .certify_original_frame_smoothness(2,100,1000000).unwrap();
    assert!(!bad.source_frame_smoothness_certified);
    assert_eq!(bad.reason,Some("guided-arc-source-speed-unproved"));
}
#[test]
fn original_curved_fixed_normal_and_frenet_c2_cover_both_spacings() {
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.,0.,0.],vec![0.5,-0.5,0.],vec![1.,-1.,1.]],
        weights:vec![1.;3],periodic:false};
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    for orientation in [Orientation::FixedNormal,Orientation::Frenet] {
        for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {
            let options=Options {normal:[1.,1.,0.],orientation,spacing,
                initial_sections:3,max_sections:5,max_deviation:0.01};
            let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap();
            let proof=sweep.certify_original_frame_smoothness(2,10000,1000000).unwrap();
            assert!(proof.source_frame_smoothness_certified,"{orientation:?} {spacing:?}: {proof:?}");
            assert!(!sweep.certify_original_frame_smoothness(2,0,1000000).unwrap().source_frame_smoothness_certified);
        }
    }
}
#[test]
#[cfg(feature = "transport")]
fn original_frame_transport_preserves_scope_budget_and_closed_refusal() {
    let profile = crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path = crate::primitives::line([0.;3],[0.,0.,4.]).unwrap();
    let scale = constant_vector_law([1.,0.,0.]).unwrap();
    let twist = constant_vector_law([0.;3]).unwrap();
    let request=value_codec::json!({"op":"surface_progressive_sweep_frame_smoothness",
        "profiles":[profile],"path":path,"scale":scale,"twist":twist,
        "normal":[1.,0.,0.],"orientation":"rmf","spacing":"parameter",
        "initial_sections":3,"max_sections":5,"max_deviation":0.01,
        "order":2,"maxCells":10000,"maxExactWork":1000000});
    let proof=crate::transport::dispatch(request.clone()).unwrap();
    assert_eq!(proof["sourceFrameSmoothnessCertified"],true);
    assert_eq!(proof["scope"],"open-original-frame-only");
    for key in ["retainedSeamsCertified","profileJoinsCertified","capJoinsCertified","continuousBound","solidCertified"] {
        assert_eq!(proof[key],false);
    }
    let mut short=request.clone(); short["maxCells"]=value_codec::json!(0);
    assert_eq!(crate::transport::dispatch(short).unwrap()["sourceFrameSmoothnessCertified"],false);
    let mut invalid=request.clone(); invalid["order"]=value_codec::json!(3);
    assert!(crate::transport::dispatch(invalid).is_err());
    let closed=Curve { degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.,0.,0.],vec![0.,0.,2.],vec![0.,0.,0.]],
        weights:vec![1.;3],periodic:false };
    let mut closed_request=request; closed_request["path"]=value_codec::json!(closed);
    let refused=crate::transport::dispatch(closed_request).unwrap();
    assert_eq!(refused["sourceFrameSmoothnessCertified"],false);
    assert_eq!(refused["reason"],"original-frame-closed-seam-unproved");
}
#[test]
fn original_authored_frame_c2_does_not_follow_from_piecewise_value_cover() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = crate::primitives::line([0., 0., 1.], [0.25, 0., 1.]).unwrap();
    let normal = crate::primitives::line([1., 0., 0.], [1., 0.25, 0.]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 5,
        max_deviation: 1.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_frame_laws(&axis, &normal)
        .unwrap();
    assert!(
        sweep
            .certify_original_frame_smoothness(2, 10000, 1000000)
            .unwrap()
            .source_frame_smoothness_certified
    );
    let broken = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 0.125, 0.], vec![1., 0.5, 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let refused = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_frame_laws(&axis, &broken)
        .unwrap()
        .certify_original_frame_smoothness(2, 10000, 1000000)
        .unwrap();
    assert!(!refused.source_frame_smoothness_certified);
    assert_eq!(
        refused.reason,
        Some("original-frame-law-knot-continuity-unproved")
    );
}
