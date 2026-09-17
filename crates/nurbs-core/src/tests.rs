use super::*;
use curve::Curve;
fn circle() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    }
}
fn near(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 1e-10, "{x} != {y}");
    }
}
#[test]
fn rational_circle_has_analytic_jets() {
    let c = circle();
    let e = c.evaluate(0.5).unwrap();
    let q = std::f64::consts::FRAC_1_SQRT_2;
    let speed = 4. / (2. + 2_f64.sqrt());
    near(&e.point, &[q, q, 0.]);
    near(&e.d1.unwrap(), &[-speed, speed, 0.]);
    near(
        &e.d2.unwrap(),
        &[
            -2_f64.sqrt() * speed * speed,
            -2_f64.sqrt() * speed * speed,
            0.,
        ],
    );
}
#[test]
fn edits_preserve_geometry_and_domain() {
    let c = circle();
    let edited = [
        c.insert(0.3, 2).unwrap(),
        c.elevate(5).unwrap(),
        c.trim(0.1, 0.9).unwrap(),
    ];
    for i in 0..=80 {
        let u = 0.1 + i as f64 / 100.;
        let source = c.evaluate(u).unwrap();
        for target in &edited {
            near(&source.point, &target.evaluate(u).unwrap().point);
        }
    }
    let reversed = c.reverse().unwrap();
    near(
        &c.evaluate(0.3).unwrap().point,
        &reversed.evaluate(0.7).unwrap().point,
    );
}
#[test]
fn natural_endpoints_and_periodic_seams_are_supported() {
    let c = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0.],
            vec![2., 4.],
            vec![5., 3.],
            vec![6., 1.],
            vec![8., -1.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    near(&c.evaluate(0.).unwrap().point, &[1., 2.]);
    near(&c.evaluate(3.).unwrap().point, &[7., 0.]);
    for u in [0., 3.] {
        let edited = c.insert(u, 2).unwrap();
        for i in 0..=30 {
            let t = i as f64 / 10.;
            near(
                &c.evaluate(t).unwrap().point,
                &edited.evaluate(t).unwrap().point,
            );
        }
    }
    let p = Curve {
        degree: 2,
        knots: (0..9).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0.],
            vec![0., 1.],
            vec![-1., 0.],
            vec![0., -1.],
            vec![1., 0.],
            vec![0., 1.],
        ],
        weights: vec![1.; 6],
        periodic: true,
    };
    let a = p.evaluate(2.).unwrap();
    let b = p.evaluate(6.).unwrap();
    near(&a.point, &b.point);
    near(&a.d1.unwrap(), &b.d1.unwrap());
    assert!(a.d2.is_none());
}
#[test]
fn surface_construction_and_iso_are_consistent() {
    let c = circle();
    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    let e = value_codec::to_value(s.evaluate(0.5, 0.4).unwrap()).unwrap();
    near(
        &value_codec::from_value::<Vec<f64>>(e["point"].clone()).unwrap(),
        &[
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
            1.2,
        ],
    );
    let iso = s.iso(surface::Axis::V, 0.4).unwrap();
    for i in 0..=10 {
        let t = i as f64 / 10.;
        let mut p = c.evaluate(t).unwrap().point;
        p[2] = 1.2;
        near(&p, &iso.evaluate(t).unwrap().point);
    }
    assert_eq!(e["gaussianCurvature"], 0.);
}
#[test]
fn malformed_boundary_input_returns_errors_without_poisoning_kernel() {
    for input in [
        "null",
        "{}",
        "{",
        r#"{"op":"curve_validate","curve":{"degree":-1}}"#,
        r#"{"op":"basis","degree":1,"knots":[0,0,1,1],"controlCount":2,"u":null}"#,
    ] {
        let v: Value = value_codec::from_str(&execute(input)).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["code"], "NURBS_INVALID_INPUT");
    }
    let mut c = circle();
    c.weights[1] = 0.;
    assert!(c.validate().is_err());
    c = circle();
    c.knots[3] = -1.;
    assert!(c.validate().is_err());
    assert_eq!(
        dispatch(json!({"op":"curve_evaluate","curve":circle(),"u":0.5})).unwrap()["derivative_status"],
        "available"
    );
}
#[test]
fn output_budget_is_checked_before_elevation() {
    let c = Curve {
        degree: 1,
        knots: std::iter::once(0.)
            .chain((0..30).map(|i| i as f64))
            .chain(std::iter::once(29.))
            .collect(),
        control_points: (0..30).map(|i| vec![i as f64, 0.]).collect(),
        weights: vec![1.; 30],
        periodic: false,
    };
    assert_eq!(c.elevate(25).unwrap_err().code, "NURBS_RESOURCE_LIMIT");
}

#[test]
fn brush_edits_move_controls_locally_and_preserve_rational_definition() {
    let c = circle();
    let b = geometry_ops::Brush {
        center: [1., 1., 0.],
        radius: 0.5,
        displacement: [0., 0., 2.],
    };
    let out = edit::brush_curve(&c, &b).unwrap();
    assert_eq!(out.degree, c.degree);
    assert_eq!(out.knots, c.knots);
    assert_eq!(out.weights, c.weights);
    assert_eq!(out.control_points[0], c.control_points[0]);
    assert_eq!(out.control_points[2], c.control_points[2]);
    near(&out.control_points[1], &[1., 1., 2.]);
    // Endpoints are interpolated by the clamped curve and remain untouched.
    near(&out.evaluate(0.).unwrap().point, &[1., 0., 0.]);
    near(&out.evaluate(1.).unwrap().point, &[0., 1., 0.]);
    assert!(out.evaluate(0.5).unwrap().point[2] > 0.);

    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    let sb = geometry_ops::Brush {
        center: [1., 1., 3.],
        radius: 0.5,
        displacement: [1., 0., 0.],
    };
    let sout = edit::brush_surface(&s, &sb).unwrap();
    assert_eq!(sout.knots_u, s.knots_u);
    assert_eq!(sout.knots_v, s.knots_v);
    assert_eq!(sout.weights, s.weights);
    let moved = s
        .control_points
        .iter()
        .flatten()
        .zip(sout.control_points.iter().flatten())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(moved, 1, "exactly one control point lies inside the brush");
    near(
        &sout.evaluate(0., 0.).unwrap().point,
        &s.evaluate(0., 0.).unwrap().point,
    );
}
#[test]
fn brush_edits_reject_invalid_brushes_and_inputs() {
    let c = circle();
    let bad = geometry_ops::Brush {
        center: [0.; 3],
        radius: 0.,
        displacement: [1., 0., 0.],
    };
    assert!(edit::brush_curve(&c, &bad).is_err());
    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    assert!(edit::brush_surface(&s, &bad).is_err());
    let ok = geometry_ops::Brush { radius: 1., ..bad };
    let mut flat = c.clone();
    flat.control_points = flat
        .control_points
        .iter()
        .map(|p| p[..2].to_vec())
        .collect();
    assert!(
        edit::brush_curve(&flat, &ok).is_err(),
        "2D controls are rejected"
    );
    let mut broken = c.clone();
    broken.knots.pop();
    assert!(edit::brush_curve(&broken, &ok).is_err());
}

#[test]
fn sculpt_targets_expose_polygon_and_net_structure() {
    let c = circle();
    let (positions, normals, rings) = edit::curve_sculpt_target(&c).unwrap();
    assert_eq!(positions.len(), 3);
    assert_eq!(rings, vec![vec![1], vec![0, 2], vec![1]]);
    near(&normals[0], &[0., 0., 0.]);
    near(
        &normals[1],
        &[
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
            0.,
        ],
    );
    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    let (_, snormals, srings) = edit::surface_sculpt_target(&s).unwrap();
    assert_eq!(
        snormals.len(),
        s.control_points.len() * s.control_points[0].len()
    );
    assert!(srings.iter().all(|r| (2..=4).contains(&r.len())));
    assert!(
        snormals
            .iter()
            .all(|n| (math_core::norm(*n) - 1.).abs() < 1e-9)
    );
}
#[test]
fn sculpt_edits_preserve_rational_definition() {
    use geometry_ops::{Falloff, SculptBrush, SculptKind};
    let c = circle();
    let draw = edit::sculpt_curve(
        &c,
        &SculptBrush::new(SculptKind::Draw { strength: 1. }, [1., 1., 0.], 0.5),
    )
    .unwrap();
    assert_eq!(draw.knots, c.knots);
    assert_eq!(draw.weights, c.weights);
    near(
        &draw.control_points[1],
        &[
            1. + std::f64::consts::FRAC_1_SQRT_2,
            1. + std::f64::consts::FRAC_1_SQRT_2,
            0.,
        ],
    );
    near(&draw.control_points[0], &c.control_points[0]);
    let smooth = edit::sculpt_curve(
        &c,
        &SculptBrush::new(SculptKind::Smooth { strength: 1. }, [1., 1., 0.], 0.5),
    )
    .unwrap();
    near(&smooth.control_points[1], &[0.5, 0.5, 0.]);
    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    let inflate = edit::sculpt_surface(
        &s,
        &SculptBrush {
            falloff: Falloff::Constant,
            ..SculptBrush::new(SculptKind::Inflate { strength: 0.5 }, [0.; 3], 100.)
        },
    )
    .unwrap();
    assert_eq!(inflate.knots_u, s.knots_u);
    assert_eq!(inflate.weights, s.weights);
    for (a, b) in s
        .control_points
        .iter()
        .flatten()
        .zip(inflate.control_points.iter().flatten())
    {
        let d: Vec<f64> = a.iter().zip(b).map(|(x, y)| y - x).collect();
        assert!((math_core::norm([d[0], d[1], d[2]]) - 0.5).abs() < 1e-9);
    }
    assert!(
        edit::sculpt_curve(
            &c,
            &SculptBrush::new(SculptKind::Flatten { strength: 1.5 }, [0.; 3], 1.)
        )
        .is_err()
    );
    let mut flat = c.clone();
    flat.control_points = flat
        .control_points
        .iter()
        .map(|p| p[..2].to_vec())
        .collect();
    assert!(
        edit::sculpt_curve(
            &flat,
            &SculptBrush::new(SculptKind::Draw { strength: 1. }, [0.; 3], 1.)
        )
        .is_err()
    );
}
