use super::*;
#[test]
fn closed_plane_first_bound_owns_zero_holonomy_and_retained_wall() {
    let path = crate::paths::closed_round_polyline(
        &[[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]],
        2.,
    )
    .unwrap();
    let profile = crate::primitives::line([0., 8., 0.25], [0., 8., 0.5]).unwrap();
    let scale = super::super::constant_vector_law([1., 0., 0.]).unwrap();
    let twist = super::super::constant_vector_law([0.; 3]).unwrap();
    let sweep = Sweep::new(
        &profile,
        &path,
        &scale,
        &twist,
        super::super::Options {
            normal: [0., 0., 1.],
            orientation: super::super::Orientation::RotationMinimizing,
            spacing: super::super::Spacing::Parameter,
            initial_sections: 5,
            max_sections: 257,
            max_deviation: 0.01,
        },
    )
    .unwrap();
    let bound = sweep
        .rmf_spatial_patch_error_bound(257, 512, 10000, 1000000)
        .unwrap();
    assert_eq!(bound.status, Status::Certified, "{bound:?}");
    assert!(bound.within_budget, "{bound:?}");
    let reference = |t: f64| {
        let u = t;
        let span = (2..path.control_points.len())
            .find(|&i| {
                path.knots[i] < path.knots[i + 1] && u >= path.knots[i] && u <= path.knots[i + 1]
            })
            .unwrap();
        let f = (u - path.knots[span]) / (path.knots[span + 1] - path.knots[span]);
        let b = [(1. - f).powi(2), 2. * f * (1. - f), f * f];
        let db = [-2. * (1. - f), 2. - 4. * f, 2. * f];
        let weights = &path.weights[span - 2..=span];
        let poles = &path.control_points[span - 2..=span];
        let w = (0..3).map(|j| b[j] * weights[j]).sum::<f64>();
        let dw = (0..3).map(|j| db[j] * weights[j]).sum::<f64>();
        let p = std::array::from_fn::<_, 3, _>(|k| {
            (0..3).map(|j| b[j] * weights[j] * poles[j][k]).sum::<f64>()
        });
        let d = std::array::from_fn::<_, 3, _>(|k| {
            (0..3)
                .map(|j| db[j] * weights[j] * poles[j][k])
                .sum::<f64>()
                * w
                - p[k] * dw
        });
        let length = math_core::norm(d);
        (p.map(|x| x / w), d.map(|x| x / length))
    };
    let (origin, t0) = reference(0.);
    let b0 = [t0[1], -t0[0], 0.];
    for patch in bound.patches.as_ref().unwrap() {
        let a = patch.knots_v[patch.degree_v];
        let b = patch.knots_v[patch.control_points[0].len()];
        for f in [0., 0.25, 0.5, 0.75, 1.] {
            let t = a + (b - a) * f;
            let (p, tangent) = reference(t);
            let side = [tangent[1], -tangent[0], 0.];
            for q in [0., 0.5, 1.] {
                let initial = [0., 8., 0.25 + 0.25 * q];
                let delta = std::array::from_fn::<_, 3, _>(|k| initial[k] - origin[k]);
                let along = (0..3).map(|k| delta[k] * t0[k]).sum::<f64>();
                let across = (0..3).map(|k| delta[k] * b0[k]).sum::<f64>();
                let expected = std::array::from_fn::<_, 3, _>(|k| {
                    p[k] + along * tangent[k]
                        + across * side[k]
                        + if k == 2 { delta[2] } else { 0. }
                });
                let actual = patch.evaluate(q, t).unwrap().point;
                assert!(
                    math_core::norm(std::array::from_fn(|k| actual[k] - expected[k]))
                        <= bound.error_upper.unwrap()
                );
            }
        }
    }
    for budget in [0, bound.cells - 1] {
        let short = sweep
            .rmf_spatial_patch_error_bound(257, 512, budget, 1000000)
            .unwrap();
        assert!(!short.within_budget && short.cells <= budget);
    }
    eprintln!(
        "closed planar first upper={:?} work={}",
        bound.error_upper, bound.cells
    );
}

#[test]
fn corrected_circle_first_images_enclose_independent_rational_derivatives() {
    let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
    let profile = crate::primitives::line([0., 1., 0.], [0., 1., 1.]).unwrap();
    let scale = super::super::constant_vector_law([1., 0., 0.]).unwrap();
    let twist = super::super::constant_vector_law([0.; 3]).unwrap();
    let sweep = Sweep::new(
        &profile,
        &path,
        &scale,
        &twist,
        super::super::Options {
            normal: [1., 0., 1.],
            orientation: super::super::Orientation::RotationMinimizing,
            spacing: super::super::Spacing::Parameter,
            initial_sections: 5,
            max_sections: 257,
            max_deviation: 0.01,
        },
    )
    .unwrap();
    let proof =
        rmf_transport::certify_original_rmf_transport_shared(&path, [1., 0., 1.], 512, 10000, true)
            .unwrap();
    let (_, owner) = Owner::new(&sweep, 10000).unwrap();
    let owner = owner.unwrap();
    assert!(owner.constant_normal.is_none());
    for t in [0.03125, 0.3125, 0.5625, 0.8125] {
        let span = (2..path.control_points.len())
            .find(|&i| {
                path.knots[i] < path.knots[i + 1] && t >= path.knots[i] && t <= path.knots[i + 1]
            })
            .unwrap();
        let h = path.knots[span + 1] - path.knots[span];
        let f = (t - path.knots[span]) / h;
        let basis = [
            [(1. - f) * (1. - f), 2. * f * (1. - f), f * f],
            [-2. * (1. - f) / h, (2. - 4. * f) / h, 2. * f / h],
            [2. / (h * h), -4. / (h * h), 2. / (h * h)],
        ];
        let w: [f64; 3] = std::array::from_fn(|d| {
            (0..3)
                .map(|j| basis[d][j] * path.weights[span - 2 + j])
                .sum()
        });
        let x: [[f64; 3]; 3] = std::array::from_fn(|d| {
            std::array::from_fn(|k| {
                (0..3)
                    .map(|j| {
                        basis[d][j]
                            * path.weights[span - 2 + j]
                            * path.control_points[span - 2 + j][k]
                    })
                    .sum()
            })
        });
        let p = x[0].map(|v| v / w[0]);
        let v = std::array::from_fn::<_, 3, _>(|k| (x[1][k] - w[1] * p[k]) / w[0]);
        let a =
            std::array::from_fn::<_, 3, _>(|k| (x[2][k] - 2. * w[1] * v[k] - w[2] * p[k]) / w[0]);
        let speed = math_core::norm(v);
        let tangent = v.map(|x| x / speed);
        let dot = (0..3).map(|k| tangent[k] * a[k]).sum::<f64>();
        let td = std::array::from_fn::<_, 3, _>(|k| (a[k] - tangent[k] * dot) / speed);
        let r = 2_f64.sqrt();
        let n = [tangent[1] / r, -tangent[0] / r, 1. / r];
        let nd = [td[1] / r, -td[0] / r, 0.];
        let report = owner
            .query(&proof, &[[[1.; 2], [0.; 2], [0.; 2]]], [t, t], 10000)
            .unwrap();
        let pose = report.pose.unwrap().0;
        for k in 0..3 {
            assert!(pose[1].value[k].lo <= n[k] && pose[1].value[k].hi >= n[k]);
            assert!(pose[1].first[k].lo <= nd[k] && pose[1].first[k].hi >= nd[k]);
        }
        for budget in [0, report.cells - 1] {
            let short = owner
                .query(&proof, &[[[1.; 2], [0.; 2], [0.; 2]]], [t, t], budget)
                .unwrap();
            assert!(short.images.is_none() && short.cells <= budget);
        }
    }
}
