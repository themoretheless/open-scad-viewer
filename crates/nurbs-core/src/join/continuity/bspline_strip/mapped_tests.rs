use crate::{
    continuity::{
        inspect_surface_mapped_projective_strip_jets as mapped,
        inspect_surface_projective_strip_jets as inferred,
    },
    curve::Curve,
    surface::Surface,
};
#[path = "../../../../tests/support/authored_seam_map.rs"]
mod fixture;
#[test]
fn authored_map_proves_general_quartic_without_inverse_and_charges_complete_partition() {
    let (a, b, map) = fixture::fixture();
    let saved = (a.clone(), b.clone(), map.clone());
    assert!(
        inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified,
        "Coupled inverse must now discover the former authored-only map"
    );
    let proof = mapped(&a, &b, "vMax", "vMin", 2, 1., &map, 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    assert!(proof.work > 0 && proof.work <= 1000000);
    assert!(
        !mapped(&a, &b, "vMax", "vMin", 2, 1., &map, proof.work - 1)
            .unwrap()
            .certified
    );
    assert!(
        !mapped(&a, &b, "vMax", "vMin", 2, 1., &map, 0)
            .unwrap()
            .certified
    );
    let mut wrong = map.clone();
    wrong.control_points[1][0] = 0.125f64.next_up();
    assert!(
        !mapped(&a, &b, "vMax", "vMin", 2, 1., &wrong, 1000000)
            .unwrap()
            .certified
    );
    let mut gap = map.clone();
    gap.control_points[4][0] = 0.9;
    assert!(
        !mapped(&a, &b, "vMax", "vMin", 2, 1., &gap, 1000000)
            .unwrap()
            .certified
    );
    let mut jump = map.clone();
    jump.knots = vec![0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1.];
    jump.control_points = [0., 0.125, 0.375, 0.4, 0.65, 1.]
        .iter()
        .map(|&v| vec![v, 0.])
        .collect();
    jump.weights = vec![1.; 6];
    match mapped(&a, &b, "vMax", "vMin", 2, 1., &jump, 1000000) {
        Ok(p) => assert!(!p.certified),
        Err(e) => assert_eq!(e.code, "NURBS_INVALID_INPUT"),
    }
    let mut changed = b.clone();
    changed.control_points[4][0][1] = 90.75f64.next_up();
    assert!(
        !mapped(&a, &changed, "vMax", "vMin", 2, 1., &map, 1000000)
            .unwrap()
            .certified
    );
    assert_eq!((a, b, map), saved);
}
#[test]
fn authored_map_does_not_promote_curvature_and_refuses_noncovering_or_reversed_phase() {
    let (a, mut b, map) = fixture::fixture();
    for row in &mut b.control_points {
        row[2][1] += 0.125;
    }
    assert!(
        mapped(&a, &b, "vMax", "vMin", 1, 1., &map, 1000000)
            .unwrap()
            .certified
    );
    assert!(
        !mapped(&a, &b, "vMax", "vMin", 2, 1., &map, 1000000)
            .unwrap()
            .certified
    );
    let (_, b, mut reversed) = fixture::fixture();
    reversed.control_points.reverse();
    assert!(
        !mapped(&a, &b, "vMax", "vMin", 1, 1., &reversed, 1000000)
            .unwrap()
            .certified
    );
    let (_, _, mut double) = fixture::fixture();
    for p in &mut double.control_points {
        p[0] *= 2.;
    }
    assert!(
        !mapped(&a, &b, "vMax", "vMin", 1, 1., &double, 1000000)
            .unwrap()
            .certified
    );
}

#[test]
fn automatic_coupled_inverse_cancels_endpoint_factors_and_preserves_refusals() {
    let (a, b, _) = fixture::fixture();
    let full = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(full.certified, "{full:?}");
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 1., full.work - 1)
            .unwrap()
            .certified
    );
    let mut bad = b.clone();
    bad.control_points[4][0][1] = bad.control_points[4][0][1].next_up();
    assert!(
        !inferred(&a, &bad, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut kink = b.clone();
    for row in &mut kink.control_points {
        row[2][1] += 0.125;
    }
    assert!(
        inferred(&a, &kink, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    assert!(
        !inferred(&a, &kink, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut reverse = b.clone();
    reverse.control_points.reverse();
    reverse.weights.reverse();
    assert!(
        !inferred(&a, &reverse, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
}

#[test]
fn automatic_coupled_inverse_recovers_rational_clock_without_authored_map() {
    let (a, _, _) = fixture::fixture();
    let mut b = a.clone();
    b.knots_u = vec![-2., -2., -2., -2., -2., 6., 6., 6., 6., 6.];
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] += 1.;
        }
        b.weights[i] = vec![2f64.powi(4 - i as i32); 3];
    }
    let full = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(full.certified, "{full:?}");
    let mut bad = b.clone();
    bad.weights[2][0] = bad.weights[2][0].next_up();
    assert!(
        !inferred(&a, &bad, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
}

#[test]
fn automatic_quadratic_inverse_finds_map_with_no_linear_projective_inverse() {
    let (mut a, mut b, _) = fixture::fixture();
    let ay = [0., 210., 700., 2100., 5880.];
    let by = [
        0.0, 52.5, 135.0, 268.125, 492.75, 890.625, 1627.5, 3045.0, 5880.0,
    ];
    for (i, row) in a.control_points.iter_mut().enumerate() {
        for p in row {
            p[1] = ay[i];
        }
    }
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for p in row {
            p[1] = by[i];
        }
    }
    let full = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(full.certified, "{full:?}");
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 1., full.work - 1)
            .unwrap()
            .certified
    );
    b.control_points[4][0][1] = b.control_points[4][0][1].next_up();
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
}

fn split_original_u(surface: &Surface, fraction: f64) -> Surface {
    let mut rows = surface.control_points.clone();
    let mut left = vec![rows[0].clone()];
    let mut right = vec![rows.last().unwrap().clone()];
    while rows.len() > 1 {
        rows = rows
            .windows(2)
            .map(|w| {
                w[0].iter()
                    .zip(&w[1])
                    .map(|(a, b)| {
                        a.iter()
                            .zip(b)
                            .map(|(x, y)| (1. - fraction) * x + fraction * y)
                            .collect()
                    })
                    .collect()
            })
            .collect();
        left.push(rows[0].clone());
        right.push(rows.last().unwrap().clone());
    }
    right.reverse();
    left.extend(right.into_iter().skip(1));
    let mut s = surface.clone();
    let lo = s.knots_u[s.degree_u];
    let hi = s.knots_u[s.control_points.len()];
    let middle = lo + (hi - lo) * fraction;
    s.knots_u = [
        vec![lo; s.degree_u + 1],
        vec![middle; s.degree_u],
        vec![hi; s.degree_u + 1],
    ]
    .concat();
    s.weights = vec![vec![1.; 3]; left.len()];
    s.control_points = left;
    s
}
#[test]
fn automatic_map_crosses_original_source_knots_without_rounded_preimages() {
    let (a, b, _) = fixture::fixture();
    let split = split_original_u(&a, 0.5);
    let r = inferred(&split, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(r.certified, "{r:?}");
    // phi^{-1}(1/2)=(-1+sqrt(5))/2 is irrational. No rounded split is authority.
    assert!(
        !inferred(&split, &b, "vMax", "vMin", 2, 1., r.work - 1)
            .unwrap()
            .certified
    );
    let mut changed = split.clone();
    changed.control_points[6][0][1] = changed.control_points[6][0][1].next_up();
    assert!(
        !inferred(&changed, &b, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let split_b = split_original_u(&b, 0.5);
    let multiple = inferred(&split, &split_b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(multiple.certified, "{multiple:?}");
}
#[test]
fn automatic_cubic_inverse_recovers_proper_quintic_rational_clock() {
    let (a, _, _) = fixture::fixture();
    let mut a = a;
    a.degree_u = 5;
    a.knots_u = [vec![10.; 6], vec![14.; 6]].concat();
    let xy = [
        [0.0, 0.0],
        [2.0, 2.0],
        [5.0, 7.0],
        [11.0, 16.0],
        [24.0, 34.0],
        [60.0, 100.0],
    ];
    a.control_points = xy
        .iter()
        .map(|p| (0..3).map(|j| vec![p[0], p[1], 0.5 * j as f64]).collect())
        .collect();
    a.weights = vec![vec![1.; 3]; 6];
    let mut b = a.clone();
    b.knots_u = [vec![-2.; 6], vec![6.; 6]].concat();
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] += 1.;
        }
        b.weights[i] = vec![2f64.powi(5 - i as i32); 3];
    }
    let r = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(r.certified, "{r:?}");
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 1., r.work - 1)
            .unwrap()
            .certified
    );
    b.weights[2][0] = b.weights[2][0].next_up();
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
}

#[path = "../../../../tests/support/switched_seam_map.rs"]
mod switched_fixture;
#[test]
fn inverse_map_switch_inside_one_target_span_has_exact_irrational_cut() {
    for (a, b) in [
        switched_fixture::fixture(),
        switched_fixture::interior_pole_fixture(),
    ] {
        let saved = (a.clone(), b.clone());
        let full = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
        assert!(full.certified, "{full:?}");
        assert!(
            !inferred(&a, &b, "vMax", "vMin", 2, 1., full.work - 1)
                .unwrap()
                .certified
        );
        let mut changed = a.clone();
        changed.control_points[3][0][1] = changed.control_points[3][0][1].next_up();
        assert!(
            !inferred(&changed, &b, "vMax", "vMin", 2, 1., 1000000)
                .unwrap()
                .certified
        );
        let mut reverse = b.clone();
        reverse.control_points.reverse();
        reverse.weights.reverse();
        assert!(
            !inferred(&a, &reverse, "vMax", "vMin", 2, 1., 1000000)
                .unwrap()
                .certified
        );
        let mut curvature = b.clone();
        for row in &mut curvature.control_points {
            row[2][1] += 0.125;
        }
        assert!(
            inferred(&a, &curvature, "vMax", "vMin", 1, 1., 1000000)
                .unwrap()
                .certified
        );
        assert!(
            !inferred(&a, &curvature, "vMax", "vMin", 2, 1., 1000000)
                .unwrap()
                .certified
        );
        assert_eq!((a, b), saved);
    }
}

#[test]
fn algebraic_inverse_clock_is_proved_without_rounded_root_or_extra_budget() {
    // q=(s+s^2)/2 has strictly positive derivative but a nonrational inverse.
    // Cartesian (3q,3q^2,z) makes both surfaces regular along the seam.
    let make = |degree, xy: &[[f64; 2]], z| Surface {
        degree_u: degree,
        degree_v: 2,
        knots_u: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: xy
            .iter()
            .map(|p| {
                (0..3)
                    .map(|j| vec![p[0], p[1], z + 0.5 * j as f64])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; xy.len()],
        periodic_u: false,
        periodic_v: false,
    };
    // Its inverse (-1+sqrt(1+8q))/2 is represented implicitly, never rounded.
    let a = make(
        4,
        &[[0., 0.], [0.375, 0.], [1., 0.125], [1.875, 0.75], [3., 3.]],
        0.,
    );
    let b = make(2, &[[0., 0.], [1.5, 0.], [3., 3.]], 1.);
    let saved = (a.clone(), b.clone());
    let full = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(full.certified, "{full:?}");
    for budget in [0, full.work - 1] {
        assert!(
            !inferred(&a, &b, "vMax", "vMin", 2, 1., budget)
                .unwrap()
                .certified
        );
    }
    let mut wrong = b.clone();
    wrong.control_points[1][0][1] = f64::from_bits(1);
    assert!(
        !inferred(&a, &wrong, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut curvature = b.clone();
    for row in &mut curvature.control_points {
        row[2][1] += 0.125;
    }
    assert!(
        inferred(&a, &curvature, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    assert!(
        !inferred(&a, &curvature, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    assert_eq!((a, b), saved);
}

#[test]
fn implicit_algebraic_clocks_cross_both_original_partitions_without_rational_inverse() {
    let make = |degree, xy: &[[f64; 2]], z, dz| Surface {
        degree_u: degree,
        degree_v: 2,
        knots_u: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: xy
            .iter()
            .map(|p| {
                (0..3)
                    .map(|j| vec![p[0], p[1], z + dz * j as f64])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; xy.len()],
        periodic_u: false,
        periodic_v: false,
    };
    // (60q,60q²,z), q_A=(s+s²)/2, q_B=(t+t³)/2. The unique map
    // solves s+s²=t+t³ and is not rational in either direction.
    let a = split_original_u(
        &make(
            4,
            &[[0., 0.], [7.5, 0.], [20., 2.5], [37.5, 15.], [60., 60.]],
            0.,
            0.5,
        ),
        0.5,
    );
    let b = split_original_u(
        &make(
            6,
            &[
                [0., 0.],
                [5., 0.],
                [10., 1.],
                [16.5, 3.],
                [26., 8.],
                [40., 20.],
                [60., 60.],
            ],
            1.,
            1.,
        ),
        0.5,
    );
    let saved = (a.clone(), b.clone());
    let full = inferred(&a, &b, "vMax", "vMin", 2, 2., 1000000).unwrap();
    assert!(full.certified, "{full:?}");
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 2., full.work - 1)
            .unwrap()
            .certified
    );
    let mut changed = b.clone();
    changed.control_points[2][0][1] = 1f64.next_up();
    assert!(
        !inferred(&a, &changed, "vMax", "vMin", 2, 2., 1000000)
            .unwrap()
            .certified
    );
    let mut curved = b.clone();
    for row in &mut curved.control_points {
        row[2][1] += 0.125;
    }
    assert!(
        inferred(&a, &curved, "vMax", "vMin", 1, 2., 1000000)
            .unwrap()
            .certified
    );
    assert!(
        !inferred(&a, &curved, "vMax", "vMin", 2, 2., 1000000)
            .unwrap()
            .certified
    );
    let mut reversed = b.clone();
    reversed.control_points.reverse();
    reversed.weights.reverse();
    assert!(
        !inferred(&a, &reversed, "vMax", "vMin", 2, 2., 1000000)
            .unwrap()
            .certified
    );
    assert_eq!((a, b), saved);
}

#[path = "../../../../tests/support/branched_seam_map.rs"]
mod branched_fixture;
#[test]
fn branched_algebraic_clock_changes_axes_and_keeps_one_original_cover() {
    let (a, b) = branched_fixture::fixture(1);
    let saved = (a.clone(), b.clone());
    let full = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(full.certified, "{full:?}");
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 1., full.work - 1)
            .unwrap()
            .certified
    );
    let (_, twice) = branched_fixture::fixture(2);
    assert!(
        !inferred(&a, &twice, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut changed = b.clone();
    changed.control_points[8][0][0] = changed.control_points[8][0][0].next_up();
    assert!(
        !inferred(&a, &changed, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut reversed = b.clone();
    reversed.control_points.reverse();
    reversed.weights.reverse();
    assert!(
        !inferred(&a, &reversed, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut curvature = b.clone();
    for row in &mut curvature.control_points {
        row[2][2] += 0.125;
        row[2][0] += 0.125;
    }
    assert!(
        inferred(&a, &curvature, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    assert!(
        !inferred(&a, &curvature, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    assert_eq!((a, b), saved);
}

#[test]
fn independently_cut_algebraic_branches_need_no_rounded_inverse_knots() {
    let (a, b) = branched_fixture::independently_cut();
    let saved = (a.clone(), b.clone());
    let proof = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 1., proof.work - 1)
            .unwrap()
            .certified
    );
    let mut changed = b.clone();
    changed.control_points[2][0][0] = changed.control_points[2][0][0].next_up();
    assert!(
        !inferred(&a, &changed, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut reversed = b.clone();
    reversed.control_points.reverse();
    reversed.weights.reverse();
    assert!(
        !inferred(&a, &reversed, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut curvature = b.clone();
    for row in &mut curvature.control_points {
        row[2][0] += 0.125;
        row[2][2] += 0.125;
    }
    assert!(
        inferred(&a, &curvature, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    assert!(
        !inferred(&a, &curvature, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    assert_eq!((a, b), saved);
}

#[path = "../../../../tests/support/overlap_seam_map.rs"]
mod overlap_fixture;
#[test]
fn implicit_chart_overlap_owns_algebraic_switches_without_common_endpoints() {
    let (a, b) = overlap_fixture::fixture();
    let saved = (a.clone(), b.clone());
    let full = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(full.certified, "{full:?}");
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 1., full.work - 1)
            .unwrap()
            .certified
    );
    let mut changed = b.clone();
    changed.control_points[2][0][0] = changed.control_points[2][0][0].next_up();
    assert!(
        !inferred(&a, &changed, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut reverse = b.clone();
    reverse.control_points.reverse();
    reverse.weights.reverse();
    reverse.knots_u = reverse.knots_u.iter().rev().map(|k| 1. - k).collect();
    assert!(
        !inferred(&a, &reverse, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut curvature = b.clone();
    for row in &mut curvature.control_points {
        row[2][0] += 0.125;
        row[2][2] += 0.125;
    }
    assert!(
        inferred(&a, &curvature, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    assert!(
        !inferred(&a, &curvature, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    assert_eq!((a, b), saved);
}

#[path = "../../../../tests/support/ramified_seam_map.rs"]
mod ramified_fixture;
#[test]
fn ramified_projection_keeps_regular_g2_but_physical_rank_loss_refuses() {
    let (a, b) = ramified_fixture::fixture();
    let saved = (a.clone(), b.clone());
    let full = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(full.certified, "{full:?}");
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 1., full.work - 1)
            .unwrap()
            .certified
    );
    let mut changed = b.clone();
    changed.control_points[2][0][0] = changed.control_points[2][0][0].next_up();
    assert!(
        !inferred(&a, &changed, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut curvature = b.clone();
    for row in &mut curvature.control_points {
        row[2][0] += 0.125;
        row[2][2] += 0.125;
    }
    assert!(
        inferred(&a, &curvature, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    assert!(
        !inferred(&a, &curvature, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let (c, d) = ramified_fixture::rank_loss();
    let refused = inferred(&c, &d, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(
        !refused.certified && !refused.regularity_certified,
        "{refused:?}"
    );
    assert!(
        refused.exact_identity,
        "rank loss must refuse even after jet identity: {refused:?}"
    );
    assert_eq!((a, b), saved);
}

#[path = "../../../../tests/support/circuit_seam_map.rs"]
mod circuit_fixture;
#[test]
fn rational_critical_projection_keeps_original_weights_and_regular_g2() {
    let (a, b) = circuit_fixture::fixture();
    let saved = (a.clone(), b.clone());
    let full = inferred(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(full.certified, "{full:?}");
    assert!(
        !inferred(&a, &b, "vMax", "vMin", 2, 1., full.work - 1)
            .unwrap()
            .certified
    );
    let mut bad = b.clone();
    bad.control_points[2][0][0] = bad.control_points[2][0][0].next_up();
    assert!(
        !inferred(&a, &bad, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut curvature = b.clone();
    for row in &mut curvature.control_points {
        row[2][0] += 0.125;
        row[2][2] += 0.125;
    }
    assert!(
        inferred(&a, &curvature, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    assert!(
        !inferred(&a, &curvature, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    assert_eq!((a, b), saved);
}
