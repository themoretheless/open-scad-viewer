use brep_core::{Model, analysis::mass_properties, boolean};
use std::f64::consts::PI;

fn translate(mut model: Model, delta: [f64; 3]) -> Model {
    for vertex in &mut model.vertices {
        for i in 0..3 {
            vertex.point[i] += delta[i];
        }
    }
    for edge in &mut model.edges {
        for point in &mut edge.curve.control_points {
            for i in 0..3 {
                point[i] += delta[i];
            }
        }
    }
    for face in &mut model.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            for i in 0..3 {
                point[i] += delta[i];
            }
        }
    }
    model.validate().unwrap();
    model
}

fn cylinder(radius: f64, center: [f64; 2], z: f64, height: f64) -> Model {
    translate(
        brep_core::cylinder(radius, height).unwrap(),
        [center[0], center[1], z],
    )
}

/// Independent closed-form two-disk lens area for transverse circles.
fn lens(a: f64, b: f64, distance: f64) -> f64 {
    assert!(distance > (a - b).abs() && distance < a + b);
    a * a * ((distance * distance + a * a - b * b) / (2. * distance * a)).acos()
        + b * b * ((distance * distance + b * b - a * a) / (2. * distance * b)).acos()
        - 0.5
            * ((-distance + a + b) * (distance + a - b) * (distance - a + b) * (distance + a + b))
                .sqrt()
}

fn roundtrip(model: &Model) -> Model {
    let encoded = value_codec::to_string(model).unwrap();
    let decoded: Model = value_codec::from_str(&encoded).unwrap();
    decoded.validate().unwrap();
    assert_eq!(encoded, value_codec::to_string(&decoded).unwrap());
    decoded
}

fn check(model: &Model, volume: f64, bodies: usize, cap_holes: &[usize]) {
    let report = model.validate().unwrap();
    assert_eq!(report.boundary_edge_count, 0);
    assert_eq!(report.body_count, bodies);
    let mut holes: Vec<_> = model
        .faces
        .iter()
        .filter(|face| !face.holes.is_empty())
        .map(|face| face.holes.len())
        .collect();
    holes.sort();
    assert_eq!(holes, cap_holes);
    let mass = mass_properties(model, 1e-7, 400_000).unwrap();
    assert!(mass.signed_volume_mm3 > 0., "Outward orientation lost");
    assert!(
        (mass.signed_volume_mm3 - volume).abs() < volume * 2e-7,
        "Analytic volume {volume}, B-rep volume {}",
        mass.signed_volume_mm3
    );
    assert!(model.edges.iter().any(|edge| {
        edge.curve.degree == 2
            && edge
                .curve
                .weights
                .iter()
                .any(|w| *w != edge.curve.weights[0])
    }));
    roundtrip(model);
}

fn check_circular_carriers(model: &Model, circles: &[([f64; 2], f64)]) {
    for edge in model.edges.iter().filter(|edge| edge.curve.degree == 2) {
        let domain = edge.curve.domain();
        for fraction in [0.17, 0.53, 0.89] {
            let t = domain[0] + fraction * (domain[1] - domain[0]);
            let p = edge.curve.evaluate(t).unwrap().point;
            assert!(
                circles.iter().any(|(center, radius)| {
                    ((p[0] - center[0]).hypot(p[1] - center[1]) - radius).abs() < 1e-8
                }),
                "Retained circular edge left both source carriers: {p:?}"
            );
        }
    }
}

#[test]
fn unequal_radius_transverse_circle_booleans_match_analytic_lenses() {
    let height = 2.7;
    let center_a = [-2., 1.];
    for (ra, rb, distance, angle) in [
        (3., 2., 2., 0.37_f64),
        (4., 1.5, 3., 1.13),
        (2.5, 3.5, 2.7, 2.41),
        (1.2, 2.1, 1.5, -0.62),
    ] {
        let center_b = [
            center_a[0] + distance * angle.cos(),
            center_a[1] + distance * angle.sin(),
        ];
        let a = cylinder(ra, center_a, 1.3, height);
        let b = cylinder(rb, center_b, 1.3, height);
        let overlap = lens(ra, rb, distance);
        for (operation, area) in [
            ("union", PI * (ra * ra + rb * rb) - overlap),
            ("intersection", overlap),
            ("difference", PI * ra * ra - overlap),
        ] {
            let result = boolean(&a, &b, operation).unwrap_or_else(|error| {
                panic!("rA={ra}, rB={rb}, d={distance}, angle={angle}, {operation}: {error:?}")
            });
            check(&result, area * height, 1, &[]);
            check_circular_carriers(&result, &[(center_a, ra), (center_b, rb)]);
        }
    }
}

#[test]
fn contained_disk_and_tube_cuts_preserve_hole_topology() {
    let a = cylinder(3., [0., 0.], 0., 3.);
    let b = cylinder(1., [0.4, 0.7], 0., 3.);
    check(
        &boolean(&a, &b, "difference").unwrap(),
        24. * PI,
        1,
        &[1, 1],
    );
    check(&boolean(&a, &b, "intersection").unwrap(), 3. * PI, 1, &[]);

    let tube = brep_core::tube(4., 2., 3.).unwrap();
    let opening = cylinder(2., [3., 0.], -1., 5.);
    let open_tube = boolean(&tube, &opening, "difference").unwrap();
    check(
        &open_tube,
        3. * (12. * PI - lens(4., 2., 3.) + lens(2., 2., 3.)),
        1,
        &[],
    );
    check_circular_carriers(
        &open_tube,
        &[([0., 0.], 4.), ([0., 0.], 2.), ([3., 0.], 2.)],
    );

    let drill = cylinder(0.4, [3., 0.], 0., 3.);
    let drilled = boolean(&tube, &drill, "difference").unwrap();
    check(&drilled, 3. * (12. - 0.16) * PI, 1, &[2, 2]);
    let restored = boolean(&roundtrip(&drilled), &drill, "union").unwrap();
    check(&restored, 36. * PI, 1, &[1, 1]);
}

#[test]
fn decoded_curved_union_can_be_reused_as_a_boolean_operand() {
    let a = cylinder(3., [-2., 1.], 1.3, 2.7);
    let angle = 0.37_f64;
    let b = cylinder(
        2.,
        [-2. + 2. * angle.cos(), 1. + 2. * angle.sin()],
        1.3,
        2.7,
    );
    let joined = boolean(&a, &b, "union").unwrap();
    let remaining = boolean(&roundtrip(&joined), &roundtrip(&b), "difference").unwrap();
    check(&remaining, 2.7 * (9. * PI - lens(3., 2., 2.)), 1, &[]);
}

#[test]
fn near_coincident_curved_boundaries_fail_explicitly_without_mutating_inputs() {
    let a = cylinder(3., [0., 0.], 0., 2.);
    let b = cylinder(3., [1e-10, 0.], 0., 2.);
    let before_a = value_codec::to_string(&a).unwrap();
    let before_b = value_codec::to_string(&b).unwrap();
    let error = boolean(&a, &b, "union").unwrap_err();
    assert_eq!(error.code, "BREP_AMBIGUOUS_PLANAR_TRIM", "{error:?}");
    assert_eq!(before_a, value_codec::to_string(&a).unwrap());
    assert_eq!(before_b, value_codec::to_string(&b).unwrap());
}
