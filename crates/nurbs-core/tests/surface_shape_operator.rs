use nurbs_core::{
    curve_differential::Side,
    surface::{Axis, Surface},
    surface_differential::Status,
    surface_edit, surface_shape_operator,
};
fn graph(a: f64, b: f64) -> Surface {
    let x = [-1., 0., 1.];
    let square = [1., -1., 1.];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![2., 2., 2., 7., 7., 7.],
        knots_v: vec![-3., -3., -3., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![x[i], x[j], a * square[i] + b * square[j]])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn verify(s: &Surface, expected: [f64; 3]) {
    let r = surface_shape_operator::at(s, [4.5, -1.], [Side::Automatic; 2]).unwrap();
    assert_eq!(r.status, Status::Available);
    let m = r.tensor.unwrap();
    for a in 0..3 {
        for b in 0..3 {
            let value = if a == b { expected[a] } else { 0. };
            assert!(m[a][b][0] <= value && value <= m[a][b][1]);
            assert!(m[a][b][1] - m[a][b][0] < 1e-8);
        }
    }
}
#[test]
fn principal_directions_matter_even_with_identical_principal_values() {
    let s = graph(1., 2.);
    let before = s.clone();
    verify(&s, [2., 4., 0.]);
    verify(&graph(2., 1.), [4., 2., 0.]);
    verify(&surface_edit::reverse(&s, Axis::U).unwrap(), [-2., -4., 0.]);
    assert_eq!(s, before);
}
#[test]
fn planes_singularities_knot_sides_and_invalid_queries_are_explicit() {
    verify(&graph(0., 0.), [0.; 3]);
    let s = graph(1., 2.);
    assert!(surface_shape_operator::at(&s, [f64::NAN, 0.], [Side::Automatic; 2]).is_err());
    let refined = surface_edit::insert(&s, Axis::U, 4.5, 1).unwrap();
    assert_eq!(
        surface_shape_operator::at(&refined, [4.5, -1.], [Side::Automatic; 2])
            .unwrap()
            .status,
        Status::ContinuityNotProven
    );
    for side in [Side::Left, Side::Right] {
        let r = surface_shape_operator::at(&refined, [4.5, -1.], [side, Side::Automatic]).unwrap();
        assert_eq!(r.status, Status::Available);
        let m = r.tensor.unwrap();
        assert!(m[0][0][0] <= 2. && m[0][0][1] >= 2.);
    }
    let mut collapsed = s;
    for row in &mut collapsed.control_points {
        for p in row {
            p[1] = 0.;
        }
    }
    let r = surface_shape_operator::at(&collapsed, [4.5, -1.], [Side::Automatic; 2]).unwrap();
    assert_eq!(r.status, Status::RegularityNotProven);
    assert!(r.tensor.is_none());
}

#[test]
fn rational_cylinder_tensor_matches_independent_quotient_derivatives() {
    let w = 0.5_f64.sqrt();
    let c = nurbs_core::curve::Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![vec![2., 0., 0.], vec![2., 2., 0.], vec![0., 2., 0.]],
        weights: vec![1., w, 1.],
        periodic: false,
    };
    let s = nurbs_core::surface::extrude(&c, [0., 0., 3.]).unwrap();
    let before = s.clone();
    for t in [0., 0.125, 0.5, 0.875, 1.] {
        let denominator = 1. + (2. * w - 2.) * t + (2. - 2. * w) * t * t;
        let d1 = 2. * w - 2. + 2. * (2. - 2. * w) * t;
        let d2 = 2. * (2. - 2. * w);
        let derivative = |coeff: [f64; 3]| {
            let n = coeff[0] + coeff[1] * t + coeff[2] * t * t;
            let n1 = coeff[1] + 2. * coeff[2] * t;
            let n2 = 2. * coeff[2];
            [
                (n1 * denominator - n * d1) / denominator.powi(2),
                n2 / denominator
                    - n * d2 / denominator.powi(2)
                    - 2. * n1 * d1 / denominator.powi(2)
                    + 2. * n * d1 * d1 / denominator.powi(3),
            ]
        };
        let x = derivative([2., 2. * (-2. + 2. * w), 2. * (1. - 2. * w)]);
        let y = derivative([0., 4. * w, 2. * (1. - 2. * w)]);
        let speed = x[0].hypot(y[0]);
        let tangent = [x[0] / speed, y[0] / speed, 0.];
        let curvature = -(x[0] * y[1] - y[0] * x[1]) / speed.powi(3);
        let r = surface_shape_operator::at(&s, [2. + 5. * t, 0.375], [Side::Automatic; 2]).unwrap();
        assert_eq!(r.status, Status::Available);
        let tensor = r.tensor.unwrap();
        for a in 0..3 {
            for b in 0..3 {
                let expected = curvature * tangent[a] * tangent[b];
                let interval = tensor[a][b];
                assert!(
                    interval[0] - 1e-12 <= expected && expected <= interval[1] + 1e-12,
                    "t={t}, [{a},{b}], {interval:?}, expected={expected}"
                );
                assert!(interval[1] - interval[0] < 1e-8);
            }
        }
        // The tangent direction rotates, giving nonzero off-diagonal terms.
        if t == 0.5 {
            assert!(tensor[0][1][0] > 0.249999);
        }
    }
    assert_eq!(s, before);
}

#[test]
fn boundary_tensor_bounds_cover_an_independent_curved_edge_formula() {
    use nurbs_core::surface_join::Boundary;
    let s = graph(1., 2.);
    let proof =
        surface_shape_operator::boundary_bounds(&s, Boundary::UMin, [0.4, 0.6], false).unwrap();
    assert_eq!(proof.status, Status::Available);
    let tensor = proof.tensor.unwrap();
    // z=x²+2y² along x=-1. Explicit world-tensor numerators over D^(5/2).
    for i in 0..=100 {
        let y = -0.2 + 0.4 * i as f64 / 100.;
        let d = 5. + 16. * y * y;
        let denominator = d * d * d.sqrt();
        let m = [
            [
                2. * (1. + 16. * y * y).powi(2) + 256. * y * y,
                176. * y + 256. * y.powi(3),
                -4. + 64. * y * y,
            ],
            [176. * y + 256. * y.powi(3), 100. + 128. * y * y, 48. * y],
            [-4. + 64. * y * y, 48. * y, 8. + 64. * y * y],
        ];
        for a in 0..3 {
            for b in 0..3 {
                let value = m[a][b] / denominator;
                assert!(tensor[a][b][0] - 1e-12 <= value && value <= tensor[a][b][1] + 1e-12);
            }
        }
    }
    let refined = surface_edit::insert(&s, Axis::V, -1., 1).unwrap();
    assert_eq!(
        surface_shape_operator::boundary_bounds(&refined, Boundary::UMin, [0., 1.], false)
            .unwrap()
            .status,
        Status::ContinuityNotProven
    );
    assert_eq!(
        surface_shape_operator::boundary_bounds(&refined, Boundary::UMin, [0., 0.1], false)
            .unwrap()
            .status,
        Status::Available
    );
    assert!(
        surface_shape_operator::boundary_bounds(&s, Boundary::UMin, [0.7, 0.2], false).is_err()
    );
}
