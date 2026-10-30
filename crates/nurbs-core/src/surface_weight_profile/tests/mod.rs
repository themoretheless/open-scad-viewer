use super::*;
fn surface() -> Surface {
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
fn interior_direction_factors_uncertain_rational_normals() {
    let mut s=surface();s.degree_u=2;s.knots_u=vec![0.,0.,0.,1.,1.,1.];
    s.control_points=vec![vec![vec![0.,0.,0.];2],vec![vec![0.5,0.,0.];2],vec![vec![1.,0.,0.],vec![1.,1.,0.]]];
    s.weights=vec![vec![1.,4.];3];
    let p=qualify_weight_profile(&s,[0,0],6).unwrap().0.unwrap();
    let domain=[[0.01,0.99],[0.5,0.5]];
    let ordinary=p.normal_bounds(domain,100).unwrap();
    assert!(ordinary.0.unwrap().iter().all(|d|d[0]<=0. && d[1]>=0.));
    let (direction,spent)=p.interior_normal_direction_bounds(domain,100).unwrap();
    assert!(spent>ordinary.1 && spent<=100);
    let z=direction.unwrap()[2];
    assert!(z[0]>0. && z[0]<=0.64 && z[1]>=0.64);
    assert_eq!(p.interior_normal_direction_bounds(domain,ordinary.1).unwrap(),ordinary);
    let wide=[[0.01,0.99],[0.,1.]];
    let ordinary_wide=p.normal_bounds(wide,100).unwrap();
    assert_eq!(p.interior_normal_direction_bounds(wide,100).unwrap().0,ordinary_wide.0);
    for budget in 0..=20 {
        let (_,spent)=p.interior_normal_direction_bounds(domain,budget).unwrap();
        assert!(spent<=budget);
    }
    assert!(p.interior_normal_direction_bounds(domain,100001).is_err());
    s.control_points.reverse();
    let reversed=qualify_weight_profile(&s,[0,0],6).unwrap().0.unwrap();
    let (direction,spent)=reversed.interior_normal_direction_bounds(domain,100).unwrap();
    let z=direction.unwrap()[2];
    assert!(spent<=100 && z[1]<0. && z[0]<=-0.64 && z[1]>=-0.64);
}
#[test]
fn interior_direction_without_collapsed_end_keeps_original_bounds() {
    let mut s=surface();s.degree_u=2;s.knots_u=vec![0.,0.,0.,1.,1.,1.];
    s.control_points=vec![vec![vec![0.,0.,0.],vec![0.,0.01,0.]],vec![vec![0.5,0.,0.];2],vec![vec![1.,0.,0.],vec![1.,1.,0.]]];
    s.weights=vec![vec![1.,4.];3];
    let p=qualify_weight_profile(&s,[0,0],6).unwrap().0.unwrap();
    let domain=[[0.01,0.99],[0.,1.]];
    let ordinary=p.normal_bounds(domain,100).unwrap();
    assert!(ordinary.0.unwrap().iter().all(|d|d[0]<=0. && d[1]>=0.));
    assert_eq!(p.interior_normal_direction_bounds(domain,100).unwrap(),ordinary);
}
#[test]
fn interior_direction_preserves_pole_uncertainty_and_budget() {
    for high in [false,true] {
        let mut s=surface();
        if high {s.control_points.reverse();s.control_points[1]=vec![vec![0.,0.,0.];2];}
        else {s.control_points[0]=vec![vec![0.,0.,0.];2];}
        let p=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
        for domain in [[[0.,1.],[0.,1.]],[[0.1,0.9],[0.,1.]]] {
            assert_eq!(p.interior_normal_direction_bounds(domain,0).unwrap(),(None,0));
            let ordinary=p.normal_bounds(domain,20).unwrap();
            let direction=p.interior_normal_direction_bounds(domain,20).unwrap();
            assert!(direction.1<=20);
            if domain[0]==[0.,1.] {assert_eq!(direction,ordinary);}
            let bounds=direction.0.unwrap();
            // S=(u,u*v,0), or reversed: every interior normal is +/-Z.
            assert!(bounds[0][0]<=0. && bounds[0][1]>=0.);
            assert!(bounds[1][0]<=0. && bounds[1][1]>=0.);
            if !high {assert!(bounds[2][1]>0.);} else {assert!(bounds[2][0]<0.);}
        }
        assert!(p.interior_normal_direction_bounds([[-0.1,0.9],[0.,1.]],0).is_err());
    }
}
#[test]
fn collapsed_profile_normal_factor_matches_bilinear_reference() {
    for high in [false,true] {
        let mut s=surface();
        if high {s.control_points[0]=vec![vec![1.,0.,0.],vec![1.,1.,0.]];s.control_points[1]=vec![vec![0.,0.,0.];2];}
        else {s.control_points[0][1]=vec![0.,0.,0.];}
        let profile=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
        let original=surface();
        let paths=[[[0.,0.],[1.,0.]],[[0.,0.],[1.,0.]]];
        let path=profile.inspect_original_factored_pole_path(&original,paths,high,1e-6,100,1000).unwrap();
        assert_eq!(path.aligned,Some(true));assert!(path.pending.is_empty());
        assert!(path.upper_sine_squared.unwrap()<=1e-6);
        assert!(profile.inspect_original_factored_pole_path(&original,paths,high,1e-6,1,1).unwrap().aligned.is_none());
        let (limits,work)=profile.factored_pole_limits(high,1e-6,12).unwrap();
        let limits=limits.unwrap();assert_eq!(work,12);
        assert!(!limits.distinct_tangent_limits);
        assert!(limits.sine_squared_interval.unwrap()[1]<=1e-6);
        assert!(profile.factored_pole_limits(high,1e-6,11).unwrap().0.is_none());
        assert!(profile.factored_pole_normal_bounds([[0.,1.],[0.,1.]],high,5).unwrap().0.is_none());
        let (normal,spent)=profile.factored_pole_normal_bounds([[0.,1.],[0.,1.]],high,6).unwrap();
        let normal=normal.unwrap();assert_eq!(spent,6);assert_eq!(normal.power,1);assert_eq!(normal.high,high);
        let expected=if high {-1.}else{1.};
        assert!(normal.components[2][0]<=expected && normal.components[2][1]>=expected);
        assert!(normal.components[2][0]>0. || normal.components[2][1]<0.);
        for axis in 0..2 {assert!(normal.components[axis][0]<=0. && normal.components[axis][1]>=0.);}
    }
    for high in [false,true] {
        let mut s=surface();s.degree_u=2;s.knots_u=vec![2.,2.,2.,4.,4.,4.];
        let pole=vec![1e8,2e8,3e8];let outer=vec![vec![1e8+1.,2e8,3e8],vec![1e8+1.,2e8+1.,3e8]];
        s.control_points=if high {vec![outer,vec![pole.clone();2],vec![pole;2]]}else{vec![vec![pole.clone();2],vec![pole;2],outer]};
        s.weights=vec![vec![1.;2];3];
        let p=qualify_weight_profile(&s,[0,0],6).unwrap().0.unwrap();
        let (limits,work)=p.factored_pole_limits(high,1e-6,12).unwrap();
        let limits=limits.unwrap();assert_eq!(work,12);
        assert!(!limits.distinct_tangent_limits);
        assert!(limits.sine_squared_interval.unwrap()[1]<=1e-6);
        let normal=p.factored_pole_normal_bounds([[2.,4.],[0.,1.]],high,6).unwrap().0.unwrap();
        assert_eq!(normal.power,3);let expected=if high {-1.}else{1.};
        assert!(normal.components[2][0]<=expected && normal.components[2][1]>=expected);
        assert!(normal.components[2][0]>0. || normal.components[2][1]<0.);
    }
    let s=surface();let p=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    assert!(p.factored_pole_normal_bounds([[0.,1.],[0.,1.]],true,6).unwrap().0.is_none());
}
#[test]
fn independent_transverse_collapse_factor_matches_quadratic_reference() {
    for high in [false,true] {
        let mut s=surface();s.degree_u=2;s.knots_u=vec![0.,0.,0.,1.,1.,1.];
        let rows=vec![vec![vec![0.,0.,0.];2],vec![vec![0.5,0.,0.];2],vec![vec![1.,0.,0.],vec![1.,1.,0.]]];
        s.control_points=if high {rows.into_iter().rev().collect()}else{rows};s.weights=vec![vec![1.;2];3];
        let p=qualify_weight_profile(&s,[0,0],6).unwrap().0.unwrap();
        let domain=[[0.,1.],[0.,1.]];
        assert!(p.factored_pole_normal_bounds(domain,high,6).unwrap().0.is_none());
        let (normal,spent)=p.factored_pole_normal_bounds(domain,high,7).unwrap();let normal=normal.unwrap();
        assert_eq!(spent,7);assert_eq!(normal.power,2);
        let expected=if high {-1.}else{1.};
        assert!(normal.components[2][0]<=expected && expected<=normal.components[2][1]);
        assert!(normal.components[2][0]>0. || normal.components[2][1]<0.);
        assert!(p.inspect_factored_pole_regularity(domain,high,1,7).unwrap().complete_factored);
        s.weights=vec![vec![1.,4.];3];
        let p=qualify_weight_profile(&s,[0,0],6).unwrap().0.unwrap();
        // S=(f,f^2*4v/(1+3v),0), factored normal z=+/-4/(1+3v)^2.
        for v in [0.,0.5,1.] {
            let normal=p.factored_pole_normal_bounds([[0.,1.],[v;2]],high,7).unwrap().0.unwrap();
            let expected=(if high {-4.}else{4.})/(1.+3.*v).powi(2);
            assert!(normal.components[2][0]<=expected && expected<=normal.components[2][1]);
            assert!(normal.components[2][0]>0. || normal.components[2][1]<0.);
        }
    }
}
#[test]
fn adaptive_factored_pole_cover_keeps_refusal_and_full_area() {
    let mut s=surface();s.control_points[0][1]=vec![0.,0.,0.];
    let p=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();let domain=[[0.,1.],[0.,1.]];
    let report=p.inspect_factored_pole_regularity(domain,false,1,6).unwrap();
    assert!(report.complete_factored);assert_eq!(report.power,Some(1));assert_eq!(report.regular,vec![domain]);
    for (cells,spans) in [(0,6),(1,5)] {
        let r=p.inspect_factored_pole_regularity(domain,false,cells,spans).unwrap();
        assert!(!r.complete_factored);assert_eq!(r.pending,vec![domain]);
    }
    // A second collapse in V produces a genuinely zero factored normal.
    s.control_points[1][1]=s.control_points[1][0].clone();
    let p=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    let r=p.inspect_factored_pole_regularity(domain,false,15,90).unwrap();
    assert!(!r.complete_factored);assert!(r.regular.is_empty());
    let area:f64=r.regular.iter().chain(&r.pending).map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();assert_eq!(area,1.);
}
#[test]
fn collapsed_profile_normal_factor_matches_rational_reference() {
    let mut s=surface();s.control_points[0][1]=vec![0.,0.,0.];s.weights[1]=vec![4.;2];
    let p=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    // S=(4u/(1+3u),4uv/(1+3u),0), (Su x Sv)/u=(0,0,16/(1+3u)^3).
    for u in [0.,0.25,0.5,0.75,1.] {
        let normal=p.factored_pole_normal_bounds([[u;2],[0.,1.]],false,6).unwrap().0.unwrap();
        let expected=16./(1.+3.*u).powi(3);
        assert!(normal.components[2][0]<=expected && normal.components[2][1]>=expected);
        assert!(normal.components[2][0]>0.);
    }
}
#[test]
fn profile_rectangle_regularity_preserves_singular_coverage() {
    let mut s=surface();
    let profile=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    let domain=[[0.,0.75],[0.,1.]];
    assert!(profile.inspect_regularity(domain,1,1).unwrap().complete_regular);
    let refused=profile.inspect_regularity(domain,0,1).unwrap();
    assert_eq!(refused.pending,vec![domain]);assert!(!refused.complete_regular);
    assert!(profile.inspect_regularity([[0.8,0.2],[0.,1.]],10,10).is_err());
    // Bilinear collapsed edge: S(u,v)=(u,u*v,0), singular at u=0.
    s.control_points[0][1]=vec![0.,0.,0.];
    let profile=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    assert!(profile.inspect_regularity([[0.25,1.],[0.,1.]],100,100).unwrap().complete_regular);
    let report=profile.inspect_regularity([[0.,1.],[0.,1.]],31,31).unwrap();
    assert!(!report.complete_regular);assert!(!report.pending.is_empty());
    let area:f64=report.regular.iter().chain(&report.pending).map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();assert_eq!(area,1.);
    assert!(report.pending.iter().any(|r|r[0][0]==0.));
}
#[test]
fn adaptive_profile_path_retains_pending_coverage() {
    let s = surface();
    let profile = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
    let line = [[[0., 0.], [1., 0.]], [[0., 0.], [1., 0.]]];
    let r = profile
        .inspect_original_path(&s, line, 1e-12, 1, 2)
        .unwrap();
    assert_eq!(r.aligned, Some(true));
    assert!(r.pending.is_empty());
    let r = profile
        .inspect_original_path(&s, line, 1e-12, 1, 1)
        .unwrap();
    assert_eq!(r.aligned, None);
    assert_eq!(r.pending, vec![[0., 1.]]);
    let mut tilted = surface();
    for row in &mut tilted.control_points {
        for p in row {
            p[2] = p[1];
        }
    }
    let r = profile
        .inspect_original_path(&tilted, line, 1e-12, 100, 200)
        .unwrap();
    assert_eq!(r.aligned, Some(false));
    assert_eq!(r.pending, vec![[0., 1.]]);
}
#[test]
fn exact_section_carrier_keeps_profile_identity_and_complete_span_budget() {
    let mut s = surface();
    s.weights = vec![vec![2., 6.], vec![8., 25.]];
    let p = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
    let cut = p.section(0, 0.5).unwrap();
    assert!(std::ptr::eq(cut.profile(), &p));
    assert_eq!(
        (cut.axis(), cut.parameter(), cut.range()),
        (0, 0.5, [0., 1.])
    );
    for v in [0., 0.3, 1.] {
        let r = cut.enclose([v, v], 1).unwrap().0.unwrap();
        let expected = [0.8, 3. * v / (1. + 2. * v), 0.];
        let tangent = cut.tangent_bounds([v, v], 1).unwrap().0.unwrap();
        let transverse = cut.transverse_bounds([v, v], 1).unwrap().0.unwrap();
        for (bounds, expected) in [
            (tangent, [0., 3. / (1. + 2. * v).powi(2), 0.]),
            (transverse, [0.64, 0., 0.]),
        ] {
            for k in 0..3 {
                assert!(bounds[k][0] <= expected[k] && expected[k] <= bounds[k][1]);
            }
        }
        for k in 0..3 {
            assert!(r[k][0] <= expected[k] && r[k][1] >= expected[k]);
        }
    }
    assert!(cut.support_normals([0.3, 0.3], 1).unwrap().0.unwrap()[2][0] > 0.);
    assert!(cut.enclose([-0.1, 0.5], 1).is_err());
    assert!(p.section(2, 0.5).is_err());
    assert!(p.section(0, 1.1).is_err());
    assert!(
        p.section(0, 0.5)
            .unwrap()
            .end_transition([0.; 3], 0.)
            .is_err()
    );
    let bridge=crate::normal_alignment::ProfileSectionBridge::new(p.section(0,0.5).unwrap(),p.section(0,0.75).unwrap(),[0.25,0.25]).unwrap();
    for t in [0.,0.5,1.] {
        let bounds=bridge.enclose([0.3,0.3],[t,t],4).unwrap().0.unwrap();
        let h00=(1.-t)*(1.-t)*(1.+2.*t);let h10=t*(1.-t)*(1.-t);
        let h01=t*t*(3.-2.*t);let h11=-t*t*(1.-t);
        let expected=[h00*0.8+h10*0.25*0.64+h01*(3./3.25)+h11*0.25*(4./(3.25*3.25)),3.*0.3/1.6,0.];
        for k in 0..3 {assert!(bounds[k][0]<=expected[k]&&expected[k]<=bounds[k][1]);}
        let n=bridge.normal_bounds([0.3,0.3],[t,t],8).unwrap().0.unwrap();
        assert!(n[2][1]<0.);
    }
    for high in [false,true] {assert_eq!(bridge.regular_join(high,[0.2,0.4],1).unwrap().0,Some(true));}
    assert!(bridge.enclose([0.3,0.3],[0.5,0.5],3).unwrap().0.is_none());
    assert!(bridge.normal_bounds([0.3,0.3],[0.5,0.5],7).unwrap().0.is_none());
    let cover=bridge.inspect_regularity([0.,1.],[0.,1.],12000,96000).unwrap();
    assert!(cover.complete_regular,"{} pending={}",cover.reason,cover.pending.len());
    let area:f64=cover.regular.iter().map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();assert_eq!(area,1.);
    let refused=bridge.inspect_regularity([0.,1.],[0.,1.],0,8000).unwrap();
    assert_eq!(refused.pending,vec![[[0.,1.],[0.,1.]]]);
    let injective=bridge.inspect_injectivity([0.,1.],[0.,1.],[[0.,1.,0.],[1.,0.,0.]],12000,96000).unwrap();
    assert!(injective.complete_regular,"{} pending={}",injective.reason,injective.pending.len());
    let refused=bridge.inspect_injectivity([0.,1.],[0.,1.],[[0.;3];2],10,80).unwrap();
    assert!(!refused.complete_regular);assert!(!refused.pending.is_empty());
    let folded=crate::normal_alignment::ProfileSectionBridge::new(p.section(0,0.5).unwrap(),p.section(0,0.5).unwrap(),[0.25,0.25]).unwrap();
    assert!(!folded.inspect_injectivity([0.,1.],[0.,1.],[[0.,1.,0.],[1.,0.,0.]],200,1600).unwrap().complete_regular);
    let transition = p
        .section(0, 0.5)
        .unwrap()
        .end_transition([1., 1., 1.], 0.25)
        .unwrap();
    assert_eq!(
        transition
            .enclose([0., 1.], [1., 1.], 0)
            .unwrap()
            .0
            .unwrap(),
        [[1., 1.]; 3]
    );
    assert!(
        transition
            .enclose([0.3, 0.3], [0.5, 0.5], 1)
            .unwrap()
            .0
            .is_none()
    );
    for (t, v) in [(0., 0.2), (0.5, 0.3), (1., 0.8)] {
        let bounds = transition.enclose([v, v], [t, t], 2).unwrap().0.unwrap();
        let source = [0.8, 3. * v / (1. + 2. * v), 0.];
        let derivative = [0.64, 0., 0.];
        let h00 = 2. * t * t * t - 3. * t * t + 1.;
        let h10 = t * t * t - 2. * t * t + t;
        let h01 = 3. * t * t - 2. * t * t * t;
        for k in 0..3 {
            let value = h00 * source[k] + h10 * 0.25 * derivative[k] + h01;
            assert!(bounds[k][0] <= value && value <= bounds[k][1]);
        }
    }
    assert_eq!(
        transition.regular_join([0.2, 0.4], 1).unwrap().0,
        Some(true)
    );
    assert!(transition.regular_join([0.2, 0.4], 0).is_err());
    let (tip, spent) = transition.tip_limits(1e-6, 8).unwrap();
    let tip = tip.unwrap();
    assert_eq!(spent, 8);
    assert_eq!(tip.point, [1., 1., 1.]);
    assert!(!tip.distinct_tangent_limits);
    assert!(tip.sine_squared_interval.unwrap()[1] < 1e-6);
    assert!(transition.tip_limits(1e-6, 7).unwrap().0.is_none());
    assert!(transition.tip_limits(f64::NAN, 8).is_err());

    let factored = transition
        .factored_normal_bounds([0.3, 0.3], [0.5, 0.5], 4)
        .unwrap()
        .0
        .unwrap();
    for (k, expected) in [7.03125, 0., -1.21875].into_iter().enumerate() {
        assert!(factored[k][0] <= expected && expected <= factored[k][1]);
    }
    let raw = transition
        .normal_bounds([0.3, 0.3], [0.5, 0.5], 4)
        .unwrap()
        .0
        .unwrap();
    for k in 0..3 {
        assert!(factored[k][0] * 0.125 <= raw[k][1] && raw[k][0] <= factored[k][1] * 0.125);
    }
    assert!(
        transition
            .factored_normal_bounds([0.3, 0.3], [0.5, 0.5], 3)
            .unwrap()
            .0
            .is_none()
    );

    let empty = transition
        .inspect_regularity([0., 1.], [0., 1.], 0, 100)
        .unwrap();
    assert_eq!(empty.pending, vec![[[0., 1.], [0., 1.]]]);
    assert_eq!(empty.cells, 0);
    let refined = transition
        .inspect_regularity([0., 1.], [0., 1.], 200, 800)
        .unwrap();
    let area: f64 = refined
        .regular
        .iter()
        .chain(&refined.pending)
        .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
        .sum();
    assert_eq!(area, 1.);
    assert!(!refined.complete_regular);
    assert!(!refined.regular.is_empty());
    assert!(refined.regular.iter().all(|r| r[1][1] < 1.));
    assert!(refined.pending.iter().any(|r| r[1][1] == 1.));
    assert!(refined.cells <= 200 && refined.spans <= 800);

    for t in [0., 0.5, 1.] {
        let free = transition
            .free_derivative_bounds([0.3, 0.3], [t, t], 2)
            .unwrap()
            .0
            .unwrap();
        let expected = (2. * t * t * t - 3. * t * t + 1.) * 3. / 1.6f64.powi(2);
        assert!(free[1][0] <= expected && expected <= free[1][1]);
        assert!(
            transition
                .normal_bounds([0.3, 0.3], [t, t], 4)
                .unwrap()
                .0
                .is_some()
        );
    }
    assert!(
        transition
            .normal_bounds([0.3, 0.3], [0.5, 0.5], 3)
            .unwrap()
            .0
            .is_none()
    );
    for high in [false, true] {
        let rail = transition.rail(high);
        let v = if high { 1. } else { 0. };
        assert!(std::ptr::eq(rail.transition(), &transition));
        assert_eq!(rail.free_parameter(), v);
        for t in [0., 0.5, 1.] {
            let bounds = rail.tangent_bounds([t, t], 2).unwrap().0.unwrap();
            let source = [0.8, 3. * v / (1. + 2. * v), 0.];
            let derivative = [0.64, 0., 0.];
            for k in 0..3 {
                let expected = (6. * t * t - 6. * t) * source[k]
                    + (3. * t * t - 4. * t + 1.) * 0.25 * derivative[k]
                    + 6. * t
                    - 6. * t * t;
                assert!(bounds[k][0] <= expected && expected <= bounds[k][1]);
            }
            assert_eq!(
                rail.enclose([t, t], 2).unwrap(),
                transition.enclose([v, v], [t, t], 2).unwrap()
            );
        }
        assert!(rail.tangent_bounds([0.2, 0.8], 1).unwrap().0.is_none());
        assert!(rail.tangent_bounds([-0.1, 0.5], 2).is_err());
    }
    s.knots_v = vec![0., 0., 0.5, 1., 1.];
    for row in &mut s.control_points {
        let x = row[0][0];
        row.insert(1, vec![x, 0.5, 0.]);
    }
    for row in &mut s.weights {
        row.insert(1, 4.);
    }
    let p = qualify_weight_profile(&s, [0, 0], 6).unwrap().0.unwrap();
    let cut = p.section(0, 0.5).unwrap();
    assert!(cut.enclose([0., 1.], 1).unwrap().0.is_none());
    assert!(cut.support_normals([0., 1.], 1).unwrap().0.is_none());
    assert!(cut.enclose([0., 1.], 2).unwrap().0.is_some());
    assert!(cut.tangent_bounds([0., 1.], 1).unwrap().0.is_none());
    assert!(cut.transverse_bounds([0., 1.], 1).unwrap().0.is_none());
    assert!(cut.tangent_bounds([0., 1.], 2).unwrap().0.is_some());
    assert!(cut.transverse_bounds([0., 1.], 2).unwrap().0.is_some());
}
#[test]
fn source_difference_uses_common_derivative_and_refuses_discontinuity() {
    let mut s=surface();
    for row in &mut s.control_points {for p in row {p[0]+=1e8;}}
    let p=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    for parameters in [[0.5,0.75],[0.75,0.5],[0.5,0.5]] {
        let (bounds,spent)=p.coordinate_difference_bounds(0,parameters,[0.,1.],1).unwrap();
        let bounds=bounds.unwrap();
        let expected=parameters[1]-parameters[0];
        assert!(bounds[0][0]<=expected&&expected<=bounds[0][1]);
        if parameters[0]!=parameters[1] {
            let a=p.enclose([[parameters[0];2],[0.,1.]],1).unwrap().0.unwrap();
            let b=p.enclose([[parameters[1];2],[0.,1.]],1).unwrap().0.unwrap();
            assert!(bounds[0][1]-bounds[0][0]<(a[0][1]-a[0][0])+(b[0][1]-b[0][0]));
        }
        assert_eq!(spent,usize::from(parameters[0]!=parameters[1]));
    }
    s.knots_u=vec![0.,0.,0.5,0.5,1.,1.];
    s.control_points.insert(1,vec![vec![1e8+0.25,0.,0.],vec![1e8+0.25,1.,0.]]);
    s.control_points.insert(2,vec![vec![1e8+0.75,0.,0.],vec![1e8+0.75,1.,0.]]);
    s.weights=vec![vec![1.;2];4];
    assert!(qualify_weight_profile(&s,[0,0],8).is_err());
}
#[test]
fn higher_rational_partials_match_independent_fourth_derivative() {
    let mut s=surface();s.weights=vec![vec![2.,2.],vec![8.,8.]];
    let p=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    for u in [0.,0.3,1.] {
        let bounds=p.higher_derivative_bounds([[u;2],[0.2,0.8]],[4,0],1).unwrap().0.unwrap();
        let expected=-2592./(1.+3.*u).powi(5);
        assert!(bounds[0][0]<=expected && expected<=bounds[0][1]);
        let mixed=p.higher_derivative_bounds([[u;2],[0.2,0.8]],[4,1],1).unwrap().0.unwrap();
        for r in mixed {assert!(r[0]<=0. && r[1]>=0.);}
    }
    assert!(p.higher_derivative_bounds([[0.,1.];2],[4,2],1).is_err());
}
#[test]
fn rational_profile_second_partials_match_independent_formula() {
    let mut s=surface();s.weights=vec![vec![2.,2.],vec![8.,8.]];
    let p=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    for u in [0.,0.3,1.] {for v in [0.,0.5,1.] {
        for axes in [[0,0],[0,1],[1,0],[1,1]] {
            let (bounds,spent)=p.second_derivative_bounds([[u;2],[v;2]],axes,1).unwrap();
            assert_eq!(spent,1);
            let bounds=bounds.unwrap();
            let expected=if axes==[0,0] {[-24./(1.+3.*u).powi(3),0.,0.]}else{[0.;3]};
            for k in 0..3 {assert!(bounds[k][0]<=expected[k]&&expected[k]<=bounds[k][1]);}
        }
    }}
    assert!(p.second_derivative_bounds([[0.,1.];2],[0,2],1).is_err());
    assert_eq!(p.second_derivative_bounds([[0.,1.];2],[0,1],1).unwrap(),p.mixed_derivative_bounds([[0.,1.];2],1).unwrap());
    s.knots_u=vec![0.,0.,0.5,1.,1.];
    s.control_points.insert(1,vec![vec![0.5,0.,0.],vec![0.5,1.,0.]]);
    s.weights.insert(1,vec![4.,4.]);
    let p=qualify_weight_profile(&s,[0,0],6).unwrap().0.unwrap();
    assert!(p.second_derivative_bounds([[0.,1.];2],[0,0],1).unwrap().0.is_none());
    assert!(p.second_derivative_bounds([[0.,1.];2],[0,0],2).unwrap().0.is_some());
}
#[test]
fn transverse_rail_correction_restores_independent_quartic_patch() {
    let mut s=surface();
    s.degree_u=4;s.degree_v=2;
    s.knots_u=vec![0.,0.,0.,0.,0.,1.,1.,1.,1.,1.];
    s.knots_v=vec![0.,0.,0.,1.,1.,1.];
    s.control_points=(0..5).map(|i|(0..3).map(|j|
        vec![i as f64/4.,j as f64/2.,if i==4 && j==2 {1.}else{0.}]
    ).collect()).collect();
    s.weights=vec![vec![1.;3];5];
    let p=qualify_weight_profile(&s,[0,0],15).unwrap().0.unwrap();
    let bridge=crate::normal_alignment::ProfileSectionBridge::new(
        p.section(0,0.5).unwrap(),p.section(0,0.75).unwrap(),[0.25;2]).unwrap();
    let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
    for high in [false,true] {
        let join=corrected.inspect_transverse_corrected_section_join(high,[0.,1.],100,1000).unwrap();
        assert!(join.complete_regular);assert!(join.pending.is_empty());
        assert_eq!(join.reason,"full-section-join-qualified");
        let refused=corrected.inspect_transverse_corrected_section_join(high,[0.,1.],0,1000).unwrap();
        assert!(!refused.complete_regular);assert!(!refused.pending.is_empty());
    }
    for free in [0.2,0.5,0.8] {for across in [0.2,0.5,0.8] {
        let u=0.5+0.25*across;
        let exact=u*u*u*u*free*free;
        let full=corrected.enclose_with_transverse_correction([free;2],[across;2],32).unwrap().0.unwrap();
        assert!(full[2][0]<=exact && exact<=full[2][1]);
        let (jet,spent)=corrected.derivatives_with_transverse_correction([free;2],[across;2],61).unwrap();
        assert_eq!(spent,61);
        let jet=jet.unwrap();
        let expected=[[0.,1.,2.*u.powi(4)*free],[0.25,0.,u.powi(3)*free*free]];
        for axis in 0..2 {for k in 0..3 {assert!(jet[axis][k][0]<=expected[axis][k] && expected[axis][k]<=jet[axis][k][1]);}}
        assert!(corrected.derivatives_with_transverse_correction([free;2],[across;2],60).unwrap().0.is_none());

        let old=corrected.enclose([free;2],[across;2],14).unwrap().0.unwrap();
        assert!(exact<old[2][0] || exact>old[2][1],"position-only correction must differ on this reference");
    }}
    for free in [0.,1.] {
        let (jet,spent)=corrected.derivatives_with_transverse_correction([free;2],[0.,1.],1).unwrap();
        assert_eq!(spent,1);
        let jet=jet.unwrap();
        for across in [0.,0.5,1.] {
            let u: f64=0.5+0.25*across;
            let expected=[[0.,1.,2.*u.powi(4)*free],[0.25,0.,u.powi(3)*free*free]];
            for axis in 0..2 {for k in 0..3 {
                assert!(jet[axis][k][0]<=expected[axis][k] && expected[axis][k]<=jet[axis][k][1]);
            }}
        }
        assert!(corrected.derivatives_with_transverse_correction([free;2],[0.,1.],0).is_err());
    }
    let projection=[[0.,1.,0.],[4.,0.,0.]];
    let injective=corrected.inspect_transverse_corrected_injectivity([0.,1.],[0.,1.],projection,1000,72000).unwrap();
    assert!(injective.complete_regular);
    let injective_area:f64=injective.regular.iter().map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
    assert_eq!(injective_area,1.);
    let failed=corrected.inspect_transverse_corrected_injectivity([0.,1.],[0.,1.],[[0.;3];2],1,61).unwrap();
    assert!(!failed.complete_regular);
    assert_eq!(failed.pending.len(),2);
    assert!(corrected.inspect_transverse_corrected_injectivity([0.,1.],[0.,1.],[[f64::NAN;3];2],1,61).is_err());
    let cover=corrected.inspect_transverse_corrected_regularity([0.,1.],[0.,1.],1000,72000).unwrap();
    assert!(cover.complete_regular);
    let area:f64=cover.regular.iter().map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
    assert_eq!(area,1.);
    let partial=corrected.inspect_transverse_corrected_regularity([0.,1.],[0.,1.],100,5700).unwrap();
    assert!(!partial.regular.is_empty(),"bounded traversal should reach certifiable local rectangles");
    let total:f64=partial.regular.iter().chain(&partial.pending).map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
    assert_eq!(total,1.);
    let limited=corrected.inspect_transverse_corrected_regularity([0.,1.],[0.,1.],0,72).unwrap();
    assert!(!limited.complete_regular);
    assert_eq!(limited.pending,vec![[[0.,1.],[0.,1.]]]);
    for high in [false,true] {
        let join=corrected.inspect_transverse_corrected_side_join(high,[0.,1.],100,100).unwrap();
        assert!(join.complete_regular);
        let length:f64=join.regular.iter().map(|r|r[1][1]-r[1][0]).sum();
        assert_eq!(length,1.);
        let pending=corrected.inspect_transverse_corrected_side_join(high,[0.,1.],0,100).unwrap();
        assert!(!pending.complete_regular);
        assert_eq!(pending.pending.len(),1);
    }
}
#[test]
fn bridge_mixed_jet_matches_independent_bilinear_surface() {
    let mut s = surface();
    s.weights = vec![vec![1.,1.],vec![1.,1.]];
    s.control_points[1][1][2] = 3.; // S(u,v) = [u,v,3uv].
    let p = qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    for axis in [0,1] {
        let bridge = crate::normal_alignment::ProfileSectionBridge::new(
            p.section(axis,0.25).unwrap(),p.section(axis,0.75).unwrap(),[0.5;2]).unwrap();
        for free in [0.,0.3,1.] { for across in [0.,0.5,1.] {
            let (bounds,spent)=bridge.mixed_derivative_bounds([free;2],[across;2],8).unwrap();
            assert_eq!(spent,8);
            let bounds=bounds.unwrap();
            for (k,expected) in [0.,0.,1.5].into_iter().enumerate() {
                assert!(bounds[k][0]<=expected && expected<=bounds[k][1]);
            }
        }}
        let bounds=bridge.mixed_derivative_bounds([0.,1.],[0.,1.],8).unwrap().0.unwrap();
        assert!(bounds[2][0]<=1.5 && bounds[2][1]>=1.5);
        assert!(bridge.mixed_derivative_bounds([0.;2],[0.5;2],7).unwrap().0.is_none());
        assert!(bridge.mixed_derivative_bounds([0.;2],[-0.1,0.5],8).is_err());
    }
}
#[test]
fn original_rail_correction_matches_independent_separable_surface_positions() {
    let mut s=surface();s.weights=vec![vec![2.,2.],vec![8.,8.]];
    let p=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    let bridge=crate::normal_alignment::ProfileSectionBridge::new(
        p.section(0,0.5).unwrap(),p.section(0,0.75).unwrap(),[0.25;2]).unwrap();
    let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
    for free in [0.,0.3,1.] {for across in [0.,0.5,1.] {
        let bounds=corrected.enclose([free;2],[across;2],14).unwrap().0.unwrap();
        let u=0.5+0.25*across;let expected=[4.*u/(1.+3.*u),free,0.];
        let full=corrected.enclose_with_transverse_correction([free;2],[across;2],32).unwrap().0.unwrap();
        for k in 0..3 {assert!(full[k][0]<=expected[k]&&expected[k]<=full[k][1]);}

        for k in 0..3 {assert!(bounds[k][0]<=expected[k]&&expected[k]<=bounds[k][1]);}
        let derivative=corrected.derivative_bounds([free;2],[across;2],36).unwrap().0.unwrap();
        let expected=[[0.,1.,0.],[1./(1.+3.*u).powi(2),0.,0.]];
        for axis in 0..2 {for k in 0..3 {
            assert!(derivative[axis][k][0]<=expected[axis][k]&&expected[axis][k]<=derivative[axis][k][1]);
        }}

    }}
    assert!(corrected.enclose([0.3;2],[0.5;2],13).unwrap().0.is_none());
    assert!(corrected.enclose_with_transverse_correction([0.3;2],[0.5;2],31).unwrap().0.is_none());

    assert!(bridge.constrain_original_rails(0).unwrap().0.is_none());
}
#[test]
fn exact_profile_pole_keeps_distinct_limiting_planes() {
    let mut s = surface();
    s.degree_u = 2;
    s.knots_u = vec![0., 0., 0., 1., 1., 1.];
    s.degree_v = 2;
    s.knots_v = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = vec![
        vec![vec![0., 0., 0.]; 3],
        vec![vec![0.5, 0., 0.]; 3],
        vec![vec![1., 0., 0.], vec![1., 0.5, 0.], vec![1., 1., 1.]],
    ];
    s.weights = vec![vec![1.; 3]; 3];
    let profile = qualify_weight_profile(&s, [0, 0], 9).unwrap().0.unwrap();
    let (limits, spent) = profile.pole_limits(false, 1e-6, 4).unwrap();
    let limits = limits.unwrap();
    assert_eq!(spent, 4);
    assert_eq!(limits.point, [0.; 3]);
    assert!(limits.distinct_tangent_limits);
    let (factored,spent)=profile.factored_pole_limits(false,1e-6,14).unwrap();
    let factored=factored.unwrap();assert_eq!(spent,14);
    assert_eq!(factored.point,[0.;3]);assert!(factored.distinct_tangent_limits);
    assert!(factored.sine_squared_interval.unwrap()[0]>1e-6);
    assert!(profile.factored_pole_limits(false,1e-6,13).unwrap().0.is_none());
    assert!(profile.factored_pole_limits(true,1e-6,14).unwrap().0.is_none());
    assert!(profile.pole_limits(false, 1e-6, 3).unwrap().0.is_none());
    assert!(profile.pole_limits(true, 1e-6, 4).unwrap().0.is_none());
}
#[test]
fn two_corrected_profiles_use_their_actual_normals() {
    let mut a = surface();
    a.control_points[1][1][2] = 1.;
    a.weights[1][1] = 1.5;
    let mut b = a.clone();
    b.weights[1][1] = 2.;
    // Same factor recipe makes the proposed surfaces equal, despite
    // different original interior weights.
    let p = qualify_weight_profile(&a, [0, 0], 4).unwrap().0.unwrap();
    let q = qualify_weight_profile(&b, [0, 0], 4).unwrap().0.unwrap();
    let line = [[[0., 0.5], [1., 0.5]], [[0., 0.5], [1., 0.5]]];
    let r = p.inspect_profile_path(&q, line, 1e-4, 1000, 2000).unwrap();
    assert_eq!(r.aligned, Some(true));
    assert!(r.pending.is_empty());
    let r = p.inspect_profile_path(&q, line, 1e-4, 1, 1).unwrap();
    assert_eq!(r.aligned, None);
    assert!(!r.pending.is_empty());
    b.control_points[1][0][2] = 2.;
    let tilted = qualify_weight_profile(&b, [0, 0], 4).unwrap().0.unwrap();
    assert_eq!(
        p.inspect_profile_path(&tilted, line, 1e-4, 1000, 2000)
            .unwrap()
            .aligned,
        Some(false)
    );
}
#[test]
fn adaptive_curved_path_covers_all_subintervals() {
    let mut s = surface();
    s.control_points[1][1][2] = 1.;
    let profile = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
    let line = [[[0., 0.5], [1., 0.5]], [[0., 0.5], [1., 0.5]]];
    let limited = profile
        .inspect_original_path(&s, line, 1e-4, 2, 1000)
        .unwrap();
    assert_eq!(limited.aligned, None);
    assert!(!limited.pending.is_empty());
    let full = profile
        .inspect_original_path(&s, line, 1e-4, 1000, 2000)
        .unwrap();
    assert_eq!(full.aligned, Some(true));
    assert!(full.pending.is_empty());
    assert!(full.cells > 1 && full.accepted_cells > 1);
    assert!(full.upper_sine_squared.unwrap() <= 1e-4);
}
#[test]
fn proposed_original_tangent_planes_require_complete_work() {
    let s = surface();
    let profile = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
    let domains = [[[0., 1.], [0., 0.]], [[0., 1.], [0., 0.]]];
    let aligned = profile
        .inspect_original_normals(&s, domains, 1e-12, 2)
        .unwrap();
    assert_eq!(aligned.aligned, Some(true));
    assert_eq!(aligned.spans, 2);
    let short = profile
        .inspect_original_normals(&s, domains, 1e-12, 1)
        .unwrap();
    assert_eq!(short.aligned, None);
    assert_eq!(short.reason, "span-limit");
    let mut tilted = surface();
    for row in &mut tilted.control_points {
        for p in row {
            p[2] = p[1];
        }
    }
    let mismatch = profile
        .inspect_original_normals(&tilted, domains, 1e-12, 2)
        .unwrap();
    assert_eq!(mismatch.aligned, Some(false));
    let mut collapsed = surface();
    for row in &mut collapsed.control_points {
        for p in row {
            p[1] = 0.;
        }
    }
    assert_eq!(
        profile
            .inspect_original_normals(&collapsed, domains, 1e-12, 2)
            .unwrap()
            .aligned,
        None
    );
    assert!(
        profile
            .inspect_original_normals(&s, domains, f64::NAN, 2)
            .is_err()
    );
}
#[test]
fn profile_normals_cover_exact_rational_derivatives() {
    let mut s = surface();
    s.weights = vec![vec![2., 6.], vec![8., 25.]];
    let profile = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
    for u in [0., 0.2, 0.5, 1.] {
        for v in [0., 0.3, 0.7, 1.] {
            let (normal, spent) = profile.normal_bounds([[u, u], [v, v]], 1).unwrap();
            assert_eq!(spent, 1);
            let n = normal.unwrap();
            // Target separability: x=4u/(1+3u), y=3v/(1+2v).
            let expected = 12. / ((1. + 3. * u).powi(2) * (1. + 2. * v).powi(2));
            assert!(n[0][0] <= 0. && n[0][1] >= 0. && n[1][0] <= 0. && n[1][1] >= 0.);
            assert!(n[2][0] <= expected && n[2][1] >= expected && n[2][0] > 0.);
        }
    }
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    s.control_points
        .insert(1, vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]]);
    s.weights.insert(1, vec![4., 12.]);
    let profile = qualify_weight_profile(&s, [0, 0], 6).unwrap().0.unwrap();
    let (missing, spent) = profile.normal_bounds([[0.2, 0.8], [0.3, 0.7]], 1).unwrap();
    assert!(missing.is_none());
    assert_eq!(spent, 1);
    assert!(
        profile
            .normal_bounds([[0.5, 0.5], [0.5, 0.5]], 2)
            .unwrap()
            .0
            .is_some()
    );
}
#[test]
fn tiny_weight_change_is_bounded_without_mutating_source() {
    let mut s = surface();
    s.weights[1][1] = 1_f64.next_up();
    let original = s.weights.clone();
    let (proof, spent) = qualify_weight_profile(&s, [0, 0], 4).unwrap();
    let proof = proof.unwrap();
    assert_eq!(spent, 4);
    assert!(std::ptr::eq(proof.source(), &s));
    assert!(proof.within_position_tolerance(1e-12).unwrap());
    assert!(!proof.within_position_tolerance(0.).unwrap());
    assert!(proof.within_position_tolerance(f64::NAN).is_err());
    let (row, column, pivot) = proof.factors();
    assert_eq!(row, &[1., 1.]);
    assert_eq!(column, &[1., 1.]);
    assert_eq!(pivot, 1.);
    let bound = proof.axis_deviation_upper();
    for u in [0.1, 0.4, 0.9] {
        for v in [0.1, 0.4, 0.9] {
            let p = s.evaluate(u, v).unwrap().point;
            assert!((p[0] - u).abs() <= bound[0] && (p[1] - v).abs() <= bound[1]);
        }
    }
    assert_eq!(s.weights, original);
}
#[test]
fn missing_controls_and_large_changes_do_not_pass_tolerance() {
    let mut s = surface();
    s.weights[1][1] = 2.;
    let (proof, spent) = qualify_weight_profile(&s, [0, 0], 3).unwrap();
    assert!(proof.is_none());
    assert_eq!(spent, 3);
    let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
    assert!(!proof.within_position_tolerance(1e-7).unwrap());
    assert!(qualify_weight_profile(&s, [2, 0], 4).is_err());
    assert!(qualify_weight_profile(&s, [0, 0], 0).unwrap().0.is_none());
}
#[test]
fn exact_profile_geometry_matches_independent_separable_formula() {
    let mut s = surface();
    s.weights = vec![vec![1., 3.], vec![2., 6.]];
    let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
    for u in [0.2, 0.37, 0.4] {
        for v in [0.3, 0.41, 0.5] {
            let (box_, spent) = proof.enclose([[u, u], [v, v]], 1).unwrap();
            let box_ = box_.unwrap();
            assert_eq!(spent, 1);
            let expected = [2. * u / (1. + u), 3. * v / (1. + 2. * v), 0.];
            for (r, x) in box_.into_iter().zip(expected) {
                assert!(r[0] <= x && x <= r[1], "{r:?} misses {x}");
            }
        }
    }
    assert!(proof.enclose([[-0.1, 0.2], [0., 1.]], 1).is_err());
    assert!(proof.enclose([[0., 1.]; 2], 0).is_err());
}
#[test]
fn profile_geometry_never_returns_partial_span_union() {
    let mut s = surface();
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    s.control_points
        .insert(1, vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]]);
    s.weights = vec![vec![1.; 2]; 3];
    let proof = qualify_weight_profile(&s, [0, 0], 6).unwrap().0.unwrap();
    let (missing, spent) = proof.enclose([[0.2, 0.8], [0.3, 0.7]], 1).unwrap();
    assert!(missing.is_none());
    assert_eq!(spent, 1);
    let (bounds, spent) = proof.enclose([[0.2, 0.8], [0.3, 0.7]], 2).unwrap();
    let bounds = bounds.unwrap();
    assert_eq!(spent, 2);
    assert!(
        bounds[0][0] <= 0.2
            && bounds[0][1] >= 0.8
            && bounds[1][0] <= 0.3
            && bounds[1][1] >= 0.7
    );
}
#[test]
fn exact_profile_separability_removes_constant_rows_at_different_points() {
    let mut s = surface();
    s.degree_u = 2;
    s.knots_u = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 1., 0.]],
        vec![vec![0.5, 0., 0.]; 2],
        vec![vec![1., 0., 0.]; 2],
    ];
    s.weights = vec![vec![1., 0.7], vec![1., 0.7_f64.next_up()], vec![1., 0.7]];
    let profile = qualify_weight_profile(&s, [0, 0], 6).unwrap().0.unwrap();
    let proof = profile
        .boundary_direction([[0.99, 1.], [0.2, 0.3]], 1, 1., 1)
        .unwrap()
        .0
        .unwrap();
    assert!(std::ptr::eq(proof.profile(), &profile));
    assert_eq!(proof.factor(), Some((true, 2)));
    let r = proof.components()[1];
    assert!(r[0] > 0.);
    // Profile y=(1-u)^2 * 0.7v/(1-0.3v); its boundary secant
    // divided by (1-u)^2 equals 1/(1-0.3v), including u=1.
    for v in [0.2, 0.25, 0.3] {
        let expected = 1. / (1. - 0.3 * v);
        assert!(r[0] <= expected && expected <= r[1]);
    }
    assert!(
        profile
            .boundary_direction([[0.99, 1.], [0.2, 0.3]], 1, 1., 0)
            .is_err()
    );
}
#[test]
fn separable_rational_boundaries_preserve_exact_weight_products() {
    for scale in [f64::from_bits(1),1.,f64::MAX/4.] {
        assert!(exact_positive_products_equal(scale,2.,scale*2.,1.));
        assert!(!exact_positive_products_equal(scale,2.,(scale*2.).next_up(),1.));
    }
    assert!(exact_positive_products_equal(f64::MAX,f64::MAX,f64::MAX,f64::MAX));
    assert!(!exact_positive_products_equal(0.,1.,0.,1.));
    assert!(!exact_positive_products_equal(f64::INFINITY,1.,1.,1.));
    let mut s=surface();s.weights=vec![vec![1.,1.125],vec![1.25,1.40625]];
    let profile=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    for axis in [0,1] {for high in [false,true] {
        assert!(profile.preserved_boundary(axis,high,2).unwrap().0.is_some());
        assert!(profile.preserved_boundary(axis,high,1).unwrap().0.is_none());
    }}
    s.weights[1][1]=s.weights[1][1].next_up();
    let profile=qualify_weight_profile(&s,[0,0],4).unwrap().0.unwrap();
    for axis in [0,1] {assert!(profile.preserved_boundary(axis,true,2).unwrap().0.is_none());}
}
#[test]
fn preserved_boundaries_are_exact_and_budgeted() {
    let mut s = surface();
    s.weights[1][1] = 1_f64.next_up();
    let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
    for axis in [0, 1] {
        let (boundary, spent) = proof.preserved_boundary(axis, false, 2).unwrap();
        let boundary = boundary.unwrap();
        assert_eq!(spent, 2);
        assert!(std::ptr::eq(boundary.profile(), &proof));
        assert_eq!(boundary.axis(), axis);
        assert_eq!(boundary.boundary(), 0.);
        assert!(proof.preserved_boundary(axis, true, 2).unwrap().0.is_none());
        assert!(
            proof
                .preserved_boundary(axis, false, 1)
                .unwrap()
                .0
                .is_none()
        );
    }
    assert!(proof.preserved_boundary(2, false, 2).is_err());
    s.control_points[1] = vec![vec![1., 0., 0.]; 2];
    let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
    assert!(proof.preserved_boundary(0, true, 2).unwrap().0.is_some());
}
