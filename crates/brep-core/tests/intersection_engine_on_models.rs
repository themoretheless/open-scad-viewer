//! Intersection-engine queries exercised on exact analytic models. These tests
//! lived beside the engine while it was part of `brep-core`; the engine now sits
//! below `brep-core` in `nurbs-intersect`, so the model-building cases run here.
#![allow(clippy::excessive_precision)]
use brep_core::{cylinder, frustum, sphere};
use nurbs_intersect::*;

fn plane(z: f64) -> Plane {
    Plane {
        normal: [0., 0., 1.],
        offset: z,
    }
}

fn points(report: &Report<CurvePlaneComponent>) -> Vec<&CurvePoint> {
    report
        .components
        .iter()
        .filter_map(|c| {
            if let CurvePlaneComponent::Point(p) = c {
                Some(p)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn rational_arc_plane_query_retains_curve_parameters() {
    let body = cylinder(2., 3.).unwrap();
    let arc = &body
        .edges
        .iter()
        .find(|e| e.curve.degree == 2)
        .unwrap()
        .curve;
    let report = curve_plane(
        arc,
        Plane {
            normal: [1., 0., 0.],
            offset: 1.,
        },
        Options::default(),
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let roots = points(&report);
    assert_eq!(roots.len(), 1);
    let p = roots[0].point;
    assert!((p[0] - 1.).abs() < 1e-9);
    assert!((p[0] * p[0] + p[1] * p[1] - 4.).abs() < 1e-10);
}

#[test]
fn ruled_cylinder_sections_preserve_exact_procedural_traces() {
    let body = cylinder(2., 4.).unwrap();
    let surface = &body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface;
    for plane in [
        plane(1.5),
        Plane {
            normal: [0.2, -0.1, 1.],
            offset: 2.,
        },
    ] {
        let report = surface_plane(surface, plane, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1);
        let SurfacePlaneComponent::Curve { trace, .. } = &report.components[0] else {
            panic!("Expected a section curve")
        };
        for i in 0..101 {
            let p = trace.evaluate(i as f64 / 100.).unwrap();
            assert!(p.plane_residual < 1e-12);
            assert!((p.point[0] * p.point[0] + p.point[1] * p.point[1] - 4.).abs() < 1e-11);
        }
    }
}

#[test]
fn ruled_u_sections_preserve_source_uv_and_serialized_traces() {
    let body = cylinder(2., 4.).unwrap();
    let mut original = body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface
        .clone();
    for knot in &mut original.knots_u {
        *knot = 3. + 2. * *knot;
    }
    for knot in &mut original.knots_v {
        *knot = -4. + 7. * *knot;
    }
    let source = transpose_surface(&original);
    for plane in [
        plane(1.5),
        Plane {
            normal: [0.2, -0.1, 1.],
            offset: 2.,
        },
        Plane {
            normal: [1., 0., 0.],
            offset: 1.,
        },
    ] {
        let a = surface_plane(&original, plane, Options::default()).unwrap();
        let b = surface_plane(&source, plane, Options::default()).unwrap();
        assert_eq!(a.coverage, b.coverage);
        assert_eq!(a.components.len(), b.components.len());
        assert!(!b.components.is_empty());
        for (a, b) in a.components.iter().zip(&b.components) {
            let (
                SurfacePlaneComponent::Curve {
                    trace: ta,
                    parameter_box: ba,
                    ..
                },
                SurfacePlaneComponent::Curve {
                    trace: tb,
                    parameter_box: bb,
                    ..
                },
            ) = (a, b)
            else {
                panic!("Expected traces");
            };
            assert_eq!(*bb, [ba[2], ba[3], ba[0], ba[1]]);
            let wire = value_codec::to_string(tb).unwrap();
            let loaded: SurfaceTrace = value_codec::from_str_strict(&wire).unwrap();
            for i in 0..=20 {
                let fraction = i as f64 / 20.;
                let a = ta.evaluate(fraction).unwrap();
                let b = loaded.evaluate(fraction).unwrap();
                assert_eq!(b.uv, [a.uv[1], a.uv[0]]);
                assert!(
                    a.point
                        .iter()
                        .zip(b.point)
                        .all(|(a, b)| (a - b).abs() < 1e-12)
                );
                let source_point = source.evaluate(b.uv[0], b.uv[1]).unwrap().point;
                assert!(
                    source_point
                        .iter()
                        .zip(b.point)
                        .all(|(a, b)| (a - b).abs() < 1e-11)
                );
                assert!(b.plane_residual < 1e-9);
            }
        }
    }
    let plane = Plane {
        normal: [0.2, -0.1, 1.],
        offset: 0.,
    };
    let options = Options {
        max_boxes: 1,
        ..Options::default()
    };
    let a = surface_plane(&original, plane, options).unwrap();
    let b = surface_plane(&source, plane, options).unwrap();
    assert_eq!(a.coverage, Coverage::Incomplete);
    assert_eq!(a.unresolved.len(), b.unresolved.len());
    for (a, b) in a.unresolved.iter().zip(&b.unresolved) {
        assert_eq!(a.reason, b.reason);
        assert_eq!(
            b.parameter_box,
            vec![
                a.parameter_box[2],
                a.parameter_box[3],
                a.parameter_box[0],
                a.parameter_box[1]
            ]
        );
    }
}

#[test]
fn trace_evaluation_admits_the_whole_parameter_definition() {
    let body = cylinder(2., 4.).unwrap();
    let surface = body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface
        .clone();
    let domain = surface_domain(&surface);
    for bad in [f64::NAN, f64::INFINITY, domain[1] + 1., domain[0] - 1.] {
        let trace = SurfaceTrace::Ruled {
            surface: surface.clone(),
            plane: plane(2.),
            u_interval: [domain[0], bad],
        };
        // Even fraction zero must admit the unused endpoint.
        assert!(trace.evaluate(0.).is_err());
        assert!(swap_trace(trace).evaluate(0.).is_err());
        let line = SurfaceTrace::Line {
            surface: surface.clone(),
            plane: plane(2.),
            start: [domain[0], domain[2]],
            end: [bad, domain[3]],
        };
        assert!(line.evaluate(0.).is_err());
    }
    // Reversed intervals remain valid oriented traces.
    let forward = SurfaceTrace::Ruled {
        surface: surface.clone(),
        plane: plane(2.),
        u_interval: [domain[0], domain[1]],
    };
    let reverse = SurfaceTrace::Ruled {
        surface,
        plane: plane(2.),
        u_interval: [domain[1], domain[0]],
    };
    assert_eq!(
        forward.evaluate(0.25).unwrap().uv,
        reverse.evaluate(0.75).unwrap().uv
    );
}

#[test]
fn unequal_ruling_weights_preserve_rational_section_parameters() {
    let body = cylinder(2., 4.).unwrap();
    let mut surface = body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface
        .clone();
    for row in &mut surface.weights {
        row[1] *= 3.;
    }
    for source in [surface.clone(), transpose_surface(&surface)] {
        let report = surface_plane(&source, plane(2.), Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(report.components.len(), 1);
        let SurfacePlaneComponent::Curve { trace, .. } = &report.components[0] else {
            panic!("Expected curve")
        };
        let loaded: SurfaceTrace =
            value_codec::from_str_strict(&value_codec::to_string(trace).unwrap()).unwrap();
        for i in 0..=20 {
            let point = loaded.evaluate(i as f64 / 20.).unwrap();
            let ruling_axis = if source.degree_v == 1 { 1 } else { 0 };
            // z=4*(3*t)/(1-t+3*t), hence z=2 at t=1/4.
            assert!((point.uv[ruling_axis] - 0.25).abs() < 1e-12);
            assert!((point.point[2] - 2.).abs() < 1e-12);
            assert!((point.point[0].powi(2) + point.point[1].powi(2) - 4.).abs() < 1e-11);
        }
    }
}

#[test]
fn knot_refinement_preserves_linear_u_section_support() {
    let body = cylinder(2., 4.).unwrap();
    let original = &body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface;
    let mut source = transpose_surface(original);
    // Insert a half-domain knot along the equal-weight linear direction.
    let middle = source.control_points[0]
        .iter()
        .zip(&source.control_points[1])
        .map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a + b) * 0.5).collect())
        .collect();
    source.control_points.insert(1, middle);
    source.weights.insert(1, source.weights[0].clone());
    source.knots_u.insert(2, 0.5);
    source.validate().unwrap();
    for height in [1., 2., 3.] {
        let report = surface_plane(&source, plane(height), Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1);
        let SurfacePlaneComponent::Curve {
            trace,
            parameter_box,
            ..
        } = &report.components[0]
        else {
            panic!("Expected curve")
        };
        assert_eq!(
            [parameter_box[0], parameter_box[1]],
            if height < 2. {
                [0., 0.5]
            } else if height == 2. {
                [0.5, 0.5]
            } else {
                [0.5, 1.]
            }
        );
        for i in 0..=20 {
            let point = trace.evaluate(i as f64 / 20.).unwrap();
            assert!((point.uv[0] - height / 4.).abs() < 1e-12);
            assert!((point.point[2] - height).abs() < 1e-12);
            assert!((point.point[0].powi(2) + point.point[1].powi(2) - 4.).abs() < 1e-11);
        }
    }
}

#[test]
fn disconnected_ruling_knot_is_rejected_before_sectioning() {
    let body = cylinder(2., 4.).unwrap();
    let original = &body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface;
    let mut source = transpose_surface(original);
    let mut first_top = source.control_points[0].clone();
    for p in &mut first_top {
        p[2] = 2.;
    }
    let mut second_bottom = first_top.clone();
    let mut second_top = source.control_points[1].clone();
    for row in [&mut second_bottom, &mut second_top] {
        for p in row {
            p[0] *= 1.5;
            p[1] *= 1.5;
        }
    }
    source.control_points = vec![
        source.control_points[0].clone(),
        first_top,
        second_bottom,
        second_top,
    ];
    source.weights = vec![source.weights[0].clone(); 4];
    source.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
    assert_eq!(
        surface_plane(&source, plane(2.), Options::default())
            .unwrap_err()
            .code,
        "NURBS_INVALID_INPUT"
    );
}

#[test]
fn proportional_ruling_weights_retain_generator_sections() {
    let body = cylinder(2., 4.).unwrap();
    let mut source = body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface
        .clone();
    for row in &mut source.weights {
        row[1] *= 3.;
    }
    let plane = Plane {
        normal: [1., 0., 0.],
        offset: 1.,
    };
    for surface in [source.clone(), transpose_surface(&source)] {
        let report = surface_plane(&surface, plane, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1);
        let SurfacePlaneComponent::Curve { trace, .. } = &report.components[0] else {
            panic!("Expected generator")
        };
        for i in 0..=20 {
            let t = i as f64 / 20.;
            let p = trace.evaluate(t).unwrap();
            assert!((p.point[0] - 1.).abs() < 1e-9);
            assert!((p.point[1] - 3_f64.sqrt()).abs() < 1e-9);
            assert!((p.point[2] - 12. * t / (1. + 2. * t)).abs() < 1e-11);
        }
    }
}

#[test]
fn nonproportional_boundary_weights_do_not_become_generator_lines() {
    let body = cylinder(2., 4.).unwrap();
    let mut surface = body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface
        .clone();
    for (row, scale) in surface.weights.iter_mut().zip([2., 3., 5.]) {
        row[1] *= scale;
    }
    let plane = Plane {
        normal: [1., 0., 0.],
        offset: 1.,
    };
    let report = surface_plane(
        &surface,
        plane,
        Options {
            max_boxes: 128,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(!report.components.is_empty(), "{report:?}");
    for component in &report.components {
        let SurfacePlaneComponent::Curve { trace, .. } = component else {
            panic!("Expected curve")
        };
        assert!(matches!(trace, SurfaceTrace::Ruled { .. }));
        for i in 0..=20 {
            let p = trace.evaluate(i as f64 / 20.).unwrap();
            assert!(p.plane_residual < 1e-9);
            assert!((p.point[0] - 1.).abs() < 1e-9);
            let [u, v] = p.uv;
            let basis = [(1. - u).powi(2), 2. * u * (1. - u), u * u];
            let base = [1., std::f64::consts::FRAC_1_SQRT_2, 1.];
            let upper: f64 = (0..3).map(|i| basis[i] * base[i] * [2., 3., 5.][i]).sum();
            let lower: f64 = (0..3).map(|i| basis[i] * base[i]).sum();
            let weight = (1. - v) * lower + v * upper;
            assert!((p.point[2] - 4. * v * upper / weight).abs() < 1e-11);
            let x = (0..2)
                .map(|i| 2. * basis[i] * base[i] * ((1. - v) + v * [2., 3.][i]))
                .sum::<f64>()
                / weight;
            assert!((x - 1.).abs() < 1e-9);
        }
    }
    // Endpoint-root bands are retained explicitly under a finite budget.
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert!(!report.unresolved.is_empty());
}

#[test]
fn rational_arc_segment_query_preserves_reversed_segment_correspondence() {
    let body = cylinder(2., 4.).unwrap();
    let arc = &body
        .edges
        .iter()
        .find(|e| e.curve.degree == 2)
        .unwrap()
        .curve;
    for (start, end, expected) in [
        ([0., 1., 0.], [3., 1., 0.], 3_f64.sqrt() / 3.),
        ([3., 1., 0.], [0., 1., 0.], 1. - 3_f64.sqrt() / 3.),
    ] {
        let report = curve_segment(arc, start, end, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1);
        let CurveSegmentComponent::Point {
            curve,
            segment_parameter,
            line_residual,
        } = &report.components[0]
        else {
            panic!("Expected arc hit")
        };
        assert!((curve.point[0] - 3_f64.sqrt()).abs() < 1e-9);
        assert!((curve.point[1] - 1.).abs() < 1e-9);
        assert!((segment_parameter - expected).abs() < 1e-9);
        assert!(*line_residual < 1e-9);
        assert!(report.boxes_visited <= Options::default().max_boxes);
    }
}

#[test]
fn ruled_sections_convert_algebraically_to_rational_curves() {
    let body = cylinder(2., 4.).unwrap();
    let mut surface = body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface
        .clone();
    for row in &mut surface.weights {
        row[1] *= 3.;
    }
    for surface in [surface.clone(), transpose_surface(&surface)] {
        for plane in [
            plane(2.),
            Plane {
                normal: [0.2, -0.1, 1.],
                offset: 2.,
            },
            Plane {
                normal: [1., 0., 0.],
                offset: 1.,
            },
        ] {
            let report = surface_plane(&surface, plane, Options::default()).unwrap();
            for component in report.components {
                let SurfacePlaneComponent::Curve { trace, .. } = component else {
                    panic!("Expected trace")
                };
                let curve = trace.to_curve().unwrap();
                let [a, b] = curve.domain();
                for i in 0..=40 {
                    let t = i as f64 / 40.;
                    let expected = trace.evaluate(t).unwrap().point;
                    let actual = curve.evaluate(a + t * (b - a)).unwrap().point;
                    assert!(
                        expected
                            .iter()
                            .zip(actual)
                            .all(|(a, b)| (a - b).abs() < 1e-10)
                    );
                }
            }
        }
    }
}

#[test]
fn trace_curve_conversion_preserves_reversed_fraction_on_shifted_domains() {
    let body = cylinder(2., 4.).unwrap();
    let mut source = body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface
        .clone();
    source.knots_u.iter_mut().for_each(|t| *t = -3. + 8. * *t);
    source.knots_v.iter_mut().for_each(|t| *t = 10. + 4. * *t);
    for surface in [source.clone(), transpose_surface(&source)] {
        for plane in [
            plane(2.),
            Plane {
                normal: [1., 0., 0.],
                offset: 1.,
            },
        ] {
            for component in surface_plane(&surface, plane, Options::default())
                .unwrap()
                .components
            {
                let SurfacePlaneComponent::Curve { trace, .. } = component else {
                    panic!("Expected trace")
                };
                let mut reverse = trace.clone();
                match &mut reverse {
                    SurfaceTrace::Line { start, end, .. } => std::mem::swap(start, end),
                    SurfaceTrace::Ruled { u_interval, .. } => u_interval.swap(0, 1),
                    SurfaceTrace::RuledU { v_interval, .. } => v_interval.swap(0, 1),
                }
                let curve = reverse.to_curve().unwrap();
                let [a, b] = curve.domain();
                for i in 0..=20 {
                    let t = i as f64 / 20.;
                    let p = trace.evaluate(1. - t).unwrap().point;
                    let q = curve.evaluate(a + t * (b - a)).unwrap().point;
                    assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-10));
                }
            }
        }
    }
}

#[test]
fn tensor_uv_diagonal_conversion_preserves_spherical_geometry() {
    let mut surface = sphere(2.).unwrap().faces[0].surface.clone();
    for knot in &mut surface.knots_u {
        *knot = -3. + 8. * *knot;
    }
    for knot in &mut surface.knots_v {
        *knot = 10. + 4. * *knot;
    }
    for flip_u in [false, true] {
        for flip_v in [false, true] {
            let u = if flip_u { [4.2, -2.2] } else { [-2.2, 4.2] };
            let v = if flip_v { [13.2, 10.8] } else { [10.8, 13.2] };
            let trace = SurfaceTrace::Line {
                surface: surface.clone(),
                plane: plane(0.),
                start: [u[0], v[0]],
                end: [u[1], v[1]],
            };
            let curve = trace.to_curve().unwrap();
            assert_eq!(curve.degree, surface.degree_u + surface.degree_v);
            for i in 0..=100 {
                let t = i as f64 / 100.;
                let p = curve.evaluate(t).unwrap().point;
                let q = trace.evaluate(t).unwrap().point;
                assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-10));
                assert!((p.iter().map(|x| x * x).sum::<f64>() - 4.).abs() < 1e-10);
            }
        }
    }
    let high_degree = surface
        .edit_axis(nurbs_core::surface::Axis::U, |c| c.elevate(13))
        .unwrap()
        .edit_axis(nurbs_core::surface::Axis::V, |c| c.elevate(13))
        .unwrap();
    let oversized = SurfaceTrace::Line {
        surface: high_degree,
        plane: plane(0.),
        start: [-2.2, 10.8],
        end: [4.2, 13.2],
    };
    assert!(
        oversized
            .to_curve()
            .unwrap_err()
            .to_string()
            .contains("degree at most 25")
    );
    let point_trace = SurfaceTrace::Line {
        surface: surface.clone(),
        plane: plane(0.),
        start: [1., 12.],
        end: [1., 12.],
    };
    let point_curve = point_trace.to_curve().unwrap();
    assert_eq!(
        point_curve.evaluate(0.3).unwrap().point,
        point_trace.evaluate(0.3).unwrap().point
    );
    let refined = surface
        .edit_axis(nurbs_core::surface::Axis::U, |c| c.insert(1., 1))
        .unwrap();
    let unsupported = SurfaceTrace::Line {
        surface: refined,
        plane: plane(0.),
        start: [-2.2, 10.8],
        end: [4.2, 13.2],
    };
    assert!(
        unsupported
            .to_curve()
            .unwrap_err()
            .to_string()
            .contains("endpoints do not coincide")
    );
}

#[test]
fn multispan_uv_diagonal_preserves_fraction_and_refined_surface() {
    let original = sphere(2.).unwrap().faces[0].surface.clone();
    let refined = original
        .edit_axis(nurbs_core::surface::Axis::U, |c| {
            c.insert(0.25, 1)?.insert(0.875, 1)
        })
        .unwrap()
        .edit_axis(nurbs_core::surface::Axis::V, |c| c.insert(0.5, 1))
        .unwrap();
    for reverse in [false, true] {
        let (start, end) = if reverse {
            ([1., 1.], [0., 0.])
        } else {
            ([0., 0.], [1., 1.])
        };
        let trace = SurfaceTrace::Line {
            surface: refined.clone(),
            plane: plane(0.),
            start,
            end,
        };
        let pieces = trace.to_curve_segments().unwrap();
        assert_eq!(pieces.len(), 4);
        let boundaries = if reverse {
            [0., 0.125, 0.5, 0.75, 1.]
        } else {
            [0., 0.25, 0.5, 0.875, 1.]
        };
        for (index, curve) in pieces.iter().enumerate() {
            assert_eq!(curve.domain(), [boundaries[index], boundaries[index + 1]]);
            assert_eq!(curve.degree, 4);
            for i in 0..=100 {
                let t = boundaries[index]
                    + (boundaries[index + 1] - boundaries[index]) * i as f64 / 100.;
                let actual = curve.evaluate(t).unwrap().point;
                let expected = trace.evaluate(t).unwrap().point;
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| (a - b).abs() < 1e-10)
                );
                assert!((actual.iter().map(|v| v * v).sum::<f64>() - 4.).abs() < 1e-10);
            }
        }
    }
    let dense = original
        .edit_axis(nurbs_core::surface::Axis::U, |c| {
            let mut result = c.clone();
            for i in 1..28 {
                result = result.insert(i as f64 / 64., 1)?;
            }
            Ok(result)
        })
        .unwrap()
        .edit_axis(nurbs_core::surface::Axis::V, |c| {
            let mut result = c.clone();
            for i in 33..60 {
                result = result.insert(i as f64 / 64., 1)?;
            }
            Ok(result)
        })
        .unwrap();
    assert!(
        SurfaceTrace::Line {
            surface: dense,
            plane: plane(0.),
            start: [0., 0.],
            end: [1., 1.]
        }
        .to_curve_segments()
        .unwrap_err()
        .to_string()
        .contains("256 control points")
    );
}

#[test]
fn frustum_sections_and_incomplete_boundary_bands_are_distinguished() {
    let body = frustum(2., 1., 4.).unwrap();
    let surface = &body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface;
    let report = surface_plane(surface, plane(2.), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let SurfacePlaneComponent::Curve { trace, .. } = &report.components[0] else {
        panic!()
    };
    for i in 0..51 {
        let p = trace.evaluate(i as f64 / 50.).unwrap();
        assert!((p.point[0] * p.point[0] + p.point[1] * p.point[1] - 2.25).abs() < 1e-10);
    }
    let crossing = surface_plane(
        surface,
        Plane {
            normal: [1., -1., 0.],
            offset: 0.,
        },
        Options {
            max_boxes: 128,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(crossing.coverage, Coverage::Incomplete);
    assert!(!crossing.unresolved.is_empty());
}

#[test]
fn vertical_cylinder_cut_is_a_retained_generator_with_root_interval() {
    let body = cylinder(2., 4.).unwrap();
    let surface = &body
        .faces
        .iter()
        .find(|f| f.surface.degree_u == 2)
        .unwrap()
        .surface;
    let plane = Plane {
        normal: [1., 0., 0.],
        offset: 1.,
    };
    let report = surface_plane(surface, plane, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert_eq!(report.components.len(), 1);
    let SurfacePlaneComponent::Curve {
        trace,
        parameter_box,
        ..
    } = &report.components[0]
    else {
        panic!()
    };
    assert!(parameter_box[1] - parameter_box[0] <= Options::default().parameter_tolerance);
    for i in 0..101 {
        let sample = trace.evaluate(i as f64 / 100.).unwrap();
        assert!(sample.plane_residual < 1e-9);
        assert!((sample.point[2] - 4. * i as f64 / 100.).abs() < 1e-12);
    }
}
