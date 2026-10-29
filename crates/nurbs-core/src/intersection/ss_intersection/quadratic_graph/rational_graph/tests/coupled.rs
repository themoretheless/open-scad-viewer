//! Nonseparable original weights, independently authored exact dyadic poles.
use super::*;
fn patch() -> Surface {
    // W=1+12u(1-u)+12v(1-v); XYZ=(315u,315v,
    // 1890*((u-.5)^2+(v-.5)^2-1/16)). Static poles were obtained by
    // exact rational Bernstein products, then checked for binary64 equality.
    let poles = [
        [
            [0.0, 0.0, 826.875],
            [0.0, 19.6875, 708.75],
            [0.0, 157.5, -55.125],
            [0.0, 295.3125, 708.75],
            [0.0, 315.0, 826.875],
        ],
        [
            [19.6875, 0.0, 708.75],
            [45.0, 45.0, 286.875],
            [49.21875, 157.5, -255.9375],
            [45.0, 270.0, 286.875],
            [19.6875, 315.0, 708.75],
        ],
        [
            [157.5, 0.0, -55.125],
            [157.5, 49.21875, -255.9375],
            [157.5, 157.5, -713.125],
            [157.5, 265.78125, -255.9375],
            [157.5, 315.0, -55.125],
        ],
        [
            [295.3125, 0.0, 708.75],
            [270.0, 45.0, 286.875],
            [265.78125, 157.5, -255.9375],
            [270.0, 270.0, 286.875],
            [295.3125, 315.0, 708.75],
        ],
        [
            [315.0, 0.0, 826.875],
            [315.0, 19.6875, 708.75],
            [315.0, 157.5, -55.125],
            [315.0, 295.3125, 708.75],
            [315.0, 315.0, 826.875],
        ],
    ];
    let weights = [
        [1.0, 4.0, 5.0, 4.0, 1.0],
        [4.0, 7.0, 8.0, 7.0, 4.0],
        [5.0, 8.0, 9.0, 8.0, 5.0],
        [4.0, 7.0, 8.0, 7.0, 4.0],
        [1.0, 4.0, 5.0, 4.0, 1.0],
    ];
    Surface {
        degree_u: 4,
        degree_v: 4,
        knots_u: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: poles
            .into_iter()
            .map(|r| r.into_iter().map(|p| p.to_vec()).collect())
            .collect(),
        weights: weights.into_iter().map(|r| r.to_vec()).collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn coupled_original_denominators_prove_full_rectangle_identity_and_closed_lifts() {
    let a = patch();
    let mut b = a.clone();
    assert_ne!(
        a.weights[1][1] * a.weights[0][0],
        a.weights[1][0] * a.weights[0][1]
    );
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 0.;
    }
    let source = FiberSources::new(&a, &b).unwrap();
    let owner = qualify_quadratic_graph_ss(&source, limits()).unwrap();
    assert_eq!(owner.kind(), QuadraticGraphSSKind::ClosedEllipse);
    assert!(owner.operations() <= 4096);
    assert_eq!(
        owner.data.heights[0],
        [[826.875, -1890., 1890.], [-1890., 0., 0.], [1890., 0., 0.]]
    );
    assert_eq!(owner.data.heights[1], [[0.; 3]; 3]);
    for u in [0., 0.125, 0.375, 0.625, 0.875, 1.] {
        for v in [0., 0.25, 0.5, 0.75, 1.] {
            let p = a.evaluate(u, v).unwrap().point;
            assert!((p[0] - 315. * u).abs() < 1e-10 && (p[1] - 315. * v).abs() < 1e-10);
            assert!((p[2] - 1890. * ((u - 0.5).powi(2) + (v - 0.5).powi(2) - 0.0625)).abs() < 1e-9);
        }
    }
    construct_graph_curves(
        &owner,
        GraphCurveTolerances {
            world: 1e-7,
            uv: 1e-9,
        },
        Default::default(),
    )
    .unwrap()
    .reverify()
    .unwrap();
    owner.reverify().unwrap();
    let exact = QuadraticGraphSSLimits {
        max_operations: owner.operations(),
    };
    assert!(qualify_quadratic_graph_ss(&source, exact).is_ok());
    assert!(
        qualify_quadratic_graph_ss(
            &source,
            QuadraticGraphSSLimits {
                max_operations: exact.max_operations - 1
            }
        )
        .is_err()
    );
    let mut changed = a.clone();
    changed.weights[1][1] = changed.weights[1][1].next_up();
    assert!(
        qualify_quadratic_graph_ss(&FiberSources::new(&changed, &b).unwrap(), limits()).is_err()
    );
    assert_eq!(owner.source().first(), &a);
}
