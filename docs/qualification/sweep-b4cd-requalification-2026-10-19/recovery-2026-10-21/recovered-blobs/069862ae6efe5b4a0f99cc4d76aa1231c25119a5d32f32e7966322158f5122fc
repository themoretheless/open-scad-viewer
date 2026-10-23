use nurbs_core::{
    curve::Curve,
    curve_surface_agreement::{Status, lift, verify},
    surface::{self, Surface},
};
fn tube(periodic: bool) -> Surface {
    let c = if periodic {
        Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        }
    } else {
        Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap()
    };
    surface::extrude(&c, [0., 0., 2.]).unwrap()
}
fn map(u: f64, v: f64) -> [f64; 3] {
    if u <= 1. {
        [u, 0., 2. * v]
    } else if u <= 2. {
        [1., u - 1., 2. * v]
    } else if u <= 3. {
        [3. - u, 1., 2. * v]
    } else {
        [0., 4. - u, 2. * v]
    }
}
#[test]
fn complete_multispan_pcurves_lift_across_original_support_knot_cells() {
    let pc = Curve::from_polyline(vec![
        vec![0.5, 0.25],
        vec![1.5, 0.25],
        vec![1.5, 0.75],
        vec![3.5, 0.75],
    ])
    .unwrap();
    for periodic in [false, true] {
        let s = tube(periodic);
        let original = s.clone();
        let result = lift(&pc, &s, None, 1e-7, 100000).unwrap();
        let c = result
            .curve
            .unwrap_or_else(|| panic!("method {:?}, cells {}", result.method, result.cells));
        assert_eq!(s, original);
        assert_eq!(result.evidence.unwrap().status, Status::WithinTolerance);
        let (a, b) = (pc.domain(), c.domain());
        for i in 0..=192 {
            let t = i as f64 / 192.;
            let uv = pc.evaluate_de_boor(a[0] + (a[1] - a[0]) * t).unwrap();
            let q = c.evaluate_de_boor(b[0] + (b[1] - b[0]) * t).unwrap();
            let expected = map(uv[0], uv[1]);
            assert!(
                q.iter()
                    .zip(expected)
                    .map(|(x, y)| (x - y).powi(2))
                    .sum::<f64>()
                    .sqrt()
                    < 1e-7
            );
        }
        let mut changed = c.clone();
        changed.control_points[1][2] += 0.01;
        assert_ne!(
            verify(&changed, &pc, &s, false, 1e-7, 10000)
                .unwrap()
                .status,
            Status::WithinTolerance
        );
    }
}
#[test]
fn owned_endpoints_and_empty_work_do_not_inherit_a_lift_certificate() {
    let pc =
        Curve::from_polyline(vec![vec![0.5, 0.25], vec![0.75, 0.25], vec![0.75, 0.75]]).unwrap();
    let s = tube(true);
    assert!(lift(&pc, &s, None, 1e-7, 0).unwrap().curve.is_none());
    assert!(
        lift(
            &pc,
            &s,
            Some([[10., 10., 10.], [11., 11., 11.]]),
            1e-7,
            10000
        )
        .unwrap()
        .curve
        .is_none()
    );
}

#[test]
fn nonlinear_rational_pcurve_composes_on_an_original_rational_support_cell() {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 0., 2.]],
            vec![vec![1., 1., 0.], vec![1., 1., 2.]],
            vec![vec![0., 1., 0.], vec![0., 1., 2.]],
        ],
        weights: vec![vec![1., 1.], vec![w, w], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let pc = Curve {
        degree: 2,
        knots: vec![-2., -2., -2., 3., 3., 3.],
        control_points: vec![vec![0.1, 0.2], vec![0.4, 0.9], vec![0.8, 0.4]],
        weights: vec![1., 0.75, 1.],
        periodic: false,
    };
    let result = lift(&pc, &s, None, 1e-7, 100000).unwrap();
    let c = result.curve.unwrap();
    assert_eq!(c.degree, 6);
    for i in 0..=128 {
        let t = i as f64 / 128.;
        let uv = pc.evaluate_de_boor(-2. + 5. * t).unwrap();
        let u = uv[0];
        let b = [(1. - u).powi(2), 2. * u * (1. - u) * w, u * u];
        let den = b.iter().sum::<f64>();
        let q = [(b[0] + b[1]) / den, (b[1] + b[2]) / den, 2. * uv[1]];
        let actual = c.evaluate_de_boor(t).unwrap();
        assert!(
            actual
                .iter()
                .zip(q)
                .map(|(x, y)| (x - y).powi(2))
                .sum::<f64>()
                .sqrt()
                < 1e-7
        );
    }
    assert_eq!(
        verify(&c, &pc, &s, false, 1e-7, 1000).unwrap().status,
        Status::WithinTolerance
    );
}

#[test]
fn rational_quadratic_multispan_lift_keeps_original_clock_and_full_proof() {
    let pc = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 4., 4., 6., 6., 6.],
        control_points: vec![
            vec![0.125, 0.125],
            vec![0.25, 0.375],
            vec![0.5, 0.5],
            vec![0.75, 0.625],
            vec![0.875, 0.875],
        ],
        weights: vec![1., 2., 1., 3., 2.],
        periodic: false,
    };
    let support = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let original = pc.clone();
    let result = lift(&pc, &support, None, 1e-7, 100000).unwrap();
    let world = result
        .curve
        .unwrap_or_else(|| panic!("Unproven lift: {:?}", result.evidence));
    assert_eq!(pc, original);
    assert_eq!(world.domain(), pc.domain());
    assert!(world.degree > 1 && world.control_points.len() > world.degree + 1);
    assert_eq!(result.evidence.unwrap().status, Status::WithinTolerance);
    assert_eq!(
        verify(&world, &pc, &support, false, 1e-7, 100000)
            .unwrap()
            .status,
        Status::WithinTolerance
    );
    let mut damaged = world;
    damaged.control_points[2][2] += 0.01;
    assert_ne!(
        verify(&damaged, &pc, &support, false, 1e-7, 100000)
            .unwrap()
            .status,
        Status::WithinTolerance
    );
    assert!(lift(&pc, &support, None, 1e-7, 1).unwrap().curve.is_none());
}
