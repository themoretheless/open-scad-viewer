use super::*;
#[test]
fn side_remainder_cache_reuses_complete_source_hulls_without_changing_jets() {
    let surface = crate::surface::Surface {
        degree_u: 2, degree_v: 1,
        knots_u: vec![0.,0.,0.,1.,1.,1.], knots_v: vec![0.,0.,1.,1.],
        control_points: [0.,0.5,1.].into_iter().map(|u|
            vec![vec![u,0.,u*u],vec![u,1.,u*u]]).collect(),
        weights: [1.,0.75,1.25].into_iter().map(|w|vec![w;2]).collect(),
        periodic_u: false, periodic_v: false,
    };
    let profile = crate::normal_alignment::qualify_weight_profile(&surface,[0,0],100).unwrap().0.unwrap();
    let bridge = ProfileSectionBridge::new(profile.section(0,0.).unwrap(),
        profile.section(0,1.).unwrap(),[1.;2]).unwrap();
    let constrained = bridge.constrain_original_rails(100).unwrap().0.unwrap();
    let mut cache = SideRemainderCache::default();
    assert_eq!(constrained.derivatives_with_transverse_correction_cached(
        [0.2,0.8],[0.1,0.4],0,&mut cache).unwrap(),(None,0));
    assert!(cache.entries.is_empty());
    assert!(cache.bridge_data.is_empty());
    assert!(cache.rail_positions.is_empty());
    let cold = constrained.derivatives_with_transverse_correction_cached(
        [0.2,0.8],[0.1,0.4],1000,&mut cache).unwrap();
    assert!(cold.0.is_some()); assert_eq!(cache.entries.len(),4);
    assert_eq!(cache.bridge_data.len(),1);
    assert_eq!(cache.rail_positions.len(),4);
    let direct_position=constrained.enclose_with_transverse_correction([0.3,0.7],[0.1,0.4],1000).unwrap();
    let shared_position=constrained.enclose_with_transverse_correction_cached([0.3,0.7],[0.1,0.4],1000,Some(&cache)).unwrap();
    assert_eq!(shared_position.0,direct_position.0);
    assert!(shared_position.1<direct_position.1,"Complete rail positions must survive a free-only domain change");
    for across in [[0.5,0.9],[0.25,0.75]] {
        let direct=bridge.enclose([0.2,0.8],across,1000).unwrap();
        let reused=bridge.enclose_cached([0.2,0.8],across,1000,Some(&cache)).unwrap();
        assert_eq!(reused.0,direct.0);
        assert!(reused.1<direct.1);
        assert_eq!(bridge.enclose_cached([0.2,0.8],across,0,Some(&cache)).unwrap(),(None,0));
        let direct=constrained.enclose_with_transverse_correction([0.2,0.8],across,1000).unwrap();
        let reused=constrained.enclose_with_transverse_correction_cached([0.2,0.8],across,1000,Some(&cache)).unwrap();
        assert_eq!(reused.0,direct.0);
        assert!(reused.1<direct.1);
    }
    for across in [[0.5,0.9],[0.25,0.75]] {
        let direct=bridge.derivative_bounds_impl([0.2,0.8],across,1000,true).unwrap();
        let reused=bridge.derivative_bounds_cached([0.2,0.8],across,1000,true,Some(&mut cache)).unwrap();
        assert_eq!(reused.0,direct.0);
        assert!(reused.1<direct.1,"Section data must survive an across-only change");
        assert_eq!(cache.bridge_data.len(),1);
    }
    let other=ProfileSectionBridge::new(profile.section(0,0.25).unwrap(),
        profile.section(0,0.75).unwrap(),[2.;2]).unwrap();
    assert_eq!(other.enclose_cached([0.2,0.8],[0.2,0.7],1000,Some(&cache)).unwrap(),
        other.enclose([0.2,0.8],[0.2,0.7],1000).unwrap(),
        "Position reuse must not cross section owners");
    let direct=other.derivative_bounds_impl([0.2,0.8],[0.2,0.7],1000,true).unwrap();
    let isolated=other.derivative_bounds_cached([0.2,0.8],[0.2,0.7],1000,true,Some(&mut cache)).unwrap();
    assert_eq!(isolated,direct,"Different section owners must not share cached data");
    assert_eq!(cache.bridge_data.len(),2);
    for across in [[0.5,0.9],[0.25,0.75],[0.4,0.4]] {
        let direct = constrained.derivatives_with_transverse_correction([0.3,0.7],across,1000).unwrap();
        let warm = constrained.derivatives_with_transverse_correction_cached(
            [0.3,0.7],across,1000,&mut cache).unwrap();
        assert_eq!(warm.0,direct.0); assert!(warm.0.is_some());
        assert!(warm.1 < direct.1,"warm={} direct={}",warm.1,direct.1);
    }
    assert!(constrained.derivatives_with_transverse_correction_cached(
        [0.2,0.8],[-0.1,0.4],1000,&mut cache).is_err());
    assert_eq!(constrained.derivatives_with_transverse_correction_cached(
        [0.2,0.8],[0.1,0.4],0,&mut cache).unwrap(),(None,0));
    // A bounded failed attempt may retain complete subqueries, but never
    // a partial fourth-derivative enclosure or a completed corrected jet.
    let mut limited = SideRemainderCache::default();
    assert!(constrained.side_remainders_cached(false,[0.1,0.4],1,&mut limited).unwrap().0.is_none());
    assert_eq!(limited.entries.len(),1);
    let retry = constrained.side_remainders_cached(false,[0.5,0.9],1000,&mut limited).unwrap();
    assert_eq!(retry.0,constrained.side_remainders(false,[0.5,0.9],1000).unwrap().0);
    assert_eq!(limited.entries.len(),2);
    let twin = crate::normal_alignment::qualify_weight_profile(&surface,[0,0],100).unwrap().0.unwrap();
    let other = ProfileSectionBridge::new(twin.section(0,0.).unwrap(),twin.section(0,1.).unwrap(),[1.;2]).unwrap();
    let other = other.constrain_original_rails(100).unwrap().0.unwrap();
    let foreign = other.side_remainders_cached(false,[0.5,0.9],1000,&mut cache).unwrap();
    assert!(foreign.1>0,"Distinct owners must compute their own source hulls");
}
#[test]
fn transverse_bezier_hull_covers_independent_curved_hermite_derivatives() {
    let controls=[[0.,0.,0.],[1.,2.,-1.],[2.,-2.,3.],[3.,1.,1.]];
    let scales=[0.75,1.25];
    for shift in [0.,1e6] {
        let surface=crate::surface::Surface {
            degree_u:3,degree_v:1,knots_u:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            knots_v:vec![0.,0.,1.,1.],
            control_points:controls.iter().map(|p|[0.,1.].into_iter().map(|v|
                vec![p[0]+shift,p[1]+v-shift,p[2]+2.*v+shift]).collect()).collect(),
            weights:vec![vec![1.;2];4],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],8).unwrap().0.unwrap();
        let bridge=ProfileSectionBridge::new(profile.section(0,0.).unwrap(),
            profile.section(0,1.).unwrap(),scales).unwrap();
        for range in [[0.,1.],[0.125,0.75],[0.5,0.5]] {
            for common in [false,true] {
                let (jet,spent)=bridge.derivative_bounds_impl([0.2,0.8],range,100,common).unwrap();
                let jet=jet.unwrap();assert!(spent<=100);
                for k in 0..3 {
                    let expected=[0.,1.,2.][k];
                    assert!(jet[0][k][0]<=expected && expected<=jet[0][k][1]);
                    assert!(jet[0][k][1]-jet[0][k][0]<1e-8,
                        "cubic free derivative must retain constant extrusion tangent");
                }
                for i in 0..=64 {
                    let t=range[0]+(range[1]-range[0])*i as f64/64.;let q=1.-t;
                    for k in 0..3 {
                        let d0=scales[0]*3.*(controls[1][k]-controls[0][k]);
                        let d1=scales[1]*3.*(controls[3][k]-controls[2][k]);
                        let expected=6.*t*q*(controls[3][k]-controls[0][k])
                            +q*(1.-3.*t)*d0+t*(3.*t-2.)*d1;
                        assert!(jet[1][k][0]<=expected && expected<=jet[1][k][1],
                            "shift={shift} common={common} range={range:?} k={k} t={t} misses {expected}");
                    }
                }
            }
        }
        assert!(bridge.derivative_bounds_impl([0.2,0.8],[0.,1.],1,true).unwrap().0.is_none());
    }
}
#[test]
fn independent_profile_chords_cover_plane_and_preserve_folded_candidate() {
    let a=crate::surface::Surface {
        degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:[0.,1.].into_iter().map(|u|vec![vec![u,0.,0.],vec![u,1.,0.]]).collect(),
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
    };
    let b=a.clone();
    let pa=crate::normal_alignment::qualify_weight_profile(&a,[0,0],100).unwrap().0.unwrap();
    let pb=crate::normal_alignment::qualify_weight_profile(&b,[0,0],100).unwrap().0.unwrap();
    let bridge=ProfileSectionBridge::new(pa.section(0,0.).unwrap(),pb.section(0,1.).unwrap(),[1.;2]).unwrap();
    assert!(bridge.constrain_original_rails(100).unwrap().0.is_none());
    assert_eq!(bridge.inspect_self_chords([[0.,1.];2],1e-4,0).unwrap(),(None,0));
    for i in 0..16 {for j in 0..16 {
        let domain=[[j as f64/16.,(j+1) as f64/16.],[i as f64/16.,(i+1) as f64/16.]];
        let (excluded,spent)=bridge.inspect_self_chords(domain,1e-4,100).unwrap();
        assert_eq!(excluded,Some(true),"domain={domain:?}");assert!(spent<=100);
    }}
    let zero=bridge.inspect_self_chord_coverage([[0.,1.];2],1e-4,0,1000).unwrap();
    assert_eq!(zero.unresolved_pairs,vec![[[[0.,1.];2];2]]);
    assert!(!zero.complete_exclusion);assert_eq!(zero.cells,0);
    let limited=bridge.inspect_self_chord_coverage([[0.,1.];2],1e-4,1,1000).unwrap();
    let mass:f64=limited.unresolved_pairs.iter().map(|p| {
        let volume:f64=p.iter().flatten().map(|r|r[1]-r[0]).product();
        volume*if p[0]==p[1]{1.}else{2.}
    }).sum();
    assert_eq!(mass,1.,"Unresolved symmetric products preserve complete coverage");
    let all=bridge.inspect_self_chord_coverage([[0.,1.];2],1e-4,10000,100000).unwrap();
    assert!(all.complete_exclusion,"plane unresolved={:?}",all.unresolved_pairs);
    assert!(all.cells<=10000 && all.spans<=100000);
    let mut shifted=a.clone();
    shifted.knots_u=vec![10.,10.,12.,12.];shifted.knots_v=vec![-2.,-2.,6.,6.];
    let shifted_twin=shifted.clone();
    let sp=crate::normal_alignment::qualify_weight_profile(&shifted,[0,0],100).unwrap().0.unwrap();
    let sq=crate::normal_alignment::qualify_weight_profile(&shifted_twin,[0,0],100).unwrap().0.unwrap();
    let reparameterized=ProfileSectionBridge::new(sp.section(0,10.).unwrap(),sq.section(0,12.).unwrap(),[2.;2]).unwrap();
    assert!(reparameterized.inspect_self_chord_coverage([[-2.,6.],[0.,1.]],1e-4,10000,100000).unwrap().complete_exclusion);
    for (domain,sine,cells,spans) in [([[0.,0.];2],1e-4,0,0),([[0.,1.];2],1.,0,0),
        ([[0.,1.];2],1e-4,100001,0),([[0.,1.];2],1e-4,0,100001)] {
        assert!(bridge.inspect_self_chord_coverage(domain,sine,cells,spans).is_err());
    }
    let arch=crate::surface::Surface {
        degree_u:1,degree_v:3,knots_u:vec![0.,0.,1.,1.],
        knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],
        control_points:[0.,1.].into_iter().map(|x|
            [[1.,0.],[1.,2.],[-1.,2.],[-1.,0.]].into_iter().map(|p|vec![x,p[0],p[1]]).collect()).collect(),
        weights:vec![vec![1.,1./3.,1./3.,1.];2],periodic_u:false,periodic_v:false,
    };
    let twin=arch.clone();
    let left=crate::normal_alignment::qualify_weight_profile(&arch,[0,0],100).unwrap().0.unwrap();
    let right=crate::normal_alignment::qualify_weight_profile(&twin,[0,0],100).unwrap().0.unwrap();
    let cylinder=ProfileSectionBridge::new(left.section(0,0.).unwrap(),right.section(0,1.).unwrap(),[1.;2]).unwrap();
    // The rational cross section has opposite endpoints and endpoint
    // tangents perpendicular to their nonzero normal diameter.
    for budget in [0,1,8,64,256] {
        let (excluded,spent)=cylinder.inspect_self_chords([[0.,1.];2],1e-4,budget).unwrap();
        assert_ne!(excluded,Some(true));assert!(spent<=budget);
    }
    let full=cylinder.inspect_self_chord_coverage([[0.,1.];2],1e-4,100,10000).unwrap();
    assert!(!full.complete_exclusion);
    let contains=|r:[[f64;2];2],p:[f64;2]|(0..2).all(|k|r[k][0]<=p[k] && p[k]<=r[k][1]);
    assert!(full.unresolved_pairs.iter().any(|p|
        (contains(p[0],[0.,0.4]) && contains(p[1],[1.,0.4])) ||
        (contains(p[1],[0.,0.4]) && contains(p[0],[1.,0.4]))),"Actual normal diameter remains covered");
    let encloses=|a:[[[f64;2];2];2],b:[[[f64;2];2];2]|(0..2).all(|e|
        (0..2).all(|k|a[e][k][0]<=b[e][k][0] && b[e][k][1]<=a[e][k][1]));
    let mut previous=vec![[[[0.,1.];2];2]];
    for budget in [0,1,7,31,100] {
        let refined=cylinder.inspect_self_chord_coverage([[0.,1.];2],1e-4,budget,10000).unwrap();
        assert!(refined.cells<=budget && refined.spans<=10000);
        for pair in &refined.unresolved_pairs {
            assert!(previous.iter().any(|old|encloses(*old,*pair)||encloses(*old,[pair[1],pair[0]])),
                "Increased work must preserve endpoint coverage inside prior pending obligations");
        }
        previous=refined.unresolved_pairs;
    }
    let separated=[[[0.1,0.11],[0.1,0.11]],[[0.2,0.21],[0.2,0.21]]];
    for pair in [separated,[separated[1],separated[0]]] {
        assert_eq!(bridge.inspect_self_chord_pair(pair,1e-4,1000).unwrap().0,Some(true));
        assert_eq!(bridge.inspect_self_chord_pair(pair,1e-4,0).unwrap(),(None,0));
    }
    let diameter=[[[0.,0.001],[0.4,0.41]],[[0.999,1.],[0.4,0.41]]];
    for pair in [diameter,[diameter[1],diameter[0]]] {for budget in [0,1,8,64,1000] {
        let (excluded,spent)=cylinder.inspect_self_chord_pair(pair,1e-4,budget).unwrap();
        assert_ne!(excluded,Some(true));assert!(spent<=budget);
    }}
    let kink=crate::surface::Surface {
        degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,0.5,1.,1.],
        control_points:[0.,1.].into_iter().map(|u|vec![vec![u,0.,0.],vec![u,0.5,0.],vec![u,1.,0.3]]).collect(),
        weights:vec![vec![1.;3];2],periodic_u:false,periodic_v:false,
    };
    let kp=crate::normal_alignment::qualify_weight_profile(&kink,[0,0],100).unwrap().0.unwrap();
    let discontinuous=ProfileSectionBridge::new(kp.section(0,0.).unwrap(),kp.section(0,1.).unwrap(),[1.;2]).unwrap();
    assert_eq!(discontinuous.inspect_self_chords([[0.,1.];2],1e-4,1000).unwrap(),(None,0));
    assert_eq!(discontinuous.inspect_self_chord_pair(separated,1e-4,1000).unwrap(),(None,0));
    for bad in [[[[0.,0.],[0.,1.]],separated[1]],[[[f64::NAN,0.1],[0.,1.]],separated[1]],
        [[[0.,1.1],[0.,1.]],separated[1]]] {
        assert!(bridge.inspect_self_chord_pair(bad,1e-4,0).is_err());
    }
    assert!(bridge.inspect_self_chord_pair(separated,1.,100).is_err());
    assert!(bridge.inspect_self_chord_pair(separated,1e-4,100001).is_err());
    let folded=ProfileSectionBridge::new(pa.section(0,0.).unwrap(),pb.section(0,0.).unwrap(),[1.;2]).unwrap();
    assert_ne!(folded.inspect_self_chords([[0.,1.];2],1e-4,100).unwrap().0,Some(true));
    for (domain,angle,budget) in [([[-0.1,1.],[0.,1.]],1e-4,100),
        ([[0.,1.],[0.,0.]],1e-4,100),([[0.,1.];2],1.,100),([[0.,1.];2],1e-4,100001)] {
        assert!(bridge.inspect_self_chords(domain,angle,budget).is_err());
    }
}
#[test]
fn mixed_bezier_hull_covers_varying_free_tangent_field() {
    let c=[[0.,0.,0.],[1.,2.,-1.],[2.,-2.,3.],[3.,1.,1.]];
    let d=[[0.,1.,2.],[1.,-1.,3.],[2.,2.,0.],[-1.,1.,-2.]];
    let scales=[0.75,1.25];
    let surface=crate::surface::Surface {
        degree_u:3,degree_v:1,knots_u:vec![0.,0.,0.,0.,1.,1.,1.,1.],
        knots_v:vec![0.,0.,1.,1.],
        control_points:(0..4).map(|i|[0.,1.].into_iter().map(|v|
            (0..3).map(|k|c[i][k]+v*d[i][k]).collect()).collect()).collect(),
        weights:vec![vec![1.;2];4],periodic_u:false,periodic_v:false,
    };
    let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],8).unwrap().0.unwrap();
    let bridge=ProfileSectionBridge::new(profile.section(0,0.).unwrap(),
        profile.section(0,1.).unwrap(),scales).unwrap();
    for range in [[0.,1.],[0.125,0.75],[0.5,0.5]] {
        let (mixed,spent)=bridge.mixed_derivative_bounds([0.2,0.8],range,100).unwrap();
        let mixed=mixed.unwrap();assert!(spent<=100);
        for i in 0..=64 {
            let t=range[0]+(range[1]-range[0])*i as f64/64.;let q=1.-t;
            for k in 0..3 {
                let m0=scales[0]*3.*(d[1][k]-d[0][k]);
                let m1=scales[1]*3.*(d[3][k]-d[2][k]);
                let expected=6.*t*q*(d[3][k]-d[0][k])+q*(1.-3.*t)*m0+t*(3.*t-2.)*m1;
                assert!(mixed[k][0]<=expected && expected<=mixed[k][1]);
            }
        }
    }
    assert!(bridge.mixed_derivative_bounds([0.2,0.8],[0.,1.],1).unwrap().0.is_none());
}
#[test]
fn anchored_hermite_enclosure_preserves_constant_coordinate_and_real_cubic() {
    let r=anchored_position_range([0.,1.],I::point(6.),I::point(6.),I::point(0.),I::new(0.,12.).unwrap()).unwrap();
    assert!(r.lo<=6. && r.hi>=6. && r.hi-r.lo<1e-10);
    assert!(anchored_position_range([-0.1,1.],I::point(1.),I::point(2.),I::point(0.),I::new(0.,3.).unwrap()).is_err());
    for range in [[0.,1.],[0.125,0.75],[0.5,0.5]] {
        let t=I::new(range[0],range[1]).unwrap();let q=I::point(1.).sub(t).unwrap();
        let tangent=t.mul(q).unwrap().mul(q).unwrap().mul(I::point(2.)).unwrap()
            .sub(t.mul(t).unwrap().mul(q).unwrap().mul(I::point(-3.)).unwrap()).unwrap();
        let r=anchored_position_range(range,I::point(3.),I::point(7.),tangent,I::new(-100.,100.).unwrap()).unwrap();
        for i in 0..=32 {
            let t=range[0]+(range[1]-range[0])*i as f64/32.;let q=1.-t;
            let expected=3.*q*q*(1.+2.*t)+7.*t*t*(3.-2.*t)+2.*t*q*q+3.*t*t*q;
            assert!(r.lo<=expected && expected<=r.hi,"{range:?}: {r:?} misses {expected}");
        }
    }
}
#[test]
fn independent_profile_injectivity_distinguishes_plane_from_fold() {
    let plane=crate::surface::Surface {degree_u:1,degree_v:1,
        knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:[0.,1.].into_iter().map(|u|vec![vec![u,0.,0.],vec![u,1.,0.]]).collect(),
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
    let other=plane.clone();
    let a=crate::normal_alignment::qualify_weight_profile(&plane,[0,0],100).unwrap().0.unwrap();
    let b=crate::normal_alignment::qualify_weight_profile(&other,[0,0],100).unwrap().0.unwrap();
    let projection=[[0.,1.,0.],[1.,0.,0.]];
    let bridge=ProfileSectionBridge::new(a.section(0,0.).unwrap(),b.section(0,1.).unwrap(),[1.;2]).unwrap();
    let admitted=bridge.inspect_injectivity([0.,1.],[0.,1.],projection,1000,10000).unwrap();
    assert!(admitted.complete_regular && admitted.pending.is_empty());
    // x(t)=4t-9t²+6t³. Its derivative at t=1/2 is -1/2;
    // t=1/2 and t=1/2 +/- sqrt(3)/6 all map to x=1/2.
    // These are distinct parameters of a genuine overlapping patch.
    let folded=ProfileSectionBridge::new(a.section(0,0.).unwrap(),b.section(0,1.).unwrap(),[4.;2]).unwrap();
    for cells in [0,1,7,100] {for spans in [0,1,100,10000] {
        let report=folded.inspect_injectivity([0.,1.],[0.,1.],projection,cells,spans).unwrap();
        assert!(!report.complete_regular);assert!(report.cells<=cells && report.spans<=spans);
        assert!(report.pending.iter().any(|r|r[0][0]<=0.5 && 0.5<=r[0][1] && r[1][0]<=0.5 && 0.5<=r[1][1]));
        let area:f64=report.regular.iter().chain(&report.pending)
            .map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
        assert_eq!(area,1.,"No unsupported product may disappear on exhaustion");
    }}
}
#[test]
fn full_corrected_injectivity_refuses_a_genuinely_folded_patch() {
    // x(u)=(u-0.625)^2: distinct u values in [0.5,0.75]
    // coincide. The cubic bridge reproduces this quadratic exactly.
    let surface = crate::surface::Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0.390625, -0.234375, 0.140625]
            .into_iter()
            .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let profile = crate::normal_alignment::qualify_weight_profile(&surface, [0, 0], 6)
        .unwrap()
        .0
        .unwrap();
    let bridge = ProfileSectionBridge::new(
        profile.section(0, 0.5).unwrap(),
        profile.section(0, 0.75).unwrap(),
        [0.25; 2],
    )
    .unwrap();
    let corrected = bridge.constrain_original_rails(100).unwrap().0.unwrap();
    let report = corrected
        .inspect_transverse_corrected_injectivity_from_rails(
            [0., 1.],
            [0., 1.],
            [[0., 1., 0.], [1., 0., 0.]],
            20,
            1220,
        )
        .unwrap();
    assert!(!report.complete_regular);
    assert!(!report.pending.is_empty());
    let area: f64 = report
        .regular
        .iter()
        .chain(&report.pending)
        .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
        .sum();
    assert_eq!(area, 1.);
    // Cell and derivative exhaustion are independent stop conditions.
    // Every stop must retain all uncovered regions of this real fold.
    for cells in [0,1,2,7] {
        for spans in [0,1,61,1220] {
            let stopped=corrected.inspect_transverse_corrected_injectivity_from_rails(
                [0.,1.],[0.,1.],[[0.,1.,0.],[1.,0.,0.]],cells,spans).unwrap();
            assert!(!stopped.complete_regular);
            assert!(stopped.cells<=cells && stopped.spans<=spans);
            let area:f64=stopped.regular.iter().chain(&stopped.pending)
                .map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
            assert_eq!(area,1.);
        }
    }
}
#[test]
fn common_source_difference_tightens_translated_bridge_jet() {
    let surface = crate::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1e8, 0., 0.], vec![1e8, 1., 0.]],
            vec![vec![1e8 + 1., 0., 0.], vec![1e8 + 1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let profile = crate::normal_alignment::qualify_weight_profile(&surface, [0, 0], 4)
        .unwrap()
        .0
        .unwrap();
    let bridge = ProfileSectionBridge::new(
        profile.section(0, 0.5).unwrap(),
        profile.section(0, 0.75).unwrap(),
        [0.25; 2],
    )
    .unwrap();
    let old = bridge
        .derivative_bounds([0.25, 0.75], [0.25, 0.75], 8)
        .unwrap()
        .0
        .unwrap();
    let (new, spent) = bridge
        .derivative_bounds_impl([0.25, 0.75], [0.25, 0.75], 9, true)
        .unwrap();
    let new = new.unwrap();
    assert_eq!(spent, 9);
    assert!(new[1][0][0] <= 0.25 && new[1][0][1] >= 0.25);
    assert!(new[1][0][1] - new[1][0][0] < old[1][0][1] - old[1][0][0]);
    assert!(
        bridge
            .derivative_bounds_impl([0.25, 0.75], [0.25, 0.75], 8, true)
            .unwrap()
            .0
            .is_none()
    );
}
#[test]
fn principal_minors_admit_nonsymmetric_maps_and_refuse_orientation_failures() {
    let matrix = |m: [[f64; 2]; 2]| m.map(|row| row.map(I::point));
    // Positive principal minors, but indefinite symmetric part.
    assert!(projected_principal_minors_positive(matrix([[1., 8.], [-1., 1.]])).unwrap());
    assert!(!projected_principal_minors_positive(matrix([[-1., 0.], [0., -1.]])).unwrap());
    assert!(!projected_principal_minors_positive(matrix([[1., 2.], [2., 1.]])).unwrap());
    let uncertain = [
        [I::new(-0.1, 1.).unwrap(), I::point(0.)],
        [I::point(0.), I::point(1.)],
    ];
    assert!(!projected_principal_minors_positive(uncertain).unwrap());
}
