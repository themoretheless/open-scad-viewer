use nurbs_core::{surface::Surface, surface_regularity::inspect};
fn panel() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![4., 0., 0.], vec![4., 4., 0.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn weighted_plane_and_polynomial_graph_are_regular() {
    let p = inspect(&panel(), 1000).unwrap();
    assert!(p.spanwise_regular, "{p:?}");
    assert!(p.interior_basis_c1);
    let g = nurbs_core::polynomial::graph(
        [0., 1., 0., 1.],
        &[vec![0., 0., 1.], vec![0., 1., 0.], vec![1., 0., 0.]],
    )
    .unwrap();
    assert!(inspect(&g, 1000).unwrap().spanwise_regular);
}
#[test]
fn collapsed_and_folded_patches_are_never_certified() {
    let mut p = panel();
    for row in &mut p.control_points {
        for point in row {
            point[0] = 0.;
        }
    }
    let r = inspect(&p, 31).unwrap();
    assert!(!r.spanwise_regular);
    assert!(!r.unresolved.is_empty());
    assert!(r.cells <= 31);
    let folded = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            vec![vec![-1., 0., 0.], vec![-1., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    assert!(!inspect(&folded, 63).unwrap().spanwise_regular);
    let mut rational = folded;
    rational.weights = vec![vec![1., 1.125], vec![2., 2.5], vec![1., 1.5]];
    // The two vertical boundaries still have the same x; the signed normal
    // reverses across the patch even though both weight derivatives vary.
    assert!(!inspect(&rational, 63).unwrap().spanwise_regular);
}
#[test]
fn rejects_invalid_source_and_budget() {
    for b in [0, 100001] {
        assert!(inspect(&panel(), b).is_err());
    }
    let mut p = panel();
    p.weights[0][0] = 0.;
    assert!(inspect(&p, 100).is_err());
}

#[test]
fn spanwise_regular_c0_seam_does_not_claim_c1() {
    let mut s = panel();
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    s.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 4., 0.]],
        vec![vec![2., 0., 0.], vec![2., 4., 0.]],
        vec![vec![4., 0., 1.], vec![4., 4., 1.]],
    ];
    s.weights = vec![vec![1.; 2]; 3];
    assert!(inspect(&s, 1).is_err());
    let r = inspect(&s, 100).unwrap();
    assert!(r.spanwise_regular);
    assert!(!r.interior_basis_c1);
}

// Actual exact-affine moving-profile patch that previously spent the entire
// 100000-cell allowance differencing increasingly tiny restricted poles.
fn thin_moving_rational_panel() -> Surface {
    let points = [
        [
            [1.2906953394412994, 0.7670649290084839, 0.2909378409385681],
            [1.3299258649349213, 0.8206284642219543, 0.31870734691619873],
            [1.369156390428543, 0.8741919994354248, 0.34647685289382935],
            [1.4062458276748657, 0.9330057799816132, 0.3799797594547272],
            [1.4360445439815521, 0.9909565448760986, 0.41208121180534363],
            [1.4658432602882385, 1.048907309770584, 0.4441826641559601],
        ],
        [
            [1.2967345714569092, 0.7621845304965973, 0.2918007969856262],
            [1.3360783755779266, 0.8159012794494629, 0.31964412331581116],
            [1.375422179698944, 0.8696180284023285, 0.3474874496459961],
            [1.4126065075397491, 0.9285755753517151, 0.3810669183731079],
            [1.4424942135810852, 0.9866998493671417, 0.41325893998146057],
            [1.4723819196224213, 1.0448241233825684, 0.44545096158981323],
        ],
        [
            [1.2992683947086334, 0.7640558779239655, 0.2846514582633972],
            [1.3386875689029694, 0.8178820908069611, 0.31254884600639343],
            [1.3781067430973053, 0.8717083036899567, 0.34044623374938965],
            [1.4153455197811127, 0.9307780563831329, 0.37408843636512756],
            [1.4452773034572601, 0.988996297121048, 0.4063299298286438],
            [1.4752090871334076, 1.047214537858963, 0.43857142329216003],
        ],
    ];
    Surface {
        degree_u: 2,
        degree_v: 5,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
        control_points: points
            .into_iter()
            .map(|r| r.into_iter().map(|p| p.to_vec()).collect())
            .collect(),
        weights: vec![vec![1.; 6], vec![0.5; 6], vec![1.; 6]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn thin_rational_moving_patch_uses_original_derivatives_and_refuses_short_work() {
    let s = thin_moving_rational_panel();
    let p = inspect(&s, 10000).unwrap();
    eprintln!(
        "thin moving rational regularity cells={} accepted={}",
        p.cells, p.proven_cells
    );
    assert!(p.spanwise_regular, "{p:?}");
    assert_eq!(p.cells, 1);
    assert!(inspect(&s, 0).is_err());
    let mut reparameterized = s.clone();
    reparameterized.knots_u = vec![-3., -3., -3., 5., 5., 5.];
    reparameterized.knots_v = [vec![2.; 6], vec![6.; 6]].concat();
    for r in &mut reparameterized.weights {
        for w in r {
            *w *= 2.;
        }
    }
    assert!(inspect(&reparameterized, 10000).unwrap().spanwise_regular);
}

#[test]
fn curved_normal_cover_refuses_one_short_cell_budget_on_non_dyadic_domains() {
    let mut s = Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![0.1, 0.1, 0.1, 0.1, 0.9, 0.9, 0.9, 0.9],
        knots_v: vec![0.2, 0.2, 1.1, 1.1],
        control_points: [[0., 0.], [1., 2.], [-1., 2.], [0., 0.]]
            .iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let report = inspect(&s, 1000).unwrap();
    assert!(report.spanwise_regular && report.cells > 1, "{report:?}");
    let short = inspect(&s, report.cells - 1).unwrap();
    assert!(!short.spanwise_regular && !short.unresolved.is_empty());
    assert!(short.cells <= report.cells - 1);
    for c in short.unresolved {
        assert!(
            c.domain[0][0] >= 0.1
                && c.domain[0][1] <= 0.9
                && c.domain[1][0] >= 0.2
                && c.domain[1][1] <= 1.1
        );
    }
    // Collapse the independent axis: every normal numerator must remain zero.
    for row in &mut s.control_points {
        row[1] = row[0].clone();
    }
    assert!(!inspect(&s, 1000).unwrap().spanwise_regular);
}
