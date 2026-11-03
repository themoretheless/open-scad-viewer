use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        original_quadric_graph_contacts, OriginalQuadricGraphParameterization,
    },
    surface::Surface,
};
fn target(kind: usize) -> (Surface, [[f64; 4]; 4], Curve, [f64; 3]) {
    let (p, x, w, multi, l, point, u) = match kind {
        0 => (3, vec![0., 1., 0., 1.], vec![1.; 4], false, 1., 0.5, 0.),
        1 => (
            4,
            vec![0., 63., 70., 15., 105.],
            vec![1., 1.25, 1.5, 1.75, 2.],
            false,
            105.,
            52.5,
            0.,
        ),
        2 => (
            3,
            vec![0., 0.25, 0.625, 0.375, 0.75, 1.],
            vec![1.; 6],
            true,
            1.,
            0.5,
            0.,
        ),
        _ => (
            3,
            vec![0., 0.75, 21. / 16., 0.375, 6., 9.],
            vec![1.; 6],
            true,
            9.,
            1.,
            -2. / 3.,
        ),
    };
    let knots = if multi {
        [vec![-2.; p + 1], vec![-1., 1.], vec![2.; p + 1]].concat()
    } else {
        [vec![-2.; p + 1], vec![2.; p + 1]].concat()
    };
    let s = Surface {
        degree_u: p,
        degree_v: 1,
        knots_u: knots,
        knots_v: vec![10., 10., 14., 14.],
        control_points: x
            .iter()
            .map(|a| vec![vec![*a, 0., 0.], vec![*a, l, *a]])
            .collect(),
        weights: w.iter().map(|w| vec![*w, *w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let q = [
        [0., 0.5, 0., 0.],
        [0.5, 0., 0., 0.],
        [0., 0., 0., -l / 2.],
        [0., 0., -l / 2., 0.],
    ];
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![
            vec![point, l / 2., point / 2. - 1.],
            vec![point, l / 2., point / 2. + 1.],
        ],
        weights: vec![1.; 2],
        periodic: false,
    };
    (s, q, c, [4., u, 12.])
}
#[test]
fn unordered_single_rational_and_multispan_clocks_preserve_stationary_uv() {
    for kind in 0..4 {
        for gauge in [1., 7.] {
            for rotated in [false, true] {
                let (mut s, mut q, mut c, expected) = target(kind);
                for row in &mut s.weights {
                    for w in row {
                        *w *= gauge;
                    }
                }
                if rotated {
                    for row in &mut s.control_points {
                        for p in row {
                            p.swap(0, 2);
                        }
                    }
                    for p in &mut c.control_points {
                        p.swap(0, 2);
                    }
                    let old = q;
                    let permutation = [2, 1, 0, 3];
                    q = std::array::from_fn(|i| {
                        std::array::from_fn(|j| old[permutation[i]][permutation[j]])
                    });
                }
                assert!(s
                    .control_points
                    .windows(2)
                    .any(|p| p[0][1][if rotated { 2 } else { 0 }]
                        > p[1][1][if rotated { 2 } else { 0 }]));
                let before = s.clone();
                let r =
                    original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
                assert_eq!(s, before);
                assert!(r.target_graph_proven && r.complete, "kind {kind}: {r:?}");
                assert_eq!(
                    r.target_parameterization,
                    OriginalQuadricGraphParameterization::HomogeneousSeparable
                );
                assert_eq!(r.contacts.len(), 1);
                for (axis, x) in expected.into_iter().enumerate() {
                    let b = r.contacts[0].parameter_box[axis];
                    assert!(b[0] <= x && x <= b[1], "kind {kind}, {b:?} excludes {x}");
                    assert!(b[1] - b[0] < 1e-12);
                }
            }
        }
    }
}
#[test]
fn positive_endpoint_slopes_do_not_hide_an_interior_clock_fold() {
    let (mut s, q, c, _) = target(0);
    for (i, x) in [0., 2., -1., 1.].into_iter().enumerate() {
        s.control_points[i][0][0] = x;
        s.control_points[i][1][0] = x;
        s.control_points[i][1][2] = x;
    }
    assert!(original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).is_err());
}
#[test]
fn unordered_clock_proof_does_not_reset_predicate_work() {
    let (s, q, c, _) = target(3);
    let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).unwrap();
    assert_eq!(r.exact_work, 0);
    assert!(!r.target_graph_proven && r.carrier.is_none());
}
