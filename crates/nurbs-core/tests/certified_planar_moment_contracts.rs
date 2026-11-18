use nurbs_core::{curve::Curve, trim::planar_mass_bounds::bound_planar_boundary_moments};

fn ellipse() -> Vec<Curve> {
    let poles = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    (0..4)
        .map(|quarter| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: poles[2 * quarter..=2 * quarter + 2]
                .iter()
                .map(|&[x, y]| vec![3. + 2. * x, -2. + y])
                .collect(),
            weights: vec![1., 1., 2.],
            periodic: false,
        })
        .collect()
}

#[test]
fn encloses_signed_moments_of_a_rational_ellipse_about_its_center() {
    let pi = std::f64::consts::PI;
    let expected = [2. * pi, 0., 0., 2. * pi, 0., pi / 2.];
    let contours = [ellipse()];
    let report = bound_planar_boundary_moments(&contours, [3., -2.], [0.02; 3], 8192).unwrap();
    assert!(report.converged, "{:?}", report.moments);
    assert!(report.cells > 4 && report.cells <= 8192);
    for ([lo, hi], expected) in report.moments.into_iter().zip(expected) {
        assert!(
            lo <= expected && expected <= hi && hi - lo <= 0.02,
            "{lo}..{hi}: {expected}"
        );
    }
}

#[test]
fn preserves_outward_bounds_when_the_cell_budget_is_exhausted() {
    let contours = [ellipse()];
    assert!(bound_planar_boundary_moments(&contours, [3., -2.], [1e-6; 3], 3).is_err());
    let report = bound_planar_boundary_moments(&contours, [3., -2.], [1e-6; 3], 4).unwrap();
    assert_eq!(report.cells, 4);
    assert!(!report.converged);
    let [lo, hi] = report.moments[0];
    let area = 2. * std::f64::consts::PI;
    assert!(lo <= area && area <= hi);
}

#[test]
fn rejects_unclosed_contours() {
    let mut open = ellipse();
    open.pop();
    assert!(bound_planar_boundary_moments(&[open], [0., 0.], [1e-3; 3], 128).is_err());
}
