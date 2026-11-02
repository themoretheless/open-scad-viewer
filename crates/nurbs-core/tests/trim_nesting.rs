use nurbs_core::{
    curve::Curve,
    trim_domain::Location,
    trim_region_audit::nesting::{self, Limits},
};
fn square(lo: f64, hi: f64, reverse: bool) -> Vec<Curve> {
    let mut p = vec![[lo, lo], [hi, lo], [hi, hi], [lo, hi]];
    if reverse {
        p.reverse();
    }
    (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
        .collect()
}
#[test]
fn unordered_deep_forest_selects_holes_islands_and_multiple_components() {
    let loops = vec![
        square(-3., 3., true),
        square(-5., 5., true),
        square(-2., 2., false),
        square(-4., 4., false),
        square(10., 12., false),
    ];
    let saved = loops.clone();
    let l = Limits::default();
    let d = nesting::build(&loops, 1e-8, l).unwrap();
    assert_eq!(
        d.nodes()
            .iter()
            .map(|n| (n.parent, n.depth))
            .collect::<Vec<_>>(),
        vec![
            (Some(3), 2),
            (None, 0),
            (Some(0), 3),
            (Some(1), 1),
            (None, 0)
        ]
    );
    assert_eq!(d.components().len(), 3);
    for (p, loc) in [
        ([4.5, 0.], Location::Inside),
        ([3.5, 0.], Location::Outside),
        ([2.5, 0.], Location::Inside),
        ([0., 0.], Location::Outside),
        ([11., 11.], Location::Inside),
        ([8., 0.], Location::Outside),
    ] {
        assert_eq!(d.classify(p.map(|x| [x; 2]), 10000).unwrap().location, loc);
    }
    assert!(
        d.proof().pairs <= l.pairs
            && d.proof().cells <= l.cells
            && d.proof().domain_cells <= l.domain_cells
    );
    assert_eq!(d.loops(), saved);
    d.reverify().unwrap();
    assert_eq!(loops, saved);
}
fn circle(r: f64) -> Vec<Curve> {
    let p = [
        [[r, 0.], [r, r], [0., r]],
        [[0., r], [-r, r], [-r, 0.]],
        [[-r, 0.], [-r, -r], [0., -r]],
        [[0., -r], [r, -r], [r, 0.]],
    ];
    p.map(|p| Curve {
        degree: 2,
        knots: vec![-3., -3., -3., 2., 2., 2.],
        control_points: p.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    })
    .to_vec()
}
#[test]
fn rational_nonunit_loop_orientation_uses_coedges_without_changing_definitions() {
    let loops = vec![circle(5.), circle(2.)];
    let d = nesting::build(&loops, 1e-8, Limits::default()).unwrap();
    assert!(!d.nodes()[0].reversed);
    assert!(d.nodes()[1].reversed);
    assert_eq!(
        d.coedges(1).unwrap(),
        vec![(3, true), (2, true), (1, true), (0, true)]
    );
    assert_eq!(d.components()[0].holes, vec![1]);
    assert_eq!(d.loops(), loops);
    assert_eq!(
        d.classify([[0.; 2], [0.; 2]], 10000).unwrap().location,
        Location::Outside
    );
    assert_eq!(
        d.classify([[3.; 2], [0.; 2]], 10000).unwrap().location,
        Location::Inside
    );
    d.reverify().unwrap();
}
#[test]
fn touching_crossing_or_duplicate_boundaries_never_publish_partial_depths() {
    for loops in [
        vec![square(-5., 5., false), square(-5., 2., true)],
        vec![square(-5., 0., false), square(-2., 3., false)],
        vec![square(-5., 5., false), square(-5., 5., false)],
    ] {
        let saved = loops.clone();
        let p = nesting::inspect(&loops, 1e-8, Limits::default()).unwrap();
        assert!(!p.proven && p.nodes.is_none());
        assert_eq!(loops, saved);
    }
}
#[test]
fn shared_budgets_keep_nesting_and_parity_unknown_and_preflight_later_inputs() {
    let loops = vec![square(-5., 5., false), square(-2., 2., false)];
    for kind in 0..3 {
        let mut l = Limits::default();
        match kind {
            0 => l.pairs = 1,
            1 => l.cells = 1,
            _ => l.domain_cells = 1,
        }
        let p = nesting::inspect(&loops, 1e-8, l).unwrap();
        assert!(!p.proven && p.nodes.is_none());
        assert!(p.pairs <= l.pairs && p.cells <= l.cells && p.domain_cells <= l.domain_cells);
    }
    let d = nesting::build(&loops, 1e-8, Limits::default()).unwrap();
    assert_eq!(
        d.classify([[1.9, 2.1], [0., 0.1]], 10000).unwrap().location,
        Location::Unresolved
    );
    assert_eq!(
        d.classify([[0.; 2], [0.; 2]], 1).unwrap().location,
        Location::Unresolved
    );
    let mut bad = loops;
    bad[1][0].control_points.iter_mut().for_each(|p| p.push(0.));
    let mut l = Limits::default();
    l.pairs = 1;
    assert!(nesting::inspect(&bad, 1e-8, l).is_err());
}
