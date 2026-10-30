use super::*;
#[test]
fn cross_profile_path_preserves_exact_normal_diameter() {
    // Positive-weight single-span rational cubic with opposite endpoints
    // and vertical endpoint tangents: the diameter is normal at both ends.
    let surface=crate::surface::Surface {
        degree_u:1,degree_v:3,knots_u:vec![0.,0.,1.,1.],
        knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],
        control_points:[0.,1.].into_iter().map(|x|
            [[1.,0.],[1.,2.],[-1.,2.],[-1.,0.]].into_iter().map(|p|vec![x,p[0],p[1]]).collect()).collect(),
        weights:vec![vec![1.,1./3.,1./3.,1.];2],periodic_u:false,periodic_v:false,
    };
    let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],8).unwrap().0.unwrap();
    let bridge=crate::normal_alignment::ProfileSectionBridge::new(
        profile.section(0,0.).unwrap(),profile.section(0,1.).unwrap(),[1.;2]).unwrap();
    let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
    let a=[[0.4,0.41],[0.,0.001]];let b=[[0.999,1.],[0.4,0.41]];
    let diameter=[[[0.,0.001],[0.4,0.41]],b];
    let mut self_workspace=corrected.profile_chord_workspace();
    assert!(self_workspace.inspect_corrected_pair(diameter,1e-4,1000).unwrap().0!=Some(true));
    for budget in [0,1,64,1000] {
        let (result,spent)=self_workspace.inspect_corrected_pair(diameter,1e-4,budget).unwrap();
        assert!(result!=Some(true) && spent<=budget);
        let reverse=[diameter[1],diameter[0]];
        let (result,spent)=self_workspace.inspect_corrected_pair(reverse,1e-4,budget).unwrap();
        assert!(result!=Some(true) && spent<=budget,
            "reversing the coordinate paths must preserve an exact normal diameter");
    }
    let mut workspace=corrected.profile_chord_workspace();
    for high in [false,true,false,true] {
        assert!(workspace.inspect(a,b,high,1e-4,1000).unwrap().0!=Some(true));
        for budget in [0,1,64,1000] {
            let (cached,cost)=workspace.inspect(a,b,high,1e-4,budget).unwrap();
            assert!(cached!=Some(true) && cost<=budget);
            let (excluded,spent)=corrected.inspect_profile_endpoint_chord(a,b,high,1e-4,budget).unwrap();
            assert!(excluded!=Some(true));assert!(spent<=budget);
        }
    }
}
#[test]
fn cross_profile_cache_allows_larger_budget_direction_retry() {
    let surface=crate::surface::Surface {
        degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.];2],vec![vec![0.5,0.,0.];2],
            vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.,4.];3],periodic_u:false,periodic_v:false,
    };
    let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],6).unwrap().0.unwrap();
    let domain=[[0.01,0.99],[0.5,0.5]];let mut cache=ProfileDerivativeCache::default();
    let small=cache.profile_direction(&profile,domain,1).unwrap();
    assert!(small.0.unwrap().iter().all(|d|d[0]<=0. && d[1]>=0.));
    assert!(cache.directions.is_empty());
    let large=cache.profile_direction(&profile,domain,100).unwrap();
    assert!(large.0.unwrap()[2][0]>0. && large.1>small.1);
    assert_eq!(cache.profile_direction(&profile,domain,1).unwrap(),(large.0,0));
}
#[test]
fn cross_profile_cache_does_not_mix_equal_uv_on_distinct_owners() {
    let first=crate::surface::Surface {
        degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:[0.,1.].into_iter().map(|u|vec![vec![u,0.,0.],vec![u,1.,0.]]).collect(),
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
    };
    let mut second=first.clone();
    for row in &mut second.control_points {for p in row {p[2]=10.+2.*p[0];}}
    let a=crate::surface_weight_profile::qualify_weight_profile(&first,[0,0],100).unwrap().0.unwrap();
    let b=crate::surface_weight_profile::qualify_weight_profile(&second,[0,0],100).unwrap().0.unwrap();
    let independent=crate::normal_alignment::ProfileSectionBridge::new(
        a.section(0,0.).unwrap(),b.section(0,1.).unwrap(),[1.;2]).unwrap();
    assert!(independent.constrain_original_rails(100).unwrap().0.is_none(),
        "The current public corrected bridge refuses different profile owners");
    let twin=crate::surface_weight_profile::qualify_weight_profile(&first,[0,0],100).unwrap().0.unwrap();
    let equal_geometry=crate::normal_alignment::ProfileSectionBridge::new(
        a.section(0,0.).unwrap(),twin.section(0,1.).unwrap(),[1.;2]).unwrap();
    assert!(equal_geometry.constrain_original_rails(100).unwrap().0.is_none(),
        "Equal source geometry still does not transfer profile ownership");
    let domain=[[0.1,0.9];2];let mut cache=ProfileDerivativeCache::default();
    let ja=a.derivative_bounds(domain,100).unwrap().0;
    let jb=b.derivative_bounds(domain,100).unwrap().0;
    assert_ne!(ja,jb);
    assert_eq!(cache.enclose(&a,domain,100).unwrap().0,ja);
    assert_eq!(cache.enclose(&b,domain,100).unwrap().0,jb);
    for owner in [&a,&b,&a,&b] {
        assert_eq!(cache.enclose(owner,domain,100).unwrap().0,owner.derivative_bounds(domain,100).unwrap().0);
        assert_eq!(cache.position(owner,domain,100).unwrap().0,owner.enclose(domain,100).unwrap().0);
        assert_eq!(cache.profile_direction(owner,domain,100).unwrap().0,owner.interior_normal_direction_bounds(domain,100).unwrap().0);
    }
    assert!(cache.hits>0);
}
#[test]
fn cross_profile_cache_never_stores_incomplete_span_hulls() {
    let surface=crate::surface::Surface {
        degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,0.5,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:[0.,0.25,0.75,1.].into_iter().map(|x|
            vec![vec![x,0.,0.],vec![x,1.,0.]]).collect(),
        weights:vec![vec![1.;2];4],periodic_u:false,periodic_v:false,
    };
    let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],8).unwrap().0.unwrap();
    let mut cache=ProfileDerivativeCache::default();let domain=[[0.,1.];2];
    assert_eq!(cache.enclose(&profile,domain,1).unwrap(),(None,1));
    assert!(cache.entries.is_empty());
    let complete=cache.enclose(&profile,domain,2).unwrap();assert!(complete.0.is_some());
    assert_eq!(complete.1,2);assert_eq!(cache.hits,0);
    assert_eq!(cache.enclose(&profile,domain,1).unwrap(),(complete.0,0));
    assert_eq!(cache.hits,1);
    assert_eq!(cache.position(&profile,domain,1).unwrap(),(None,1));
    assert!(cache.positions.is_empty());
    let position=cache.position(&profile,domain,2).unwrap();assert!(position.0.is_some());
    assert_eq!(position.1,2);
    assert_eq!(cache.position(&profile,domain,1).unwrap(),(position.0,0));
}
#[test]
fn cross_profile_path_refuses_unqualified_free_derivative_seam() {
    let surface=crate::surface::Surface {
        degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],
        knots_v:vec![0.,0.,0.5,1.,1.],
        control_points:[0.,1.].into_iter().map(|x|
            [0.,0.4,1.].into_iter().map(|y|vec![x,y,0.]).collect()).collect(),
        weights:vec![vec![1.;3];2],periodic_u:false,periodic_v:false,
    };
    let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],8).unwrap().0.unwrap();
    let bridge=crate::normal_alignment::ProfileSectionBridge::new(
        profile.section(0,0.).unwrap(),profile.section(0,1.).unwrap(),[1.;2]).unwrap();
    assert!(bridge.constrain_original_rails(100).unwrap().0.is_none(),
        "unadmitted source derivative seams cannot supply a corrected cross-chart owner");
    let mut disconnected=surface.clone();
    disconnected.knots_v=vec![0.,0.,0.5,0.5,1.,1.];
    for row in &mut disconnected.control_points {row.insert(2,vec![row[0][0],0.6,0.]);}
    disconnected.weights=vec![vec![1.;4];2];
    assert!(disconnected.validate().is_err());
}
#[test]
fn cross_profile_path_preserves_nonunit_parameter_domains() {
    let surface=crate::surface::Surface {
        degree_u:1,degree_v:1,knots_u:vec![2.,2.,4.,4.],knots_v:vec![-3.,-3.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],
            vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
    };
    let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],4).unwrap().0.unwrap();
    for axis in [0,1] {
        let cuts=if axis==0 {[3.,3.5]}else{[-2.,-1.]};
        let bridge=crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(axis,cuts[0]).unwrap(),profile.section(axis,cuts[1]).unwrap(),[cuts[1]-cuts[0];2]).unwrap();
        let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
        let source=[[2.125,2.25],[-2.75,-2.5]];
        let target=[if axis==0 {[-0.25,0.]}else{[3.5,3.75]},[0.25,0.5]];
        for high in [false,true] {
            let (excluded,spent)=corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,1000).unwrap();
            assert_eq!(excluded,Some(true));assert!(spent<=1000);
            assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,0).unwrap(),(None,0));
        }
    }
}
#[test]
fn owned_cross_chart_proof_covers_zero_separation_at_both_seam_ends() {
    let surface=crate::surface::Surface {
        degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
    };
    let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],4).unwrap().0.unwrap();
    for axis in [0,1] {
        let bridge=crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(axis,0.4).unwrap(),profile.section(axis,0.6).unwrap(),[0.6-0.4;2]).unwrap();
        let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
        for high in [false,true] {for epsilon in [0.1,1e-6] {
            let mut source=[[0.45,0.55];2];
            source[axis]=if high {[0.6,0.6+epsilon]}else{[0.4-epsilon,0.4]};
            let target=[[0.45,0.55],if high {[1.-epsilon,1.]}else{[0.,epsilon]}];
            let a=profile.enclose(source,1000).unwrap().0.unwrap();
            let b=corrected.enclose_with_transverse_correction(target[0],target[1],1000).unwrap().0.unwrap();
            let chord=std::array::from_fn(|k| {
                let d=I::new(b[k][0],b[k][1]).unwrap().sub(I::new(a[k][0],a[k][1]).unwrap()).unwrap();
                [d.lo,d.hi]
            });
            assert!(chord.iter().all(|d|d[0]<=0. && d[1]>=0.));
            assert_ne!(crate::normal_alignment::inspect_pair_bounds([
                [[0.,0.],[0.,0.],[1.,1.]],chord],1e-4).unwrap().aligned,Some(false));
            assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,1000).unwrap().0,Some(true));
            assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,0).unwrap(),(None,0));
        }}
    }
}
#[test]
fn owned_cross_chart_proof_covers_curved_seam_zero_separation() {
    for rational in [false,true] {
    let surface=crate::surface::Surface {
        degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],
            vec![vec![0.5,0.,0.],vec![0.5,1.,0.]],
            vec![vec![1.,0.,0.01],vec![1.,1.,0.01]]],
        weights:if rational {vec![vec![1.,1.125],vec![1.25,1.40625],vec![1.,1.125]]} else {vec![vec![1.;2];3]},periodic_u:false,periodic_v:false,
    };
    let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],6).unwrap().0.unwrap();
    for axis in [0,1] {
        let bridge=crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(axis,0.4).unwrap(),profile.section(axis,0.6).unwrap(),[0.6-0.4;2]).unwrap();
        let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
        for high in [false,true] {for epsilon in [0.1,1e-6] {
            let mut source=[[0.45,0.55];2];
            source[axis]=if high {[0.6,0.6+epsilon]}else{[0.4-epsilon,0.4]};
            let target=[[0.45,0.55],if high {[1.-epsilon,1.]}else{[0.,epsilon]}];
            let a=profile.enclose(source,1000).unwrap().0.unwrap();
            let b=corrected.enclose_with_transverse_correction(target[0],target[1],1000).unwrap().0.unwrap();
            let chord=std::array::from_fn(|k| {
                let d=I::new(b[k][0],b[k][1]).unwrap().sub(I::new(a[k][0],a[k][1]).unwrap()).unwrap();
                [d.lo,d.hi]
            });
            assert!(chord.iter().all(|d|d[0]<=0. && d[1]>=0.));
            let normal=profile.interior_normal_direction_bounds(source,1000).unwrap().0.unwrap();
            assert_ne!(crate::normal_alignment::inspect_pair_bounds([normal,chord],1e-4).unwrap().aligned,Some(false));
            assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,1000).unwrap().0,Some(true));
            assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,0).unwrap(),(None,0));
        }}
    }
}
}
#[test]
fn owned_profile_transition_chord_uses_exact_section_and_budget() {
    let surface=crate::surface::Surface {
        degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],
            vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
    };
    let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],4).unwrap().0.unwrap();
    for axis in [0,1] {
        let bridge=crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(axis,0.4).unwrap(),profile.section(axis,0.6).unwrap(),[0.6-0.4;2]).unwrap();
        let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
        let mut shared=corrected.profile_chord_workspace();
        let shared_domain=[[0.1,0.2],[0.3,0.4]];
        assert_eq!(shared.enclose_position(shared_domain,1).unwrap(),(None,1));
        let position=shared.enclose_position(shared_domain,1000).unwrap();
        assert_eq!(position,corrected.enclose_with_transverse_correction(shared_domain[0],shared_domain[1],1000).unwrap());
        assert!(position.0.is_some());
        assert_eq!(shared.enclose_position(shared_domain,1).unwrap(),(position.0,0));
        assert_eq!(shared.enclose_position(shared_domain,0).unwrap(),(None,0));
        let partial_jet=shared.enclose_derivatives(shared_domain,1).unwrap();
        assert!(partial_jet.0.is_none() && partial_jet.1<=1);
        assert!(shared.derivatives.corrected.is_empty());
        let derivatives=shared.enclose_derivatives(shared_domain,1000).unwrap();
        assert_eq!(derivatives,corrected.derivatives_with_transverse_correction(shared_domain[0],shared_domain[1],1000).unwrap());
        assert!(derivatives.0.is_some());
        assert_eq!(shared.enclose_derivatives(shared_domain,1).unwrap(),(derivatives.0,0));
        assert_eq!(shared.enclose_derivatives(shared_domain,0).unwrap(),(None,0));
        for invalid in [[[-0.1,0.2],[0.3,0.4]],[[f64::NAN,0.2],[0.3,0.4]]] {
            assert!(shared.enclose_position(invalid,0).is_err());
            assert!(shared.enclose_derivatives(invalid,0).is_err());
        }
        assert!(shared.enclose_position(shared_domain,100001).is_err());
        assert!(shared.enclose_derivatives(shared_domain,100001).is_err());
        let mut position_cache=ProfileDerivativeCache::default();
        let position_domain=[[0.1,0.2],[0.3,0.4]];
        let incomplete=position_cache.corrected_position(&corrected,position_domain,1).unwrap();
        assert_eq!(incomplete,(None,1));assert!(position_cache.corrected_positions.is_empty());
        let complete=position_cache.corrected_position(&corrected,position_domain,1000).unwrap();
        assert!(complete.0.is_some());assert!(complete.1>1 && complete.1<=1000);
        assert_eq!(position_cache.corrected_position(&corrected,position_domain,1).unwrap(),(complete.0,0));
        let pair=[[[0.1,0.2];2],[[0.8,0.9];2]];
        let mut self_workspace=corrected.profile_chord_workspace();
        let cold=self_workspace.inspect_corrected_pair(pair,1e-4,1000).unwrap();
        assert_eq!(cold.0,Some(true));
        assert_eq!(cold,corrected.inspect_self_chord_pair(pair,1e-4,1000).unwrap());
        let union=[[0.1,0.9];2];
        let direct_intrinsic=corrected.inspect_self_chords(union,1e-4,1000).unwrap();
        assert!(direct_intrinsic.0.is_some() && direct_intrinsic.1>0);
        let shared_intrinsic=self_workspace.inspect_self_chords(union,1e-4,1000).unwrap();
        assert_eq!(shared_intrinsic,(direct_intrinsic.0,0));
        assert_eq!(self_workspace.inspect_self_chords(union,1e-4,0).unwrap(),(None,0));
        assert!(self_workspace.inspect_self_chords([[-0.1,0.9];2],1e-4,0).is_err());
        assert!(self_workspace.inspect_self_chords(union,f64::NAN,0).is_err());
        let mut fresh_intrinsic=corrected.profile_chord_workspace();
        assert_eq!(fresh_intrinsic.inspect_self_chords(union,1e-4,1000).unwrap(),direct_intrinsic);

        let mut seeded=corrected.profile_chord_workspace();
        for endpoint in pair {
            assert!(seeded.enclose_position(endpoint,1000).unwrap().0.is_some());
            assert!(seeded.enclose_derivatives(endpoint,1000).unwrap().0.is_some());
        }
        let shared_pair=seeded.inspect_corrected_pair(pair,1e-4,1000).unwrap();
        assert_eq!(shared_pair.0,cold.0);
        assert!(shared_pair.1<cold.1,"Endpoint-pair work must reuse the already paid wall enclosures");
        assert!(seeded.cache_hits()>0);
        assert_eq!(self_workspace.inspect_corrected_pair(pair,1e-4,1).unwrap(),(cold.0,0));
        assert_eq!(self_workspace.inspect_corrected_pair(pair,1e-4,0).unwrap(),(None,0));
        let source=[[0.1,0.2];2];let target=[[0.7,0.8],[0.3,0.4]];
        for high in [false,true] {
            let (excluded,spent)=corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,100).unwrap();
            assert_eq!(excluded,Some(true));assert!(spent<=100);
            assert!(spent<66,"The complete first path must exclude before optional alternate-path work");
            let mut workspace=corrected.profile_chord_workspace();
            let first=workspace.inspect(source,target,high,1e-4,1000).unwrap();
            let second=workspace.inspect(source,target,high,1e-4,1000).unwrap();
            assert_eq!(first.0,second.0);assert!(second.1<first.1);
            assert!(workspace.cache_hits()>0);
            assert_eq!(workspace.inspect(source,target,high,1e-4,0).unwrap(),(None,0));
            assert!(workspace.inspect([[-0.1,0.2];2],target,high,1e-4,0).is_err());
            let mut independent=corrected.profile_chord_workspace();
            assert_eq!(independent.inspect(source,target,high,1e-4,1000).unwrap(),first);
            let mut partial=ProfileDerivativeCache::default();
            let exhausted=partial.corrected_jet(&corrected,target,1).unwrap();
            assert!(exhausted.0.is_none() && exhausted.1<=1);
            assert!(partial.corrected.is_empty());
            let completed=partial.corrected_jet(&corrected,target,1000).unwrap();
            assert!(completed.0.is_some());
            assert_eq!(partial.corrected_jet(&corrected,target,1).unwrap(),(completed.0,0));
            let actual=corrected.normals_with_transverse_correction(target[0],target[1],1000).unwrap().0;
            assert_eq!(workspace.derivatives.corrected_normal(&corrected,target,1000).unwrap().0,actual);
            assert_eq!(workspace.derivatives.corrected_normal(&corrected,target,1).unwrap(),(actual,0));
            let mut overlap=[[0.;2];2];overlap[axis]=[0.46,0.48];overlap[1-axis]=target[0];
            assert_eq!(corrected.inspect_profile_endpoint_chord(overlap,target,high,1e-4,1000).unwrap().0,Some(false));
            assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,0).unwrap(),(None,0));
            for budget in 1..=8 {
                let (result,used)=corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,budget).unwrap();
                assert!(used<=budget);
                if used<spent {assert!(result!=Some(true));}
            }
            assert!(corrected.inspect_profile_endpoint_chord(source,target,high,1.,0).is_err());
            assert!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,100001).is_err());
            assert!(corrected.inspect_profile_endpoint_chord([[0.1;2];2],target,high,1e-4,0).is_err());
            assert!(corrected.inspect_profile_endpoint_chord([[-0.1,0.2];2],target,high,1e-4,0).is_err());
            assert!(corrected.inspect_profile_endpoint_chord(source,[[0.7,0.8],[0.9,1.1]],high,1e-4,0).is_err());
        }
    }
}
#[test]
fn law_jets_preserve_nonunit_parameter_scaling_and_work_refusal() {
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 4., 4.],
        control_points: vec![vec![1., 2., 3.], vec![5., 6., 7.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    curve.validate().unwrap();
    let (derivative, spent) =
        law_derivative(&curve, I::new(2.5, 3.5).unwrap(), [2.5, 3.5], 1).unwrap();
    assert_eq!(spent, 1);
    for d in derivative.unwrap() {
        assert!(d.lo <= 2. && d.hi >= 2.);
        assert!(d.hi - d.lo < 1e-10);
    }
    assert!(
        law_derivative(&curve, I::new(2.5, 3.5).unwrap(), [2.5, 3.5], 0)
            .unwrap()
            .0
            .is_none()
    );
}
#[test]
fn endpoint_position_refinement_preserves_translated_scale() {
    let w=std::f64::consts::FRAC_1_SQRT_2;
    for scale in [1e-3,1.,1e3] {
        for far in [false,true] {
        let shift=if far {[1e6,-2e6,3e6]}else{[0.;3]};
        let surface=crate::surface::Surface {
            degree_u:1,degree_v:2,knots_u:vec![0.,0.,1.,1.],
            knots_v:vec![0.,0.,0.,1.,1.,1.],
            control_points:[0.,1.].into_iter().map(|x|
                [[x,1.,0.],[x,1.,1.],[x,0.,1.]].into_iter().map(|p|
                    (0..3).map(|k|shift[k]+scale*p[k]).collect()).collect()).collect(),
            weights:vec![vec![1.,w,1.];2],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],6).unwrap().0.unwrap();
        // Exact extrusion derivative dX/du=scale; translation must not
        // widen its enclosure by the absolute coordinate magnitude.
        let (jet,spent)=profile.derivative_bounds([[0.1,0.11],[0.1,0.11]],100).unwrap();
        let dx=jet.unwrap()[0][0];assert_eq!(spent,1);
        assert!(dx[0]>0.5*scale && dx[1]<2.*scale,"centered derivative scale={scale} far={far} dx={dx:?}");
        // X is exactly linear in u and independent of v. Higher axial
        // partials must contain zero without absorbing the translation.
        let rectangle=[[0.1,0.11],[0.1,0.11]];
        let normal=profile.normal_bounds(rectangle,100).unwrap();assert_eq!(normal.1,1);
        let axial_normal=normal.0.unwrap()[0];
        assert!(axial_normal[0]<=0. && axial_normal[1]>=0.
            && axial_normal[1]-axial_normal[0]<1e-6*scale*scale,
            "centered axial normal scale={scale} far={far} bounds={axial_normal:?}");
        let second=profile.second_derivative_bounds(rectangle,[0,1],100).unwrap();
        assert_eq!(second.1,1);let mixed=second.0.unwrap()[0];
        assert!(mixed[0]<=0. && mixed[1]>=0. && mixed[1]-mixed[0]<scale);
        let higher=profile.higher_derivative_bounds(rectangle,[2,1],100).unwrap();
        assert_eq!(higher.1,1);let axial=higher.0.unwrap()[0];
        assert!(axial[0]<=0. && axial[1]>=0. && axial[1]-axial[0]<scale);

        let bridge=crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(0,0.).unwrap(),profile.section(0,1.).unwrap(),[1.;2]).unwrap();
        let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
        for high in [false,true] {
            let source=[[0.1,0.11],[0.1,0.11]];let target=[[0.11,0.12],[0.6,0.61]];
            let mut workspace=corrected.profile_chord_workspace();
            let fresh=workspace.inspect(source,target,high,1e-4,1000).unwrap();
            assert_eq!(fresh.0,Some(true),"scale={scale} high={high} far={far}");
            assert!(fresh.1<=1000);
            assert_eq!(workspace.inspect(source,target,high,1e-4,1000).unwrap().0,fresh.0);
            assert_eq!(workspace.inspect(source,target,high,1e-4,0).unwrap(),(None,0));
        }
    }
}
}
#[test]
fn rational_cylinder_radius_covers_whole_corrected_expression() {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let surface = crate::surface::Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: [0., 1.]
            .into_iter()
            .map(|x| vec![vec![x, 1., 0.], vec![x, 1., 1.], vec![x, 0., 1.]])
            .collect(),
        weights: vec![vec![1., w, 1.]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let profile = crate::normal_alignment::qualify_weight_profile(&surface, [0, 0], 6)
        .unwrap()
        .0
        .unwrap();
    let bridge = crate::normal_alignment::ProfileSectionBridge::new(
        profile.section(0, 0.).unwrap(),
        profile.section(0, 1.).unwrap(),
        [1.; 2],
    )
    .unwrap();
    let corrected = bridge.constrain_original_rails(100).unwrap().0.unwrap();
    let mut shared=corrected.profile_chord_workspace();
    for domain in [[[0.05,0.2],[0.1,0.3]],[[0.25,0.5],[0.4,0.6]],[[0.7,0.95],[0.75,0.9]]] {
        let position=corrected.enclose_with_transverse_correction(domain[0],domain[1],1000).unwrap();
        let jet=corrected.derivatives_with_transverse_correction(domain[0],domain[1],1000).unwrap();
        assert!(position.0.is_some() && jet.0.is_some());
        assert_eq!(shared.enclose_position(domain,1000).unwrap(),position);
        let shared_jet=shared.enclose_derivatives(domain,1000).unwrap();
        assert_eq!(shared_jet.0,jet.0);
        assert!(shared_jet.1<=jet.1,"Retained source hulls must not increase work");
        assert_eq!(shared.enclose_position(domain,1).unwrap(),(position.0,0));
        assert_eq!(shared.enclose_derivatives(domain,1).unwrap(),(jet.0,0));
        let intrinsic=corrected.inspect_self_chords(domain,1e-4,1000).unwrap();
        assert!(intrinsic.0.is_some());
        assert_eq!(shared.inspect_self_chords(domain,1e-4,1000).unwrap(),(intrinsic.0,0));
        assert_eq!(shared.inspect_self_chords(domain,1e-4,0).unwrap(),(None,0));
    }
    for high in [false,true] {
        let a=if high {0.99}else{0.001};let b=if high {0.999}else{0.01};
        let (excluded,spent)=corrected.inspect_profile_endpoint_chord(
            [[a,a+1e-6],[0.,1e-6]],[[0.,1e-6],[b,b+1e-6]],high,1e-4,1000).unwrap();
        assert_eq!(excluded,Some(true));assert!(spent<=1000);
        assert_eq!(corrected.inspect_profile_endpoint_chord(
            [[0.1,0.11],[0.1,0.11]],[[0.11,0.12],[0.6,0.61]],high,1e-4,1000).unwrap().0,Some(true),
            "axial separation >= 0.49 with zero axial normal excludes alignment");
    }
    let centers = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let radius = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let report = corrected
        .inspect_radius(&centers, &radius, 0.1, 10000, 1000000)
        .unwrap();
    assert!(
        report.complete,
        "{} cells={} spans={} pending={}",
        report.reason,
        report.cells,
        report.spans,
        report.pending.len()
    );
    assert!(report.pending.is_empty());
    assert_eq!(report.pending_parameter_fraction(), 0.);
    assert!(report.accepted_error_upper <= 0.1);
    let mut extended = centers.clone();
    extended.control_points[0][0] = -1.;
    extended.control_points[1][0] = 2.;
    let restricted = corrected
        .inspect_radius_on_domains(
            &extended,
            &radius,
            [[1. / 3., 2. / 3.], [0.25, 0.75]],
            0.1,
            10000,
            1000000,
        )
        .unwrap();
    assert!(restricted.complete);
    assert!(restricted.pending.is_empty());
    assert!(corrected
        .inspect_radius_on_domains(&centers, &radius, [[-0.1, 1.], [0., 1.]], 0.1, 100, 10000)
        .is_err());
    assert!(corrected
        .inspect_radius_on_domains(&centers, &radius, [[0.5, 0.5], [0., 1.]], 0.1, 100, 10000)
        .is_err());
    for (cells, spans) in [(0, 1000), (100, 0), (1, 1)] {
        let refused = corrected
            .inspect_radius(&centers, &radius, 0.1, cells, spans)
            .unwrap();
        assert!(!refused.complete);
        assert!(!refused.pending.is_empty());
        assert_eq!(refused.pending_parameter_fraction(), 1.);
    }
    let mut wrong = radius.clone();
    // Radius equals the cylinder at the center but differs at interior
    // quarter parameters. A center-only refutation probe misses this law.
    let off_center_law = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1.2, 0.], vec![0.8, 0.], vec![1.2, 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let off_center = corrected
        .inspect_radius(&centers, &off_center_law, 0.01, 100, 10000)
        .unwrap();
    assert_eq!(off_center.reason, "expression-radius-law-mismatch");
    assert!(!off_center.complete);
    let violation = off_center.violation.as_ref().unwrap();
    assert!(violation.error_bounds[0] > 0.01);
    assert_eq!(off_center.cells, 1);
    assert_eq!(violation.domain[1][0], violation.domain[1][1]);
    assert_ne!(violation.domain[1][0], 0.5);
    for p in &mut wrong.control_points {
        p[0] = 2.;
    }
    let refused = corrected
        .inspect_radius(&centers, &wrong, 0.1, 100, 10000)
        .unwrap();
    assert!(!refused.complete);
    assert!(!refused.pending.is_empty());
    assert_eq!(refused.reason, "expression-radius-law-mismatch");
    let violation = refused.violation.unwrap();
    assert!(violation.error_bounds[0] > 0.1);
    assert!(violation.error_bounds[0] <= 1. && violation.error_bounds[1] >= 1.);
    let jump = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.5, 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 0.], vec![2., 0.], vec![2., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(jump.validate().is_err());
    assert!(corrected
        .inspect_radius(&centers, &jump, 0.1, 100, 1000)
        .is_err());
    let (mean, spent) = corrected
        .mean_radius_residual(
            &centers,
            &jump,
            [[0., 1.]; 2],
            [[0., 1.]; 2],
            [[0.; 2]; 3],
            &[I::point(0.); 3],
            I::point(1.),
            1000,
        )
        .unwrap();
    assert!(mean.is_none());
    assert_eq!(spent, 0);
}
