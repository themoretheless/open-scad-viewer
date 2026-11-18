
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
