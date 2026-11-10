use super::*;
#[test]
fn diagonal_split_covers_closed_product_up_to_endpoint_exchange() {
    let rect=[[0.,1.],[-2.,2.]];
    for axis in 0..2 {
        let mid=(rect[axis][0]+rect[axis][1])*0.5;
        let children=diagonal_children(rect,axis,mid);
        let points:Vec<_>=[0.,0.25,0.5,0.75,1.].into_iter().flat_map(|u|
            [-2.,-1.,0.,1.,2.].into_iter().map(move|v|[u,v])).collect();
        let contains=|r:Rectangle,p:[f64;2]| (0..2).all(|a|r[a][0]<=p[a] && p[a]<=r[a][1]);
        for &a in &points {for &b in &points {
            assert!(children.iter().any(|c|
                contains(c[0],a)&&contains(c[1],b) || contains(c[0],b)&&contains(c[1],a)));
        }}
        assert_eq!(children[0][0][axis][1],mid);
        assert_eq!(children[2][0][axis][0],mid);
    }
}
#[test]
fn budget_refusal_preserves_all_closed_pairs_including_exchanged_endpoints() {
    let mut s=graph();
    for row in &mut s.control_points {for point in row {point.fill(0.);}}
    let points:Vec<_>=[0.,0.25,0.5,0.75,1.].into_iter().flat_map(|u|
        [0.,0.25,0.5,0.75,1.].into_iter().map(move|v|[u,v])).collect();
    let contains=|r:Rectangle,p:[f64;2]| (0..2).all(|a|r[a][0]<=p[a] && p[a]<=r[a][1]);
    for cells in 1..=12 {
        let report=qualify(&s,0.1,1e-4,Limits{cells,spans:1000}).unwrap();
        assert!(report.certificate.is_none());
        assert_eq!(report.cells,cells);
        assert_eq!(report.pending,report.unresolved_pairs.len());
        assert_eq!(report.uncertain,report.unresolved_pairs.last().copied());
        for &a in &points {for &b in &points {
            assert!(report.unresolved_pairs.iter().any(|c|
                contains(c[0],a)&&contains(c[1],b) || contains(c[0],b)&&contains(c[1],a)),
                "Missing original endpoint obligation after {cells} cells: {a:?}, {b:?}");
        }}
    }
}
#[test]
fn unresolved_singular_corner_does_not_starve_other_original_pairs() {
    let mut s=graph();
    for row in &mut s.control_points {for point in row {point.fill(0.);}}
    let report=qualify(&s,0.1,1e-4,Limits{cells:64,spans:10000}).unwrap();
    assert!(report.certificate.is_none());
    assert_eq!(report.cells,64);
    assert_eq!(report.pending,report.unresolved_pairs.len());
    // A depth-first traversal spends all cells shrinking one corner;
    // level-order coverage keeps all unresolved parameter boxes useful.
    assert!(report.unresolved_pairs.iter().flatten().flatten()
        .all(|range| range[1]-range[0] >= 1./64.));
}
#[test]
fn rectangle_cache_reuses_only_complete_original_coverage() {
    let s=graph();
    let mut cache=RectangleCache::new(&s,1e-6);
    let domain=[[0.,1.];2];
    assert!(cache.qualify(domain,0).is_err());
    let (paid,reused)=cache.qualify(domain,1).unwrap();
    assert!(paid.complete && !reused && paid.spans==1);
    let (free,reused)=cache.qualify(domain,0).unwrap();
    assert!(free.complete && reused && free.spans==0);
    assert_eq!(paid.cosine_squared_upper,free.cosine_squared_upper);
    assert!(free.mean_matrix.is_some());
    assert!(cache.qualify([[0.,0.5],[0.,1.]],0).is_err());
    let mut multiple=s.clone();
    multiple.control_points.push(multiple.control_points[2].clone());
    multiple.weights.push(vec![1.;2]);
    multiple.knots_u=vec![0.,0.,0.,0.5,1.,1.,1.];
    let mut partial=RectangleCache::new(&multiple,1e-6);
    for _ in 0..2 {
        let (report,reused)=partial.qualify(domain,1).unwrap();
        assert!(!report.complete && !reused && report.spans==1);
    }
    let (report,reused)=partial.qualify(domain,2).unwrap();
    assert!(report.complete && !reused && report.spans==2);
    assert!(partial.qualify(domain,0).unwrap().1);
}
#[test]
fn original_normal_cache_retains_complete_rectangles_but_not_partial_pairs() {
    let s=graph();
    let pair=[[[0.,0.1],[0.,0.1]],[[0.8,0.9],[0.8,0.9]]];
    let mut cache=normal_alignment::OriginalNormalCache::new(&s);
    let (first,proof,hits)=cache.inspect_pair(pair,1e-6,1).unwrap();
    assert_eq!((first.spans,proof.is_some(),hits),(1,false,0));
    let (missing,proof,hits)=cache.inspect_pair(pair,1e-6,0).unwrap();
    assert_eq!((missing.spans,proof.is_some(),hits),(0,false,1));
    let (paid,proof,hits)=cache.inspect_pair(pair,1e-6,1).unwrap();
    assert_eq!((paid.spans,proof.is_some(),hits),(1,true,1));
    let (free,proof,hits)=cache.inspect_pair([pair[1],pair[0]],1e-6,0).unwrap();
    assert_eq!((free.spans,proof.is_some(),hits),(0,true,2));
    assert_eq!(paid.sine_squared_interval,free.sine_squared_interval);
    assert!(cache.inspect_pair([[[0.,1.01],[0.,1.]],pair[1]],1e-6,0).is_err());
    assert!(cache.inspect_pair(pair,f64::NAN,0).is_err());
    let foreign=s.clone();
    assert!(proof.unwrap().components(&foreign,[pair[1],pair[0]]).is_err());
    let (_,foreign_proof,hits)=normal_alignment::OriginalNormalCache::new(&foreign)
        .inspect_pair(pair,1e-6,0).unwrap();
    assert!(foreign_proof.is_none() && hits==0);
}
fn graph() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|v| vec![i as f64 / 2., v as f64, if i == 2 { 2. } else { 0. }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn factored_original_pole_normals_exclude_oblique_secants_but_refuse_foreign_source() {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.]; 2],
            vec![vec![0.5, 0., 0.]; 2],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let prepared = crate::factored_surface_normals::prepare(&s, false, 1_000_000).unwrap();
    let proof = prepared.certificate.unwrap();
    let pair = [[[0., 0.], [0., 1.]], [[0.5, 0.6], [0., 1.]]];
    let a = surface_distance::rectangle_bounds(&s, pair[0]).unwrap();
    let b = surface_distance::rectangle_bounds(&s, pair[1]).unwrap();
    assert!(
        !secant_exclusion(&s, pair, [&a, &b], 1e-6, 1, None)
            .unwrap()
            .0
    );
    let (excluded, used) = secant_exclusion(&s, pair, [&a, &b], 1e-6, 1, Some(&proof)).unwrap();
    assert!(excluded && used == 1);
    let cache = proof.pair_bounds(pair).unwrap();
    assert_eq!(secant_exclusion_cached(&s, pair, [&a, &b], 1e-6, 0,
        Some(&proof), Some(&cache), None).unwrap(), (true, 0));
    assert!(cache.components(&proof, [pair[0]; 2]).is_err());
    let other = s.clone();
    let other_proof = crate::factored_surface_normals::prepare(&other, false, 1_000_000)
        .unwrap().certificate.unwrap();
    assert!(cache.components(&other_proof, pair).is_err());
    let other_end = crate::factored_surface_normals::prepare(&s, true, 1_000_000)
        .unwrap();
    if let Some(other_end) = other_end.certificate {
        assert!(cache.components(&other_end, pair).is_err());
    }

    let foreign = s.clone();
    assert!(qualify_with_factored_normals(
        &foreign,
        0.1,
        1e-6,
        Limits {
            cells: 10,
            spans: 100
        },
        &proof
    )
    .is_err());
    let r = qualify_with_factored_normals(&s, 0.1, 1e-6, Limits { cells: 1, spans: 1 }, &proof)
        .unwrap();
    assert!(r.certificate.is_none() && r.pending > 0 && r.spans <= 1);
}
#[test]
fn original_secant_direction_excludes_oblique_chords_and_refuses_overlap_or_no_work() {
    let s = graph();
    let pair = [[[0., 0.01], [0., 0.01]], [[0.1, 0.11], [0., 0.01]]];
    let a = surface_distance::rectangle_bounds(&s, pair[0]).unwrap();
    let b = surface_distance::rectangle_bounds(&s, pair[1]).unwrap();
    let (excluded, used) = secant_exclusion(&s, pair, [&a, &b], 1e-6, 100, None).unwrap();
    assert!(excluded);
    assert!(used > 0 && used <= 100);
    let (paid,cache)=normal_alignment::inspect_original_pair_cached(&s,pair,1e-6,100).unwrap();
    assert!(paid.spans>0);
    let cache=cache.unwrap();
    assert_eq!(secant_exclusion_cached(&s,pair,[&a,&b],1e-6,0,None,None,
        Some(&cache)).unwrap(),(true,0));
    let foreign=s.clone();
    assert!(secant_exclusion_cached(&foreign,pair,[&a,&b],1e-6,0,None,None,
        Some(&cache)).is_err());
    assert!(secant_exclusion_cached(&s,[pair[0];2],[&a,&a],1e-6,0,None,None,
        Some(&cache)).is_err());

    assert_eq!(
        secant_exclusion(&s, pair, [&a, &b], 1e-6, 0, None).unwrap(),
        (false, 0)
    );
    assert_eq!(
        secant_exclusion(&s, [pair[0]; 2], [&a, &a], 1e-6, 100, None).unwrap(),
        (false, 0)
    );
}
#[test]
fn complete_subdivision_proves_strongly_curved_graph_and_missing_work_refuses() {
    let s = graph();
    assert!(surface_self_chord::qualify(&s, 1e-6, 1)
        .unwrap()
        .certificate
        .is_none());
    let r = qualify(
        &s,
        0.1,
        1e-6,
        Limits {
            cells: 10000,
            spans: 100000,
        },
    )
    .unwrap();
    eprintln!(
        "self adaptive {} cells={} spans={} intrinsic={} angular={} distance={} pending={}",
        r.reason,
        r.cells,
        r.spans,
        r.intrinsic_exclusions,
        r.angular_exclusions,
        r.distance_exclusions,
        r.pending
    );
    let c = r
        .certificate
        .expect("all original endpoint pairs must be covered");
    assert!(std::ptr::eq(c.surface(), &s));
    assert_eq!(c.minimum_mm(), 0.1);
    assert_eq!(c.max_sine_squared(), 1e-6);
    assert!(r.intrinsic_exclusions > 1);
    assert_eq!(r.pending, 0);
    assert!(r.uncertain.is_none());
    assert!(r.unresolved_pairs.is_empty());
    assert!(c
        .intrinsic_certificates()
        .iter()
        .all(|p| std::ptr::eq(p.surface(), &s)));
    for limits in [
        Limits {
            cells: 1,
            spans: 100000,
        },
        Limits {
            cells: 10000,
            spans: 1,
        },
    ] {
        let r = qualify(&s, 0.1, 1e-6, limits).unwrap();
        assert!(r.certificate.is_none());
        assert!(r.pending > 0);
        assert!(r.uncertain.is_some());
        assert_eq!(r.unresolved_pairs.len(), r.pending);
        assert_eq!(r.unresolved_pairs.last().copied(), r.uncertain);
        for pair in &r.unresolved_pairs {
            for rect in pair {
                for axis in 0..2 {
                    assert!(rect[axis][0] >= 0. && rect[axis][0] <= rect[axis][1]
                        && rect[axis][1] <= 1.);
                }
            }
        }
    }
}
#[test]
fn rational_half_cylinder_with_short_exact_normal_chord_cannot_qualify() {
    let points = [[1., 0.], [1., 1.], [0., 1.], [-1., 1.], [-1., 0.]];
    let w = 0.5_f64.sqrt();
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: points
            .iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
            .collect(),
        weights: [1., w, 1., w, 1.].iter().map(|w| vec![*w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    // Original endpoints at U=0/1 have distance exactly 2 and normals
    // parallel to their connecting line, independent of rounded arc weight.
    let pair = [[[0., 0.001], [0.4, 0.401]], [[0.999, 1.], [0.4, 0.401]]];
    let a = surface_distance::rectangle_bounds(&s, pair[0]).unwrap();
    let b = surface_distance::rectangle_bounds(&s, pair[1]).unwrap();
    let (excluded, used) = secant_exclusion(&s, pair, [&a, &b], 1e-6, 100, None).unwrap();
    assert!(!excluded, "the exact diameter must remain a candidate");
    assert!(used > 0 && used <= 100);
    let r = qualify(
        &s,
        2.1,
        1e-6,
        Limits {
            cells: 2000,
            spans: 10000,
        },
    )
    .unwrap();
    eprintln!(
        "short original normal chord refusal {} cells={} spans={} pending={}",
        r.reason, r.cells, r.spans, r.pending
    );
    assert!(r.certificate.is_none());
    assert!(r.pending > 0);
    assert!(r.uncertain.is_some());
    assert!(r.cells <= 2000 && r.spans <= 10000);
    assert!(qualify(
        &s,
        0.,
        1e-6,
        Limits {
            cells: 2000,
            spans: 10000
        }
    )
    .is_err());
}
