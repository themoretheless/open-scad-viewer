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

    #[test]
    fn oblique_multi_span_tangent_is_exact_and_budgeted() {
        let p=curve(&[[1.,0.,0.],[2.,0.,0.]],&[1.,1.],[0.,1.]);
        let path=Curve { degree:2, knots:vec![0.,0.,0.,0.5,0.5,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![0.,0.015625,1.],vec![0.,0.03125,2.],vec![0.,0.046875,3.],vec![0.,0.,4.]],
            weights:vec![1.;5],periodic:false };
        let original=path.clone();
        let law=constant_vector_law([1.,0.,0.]).unwrap();
        let surface=retained(&p,&path,&law,[1.,0.,0.],9);
        let report=certify(&p,&path,&law,[1.,0.,0.],&surface,0.01,10000).unwrap();
        assert!(report.within_budget,"{report:?}");
        assert_eq!(report.method,"interval-planar-bishop-frame");
        let mut changed=path.clone();changed.control_points[3][1]=changed.control_points[3][1].next_up();
        let refused=certify(&p,&changed,&law,[1.,0.,0.],&surface,0.01,10000).unwrap();
        assert_eq!(refused.reason,Some("continuous-path-tangent-unproved"));
        assert!(refused.error_upper.is_none());
        for limit in [0,1,2,10] {
            let limited=certify(&p,&path,&law,[1.,0.,0.],&surface,0.01,limit).unwrap();
            assert!(!limited.within_budget && limited.error_upper.is_none());assert!(limited.cells<=limit);
        }
        assert_eq!(path,original);
    }

    #[test]
    fn tilted_planar_bishop_bound_contains_independent_constant_normal_oracle() {
        let p=curve(&[[1.,-1.,0.],[2.,-2.,0.]],&[1.,2.],[0.,1.]);
        let path=curve(&[[0.,0.,0.],[0.125,0.125,2.],[0.,0.,4.]],&[1.,1.,1.],[-2.,3.]);
        let law=constant_vector_law([1.,0.,0.]).unwrap();
        let surface=retained(&p,&path,&law,[1.,-1.,0.],9);
        let report=certify(&p,&path,&law,[1.,-1.,0.],&surface,0.01,10000).unwrap();
        assert!(report.within_budget,"{report:?}");
        assert_eq!(report.method,"interval-planar-bishop-frame");
        for i in 0..=100 {
            let v=i as f64/100.; let c=path.evaluate(-2.+5.*v).unwrap().point;
            let q=p.evaluate(0.37).unwrap().point;let actual=surface.evaluate(0.37,v).unwrap().point;
            let distance=(0..3).map(|k|(actual[k]-c[k]-q[k]).powi(2)).sum::<f64>().sqrt();
            assert!(distance<=report.error_upper.unwrap(),"{distance} {report:?}");
        }
        let tiny=certify(&p,&path,&law,[1.,-1.,0.],&surface,0.01,1).unwrap();
        assert!(tiny.error_upper.is_none() && !tiny.within_budget && tiny.cells<=1);
    }

    #[test]
    fn closed_tilted_planar_path_has_exact_oblique_seam_premise_at_six_and_ten_stations() {
        let path=Curve{degree:2,knots:vec![0.,0.,0.,0.25,0.25,0.5,0.5,0.75,0.75,1.,1.,1.],
            control_points:vec![vec![3.,0.,0.],vec![3.,3.,3.],vec![0.,3.,3.],vec![-3.,3.,3.],vec![-3.,0.,0.],vec![-3.,-3.,-3.],vec![0.,-3.,-3.],vec![3.,-3.,-3.],vec![3.,0.,0.]],
            weights:vec![1.,1.,2.,2.,4.,4.,8.,8.,16.],periodic:false};
        let p=curve(&[[3.,-0.125,0.125],[3.,-0.25,0.25]],&[1.,1.],[0.,1.]);
        let law=constant_vector_law([1.,0.,0.]).unwrap();
        for count in [6,10] {
            let surface=retained(&p,&path,&law,[0.,-1.,1.],count);
            let report=certify(&p,&path,&law,[0.,-1.,1.],&surface,1.,100000).unwrap();
            assert!(report.within_budget,"{count}: {report:?}");
            assert_eq!(report.method,"interval-planar-bishop-frame");
            for i in 0..=100 {
                let v=i as f64/100.;let c=path.evaluate(v).unwrap().point;
                let q=p.evaluate(0.37).unwrap().point;let actual=surface.evaluate(0.37,v).unwrap().point;
                let distance=(0..3).map(|k|(actual[k]-c[k]-q[k]+if k==0{3.}else{0.}).powi(2)).sum::<f64>().sqrt();
                assert!(distance<=report.error_upper.unwrap(),"{count}: {distance} {report:?}");
            }
        }
    }
