use nurbs_core::{
    curve::Curve,
    surface::Surface,
    surface_offset::contact_curve::chain::{
        self, Limits, Span,
        region::{self, compound},
    },
    trim_domain::Location,
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
fn outer() -> region::Region {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    let span = Span {
        owner: 0,
        axis: 0,
        drive: [0.1, 0.9],
        free: [0.7, 0.9],
        other: [[0.2, 0.8], [0.7, 0.9]],
        reversed: false,
    };
    chain::build(
        [&a, &b],
        [0.8, -0.8],
        &[span],
        1e-7,
        1e-7,
        Limits::default(),
    )
    .unwrap()
    .close_to_boundary(0, 0, region::Limits::default())
    .unwrap()
}
fn square(lo: f64, hi: f64) -> Vec<Curve> {
    let p = [[lo, lo], [hi, lo], [hi, hi], [lo, hi]];
    (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
        .collect()
}
fn qualified(c: &compound::Compound, l: compound::Limits) {
    assert!(c.uv_domain().proof().proven);
    assert_eq!(c.world_loops().len(), c.uv_domain().loops().len());
    for loop_ in c.world_loops() {
        for (i, curve) in loop_.iter().enumerate() {
            assert_eq!(
                curve.control_points.last().unwrap(),
                &loop_[(i + 1) % loop_.len()].control_points[0]
            );
        }
    }
    assert!(
        c.lift_proofs()
            .iter()
            .flatten()
            .all(|r| r.lift_correspondence_proven
                && r.error_upper_mm.unwrap() <= 1e-7
                && r.visited <= l.final_lift.cells)
    );
    assert!(c.inner_fitting().iter().flatten().all(|r| r.curve.is_some()
        && r.evaluations <= l.fit.evaluations
        && r.coefficient_work <= l.fit.coefficient_work
        && r.proof_cells <= l.fit.proof_cells));
    c.reverify().unwrap();
}
#[test]
fn holes_and_islands_get_material_parity_with_original_uv_and_closed_world_lifts() {
    let inputs = vec![square(0.2, 0.7), square(0.3, 0.6), square(0.4, 0.5)];
    let saved = inputs.clone();
    let l = compound::Limits::default();
    let c = outer().with_nested(&inputs, l).unwrap();
    assert_eq!(
        c.uv_domain()
            .nodes()
            .iter()
            .map(|n| n.depth)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(c.uv_domain().components().len(), 2);
    assert_eq!(&c.uv_domain().loops()[1..], &saved);
    for (p, loc) in [
        ([0.15, 0.4], Location::Inside),
        ([0.25, 0.4], Location::Outside),
        ([0.35, 0.4], Location::Inside),
        ([0.45, 0.45], Location::Outside),
    ] {
        assert_eq!(c.classify(p.map(|x| [x; 2]), 10000).unwrap().location, loc);
    }
    assert_eq!(
        c.classify([[0.39, 0.41], [0.45, 0.46]], 10000)
            .unwrap()
            .location,
        Location::Unresolved
    );
    qualified(&c, l);
    assert_eq!(inputs, saved);
}
#[test]
fn outside_touching_or_escaped_extra_loops_are_not_holes_of_the_outer_region() {
    for inputs in [
        vec![square(0.91, 0.97)],
        vec![square(0.2, 0.8)],
        vec![square(1.1, 1.2)],
    ] {
        let saved = inputs.clone();
        assert!(
            outer()
                .with_nested(&inputs, compound::Limits::default())
                .is_err()
        );
        assert_eq!(inputs, saved);
    }
}
#[test]
fn compound_budgets_refuse_before_any_partial_world_region_is_published() {
    let inputs = vec![square(0.2, 0.7)];
    for kind in 0..4 {
        let mut l = compound::Limits::default();
        match kind {
            0 => l.nesting.pairs = 1,
            1 => l.fit.evaluations = 1,
            2 => l.final_lift.cells = 0,
            _ => l.nesting.max_loops = 1,
        }
        assert!(outer().with_nested(&inputs, l).is_err());
    }
}
fn curved_outer() -> region::Region {
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
    let span = Span {
        owner: 0,
        axis: 1,
        drive: [0.35, 0.55],
        free: [0.88, 0.96],
        other: [[0.07, 0.22], [0.3, 0.6]],
        reversed: false,
    };
    chain::build([&a, &b], [0.1; 2], &[span], 1e-7, 1e-7, Limits::default())
        .unwrap()
        .close_to_boundary(0, 3, region::Limits::default())
        .unwrap()
}
#[test]
fn rational_hole_on_nonlinear_offset_keeps_weights_and_literal_source_clocks() {
    let p = [
        [[1., 0.], [1., 1.], [0., 1.]],
        [[0., 1.], [-1., 1.], [-1., 0.]],
        [[-1., 0.], [-1., -1.], [0., -1.]],
        [[0., -1.], [1., -1.], [1., 0.]],
    ];
    let hole = p
        .map(|p| Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 2., 2., 2.],
            control_points: p
                .map(|p| vec![0.25 + 0.02 * p[0], 0.45 + 0.02 * p[1]])
                .to_vec(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        })
        .to_vec();
    let saved = hole.clone();
    let l = compound::Limits::default();
    let c = curved_outer().with_nested(&[hole], l).unwrap();
    assert_eq!(c.uv_domain().loops()[1], saved);
    assert_eq!(c.uv_domain().nodes()[1].parent, Some(0));
    assert_eq!(
        c.classify([[0.25; 2], [0.45; 2]], 10000).unwrap().location,
        Location::Outside
    );
    assert_eq!(
        c.classify([[0.4; 2], [0.45; 2]], 10000).unwrap().location,
        Location::Inside
    );
    qualified(&c, l);
}
