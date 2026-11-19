use super::*;
use crate::healing_report::HEALING_REPORT_VERSION;

fn options(tolerance_mm: f64, area_delta_cap_mm2: f64) -> CanonicalCastBudget {
    CanonicalCastBudget {
        tolerance_mm,
        area_delta_cap_mm2,
        max_candidates: 8,
        ..CanonicalCastBudget::default()
    }
}

/// A cubic patch whose authored boundary is exactly the original bottom
/// square. Four symmetric interior poles make the best-fit plane z = 0,
/// while the carrier itself is not planar.
fn near_plane(amplitude: f64, interior_weight: f64) -> Model {
    let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
    model.faces[0].surface = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: [vec![0.; 4], vec![1.; 4]].concat(),
        knots_v: [vec![0.; 4], vec![1.; 4]].concat(),
        control_points: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        let interior = matches!((i, j), (1, 1) | (1, 2) | (2, 1) | (2, 2));
                        let sign = if (i + j) % 2 == 0 { 1. } else { -1. };
                        vec![
                            2. * j as f64 / 3.,
                            2. * i as f64 / 3.,
                            if interior { sign * amplitude } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        if matches!((i, j), (1, 1) | (1, 2) | (2, 1) | (2, 2)) {
                            interior_weight
                        } else {
                            1.
                        }
                    })
                    .collect()
            })
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    model.validate().unwrap();
    model
}

#[test]
fn near_plane_success_reprojects_trims_preserves_ids_and_round_trips() {
    let model = near_plane(1e-4, 1.);
    let ids = model.1.clone();
    let edges = model.edges.clone();
    let vertices = model.vertices.clone();
    let old_pcurves = model.loops[model.faces[0].outer].coedges.clone();
    let area_before = face_area(&model, 0, &CanonicalCastBudget::default().work).unwrap();
    let run = cast_canonical_surfaces(&model, options(1e-3, 1e-6)).unwrap();
    let area_after = face_area(&run.model, 0, &CanonicalCastBudget::default().work).unwrap();

    assert!(exact_affine_plane(&run.model.faces[0].surface));
    assert!((area_after - area_before).abs() < 1e-6);
    assert!(
        area_after <= area_before + 1e-9,
        "{area_before} -> {area_after}"
    );
    assert_eq!(run.model.1, ids);
    assert_eq!(run.model.edges, edges);
    assert_eq!(run.model.vertices, vertices);
    assert_eq!(run.report.conversions.len(), 1);
    assert_eq!(run.report.conversions[0].entity_id, model.1.faces[0]);
    assert_eq!(run.report.conversions[0].to, "plane");
    assert_eq!(run.report.stages[0].stage, "canonicalCast");
    assert_eq!(run.report.stages[0].fixed, 1);
    assert!(run.report.gaps.before_mm >= 1e-4);
    assert!(run.report.gaps.after_mm <= 1e-12);
    assert!(
        run.model.loops[run.model.faces[0].outer]
            .coedges
            .iter()
            .zip(&old_pcurves)
            .any(|(after, before)| after.pcurve != before.pcurve)
    );
    run.model.validate().unwrap();
    run.model.validate_continuous(100_000).unwrap();
    assert_eq!(HEALING_REPORT_VERSION, 3);
    let json = run.report.to_json().unwrap();
    assert_eq!(HealingReport::from_json(&json).unwrap(), run.report);
}

#[test]
fn replay_is_bit_identical_and_converts_nothing() {
    let model = near_plane(1e-4, 1.);
    let first = cast_canonical_surfaces(&model, options(1e-3, 1.)).unwrap();
    let second = cast_canonical_surfaces(&first.model, options(1e-3, 1.)).unwrap();
    assert!(second.report.conversions.is_empty());
    assert_eq!(second.report.stages[0].fixed, 0);
    assert_eq!(second.model, first.model);
    assert_eq!(model_hash(&second.model), model_hash(&first.model));
}

#[test]
fn excessive_control_deviation_is_refused_atomically() {
    // Small interior weights keep evaluated samples near z = 0 while the
    // complete control net is deliberately much farther from the plane.
    let model = near_plane(1e-2, 1e-3);
    let before = model.clone();
    let error = cast_canonical_surfaces(&model, options(1e-3, 1.)).unwrap_err();
    assert_eq!(error.code, CANONICAL_CAST_DEVIATION);
    assert_eq!(model, before);
}

#[test]
fn measured_area_delta_cap_rolls_back_atomically() {
    let model = near_plane(1e-2, 1.);
    let before = model.clone();
    let error = cast_canonical_surfaces(&model, options(1e-1, 0.)).unwrap_err();
    assert_eq!(error.code, CANONICAL_CAST_AREA_DELTA);
    assert_eq!(model, before);
}

#[test]
fn area_growth_guard_is_typed_and_nan_guards_are_active() {
    // Orthogonal reprojection of a graph-like near-plane cannot grow its
    // trimmed area. Exercise the shared growth branch directly; the real
    // measured before/after integration gate is covered above.
    assert_eq!(
        check_area_delta(4., 4.1, 0.01).unwrap_err().code,
        CANONICAL_CAST_AREA_GREW
    );
    assert_eq!(
        check_area_delta(4., f64::NAN, 1.).unwrap_err().code,
        CANONICAL_CAST_AREA_DELTA
    );
    let model = near_plane(1e-4, 1.);
    let error = cast_canonical_surfaces(&model, options(f64::NAN, 1.)).unwrap_err();
    assert_eq!(error.code, CANONICAL_CAST_BUDGET);
}

#[test]
fn analytic_cylinder_and_cone_faces_are_certified_and_annotated() {
    for (model, kind) in [
        (crate::cylinder(1.5, 3.).unwrap(), "Cylinder"),
        (crate::frustum(2., 1., 3.).unwrap(), "Cone"),
    ] {
        let run = cast_canonical_surfaces(&model, options(1e-6, 1e-6)).unwrap();
        assert_eq!(run.report.input_hash, run.report.output_hash, "geometry unchanged");
        let stage = run
            .report
            .stages
            .iter()
            .find(|s| s.stage == "analyticRecognition")
            .expect("annotation stage");
        assert_eq!(stage.skipped, 4, "{kind}: four side patches");
        for skip in &stage.skipped_details {
            assert_eq!(skip.code, CANONICAL_CAST_ANALYTIC_ANNOTATED);
            assert!(skip.message.contains(kind), "{}", skip.message);
            assert!(skip.measured_mm.unwrap() <= 1e-6);
        }
    }
}

#[test]
fn near_cylinder_off_tolerance_is_not_annotated() {
    let mut model = crate::cylinder(1.5, 3.).unwrap();
    let side = (0..model.faces.len())
        .find(|&f| model.faces[f].surface.degree_u == 2 || model.faces[f].surface.degree_v == 2)
        .unwrap();
    let class = classify_surface(&model.faces[side].surface, 1e-3, &Budget::default()).unwrap();
    assert_eq!(class.class, SurfaceClass::Cylinder);
    // Shift the carrier radially by 1e-4: still a sampled near-cylinder at
    // 1e-3, but the coefficient certificate at 1e-6 must refuse it.
    for row in &mut model.faces[side].surface.control_points {
        for p in row {
            p[0] *= 1. + 1e-4;
        }
    }
    let class = classify_surface(&model.faces[side].surface, 1e-3, &Budget::default()).unwrap();
    assert!(certify_rotational(&model.faces[side].surface, &class, 1e-6).unwrap().is_none());
}

#[test]
fn cylinder_and_cone_targets_are_explicitly_unsupported() {
    for class in [SurfaceClass::Cylinder, SurfaceClass::Cone] {
        let error = unsupported_rotational(class).unwrap_err();
        assert_eq!(error.code, CANONICAL_CAST_UNSUPPORTED);
    }
    assert!(unsupported_rotational(SurfaceClass::Plane).is_ok());
}
