use super::*;
fn surface(p: usize, q: usize, scale: f64, weights: &[f64], _cubic: bool) -> Surface {
    Surface {
        degree_u: p,
        degree_v: q,
        knots_u: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
        knots_v: [vec![0.; q + 1], vec![1.; q + 1]].concat(),
        control_points: (0..=p)
            .map(|i| {
                (0..=q)
                    .map(|j| {
                        vec![
                            i as f64 / p as f64,
                            scale * j as f64 / q as f64,
                            if i == p { 1. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..=p).map(|i| vec![weights[i]; q + 1]).collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn boundary(s: &Surface) -> Curve {
    Curve {
        degree: s.degree_u,
        knots: s.knots_u.clone(),
        control_points: s.control_points.iter().map(|r| r[0].clone()).collect(),
        weights: s.weights.iter().map(|r| r[0]).collect(),
        periodic: false,
    }
}
fn pc() -> Curve {
    Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap()
}
const T: [f64; 3] = [1e-7, 1e-5, 1e-4];
fn multispan_sources() -> (Surface, Surface) {
    use crate::{surface::Axis, surface_edit};
    let a = surface_edit::insert(&surface(2, 2, 1., &[1.; 3], false), Axis::U, 0.5, 2).unwrap();
    let b = surface_edit::insert(&surface(2, 2, 0.125, &[1.; 3], false), Axis::U, 0.25, 2).unwrap();
    let b = surface_edit::insert(&b, Axis::U, 0.75, 2).unwrap();
    (a, b)
}
#[test]
fn all_original_source_cells_cover_different_candidate_knots() {
    let (a, b) = multispan_sources();
    let saved = [a.clone(), b.clone()];
    let c = boundary(&a);
    let p = pc();
    let cover = prepared::Prepared::new([&a, &b], [&p, &p], 1, [None, None],10000).unwrap();
    assert_eq!(cover.sources.len(), 2);
    assert_eq!(cover.segments.len(), 4);
    let r = inspect_extrusion_boundary(
        &c,
        [&p, &p],
        [&a, &b],
        1,
        [None, None],
        crate::surface_g2_pcurves::G2Budget {
            tolerances: T,
            max_cells: 10000,
            max_exact: 1000000,
        },
    )
        .unwrap();
    assert_eq!(
        r.decision,
        Decision::WithinTolerance,
        "normal={:?} tensor={:?} pending={}",
        r.normal,
        r.tensor,
        r.pending
    );
    assert!(r.positions.proven && r.regularity_proven && r.pending == 0 && r.cells <= 10000);
    let short = inspect_extrusion_boundary(
        &c,
        [&p, &p],
        [&a, &b],
        1,
        [None, None],
        crate::surface_g2_pcurves::G2Budget {
            tolerances: T,
            max_cells: r.cells - 1,
            max_exact: 1000000,
        },
    )
    .unwrap();
    assert_ne!(short.decision, Decision::WithinTolerance);
    assert!(short.cells <= r.cells - 1);
    assert_eq!([a, b], saved);
}
#[test]
fn nondyadic_source_knots_and_reversed_clocks_keep_complete_coverage() {
    use crate::{surface::Axis, surface_edit};
    let a = surface_edit::insert(&surface(2, 2, 1., &[1.; 3], false), Axis::U, 0.3, 2).unwrap();
    let (_, b) = multispan_sources();
    let c = boundary(&a).reverse().unwrap();
    let p = pc().reverse().unwrap();
    let r = inspect_extrusion_boundary(
        &c,
        [&p, &p],
        [&a, &b],
        1,
        [None, None],
        crate::surface_g2_pcurves::G2Budget {
            tolerances: T,
            max_cells: 10000,
            max_exact: 1000000,
        },
    )
        .unwrap();
    assert_eq!(
        r.decision,
        Decision::WithinTolerance,
        "normal={:?} tensor={:?}",
        r.normal,
        r.tensor
    );
    assert!(r.positions.proven && r.regularity_proven && r.pending == 0);
}
#[test]
fn shared_knot_endpoint_bands_keep_original_nonzero_derivatives() {
    use crate::{surface::Axis, surface_edit};
    let (a, b) = multispan_sources();
    let b = surface_edit::insert(&b, Axis::U, 0.5, 2).unwrap();
    let c = boundary(&a);
    let p = pc();
    let r = inspect_extrusion_boundary(
        &c,
        [&p, &p],
        [&a, &b],
        1,
        [None, None],
        crate::surface_g2_pcurves::G2Budget {
            tolerances: T,
            max_cells: 10000,
            max_exact: 1000000,
        },
    )
        .unwrap();
    assert_eq!(
        r.decision,
        Decision::WithinTolerance,
        "profile failures={} source failures={} pending={}",
        r.profile_unresolved_queries,
        r.source_unresolved_queries,
        r.pending
    );
    assert!(r.regularity_proven && r.pending == 0);
    let cover = source::Source::cover(&a, &p, [0.; 3]).unwrap();
    let ends = [I::point(0.), I::point(1.)];
    assert!(cover[0].contains_point(ends, 0.25).unwrap());
    assert!(!cover[0].contains_point(ends, 0.75).unwrap());
    assert!(cover[1].contains_point(ends, 0.75).unwrap());
}
#[test]
fn only_literal_products_can_remove_the_stationary_trace_parameter() {
    let (mut a, _) = multispan_sources();
    let p = pc();
    {
        let source = source::Source::cover(&a, &p, [0.; 3]).unwrap();
        assert_eq!(source[0].trace[0].len(), 5);
        assert_eq!(source[0].projected_trace(1).unwrap()[0].len(), 3);
    }
    a.control_points[0][1][2] = f64::from_bits(1);
    let source = source::Source::cover(&a, &p, [0.; 3]).unwrap();
    assert_eq!(source[0].projected_trace(1).unwrap()[0].len(), 5);
}
#[test]
fn a_hidden_second_jet_in_a_later_original_cell_is_not_skipped() {
    let (mut a, b) = multispan_sources();
    let c = boundary(&a);
    let p = pc();
    let last = a.control_points.len() - 1;
    a.control_points[last][2][2] += 0.1;
    let r = inspect_extrusion_boundary(
        &c,
        [&p, &p],
        [&a, &b],
        1,
        [None, None],
        crate::surface_g2_pcurves::G2Budget {
            tolerances: T,
            max_cells: 10000,
            max_exact: 1000000,
        },
    )
        .unwrap();
    assert!(r.positions.proven);
    assert_ne!(r.decision, Decision::WithinTolerance);
    let empty =
        inspect_extrusion_boundary(
            &c,
            [&p, &p],
            [&a, &b],
            1,
            [None, None],
            crate::surface_g2_pcurves::G2Budget {
                tolerances: T,
                max_cells: 0,
                max_exact: 0,
            },
        ).unwrap();
    assert_ne!(empty.decision, Decision::WithinTolerance);
    assert!(!empty.regularity_proven);
}
#[test]
fn correlated_original_rational_profiles_and_common_weight_gauges() {
    for gauge in [1e-12, 1., 5e11] {
        let weights = [gauge, 2. * gauge, gauge];
        let a = surface(2, 2, 1., &weights, false);
        let b = surface(2, 2, 0.125, &weights, false);
        let saved = [a.clone(), b.clone()];
        let curve = boundary(&a);
        let pc = pc();
        let r = inspect_extrusion_boundary(
            &curve,
            [&pc, &pc],
            [&a, &b],
            1,
            [None, None],
            crate::surface_g2_pcurves::G2Budget {
                tolerances: T,
                max_cells: 3000,
                max_exact: 100000,
            },
        )
        .unwrap();
        assert_eq!(
            r.decision,
            Decision::WithinTolerance,
            "gauge={gauge}: normal={:?} tensor={:?}",
            r.normal,
            r.tensor
        );
        assert!(r.positions.proven && r.regularity_proven && r.pending == 0 && r.cells <= 3000);
        assert_eq!([a, b], saved);
    }
    let invalid = surface(2, 2, 1., &[1e12, 2e12, 1e12], false);
    assert!(invalid.validate().is_err());
}
#[test]
fn real_source_second_jet_defect_is_not_replaced_by_an_ideal_cylinder() {
    let mut a = surface(2, 2, 1., &[1.; 3], false);
    let b = surface(2, 2, 0.125, &[1.; 3], false);
    for row in &mut a.control_points {
        row[2][2] += 0.01;
    }
    let curve = boundary(&a);
    let pc = pc();
    let r = inspect_extrusion_boundary(
        &curve,
        [&pc, &pc],
        [&a, &b],
        1,
        [None, None],
        crate::surface_g2_pcurves::G2Budget {
            tolerances: T,
            max_cells: 1000,
            max_exact: 100000,
        },
    )
    .unwrap();
    assert!(r.positions.proven);
    assert_ne!(r.decision, Decision::WithinTolerance);
    assert!(r.cells <= 1000);
}
#[test]
fn literal_candidate_two_jet_recognition_refuses_even_subnormal_drift() {
    let a = surface(2, 2, 1., &[1.; 3], false);
    let mut b = surface(2, 2, 0.125, &[1.; 3], false);
    let curve = boundary(&a);
    let pc = pc();
    b.control_points[0][2][2] = 0_f64.next_up();
    assert!(
        inspect_extrusion_boundary(
            &curve,
            [&pc, &pc],
            [&a, &b],
            1,
            [None, None],
            crate::surface_g2_pcurves::G2Budget {
                tolerances: T,
                max_cells: 1000,
                max_exact: 100000,
            },
        )
        .is_err()
    );
}
#[test]
fn collapsed_endpoint_requires_the_same_original_pole_certificate() {
    let a = surface(3, 2, 1., &[1.; 4], true);
    let mut b = surface(3, 2, 1., &[1.; 4], true);
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for point in row {
            point[1] *= i as f64 / 3.;
        }
    }
    let curve = boundary(&a);
    let pc = pc();
    let pole =
        crate::surface_shape_operator::inspect_original_flat_pole(&b, false, 2, 1. / 64., 1000)
            .unwrap();
    let cert = pole.certificate.as_ref().expect("owned pole quotient");
    let r = inspect_extrusion_boundary(
        &curve,
        [&pc, &pc],
        [&a, &b],
        1,
        [Some(cert), None],
        crate::surface_g2_pcurves::G2Budget {
            tolerances: T,
            max_cells: 3000,
            max_exact: 100000,
        },
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance);
    assert!(
        inspect_extrusion_boundary(
            &curve,
            [&pc, &pc],
            [&a, &b],
            1,
            [None, None],
            crate::surface_g2_pcurves::G2Budget {
                tolerances: T,
                max_cells: 1000,
                max_exact: 100000,
            },
        )
        .is_err()
    );
    let other = b.clone();
    assert!(
        inspect_extrusion_boundary(
            &curve,
            [&pc, &pc],
            [&a, &other],
            1,
            [Some(cert), None],
            crate::surface_g2_pcurves::G2Budget {
                tolerances: T,
                max_cells: 1000,
                max_exact: 100000,
            },
        )
        .is_err()
    );
}
#[test]
fn planar_collars_prove_zero_tensor_and_refuse_a_hidden_second_jet() {
    let mut a = surface(2, 2, 1., &[1.; 3], false);
    for row in &mut a.control_points {
        for point in row {
            point[2] = 0.;
        }
    }
    let mut b = a.clone();
    for row in &mut b.control_points {
        for point in row {
            point[1] *= 0.125;
        }
    }
    let curve = boundary(&a);
    let pc = pc();
    let r = inspect_planar_boundary(
        &curve,
        [&pc, &pc],
        [&a, &b],
        2,
        [None, None],
        crate::surface_g2_pcurves::G2Budget {
            tolerances: T,
            max_cells: 1000,
            max_exact: 100000,
        },
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance);
    assert_eq!(r.tensor, [0., 0.]);
    b.control_points[1][2][2] = 0_f64.next_up();
    assert!(
        inspect_planar_boundary(
            &curve,
            [&pc, &pc],
            [&a, &b],
            2,
            [None, None],
            crate::surface_g2_pcurves::G2Budget {
                tolerances: T,
                max_cells: 1000,
                max_exact: 100000,
            },
        )
        .is_err()
    );
}
#[test]
fn no_query_budget_does_not_grant_exterior_g2() {
    let a = surface(2, 2, 1., &[1.; 3], false);
    let b = surface(2, 2, 0.125, &[1.; 3], false);
    let curve = boundary(&a);
    let pc = pc();
    let r =
        inspect_extrusion_boundary(
            &curve,
            [&pc, &pc],
            [&a, &b],
            1,
            [None, None],
            crate::surface_g2_pcurves::G2Budget {
                tolerances: T,
                max_cells: 0,
                max_exact: 0,
            },
        ).unwrap();
    assert_eq!(r.decision, Decision::Unresolved);
    assert_eq!(r.cells, 0);
    assert!(r.pending > 0);
    assert!(
        inspect_extrusion_boundary(
            &curve,
            [&pc, &pc],
            [&a, &b],
            1,
            [None, None],
            crate::surface_g2_pcurves::G2Budget {
                tolerances: T,
                max_cells: 100001,
                max_exact: 0,
            },
        )
            .is_err()
    );
}

#[test]
fn reversed_original_clocks_keep_the_same_oriented_surface_fields() {
    let a = surface(2, 2, 1., &[1., 2., 1.], false);
    let b = surface(2, 2, 0.125, &[1., 2., 1.], false);
    let curve = boundary(&a).reverse().unwrap();
    let pc = pc().reverse().unwrap();
    let r = inspect_extrusion_boundary(
        &curve,
        [&pc, &pc],
        [&a, &b],
        1,
        [None, None],
        crate::surface_g2_pcurves::G2Budget {
            tolerances: T,
            max_cells: 3000,
            max_exact: 100000,
        },
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance);
    assert_eq!(r.orientation, Orientation::Same);
}
