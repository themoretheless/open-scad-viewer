use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
fn square(lo: f64, hi: f64) -> Vec<Curve> {
    vec![
        Curve::from_polyline(vec![
            vec![lo, lo],
            vec![hi, lo],
            vec![hi, hi],
            vec![lo, hi],
            vec![lo, lo],
        ])
        .unwrap(),
    ]
}
fn arch(weights: Vec<f64>) -> Vec<Curve> {
    vec![
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![-2., 0.], vec![0., 4.], vec![2., 0.]],
            weights,
            periodic: false,
        },
        Curve::from_polyline(vec![vec![2., 0.], vec![-2., 0.]]).unwrap(),
    ]
}
fn check(loops: &[Vec<Curve>], q: [f64; 2]) -> p::VerifiedPointWinding {
    let c = p::certify_point_winding(loops, q).unwrap();
    p::verify_point_winding(loops, q, &c).unwrap()
}
#[test]
fn exact_original_nested_windings_keep_signed_multiplicity() {
    let outer = square(0., 4.);
    let inner = square(1., 3.);
    let inner = inner.iter().rev().map(|c| c.reverse().unwrap()).collect();
    let loops = vec![outer, inner];
    let r = check(&loops, [2., 2.]);
    assert_eq!(r.windings(), [1, -1]);
    assert_eq!(r.weighted_winding(&[1, 1]).unwrap(), 0);
    assert_eq!(r.weighted_winding(&[2, 1]).unwrap(), 1);
    assert_eq!(r.weighted_winding(&[-2, -1]).unwrap(), -1);
    assert_eq!(check(&loops, [0.5, 2.]).windings(), [1, 0]);
    assert_eq!(check(&loops, [5., 2.]).windings(), [0, 0]);
    assert!(r.weighted_winding(&[1]).is_err());
    assert!(r.weighted_winding(&[1_000_001, 1]).is_err());
}
#[test]
fn irrational_ray_roots_use_exact_side_intervals_and_complete_replay() {
    let loops = vec![arch(vec![1.; 3])];
    let c = p::certify_point_winding(&loops, [0., 1.]).unwrap();
    let r = p::verify_point_winding(&loops, [0., 1.], &c).unwrap();
    assert_eq!(r.windings(), [-1]);
    assert_eq!(r.source_cells(), 2);
    assert_eq!(r.root_events(), 2);
    assert!(
        c.cells[0]
            .roots
            .iter()
            .all(|r| r.interval[0] != r.interval[1])
    );
    assert_eq!(
        c.cells[0]
            .roots
            .iter()
            .map(|r| r.sign_x)
            .collect::<Vec<_>>(),
        [-1, 1]
    );
    let mut bad = c.clone();
    bad.cells[0].roots.pop();
    assert!(p::verify_point_winding(&loops, [0., 1.], &bad).is_err());
    let mut bad = c.clone();
    bad.cells[0].roots[1].sign_x = -1;
    assert!(p::verify_point_winding(&loops, [0., 1.], &bad).is_err());
    let mut bad = c.clone();
    bad.cells[0].roots[1].jump = 1;
    assert!(p::verify_point_winding(&loops, [0., 1.], &bad).is_err());
    let mut bad = c.clone();
    bad.cells[0].roots.reverse();
    assert!(p::verify_point_winding(&loops, [0., 1.], &bad).is_err());
    let mut bad = c.clone();
    bad.windings[0] = 0;
    assert!(p::verify_point_winding(&loops, [0., 1.], &bad).is_err());
    let mut bad = c.clone();
    bad.cells.pop();
    assert!(p::verify_point_winding(&loops, [0., 1.], &bad).is_err());
    assert!(p::verify_point_winding(&loops, [0., 1.5], &c).is_err());
    let mut changed = loops.clone();
    changed[0][0].control_points[1][1] = 4_f64.next_up();
    assert!(p::verify_point_winding(&changed, [0., 1.], &c).is_err());
}
#[test]
fn even_tangency_and_stationary_odd_germs_are_not_sampled_away() {
    let loops = vec![arch(vec![1.; 3])];
    let c = p::certify_point_winding(&loops, [-3., 2.]).unwrap();
    assert_eq!(c.cells[0].roots.len(), 1);
    assert_eq!(c.cells[0].roots[0].jump, 0);
    assert_eq!(
        p::verify_point_winding(&loops, [-3., 2.], &c)
            .unwrap()
            .windings(),
        [0]
    );
    assert!(p::certify_point_winding(&loops, [0., 2.]).is_err());
    let cubic = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![vec![-3., -1.], vec![-1., 1.], vec![1., -1.], vec![3., 1.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let close = Curve::from_polyline(vec![
        vec![3., 1.],
        vec![4., 1.],
        vec![4., -1.],
        vec![-3., -1.],
    ])
    .unwrap();
    let loops = vec![vec![cubic, close]];
    let c = p::certify_point_winding(&loops, [-4., 0.]).unwrap();
    assert_eq!(c.cells[0].roots[0].jump, 1);
    assert_eq!(
        p::verify_point_winding(&loops, [-4., 0.], &c)
            .unwrap()
            .windings(),
        [0]
    );
    assert_eq!(check(&loops, [3.5, 0.]).windings(), [-1]);
}
#[test]
fn rational_multispan_sources_preserve_every_original_knot_clock() {
    let original = arch(vec![1., 2., 1.]);
    let mut wire = original.clone();
    wire[0] = wire[0].insert(0.5, 1).unwrap();
    for c in &mut wire {
        for u in &mut c.knots {
            *u = 2. + 4. * *u;
        }
    }
    let before = wire.clone();
    let loops = vec![wire];
    assert_eq!(check(&loops, [0., 1.]).windings(), [-1]);
    assert_eq!(check(&loops, [0., 3.]).windings(), [0]);
    assert_eq!(check(&[original], [0., 1.]).windings(), [-1]);
    assert_eq!(loops[0], before);
    assert_eq!(check(&loops, [0., 1.]).source_cells(), 3);
}
#[test]
fn half_open_join_rules_and_boundary_refusals_cover_horizontal_spans() {
    let loops = vec![square(0., 2.)];
    for q in [[-1., 0.], [-1., 2.], [3., 0.], [3., 2.]] {
        assert_eq!(check(&loops, q).windings(), [0]);
    }
    for q in [[0., 0.], [2., 2.], [1., 0.], [2., 1.], [0., 1.]] {
        assert!(p::certify_point_winding(&loops, q).is_err());
    }
    assert!(p::certify_point_winding(&loops, [f64::NAN, 0.]).is_err());
    assert!(
        p::certify_point_winding(
            &[vec![
                Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap()
            ]],
            [2., 0.]
        )
        .is_err()
    );
    assert!(check(&[], [0., 0.]).windings().is_empty());
}

#[test]
fn self_crossing_and_coincident_sources_are_counted_by_actual_traversals() {
    let bow = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![4., 4.],
        vec![0., 4.],
        vec![4., 0.],
        vec![0., 0.],
    ])
    .unwrap();
    let loops = vec![vec![bow.clone()]];
    assert_eq!(check(&loops, [2., 1.]).windings(), [-1]);
    assert_eq!(check(&loops, [2., 3.]).windings(), [1]);
    assert!(p::certify_point_winding(&loops, [2., 2.]).is_err());
    let reverse = bow.reverse().unwrap();
    let r = check(&[vec![bow.clone()], vec![bow], vec![reverse]], [2., 3.]);
    assert_eq!(r.windings(), [1, 1, -1]);
    assert_eq!(r.weighted_winding(&[1, 1, 2]).unwrap(), 0);
}

#[test]
fn a_wide_interval_or_forged_point_cannot_hide_multiple_crossings() {
    let loops = vec![arch(vec![1.; 3])];
    let c = p::certify_point_winding(&loops, [0., 1.]).unwrap();
    let mut bad = c.clone();
    bad.cells[0].roots[0].interval = [["0".into(), "1".into()], ["1".into(), "1".into()]];
    assert!(p::verify_point_winding(&loops, [0., 1.], &bad).is_err());
    let mut bad = c;
    bad.point[0] = ["1".into(), "1".into()];
    assert!(p::verify_point_winding(&loops, [0., 1.], &bad).is_err());
}
