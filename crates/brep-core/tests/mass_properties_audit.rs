use brep_core::{analysis::mass_properties, cylinder};

#[test]
fn green_quadrature_does_not_skip_narrow_surface_v_knot_spans() {
    let mut model = cylinder(3., 8.).unwrap();
    let knots = vec![0., 0., 0.500001, 0.500002, 1., 1.];
    let heights = [0., 0.000001, 7.999999, 8.];
    for face in model.faces.iter_mut().filter(|f| f.surface.degree_u == 2) {
        let surface = &mut face.surface;
        surface.knots_v = knots.clone();
        for row in &mut surface.control_points {
            let p = row[0].clone();
            *row = heights.iter().map(|z| vec![p[0], p[1], *z]).collect();
        }
        for row in &mut surface.weights {
            *row = vec![row[0]; 4];
        }
    }
    for edge in model.edges.iter_mut().filter(|e| e.curve.degree == 1) {
        let p = edge.curve.control_points[0].clone();
        assert!(p[2] == 0.);
        edge.curve.knots = knots.clone();
        edge.curve.control_points = heights.iter().map(|z| vec![p[0], p[1], *z]).collect();
        edge.curve.weights = vec![1.; 4];
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    let measured = mass_properties(&model, 1e-7, 2_000_000).unwrap();
    let expected = 72. * std::f64::consts::PI;
    assert!(
        (measured.signed_volume_mm3 / expected - 1.).abs() < 1e-7,
        "False convergence: expected {expected}, got {}, estimated error {}",
        measured.signed_volume_mm3,
        measured.volume_error_estimate_mm3
    );
}

#[test]
fn rational_trim_knot_crossings_preserve_sphere_mass() {
    use nurbs_core::surface::Axis;
    let mut model = brep_core::sphere(3.).unwrap();
    for face in &mut model.faces {
        face.surface = face
            .surface
            .edit_axis(Axis::U, |c| c.insert(0.3, 1)?.insert(0.6, 1))
            .unwrap();
        face.surface = face
            .surface
            .edit_axis(Axis::V, |c| c.insert(0.3, 1)?.insert(0.6, 1))
            .unwrap();
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    let measured = mass_properties(&model, 1e-7, 2_000_000).unwrap();
    let expected = 36. * std::f64::consts::PI;
    assert!((measured.signed_volume_mm3 / expected - 1.).abs() < 1e-7);
    assert!((measured.surface_area_mm2 / expected - 1.).abs() < 1e-7);
}

#[test]
fn rational_weight_concentration_cannot_falsely_report_converged_volume() {
    let mut model = cylinder(3., 8.).unwrap();
    let ratio = 7e11;
    for face in model.faces.iter_mut().filter(|f| f.surface.degree_u == 2) {
        for row in &mut face.surface.weights {
            row[1] *= ratio;
        }
    }
    for edge in model.edges.iter_mut().filter(|e| e.curve.degree == 1) {
        edge.curve.weights[1] *= ratio;
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    match mass_properties(&model, 1e-7, 2_000_000) {
        Ok(measured) => {
            let expected = 72. * std::f64::consts::PI;
            assert!(
                (measured.signed_volume_mm3 / expected - 1.).abs() < 1e-7,
                "Concentrated rational weights falsely converged: expected {expected}, got {}",
                measured.signed_volume_mm3
            );
        }
        Err(error) => assert!(matches!(
            error.code,
            "BREP_ANALYSIS_INDETERMINATE" | "BREP_RESOURCE_LIMIT"
        )),
    }
}
