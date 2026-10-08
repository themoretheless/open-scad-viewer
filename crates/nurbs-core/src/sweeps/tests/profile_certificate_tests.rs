    use super::*;
    use crate::sweeps::progressive_sweep::{
        Options, Orientation, Spacing, Sweep, constant_vector_law,
    };
    fn curve(p: &[[f64; 3]], w: &[f64], d: [f64; 2]) -> Curve {
        Curve {
            degree: p.len() - 1,
            knots: [vec![d[0]; p.len()], vec![d[1]; p.len()]].concat(),
            control_points: p.iter().map(|p| p.to_vec()).collect(),
            weights: w.to_vec(),
            periodic: false,
        }
    }
    fn retained(p: &Curve, path: &Curve, s: &Curve, n: [f64; 3], count: usize) -> Surface {
        let twist = constant_vector_law([0.; 3]).unwrap();
        let sweep = Sweep::new(
            p,
            path,
            s,
            &twist,
            Options {
                normal: n,
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::Parameter,
                initial_sections: count,
                max_sections: count,
                max_deviation: 1.,
            },
        )
        .unwrap();
        let mut result = crate::surface::loft(&sweep.sections_at(count).unwrap()).unwrap();
        for k in &mut result.knots_v {
            *k /= (count - 1) as f64;
        }
        result
    }
    #[test]
    fn affine_rational_profile_has_rounding_inclusive_continuous_bound() {
        let p = curve(
            &[[1., 2., 0.], [2., 3., 1.], [4., 1., 0.]],
            &[1., 0.25, 3.],
            [2., 7.],
        );
        let path = curve(&[[0., 0., 0.], [0., 0., 5.]], &[2., 2.], [-4., 9.]);
        let s = curve(&[[1., 0., 0.], [2., 0., 0.]], &[3., 3.], [10., 12.]);
        let surface = retained(&p, &path, &s, [1., 0., 0.], 6);
        let r = certify(&p, &path, &s, [1., 0., 0.], &surface, 1e-11, 1000).unwrap();
        assert!(r.within_budget, "{r:?}");
        assert!(r.error_upper.unwrap() > 0. && r.error_upper.unwrap() < 1e-11);
        for u in [2., 2.7, 4.2, 7.] {
            for v in [0., 0.13, 0.77, 1.] {
                let source = p.evaluate(u).unwrap().point;
                let actual = surface.evaluate(u, v).unwrap().point;
                let expected = [
                    (1. + v) * source[0],
                    (1. + v) * source[1],
                    5. * v + (1. + v) * source[2],
                ];
                let error = (0..3)
                    .map(|k| (actual[k] - expected[k]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(error <= r.error_upper.unwrap());
            }
        }
    }
    #[test]
    fn budgets_and_changed_retained_geometry_never_publish_partial_bounds() {
        let p = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 1.], [0., 1.]);
        let path = curve(&[[0., 0., 0.], [0., 0., 5.]], &[1., 1.], [0., 1.]);
        let s = constant_vector_law([1., 0., 0.]).unwrap();
        let surface = retained(&p, &path, &s, [1., 0., 0.], 5);
        for limit in [0, 1, 3] {
            let r = certify(&p, &path, &s, [1., 0., 0.], &surface, 1e-9, limit).unwrap();
            assert!(!r.within_budget && r.error_upper.is_none());
            assert!(r.cells <= limit);
        }
        let mut changed = surface;
        changed.control_points[0][2][0] += 1.;
        let r = certify(&p, &path, &s, [1., 0., 0.], &changed, 0.01, 1000).unwrap();
        assert!(!r.within_budget && r.error_upper.unwrap() >= 1.);
    }
    #[test]
    fn planar_curved_source_is_bounded_between_stations() {
        let p = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 2.], [0., 1.]);
        let path = curve(
            &[[0., 0., 0.], [0., 0.08, 2.5], [0., 0., 5.]],
            &[1., 1., 1.],
            [-2., 3.],
        );
        let s = constant_vector_law([1., 0., 0.]).unwrap();
        let surface = retained(&p, &path, &s, [1., 0., 0.], 9);
        let r = certify(&p, &path, &s, [1., 0., 0.], &surface, 0.01, 10000).unwrap();
        assert!(r.within_budget, "{r:?}");
        for i in 0..=200 {
            let v = i as f64 / 200.;
            let e = path.evaluate(-2. + 5. * v).unwrap();
            let q = surface.evaluate(0.37, v).unwrap().point;
            // X is a constant exact Bishop normal; the weighted profile is
            // independent of path curvature, so this oracle needs no frame code.
            let x = p.evaluate(0.37).unwrap().point[0];
            let d = [q[0] - x, q[1] - e.point[1], q[2] - e.point[2]];
            assert!(d.iter().map(|x| x * x).sum::<f64>().sqrt() <= r.error_upper.unwrap());
        }
    }
    #[test]
    fn open_spatial_frame_bound_is_tight_and_parameter_invariant() {
        let make = |domain| {
            curve(
                &[
                    [0., 0., 0.],
                    [0.002, 0., 1.],
                    [0., 0.003, 2.],
                    [0.004, 0.001, 3.],
                ],
                &[1.; 4],
                domain,
            )
        };
        let evaluate = |path: &Curve, closed| {
            let tangent = initial_tangent(path).unwrap();
            let normal = unit(
                sub(
                    point(&[1., 0., 0.]),
                    scale(tangent, dot(point(&[1., 0., 0.]), tangent).unwrap()).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
            let r = Reference {
                path,
                scale: path,
                plane: None,
                initial: normal,
                initial_side: cross(tangent, normal).unwrap(),
                initial_tangent: tangent,
                coordinates: vec![],
                closed,
            };
            let mut used = 0;
            (r.open_frame(1., 100, &mut used).unwrap(), used)
        };
        let a = make([0., 1.]);
        let b = make([-4., 9.]);
        let (Some((an, ab)), used) = evaluate(&a, false) else {
            panic!("missing bound")
        };
        let (Some((bn, bb)), _) = evaluate(&b, false) else {
            panic!("missing reparameterized bound")
        };
        assert!(used > 0);
        for (x, y) in an.into_iter().chain(ab).zip(bn.into_iter().chain(bb)) {
            assert!(x.hi - x.lo < 0.1);
            assert!((x.lo - y.lo).abs() < 1e-10 && (x.hi - y.hi).abs() < 1e-10);
        }
        assert!(evaluate(&a, true).0.is_none());
    }
    #[test]
    fn hidden_stationary_tangent_is_unresolved() {
        let p = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 1.], [0., 1.]);
        let path = curve(
            &[[0., 0., 0.], [0., 0., 1.], [0., 0., -1.], [0., 0., 0.]],
            &[1.; 4],
            [0., 1.],
        );
        let s = constant_vector_law([1., 0., 0.]).unwrap();
        let mut surface = crate::surface::extrude(&p, [0., 0., 1.]).unwrap();
        surface.knots_v = vec![0., 0., 1., 1.];
        let r = certify(&p, &path, &s, [1., 0., 0.], &surface, 100., 100).unwrap();
        assert!(!r.within_budget && r.error_upper.is_none());
    }
