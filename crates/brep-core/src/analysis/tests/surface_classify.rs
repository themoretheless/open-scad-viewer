use super::*;

fn budget() -> Budget {
    Budget::new(20_000_000, 8, 120_000).unwrap()
}

/// Exact cylinder side surface as a rational-quadratic B-spline patch.
fn cylinder_patch(radius: f64, height: f64) -> Surface {
    // Quarter-circle in u (rational degree 2), linear in v.
    let r = radius;
    let w = (0.5f64).sqrt();
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![[r, 0., 0.].to_vec(), [r, 0., height].to_vec()],
            vec![[r, r, 0.].to_vec(), [r, r, height].to_vec()],
            vec![[0., r, 0.].to_vec(), [0., r, height].to_vec()],
        ],
        weights: vec![vec![1., 1.], vec![w, w], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

/// Same quarter cylinder, degree-elevated to cubic in u (exact rational
/// circle) with the interior control points nudged by `noise`: the exact
/// type is a perturbed B-spline, but the geometry stays within ~`noise`
/// of a true cylinder — the best-fit path must still say Cylinder.
fn noisy_cylinder_patch(radius: f64, height: f64, noise: f64) -> Surface {
    // Degree-elevate the rational quadratic quarter arc 2→3 in
    // homogeneous coordinates: Q1 = P0/3 + 2P1/3, Q2 = 2P1/3 + P2/3.
    let w = (0.5f64).sqrt();
    let h0 = ([radius, 0., 0.], 1.);
    let h1 = ([radius * w, radius * w, 0.], w);
    let h2 = ([0., radius, 0.], 1.);
    let combine = |a: ([f64; 3], f64), b: ([f64; 3], f64), ta: f64| {
        let (p, q) = (
            [
                ta * a.0[0] + (1. - ta) * b.0[0],
                ta * a.0[1] + (1. - ta) * b.0[1],
                0.,
            ],
            ta * a.1 + (1. - ta) * b.1,
        );
        ([p[0] / q, p[1] / q], q)
    };
    let (q1, w1) = combine(h0, h1, 1. / 3.); // 1/3·P0 + 2/3·P1
    let (q2, w2) = combine(h2, h1, 1. / 3.); // 1/3·P2 + 2/3·P1
    let bump = |p: [f64; 2], dx: f64, dy: f64| [p[0] + dx, p[1] + dy];
    let q1 = bump(q1, noise, noise * 0.5);
    let q2 = bump(q2, -noise, noise * 0.5);
    let row = |p: [f64; 2]| vec![[p[0], p[1], 0.].to_vec(), [p[0], p[1], height].to_vec()];
    Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            row([radius, 0.]),
            row(q1),
            row(q2),
            row([0., radius]),
        ],
        weights: vec![vec![1., 1.], vec![w1, w1], vec![w2, w2], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn exact_cylinder_patch_classifies_with_axis_and_radius() {
    let surface = cylinder_patch(2., 5.);
    let result = classify_surface(&surface, 1e-6, &budget()).unwrap();
    assert_eq!(result.class, SurfaceClass::Cylinder);
    let axis = result.axis.unwrap();
    assert!((axis.direction[2].abs() - 1.).abs() < 1e-6, "axis along z: {axis:?}");
    assert!((result.radius.unwrap() - 2.).abs() < 1e-6);
    assert!(result.max_deviation < 1e-9, "exact patch fits exactly");
}

#[test]
fn noisy_cylinder_patch_classifies_via_best_fit() {
    // Noise ~1e-4 mm, tolerance 1e-2 mm: well inside the acceptance band.
    let surface = noisy_cylinder_patch(2., 5., 1e-4);
    let result = classify_surface(&surface, 1e-2, &budget()).unwrap();
    assert_eq!(
        result.class,
        SurfaceClass::Cylinder,
        "almost-cylinder must best-fit to Cylinder: {result:?}"
    );
    assert!((result.radius.unwrap() - 2.).abs() < 1e-2);
    // But with a tolerance below the noise level it must refuse.
    let strict = classify_surface(&surface, 1e-6, &budget()).unwrap();
    assert_eq!(strict.class, SurfaceClass::Freeform);
}

#[test]
fn planar_patch_classifies_as_plane() {
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![[0., 0., 1.].to_vec(), [0., 2., 1.].to_vec()],
            vec![[3., 0., 1.].to_vec(), [3., 2., 1.].to_vec()],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let result = classify_surface(&surface, 1e-9, &budget()).unwrap();
    assert_eq!(result.class, SurfaceClass::Plane);
}

#[test]
fn wavey_patch_is_freeform() {
    // Mild sinusoidal height field: no analytic primitive within 1e-6.
    let mut control = vec![];
    for i in 0..5 {
        let mut row = vec![];
        for j in 0..5 {
            let (x, y) = (i as f64, j as f64);
            row.push([x, y, 0.05 * (x * 1.7).sin() * (y * 1.3).cos()].to_vec());
        }
        control.push(row);
    }
    let knots = |n: usize| {
        // degree 3: knot count = n + 4; interior knots evenly spaced.
        let interior = n - 4 + 1;
        let mut k = vec![0.; 4];
        k.extend((1..interior).map(|i| i as f64 / interior as f64));
        k.extend([1.; 4]);
        k
    };
    let surface = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: knots(5),
        knots_v: knots(5),
        control_points: control,
        weights: vec![vec![1.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let result = classify_surface(&surface, 1e-6, &budget()).unwrap();
    assert_eq!(result.class, SurfaceClass::Freeform);
}

#[test]
fn rejects_nonpositive_tolerance_and_tight_budget() {
    let surface = cylinder_patch(1., 1.);
    assert!(classify_surface(&surface, 0., &budget()).is_err());
    assert!(classify_surface(&surface, -1., &budget()).is_err());
    let tight = Budget::with_iterations(3).unwrap();
    let error = classify_surface(&surface, 1e-6, &tight).unwrap_err();
    assert!(
        error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
        "budget exhaustion must be a typed resource error: {error:?}"
    );
}

#[test]
fn cuboid_face_areas_match_mass_properties() {
    let model = crate::cuboid([0.; 3], [2., 3., 5.]).unwrap();
    let mut sum = 0.;
    for face in 0..model.faces.len() {
        sum += face_area(&model, face, &budget()).unwrap();
    }
    let mass = crate::analysis::mass_properties(&model, 1e-9, 1_000_000).unwrap();
    let expected = 2. * (2. * 3. + 3. * 5. + 2. * 5.);
    assert!((sum - expected).abs() / expected < 1e-9, "sum={sum}");
    assert!(
        (sum - mass.surface_area_mm2).abs() / mass.surface_area_mm2 < 1e-6,
        "face areas must converge to the mass_properties integral: {sum} vs {}",
        mass.surface_area_mm2
    );
}

#[test]
fn cylinder_face_areas_match_mass_properties() {
    let model = crate::analytic::cylinder(1.5, 4.).unwrap();
    let mut sum = 0.;
    for face in 0..model.faces.len() {
        sum += face_area(&model, face, &budget()).unwrap();
    }
    let mass = crate::analysis::mass_properties(&model, 1e-9, 1_900_000).unwrap();
    assert!(
        (sum - mass.surface_area_mm2).abs() / mass.surface_area_mm2 < 1e-6,
        "cylinder: {sum} vs {}",
        mass.surface_area_mm2
    );
}

#[test]
fn face_area_rejects_out_of_range_and_tight_budget() {
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    assert!(face_area(&model, model.faces.len(), &budget()).is_err());
    let tight = Budget::with_iterations(5).unwrap();
    let error = face_area(&model, 0, &tight).unwrap_err();
    assert!(
        error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
        "{error:?}"
    );
}
