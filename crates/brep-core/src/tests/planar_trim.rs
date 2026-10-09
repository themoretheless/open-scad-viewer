use super::*;
#[test]
fn component_grouping_retains_nested_islands_and_parameter_domains() {
    let mut loops = orient_even_odd(
        &[
            rectangle(1., 1., 9., 9.),
            rectangle(2., 2., 3., 3.),
            rectangle(0., 0., 10., 10.),
            rectangle(12., 0., 13., 1.),
        ],
        1e-7,
    )
    .unwrap();
    for wire in &mut loops {
        for curve in wire {
            for knot in &mut curve.knots {
                *knot = 2. + 3. * *knot;
            }
        }
    }
    assert_eq!(
        components(&loops, 1e-7).unwrap(),
        vec![(1, vec![]), (2, vec![0]), (3, vec![])]
    );
    let invalid = vec![rectangle(0., 0., 2., 2.), rectangle(1., 1., 3., 3.)];
    assert!(components(&invalid, 1e-7).is_err());
}
fn line(a: Point, b: Point) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Curve> {
    let p = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    (0..4).map(|i| line(p[i], p[(i + 1) % 4])).collect()
}
fn circle(center: Point, r: f64, start: f64) -> Vec<Curve> {
    (0..4)
        .map(|i| {
            let a = start + i as f64 * PI / 2.;
            let b = a + PI / 2.;
            let m = (a + b) * 0.5;
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![
                    add(center, [r * a.cos(), r * a.sin()]).to_vec(),
                    add(
                        center,
                        [r * 2_f64.sqrt() * m.cos(), r * 2_f64.sqrt() * m.sin()],
                    )
                    .to_vec(),
                    add(center, [r * b.cos(), r * b.sin()]).to_vec(),
                ],
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            }
        })
        .collect()
}
fn area(loops: &[Vec<Curve>]) -> f64 {
    loops.iter().map(|l| signed_area(l, 1e-7).unwrap()).sum()
}
fn reverse(loop_: &[Curve]) -> Vec<Curve> {
    loop_.iter().rev().map(|c| c.reverse().unwrap()).collect()
}
#[test]
fn analytic_area_winding_and_material_left_holes() {
    let outer = circle([0., 0.], 2., 0.);
    let hole = reverse(&circle([0., 0.], 1., 0.));
    assert!((signed_area(&outer, 1e-7).unwrap() - 4. * PI).abs() < 1e-12);
    assert!((signed_area(&hole, 1e-7).unwrap() + PI).abs() < 1e-12);
    let region = vec![hole.clone(), outer];
    validate(&region, 1e-7).unwrap();
    assert_eq!(
        locate_point(&region, [0., 0.], 1e-7).unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        locate_point(&region, [1.5, 0.], 1e-7).unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        locate_point(&[hole], [0., 0.], 1e-7).unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn circle_line_cut_retains_rational_arcs_and_semicircle_area() {
    let disk = vec![circle([0., 0.], 1., 0.)];
    let half = vec![rectangle(0., -2., 2., 2.)];
    for operation in ["intersection", "difference"] {
        let out = boolean(&disk, &half, operation, 1e-7).unwrap();
        assert!((area(&out) - PI / 2.).abs() < 1e-11, "{operation}");
        assert!(out.iter().flatten().any(|c| c.degree == 2));
        assert!(out.iter().flatten().any(|c| c.degree == 1));
        for curve in out.iter().flatten().filter(|c| c.degree == 2) {
            for i in 0..19 {
                let d = curve.domain();
                let p = point(curve, d[0] + i as f64 / 18. * (d[1] - d[0])).unwrap();
                assert!((dot(p, p) - 1.).abs() < 1e-12);
            }
        }
    }
}
#[test]
fn transverse_circle_boolean_matches_independent_lens_area() {
    let a = vec![circle([0., 0.], 1., 0.)];
    let b = vec![circle([1., 0.], 1., 0.)];
    let lens = 2. * (0.5_f64).acos() - 0.5 * 3_f64.sqrt();
    for (op, expected) in [
        ("intersection", lens),
        ("union", 2. * PI - lens),
        ("difference", PI - lens),
        ("xor", 2. * PI - 2. * lens),
    ] {
        let out = boolean(&a, &b, op, 1e-7).unwrap();
        assert!(
            (area(&out) - expected).abs() < 1e-10,
            "{op}: {} vs {expected}",
            area(&out)
        );
        if op != "difference" {
            assert!((area(&boolean(&b, &a, op, 1e-7).unwrap()) - expected).abs() < 1e-10);
        }
    }
}
#[test]
fn coincidence_tangency_and_disjoint_components_are_regularized() {
    let a = vec![circle([0., 0.], 1., 0.)];
    let same = vec![circle([0., 0.], 1., PI / 4.)];
    for op in ["union", "intersection"] {
        let result = boolean(&a, &same, op, 1e-7).unwrap();
        assert!((area(&result) - PI).abs() < 1e-10);
    }
    assert!(boolean(&a, &same, "difference", 1e-7).unwrap().is_empty());
    for shift in [2., 3.] {
        let b = vec![circle([shift, 0.], 1., 0.)];
        let union = boolean(&a, &b, "union", 1e-7).unwrap();
        assert_eq!(union.len(), 2);
        assert!((area(&union) - 2. * PI).abs() < 1e-10);
        assert!(boolean(&a, &b, "intersection", 1e-7).unwrap().is_empty());
    }
}
#[test]
fn holes_multiple_outputs_and_shared_line_subsegments() {
    let a = vec![rectangle(-3., -3., 3., 3.)];
    let b = vec![circle([0., 0.], 1., 0.)];
    let hole = boolean(&a, &b, "difference", 1e-7).unwrap();
    assert_eq!(hole.len(), 2);
    assert!((area(&hole) - (36. - PI)).abs() < 1e-10);
    let cutter = vec![rectangle(-0.25, -4., 0.25, 4.)];
    let split = boolean(&hole, &cutter, "difference", 1e-7).unwrap();
    assert_eq!(split.len(), 2);
    validate(&split, 1e-7).unwrap();
    let union = boolean(
        &[rectangle(0., 0., 2., 1.)],
        &[rectangle(1., 0., 3., 1.)],
        "union",
        1e-7,
    )
    .unwrap();
    assert_eq!(union.len(), 1);
    assert!((area(&union) - 3.).abs() < 1e-12);
}
#[test]
fn rejects_noncircular_quadratics_and_wrong_hole_orientation() {
    let mut fake = circle([0., 0.], 1., 0.);
    fake[0].weights[1] = 1.;
    assert!(validate(&[fake], 1e-7).is_err());
    assert!(validate(&[circle([0., 0.], 2., 0.), circle([0., 0.], 1., 0.)], 1e-7).is_err());
}

#[test]
fn rotated_translated_and_reparameterized_arrangements_keep_area_and_carriers() {
    let a = vec![circle([0., 0.], 2., 0.)];
    let b = vec![rectangle(0.3, -3., 4., 3.)];
    let cap = 4. * (0.15_f64).acos() - 0.3 * (4. - 0.09_f64).sqrt();
    let transform = |region: &[Vec<Curve>], reflect: bool| -> Vec<Vec<Curve>> {
        region
            .iter()
            .map(|loop_| {
                let mut result: Vec<_> = loop_
                    .iter()
                    .map(|curve| {
                        let mut curve = curve.clone();
                        for p in &mut curve.control_points {
                            let x = if reflect { -p[0] } else { p[0] };
                            let y = p[1];
                            let (s, c) = 0.371_f64.sin_cos();
                            p[0] = 123. + 2.7 * (c * x - s * y);
                            p[1] = -55. + 2.7 * (s * x + c * y);
                        }
                        for knot in &mut curve.knots {
                            *knot = 1000. + 0.125 * (*knot);
                        }
                        curve
                    })
                    .collect();
                result.rotate_left(1);
                if reflect { reverse(&result) } else { result }
            })
            .collect()
    };
    for reflect in [false, true] {
        let a = transform(&a, reflect);
        let b = transform(&b, reflect);
        for (operation, expected) in [
            ("intersection", cap),
            ("difference", 4. * PI - cap),
            ("union", 4. * PI + 22.2 - cap),
            ("xor", 4. * PI + 22.2 - 2. * cap),
        ] {
            let output = boolean(&a, &b, operation, 1e-7).unwrap();
            assert!(
                (area(&output) - expected * 2.7 * 2.7).abs() < 1e-8,
                "{operation} reflect={reflect}"
            );
            assert!(output.iter().flatten().any(|c| c.degree == 2));
            for curve in output.iter().flatten() {
                let domain = curve.domain();
                assert!(domain[0] >= 1000. && domain[1] <= 1000.125);
                let p = point(curve, (domain[0] + domain[1]) * 0.5).unwrap();
                assert!(
                    p[0] > 110. && p[0] < 140. && p[1] > -75. && p[1] < -35.,
                    "Output curve left its original coordinate frame"
                );
                if curve.degree == 2 {
                    assert!((distance(p, [123., -55.]) - 5.4).abs() < 1e-9);
                }
            }
            if operation != "difference" {
                assert!(
                    (area(&boolean(&b, &a, operation, 1e-7).unwrap()) - area(&output)).abs()
                        < 1e-8
                );
            }
        }
    }
    let tangent = vec![circle([4., 0.], 2., 0.)];
    let out = boolean(
        &transform(&a, false),
        &transform(&tangent, false),
        "union",
        1e-7,
    )
    .unwrap();
    assert_eq!(out.len(), 2);
    assert!((area(&out) - 8. * PI * 2.7 * 2.7).abs() < 1e-8);
}

#[test]
#[cfg(feature = "codec")]
fn bounded_refusal_preserves_sources_and_does_not_snap_near_coincidence() {
    let a = vec![rectangle(0., 0., 1., 1.)];
    let b = vec![rectangle(0., 1e-9, 1., 1. + 1e-9)];
    let before = value_codec::to_value(&a).unwrap();
    assert!(boolean(&a, &b, "union", 1e-7).is_err());
    assert_eq!(value_codec::to_value(&a).unwrap(), before);
    let huge = vec![vec![line([0., 0.], [1., 0.]); MAX_SPANS + 1]];
    assert_eq!(
        validate(&huge, 1e-7).unwrap_err().code,
        "BREP_RESOURCE_LIMIT"
    );
    assert!(boolean(&a, &[], "intersection", 1e-7).unwrap().is_empty());
    assert!((area(&boolean(&a, &[], "difference", 1e-7).unwrap()) - 1.).abs() < 1e-12);
}

#[test]
fn validation_rejects_endpoint_encoded_crossings_and_hole_contacts() {
    let polygon = |points: &[Point]| -> Vec<Curve> {
        (0..points.len())
            .map(|i| line(points[i], points[(i + 1) % points.len()]))
            .collect()
    };
    let a = polygon(&[[0., 0.], [1., 0.], [2., 0.], [2., 1.], [2., 2.], [0., 2.]]);
    let b = polygon(&[[1., -1.], [3., -1.], [3., 1.], [2., 1.], [1., 1.], [1., 0.]]);
    assert!(validate(&[a, b], 1e-7).is_err());
    let outer = circle([0., 0.], 2., 0.);
    let tangent_hole = reverse(&circle([1., 0.], 1., 0.));
    assert!(validate(&[outer, tangent_hole], 1e-7).is_err());
    assert!(
        validate(
            &[rectangle(0., 0., 1., 1.), rectangle(1., 0., 2., 1.)],
            1e-7
        )
        .is_err()
    );
}

#[test]
fn near_coincidence_refusal_is_independent_of_translation() {
    for [x, y] in [[0., 0.], [10_000., 0.], [10_000., -20_000.]] {
        let a = vec![rectangle(x, y, x + 1., y + 1.)];
        let b = vec![rectangle(x, y + 1e-10, x + 1., y + 1. + 1e-10)];
        assert!(
            boolean(&a, &b, "union", 1e-7).is_err(),
            "Near-coincident boundary silently merged at {x},{y}"
        );
    }
}

#[test]
#[cfg(feature = "codec")]
fn local_queries_preserve_world_outputs_and_refuse_insufficient_parameter_precision() {
    let a = vec![rectangle(10_000., -20_000., 10_001., -19_999.)];
    assert_eq!(
        locate_point(&a, [10_000.5, -19_999.5], 1e-7).unwrap(),
        PointLocation::Inside
    );
    let before = value_codec::to_value(&a).unwrap();
    let b = vec![rectangle(10_000.25, -20_001., 10_002., -19_998.)];
    let output = boolean(&a, &b, "intersection", 1e-7).unwrap();
    assert!((area(&output) - 0.75).abs() < 1e-12);
    for curve in output.iter().flatten() {
        for p in &curve.control_points {
            assert!(
                p[0] >= 10_000.25 && p[0] <= 10_001. && p[1] >= -20_000. && p[1] <= -19_999.
            );
        }
    }
    assert_eq!(value_codec::to_value(&a).unwrap(), before);
    let mut ill_conditioned = a.clone();
    for curve in ill_conditioned.iter_mut().flatten() {
        for knot in &mut curve.knots {
            *knot = 1e8 + 1e-6 * (*knot);
        }
        curve.validate().unwrap();
    }
    assert_eq!(
        validate(&ill_conditioned, 1e-7).unwrap_err().code,
        "BREP_AMBIGUOUS_PLANAR_TRIM"
    );
}
