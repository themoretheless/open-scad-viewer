use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
use nurbs_core::{ss_intersection as ss, surface::Surface};
use std::collections::BTreeMap;
fn choose(n: usize, k: usize) -> usize {
    (0..k).fold(1, |v, j| v * (n - j) / (j + 1))
}
fn cusp_pair(sign: f64) -> ss::FiberSources {
    let f = BTreeMap::from([
        ([3, 0], 1.),
        ([2, 0], -1.5),
        ([1, 0], 0.75),
        ([0, 0], -0.375),
        ([0, 1], 1.),
        ([0, 2], -1.),
    ]);
    let mut p = BTreeMap::<[usize; 2], Q>::new();
    for (a, x) in &f {
        for (b, y) in &f {
            *p.entry([a[0] + b[0], a[1] + b[1]]).or_default() +=
                Q::from_float(x * y * 720. * sign).unwrap();
        }
    }
    let mut a = Surface {
        degree_u: 6,
        degree_v: 4,
        knots_u: [vec![0.; 7], vec![1.; 7]].concat(),
        knots_v: [vec![0.; 5], vec![1.; 5]].concat(),
        control_points: vec![vec![vec![0.; 3]; 5]; 7],
        weights: vec![vec![1.; 5]; 7],
        periodic_u: false,
        periodic_v: false,
    };
    for i in 0..7 {
        for j in 0..5 {
            a.control_points[i][j] =
                vec![i as f64 / 2., j as f64 / 4., if i == 6 { 1. } else { 0. }];
        }
    }
    let mut b = a.clone();
    for i in 0..7 {
        for j in 0..5 {
            let mut z = Q::zero();
            for (e, v) in &p {
                if e[0] <= i && e[1] <= j {
                    z += v
                        * Q::new(choose(i, e[0]).into(), choose(6, e[0]).into())
                        * Q::new(choose(j, e[1]).into(), choose(4, e[1]).into());
                }
            }
            let value = z.to_f64().unwrap();
            assert_eq!(Q::from_float(value).unwrap(), z);
            b.control_points[i][j][2] += value;
        }
    }
    ss::FiberSources::new(&a, &b).unwrap()
}
#[test]
fn nonisolated_cusp_contact_is_a_complete_exact_square_identity() {
    let source = cusp_pair(1.);
    let receipt = ss::certify_polynomial_graph_order(&source, Default::default()).unwrap();
    assert!(receipt.has_global_square_identity());
    assert_eq!(receipt.cells(), 1);
    receipt.reverify().unwrap();
    assert_eq!(
        ss::prove_polynomial_graph_boundary_side(
            source.first(),
            source.second(),
            false,
            Default::default()
        )
        .unwrap(),
        1
    );
    ss::prove_graph_projection_inside(source.first(), source.second()).unwrap();
}
#[test]
fn negative_square_and_out_of_domain_boundary_refuse_material_order() {
    assert!(
        ss::certify_polynomial_graph_order(
            &cusp_pair(-1.),
            ss::GraphOrderLimits {
                max_cells: 128,
                max_depth: 8
            }
        )
        .is_err()
    );
    let source = cusp_pair(1.);
    let mut outside = source.second().clone();
    for p in outside.control_points.iter_mut().flatten() {
        p[0] += 0.125;
    }
    assert!(ss::prove_graph_projection_inside(source.first(), &outside).is_err());
    assert!(
        ss::certify_polynomial_graph_order(
            &source,
            ss::GraphOrderLimits {
                max_cells: 0,
                max_depth: 0
            }
        )
        .is_err()
    );
}

#[test]
fn a_vertical_first_carrier_cannot_receive_a_graph_barrier_order_receipt() {
    let source = cusp_pair(1.);
    let mut vertical = source.first().clone();
    for (i, row) in vertical.control_points.iter_mut().enumerate() {
        for p in row {
            p[0] = 0.;
            p[2] = -1000. + i as f64;
        }
    }
    let pair = ss::FiberSources::new(&vertical, source.second()).unwrap();
    assert!(ss::certify_polynomial_graph_order(&pair, Default::default()).is_err());
}
