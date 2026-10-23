    use super::*;
    #[test]
    fn guide_affine_generation_certifies_actual_rail_orientation() {
        let profiles=[crate::primitives::line([0.1,0.,0.],[0.2,0.,0.]).unwrap()];let points=[[0.,0.,0.],[0.,0.,10.]];
        let scale=scalar(1.,1.);let twist=scalar(0.,0.);
        let vector=|a:V,b:V,domain:[f64;2]|Curve{degree:1,knots:vec![domain[0],domain[0],domain[1],domain[1]],control_points:vec![a.to_vec(),b.to_vec()],weights:vec![1.;2],periodic:false};
        let longitudinal=vector([0.,0.,1.],[0.,0.,1.],[2.,5.]);
        let transverse=vector([1.,0.,0.],[1.,1.,0.],[11.,13.]);
        let axes=vector([1.,1.,1.],[2.,1.,1.],[17.,19.]);
        let center=Curve{degree:2,knots:vec![7.,7.,7.,9.,9.,9.],control_points:vec![vec![0.,0.,0.],vec![0.,0.,0.125],vec![0.,1.,0.25]],weights:vec![1.;3],periodic:false};
        let guide=vector([1.,0.,0.],[1.,1.,10.],[31.,41.]);
        let options=Options{normal:[1.,0.,0.],closed:false,miter_limit:2.,initial_steps:1,max_steps:64,max_deviation:0.01};
        let sweep=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_orientation_guide(&guide).unwrap().with_affine_laws(&axes,&center).unwrap();
        assert!(!sweep.preview_at(1).unwrap().report.accepted);
        let level=sweep.preview_at(16).unwrap();assert!(level.report.accepted,"{:?}",level.report);
        assert!(level.report.affine_laws_applied && level.report.orientation_guide_applied && !level.report.authored_frames_applied);
        let reverse_order=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_affine_laws(&axes,&center).unwrap().with_orientation_guide(&guide).unwrap().sections_at(16).unwrap();
        assert_eq!(level.sections.last().unwrap()[0].control_points,reverse_order.last().unwrap()[0].control_points);
        for framed in [
            Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_frame_laws(&longitudinal,&transverse).unwrap().with_orientation_guide(&guide).unwrap().with_affine_laws(&axes,&center).unwrap(),
            Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_orientation_guide(&guide).unwrap().with_frame_laws(&longitudinal,&transverse).unwrap().with_affine_laws(&axes,&center).unwrap(),
        ] {
            let combined=framed.preview_at(16).unwrap();
            assert!(combined.report.accepted && combined.report.authored_frames_applied && combined.report.orientation_guide_applied && combined.report.affine_laws_applied);
            for (a,b) in combined.sections.iter().zip(&level.sections) {for (a,b) in a.iter().zip(b) {assert_eq!(a.control_points,b.control_points);}}
        }
        let bound=level.report.certified_error_upper.unwrap();
        for i in 0..16 {for local in [0.,0.25,0.5,0.75,1.] {let f=(i as f64+local)/16.;let h=(1.+f*f).sqrt();
            for u in [0.,0.3,0.7,1.] {let r=0.1+0.1*u;let ideal=[(r*(1.+f)-f*f*f)/h,(r*(1.+f)*f+f*f)/h,10.*f+0.25*f];
                let a=level.sections[i][0].evaluate(u).unwrap().point;let b=level.sections[i+1][0].evaluate(u).unwrap().point;
                let retained=std::array::from_fn(|k|(1.-local)*a[k]+local*b[k]);assert!(norm(sub(ideal,retained))<=bound);
            }
        }}
        assert!(sweep.certify_level(16,&level.sections,10000,10).unwrap().error_upper.is_none());
        let mut altered=level.sections.clone();altered.last_mut().unwrap()[0].control_points[0][2]+=0.125;
        assert!(sweep.certify_level(16,&altered,10000,64).unwrap().endpoint_contour_error_upper.unwrap()[1]>=0.125);
    }
    #[test]
    fn combined_authored_frame_affine_laws_certify_actual_interpolation() {
        let profiles=[crate::primitives::line([0.1,0.,0.],[0.2,0.,0.]).unwrap()];let points=[[0.,0.,0.],[0.,0.,10.]];
        let scale=scalar(1.,1.);let twist=scalar(0.,0.);
        let vector=|a:V,b:V,domain:[f64;2]|Curve{degree:1,knots:vec![domain[0],domain[0],domain[1],domain[1]],control_points:vec![a.to_vec(),b.to_vec()],weights:vec![1.;2],periodic:false};
        let longitudinal=vector([0.,0.,1.],[0.,0.,1.],[2.,5.]);
        let transverse=vector([1.,0.,0.],[1.,1.,0.],[11.,13.]);
        let axes=vector([1.,1.,1.],[2.,1.,1.],[17.,19.]);
        let center=Curve{degree:2,knots:vec![7.,7.,7.,9.,9.,9.],control_points:vec![vec![0.,0.,0.],vec![0.,0.,0.125],vec![0.,1.,0.25]],weights:vec![1.;3],periodic:false};
        let options=Options{normal:[1.,0.,0.],closed:false,miter_limit:2.,initial_steps:1,max_steps:64,max_deviation:0.01};
        let sweep=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_frame_laws(&longitudinal,&transverse).unwrap().with_affine_laws(&axes,&center).unwrap();
        assert!(!sweep.preview_at(1).unwrap().report.accepted);
        let level=sweep.preview_at(16).unwrap();assert!(level.report.accepted,"{:?}",level.report);
        assert!(level.report.affine_laws_applied && level.report.authored_frames_applied);
        let reverse_order=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_affine_laws(&axes,&center).unwrap().with_frame_laws(&longitudinal,&transverse).unwrap().sections_at(16).unwrap();
        assert_eq!(level.sections.last().unwrap()[0].control_points,reverse_order.last().unwrap()[0].control_points);
        let bound=level.report.certified_error_upper.unwrap();
        for i in 0..16 {for local in [0.,0.25,0.5,0.75,1.] {let f=(i as f64+local)/16.;let h=(1.+f*f).sqrt();
            for u in [0.,0.3,0.7,1.] {let r=0.1+0.1*u;let ideal=[(r*(1.+f)-f*f*f)/h,(r*(1.+f)*f+f*f)/h,10.*f+0.25*f];
                let a=level.sections[i][0].evaluate(u).unwrap().point;let b=level.sections[i+1][0].evaluate(u).unwrap().point;
                let retained=std::array::from_fn(|k|(1.-local)*a[k]+local*b[k]);assert!(norm(sub(ideal,retained))<=bound);
            }
        }}
        assert!(sweep.certify_level(16,&level.sections,10000,13).unwrap().error_upper.is_none());
        let mut altered=level.sections.clone();altered.last_mut().unwrap()[0].control_points[0][2]+=0.125;
        assert!(sweep.certify_level(16,&altered,10000,64).unwrap().endpoint_contour_error_upper.unwrap()[1]>=0.125);
    }
    #[test]
    fn authored_frame_generation_refines_and_certifies_original_profile_interpolation() {
        let profiles=[crate::primitives::line([0.1,0.,0.],[0.2,0.,0.]).unwrap()];
        let points=[[0.,0.,0.],[0.,0.,10.]];
        let scale=scalar(1.,1.);let twist=scalar(0.,0.);
        let longitudinal=Curve {degree:1,knots:vec![2.,2.,5.,5.],control_points:vec![vec![0.,0.,1.];2],weights:vec![1.;2],periodic:false};
        let transverse=Curve {degree:1,knots:vec![7.,7.,9.,9.],control_points:vec![vec![1.,0.,0.],vec![1.,1.,0.]],weights:vec![1.;2],periodic:false};
        let options=Options {normal:[1.,0.,0.],closed:false,miter_limit:2.,initial_steps:1,max_steps:16,max_deviation:0.001};
        let sweep=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_frame_laws(&longitudinal,&transverse).unwrap();
        assert!(!sweep.preview_at(1).unwrap().report.accepted);
        let local=sweep.transport_certificate.profile_local_enclosure([0.1,0.,0.]).unwrap().unwrap();
        sweep.transport_certificate.interpolation_upper_authored_frame(0,[0.,1./16.],local,&longitudinal,&transverse,&scale,&twist,64).unwrap().unwrap();
        sweep.transport_certificate.station_local_authored_frame(0,[0.,0.],local,&longitudinal,&transverse,&scale,&twist,64).unwrap().unwrap();
        let level=sweep.preview_at(16).unwrap();
        assert!(level.report.accepted && level.report.authored_frames_applied,"{:?}",level.report);
        assert!(level.report.wall_regularity_certified.unwrap());
        let bound=level.report.certified_error_upper.unwrap();
        for i in 0..16 {
            for local in [0.,0.25,0.5,0.75,1.] {
                let f=(i as f64+local)/16.;let h=(1.+f*f).sqrt();
                for u in [0.,0.3,0.7,1.] {
                    let radius=0.1+0.1*u;let ideal=[radius/h,radius*f/h,10.*f];
                    let a=level.sections[i][0].evaluate(u).unwrap().point;
                    let b=level.sections[i+1][0].evaluate(u).unwrap().point;
                    let retained=std::array::from_fn(|k|(1.-local)*a[k]+local*b[k]);
                    assert!(norm(sub(ideal,retained))<=bound);
                }
            }
        }
        let mut damaged=level.sections.clone();damaged.last_mut().unwrap()[0].control_points[0][0]+=0.125;
        let report=sweep.certify_level(16,&damaged,10000,64).unwrap();
        assert!(report.error_upper.unwrap()>=0.125);
        assert!(report.endpoint_contour_error_upper.unwrap()[1]>=0.125);
        assert!(sweep.certify_level(16,&level.sections,10000,7).unwrap().error_upper.is_none());
        let mut singular=longitudinal.clone();singular.control_points=vec![vec![0.;3];2];
        let singular_sweep=Sweep::new(&profiles,&points,&scale,&twist,options).unwrap().with_frame_laws(&singular,&transverse).unwrap();
        assert!(singular_sweep.certify_level(16,&level.sections,10000,64).unwrap().error_upper.is_none());
    }
    #[test]
    fn affine_closed_miter_checks_law_closure_and_last_retained_interval(){
        let profiles=[crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap()];
        let points=[[0.,0.,0.],[10.,0.,0.],[10.,10.,0.],[0.,10.,0.]];
        let scale=scalar(1.,1.);let twist=scalar(0.,0.);
        let axes=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],control_points:vec![vec![1.,1.,1.],vec![2.,1.,1.],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
        let center=Curve {degree:2,knots:vec![7.,7.,7.,9.,9.,9.],control_points:vec![vec![0.,0.,0.],vec![0.2,0.,0.],vec![0.,0.,0.]],weights:vec![1.;3],periodic:false};
        let opts=Options {normal:[0.,0.,1.],closed:true,miter_limit:2.,initial_steps:1,max_steps:16,max_deviation:0.01};
        let sweep=Sweep::new(&profiles,&points,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
        let level=sweep.preview_at(8).unwrap();
        assert!(level.report.accepted && level.report.affine_laws_applied);
        assert_eq!(level.sections[0][0].control_points,level.sections.last().unwrap()[0].control_points);
        let mut altered=level.sections.clone();
        altered.last_mut().unwrap()[0].control_points[0][2]+=0.125;
        let proof=sweep.certify_level(8,&altered,10000,64).unwrap();
        assert!(proof.error_upper.unwrap()>=0.125);
        assert!(proof.endpoint_contour_error_upper.unwrap()[1]>=0.125);
        for component in 0..2 {
            let mut bad_axes=axes.clone();let mut bad_center=center.clone();
            if component==0 {bad_axes.control_points[2][0]=1.25;}else{bad_center.control_points[2][0]=0.125;}
            assert!(Sweep::new(&profiles,&points,&scale,&twist,opts).unwrap().with_affine_laws(&bad_axes,&bad_center).is_err());
        }
        let discontinuous=Curve {degree:1,knots:vec![0.,0.,0.5,0.5,1.,1.],control_points:vec![vec![0.;3];4],weights:vec![1.;4],periodic:false};
        assert!(Sweep::new(&profiles,&points,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&discontinuous).is_err());
    }
    #[test]
    fn affine_generator_retains_laws_and_certifies_entire_interpolated_profile(){
        let profiles=[crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap()];
        let points=[[0.,0.,0.],[0.,0.,10.]];
        let scale=scalar(1.,1.);let twist=scalar(0.,0.);
        let axes=Curve {degree:1,knots:vec![2.,2.,5.,5.],control_points:vec![vec![1.,1.,1.],vec![2.,1.,1.]],weights:vec![1.;2],periodic:false};
        let center=Curve {degree:2,knots:vec![7.,7.,7.,9.,9.,9.],control_points:vec![vec![0.,0.,0.],vec![0.,0.,0.],vec![0.,1.,0.]],weights:vec![1.;3],periodic:false};
        let mut opts=options(false);opts.normal=[1.,0.,0.];opts.max_deviation=0.01;
        let sweep=Sweep::new(&profiles,&points,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
        let level=sweep.preview_at(8).unwrap();
        assert!(level.report.affine_laws_applied && level.report.accepted);
        let bound=level.report.certified_error_upper.unwrap();
        assert!(bound<opts.max_deviation);
        assert_eq!(level.report.continuous_error_upper,bound);
        for (i,pair) in level.sections.windows(2).enumerate(){
            for j in 0..=10 {let local=j as f64/10.;let f=(i as f64+local)/8.;
                for u in [0.,0.25,0.5,0.75,1.] {
                    let a=pair[0][0].evaluate(u).unwrap().point;let b=pair[1][0].evaluate(u).unwrap().point;
                    let stored:V=std::array::from_fn(|k|(1.-local)*a[k]+local*b[k]);
                    let ideal=[(1.+f)*(1.+u),f*f,10.*f];
                    assert!(norm(sub(stored,ideal))<=bound);
                }
            }
        }
        assert!(!sweep.preview_at(1).unwrap().report.accepted);
        let mut invalid=axes.clone();invalid.control_points[0][0]=0.;
        assert!(Sweep::new(&profiles,&points,&scale,&twist,opts).unwrap().with_affine_laws(&invalid,&center).is_err());
        #[cfg(feature="transport")]
        {
            let request=value_codec::json!({"op":"curve_progressive_miter_level","profiles":profiles,"points":points,
                "scale":scale,"twist":twist,"axis_scale":axes,"center_law":center,"normal":opts.normal,
                "closed":false,"miter_limit":opts.miter_limit,"initial_steps":opts.initial_steps,"max_steps":opts.max_steps,
                "max_deviation":opts.max_deviation,"preview_steps":8});
            let report=crate::transport::dispatch(request).unwrap();
            assert_eq!(report["report"]["accepted"],true);
            assert_eq!(report["report"]["affineLawsApplied"],true);
            assert_eq!(report["report"]["continuousErrorMethod"],"interval-affine-law-interpolation");
            assert_eq!(report["report"]["continuousBound"],false);
        }
    }
    #[test]
    fn local_phase_hulls_prevent_whole_turn_probe_aliasing() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 4. * std::f64::consts::TAU);
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        opts.max_steps = 256;
        opts.max_deviation = 1e-3;
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        let level = sweep.preview_at(1).unwrap();
        assert!(level.report.sampled_control_deviation < 1e-12);
        assert!(!level.report.phase_resolved && !level.report.accepted);
        let result = approximate(&profiles, &points, &scale, &twist, opts).unwrap();
        assert!(result.levels.last().unwrap().accepted);
        assert!(result.levels.last().unwrap().steps >= 16);
    }
    #[test]
    fn real_arithmetic_bound_covers_nondyadic_rational_law_interpolation() {
        let profiles = [crate::paths::bezier(
            vec![vec![0.1, 0., 0.], vec![0.2, 0.1, 0.], vec![0.3, 0., 0.]],
            Some(vec![1., 0.7, 2.]),
        )
        .unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.]];
        let scale = crate::paths::bezier(
            vec![vec![1., 0., 0.], vec![1.7, 0., 0.], vec![1.1, 0., 0.]],
            Some(vec![1., 0.8, 1.2]),
        )
        .unwrap();
        let twist = crate::paths::bezier(
            vec![vec![0., 0., 0.], vec![1.2, 0., 0.], vec![3., 0., 0.]],
            Some(vec![1., 1.1, 0.9]),
        )
        .unwrap();
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        opts.max_steps = 64;
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        let level = sweep.preview_at(8).unwrap();
        for edge in 0..2 {
            for interval in 0..8 {
                for probe in 1..127 {
                    let f = probe as f64 / 127.;
                    let actual = sweep.station(edge, (interval as f64 + f) / 8.).unwrap();
                    let a = &level.sections[edge * 8 + interval][0];
                    let b = &level.sections[edge * 8 + interval + 1][0];
                    for ((p, a), b) in actual[0]
                        .control_points
                        .iter()
                        .zip(&a.control_points)
                        .zip(&b.control_points)
                    {
                        let error =
                            norm(std::array::from_fn(|k| p[k] - (1. - f) * a[k] - f * b[k]));
                        assert!(
                            error <= level.report.continuous_error_upper + 1e-12,
                            "{error} exceeds {}",
                            level.report.continuous_error_upper
                        );
                    }
                }
            }
        }
        assert!(level.report.continuous_error_upper >= level.report.sampled_control_deviation);
    }
    #[test]
    fn discontinuous_laws_are_refused_and_internal_corners_use_lipschitz_bound() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = Curve {
            degree: 1,
            knots: vec![0., 0., 0.37, 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![2., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        let twist = scalar(0., 0.);
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        let level = sweep.preview_at(1).unwrap();
        assert!(level.report.continuous_error_upper >= 0.2);
        let certified = sweep.certify_level(1, &level.sections, 10, 4).unwrap();
        assert_eq!(certified.status, scalar_certificate::Status::Certified);
        assert!(certified.error_upper.unwrap() >= 0.2);
        assert!(
            sweep
                .certify_level(1, &level.sections, 10, 1)
                .unwrap()
                .error_upper
                .is_none()
        );
        for f in [0.13, 0.37, 0.63, 0.91] {
            let ideal = sweep.station(0, f).unwrap();
            for k in 0..2 {
                let a = &level.sections[0][0].control_points[k];
                let b = &level.sections[1][0].control_points[k];
                let p = &ideal[0].control_points[k];
                assert!(
                    norm(std::array::from_fn(|j| p[j] - ((1. - f) * a[j] + f * b[j])))
                        <= certified.error_upper.unwrap()
                );
            }
        }
        assert!(!level.report.accepted);
        let discontinuous = Curve {
            degree: 1,
            knots: vec![0., 0., 0.37, 0.37, 1., 1.],
            control_points: vec![
                vec![1., 0., 0.],
                vec![2., 0., 0.],
                vec![3., 0., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(Sweep::new(&profiles, &points, &discontinuous, &twist, opts).is_err());
    }
    #[test]
    fn unresolved_transport_limit_cannot_promote_a_zero_error_level() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 5.], [0., 0., 10.]];
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        opts.miter_limit = 1.;
        opts.max_steps = 2;
        let approximation =
            approximate(&profiles, &points, &scalar(1., 1.), &scalar(0., 0.), opts).unwrap();
        assert!(approximation.sections.is_none());
        let r = approximation.levels.last().unwrap();
        assert!(r.sampled_control_deviation < 1e-12 && r.phase_resolved);
        assert!(!r.frame_transport_certified && !r.accepted);
        assert_eq!(
            r.frame_transport_reason,
            Some("transport-enclosure-unresolved")
        );
    }
    fn scalar(a: f64, b: f64) -> Curve {
        crate::paths::bezier(vec![vec![a, 0., 0.], vec![b, 0., 0.]], None).unwrap()
    }
    fn options(closed: bool) -> Options {
        Options {
            normal: [0., 0., 1.],
            closed,
            miter_limit: 4.,
            initial_steps: 1,
            max_steps: 64,
            max_deviation: 1e-4,
        }
    }
    #[test]
    fn constant_laws_match_existing_extrusion_miter_sections() {
        let profile = crate::primitives::line([0., 0.1, 0.2], [0., 0.3, 0.4]).unwrap();
        let profiles = [profile];
        let points = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, options(false)).unwrap();
        let expected = crate::paths::miter_sections(&profiles, &points, [0., 0., 1.], 4.).unwrap();
        let actual = sweep.sections_at(1).unwrap();
        for (a, b) in actual.iter().zip(expected) {
            for (a, b) in a.iter().zip(b) {
                for (a, b) in a.control_points.iter().zip(b.control_points) {
                    assert!(norm(sub([a[0], a[1], a[2]], [b[0], b[1], b[2]])) < 1e-12);
                }
            }
        }
        assert!(sweep.preview_at(1).unwrap().report.accepted);
    }
    #[test]
    fn distributed_twist_corrects_skew_loop_holonomy_and_shares_all_corners() {
        let profiles = [crate::primitives::line([0., 0.1, 0.2], [0., 0.3, 0.4]).unwrap()];
        let points = [
            [0., 0., 0.],
            [10., 0., 0.],
            [10., 10., 4.],
            [0., 10., 1.],
            [0., 5., -2.],
        ];
        assert!(crate::paths::closed_miter_sections(&profiles, &points, [0., 0., 1.], 4.).is_err());
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, options(true)).unwrap();
        assert!(sweep.correction.abs() > 1e-3);
        let angle = sweep.transport_certificate.holonomy_angle.unwrap();
        assert!(angle[0] <= sweep.correction && sweep.correction <= angle[1]);
        for edge in 0..sweep.tangents.len() {
            for f in [0., 0.37, 1.] {
                let (n, b) = sweep
                    .transport_certificate
                    .rotated_frame(edge, [f, f], [0., 0.])
                    .unwrap()
                    .unwrap();
                let total = *sweep.lengths.last().unwrap();
                let traversal = (sweep.lengths[edge]
                    + f * (sweep.lengths[edge + 1] - sweep.lengths[edge]))
                    / total;
                let normal = rotate(
                    sweep.normals[edge],
                    sweep.tangents[edge],
                    sweep.correction * traversal,
                );
                let binormal = cross(sweep.tangents[edge], normal);
                let station = sweep.station(edge, f).unwrap();
                for (profile, section) in profiles.iter().zip(&station) {
                    for (authored, stored) in
                        profile.control_points.iter().zip(&section.control_points)
                    {
                        let bound = sweep
                            .transport_certificate
                            .station_point(
                                edge,
                                [f, f],
                                [authored[0], authored[1], authored[2]],
                                [1., 1.],
                                [0., 0.],
                            )
                            .unwrap()
                            .unwrap();
                        for k in 0..3 {
                            assert!(bound[k][0] <= stored[k] && stored[k] <= bound[k][1]);
                        }
                    }
                }
                let sheared = sweep
                    .transport_certificate
                    .miter_offset(edge, [f, f], n)
                    .unwrap()
                    .unwrap();
                let t = sweep.tangents[edge];
                let shift = |h: Option<V>| h.map_or(0., |h| dot(normal, h) / dot(t, h));
                let axial = (1. - f) * shift(sweep.planes[edge])
                    + f * shift(sweep.planes[(edge + 1) % points.len()]);
                for k in 0..3 {
                    let expected = normal[k] - axial * t[k];
                    assert!(sheared[k][0] <= expected && expected <= sheared[k][1]);
                }
                for k in 0..3 {
                    assert!(n[k][0] <= normal[k] && normal[k] <= n[k][1]);
                    assert!(b[k][0] <= binormal[k] && binormal[k] <= b[k][1]);
                }
            }
        }
        for i in 0..points.len() {
            let a = sweep.station(i, 1.).unwrap();
            let b = sweep.station((i + 1) % points.len(), 0.).unwrap();
            for (a, b) in a.iter().zip(b) {
                for (a, b) in a.control_points.iter().zip(b.control_points) {
                    assert!(norm(sub([a[0], a[1], a[2]], [b[0], b[1], b[2]])) < 1e-10);
                }
            }
        }
        let result = approximate(&profiles, &points, &scale, &twist, options(true)).unwrap();
        assert!(
            result.levels.last().unwrap().accepted,
            "{:?}",
            result.levels
        );
        assert!(result.levels.len() > 1);
        let sections = result.sections.unwrap();
        assert_eq!(sections.first(), sections.last());
    }
    #[test]
    fn scalar_laws_follow_independent_scaled_full_turn_and_fail_closed_mismatch() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 2.);
        let twist = scalar(0., std::f64::consts::TAU);
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        opts.max_steps = 256;
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        for f in [0., 0.17, 0.5, 0.93, 1.] {
            let p = &sweep.station(0, f).unwrap()[0].control_points[1];
            let angle = std::f64::consts::TAU * f;
            assert!((p[0] - 0.2 * (1. + f) * angle.cos()).abs() < 1e-12);
            assert!((p[1] - 0.2 * (1. + f) * angle.sin()).abs() < 1e-12);
            assert!((p[2] - 10. * f).abs() < 1e-12);
        }
        assert!(
            approximate(&profiles, &points, &scale, &twist, opts)
                .unwrap()
                .sections
                .is_some()
        );
        let closed_profiles = [crate::primitives::line([0., 0.1, 0.2], [0., 0.2, 0.3]).unwrap()];
        let closed_points = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]];
        assert!(
            Sweep::new(
                &closed_profiles,
                &closed_points,
                &scale,
                &twist,
                options(true)
            )
            .is_err()
        );
        assert!(
            Sweep::new(
                &closed_profiles,
                &closed_points,
                &scalar(1., 1.),
                &scalar(0., 1.),
                options(true)
            )
            .is_err()
        );
    }
    #[test]
    fn progression_keeps_unaccepted_preview_and_can_resume_without_promoting_it() {
        let profiles = [crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 2.);
        let mut opts = options(false);
        opts.normal = [1., 0., 0.];
        opts.max_steps = 1;
        let result = approximate(&profiles, &points, &scale, &twist, opts).unwrap();
        assert!(result.sections.is_none());
        assert!(!result.levels[0].accepted);
        opts.max_steps = 256;
        let mut sweep = Sweep::new(&profiles, &points, &scale, &twist, opts).unwrap();
        let preview = sweep.next().unwrap().unwrap();
        assert!(!preview.report.accepted);
        assert_eq!(preview.sections.len(), 2);
        let tail = sweep.collect::<Result<Vec<_>>>().unwrap();
        assert!(tail.last().unwrap().report.accepted);
        assert!(
            tail.windows(2)
                .all(|levels| levels[1].report.steps == 2 * levels[0].report.steps)
        );
    }
    #[test]
    fn retained_level_certificate_counts_stored_error_and_discards_partial_budget() {
        let profiles = [crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap()];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let mut o = options(false);
        o.normal = [1., 0., 0.];
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, o).unwrap();
        assert!(
            sweep
                .certify_profile_regularity(1)
                .unwrap()
                .spanwise_regular
        );
        let regularity = sweep.certify_profile_regularity(0).unwrap();
        assert!(!regularity.spanwise_regular && regularity.unresolved_profiles == vec![0]);
        let mut sections = sweep.sections_at(2).unwrap();
        let separation = sweep
            .inspect_wall_separation(&sweep.sections_at(3).unwrap(), 0., 1e-6, 100, 63)
            .unwrap();
        assert!(!separation.all_pairs_separated);
        assert_eq!(separation.separated_pairs, 1);
        assert_eq!(separation.boundary_only_pairs.len(), 2);
        assert!(separation.all_pairs_compatible && separation.unresolved.is_empty());
        let geometry = sweep
            .inspect_wall_geometry(&sweep.sections_at(3).unwrap(), 0., 1e-6, 100, 100, 63)
            .unwrap();
        assert!(geometry.charts_and_pairs_certified, "{geometry:?}");
        assert!(geometry.declared_boundaries_c0);
        assert_eq!(geometry.c0_boundaries, vec![[0, 1], [1, 2]]);
        let geometry = sweep
            .inspect_wall_geometry(&sweep.sections_at(3).unwrap(), 0., 1e-6, 0, 100, 63)
            .unwrap();
        assert!(!geometry.charts_and_pairs_certified);
        assert_eq!(geometry.injectivity_cells, 0);
        assert_eq!(geometry.unresolved_charts, vec![0, 1, 2]);
        assert!(geometry.pairs.all_pairs_compatible);

        let wall = sweep.certify_wall_regularity(&sections, 2).unwrap();
        assert!(wall.spanwise_regular && wall.cells == 2);
        let exhausted = sweep.certify_wall_regularity(&sections, 1).unwrap();
        assert!(!exhausted.spanwise_regular && exhausted.unresolved_patches == vec![[1, 0]]);
        let mut collapsed = sections.clone();
        collapsed[1] = collapsed[0].clone();
        assert!(
            !sweep
                .certify_wall_regularity(&collapsed, 31)
                .unwrap()
                .spanwise_regular
        );

        let c = sweep.certify_level(2, &sections, 10, 4).unwrap();
        assert_eq!(c.status, scalar_certificate::Status::Certified);
        assert!(c.error_upper.unwrap() < 1e-10);
        assert!(c.endpoint_contour_error_upper.unwrap().iter().all(|v|*v>=0. && *v<1e-10));
        let mut altered_end=sections.clone();
        altered_end.last_mut().unwrap()[0].control_points[0][0]+=0.125;
        let endpoints=sweep.certify_level(2,&altered_end,10,4).unwrap().endpoint_contour_error_upper.unwrap();
        assert!(endpoints[0]<1e-10 && endpoints[1]>=0.125);
        let u = sweep.certify_level(2, &sections, 1, 4).unwrap();
        assert_eq!(u.status, scalar_certificate::Status::Unresolved);
        assert!(u.error_upper.is_none());
        assert!(u.endpoint_contour_error_upper.is_none());
        sections[1][0].control_points[0][0] += 0.1;
        assert!(
            sweep
                .certify_level(2, &sections, 10, 4)
                .unwrap()
                .error_upper
                .unwrap()
                >= 0.1
        );
        sections[1][0].weights[0] = 2.;
        assert!(sweep.certify_level(2, &sections, 10, 4).is_err());
    }
    #[test]
    fn wall_regularity_refuses_valid_sections_with_changed_periodicity() {
        let profiles = [Curve {
            degree: 2,
            knots: (0..9).map(|i| i as f64).collect(),
            control_points: vec![
                vec![1., 0., 0.], vec![0., 1., 0.], vec![-1., 0., 0.],
                vec![0., -1., 0.], vec![1., 0., 0.], vec![0., 1., 0.],
            ],
            weights: vec![1.; 6],
            periodic: true,
        }];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let mut o = options(false);
        o.normal = [1., 0., 0.];
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, o).unwrap();
        let mut sections = sweep.sections_at(1).unwrap();
        assert!(sweep.certify_wall_regularity(&sections, 10000).unwrap().spanwise_regular);
        sections[1][0].periodic = false;
        sections[1][0].validate().unwrap();
        assert!(sweep.certify_wall_regularity(&sections, 10000).is_err());
    }
    #[test]
    fn certified_closed_rational_levels_cover_profile_interior_and_cyclic_ownership() {
        let profiles = [crate::paths::bezier(
            vec![vec![0., 0.1, 0.2], vec![0., 0.3, 0.4], vec![0., 0.2, 0.1]],
            Some(vec![1., 0.7, 1.3]),
        )
        .unwrap()];
        let points = [
            [0., 0., 0.],
            [10., 0., 0.],
            [10., 10., 4.],
            [0., 10., 1.],
            [0., 5., -2.],
        ];
        let mut scale = crate::paths::bezier(
            vec![vec![1., 0., 0.], vec![1.5, 0., 0.], vec![1., 0., 0.]],
            Some(vec![1., 0.8, 1.]),
        )
        .unwrap();
        scale.knots.iter_mut().for_each(|k| *k = 3. + 4. * (*k));
        let mut twist = scalar(0., std::f64::consts::TAU);
        twist.knots.iter_mut().for_each(|k| *k = -2. + 4. * (*k));
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, options(true)).unwrap();
        let mut previous = f64::INFINITY;
        for steps in [8, 16] {
            let sections = sweep.sections_at(steps).unwrap();
            let certificate = sweep.certify_level(steps, &sections, 10000, 16).unwrap();
            let regularity = sweep.certify_wall_regularity(&sections, 10000).unwrap();
            if steps == 8 {
                let geometry = sweep
                    .inspect_wall_geometry(&sections, 0., 1e-6, 0, 10000, 256)
                    .unwrap();
                assert!(!geometry.charts_and_pairs_certified);
                assert_eq!(geometry.unresolved_charts.len(), sections.len() - 1);
                assert_eq!(geometry.injectivity_cells, 0);
                assert!(geometry.declared_boundaries_c0);
                assert_eq!(geometry.c0_boundaries.len(), sections.len() - 1);
                let separation = sweep
                    .inspect_wall_separation(&sections, 0., 1e-6, 10000, 256)
                    .unwrap();
                assert!(!separation.all_pairs_separated);
                assert!(
                    separation
                        .unresolved
                        .iter()
                        .any(|p| p.patches == [0, sections.len() - 2]
                            && p.reason == "shared-boundary-interior-separation-unproved")
                );
            }

            assert!(regularity.spanwise_regular, "{regularity:?}");
            assert!(regularity.cells <= 10000);

            assert_eq!(
                certificate.status,
                scalar_certificate::Status::Certified,
                "{certificate:?}"
            );
            let upper = certificate.error_upper.unwrap();
            assert!(upper < previous && upper < 0.1, "{upper}");
            previous = upper;
            for edge in 0..points.len() {
                for i in 0..steps {
                    for local in [0.13, 0.37, 0.81] {
                        let f = (i as f64 + local) / steps as f64;
                        let ideal = sweep.station(edge, f).unwrap();
                        let index = edge * steps + i;
                        for u in [0.17, 0.53, 0.91] {
                            let a = sections[index][0].evaluate(u).unwrap().point;
                            let b = sections[index + 1][0].evaluate(u).unwrap().point;
                            let exact = ideal[0].evaluate(u).unwrap().point;
                            let error = norm(std::array::from_fn(|k| {
                                exact[k] - ((1. - local) * a[k] + local * b[k])
                            }));
                            assert!(error <= upper, "{error}>{upper} edge{edge} interval{i}");
                        }
                    }
                }
            }
            let mut altered = sections.clone();
            altered.last_mut().unwrap()[0].control_points[0][0] += 0.25;
            assert!(
                sweep
                    .inspect_wall_separation(&altered, 0., 1e-6, 10000, 256)
                    .is_err()
            );
            assert!(
                sweep
                    .certify_level(steps, &altered, 10000, 16)
                    .unwrap()
                    .error_upper
                    .unwrap()
                    >= 0.25
            );
        }
    }
    #[test]
    fn admission_requires_certified_rounding_bound_even_when_samples_match() {
        let profiles = [crate::primitives::line([1e8 + 1., 0., 0.], [1e8 + 2., 0., 0.]).unwrap()];
        let points = [[1e8, 0., 0.], [1e8, 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let mut o = options(false);
        o.normal = [1., 0., 0.];
        o.max_deviation = 1e-12;
        let sweep = Sweep::new(&profiles, &points, &scale, &twist, o).unwrap();
        let level = sweep.preview_at(1).unwrap();
        assert!(level.report.sampled_control_deviation < 1e-12);
        assert!(level.report.certified_error_upper.unwrap() > o.max_deviation);
        assert!(!level.report.accepted);
        o.max_deviation = 1e-4;
        let level = Sweep::new(&profiles, &points, &scale, &twist, o)
            .unwrap()
            .preview_at(1)
            .unwrap();
        assert!(level.report.accepted);
        assert!(level.report.certified_error_upper.unwrap() <= o.max_deviation);
    }
    #[test]
    fn unproved_profile_regularity_refuses_even_a_certified_zero_error_level() {
        let profile = crate::paths::bezier(vec![vec![1., 0., 0.], vec![1., 0., 0.]], None).unwrap();
        let profiles = [profile];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let scale = scalar(1., 1.);
        let twist = scalar(0., 0.);
        let mut o = options(false);
        o.normal = [1., 0., 0.];
        let level = Sweep::new(&profiles, &points, &scale, &twist, o)
            .unwrap()
            .preview_at(1)
            .unwrap();
        assert!(level.report.certified_error_upper.unwrap() < o.max_deviation);
        assert!(!level.report.profile_regularity_certified && !level.report.accepted);
        assert_eq!(level.report.wall_regularity_certified, None);
    }
