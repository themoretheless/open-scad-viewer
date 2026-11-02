use nurbs_core::{
    curve::Curve,
    original_graph_transition::{
        self as graph, Budget, CurvatureLawDecision, SourceContactCurvatureLaw,
    },
    surface::Surface,
};
fn inputs(end: bool) -> ([Surface; 2], Curve) {
    let degree = if end { 4 } else { 1 };
    let count = degree + 1;
    let s = std::array::from_fn(|side| Surface {
        degree_u: degree,
        degree_v: 1,
        knots_u: [vec![0.; count], vec![1.; count]].concat(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..count)
            .map(|i| {
                let h = if !end || i == degree { 1. / 1024. } else { 0. };
                vec![
                    vec![i as f64 / degree as f64, 0., 0.],
                    vec![
                        i as f64 / degree as f64,
                        if side == 0 { -1. } else { 1. },
                        -h,
                    ],
                ]
            })
            .collect(),
        weights: vec![vec![1.; 2]; count],
        periodic_u: false,
        periodic_v: false,
    });
    let w = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![if end { 0.25 } else { 1. / 1024. }, 0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    };
    (s, w)
}
#[test]
fn one_source_owned_law_accepts_rebuilt_profile_and_rejects_old_quartic() {
    let (s, w) = inputs(false);
    let mut b = Budget::new(1_000_000).unwrap();
    let law = SourceContactCurvatureLaw::new(&s[0], &w, &mut b).unwrap();
    let old = graph::build([&s[0], &s[1]], &w, &mut b).unwrap();
    let new = graph::build_quadratic_core_g2([&s[0], &s[1]], &w, &mut b).unwrap();
    let domain = [[0., 1.], [0.25, 0.75]];
    let a = old
        .inspect_source_curvature_law(&law, 0.001, 0.001, 0.001, domain, &mut b)
        .unwrap();
    assert_eq!(a.decision, CurvatureLawDecision::Exceeds);
    let z = new
        .inspect_source_curvature_law(&law, 0.001, 0.001, 0.001, domain, &mut b)
        .unwrap();
    assert_eq!(z.decision, CurvatureLawDecision::Within);
    assert!(a.radial_distance_error_upper_mm.unwrap() < 0.001); // position alone would accept the wrong curvature
    assert!(
        z.error_inverse_mm.unwrap()[1] <= 0.001
            && z.radial_distance_error_upper_mm.unwrap() <= 0.001
    );
    let strict = new
        .inspect_source_curvature_law(&law, 0.001, 0., 0.001, domain, &mut b)
        .unwrap();
    assert_ne!(strict.decision, CurvatureLawDecision::Within); // no exact-circle claim
    let strict_normal = new
        .inspect_source_curvature_law(&law, 0.001, 0.001, 0., domain, &mut b)
        .unwrap();
    assert_ne!(strict_normal.decision, CurvatureLawDecision::Within); // normal constraint is not ignored
}

#[test]
fn infinite_radius_flat_end_is_qualified_without_division_by_zero() {
    let (s, w) = inputs(true);
    let mut b = Budget::new(1_000_000).unwrap();
    let law = SourceContactCurvatureLaw::new(&s[0], &w, &mut b).unwrap();
    let new = graph::build_quadratic_core_g2([&s[0], &s[1]], &w, &mut b).unwrap();
    let r = new
        .inspect_source_curvature_law(&law, 0.001, 0.001, 0.001, [[0., 1.], [0.25, 0.75]], &mut b)
        .unwrap();
    println!(
        "flat-end source law: {:?}, curvature={:?}, error={:?}, radial={:?}",
        r.decision,
        r.source_inverse_radius_range,
        r.error_inverse_mm,
        r.radial_distance_error_upper_mm
    );
    assert_eq!(r.decision, CurvatureLawDecision::Within);
    assert_eq!(r.source_inverse_radius_range.unwrap()[0], 0.);
}
#[test]
fn stale_reference_and_incomplete_work_never_acquire_law_admission() {
    let (s, w) = inputs(false);
    let mut b = Budget::new(1_000_000).unwrap();
    let law = SourceContactCurvatureLaw::new(&s[0], &w, &mut b).unwrap();
    let new = graph::build_quadratic_core_g2([&s[0], &s[1]], &w, &mut b).unwrap();
    let r = new
        .inspect_source_curvature_law(
            &law,
            0.001,
            0.001,
            0.001,
            [[0., 1.], [0.25, 0.75]],
            &mut Budget::new(0).unwrap(),
        )
        .unwrap();
    assert_eq!(r.decision, CurvatureLawDecision::Unresolved);
    let mut changed = w.clone();
    changed.control_points[0][0] += f64::EPSILON;
    let wrong = SourceContactCurvatureLaw::new(&s[0], &changed, &mut b).unwrap();
    assert!(!wrong.matches(&new));
    assert!(
        new.inspect_source_curvature_law(
            &wrong,
            0.001,
            0.001,
            0.001,
            [[0., 1.], [0.25, 0.75]],
            &mut b
        )
        .is_err()
    );
}

#[test]
fn complete_patch_law_retains_seams_and_every_unresolved_leaf() {
    let (s, w) = inputs(false);
    let mut b = Budget::new(2_000_000).unwrap();
    let law = SourceContactCurvatureLaw::new(&s[0], &w, &mut b).unwrap();
    let patch = graph::build_quadratic_core_g2([&s[0], &s[1]], &w, &mut b).unwrap();
    let coverage = patch.inspect_complete_source_curvature_law(
        &law, 0.001, 0.001, 0.001, 64, &mut b,
    ).unwrap();
    assert_ne!(coverage.decision(), CurvatureLawDecision::Within);
    assert!(coverage.cells.len() <= 64);
    let area: f64 = coverage.cells.iter().map(|r| {
        let d = r.normalized_domain;
        (d[0][1]-d[0][0])*(d[1][1]-d[1][0])
    }).sum();
    assert_eq!(area, 1.);
    assert!(coverage.cells.iter().any(|r| r.normalized_domain[1][0] == 0.));
    assert!(coverage.cells.iter().any(|r| r.normalized_domain[1][1] == 1.));
    let empty = patch.inspect_complete_source_curvature_law(
        &law, 0.001, 0.001, 0.001, 64, &mut Budget::new(0).unwrap(),
    ).unwrap();
    assert_eq!(empty.decision(), CurvatureLawDecision::Unresolved);
    assert_eq!(empty.cells.len(), 1);
    assert_eq!(empty.cells[0].normalized_domain, [[0.,1.],[0.,1.]]);
    assert!(patch.inspect_complete_source_curvature_law(
        &law, 0.001, 0.001, 0.001, 0, &mut b,
    ).is_err());
    println!("complete patch: {:?}, leaves={}, exceeded={}, unresolved={}", coverage.decision(), coverage.cells.len(), coverage.cells.iter().filter(|r|r.decision==CurvatureLawDecision::Exceeds).count(), coverage.cells.iter().filter(|r|r.decision==CurvatureLawDecision::Unresolved).count());
}

#[test]
fn g2_source_seams_refuse_incompatible_positive_inverse_radius() {
    let (s, w) = inputs(false);
    let mut b = Budget::new(1_000_000).unwrap();
    let law = SourceContactCurvatureLaw::new(&s[0], &w, &mut b).unwrap();
    let patch = graph::build_quadratic_core_g2([&s[0], &s[1]], &w, &mut b).unwrap();
    let seams = patch.inspect_source_curvature_seams(&law, 0.001, 0.001, 0.001, &mut b).unwrap();
    for (i,r) in seams.iter().enumerate() {
        assert_eq!(r.normalized_domain, [[0.,1.],[i as f64,i as f64]]);
        assert_eq!(r.decision, CurvatureLawDecision::Exceeds);
        assert!(r.error_inverse_mm.unwrap()[0] > 0.99);
    }
    // The ordinary area API still refuses zero-area domains.
    assert!(patch.inspect_source_curvature_law(&law,0.001,0.001,0.001,
        [[0.,1.],[0.,0.]], &mut b).is_err());
    let limited = patch.inspect_source_curvature_seams(&law,0.001,0.001,0.001,
        &mut Budget::new(0).unwrap()).unwrap();
    assert!(limited.iter().all(|r|r.decision==CurvatureLawDecision::Unresolved));
}
