use super::*;
#[test]
fn coordinate_strips_exclude_planar_chart_with_collapsed_boundary() {
    let s=crate::surface::Surface {
        degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,0.,0.]],
            vec![vec![0.,1.,0.],vec![1.,0.,0.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
    };
    let p=crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
    let full=p.derivative_bounds([[0.,1.];2],100).unwrap().0.unwrap();
    assert!(!crate::surface_self_chord::inspect_jacobian_rank_bound(full).unwrap());
    let pair=[[[0.,0.1],[0.,0.1]],[[0.9,1.],[0.9,1.]]];
    assert_eq!(p.inspect_self_chord_pair(pair,1e-4,100).unwrap().0,Some(true));
    assert_eq!(p.inspect_self_chord_pair(pair,1e-4,1).unwrap(),(None,1));
    // S(u,v)=(uv,u(1-v),0): every nonzero chord lies in XY;
    // at B, u>=0.9, the analytic cross product is (0,0,-u).
    // The collapsed A boundary is not assigned a regular normal.
    let mut spatial=s.clone();
    for row in &mut spatial.control_points {
        for (v,point) in row.iter_mut().enumerate() {point[2]=v as f64;}
    }
    let spatial=crate::surface_weight_profile::qualify_weight_profile(&spatial,[0,0],100).unwrap().0.unwrap();
    // The endpoint-domain enclosure resolves the previously insufficient
    // S=(uv,u(1-v),v) control. Analytic corner chords check the direction.
    assert_eq!(spatial.inspect_self_chord_pair(pair,1e-4,100).unwrap().0,Some(true));
    for u in [0.,0.1] {for v in [0.,0.1] {for x in [0.9,1.] {for y in [0.9,1.] {
        let chord=[x*y-u*v,x*(1.-y)-u*(1.-v),y-v];
        for (a,b) in [(u,v),(x,y)] {
            let normal=[1.-b,-b,-a];
            let dot:f64=(0..3).map(|k|normal[k]*chord[k]).sum();
            let nn:f64=normal.iter().map(|n|n*n).sum();
            let cc:f64=chord.iter().map(|c|c*c).sum();
            assert!(dot*dot/(nn*cc)<1.-1e-4);
        }
    }}}}
}
#[test]
fn rational_interior_endpoint_directions_keep_planar_chords_excluded() {
    let mut s=crate::surface::Surface {
        degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.];2],vec![vec![0.5,0.,0.];2],
            vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.,4.];3],periodic_u:false,periodic_v:false,
    };
    let pair=[[[0.3,0.31],[0.49,0.51]],[[0.5,0.51],[0.49,0.51]]];
    for reversed in [false,true] {
        if reversed {s.control_points.reverse();}
        let p=crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
        let (excluded,spent)=p.inspect_self_chord_pair(pair,1e-4,100).unwrap();
        assert_eq!(excluded,Some(true));assert!(spent<=100);
        assert_eq!(p.inspect_self_chord_pair(pair,1e-4,0).unwrap(),(None,0));
        let wide=[[[0.01,0.05],[0.49,0.51]],[[0.8,0.9],[0.49,0.51]]];
        assert_eq!(p.inspect_self_chord_pair(wide,1e-4,100).unwrap().0,Some(true),
            "direct chord exclusion does not require uniform matrix rank");
        // Both endpoint charts have nonzero normal +/-Z and every chord
        // lies in XY. Positive interior factor removal preserves this angle.
    }
}
#[test]
fn retained_workspace_reuses_complete_hulls_and_preserves_refusals() {
    let s=crate::surface::Surface {
        degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],
            vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
    };
    let profile=crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
    let pair=[[[0.,0.2],[0.,0.2]],[[0.8,1.],[0.8,1.]]];
    let direct=profile.inspect_self_chord_pair(pair,1e-4,100).unwrap();
    let mut workspace=profile.chord_workspace();
    assert_eq!(workspace.inspect_pair(pair,1e-4,100).unwrap(),direct);
    let warm=workspace.inspect_pair(pair,1e-4,100).unwrap();
    assert_eq!(warm.0,direct.0);assert!(warm.1<direct.1);
    assert!(workspace.cache_hits()>0);
    let union=std::array::from_fn(|axis|[pair[0][axis][0].min(pair[1][axis][0]),pair[0][axis][1].max(pair[1][axis][1])]);
    let intrinsic=profile.inspect_self_chords(union,1e-4,100).unwrap();
    let shared=workspace.inspect_self_chords(union,1e-4,100).unwrap();
    assert_eq!(shared.0,intrinsic.0);
    assert!(shared.1<intrinsic.1);
    assert_eq!(workspace.inspect_self_chords(union,1e-4,0).unwrap(),(None,0));
    assert!(workspace.inspect_self_chords([[-1.,1.],[0.,1.]],1e-4,0).is_err());
    assert_eq!(workspace.inspect_pair(pair,1e-4,0).unwrap(),(None,0));
    let mut invalid=pair;invalid[0][0][1]=2.;
    assert!(workspace.inspect_pair(invalid,1e-4,0).is_err());
    assert!(workspace.inspect_pair(pair,1.,100).is_err());
    let mut retry=profile.chord_workspace();
    assert_eq!(retry.inspect_pair(pair,1e-4,1).unwrap(),(None,1));
    let completed=retry.inspect_pair(pair,1e-4,100).unwrap();
    assert_eq!(completed.0,direct.0);assert!(completed.1<=direct.1);
    // Same UV coordinates on another owner start cold.
    let other=crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
    assert_eq!(other.chord_workspace().inspect_pair(pair,1e-4,100).unwrap(),direct);
}
#[test]
fn retained_half_cylinder_preserves_exact_diameter_candidate() {
    let points = [[1.,0.],[1.,1.],[0.,1.],[-1.,1.],[-1.,0.]];
    let w = 0.5_f64.sqrt();
    let s = crate::surface::Surface {
        degree_u:2, degree_v:1,
        knots_u:vec![0.,0.,0.,0.5,0.5,1.,1.,1.], knots_v:vec![0.,0.,1.,1.],
        control_points:points.iter().map(|p|vec![vec![p[0],p[1],0.],vec![p[0],p[1],1.]]).collect(),
        weights:[1.,w,1.,w,1.].iter().map(|w|vec![*w;2]).collect(),
        periodic_u:false,periodic_v:false,
    };
    let p = crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
    // The exact U=0/1 endpoints have an aligned diameter chord; an
    // intrinsic exclusion of the complete chart would hide a thin wall.
    assert_eq!(p.inspect_self_chords([[0.,1.];2],1e-6,100).unwrap().0,Some(false));
    assert_eq!(p.inspect_self_chords([[0.,1.];2],1e-6,1).unwrap(),(None,1));
    let pair=[[[0.,0.01],[0.4,0.5]],[[0.99,1.],[0.4,0.5]]];
    assert_eq!(p.inspect_self_chord_pair(pair,1e-6,100).unwrap().0,Some(false));
    let mut workspace=p.chord_workspace();
    assert_eq!(workspace.inspect_self_chords([[0.,1.];2],1e-6,100).unwrap().0,Some(false));
    for budget in [0,1,100] {
        let result=workspace.inspect_self_chords([[0.,1.];2],1e-6,budget).unwrap();
        assert_ne!(result.0,Some(true));
        assert!(result.1<=budget);
        if budget==0 {assert_eq!(result,(None,0));}
    }
    assert_eq!(workspace.inspect_pair(pair,1e-6,100).unwrap().0,Some(false));
    for endpoints in [pair,[pair[1],pair[0]]] {for budget in [0,1,4,8,100] {
        let (decision,spent)=workspace.inspect_pair(endpoints,1e-6,budget).unwrap();
        assert_ne!(decision,Some(true),"Warm retained cache must preserve a real normal diameter");
        assert!(spent<=budget);
        if budget==0 {assert_eq!((decision,spent),(None,0));}
    }}
    assert_eq!(p.inspect_self_chord_pair(pair,1e-6,1).unwrap(),(None,1));
    for endpoints in [pair,[pair[1],pair[0]]] {
        for budget in [0,1,4,8,100] {
            let (result,spent)=p.inspect_self_chord_pair(endpoints,1e-6,budget).unwrap();
            assert!(result!=Some(true) && spent<=budget,
                "an actual normal diameter survives both paths and incomplete work");
        }
    }
}
#[test]
fn retained_diameter_candidate_survives_scale_translation_and_endpoint_exchange() {
    let w=0.5_f64.sqrt();
    for scale in [1e-8,1.,1e6] {for shift in [0.,100.] {
        let points=[[1.,0.],[1.,1.],[0.,1.],[-1.,1.],[-1.,0.]];
        let surface=crate::surface::Surface {
            degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,0.5,0.5,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:points.iter().map(|p|[0.,1.].iter().map(|z|
                vec![scale*(p[0]+shift),scale*(p[1]-shift),scale*(z+shift)]).collect()).collect(),
            weights:[1.,w,1.,w,1.].iter().map(|w|vec![*w;2]).collect(),periodic_u:false,periodic_v:false,
        };
        let profile=crate::surface_weight_profile::qualify_weight_profile(&surface,[0,0],100).unwrap().0.unwrap();
        let pair=[[[0.,0.01],[0.4,0.5]],[[0.99,1.],[0.4,0.5]]];
        for endpoints in [pair,[pair[1],pair[0]]] {for budget in [0,1,4,8,100] {
            let (decision,spent)=profile.inspect_self_chord_pair(endpoints,1e-6,budget).unwrap();
            assert_ne!(decision,Some(true),"Normal diameter excluded at scale={scale} shift={shift} budget={budget}");
            assert!(spent<=budget);
            if budget==100 {assert_eq!(decision,Some(false));}
        }}
    }}
}
#[test]
fn exact_profile_excludes_plane_but_preserves_discontinuity_and_budget_refusal() {
    let mut s = crate::surface::Surface {
        degree_u: 1, degree_v: 1,
        knots_u: vec![0.,0.,1.,1.], knots_v: vec![0.,0.,1.,1.],
        control_points: vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],
            vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights: vec![vec![1.;2];2], periodic_u:false, periodic_v:false,
    };
    let p = crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
    assert_eq!(p.inspect_self_chords([[0.,1.];2],1e-4,0).unwrap(),(None,0));
    assert_eq!(p.inspect_self_chords([[0.,1.];2],1e-4,100).unwrap().0,Some(true));
    assert_eq!(p.inspect_self_chord_pair([[[0.,0.4],[0.,1.]],[[0.6,1.],[0.,1.]]],1e-4,100).unwrap().0,Some(true));
    assert!(p.inspect_self_chords([[-1.,1.],[0.,1.]],1e-4,0).is_err());
    s.knots_u = vec![0.,0.,0.5,0.5,1.,1.];
    s.control_points.extend(s.control_points.clone());
    s.weights = vec![vec![1.;2];4];
    assert!(!continuous_knots(&s.knots_u,1,4,[0.,1.]));
    assert!(crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).is_err());
}
