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
