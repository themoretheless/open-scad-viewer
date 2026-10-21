
    use super::*;
    use crate::analysis::mass_properties;

    fn volume(model: &Model) -> f64 {
        // Hundreds of faces: the coarsest admitted relative tolerance keeps
        // the divergence integration inside its evaluation budget.
        mass_properties(model, 1e-3, 2_000_000)
            .unwrap()
            .signed_volume_mm3
    }

    #[test]
    fn involute_flanks_follow_analytic_geometry_and_never_relax_remainder() {
        for base in [2., 20., 100.] {
            let half = |r: f64| {
                let t = ((r / base).powi(2) - 1.).max(0.).sqrt();
                0.18 - (t - t.atan())
            };
            for sign in [-1., 1.] {
                for (start, end) in [(0., 0.7), (0.7, 0.), (0.2, 0.7)] {
                    let (curve, bound) = involute(base, start, end, 0.4, sign, &half).unwrap();
                    assert!(bound > 0. && bound <= FLANK_FIT);
                    for i in 0..=200 {
                        let s = i as f64 / 200.;
                        let t = start + (end - start) * s;
                        // Independent roll-radius/polar-angle definition.
                        let radius = base * t.hypot(1.);
                        let angle = 0.4 + sign * (0.18 - t + t.atan());
                        let point = curve.evaluate(s).unwrap().point;
                        let distance = (point[0] - radius * angle.cos())
                            .hypot(point[1] - radius * angle.sin());
                        assert!(distance <= bound + 1e-11, "{distance} > {bound}");
                    }
                }
            }
        }
        let half = |_: f64| 0.;
        assert!(involute(2., 0., 100., 0., 1., &half).is_err());
    }

    #[test]
    fn spur_gear_is_a_closed_exact_solid_with_the_expected_volume() {
        let spec = GearSpec {
            module: 2.,
            teeth: 20,
            height: 6.,
            ..GearSpec::default()
        };
        let (model, geometry) = gear_with_report(&spec).unwrap();
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        assert_eq!(geometry.face_count, 20 * 6 + 2);
        assert!(geometry.flank_deviation <= FLANK_FIT);
        assert!(geometry.flank_rounding_certified);
        assert!(geometry.sweep_deviation <= SWEEP_FIT);
        // The volume lies between the root and tip cylinders and close to
        // the pitch cylinder (teeth fill about half the band).
        let v = volume(&model);
        let root = PI * geometry.root_radius.powi(2) * 6.;
        let tip = PI * geometry.tip_radius.powi(2) * 6.;
        assert!(v > root && v < tip, "{v} not in ({root}, {tip})");
        let pitch = PI * geometry.pitch_radius.powi(2) * 6.;
        assert!((v - pitch).abs() < 0.08 * pitch, "{v} vs pitch {pitch}");
    }

    #[test]
    fn helical_and_herringbone_gears_match_the_sweep_within_tolerance() {
        for herringbone in [false, true] {
            let spec = GearSpec {
                module: 1.5,
                teeth: 16,
                height: 8.,
                helix_angle_deg: 25.,
                herringbone,
                bore: 6.,
                ..GearSpec::default()
            };
            let (model, geometry) = gear_with_report(&spec).unwrap();
            assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
            assert!(geometry.sweep_deviation > 0. && geometry.sweep_deviation <= SWEEP_FIT);
            assert!(geometry.sweep_rounding_certified);
            let expected_faces = (16 * 6 + 8) * if herringbone { 2 } else { 1 } + 2;
            assert_eq!(geometry.face_count, expected_faces);
            let spur = gear(&GearSpec {
                helix_angle_deg: 0.,
                herringbone: false,
                ..spec.clone()
            })
            .unwrap();
            // Twisting does not change the volume.
            let (vh, vs) = (volume(&model), volume(&spur));
            assert!((vh - vs).abs() < 2e-3 * vs, "helical {vh} vs spur {vs}");
        }
    }

    #[test]
    fn internal_gear_has_a_rim_and_a_toothed_opening() {
        let spec = GearSpec {
            module: 1.5,
            teeth: 40,
            height: 10.,
            internal: true,
            rim_width: 2.,
            helix_angle_deg: 15.,
            herringbone: true,
            ..GearSpec::default()
        };
        let (model, geometry) = gear_with_report(&spec).unwrap();
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        assert!(geometry.outside_radius > geometry.root_radius);
        let v = volume(&model);
        let outer = PI * geometry.outside_radius.powi(2) * 10.;
        let pitch = PI * geometry.pitch_radius.powi(2) * 10.;
        assert!(v > 0. && v < outer - pitch * 0.9, "{v}");
    }

    #[test]
    fn few_teeth_planet_and_refusals() {
        // A four-tooth planet as in the spinner: undercut is not modelled,
        // the outline is still a valid solid.
        let planet = gear(&GearSpec {
            module: 1.5,
            teeth: 4,
            height: 10.,
            pressure_angle_deg: 20.,
            ..GearSpec::default()
        });
        assert!(planet.is_ok(), "{:?}", planet.err());
        assert!(
            gear(&GearSpec {
                teeth: 2,
                ..GearSpec::default()
            })
            .is_err()
        );
        assert!(
            gear(&GearSpec {
                bore: 100.,
                ..GearSpec::default()
            })
            .is_err()
        );
        assert!(
            gear(&GearSpec {
                internal: true,
                rim_width: 0.,
                ..GearSpec::default()
            })
            .is_err()
        );
    }
