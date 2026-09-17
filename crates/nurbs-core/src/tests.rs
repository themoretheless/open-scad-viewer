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
fn foundation_curve_certificate_is_outward_and_positive() {
    let c = circle();
    let certificate = foundation::certify_curve(&c, None).unwrap();
    assert_eq!(certificate["version"], "nurbs-foundation/1");
    let spans = certificate["spans"].as_array().unwrap();
    assert_eq!(spans.len(), 1);
    assert!(spans[0]["denominatorLower"].as_f64().unwrap() > 0.);
    assert!(spans[0]["min"][0].as_f64().unwrap() < 0.);
    assert!(spans[0]["max"][1].as_f64().unwrap() > 1.);
    assert_eq!(
        spans[0]["regularity"]["classification"],
        "certified_regular"
    );
}

#[test]
fn foundation_handles_adversarial_positive_weights_and_surface_cells() {
    for exponent in -6..=6 {
        let mut c = circle();
        c.weights = vec![1., 10_f64.powi(exponent), 1.];
        let certificate = foundation::certify_curve(&c, None).unwrap();
        assert!(certificate["spans"][0]["denominatorLower"]
            .as_f64()
            .unwrap()
            > 0.);
        for i in 0..=32 {
            let point = c.evaluate(i as f64 / 32.).unwrap().point;
            for axis in 0..3 {
                assert!(point[axis] >= certificate["spans"][0]["min"][axis].as_f64().unwrap());
                assert!(point[axis] <= certificate["spans"][0]["max"][axis].as_f64().unwrap());
            }
        }
    }
    let surface = surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 2., 0.]],
            vec![vec![3., 0., 0.], vec![3., 2., 0.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 4.]],
        periodic_u: false,
        periodic_v: false,
    };
    let certificate = foundation::certify_surface(&surface, None).unwrap();
    assert_eq!(certificate["cells"].as_array().unwrap().len(), 1);
    assert_eq!(
        certificate["cells"][0]["normalRegularity"]["classification"],
        "certified_planar_regular"
    );
}

#[test]
fn certified_projection_reports_uniqueness_and_ambiguity() {
    let line = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let projected = foundation::project_curve(&line, &[0.5, 1.], None).unwrap();
    assert_eq!(projected["status"], "unique");
    assert!((projected["candidates"][0]["point"][0].as_f64().unwrap() - 0.5).abs() < 1e-14);

    let polyline =
        Curve::from_polyline(vec![vec![-1., 0.], vec![0., 1.], vec![1., 0.]]).unwrap();
    let ambiguous = foundation::project_curve(&polyline, &[0., 0.], None).unwrap();
    assert_eq!(ambiguous["status"], "nonunique_or_unresolved");
    assert_eq!(ambiguous["candidates"].as_array().unwrap().len(), 2);
}

#[test]
fn interpolation_approximation_and_exact_reparameterization_are_certified() {
    let interpolation = foundation::interpolate_polyline(
        vec![vec![0., 0.], vec![1., 2.], vec![3., 4.]],
        None,
    )
    .unwrap();
    assert_eq!(
        interpolation["certificate"]["dataSiteErrorUpper"].as_f64(),
        Some(0.)
    );
    let approximation = foundation::approximate_curve(&circle(), None).unwrap();
    assert!(approximation["certificate"]["hausdorffErrorUpper"]
        .as_f64()
        .unwrap()
        .is_finite());

    let periodic = Curve {
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
    let result = foundation::reparameterize_curve(&periodic, [-3., 5.], None).unwrap();
    let mapped: Curve = value_codec::from_value(result["curve"].clone()).unwrap();
    assert!(mapped.periodic);
    for i in 0..=32 {
        let old_u = 2. + 4. * i as f64 / 32.;
        let new_u = -3. + 8. * i as f64 / 32.;
        near(
            &periodic.evaluate(old_u).unwrap().point,
            &mapped.evaluate(new_u).unwrap().point,
        );
    }
}

#[test]
fn foundation_boundary_and_mutation_limits_are_deterministic() {
    let degree = 25;
    let controls = 26;
    let c = Curve {
        degree,
        knots: std::iter::repeat_n(0., degree + 1)
            .chain(std::iter::repeat_n(1., degree + 1))
            .collect(),
        control_points: (0..controls)
            .map(|i| vec![i as f64 / 25., (i % 3) as f64])
            .collect(),
        weights: (0..controls)
            .map(|i| 10_f64.powi((i as i32 % 13) - 6))
            .collect(),
        periodic: false,
    };
    assert!(foundation::certify_curve(&c, None).is_ok());
    let mut too_high = c.clone();
    too_high.degree = 26;
    too_high.knots.push(1.);
    assert_eq!(too_high.validate().unwrap_err().code, "NURBS_INVALID_INPUT");
    let mut nonpositive = c;
    nonpositive.weights[7] = 0.;
    assert_eq!(
        nonpositive.validate().unwrap_err().code,
        "NURBS_INVALID_INPUT"
    );
}
