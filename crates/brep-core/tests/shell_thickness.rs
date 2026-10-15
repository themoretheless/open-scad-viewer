use brep_core::shell_thickness::{self, Limits, Stop};
use nurbs_core::surface::Surface;
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 5.], vec![0., 3., 5.]],
            vec![vec![2., 0., 5.], vec![2., 3., 5.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn cylinder() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 0., 2.]],
            vec![vec![1., 1., 0.], vec![1., 1., 2.]],
            vec![vec![0., 1., 0.], vec![0., 1., 2.]],
        ],
        weights: vec![
            vec![1.; 2],
            vec![std::f64::consts::FRAC_1_SQRT_2; 2],
            vec![1.; 2],
        ],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn plane_skins_have_independent_thickness_volume_and_exact_owned_source() {
    let source = plane();
    let before = source.clone();
    for h in [0.125, -0.25] {
        let r = shell_thickness::thicken_patch(&source, h, 1e-6, Limits::default()).unwrap();
        assert_eq!(r.stop, Stop::ConstructedSampledThickness);
        assert!(r.agreement.as_ref().unwrap().complete);
        assert_eq!(r.regularity.len(), 6);
        assert!(r.regularity.iter().all(|r| r.spanwise_regular));
        assert!(
            r.continuous_thickness_error_upper.is_none()
                && !r.rounding_inclusive_thickness_certified
                && !r.solid_geometry_certified
        );
        let model = r.model.unwrap();
        assert_eq!(model.faces[0].surface, source);
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        assert_eq!(model.faces.len(), 6);
        assert_eq!(model.edges.len(), 12);
        assert_eq!(model.bodies.len(), 1);
        for (u, v) in [(0., 0.), (0.13, 0.8), (0.8, 0.2), (1., 1.)] {
            let p = model.faces[1].surface.evaluate(u, v).unwrap().point;
            assert!((p[2] - (5. + h)).abs() < 1e-14);
        }
        let mass = brep_core::analysis::mass_properties(&model, 1e-9, 10000).unwrap();
        assert!((mass.signed_volume_mm3 - 6. * h.abs()).abs() < 1e-10);
    }
    assert_eq!(source, before);
}
#[test]
fn rational_curved_skin_retains_sampled_assurance_and_continuous_saved_seams() {
    let source = cylinder();
    let r = shell_thickness::thicken_patch(&source, 0.02, 0.005, Limits::default()).unwrap();
    assert_eq!(
        r.stop,
        Stop::ConstructedSampledThickness,
        "stop={:?} regularities={} sampled={:?}",
        r.stop,
        r.regularity.len(),
        r.max_sampled_thickness_deviation
    );
    assert!(r.max_sampled_thickness_deviation.unwrap() < 1e-10);
    assert!(r.continuous_thickness_error_upper.is_none());
    let model = r.model.unwrap();
    assert_eq!(model.faces[0].surface, source);
    assert!(r.agreement.unwrap().complete);
    for i in 0..=16 {
        let u = i as f64 / 16.;
        let p = source.evaluate(u, 0.37).unwrap().point;
        let q = model.faces[1].surface.evaluate(u, 0.37).unwrap().point;
        let radius = p[0].hypot(p[1]);
        let ideal = [p[0] * (1. + 0.02 / radius), p[1] * (1. + 0.02 / radius)];
        let error = (q[0] - ideal[0]).hypot(q[1] - ideal[1]);
        assert!(error < 0.005);
    }
    let mut freeform = source.clone();
    freeform.control_points[1][0][0] += 0.05;
    freeform.control_points[1][1][0] += 0.05;
    let approximate =
        shell_thickness::thicken_patch(&freeform, 0.02, 0.01, Limits::default()).unwrap();
    assert_eq!(approximate.stop, Stop::ConstructedSampledThickness);
    assert!(approximate.max_sampled_thickness_deviation.unwrap() > 0.0001);
    let saved = approximate.model.unwrap();
    let w = std::f64::consts::FRAC_1_SQRT_2;
    for i in 0..=16 {
        let t = i as f64 / 16.;
        let a = 1. - t;
        let d = a * a + 2. * w * t * a + t * t;
        let dd = -2. * a + 2. * w * (1. - 2. * t) + 2. * t;
        let n = [a * a + 2. * w * 1.05 * t * a, 2. * w * t * a + t * t];
        let dn = [
            -2. * a + 2. * w * 1.05 * (1. - 2. * t),
            2. * w * (1. - 2. * t) + 2. * t,
        ];
        let tangent = [
            (dn[0] * d - n[0] * dd) / (d * d),
            (dn[1] * d - n[1] * dd) / (d * d),
        ];
        let speed = tangent[0].hypot(tangent[1]);
        let ideal = [
            n[0] / d + 0.02 * tangent[1] / speed,
            n[1] / d - 0.02 * tangent[0] / speed,
        ];
        let q = saved.faces[1].surface.evaluate(t, 0.37).unwrap().point;
        assert!((q[0] - ideal[0]).hypot(q[1] - ideal[1]) < 0.01);
    }
    let r = shell_thickness::thicken_patch(&freeform, 0.02, 1e-10, Limits::default()).unwrap();
    assert_eq!(r.stop, Stop::SampledToleranceNotMet);
    assert!(r.model.is_none());
    assert!(r.offset_fallback_reason.is_some());
}
#[test]
fn unsupported_singular_inputs_and_each_shared_budget_refuse_a_body() {
    let s = plane();
    for h in [0., f64::NAN, 1e7] {
        assert!(shell_thickness::thicken_patch(&s, h, 1e-6, Limits::default()).is_err());
    }
    let r = shell_thickness::thicken_patch(
        &s,
        0.125,
        1e-6,
        Limits {
            regularity_cells: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(r.model.is_none());
    assert_eq!(r.stop, Stop::RegularityUnresolved);
    let r = shell_thickness::thicken_patch(
        &s,
        0.125,
        1e-6,
        Limits {
            agreement_cells: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(r.model.is_none());
    assert_eq!(r.stop, Stop::AgreementUnresolved);
    let mut singular = s.clone();
    for row in &mut singular.control_points {
        for p in row {
            p[0] = 0.;
        }
    }
    assert!(
        shell_thickness::thicken_patch(&singular, 0.125, 1e-6, Limits::default())
            .unwrap()
            .model
            .is_none()
    );
    let mut periodic = s.clone();
    periodic.periodic_u = true;
    assert!(shell_thickness::thicken_patch(&periodic, 0.125, 1e-6, Limits::default()).is_err());
}
