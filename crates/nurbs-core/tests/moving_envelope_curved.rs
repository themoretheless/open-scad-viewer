use nurbs_core::{curve::Curve, moving_envelope, moving_radius, surface::Surface};
fn torus(major: f64, r: f64) -> (Surface, Curve, Curve) {
    let arc = [[1., 0.], [1., 1.], [0., 1.]];
    let a = [1., 0.5f64.sqrt(), 1.];
    let surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: arc
            .iter()
            .map(|e| {
                [(0., -1.), (1., -1.), (1., 0.)]
                    .iter()
                    .map(|(x, z)| vec![(major + r * x) * e[0], (major + r * x) * e[1], r + r * z])
                    .collect()
            })
            .collect(),
        weights: a
            .iter()
            .map(|u| a.iter().map(|v| u * v).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let c = Curve {
        degree: 2,
        knots: surface.knots_u.clone(),
        control_points: arc
            .iter()
            .map(|e| vec![major * e[0], major * e[1], r])
            .collect(),
        weights: a.to_vec(),
        periodic: false,
    };
    let r = Curve::from_polyline(vec![vec![r, 0.]; 2]).unwrap();
    (surface, c, r)
}
fn limits() -> moving_envelope::Limits {
    moving_envelope::Limits {
        max_sine_squared: 1e-10,
        cells: 2048,
        surface_spans: 8192,
        center_spans: 8192,
        radial_work: 1000000,
    }
}
fn qualify(s: &Surface, c: &Curve, r: &Curve) -> moving_envelope::Report {
    let radius = moving_radius::qualify(s, c, r, 1e-8, 1024, 1000000)
        .unwrap()
        .certificate
        .unwrap();
    moving_envelope::qualify(radius, limits()).unwrap()
}
#[test]
fn curved_rational_spine_torus_has_full_original_radial_normals() {
    let (s, c, r) = torus(3., 0.5);
    let before = (s.clone(), c.clone(), r.clone());
    let report = qualify(&s, &c, &r);
    assert!(
        report.envelope.is_some(),
        "{} {:?} cells {} work {}",
        report.reason,
        report.uncertain_uv,
        report.cells,
        report.radial_work
    );
    let q = report.envelope.unwrap();
    assert!(q.sine_squared_bounds()[1] <= 1e-10);
    assert_eq!(q.radius().surface(), &s);
    assert_eq!((s, c, r), before);
}
#[test]
fn oblique_placement_and_nonunit_domains_keep_original_curved_envelope() {
    let (mut s, mut c, mut r) = torus(2.5, 0.3);
    let place = |p: &mut Vec<f64>| {
        let q = p.clone();
        *p = vec![
            7. + 0.6 * q[0] + 0.8 * q[2],
            q[1] - 3.,
            11. - 0.8 * q[0] + 0.6 * q[2],
        ];
    };
    for p in s.control_points.iter_mut().flatten() {
        place(p);
    }
    for p in &mut c.control_points {
        place(p);
    }
    for k in s.knots_u.iter_mut().chain(&mut c.knots).chain(&mut r.knots) {
        *k = 10. + 10. * *k;
    }
    for k in &mut s.knots_v {
        *k = -3. + 5. * *k;
    }
    let report = qualify(&s, &c, &r);
    assert!(
        report.envelope.is_some(),
        "{} {:?} {}",
        report.reason,
        report.uncertain_uv,
        report.radial_work
    );
}
#[test]
fn true_variable_radius_sphere_envelope_has_the_characteristic_circle_shift() {
    let a = [1., 0.5f64.sqrt(), 1.];
    let xy = [[1., 0.], [1., 1.], [0., 1.]];
    let s = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..2)
            .map(|i| {
                let r = 1. + 3. * i as f64;
                xy.iter()
                    .map(|p| vec![0.8 * r * p[0], 0.8 * r * p[1], 5. * i as f64 - 0.6 * r])
                    .collect()
            })
            .collect(),
        weights: vec![a.to_vec(); 2],
        periodic_u: false,
        periodic_v: false,
    };
    let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., 5.]]).unwrap();
    let r = Curve::from_polyline(vec![vec![1., 0.], vec![4., 0.]]).unwrap();
    let report = qualify(&s, &c, &r);
    assert!(
        report.envelope.is_some(),
        "{} {:?} {}",
        report.reason,
        report.uncertain_uv,
        report.radial_work
    );
}
#[test]
fn radius_correct_tilted_sections_are_not_a_sphere_envelope() {
    let a = [1., 0.5f64.sqrt(), 1.];
    let xy = [[1., 0.], [1., 1.], [0., 1.]];
    let s = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..2)
            .map(|i| {
                xy.iter()
                    .map(|p| vec![p[0], 0.6 * p[1], i as f64 + 0.8 * p[1]])
                    .collect()
            })
            .collect(),
        weights: vec![a.to_vec(); 2],
        periodic_u: false,
        periodic_v: false,
    };
    let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., 1.]]).unwrap();
    let r = Curve::from_polyline(vec![vec![1., 0.]; 2]).unwrap();
    let report = qualify(&s, &c, &r);
    assert!(report.envelope.is_none());
    assert!(report.angular_break_sine_squared.unwrap()[0] > 1e-10);
}
#[test]
fn singular_poles_and_exhausted_work_publish_no_envelope() {
    let (s, c, r) = torus(0., 0.5);
    let radius = moving_radius::qualify(&s, &c, &r, 1e-8, 100, 1000000)
        .unwrap()
        .certificate
        .unwrap();
    let mut o = limits();
    o.cells = 32;
    assert!(
        moving_envelope::qualify(radius, o)
            .unwrap()
            .envelope
            .is_none()
    );
    let (s, c, r) = torus(3., 0.5);
    let radius = moving_radius::qualify(&s, &c, &r, 1e-8, 100, 1000000)
        .unwrap()
        .certificate
        .unwrap();
    let mut o = limits();
    o.radial_work = 1;
    assert!(
        moving_envelope::qualify(radius, o)
            .unwrap()
            .envelope
            .is_none()
    );
}

#[test]
fn independently_inserted_original_surface_and_center_knots_are_all_covered() {
    let (s, c, r) = torus(3., 0.5);
    let s = nurbs_core::surface_edit::insert(&s, nurbs_core::surface::Axis::U, 0.37, 1).unwrap();
    let c = c.insert(0.61, 1).unwrap();
    let report = qualify(&s, &c, &r);
    assert!(
        report.envelope.is_some(),
        "{} {:?} work {}",
        report.reason,
        report.uncertain_uv,
        report.radial_work
    );
    assert!(report.surface_spans >= 3 && report.center_spans >= 3);
    assert!(report.radial_work <= 1000000);
}
