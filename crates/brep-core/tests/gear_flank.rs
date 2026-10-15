use brep_core::gear::involute_flank;
#[test]
fn actual_placed_and_reversed_flanks_have_inclusive_geometric_bounds() {
    for base in [2., 20., 100.] {
        for angle in [-6.2_f64, 0.4, 6.2] {
            for reflection in [-1., 1.] {
                for (start, end) in [(0.1, 0.7), (0.7, 0.1)] {
                    let (curve, certificate) =
                        involute_flank(base, start, end, angle, reflection).unwrap();
                    assert!(certificate.rounding_included);
                    assert!(certificate.error_upper <= 1e-6);
                    for i in 0..=100 {
                        let u = i as f64 / 100.;
                        let t = start + (end - start) * u;
                        let radius = base * t.hypot(1.);
                        let phase = angle + reflection * (t - t.atan());
                        let p = curve.evaluate(u).unwrap().point;
                        let error =
                            (p[0] - radius * phase.cos()).hypot(p[1] - radius * phase.sin());
                        assert!(
                            error <= certificate.error_upper + 2e-12,
                            "{error} > {certificate:?}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn unsupported_flank_inputs_are_refused() {
    for (r, a, b, angle, reflection) in [
        (0., 0., 1., 0., 1.),
        (1., 1., 1., 0., 1.),
        (1., 0., 1., 9., 1.),
        (1., 0., 1., 0., 0.),
        (1., 0., 100., 0., 1.),
    ] {
        assert!(involute_flank(r, a, b, angle, reflection).is_err());
    }
}

#[test]
fn gear_variants_keep_closed_incidence_after_flank_qualification() {
    use brep_core::gear::{GearSpec, gear_with_report};
    for (helix, herringbone, internal) in [
        (0., false, false),
        (20., false, false),
        (20., true, false),
        (0., false, true),
    ] {
        let spec = GearSpec {
            teeth: if internal { 40 } else { 16 },
            helix_angle_deg: helix,
            herringbone,
            internal,
            ..GearSpec::default()
        };
        let (model, geometry) = gear_with_report(&spec).unwrap();
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        assert!(model.shells[0].closed);
        assert_eq!(model.bodies.len(), 1);
        assert!(geometry.flank_deviation <= 1e-6);
        assert!(geometry.flank_stored_parameters_rounding_certified);
        // Machin/Taylor parameter boxes now include the full GearSpec
        // derivation. Root topology and stored helical-wall rounding are separate.
        assert!(geometry.flank_rounding_certified);
        for edge in 0..model.edges.len() {
            assert_eq!(
                model
                    .loops
                    .iter()
                    .flat_map(|l| &l.coedges)
                    .filter(|c| c.edge == edge)
                    .count(),
                2
            );
        }
    }
}

#[test]
fn every_nonfinite_spec_field_is_rejected_before_geometry_authoring() {
    use brep_core::gear::{GearSpec, gear_with_report};
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for index in 0..8 {
            let mut spec = GearSpec::default();
            match index {
                0 => spec.module = value,
                1 => spec.pressure_angle_deg = value,
                2 => spec.height = value,
                3 => spec.helix_angle_deg = value,
                4 => spec.bore = value,
                5 => spec.rim_width = value,
                6 => spec.clearance = value,
                _ => spec.backlash = value,
            }
            assert!(gear_with_report(&spec).is_err());
        }
    }
}
