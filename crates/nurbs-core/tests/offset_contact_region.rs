use nurbs_core::{
    curve::Curve,
    surface::Surface,
    surface_offset::contact_curve::chain::{self, Limits, Span, region},
    trim_domain::{self, Location},
};
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
fn planes() -> [Surface; 2] {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    [a, b]
}
fn spans() -> [Span; 2] {
    [
        Span {
            owner: 0,
            axis: 0,
            drive: [0.2, 0.5],
            free: [0.1, 0.3],
            other: [[0.3, 0.6], [0.1, 0.3]],
            reversed: false,
        },
        Span {
            owner: 1,
            axis: 0,
            drive: [0.5000000001, 0.6],
            free: [0.1, 0.3],
            other: [[0.4, 0.9], [0.1, 0.3]],
            reversed: false,
        },
    ]
}
fn chain() -> chain::Chain {
    let s = planes();
    chain::build(
        s.each_ref(),
        [0.2, -0.2],
        &spans(),
        1e-7,
        1e-7,
        Limits::default(),
    )
    .unwrap()
}
fn qualified(r: &region::Region, l: region::Limits) {
    assert!(r.simplicity().proven_simple);
    assert_eq!(
        trim_domain::exact_loop_joins(r.uv_loop()).unwrap(),
        Some(true)
    );
    assert_eq!(r.uv_loop().len(), r.world_loop().len());
    for (i, c) in r.world_loop().iter().enumerate() {
        assert_eq!(
            c.control_points.last().unwrap(),
            &r.world_loop()[(i + 1) % r.world_loop().len()].control_points[0]
        );
    }
    assert!(r.lift_proofs().iter().all(|p| p.lift_correspondence_proven
        && p.error_upper_mm.unwrap() <= 1e-7
        && p.visited <= l.final_lift.cells));
    for p in r.rail_fitting() {
        assert!(p.curve.is_some());
        assert!(
            p.evaluations <= l.closing_fit.evaluations
                && p.coefficient_work <= l.closing_fit.coefficient_work
                && p.proof_cells <= l.closing_fit.proof_cells
        );
    }
    r.reverify().unwrap();
}
#[test]
fn switched_contact_clock_closes_as_actual_uv_jordan_and_world_lift_loop() {
    let c = chain();
    let l = region::Limits::default();
    let r = c.close_to_boundary(0, 0, l).unwrap();
    assert_eq!(r.uv_loop().len(), 5);
    qualified(&r, l);
    assert_eq!(
        r.classify([[0.3, 0.4], [0.05, 0.1]], 10000)
            .unwrap()
            .location,
        Location::Inside
    );
    assert_eq!(
        r.classify([[0.3, 0.4], [0.4, 0.5]], 10000)
            .unwrap()
            .location,
        Location::Outside
    );
    assert_eq!(
        r.classify([[0.35, 0.36], [0.19, 0.21]], 10000)
            .unwrap()
            .location,
        Location::Unresolved
    );
    let other = c.close_to_boundary(1, 0, l).unwrap();
    qualified(&other, l);
    assert_eq!(other.source_side(), 1);
    assert_eq!(
        other
            .classify([[0.45, 0.5], [0.05, 0.1]], 10000)
            .unwrap()
            .location,
        Location::Inside
    );
}

#[test]
fn reversed_nonunit_contact_clocks_still_close_and_classify_in_original_uv() {
    let mut s = planes();
    s[0].knots_u = vec![2., 2., 7., 7.];
    s[1].knots_u = vec![-3., -3., 4., 4.];
    let specs = [
        Span {
            owner: 0,
            axis: 0,
            drive: [4., 6.],
            free: [0.1, 0.3],
            other: [[-0.5, 1.5], [0.1, 0.3]],
            reversed: true,
        },
        Span {
            owner: 0,
            axis: 0,
            drive: [3., 4.],
            free: [0.1, 0.3],
            other: [[-0.5, 1.5], [0.1, 0.3]],
            reversed: true,
        },
    ];
    let c = chain::build(
        s.each_ref(),
        [0.2, -0.2],
        &specs,
        1e-7,
        1e-7,
        Limits::default(),
    )
    .unwrap();
    let l = region::Limits::default();
    let r = c.close_to_boundary(0, 0, l).unwrap();
    qualified(&r, l);
    assert_eq!(
        r.classify([[4.5, 5.], [0.05, 0.1]], 10000)
            .unwrap()
            .location,
        Location::Inside
    );
    assert_eq!(
        r.classify([[6.5, 6.7], [0.05, 0.1]], 10000)
            .unwrap()
            .location,
        Location::Outside
    );
}
fn curved() -> [Surface; 2] {
    let mut a = plane();
    a.degree_u = 2;
    a.knots_u = vec![0., 0., 0., 1., 1., 1.];
    a.control_points = [[1., 0.], [1., 1.], [0., 1.]]
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
        .to_vec();
    a.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
        .map(|w| vec![w; 2])
        .to_vec();
    let mut b = plane();
    b.control_points = vec![
        vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        vec![vec![0., 2., 0.], vec![0.5, 2., 1.]],
    ];
    [a, b]
}
#[test]
fn nonlinear_contact_region_closes_on_literal_cylinder_boundary() {
    let s = curved();
    let saved = s.clone();
    let span = Span {
        owner: 0,
        axis: 1,
        drive: [0.35, 0.55],
        free: [0.88, 0.96],
        other: [[0.07, 0.22], [0.3, 0.6]],
        reversed: false,
    };
    let c = chain::build(
        s.each_ref(),
        [0.1; 2],
        &[span],
        1e-7,
        1e-7,
        Limits::default(),
    )
    .unwrap();
    let l = region::Limits::default();
    let r = c.close_to_boundary(0, 3, l).unwrap();
    qualified(&r, l);
    assert_eq!(
        r.classify([[0.2, 0.3], [0.4, 0.45]], 10000)
            .unwrap()
            .location,
        Location::Inside
    );
    assert_eq!(
        r.classify([[0.97, 0.98], [0.4, 0.45]], 10000)
            .unwrap()
            .location,
        Location::Outside
    );
    assert_eq!(s, saved);
}
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1.; 2],
        periodic: false,
    }
}
#[test]
fn exact_endpoint_closure_alone_cannot_admit_crossing_or_escaped_regions() {
    let c = chain();
    let first = &c.pieces()[0].source_uv()[0].control_points[0];
    let last = c.pieces().last().unwrap().source_uv()[0]
        .control_points
        .last()
        .unwrap();
    let a = [first[0], first[1]];
    let b = [last[0], last[1]];
    let crossing = vec![
        line(b, [0.3, 0.05]),
        line([0.3, 0.05], [0.7, 0.05]),
        line([0.7, 0.05], a),
    ];
    assert!(
        c.close_with(0, &crossing, region::Limits::default())
            .is_err()
    );
    let escaped = vec![line(b, [1.2, 0.05]), line([1.2, 0.05], a)];
    assert!(
        c.close_with(0, &escaped, region::Limits::default())
            .is_err()
    );
    assert!(
        c.close_to_boundary(0, 4, region::Limits::default())
            .is_err()
    );
}
#[test]
fn independent_closure_work_limits_refuse_without_publishing_partial_regions() {
    let c = chain();
    for kind in 0..4 {
        let mut l = region::Limits::default();
        match kind {
            0 => l.max_curves = 3,
            1 => l.simplicity_pairs = 1,
            2 => l.closing_fit.evaluations = 1,
            _ => l.final_lift.cells = 0,
        }
        assert!(c.close_to_boundary(0, 0, l).is_err());
    }
}
