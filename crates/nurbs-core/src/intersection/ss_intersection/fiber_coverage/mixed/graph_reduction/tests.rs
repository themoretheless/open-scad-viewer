use super::*;

#[test]
fn exact_midpoint_root_image_is_invariant_under_sign_and_degree_elevation() {
    let cell = full_cell();
    let net = vec![
        vec![q(-0.25).unwrap(), q(-0.125).unwrap()],
        vec![q(0.75).unwrap(), q(0.875).unwrap()],
    ];
    let expected = [[q(0.125).unwrap(), q(0.25).unwrap()], cell[1].clone()];
    for degree in [1, 2, 3] {
        let mut elevated = net.clone();
        for _ in 1..degree {
            elevated = elevate(&elevated, 0);
        }
        assert_eq!(root_image(&cell, &elevated, 0), Some(expected.clone()));
        let transposed: Net = (0..elevated[0].len())
            .map(|j| elevated.iter().map(|row| row[j].clone()).collect())
            .collect();
        assert_eq!(
            root_image(&cell, &transposed, 1),
            Some([expected[1].clone(), expected[0].clone()])
        );
        for value in elevated.iter_mut().flatten() {
            *value = -&*value;
        }
        assert_eq!(root_image(&cell, &elevated, 0), Some(expected.clone()));
    }
    let turning_net = [0.25, -0.25, 0.25]
        .into_iter()
        .map(|value| vec![q(value).unwrap(); 2])
        .collect();
    assert_eq!(root_image(&cell, &turning_net, 0), None);

    let unequal_denominators = vec![
        vec![Q::new((-1).into(), 3.into()); 2],
        vec![Q::new(2.into(), 5.into()); 2],
    ];
    let exact = Q::new(5.into(), 11.into());
    assert_eq!(
        root_image(&cell, &unequal_denominators, 0),
        Some([[exact.clone(), exact], cell[1].clone()])
    );
}

#[test]
fn window_patches_restrict_the_net_in_original_normalized_coordinates() {
    let fraction = |n: i32, d: i32| Q::new(n.into(), d.into());
    let net = vec![
        vec![Q::zero(), fraction(2, 1)],
        vec![fraction(1, 1), fraction(3, 1)],
    ];
    let window = [
        [fraction(1, 4), fraction(3, 4)],
        [fraction(1, 8), fraction(1, 2)],
    ];
    let mut work = Work { used: 0, max: 2 };
    let clipped =
        window_patches(vec![(full_cell(), net)], &window, &mut work).unwrap();
    assert_eq!(work.used, 2);
    assert_eq!(
        clipped,
        vec![(
            window,
            vec![
                vec![fraction(1, 2), fraction(5, 4)],
                vec![fraction(1, 1), fraction(7, 4)],
            ],
        )],
    );
}

#[test]
fn diagonal_chart_intersects_distinct_native_parameter_ranges_exactly() {
    let domain = [[-3., 3.], [2., 10.], [10., 13.], [-4., 0.]];
    let chart = diagonal_chart(
        domain,
        [[-2., 2.], [3., 8.], [11., 13.], [-3., -1.]],
    )
    .unwrap();
    let fraction = |n: i32, d: i32| Q::new(n.into(), d.into());
    let expected = [
        [fraction(1, 3), fraction(5, 6)],
        [fraction(1, 4), fraction(3, 4)],
    ];
    assert_eq!(chart, expected);
    assert!(contained(&expected, &chart));

    let mut touching = expected.clone();
    touching[0] = [Q::zero(), expected[0][0].clone()];
    assert!(overlaps(&touching, &chart));
    touching[0][1] -= fraction(1, 1_000_000);
    assert!(!overlaps(&touching, &chart));

    let mut outside = expected.clone();
    outside[0][0] = fraction(1, 3) - fraction(1, 1_000_000);
    assert!(!contained(&outside, &chart));

    let disjoint = diagonal_chart(
        domain,
        [[-3., -2.], [2., 10.], [11., 13.], [-4., 0.]],
    )
    .unwrap();
    assert!(!contained(&expected, &disjoint));
}

#[test]
fn exact_reduction_replay_rejects_missing_cells_and_forged_exclusions() {
    let mut a = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![vec![0.; 3]; 3]; 3],
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let q = [0.25, -0.25, 0.25];
    for i in 0..3 {
        for j in 0..3 {
            a.control_points[i][j] = vec![i as f64 / 2., j as f64 / 2., 0.];
        }
    }
    let mut b = a.clone();
    for i in 0..3 {
        for j in 0..3 {
            b.control_points[i][j][2] = q[i] + q[j] - 0.0625;
        }
    }
    let source = super::super::super::super::FiberSources::new(&a, &b).unwrap();
    let mut reversed = a.clone();
    for (i, row) in reversed.control_points.iter_mut().enumerate() {
        for point in row {
            point[0] = 1.25 - 0.75 * i as f64;
        }
    }
    let different = FiberSources::new(&reversed, &b).unwrap();
    assert!(exclude_shared_frame_graph_contact(
        &different,
        [[0., 1.], [0., 0.125], [0.25, 0.75], [0., 0.125]],
    ).unwrap());
    assert!(!exclude_shared_frame_graph_contact(&different, [[0., 1.]; 4]).unwrap());
    let owner = crate::ss_intersection::construct_qualified_closed_ss_branches(
        &source,
        Default::default(),
    )
    .unwrap();
    let coverage = owner.owner().proof().coverage();
    coverage.reverify().unwrap();
    assert!(coverage.reduction.is_some());
    let mut bad = coverage.clone();
    bad.reduction.as_mut().unwrap().nodes[0] = Node::Excluded;
    assert!(bad.reverify().is_err());
    let mut bad = coverage.clone();
    bad.reduction.as_mut().unwrap().nodes[0] = Node::Chart(usize::MAX);
    assert!(bad.reverify().is_err());
    let mut bad = coverage.clone();
    bad.reduction.as_mut().unwrap().nodes.pop();
    assert!(bad.reverify().is_err());
    let mut bad = coverage.clone();
    bad.report.rectangles += 1;
    assert!(bad.reverify().is_err());
}
