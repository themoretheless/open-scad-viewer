use nurbs_core::{
    curve::Curve,
    curve_surface_agreement,
    surface::{Axis, Surface},
    surface_collar, surface_g2_pcurves,
};
fn source() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                [0., 1.]
                    .iter()
                    .map(|&v| vec![i as f64 / 2., v, if i == 2 { 0.25 + v } else { v }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn trace() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.5], vec![0.5, 0.5], vec![1., 0.25]],
        weights: vec![1.; 3],
        periodic: false,
    }
}
#[test]
fn nonlinear_collar_has_whole_rounding_bound_and_original_boundary_g2() {
    let s = source();
    let pc = trace();
    let before = (s.clone(), pc.clone());
    let out = surface_collar::construct(&s, &pc, [0., 0.1], 1e-7, 2000000).unwrap();
    assert!(out.position_error_upper_mm <= 1e-7 && out.coefficient_work <= 2000000);
    for u in [0., 0.13, 0.5, 0.81, 1.] {
        for v in [0., 0.17, 0.5, 1.] {
            let p = out.surface.evaluate(u, v).unwrap().point;
            let target = [u, 0.5 - u * u / 4. + v * 0.1, 0.5 + v * 0.1];
            assert!(
                p.iter()
                    .zip(target)
                    .all(|(a, b)| (a - b).abs() <= out.position_error_upper_mm)
            );
        }
    }
    let edge = out.surface.iso(Axis::V, 0.).unwrap();
    let own = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let positions = curve_surface_agreement::verify(&edge, &pc, &s, false, 1e-7, 100000).unwrap();
    assert_eq!(
        positions.status,
        curve_surface_agreement::Status::WithinTolerance
    );
    let g2 = surface_g2_pcurves::inspect(
        &edge,
        [&pc, &own],
        [&s, &out.surface],
        [false; 2],
        1e-7,
        1e-5,
        1e-4,
        100000,
        1000000,
    )
    .unwrap();
    assert!(g2.positions.proven && g2.geometric.regularity_proven);
    assert_eq!(
        g2.geometric.decision,
        nurbs_core::surface_join::Decision::WithinTolerance,
        "{:?}",
        g2.geometric
    );
    let mut defect = out.surface.clone();
    for row in &mut defect.control_points {
        row[2][2] += 0.01;
    }
    let jets = surface_collar::inspect_jets(
        &s,
        &pc,
        [0., 0.1],
        &out.surface,
        [[0., 1.], [0., 0.]],
        2000000,
    )
    .unwrap();
    for order in 0..=2 {
        for a in 0..=order {
            assert!(jets.error_upper[a][order - a].unwrap() < 1e-8);
        }
    }
    let damaged =
        surface_collar::inspect_jets(&s, &pc, [0., 0.1], &defect, [[0., 1.], [0., 0.]], 2000000)
            .unwrap();
    assert!(damaged.error_upper[0][0].unwrap() < 1e-8 && damaged.error_upper[0][1].unwrap() < 1e-8);
    let refusal = surface_collar::inspect_g2(
        &s,
        &pc,
        [0., 0.1],
        &defect,
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
    )
    .unwrap();
    assert_eq!(
        refusal.decision,
        nurbs_core::surface_join::Decision::CorrespondenceExceedsTolerance
    );
    assert!(refusal.witness_parameter.is_some());
    assert!(
        damaged.error_upper[0][2].unwrap() >= 0.06 - 1e-10
            && damaged.error_upper[0][2].unwrap() < 0.0600001
    );
    let bad = surface_g2_pcurves::inspect(
        &edge,
        [&pc, &own],
        [&s, &defect],
        [false; 2],
        1e-7,
        1e-5,
        1e-4,
        100000,
        1000000,
    )
    .unwrap();
    assert!(bad.positions.proven);
    assert_ne!(
        bad.geometric.decision,
        nurbs_core::surface_join::Decision::WithinTolerance,
        "{:?}",
        g2.geometric
    );
    assert!(surface_collar::construct(&s, &pc, [0., 1.], 1e-7, 2000000).is_err());
    assert!(surface_collar::construct(&s, &pc, [0., 0.1], 1e-7, out.coefficient_work - 1).is_err());
    assert!(
        surface_collar::construct(
            &s,
            &pc,
            [0., 0.1],
            out.position_error_upper_mm * 0.5,
            2000000
        )
        .is_err()
    );
    assert_eq!((s, pc), before);
}
#[test]
fn rational_collar_qualifies_by_fresh_coefficients_while_generic_query_is_unresolved() {
    let mut s = source();
    s.weights[1] = vec![0.7; 2];
    let mut pc = trace();
    pc.control_points = vec![vec![0.2, 0.4], vec![0.4, 0.5], vec![0.7, 0.3]];
    pc.weights = vec![1., 0.7, 1.2];
    let saved = (s.clone(), pc.clone());
    let out = surface_collar::construct(&s, &pc, [0.05, 0.1], 1e-7, 2000000).unwrap();
    let jet_proof = surface_collar::inspect_jets(
        &s,
        &pc,
        [0.05, 0.1],
        &out.surface,
        [[0., 1.], [0., 0.]],
        2000000,
    )
    .unwrap();
    assert!(jet_proof.coefficient_work <= 2000000);
    let correlated = surface_collar::inspect_g2(
        &s,
        &pc,
        [0.05, 0.1],
        &out.surface,
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
    )
    .unwrap();
    assert_eq!(
        correlated.decision,
        nurbs_core::surface_join::Decision::WithinTolerance,
        "cells={} work={}",
        correlated.cells,
        correlated.coefficient_work
    );
    assert!(correlated.cells <= 100000 && correlated.coefficient_work <= 2000000);
    let upper = correlated.upper.unwrap();
    assert!(upper[0] <= 1e-7 && upper[1] <= 1e-5 && upper[2] <= 1e-4);
    for order in 0..=2 {
        for a in 0..=order {
            assert!(
                jet_proof.error_upper[a][order - a].unwrap() < 1e-6,
                "{:?}",
                jet_proof.error_upper
            );
        }
    }
    assert!(
        surface_collar::inspect_jets(
            &s,
            &pc,
            [0.05, 0.1],
            &out.surface,
            [[0., 1.], [0., 0.]],
            jet_proof.coefficient_work - 1
        )
        .is_err()
    );
    assert!(
        out.surface
            .weights
            .iter()
            .flatten()
            .any(|w| *w != out.surface.weights[0][0])
    );
    for u in [0., 0.13, 0.5, 0.81, 1.] {
        for v in [0., 0.17, 0.5, 1.] {
            let uv = pc.evaluate(u).unwrap().point;
            let expected = s.evaluate(uv[0] + v * 0.05, uv[1] + v * 0.1).unwrap().point;
            let actual = out.surface.evaluate(u, v).unwrap().point;
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() <= out.position_error_upper_mm)
            );
        }
    }
    let edge = out.surface.iso(Axis::V, 0.).unwrap();
    assert_eq!(
        curve_surface_agreement::verify(&edge, &pc, &s, false, 1e-7, 100000)
            .unwrap()
            .status,
        curve_surface_agreement::Status::WithinTolerance
    );
    let own = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let g2 = surface_g2_pcurves::inspect(
        &edge,
        [&pc, &own],
        [&s, &out.surface],
        [false; 2],
        1e-7,
        1e-5,
        1e-4,
        100000,
        1000000,
    )
    .unwrap();
    assert!(g2.positions.proven);
    assert_eq!(
        g2.geometric.decision,
        nurbs_core::surface_join::Decision::Unresolved,
        "{:?}",
        g2.geometric
    );
    assert_eq!(g2.geometric.cells, 100000);
    assert!(g2.geometric.regularity_proven);
    assert!(
        g2.geometric.branches[0].normal_upper.is_some()
            && g2.geometric.branches[0].tensor_upper.is_some()
    );
    let mut chart = s.clone();
    let mut chart_pc = pc.clone();
    for k in &mut chart.knots_u {
        *k = 2. + 2. * *k;
    }
    for k in &mut chart.knots_v {
        *k = -1. + 4. * *k;
    }
    for point in &mut chart_pc.control_points {
        point[0] = 2. + 2. * point[0];
        point[1] = -1. + 4. * point[1];
    }
    let scaled = surface_collar::construct(&chart, &chart_pc, [0.1, 0.4], 1e-7, 2000000).unwrap();
    for u in [0., 0.13, 0.5, 0.81, 1.] {
        for v in [0., 0.17, 0.5, 1.] {
            let a = out.surface.evaluate(u, v).unwrap().point;
            let b = scaled.surface.evaluate(u, v).unwrap().point;
            assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs()
                <= out.position_error_upper_mm + scaled.position_error_upper_mm));
        }
    }
    assert_eq!((s, pc), saved);
}
