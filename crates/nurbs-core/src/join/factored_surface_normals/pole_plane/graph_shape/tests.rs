use super::*;
#[test]
fn original_polar_hessian_is_finite_at_the_pole_for_axes_weights_and_nonunit_domains() {
    for weighted in [false, true] {
        for boundary in [
            Boundary::UMin,
            Boundary::UMax,
            Boundary::VMin,
            Boundary::VMax,
        ] {
            for drop in 0..3 {
                let mut s = super::super::tests::oriented(
                    super::super::tests::source(1, weighted),
                    boundary,
                );
                for p in s.control_points.iter_mut().flatten() {
                    let old = p.clone();
                    p[(drop + 1) % 3] = old[0] + 3.;
                    p[(drop + 2) % 3] = old[1] - 7.;
                    p[drop] = old[2] + 5.;
                }
                s.knots_u.iter_mut().for_each(|x| *x = 2. + 3. * *x);
                s.knots_v.iter_mut().for_each(|x| *x = -4. + 12. * *x);
                let saved = s.clone();
                let plane = prepare_plane_limit(&s, boundary, 1000000)
                    .unwrap()
                    .certificate
                    .unwrap();
                let prepared = plane.prepare_graph_shape(drop, 100000000).unwrap();
                let shape = prepared.certificate.expect(prepared.reason);
                assert!(std::ptr::eq(shape.surface(), &s));
                assert_eq!(axes(shape.boundary()), axes(boundary));
                assert_eq!(shape.dropped_axis(), drop);
                let (axis, end) = axes(boundary);
                let domains = [[2., 5.], [-4., 8.]];
                let mut query = domains.map(|d| {
                    let t = (d[0] + d[1]) * 0.5;
                    [t, t]
                });
                query[axis] = [domains[axis][usize::from(end)]; 2];
                let r = shape.rectangle_bounds(query, 1000000).unwrap();
                let b = r.bounds.expect(r.reason);
                for (bounds, value) in b.hessian.into_iter().zip([0.5, 0., 0.]) {
                    assert!(
                        bounds[0] <= value && value <= bounds[1],
                        "{boundary:?} axis {drop} weighted {weighted}: {bounds:?} versus {value}"
                    );
                }
                assert!(b.hessian[0][1] - b.hessian[0][0] < 0.01);
                let orientation = if plane.normal_bounds()[drop][0] > 0. {
                    1.
                } else {
                    -1.
                };
                assert!(b.normal[drop][0] <= orientation && b.normal[drop][1] >= orientation);
                let curvature = 0.5 * orientation;
                assert!(
                    b.tensor[(drop + 1) % 3][(drop + 1) % 3][0] <= curvature
                        && b.tensor[(drop + 1) % 3][(drop + 1) % 3][1] >= curvature
                );
                assert!(
                    shape
                        .rectangle_bounds(query, r.coefficients - 1)
                        .unwrap()
                        .bounds
                        .is_none()
                );
                let short = plane
                    .prepare_graph_shape(drop, prepared.products - 1)
                    .unwrap();
                assert!(short.certificate.is_none());
                assert_eq!(short.products, prepared.products - 1);
                // Independent original-parameter jet path at a regular point
                // checks orientation AND the world tensor, rather than using
                // the factored normal itself as a reference.
                let mut regular = query;
                let inward = if end { 0.875 } else { 0.125 };
                let t = domains[axis][0] + inward * (domains[axis][1] - domains[axis][0]);
                regular[axis] = [t, t];
                let q = shape.rectangle_bounds(regular, 1000000).unwrap();
                let q = q.bounds.expect(q.reason);
                let reference = crate::surface_shape_operator::at(
                    &s,
                    [regular[0][0], regular[1][0]],
                    [crate::curve_differential::Side::Automatic; 2],
                )
                .unwrap();
                let normal = reference.normal.expect("regular original normal");
                let tensor = reference.tensor.expect("regular original tensor");
                for k in 0..3 {
                    assert!(q.normal[k][0] <= normal[k][1] && normal[k][0] <= q.normal[k][1]);
                    for l in 0..3 {
                        assert!(
                            q.tensor[k][l][0] <= tensor[k][l][1]
                                && tensor[k][l][0] <= q.tensor[k][l][1]
                        );
                    }
                }
                assert_eq!(s, saved);
            }
        }
    }
}
#[test]
fn unsupported_radial_orders_and_missing_projection_do_not_produce_shape_tokens() {
    let high = super::super::tests::source(2, false);
    let p = prepare_plane_limit(&high, Boundary::UMin, 1000000)
        .unwrap()
        .certificate
        .unwrap();
    assert!(
        p.prepare_graph_shape(2, 1000000)
            .unwrap()
            .certificate
            .is_none()
    );
    let s = super::super::tests::source(1, false);
    let p = prepare_plane_limit(&s, Boundary::UMin, 1000000)
        .unwrap()
        .certificate
        .unwrap();
    assert!(
        p.prepare_graph_shape(0, 1000000)
            .unwrap()
            .certificate
            .is_none()
    );
    let shape = p
        .prepare_graph_shape(2, 1000000)
        .unwrap()
        .certificate
        .unwrap();
    assert!(
        shape
            .rectangle_bounds([[-1., 0.], [0., 1.]], 100000)
            .is_err()
    );
    assert!(p.prepare_graph_shape(2, 0).is_err());
}

#[test]
fn quadratic_bowl_retains_mixed_and_angular_curvature_at_the_pole() {
    for weighted in [false, true] {
        let s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![2., 2., 2., 5., 5., 5.],
            knots_v: vec![-4., -4., -4., 8., 8., 8.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            vec![
                                i as f64 / 2.,
                                (i * j) as f64 / 4.,
                                if i == 2 { [1., 1.25, 3.5][j] } else { 0. },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            if weighted {
                                2f64.powi(i as i32 - j as i32)
                            } else {
                                1.
                            }
                        })
                        .collect()
                })
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let plane = prepare_plane_limit(&s, Boundary::UMin, 1000000)
            .unwrap()
            .certificate
            .unwrap();
        let r = plane.prepare_graph_shape(2, 100000000).unwrap();
        let shape = r.certificate.expect(r.reason);
        let r = shape
            .rectangle_bounds([[2., 2.], [2., 2.]], 1000000)
            .unwrap();
        let b = r.bounds.expect(r.reason);
        for (b, v) in b.hessian.into_iter().zip([2., 0.5, 4.]) {
            assert!(
                b[0] <= v && v <= b[1],
                "weighted {weighted}: {b:?} versus {v}"
            );
        }
        for (b, v) in [
            (b.tensor[0][0], 2.),
            (b.tensor[0][1], 0.5),
            (b.tensor[1][1], 4.),
        ] {
            assert!(b[0] <= v && v <= b[1]);
        }
    }
}
