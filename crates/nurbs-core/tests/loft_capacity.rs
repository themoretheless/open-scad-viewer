use nurbs_core::{loft_alignment, natural_loft, primitives, surface::Surface};
#[test]
fn natural_loft_retains_all_86_sections_at_256_control_boundary() {
    let sections = (0..86)
        .map(|i| primitives::line([0., 0., i as f64], [1., 0., i as f64]).unwrap())
        .collect::<Vec<_>>();
    let parameters = (0..86).map(|i| i as f64).collect::<Vec<_>>();
    let surface = natural_loft::interpolate(&sections, &parameters).unwrap();
    assert_eq!(surface.control_points[0].len(), 256);
    for i in 0..86 {
        let p = surface.evaluate(0.37, i as f64 / 85.).unwrap().point;
        assert!((p[0] - 0.37).abs() < 1e-11);
        assert!((p[2] - i as f64).abs() < 1e-10);
    }
    let mut more = sections;
    more.push(primitives::line([0., 0., 86.], [1., 0., 86.]).unwrap());
    let mut parameters = parameters;
    parameters.push(86.);
    assert!(natural_loft::interpolate(&more, &parameters).is_err());
    let mut invalid = surface.clone();
    for row in &mut invalid.control_points {
        row.push(row.last().unwrap().clone());
    }
    for row in &mut invalid.weights {
        row.push(1.);
    }
    invalid.knots_v.push(1.);
    assert!(invalid.validate().is_err());
    let _: Surface = surface;
}
#[test]
fn twelve_sections_and_ten_guides_cross_previous_limits_without_refitting() {
    let sections = (0..12)
        .map(|i| primitives::line([0., 0., i as f64], [1., 0., i as f64]).unwrap())
        .collect::<Vec<_>>();
    let parameters = (0..12).map(|i| i as f64).collect::<Vec<_>>();
    let guides = (1..=10)
        .map(|i| {
            let u = i as f64 / 11.;
            primitives::line([u, 0., 0.], [u, 0., 11.]).unwrap()
        })
        .collect::<Vec<_>>();
    let report = loft_alignment::interpolate(&sections, &parameters, &guides, 1e-6).unwrap();
    assert!(report.surface.control_points.len() > 32);
    assert!(report.surface.control_points[0].len() > 32);
    for (i, u) in report.guide_parameters.iter().enumerate() {
        let p = report.surface.evaluate(*u, 0.37).unwrap().point;
        assert!((p[0] - (i + 1) as f64 / 11.).abs() < 1e-8);
        assert!((p[2] - 11. * 0.37).abs() < 1e-8);
    }
}
