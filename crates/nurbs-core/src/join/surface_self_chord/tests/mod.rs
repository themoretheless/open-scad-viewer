use super::*;
#[test]
fn rational_coordinate_chords_enclose_independent_surface_evaluations() {
    let s=Surface{degree_u:2,degree_v:2,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,0.,1.,1.,1.],
        control_points:(0..3).map(|u|(0..3).map(|v|vec![u as f64,v as f64,(u*v) as f64]).collect()).collect(),
        weights:vec![vec![1.,0.7,1.],vec![0.9,0.63,0.9],vec![1.,0.7,1.]],periodic_u:false,periodic_v:false};
    let original=qualify_rectangle(&s,[[0.,1.];2],1e-6,1).unwrap().mean_matrix.unwrap();
    let initial=[[[0.1,0.3],[0.7,0.9]],[[0.7,0.9],[0.1,0.3]]];
    for pair in [initial,[initial[1],initial[0]]]{
        let u=qualify_rectangle(&s,[[0.1,0.9],pair[0][1]],1e-6,1).unwrap().mean_matrix.unwrap();
        let v=qualify_rectangle(&s,[pair[1][0],[0.1,0.9]],1e-6,1).unwrap().mean_matrix.unwrap();
        let bounds=original.coordinate_path(pair,&u,&v).unwrap().chord_bounds().unwrap();
        for i in 0..3 {for j in 0..3 {for k in 0..3 {for l in 0..3 {
            let t=|axis:[f64;2],n:usize|axis[0]+(axis[1]-axis[0])*n as f64/2.;
            let a=s.evaluate(t(pair[0][0],i),t(pair[0][1],j)).unwrap().point;
            let b=s.evaluate(t(pair[1][0],k),t(pair[1][1],l)).unwrap().point;
            for axis in 0..3 {let chord=b[axis]-a[axis];assert!(bounds[axis][0]<=chord&&chord<=bounds[axis][1]);}
        }}}}
    }
}
#[test]
fn coordinate_path_requires_same_source_and_complete_endpoint_strips() {
    let s=Surface{degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
    let pair=[[[0.,0.2],[0.,0.2]],[[0.8,1.],[0.8,1.]]];
    let full=qualify_rectangle(&s,[[0.,1.];2],1e-6,1).unwrap().mean_matrix.unwrap();
    let u=qualify_rectangle(&s,[[0.,1.],[0.,0.2]],1e-6,1).unwrap().mean_matrix.unwrap();
    let v=qualify_rectangle(&s,[[0.8,1.],[0.,1.]],1e-6,1).unwrap().mean_matrix.unwrap();
    let path=full.coordinate_path(pair,&u,&v).unwrap();
    assert_eq!(path.exclude_pair(pair,1e-6,1,None,None).unwrap(),(true,1));
    assert!(path.exclude_pair([pair[0],pair[0]],1e-6,1,None,None).is_err());
    let chord=path.chord_bounds().unwrap();
    for (axis,bound) in chord.iter().enumerate(){
        if axis<2 {assert!(bound[0]<=0.6&&bound[1]>=1.);assert!(bound[0]>0.59&&bound[1]<1.01);}
        else {assert!(bound[0]<=0.&&bound[1]>=0.);assert!(bound[0]>-1e-12&&bound[1]<1e-12);}
    }

    let short=qualify_rectangle(&s,[[0.,0.5],[0.,0.2]],1e-6,1).unwrap().mean_matrix.unwrap();
    assert!(full.coordinate_path(pair,&short,&v).is_err());
    let twin=s.clone();
    let foreign=qualify_rectangle(&twin,[[0.,1.];2],1e-6,1).unwrap().mean_matrix.unwrap();
    assert!(full.coordinate_path(pair,&foreign,&v).is_err());
    assert!(full.coordinate_path(pair,&u,&foreign).is_err());
    let mut invalid=pair;invalid[0][0]=[0.2,0.];assert!(full.coordinate_path(invalid,&u,&v).is_err());
    invalid[0][0]=[f64::NAN,0.2];assert!(full.coordinate_path(invalid,&u,&v).is_err());
}
#[test]
fn original_mean_matrix_excludes_planar_tangent_variation_with_bounded_normals() {
    let s=Surface { degree_u:2,degree_v:1,
        knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:(0..3).map(|u|(0..2).map(|v|
            vec![u as f64/2.,10.*v as f64+if u==2 {10.} else {0.},0.]).collect()).collect(),
        weights:vec![vec![1.;2];3],periodic_u:false,periodic_v:false };
    let r=qualify_rectangle(&s,[[0.,1.];2],1e-4,1).unwrap();
    assert!(r.complete && r.certificate.is_none());
    let matrix=r.mean_matrix.unwrap();
    let pair=[[[0.,0.1],[0.,0.1]],[[0.8,0.9],[0.8,0.9]]];
    let mut broad=qualify_rectangle(&s,[[0.,1.];2],1e-4,1).unwrap().mean_matrix.unwrap();
    // A conservative widening includes rank-deficient matrices while
    // every source chord still has separated X and zero Z components.
    broad.jacobian[1][1]=I::new(0.,20.).unwrap();
    assert!(!inspect_jacobian_rank_bound(broad.jacobian.map(|r|r.map(|d|[d.lo,d.hi]))).unwrap());
    assert_eq!(broad.exclude_pair(pair,1e-4,2,None,None).unwrap(),(true,1));
    assert_eq!(inspect_original_endpoint_pair(&s,pair,1e-4,0).unwrap(),(false,0));
    assert_eq!(inspect_original_endpoint_pair(&s,pair,1e-4,1).unwrap(),(false,1));
    assert_eq!(inspect_original_endpoint_pair(&s,pair,1e-4,2).unwrap(),(true,2));
    let mut workspace=OriginalChordWorkspace::new(&s,1e-4).unwrap();
    assert_eq!(workspace.inspect_pair(pair,2).unwrap(),(true,2));
    assert_eq!(workspace.inspect_pair(pair,1).unwrap(),(true,0),
        "Warm mean matrix and endpoint hull must not repay source normal work");
    assert_eq!(workspace.inspect_pair(pair,0).unwrap(),(false,0));
    assert!(workspace.cache_hits()>0);
    let mut seeded=OriginalChordWorkspace::new(&s,1e-4).unwrap();
    assert!(seeded.qualify_rectangle([[0.,0.9];2],1).unwrap().complete);
    assert_eq!(seeded.inspect_pair(pair,1).unwrap(),(true,1));
    assert!(seeded.cache_hits()>0);
    assert!(seeded.qualify_rectangle([[0.,0.9];2],0).is_err());
    assert!(seeded.qualify_rectangle([[0.,0.9];2],100001).is_err());
    assert!(seeded.qualify_rectangle([[-0.1,0.9];2],1).is_err());
    assert!(workspace.inspect_pair(pair,100001).is_err());
    assert!(workspace.inspect_pair([pair[0],[[0.,1.1],[0.,1.]]],0).is_err());
    let mut independent=OriginalChordWorkspace::new(&s,1e-4).unwrap();
    assert_eq!(independent.inspect_pair(pair,1).unwrap(),(false,1));
    assert!(OriginalChordWorkspace::new(&s,f64::NAN).is_err());
    let split=Surface{degree_u:1,degree_v:1,knots_u:vec![0.,0.,0.5,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:[0.,0.5,1.].iter().map(|u|vec![vec![*u,0.,0.],vec![*u,1.,0.]]).collect(),
        weights:vec![vec![1.;2];3],periodic_u:false,periodic_v:false};
    let mut partial=OriginalChordWorkspace::new(&split,1e-4).unwrap();
    assert!(!partial.qualify_rectangle([[0.,1.];2],1).unwrap().complete);
    assert!(partial.rectangles.is_empty());
    assert!(partial.qualify_rectangle([[0.,1.];2],2).unwrap().complete);
    assert_eq!(partial.qualify_rectangle([[0.,1.];2],1).unwrap().spans,0);


    assert!(inspect_original_endpoint_pair(&s,[pair[0],[[0.,1.1],[0.,1.]]],1e-4,0).is_err());
    assert_eq!(matrix.exclude_pair(pair,1e-4,0,None,None).unwrap(),(false,0));
    assert_eq!(matrix.exclude_pair(pair,1e-4,1,None,None).unwrap(),(true,1));
    let (paid,cache)=normal_alignment::inspect_original_pair_cached(&s,pair,1e-4,2).unwrap();
    assert_eq!(paid.spans,2);
    let cache=cache.unwrap();
    assert_eq!(matrix.exclude_pair(pair,1e-4,0,None,Some(&cache)).unwrap(),(true,0));
    assert!(cache.components(&s,[pair[0];2]).is_err());
    let foreign=s.clone();
    assert!(cache.components(&foreign,pair).is_err());
    let (_,incomplete_cache)=normal_alignment::inspect_original_pair_cached(&s,pair,1e-4,1).unwrap();
    assert!(incomplete_cache.is_none());
    let foreign_matrix=qualify_rectangle(&foreign,[[0.,1.];2],1e-4,1).unwrap().mean_matrix.unwrap();
    assert!(foreign_matrix.exclude_pair(pair,1e-4,0,None,Some(&cache)).is_err());

    assert!(matrix.exclude_pair([[[0.,1.01],[0.,1.]],pair[1]],1e-4,0,None,None).is_err());
    let mut collapsed=s.clone();
    for row in &mut collapsed.control_points {for point in row {point.fill(0.);}}
    let collapsed=qualify_rectangle(&collapsed,[[0.,1.];2],1e-4,1).unwrap();
    assert_eq!(collapsed.mean_matrix.unwrap().exclude_pair(pair,1e-4,2,None,None)
        .unwrap(),(false,2));
    let mut multiple=s.clone();multiple.degree_u=1;
    multiple.knots_u=vec![0.,0.,0.5,1.,1.];
    let incomplete=qualify_rectangle(&multiple,[[0.,1.];2],1e-4,1).unwrap();
    assert!(!incomplete.complete && incomplete.mean_matrix.is_none());
}
#[test]
fn direct_gram_lower_bound_covers_interval_matrix_chords_and_aligned_case() {
    let j = [[I::new(0.9,1.1).unwrap(),I::point(0.)],
        [I::point(0.),I::new(0.9,1.1).unwrap()], [I::point(0.);2]];
    let n = [I::point(0.3),I::point(0.),I::point(1.)];
    let upper = endpoint_matrix_cosine(j,n).unwrap();
    assert!(cosine_upper(j).unwrap() < 0.1);
    let large = direct_gram_lower([I::point(1e200);2], 0.).unwrap();
    assert!(large > 0.99e200 && large <= 1e200);
    let constant_scaled = [[I::point(1e100),I::point(0.)],
        [I::point(0.),I::point(1.)], [I::point(0.);2]];
    assert!(cosine_upper(constant_scaled).unwrap() < 1e-10);
    assert!(upper < 0.13); // Minor / largest-eigenvalue alone exceeds .18.
    for a in [0.9,1.,1.1] { for b in [0.9,1.,1.1] {
        for x in [-1.,0.,1.] { for y in [-1.,0.,1.] {
            if x==0. && y==0. { continue; }
            let cosine = (0.3*a*x)*(0.3*a*x)/(1.09*((a*x)*(a*x)+(b*y)*(b*y)));
            assert!(cosine <= upper);
        }}
    }}
    assert_eq!(endpoint_matrix_cosine(j,[I::point(1.),I::point(0.),I::point(0.)])
        .unwrap(),1.);
}
#[test]
fn unequal_gram_diagonals_enclose_sheared_independent_chords() {
    let j = [[I::point(1.), I::point(0.1)],
        [I::point(0.), I::point(0.5)], [I::point(0.), I::point(0.1)]];
    let n = [I::point(0.), I::point(0.4), I::point(1.)];
    let upper = endpoint_matrix_cosine(j, n).unwrap();
    assert!(upper < 0.31); // Gershgorin alone exceeds .328.
    for x in [-2., -1., -0.1, 0., 1., 2.] {
        for y in [-2., -1., 0., 1., 2.] {
            if x == 0. && y == 0. { continue; }
            let length_squared = (x+0.1*y)*(x+0.1*y)+0.26*y*y;
            let cosine_squared = (0.3*y)*(0.3*y)/(1.16*length_squared);
            assert!(cosine_squared <= upper);
        }
    }
}
#[test]
fn endpoint_gram_eigenvalue_bound_encloses_independent_chord_directions() {
    let j = [[I::point(1.), I::point(0.)],
        [I::point(0.), I::point(1.)], [I::point(0.), I::point(0.2)]];
    let n = [I::point(0.), I::point(0.4), I::point(1.)];
    let upper = endpoint_matrix_cosine(j, n).unwrap();
    assert!(upper < 0.32); // The trace-only estimate exceeds 0.60.
    for x in [-2., -1., 0., 1., 2.] {
        for y in [-2., -1., 0., 1., 2.] {
            if x == 0. && y == 0. { continue; }
            let length_squared = x*x + 1.04*y*y;
            let cosine_squared = (0.6*y)*(0.6*y)/(1.16*length_squared);
            assert!(cosine_squared <= upper);
        }
    }
}
#[test]
fn endpoint_normal_matrix_preserves_tangent_variation_and_refuses_aligned_chords() {
    let zero = I::point(0.);
    let wide = I::new(1., 10.).unwrap();
    let j = [[wide, zero], [zero, wide], [zero, zero]];
    assert_eq!(cosine_upper(j).unwrap(), 1.);
    assert!(endpoint_matrix_cosine(j, [zero, zero, I::point(1.)]).unwrap() < 1e-20);
    let exact = [[I::point(1.), zero], [zero, I::point(1.)], [zero, zero]];
    assert_eq!(
        endpoint_matrix_cosine(exact, [I::point(1.), zero, zero]).unwrap(),
        1.
    );
    assert_eq!(endpoint_matrix_cosine(exact, [zero; 3]).unwrap(), 1.);
    assert_eq!(
        endpoint_matrix_cosine([[zero; 2]; 3], [zero, zero, I::point(1.)]).unwrap(),
        1.
    );
}
#[test]
fn combined_minors_strengthen_oblique_differential_lower_bound() {
    let low = I::new(-0.1, 0.1).unwrap();
    let high = I::new(0.9, 1.1).unwrap();
    let j = [[high, low], [high, high], [low, high]];
    // Independently computed single-minor best is at least
    // 0.24 * 2.44 / 0.8^2 = 0.915. All three minors jointly
    // give an upper cosine bound below 0.8 for the same enclosure.
    assert!(cosine_upper(j).unwrap() < 0.8);
    assert!(0.24 * 2.44 / 0.8_f64.powi(2) > 0.9);
}
#[test]
fn parameter_units_preserve_curved_chord_bounds_and_rejections() {
    let mut old_bound_refused = false;
    for height in [0.01, 2.] {
        let original = graph(height);
        for scales in [
            [1048576., 0.00000095367431640625],
            [0.00000095367431640625, 1048576.],
        ] {
            let mut s = original.clone();
            for x in &mut s.knots_u {
                *x *= scales[0];
            }
            for x in &mut s.knots_v {
                *x *= scales[1];
            }
            if height == 0.01 {
                let derivative = normal_alignment::jacobian_on(
                    &s,
                    [s.degree_u, s.degree_v],
                    [[0., scales[0]], [0., scales[1]]],
                )
                .unwrap();
                old_bound_refused |= cosine_upper(derivative).unwrap() >= 1.;
            }
            let r = qualify(&s, 1e-6, 1).unwrap();
            if height == 2. {
                assert!(r.certificate.is_none());
                continue;
            }
            let proof = r
                .certificate
                .expect("Affine parameter units must not hide an intrinsic exclusion");
            assert!(std::ptr::eq(proof.surface(), &s));
            for a in 0..16 {
                for b in 0..16 {
                    if a == b {
                        continue;
                    }
                    let u = a as f64 / 15.;
                    let v = b as f64 / 15.;
                    let dx = v - u;
                    let dz = 0.01 * (v * v - u * u);
                    let nx = -0.02 * u;
                    let cosine2 =
                        (nx * dx + dz).powi(2) / ((nx * nx + 1.) * (dx * dx + dz * dz));
                    assert!(cosine2 <= proof.cosine_squared_upper());
                }
            }
        }
    }
    assert!(
        old_bound_refused,
        "Regression must exercise a previously inconclusive parameter scaling"
    );
}
fn graph(height: f64) -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|v| vec![i as f64 / 2., v as f64, if i == 2 { height } else { 0. }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn original_endpoint_pair_matches_independent_parabola_chords() {
    let s=graph(0.01);
    let pair=[[[0.,0.2],[0.,0.2]],[[0.8,1.],[0.8,1.]]];
    assert!(inspect_original_endpoint_pair(&s,pair,1e-4,3).unwrap().0);
    for u in [0.,0.1,0.2] {
        for v in [0.8,0.9,1.] {
            for dy in [0.6,0.8,1.] {
                let dx:f64=v-u;
                let dz:f64=0.01*(v*v-u*u);
                for endpoint in [u,v] {
                    let nx:f64=-0.02*endpoint;
                    let cosine_squared=(nx*dx+dz).powi(2)
                        / ((nx*nx+1.)*(dx*dx+dy*dy+dz*dz));
                    assert!(cosine_squared<1.-1e-4);
                }
            }
        }
    }
    assert!(inspect_original_endpoint_pair(&s,pair,1.,0).is_err());
    assert!(inspect_original_endpoint_pair(&s,pair,1e-4,100001).is_err());
    let mut invalid=pair;
    invalid[0][0]=[f64::NAN,0.2];
    assert!(inspect_original_endpoint_pair(&s,invalid,1e-4,0).is_err());
}
#[test]
fn original_cache_keeps_angular_policy_and_rectangle_authority() {
    let s=graph(0.01);
    let domain=[[0.,1.];2];
    let mut strict=OriginalChordWorkspace::new(&s,1e-6).unwrap();
    assert!(strict.qualify_rectangle(domain,1).unwrap().certificate.is_some());
    let warmed=strict.qualify_rectangle(domain,1).unwrap();
    assert_eq!(warmed.spans,0);
    assert_eq!(warmed.certificate.unwrap().max_sine_squared(),1e-6);
    // A much wider normal cone includes chords excluded by the strict
    // policy. Identical source and UV coordinates cannot transfer that
    // exclusion across policies, even after both caches are warm.
    let mut wide=OriginalChordWorkspace::new(&s,0.999999).unwrap();
    for _ in 0..2 {
        let r=wide.qualify_rectangle(domain,1).unwrap();
        assert!(r.complete && r.certificate.is_none());
        assert_eq!(r.cosine_squared_upper,
            qualify_rectangle(&s,domain,0.999999,1).unwrap().cosine_squared_upper);
    }
    // Warm keys do not authorize degenerate, nonfinite or foreign UV
    // rectangles, including refusals requested with a zero budget.
    for bad in [[[0.,0.],[0.,1.]],[[0.,f64::NAN],[0.,1.]],
        [[0.,1.],[0.,f64::INFINITY]],[[-0.01,1.],[0.,1.]]] {
        for budget in [0,1] {assert!(strict.qualify_rectangle(bad,budget).is_err());}
    }
    assert!(strict.qualify_rectangle([[0.,0.5];2],1).unwrap().complete);
    assert_eq!(strict.qualify_rectangle([[0.,0.5];2],1).unwrap().spans,0);
    assert!(strict.qualify_rectangle(domain,0).is_err());
}
#[test]
fn original_two_coordinate_paths_preserve_exact_normal_diameter() {
    let s=Surface {
        degree_u:1,degree_v:3,knots_u:vec![0.,0.,1.,1.],
        knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],
        control_points:[0.,1.].into_iter().map(|x|
            [[1.,0.],[1.,2.],[-1.,2.],[-1.,0.]].into_iter().map(|p|vec![x,p[0],p[1]]).collect()).collect(),
        weights:vec![vec![1.,1./3.,1./3.,1.];2],periodic_u:false,periodic_v:false,
    };
    let pair=[[[0.4,0.41],[0.,0.001]],[[0.4,0.41],[0.999,1.]]];
    // Equal UV domains and degrees on a different source cannot borrow
    // the plane's exclusion: this rational arch has a normal diameter.
    let mut plane=s.clone();
    for row in &mut plane.control_points {for (i,point) in row.iter_mut().enumerate() {
        point[1]=i as f64/3.;point[2]=0.;
    }}
    plane.weights=vec![vec![1.;4];2];
    let mut planar=OriginalChordWorkspace::new(&plane,1e-4).unwrap();
    assert!(planar.inspect_pair(pair,1000).unwrap().0);
    assert!(planar.inspect_pair(pair,1000).unwrap().0);
    let mut workspace=OriginalChordWorkspace::new(&s,1e-4).unwrap();
    let first=workspace.inspect_pair(pair,1000).unwrap();
    assert!(!first.0);
    let direct=inspect_original_endpoint_pair(&s,pair,1e-4,1000).unwrap();
    assert_eq!(first.0,direct.0);
    assert!(first.1<direct.1,
        "Even a cold workspace shares endpoint hulls between the two exclusion stages");
    assert_eq!(workspace.normals.len(),2,"Both original endpoint normals are completely enclosed");
    let second=workspace.inspect_pair(pair,1000).unwrap();
    assert!(!second.0 && second.1<first.1,"Warm source-bound normals save actual span work");
    // Hold all rectangle certificates warm and remove only the normals:
    // the extra work must come from the two original endpoint enclosures.
    workspace.normals.clear();
    let rectangle_warm=workspace.inspect_pair(pair,1000).unwrap();
    assert_eq!(rectangle_warm.0,second.0);
    assert_eq!(rectangle_warm.1,second.1+2);
    eprintln!("original normal cache cold={} rectangleWarm={} fullyWarm={}",first.1,rectangle_warm.1,second.1);
    assert_eq!(workspace.inspect_pair(pair,0).unwrap(),(false,0));
    for endpoints in [pair,[pair[1],pair[0]]] {
        for budget in [0,1,4,16,64,1000] {
            let (cached,paid)=workspace.inspect_pair(endpoints,budget).unwrap();
            assert!(!cached&&paid<=budget,"Warm original cache must retain an actual normal diameter");
            let (excluded,spent)=inspect_original_endpoint_pair(&s,endpoints,1e-4,budget).unwrap();
            assert!(!excluded && spent<=budget,
                "original normal diameter must survive two complete or partial path bounds");
        }
    }
}
#[test]
fn original_coordinate_path_preserves_collapsed_boundary_and_work_limits() {
    let s=Surface {
        degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,0.,0.]],
            vec![vec![0.,1.,0.],vec![1.,0.,0.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
    };
    let pair=[[[0.,0.1],[0.,0.1]],[[0.9,1.],[0.9,1.]]];
    assert!(qualify_rectangle(&s,[[0.,1.];2],1e-4,1).unwrap().certificate.is_none());
    for budget in 0..=3 {
        let (excluded,spent)=inspect_original_endpoint_pair(&s,pair,1e-4,budget).unwrap();
        assert!(!excluded);
        assert!(spent<=budget);
    }
    // XY chords cannot align with the regular B normal (0,0,-u).
    assert!(inspect_original_endpoint_pair(&s,pair,1e-4,5).unwrap().0);
}
#[test]
fn whole_curved_graph_excludes_aligned_chords_and_retains_source_identity() {
    let s = graph(0.01);
    let r = qualify(&s, 1e-6, 1).unwrap();
    assert!(r.complete);
    assert_eq!(r.spans, 1);
    let c = r.certificate.expect("small curvature full chart proof");
    assert!(std::ptr::eq(c.surface(), &s));
    assert_eq!(c.max_sine_squared(), 1e-6);
    assert!(c.cosine_squared_upper() < 0.001);
    let strong = graph(2.);
    assert!(qualify(&strong, 1e-6, 1).unwrap().certificate.is_none());
    let mut collapsed = s.clone();
    for row in &mut collapsed.control_points {
        for p in row {
            p[1] = 0.
        }
    }
    assert!(qualify(&collapsed, 1e-6, 1).unwrap().certificate.is_none());
    assert!(qualify(&s, 1., 1).is_err());
    assert!(qualify(&s, 1e-6, 0).is_err());
}
#[test]
fn missing_knot_span_or_discontinuous_chart_never_excludes() {
    let mut s = graph(0.01);
    s.degree_u = 1;
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    let r = qualify(&s, 1e-6, 1).unwrap();
    assert!(!r.complete);
    assert_eq!(r.spans, 1);
    assert!(r.certificate.is_none());
    assert!(qualify(&s, 1e-6, 2).unwrap().certificate.is_some());
    s.control_points.insert(2, s.control_points[1].clone());
    s.weights.insert(2, vec![1.; 2]);
    s.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
    assert!(qualify(&s, 1e-6, 2).is_err());
}
#[test]
fn independent_parabola_chords_and_reversed_coordinates_stay_inside_bound() {
    let mut s = graph(0.01);
    for sign in [1., -1.] {
        for row in &mut s.control_points {
            for p in row {
                p[0] = sign * p[0].abs();
            }
        }
        let proof = qualify(&s, 1e-6, 1).unwrap().certificate.unwrap();
        for a in 0..16 {
            for b in 0..16 {
                if a == b {
                    continue;
                }
                let u = a as f64 / 15.;
                let v = b as f64 / 15.;
                // Independent analytic S(u,t)=(sign*u,t,0.01*u^2).
                let dx = sign * (v - u);
                let dz = 0.01 * (v * v - u * u);
                let nx = -sign * 0.02 * u;
                let cosine2 = (nx * dx + dz).powi(2) / ((nx * nx + 1.) * (dx * dx + dz * dz));
                assert!(cosine2 <= proof.cosine_squared_upper());
            }
        }
    }
}
#[test]
fn original_rational_weights_have_continuous_exclusion_with_analytic_oracle() {
    let mut s = graph(0.01);
    let w = 1.001;
    s.weights[2] = vec![w; 2];
    let proof = qualify(&s, 1e-6, 1)
        .unwrap()
        .certificate
        .expect("mild rational graph");
    let point = |u: f64| {
        let d = 1. + (w - 1.) * u * u;
        [(u + (w - 1.) * u * u) / d, w * 0.01 * u * u / d]
    };
    for a in 0..16 {
        for b in 0..16 {
            if a == b {
                continue;
            }
            let u = a as f64 / 15.;
            let v = b as f64 / 15.;
            let d = 1. + (w - 1.) * u * u;
            let dxdu = ((1. + 2. * (w - 1.) * u) * d
                - (u + (w - 1.) * u * u) * 2. * (w - 1.) * u)
                / (d * d);
            let dzdu = 2. * w * 0.01 * u / (d * d);
            let pa = point(u);
            let pb = point(v);
            let dx = pb[0] - pa[0];
            let dz = pb[1] - pa[1];
            let cosine2 = (-dzdu * dx + dxdu * dz).powi(2)
                / ((dxdu * dxdu + dzdu * dzdu) * (dx * dx + dz * dz));
            assert!(cosine2 <= proof.cosine_squared_upper());
        }
    }
}
#[test]
fn local_certificate_retains_its_rectangle_and_does_not_prove_full_chart() {
    let s = graph(2.);
    let domain = [[0.2, 0.21], [0.3, 0.8]];
    assert!(qualify(&s, 1e-6, 1).unwrap().certificate.is_none());
    let c = qualify_rectangle(&s, domain, 1e-6, 1)
        .unwrap()
        .certificate
        .unwrap();
    assert_eq!(c.domain(), domain);
    assert!(std::ptr::eq(c.surface(), &s));
    assert!(c.cosine_squared_upper() < 0.01);
    assert!(qualify_rectangle(&s, [[0.2, 0.2], [0., 1.]], 1e-6, 1).is_err());
    assert!(qualify_rectangle(&s, [[-0.1, 0.2], [0., 1.]], 1e-6, 1).is_err());
}
#[test]
fn local_rectangle_spends_only_intersecting_original_spans() {
    let mut s = graph(0.01);
    s.degree_u = 1;
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    let domain = [[0.6, 0.8], [0., 1.]];
    let r = qualify_rectangle(&s, domain, 1e-6, 1).unwrap();
    assert!(r.complete);
    assert_eq!(r.spans, 1);
    assert!(r.certificate.is_some());
    let r = qualify_rectangle(&s, [[0.4, 0.6], [0., 1.]], 1e-6, 1).unwrap();
    assert!(!r.complete);
    assert!(r.certificate.is_none());
}
