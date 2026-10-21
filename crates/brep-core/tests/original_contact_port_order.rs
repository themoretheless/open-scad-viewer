use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn curve(points: Vec<Vec<f64>>) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    }
}
#[test]
fn exact_six_port_triple_order_retains_original_sources_and_traversals() {
    let a = curve(vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]]);
    let b = curve(vec![vec![-4., 1.5], vec![4., 1.5]]);
    let c = curve(vec![
        vec![-3., 3.],
        vec![-3., 3.],
        vec![-2., 1.],
        vec![0., -3.],
        vec![3., 3.],
    ]);
    let atlas = Arc::new(
        p::assemble_original_contact_atlas(&[a, b, c], 1e-12, p::ContactAtlasLimits::default())
            .unwrap(),
    );
    for (group, expected) in [
        (0, vec![(1, 1), (0, 1), (2, -1), (1, -1), (0, -1), (2, 1)]),
        (1, vec![(1, 1), (2, 1), (0, -1), (1, -1), (2, -1), (0, 1)]),
    ] {
        let order = p::own_original_contact_port_order(
            atlas.clone(),
            group,
            p::ContactPortOrderLimits::default(),
        )
        .unwrap();
        order.reverify_source().unwrap();
        assert_eq!(
            order
                .ports()
                .iter()
                .map(|p| (p.source, p.direction))
                .collect::<Vec<_>>(),
            expected
        );
        assert!(order.ports().iter().all(|p| p.branch().ray().order == 1));
    }
    let endpoint =
        p::own_original_contact_port_order(atlas, 2, p::ContactPortOrderLimits::default()).unwrap();
    endpoint.reverify_source().unwrap();
    assert_eq!(endpoint.ports().len(), 1);
    assert_eq!(endpoint.ports()[0].branch().ray().order, 2);
}
#[test]
fn self_contact_has_four_owned_ports_at_two_distinct_parameters() {
    let c = curve(vec![
        vec![3., -1.5],
        vec![-1., 3.5],
        vec![-1., -3.5],
        vec![3., 1.5],
    ]);
    let atlas = Arc::new(
        p::assemble_original_contact_atlas(&[c], 1e-12, p::ContactAtlasLimits::default()).unwrap(),
    );
    let order =
        p::own_original_contact_port_order(atlas, 0, p::ContactPortOrderLimits::default()).unwrap();
    order.reverify_source().unwrap();
    assert_eq!(
        order
            .ports()
            .iter()
            .map(|p| (p.parameter_group, p.direction))
            .collect::<Vec<_>>(),
        [(1, 1), (0, 1), (1, -1), (0, -1)]
    );
}
#[test]
fn tangent_contacts_and_cusps_have_intrinsic_order_while_partial_work_refuses() {
    let a = curve(vec![vec![-3., 9.], vec![0., -9.], vec![3., 9.]]);
    let b = curve(vec![
        vec![-3., 29.25],
        vec![-1.5, -60.75],
        vec![0., 125.25],
        vec![1.5, -60.75],
        vec![3., 29.25],
    ]);
    let atlas = Arc::new(
        p::assemble_original_contact_atlas(&[a, b], 1e-12, p::ContactAtlasLimits::default())
            .unwrap(),
    );
    assert_eq!(atlas.groups().len(), 2);
    for (group, expected) in [
        (0, vec![(1, -1), (0, -1), (0, 1), (1, 1)]),
        (1, vec![(0, 1), (1, 1), (1, -1), (0, -1)]),
    ] {
        let order = p::own_original_contact_port_order(
            atlas.clone(),
            group,
            p::ContactPortOrderLimits::default(),
        )
        .unwrap();
        order.reverify_source().unwrap();
        assert_eq!(
            order
                .ports()
                .iter()
                .map(|p| (p.source, p.direction))
                .collect::<Vec<_>>(),
            expected
        );
    }
    let cusp = curve(vec![
        vec![3., -1.],
        vec![-1., 1.],
        vec![-1., -1.],
        vec![3., 1.],
    ]);
    let atlas = Arc::new(
        p::assemble_original_contact_atlas(&[cusp], 1e-12, p::ContactAtlasLimits::default())
            .unwrap(),
    );
    let order =
        p::own_original_contact_port_order(atlas, 0, p::ContactPortOrderLimits::default()).unwrap();
    order.reverify_source().unwrap();
    assert_eq!(
        order
            .ports()
            .iter()
            .map(|p| p.direction)
            .collect::<Vec<_>>(),
        [1, -1]
    );
    let a = curve(vec![vec![-1., -1.], vec![1., 1.]]);
    let b = curve(vec![vec![-1., 1.], vec![1., -1.]]);
    let atlas = Arc::new(
        p::assemble_original_contact_atlas(&[a, b], 1e-12, p::ContactAtlasLimits::default())
            .unwrap(),
    );
    let limits = p::ContactPortOrderLimits {
        max_comparisons: 0,
        ..p::ContactPortOrderLimits::default()
    };
    assert!(p::own_original_contact_port_order(atlas, 0, limits).is_err());
}

#[test]
fn cusp_axis_rotations_preserve_intrinsic_order_and_positive_x_cut() {
    for rotation in 0..4 {
        let mut c = curve(vec![
            vec![3., -1.],
            vec![-1., 1.],
            vec![-1., -1.],
            vec![3., 1.],
        ]);
        for p in &mut c.control_points {
            let (x, y) = (p[0], p[1]);
            let q = match rotation {
                0 => [x, y],
                1 => [-y, x],
                2 => [-x, -y],
                _ => [y, -x],
            };
            p[0] = q[0];
            p[1] = q[1];
        }
        let atlas = Arc::new(
            p::assemble_original_contact_atlas(&[c], 1e-12, p::ContactAtlasLimits::default())
                .unwrap(),
        );
        let order =
            p::own_original_contact_port_order(atlas, 0, p::ContactPortOrderLimits::default())
                .unwrap();
        order.reverify_source().unwrap();
        let expected = if rotation == 0 {
            vec![1, -1]
        } else {
            vec![-1, 1]
        };
        assert_eq!(
            order
                .ports()
                .iter()
                .map(|p| p.direction)
                .collect::<Vec<_>>(),
            expected
        );
    }
}
#[test]
fn intrinsic_tangent_order_is_invariant_under_quartic_to_degree_eight_clock_change() {
    let choose = |n: usize, k: usize| -> i64 {
        if k > n {
            return 0;
        }
        let mut v = 1;
        for j in 0..k {
            v = v * (n - j) as i64 / (j + 1) as i64;
        }
        v
    };
    let a = curve(vec![vec![-14., 980.], vec![0., -980.], vec![14., 980.]]);
    for nonlinear in [false, true] {
        let (degree, power) = if nonlinear {
            (
                8,
                vec![49000, 0, -772240, 0, 3845520, 0, -6146560, 0, 3073280],
            )
        } else {
            (4, vec![49000, -772240, 3845520, -6146560, 3073280])
        };
        let points = (0..=degree)
            .map(|i| {
                let y = power
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| *k <= i)
                    .map(|(k, &c)| {
                        let n = c * choose(i, k);
                        let d = choose(degree, k);
                        assert_eq!(n % d, 0);
                        n / d
                    })
                    .sum::<i64>();
                let x = if nonlinear {
                    (i * (i.saturating_sub(1)) / 2) as i64 - 14
                } else {
                    7 * i as i64 - 14
                };
                vec![x as f64, y as f64]
            })
            .collect();
        let b = curve(points);
        let atlas = Arc::new(
            p::assemble_original_contact_atlas(
                &[a.clone(), b],
                1e-12,
                p::ContactAtlasLimits::default(),
            )
            .unwrap(),
        );
        for (group, expected) in [
            (0, vec![(1, -1), (0, -1), (0, 1), (1, 1)]),
            (1, vec![(0, 1), (1, 1), (1, -1), (0, -1)]),
        ] {
            let order = p::own_original_contact_port_order(
                atlas.clone(),
                group,
                p::ContactPortOrderLimits::default(),
            )
            .unwrap();
            order.reverify_source().unwrap();
            assert_eq!(
                order
                    .ports()
                    .iter()
                    .map(|p| (p.source, p.direction))
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}

#[test]
fn tangent_order_survives_frame_rotation_and_requires_actual_local_monotonicity() {
    let base = [
        vec![(1, -1), (0, -1), (0, 1), (1, 1)],
        vec![(0, 1), (1, 1), (1, -1), (0, -1)],
    ];
    for rotation in 0..4 {
        let mut a = curve(vec![vec![-3., 9.], vec![0., -9.], vec![3., 9.]]);
        let mut b = curve(vec![
            vec![-3., 29.25],
            vec![-1.5, -60.75],
            vec![0., 125.25],
            vec![1.5, -60.75],
            vec![3., 29.25],
        ]);
        for c in [&mut a, &mut b] {
            for p in &mut c.control_points {
                let (x, y) = (p[0], p[1]);
                let q = match rotation {
                    0 => [x, y],
                    1 => [-y, x],
                    2 => [-x, -y],
                    _ => [y, -x],
                };
                p[0] = q[0];
                p[1] = q[1];
            }
        }
        let atlas = Arc::new(
            p::assemble_original_contact_atlas(&[a, b], 1e-12, p::ContactAtlasLimits::default())
                .unwrap(),
        );
        for group in 0..2 {
            let order = p::own_original_contact_port_order(
                atlas.clone(),
                group,
                p::ContactPortOrderLimits::default(),
            )
            .unwrap();
            order.reverify_source().unwrap();
            let actual = order
                .ports()
                .iter()
                .map(|p| (p.source, p.direction))
                .collect::<Vec<_>>();
            assert!((0..4).any(|shift| (0..4).all(|i| actual[i] == base[group][(i + shift) % 4])));
        }
        if rotation == 1 {
            let limits = p::ContactPortOrderLimits {
                max_neighbourhood_steps: 1,
                ..p::ContactPortOrderLimits::default()
            };
            assert!(p::own_original_contact_port_order(atlas, 0, limits).is_err());
        }
    }
}
