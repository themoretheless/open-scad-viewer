#[test]
fn original_rational_cylinder_normals_preserve_far_translation_and_scale() {
    let w=std::f64::consts::FRAC_1_SQRT_2;
    for scale in [1e-3,1.,1e3] {
        for shift in [0.,1e6] {
            let s=crate::surface::Surface {
                degree_u:1,degree_v:2,knots_u:vec![0.,0.,1.,1.],
                knots_v:vec![0.,0.,0.,1.,1.,1.],
                control_points:[0.,1.].into_iter().map(|x|
                    [[x,1.,0.],[x,1.,1.],[x,0.,1.]].into_iter().map(|p|
                        vec![shift+scale*p[0],-2.*shift+scale*p[1],3.*shift+scale*p[2]]).collect()).collect(),
                weights:vec![vec![1.,w,1.];2],periodic_u:false,periodic_v:false,
            };
            let (normal,spent)=super::normal_bounds(&s,[[0.1,0.11],[0.1,0.11]],1).unwrap();
            let normal=normal.unwrap();assert_eq!(spent,1);
            assert!(normal[0][0]<=0. && normal[0][1]>=0.);
            assert!(normal[0][1]-normal[0][0]<1e-6*scale*scale,
                "original cylinder axial normal scale={scale} shift={shift} bounds={normal:?}");
            assert!(normal[1][1]<0. && normal[2][1]<0.,
                "nonzero transverse normal directions must survive translation");
            let jet=super::jet_on(&s,[1,2],[[0.1,0.11],[0.1,0.11]]).unwrap();
            for derivative in jet.second {
                assert!(derivative[0].lo<=0. && derivative[0].hi>=0.
                    && derivative[0].hi-derivative[0].lo<0.1*scale,
                    "original axial second partial scale={scale} shift={shift} bounds={derivative:?}");
            }
            let point=s.evaluate_validated(0.105,0.105).unwrap().point;
            for k in 0..3 {assert!(jet.point[k].lo<=point[k] && point[k]<=jet.point[k].hi,
                "centering must retain the original world position");}
        }
    }
}
#[test]
fn incompatible_endpoint_normals_exclude_overlapping_chord_boxes() {
    let boxes=[[[-1.,1.];3];2];
    let x=Some([[1.,1.],[0.,0.],[0.,0.]]);
    let y=Some([[0.,0.],[1.,1.],[0.,0.]]);
    assert!(super::endpoint_chord_excluded(boxes,[x,y],1e-4).unwrap());
    assert!(super::endpoint_chord_excluded(boxes,[y,x],1e-4).unwrap());
    assert!(!super::endpoint_chord_excluded(boxes,[x,y],0.5).unwrap());
    assert!(!super::endpoint_chord_excluded(boxes,[x,x],1e-4).unwrap());
    assert!(!super::endpoint_chord_excluded(boxes,[x,Some([[-1.,1.];3])],1e-4).unwrap());
}
#[test]
fn endpoint_chord_requires_continuous_direction_exclusion() {
    use super::endpoint_chord_excluded as excluded;
    let origin=[[0.,0.];3];
    let normal=Some([[0.,0.],[0.,0.],[1.,1.]]);
    assert!(excluded([origin,[[2.,3.],[0.,0.],[0.,0.]]],[normal,normal],1e-6).unwrap());
    assert!(!excluded([origin,[[0.,0.],[0.,0.],[2.,3.]]],[normal,normal],1e-6).unwrap());
    assert!(!excluded([origin,[[-1.,1.];3]],[normal,normal],1e-6).unwrap());
    assert!(!excluded([origin,[[2.,3.],[0.,0.],[0.,0.]]],[None,None],1e-6).unwrap());
    assert!(excluded([origin,[[2.,3.],[0.,0.],[0.,0.]]],[None,normal],1e-6).unwrap());
    assert!(excluded([origin,[[f64::NAN,3.],[0.,0.],[0.,0.]]],[normal,normal],1e-6).is_err());
}

#[test]
fn chord_angle_boundary_and_degenerate_cones_remain_unproved() {
    use super::endpoint_chord_excluded as excluded;
    let origin=[[0.,0.];3];
    let normal=Some([[0.,0.],[0.,0.],[1.,1.]]);
    let diagonal=[[1.,1.],[0.,0.],[1.,1.]];
    assert!(excluded([origin,diagonal],[normal,None],0.49).unwrap());
    assert!(!excluded([origin,diagonal],[normal,None],0.5).unwrap());
    assert!(!excluded([origin,diagonal],[normal,None],0.51).unwrap());
    assert!(!excluded([origin,diagonal],[Some([[0.,0.];3]),None],0.49).unwrap());
    assert!(!excluded([origin,diagonal],[Some([[-1.,1.];3]),None],0.49).unwrap());
    let bad=Some([[f64::NAN,1.],[0.,0.],[1.,1.]]);
    assert!(excluded([origin,[[2.,2.],[0.,0.],[0.,0.]]],[normal,bad],0.49).is_err());
    for scale in [1e-90,1.,1e90] {
        let along=[[0.,0.],[0.,0.],[scale,scale]];
        let across=[[scale,scale],[0.,0.],[0.,0.]];
        let n=Some([[0.,0.],[0.,0.],[-scale,-scale]]);
        assert!(excluded([origin,across],[n,None],1e-6).unwrap());
        assert!(!excluded([origin,along],[n,None],1e-6).unwrap());
        assert!(excluded([across,origin],[n,None],1e-6).unwrap());
    }
}

#[test]
fn locally_centered_rational_derivatives_enclose_independent_evaluations() {
    for offset in [0.,1_000_000.] {
        let s=Surface{degree_u:2,degree_v:2,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,0.,1.,1.,1.],
            control_points:(0..3).map(|u|(0..3).map(|v|vec![offset+u as f64,offset+v as f64,offset+(u*v) as f64]).collect()).collect(),
            weights:vec![vec![1.,0.7,1.],vec![0.9,0.63,0.9],vec![1.,0.7,1.]],periodic_u:false,periodic_v:false};
        for domain in [[[0.1,0.3],[0.7,0.9]],[[0.49,0.51],[0.49,0.51]]] {
            let j=super::jacobian_on(&s,[2,2],domain).unwrap();
            for u in 0..=8 {for v in 0..=8 {
                let t=|a:[f64;2],n:usize|a[0]+(a[1]-a[0])*n as f64/8.;
                let (du,dv)=s.evaluate(t(domain[0],u),t(domain[1],v)).unwrap().first_derivatives().unwrap();
                for k in 0..3 {for a in 0..2 {let d=[du,dv][a][k];assert!(j[k][a].lo<=d&&d<=j[k][a].hi);}}
            }}
        }
    }
}

use super::*;
#[test]
fn original_boundary_derivative_keeps_source_and_budget() {
    let s=plane();
    let domain=[[0.2,0.8],[1.,1.]];
    let (bound,spent)=derivative_path_bounds(&s,domain,0,1).unwrap();
    let bound=bound.unwrap();
    assert!(std::ptr::eq(bound.surface(),&s));
    assert_eq!(bound.domain(),domain); assert_eq!(bound.axis(),0); assert_eq!(spent,1);
    for (r,value) in bound.components().into_iter().zip([1.,0.,0.]) {
        assert!(r[0]<=value&&value<=r[1]);
    }
    assert!(derivative_path_bounds(&s,domain,2,1).is_err());
    assert!(derivative_path_bounds(&s,[[-0.1,0.8],[1.,1.]],0,1).is_err());
    assert!(derivative_path_bounds(&s,domain,0,0).is_err());
    let mut jump=plane();jump.knots_u=vec![0.,0.,0.5,0.5,1.,1.];
    jump.control_points=vec![jump.control_points[0].clone(),jump.control_points[1].clone(),
        vec![vec![3.,0.,0.],vec![3.,1.,0.]],vec![vec![4.,0.,0.],vec![4.,1.,0.]]];
    jump.weights=vec![vec![1.;2];4];
    assert!(derivative_path_bounds(&jump,domain,0,8).is_err());
    assert!(derivative_path_bounds(&jump,[[0.2,0.5],[1.,1.]],0,8).is_err());

}
#[test]
fn boundary_mean_integrates_complete_intervals_and_refuses_missing_work() {
    let mut s=plane();s.degree_v=2;s.knots_v=vec![0.,0.,0.,1.,1.,1.];
    s.control_points=vec![vec![vec![0.,0.,0.],vec![0.,0.,0.],vec![0.,1.,0.]],
        vec![vec![1.,0.,0.],vec![1.,0.,0.],vec![1.,1.,0.]]];s.weights=vec![vec![1.;3];2];
    let endpoints=[[0.2,0.8],[0.,0.]];
    let (bound,spans)=boundary_mean_bounds(&s,endpoints,1,1.,8,8).unwrap();
    let bound=bound.unwrap();assert_eq!(spans,8);assert!(std::ptr::eq(bound.surface(),&s));
    assert_eq!(bound.endpoints(),endpoints);assert_eq!(bound.axis(),1);assert_eq!(bound.boundary(),1.);
    let r=bound.components()[1];assert!(r[0]<=1.&&r[1]>=1.);assert!(r[1]-r[0]<0.26);
    assert!(boundary_mean_bounds(&s,endpoints,1,1.,8,7).unwrap().0.is_none());
    assert!(boundary_mean_bounds(&s,endpoints,1,0.5,8,8).is_err());
    assert!(boundary_mean_bounds(&s,endpoints,1,1.,17,100).is_err());
    s.weights=vec![vec![1.,0.7,1.];2];
    let rational=boundary_mean_bounds(&s,[[0.2,0.8],[0.,0.2]],1,1.,16,16).unwrap().0.unwrap();
    for i in 0..=20 {
        let v=i as f64*0.01;
        let value=v*v/((1.-v)*(1.-v)+1.4*v*(1.-v)+v*v);
        let mean=(1.-value)/(1.-v);
        let r=rational.components()[1];assert!(r[0]<=mean&&mean<=r[1]);
    }

}
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
fn factored_product_accounts_queries_and_rejects_rebound_sources() {
    let mut a=plane();a.degree_u=2;a.knots_u=vec![0.,0.,0.,1.,1.,1.];
    a.control_points=vec![vec![vec![0.,0.,0.];2],vec![vec![0.5,0.,0.];2],
        vec![vec![1.,0.,0.],vec![1.,1.,0.]]];a.weights=vec![vec![1.;2];3];
    let clone=a.clone();
    let proof=crate::factored_surface_normals::prepare(&a,false,100_000_000).unwrap();
    let c=proof.certificate.as_ref().unwrap();
    let cloned=crate::factored_surface_normals::prepare(&clone,false,100_000_000).unwrap();
    let mut b=plane();for row in &mut b.control_points {for p in row {p.swap(0,2);}}
    let mut ca=OriginalNormalCache::new(&a);let mut cb=OriginalNormalCache::new(&b);
    let domains=[[[0.,1.];2];2];
    let refused=ca.inspect_product_with_factors(&mut cb,domains,1e-6,1,1,[Some(c),None],0).unwrap();
    assert!(!refused.excluded);assert_eq!(refused.unresolved,vec![domains]);
    let paid=ca.inspect_product_with_factors(&mut cb,domains,1e-6,1,1,[Some(c),None],1).unwrap();
    assert!(paid.excluded);assert_eq!(paid.factored_queries,1);
    let cached=ca.inspect_product_with_factors(&mut cb,domains,1e-6,1,0,[Some(c),None],0).unwrap();
    assert!(cached.excluded);assert_eq!(cached.factored_queries,0);assert_eq!(cached.spans,0);
    assert!(ca.inspect_product_with_factors(&mut cb,domains,1e-6,1,0,
        [cloned.certificate.as_ref(),None],0).is_err());
    let raw=ca.inspect_product(&mut cb,domains,1e-6,1,0).unwrap();
    assert!(!raw.excluded);assert_eq!(raw.unresolved,vec![domains]);
    let mut direct_a=OriginalNormalCache::new(&a);let mut direct_b=OriginalNormalCache::new(&b);
    let (limited,used)=direct_a.inspect_other_with_factors(&mut direct_b,domains,1e-6,1,[Some(c),None],0).unwrap();
    assert_eq!(used,0);assert_eq!(limited.aligned,None);
    let (paid,used)=direct_a.inspect_other_with_factors(&mut direct_b,domains,1e-6,1,[Some(c),None],1).unwrap();
    assert_eq!(used,1);assert_eq!(paid.aligned,Some(false));
    let (cached,used)=direct_a.inspect_other_with_factors(&mut direct_b,domains,1e-6,0,[Some(c),None],0).unwrap();
    assert_eq!(used,0);assert_eq!(cached.spans,0);assert_eq!(cached.normal_components,paid.normal_components);
    assert!(direct_a.inspect_other_with_factors(&mut direct_b,domains,1e-6,0,[cloned.certificate.as_ref(),None],0).is_err());
    assert!(direct_a.inspect_other_with_factors(&mut direct_b,domains,1e-6,0,[Some(c),None],100001).is_err());

}
#[test]
fn closed_product_excludes_only_complete_oblique_normal_coverage() {
    let a=plane();let mut b=plane();
    for row in &mut b.control_points {for p in row {p.swap(0,2);}}
    let domains=[[[0.,1.];2];2];
    let mut ca=OriginalNormalCache::new(&a);let mut cb=OriginalNormalCache::new(&b);
    let refused=ca.inspect_product(&mut cb,domains,1e-6,1,1).unwrap();
    assert!(!refused.excluded);assert_eq!(refused.unresolved,vec![domains]);
    let paid=ca.inspect_product(&mut cb,domains,1e-6,1,2).unwrap();
    assert!(paid.excluded);assert!(paid.unresolved.is_empty());
    let cached=ca.inspect_product(&mut cb,domains,1e-6,1,0).unwrap();
    assert!(cached.excluded);assert_eq!(cached.spans,0);
    let reverse=cb.inspect_product(&mut ca,domains,1e-6,1,0).unwrap();
    assert!(reverse.excluded);
    let mut same=OriginalNormalCache::new(&a);
    let aligned=ca.inspect_product(&mut same,domains,1e-6,100,2).unwrap();
    assert!(!aligned.excluded);assert_eq!(aligned.unresolved,vec![domains]);
    assert_eq!(aligned.reason,"product-aligned-region");
    assert!(ca.inspect_product(&mut cb,domains,1e-6,0,0).is_err());
    let mut invalid=domains;invalid[1][0]=[f64::NAN,1.];
    assert!(ca.inspect_product(&mut cb,invalid,1e-6,1,0).is_err());
}
#[test]
fn distinct_source_cache_reuses_only_complete_original_rectangles() {
    let a=plane();let mut b=plane();
    for row in &mut b.control_points {for p in row {p.swap(0,2);}}
    let domains=[[[0.,1.],[0.,1.]];2];
    let mut ac=OriginalNormalCache::new(&a);let mut bc=OriginalNormalCache::new(&b);
    let paid=ac.inspect_other(&mut bc,domains,1e-6,2).unwrap();
    let plain=inspect_pair([&a,&b],domains,1e-6,2).unwrap();
    assert_eq!(paid.aligned,Some(false));assert_eq!(paid.spans,2);
    assert_eq!(paid.normal_components,plain.normal_components);
    assert_eq!(paid.sine_squared_interval,plain.sine_squared_interval);
    let free=ac.inspect_other(&mut bc,domains,1e-6,0).unwrap();
    assert_eq!(free.spans,0);assert_eq!(free.aligned,paid.aligned);
    assert_eq!(free.sine_squared_interval,paid.sine_squared_interval);
    let mut changed=domains;changed[1][0]=[-0.1,1.];
    assert!(ac.inspect_other(&mut bc,changed,1e-6,0).is_err());
    assert!(ac.inspect_other(&mut bc,domains,1.,0).is_err());
    let c=plane();let mut cc=OriginalNormalCache::new(&c);
    let missing=ac.inspect_other(&mut cc,domains,1e-6,0).unwrap();
    assert_eq!(missing.aligned,None);assert_eq!(missing.normal_components[1],None);
    let fresh=ac.inspect_other(&mut cc,domains,1e-6,1).unwrap();
    assert_eq!(fresh.aligned,Some(true));assert_eq!(fresh.spans,1);
}
#[test]
fn incomplete_distinct_normal_queries_never_enter_the_cache() {
    let mut a=plane();a.knots_u=vec![0.,0.,0.5,1.,1.];
    a.control_points.push(vec![vec![2.,0.,1.],vec![2.,1.,1.]]);
    a.weights.push(vec![1.;2]);let b=plane();
    let mut ac=OriginalNormalCache::new(&a);let mut bc=OriginalNormalCache::new(&b);
    let domains=[[[0.,1.],[0.,1.]];2];
    for _ in 0..2 {
        let r=ac.inspect_other(&mut bc,domains,1e-6,1).unwrap();
        assert_eq!(r.spans,1);assert_eq!(r.aligned,None);
        assert_eq!(r.normal_components,[None,None]);
    }
    let full=ac.inspect_other(&mut bc,domains,1e-6,3).unwrap();
    assert_eq!(full.spans,3);assert!(full.normal_components.iter().all(Option::is_some));
    let cached=ac.inspect_other(&mut bc,domains,1e-6,0).unwrap();
    assert_eq!(cached.spans,0);assert_eq!(cached.normal_components,full.normal_components);
    let point=[[[0.5,0.5],[0.3,0.7]];2];
    let unpaid=ac.inspect_other(&mut bc,point,1e-6,0).unwrap();
    assert_eq!(unpaid.normal_components,[None,None]);
}
#[test]
fn exact_factored_limits_resolve_pole_angles_without_inventing_regular_normals() {
    // Independent S=(3u,3u²v,u³) has factored normal (-u²,0,1).
    let s = Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0usize..4)
            .map(|i| {
                (0..2)
                    .map(|v| {
                        vec![
                            i as f64,
                            (i * i.saturating_sub(1)) as f64 / 2. * v as f64,
                            if i == 3 { 1. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let pair = [[[0., 0.], [0., 1.]], [[1., 1.], [0., 1.]]];
    assert!(inspect_pair([&s, &s], pair, 0.1, 2)
        .unwrap()
        .aligned
        .is_none());
    let c = crate::factored_surface_normals::prepare(&s, false, 10_000_000)
        .unwrap()
        .certificate
        .unwrap();
    let r = inspect_pair_bounds(
        [c.bounds(pair[0]).unwrap(), c.bounds(pair[1]).unwrap()],
        0.1,
    )
    .unwrap();
    assert_eq!(r.aligned, Some(false));
    let bounds = r.sine_squared_interval.unwrap();
    assert!(bounds[0] <= 0.5 && bounds[1] >= 0.5);
    assert_eq!(r.spans, 0); // Pure comparison: the caller pays source queries.
    let z = [[0., 0.], [0., 0.], [1., 1.]];
    let minus_z = [[0., 0.], [0., 0.], [-1., -1.]];
    assert_eq!(
        inspect_pair_bounds([z, minus_z], 0.1).unwrap().aligned,
        Some(true)
    );
    assert!(inspect_pair_bounds([z, [[0., 0.]; 3]], 0.1)
        .unwrap()
        .aligned
        .is_none());
}
#[test]
#[ignore = "native restriction microbenchmark; no production speed claim"]
fn benchmark_closed_rational_restriction() {
    let (p, q) = (16, 8);
    let net = (0..=p)
        .map(|u| {
            (0..=q)
                .map(|v| {
                    let w = 1. + u as f64 / p as f64 + v as f64 / q as f64;
                    [I::point(w), I::point(1.), I::point(0.), I::point(w)]
                })
                .collect()
        })
        .collect::<Net>();
    let domain = [[0.125, 0.875], [0.25, 0.75]];
    let repeats = 50;
    let start = std::time::Instant::now();
    for _ in 0..repeats {
        let result = restrict_net(&net, [p, q], [[0., 1.], [0., 1.]], domain).unwrap();
        for h in result.iter().flatten() {
            assert!(h[0].div(h[3]).unwrap().contains(1.));
        }
        std::hint::black_box(result);
    }
    let blossom = start.elapsed().as_secs_f64();
    let start = std::time::Instant::now();
    for _ in 0..repeats {
        let result = restrict_unit_net(&net, domain).unwrap();
        for h in result.iter().flatten() {
            assert!(h[0].div(h[3]).unwrap().contains(1.));
        }
        std::hint::black_box(result);
    }
    let splits = start.elapsed().as_secs_f64();
    let hull = rational_quotient_hull(&net, domain).unwrap();
    for (u, v) in [(0.125, 0.25), (0.5, 0.5), (0.875, 0.75)] {
        assert!(hull[0].contains(1.) && hull[1].contains(1. / (1. + u + v)));
    }
    eprintln!("restriction-benchmark degree={p},{q} repeats={repeats} blossom_ms={} split_ms={} ratio={}",
        blossom*1000.,splits*1000.,blossom/splits);
}
#[test]
fn general_span_splits_enclose_independent_polynomials_and_closed_points() {
    let net:Net=(0..3).map(|u|(0..2).map(|v|
        [I::point(if u==2 {1.} else {0.}+2.*v as f64),
         I::point(u as f64/2.*v as f64),I::point(1.),I::point(1.)]).collect()).collect();
    for span in [[[0.,1.],[0.,1.]],[[1e6,1e6+8.],[-10.,-6.]]] {
        let point=|t:f64,v:f64|[span[0][0]+t*(span[0][1]-span[0][0]),
            span[1][0]+v*(span[1][1]-span[1][0])];
        let a=point(0.125,0.125);let b=point(0.875,0.875);
        let domain=[[a[0],b[0]],[a[1],b[1]]];
        let hull=restrict_hull(&net,[2,1],span,domain).unwrap();
        for t in [0.125,0.375,0.875] {for v in [0.125,0.625,0.875] {
            assert!(hull[0].contains(t*t+2.*v) && hull[1].contains(t*v));
        }}
        let p=point(0.375,0.625);
        let hull=restrict_hull(&net,[2,1],span,[[p[0];2],[p[1];2]]).unwrap();
        assert!(hull[0].contains(0.375*0.375+2.*0.625));
        assert!(hull[1].contains(0.375*0.625));
        let hull=restrict_hull(&net,[2,1],span,span).unwrap();
        assert!(hull[0].contains(0.) && hull[0].contains(3.));
        assert!(restrict_hull(&net,[2,1],span,
            [[span[0][0]-1.,span[0][1]],span[1]]).is_err());
    }
}
#[test]
fn rational_coefficient_quotients_keep_shared_denominator_and_reject_zero() {
    let net = vec![
        vec![[I::point(2.), I::point(1.), I::point(0.), I::point(2.)]; 2],
        vec![[I::point(10.), I::point(1.), I::point(0.), I::point(10.)]; 2],
    ];
    let full = rational_quotient_hull(&net, [[0., 1.], [0., 1.]]).unwrap();
    assert!(full[0].contains(1.) && full[0].hi - full[0].lo < 1e-12);
    for u in [0., 0.25, 0.5, 1.] {
        assert!(full[1].contains(1. / (2. + 8. * u)));
        let point = rational_quotient_hull(&net, [[u, u], [0.3, 0.3]]).unwrap();
        assert!(point[1].contains(1. / (2. + 8. * u)));
        assert!(point[1].hi - point[1].lo < 1e-12);
    }
    let mut negative = net.clone();
    for h in negative.iter_mut().flatten() {
        for x in h {
            *x = I::point(0.).sub(*x).unwrap();
        }
    }
    let signed = signed_rational_quotient_hull(&negative, [[0., 1.], [0., 1.]])
        .unwrap()
        .unwrap();
    assert!(signed[0].contains(1.) && signed[1].contains(0.25));
    negative[0][0][3] = I::point(1.);
    assert!(
        signed_rational_quotient_hull(&negative, [[0., 1.], [0., 1.]])
            .unwrap()
            .is_none()
    );
    let mut invalid = net;
    invalid[0][0][3] = I::point(0.);
    assert!(rational_quotient_hull(&invalid, [[0., 1.], [0., 1.]]).is_err());
}
#[test]
fn pair_normals_distinguish_tangency_fold_and_singular_transition() {
    let a = plane();
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            p[0] += 100.;
            p[1] -= 20.;
        }
    }
    let domains = [[[0., 1.], [0., 1.]]; 2];
    let r = inspect_pair([&a, &b], domains, 1e-6, 2).unwrap();
    assert_eq!(r.aligned, Some(true));
    assert_eq!(r.spans, 2);
    // Position is deliberately different: angular agreement is not G0/G1.
    b.control_points.reverse();
    assert_eq!(
        inspect_pair([&a, &b], domains, 1e-6, 2).unwrap().aligned,
        Some(true)
    );
    for row in &mut b.control_points {
        for p in row {
            p[2] = p[0] - 100.;
        }
    }
    let r = inspect_pair([&a, &b], domains, 1e-6, 2).unwrap();
    assert_eq!(r.aligned, Some(false));
    let d = r.sine_squared_interval.unwrap();
    assert!(d[0] <= 0.5 && d[1] >= 0.5);
    for row in &mut b.control_points {
        for p in row {
            p[1] = 0.;
            p[2] = 0.;
        }
    }
    let r = inspect_pair([&a, &b], domains, 1e-6, 2).unwrap();
    assert_eq!(r.aligned, None);
    assert_eq!(r.reason, "normal-unresolved");
    let r = inspect_pair([&a, &a], domains, 1e-6, 1).unwrap();
    assert_eq!(r.aligned, None);
    assert_eq!(r.reason, "span-limit");
    assert_eq!(r.spans, 1);
    let invalid = [domains[0], [[0., 2.], [0., 1.]]];
    assert!(inspect_pair([&a, &a], invalid, 1e-6, 1).is_err());
}
#[test]
fn parallel_antiparallel_oblique_and_collapsed_normals_are_distinct() {
    let s = plane();
    let before = format!("{s:?}");
    for d in [[0., 0., 1.], [0., 0., -1.], [0., 0., f64::MAX]] {
        let r = inspect(&s, [[0.2, 0.8]; 2], d, 1e-10, 1).unwrap();
        assert_eq!(r.aligned, Some(true));
    }
    let r = inspect(&s, [[0.2, 0.8]; 2], [1., 0., 1.], 1e-10, 1).unwrap();
    assert_eq!(r.aligned, Some(false));
    let b = r.sine_squared_interval.unwrap();
    assert!(b[0] <= 0.5 && b[1] >= 0.5);
    let mut collapsed = s.clone();
    collapsed.control_points[1] = collapsed.control_points[0].clone();
    assert_eq!(
        inspect(&collapsed, [[0.2, 0.8]; 2], [0., 0., 1.], 1e-10, 1)
            .unwrap()
            .aligned,
        None
    );
    assert_eq!(format!("{s:?}"), before);
}
#[test]
fn knot_sides_and_span_exhaustion_cannot_be_hidden() {
    let mut s = plane();
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    s.control_points
        .push(vec![vec![2., 0., 1.], vec![2., 1., 1.]]);
    s.weights.push(vec![1.; 2]);
    let r = inspect(&s, [[0.5, 0.5], [0.3, 0.7]], [0., 0., 1.], 1e-6, 1).unwrap();
    assert_eq!(r.aligned, None);
    assert_eq!(r.reason, "span-limit");
    assert!(r.normal_components.is_none());
    let r = inspect(&s, [[0.5, 0.5], [0.3, 0.7]], [0., 0., 1.], 1e-6, 2).unwrap();
    assert_ne!(r.aligned, Some(true));
    assert_eq!(r.spans, 2);
    for (d, t, n) in [
        ([0.; 3], 1e-6, 1),
        ([0., 0., 1.], 1., 1),
        ([0., 0., 1.], 1e-6, 0),
    ] {
        assert!(inspect(&s, [[0., 1.]; 2], d, t, n).is_err());
    }
}
#[test]
fn rational_original_derivatives_stay_enclosed_on_narrow_and_point_rectangles() {
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0.13, 0.13, 0.13, 3.17, 3.17, 3.17],
        knots_v: vec![-0.71, -0.71, -0.71, 0.29, 0.29, 0.29],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64, j as f64, (i * i + 2 * j * j + i * j) as f64 / 10.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., 0.7, 1.2], vec![0.9, 1.1, 0.8], vec![1.3, 0.6, 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let before = format!("{s:?}");
    for domain in [
        [[0.43, 0.53], [-0.23, -0.13]],
        [[0.43, 0.43_f64.next_up()], [-0.23, (-0.23_f64).next_up()]],
        [[0.43, 0.43], [-0.23, -0.23]],
    ] {
        let r = inspect(&s, domain, [0.3, -0.2, 1.], 1e-6, 1).unwrap();
        let bounds = r.normal_components.unwrap();
        let sine = r.sine_squared_interval.unwrap();
        for i in 0..=4 {
            for j in 0..=4 {
                let u = domain[0][0] + (domain[0][1] - domain[0][0]) * i as f64 / 4.;
                let v = domain[1][0] + (domain[1][1] - domain[1][0]) * j as f64 / 4.;
                let (a, b) = s.evaluate(u, v).unwrap().first_derivatives().unwrap();
                let n = std::array::from_fn::<_, 3, _>(|k| {
                    a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3]
                });
                for k in 0..3 {
                    assert!(n[k] >= bounds[k][0] - 1e-11 && n[k] <= bounds[k][1] + 1e-11);
                }
                let d = [0.3, -0.2, 1.];
                let cross = std::array::from_fn::<_, 3, _>(|k| {
                    n[(k + 1) % 3] * d[(k + 2) % 3] - n[(k + 2) % 3] * d[(k + 1) % 3]
                });
                let ratio = cross.iter().map(|x| x * x).sum::<f64>()
                    / (n.iter().map(|x| x * x).sum::<f64>()
                        * d.iter().map(|x| x * x).sum::<f64>());
                assert!(ratio >= sine[0] - 1e-11 && ratio <= sine[1] + 1e-11);
            }
        }
    }
    let direction = s.evaluate(0.43, -0.23).unwrap().unit_normal().unwrap();
    let r = inspect(&s, [[0.43, 0.43], [-0.23, -0.23]], direction, 1e-10, 1).unwrap();
    assert_eq!(r.aligned, Some(true));
    assert_eq!(format!("{s:?}"), before);
}
#[test]
fn general_spatial_plane_does_not_depend_on_a_cartesian_axis() {
    let mut s = plane();
    for row in &mut s.control_points {
        for p in row {
            let (u, v) = (p[0], p[1]);
            *p = vec![1024. + u, -2048. + 2. * u + v, 512. + 3. * v];
        }
    }
    let r = inspect(
        &s,
        [[0.31, 0.31_f64.next_up()], [0.67, 0.67_f64.next_up()]],
        [6., -3., 1.],
        1e-10,
        1,
    )
    .unwrap();
    assert_eq!(r.aligned, Some(true));
    let b = r.normal_components.unwrap();
    for (i, n) in [6., -3., 1.].into_iter().enumerate() {
        assert!(b[i][0] <= n && b[i][1] >= n);
    }
}
