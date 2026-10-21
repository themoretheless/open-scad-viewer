    use super::*;

    #[test]
    fn rack_and_pulley_profiles_are_valid() {
        let rack = involute_rack_profile(&RackOptions::default()).unwrap();
        assert_eq!(rack.len(), 10 * 4 + 2);
        let pulley = timing_pulley_profile(&TimingPulleyOptions::default()).unwrap();
        assert_eq!(pulley.len(), 2);
        assert_eq!(pulley[0].len(), 160);
    }

    #[test]
    fn iso_fasteners_and_nut_profile_match_standards() {
        let m6 = iso_metric_fastener(6.0).unwrap();
        assert_eq!(m6.coarse_pitch_mm, 1.0);
        assert_eq!(m6.hex_width_across_flats_mm, 10.0);
        let nut = hex_nut_profile(6.0).unwrap();
        assert_eq!(nut.len(), 2);
        assert_eq!(nut[0].len(), 6);
        assert!(iso_metric_fastener(7.0).is_err());
    }

    #[test]
    fn helical_bevel_worm_and_sprocket_calculations() {
        let h = helical_gear_params(&HelicalGearOptions {
            teeth: 20,
            normal_module_mm: 2.0,
            normal_pressure_angle_deg: 20.0,
            helix_angle_deg: 15.0,
            face_width_mm: 12.0,
        })
        .unwrap();
        assert!(h.transverse_module_mm > 2.0);
        assert!(h.virtual_spur_teeth > 20.0);

        let b = bevel_gear_params(&BevelGearOptions {
            pinion_teeth: 20,
            gear_teeth: 40,
            module_mm: 2.0,
            shaft_angle_deg: 90.0,
            pressure_angle_deg: 20.0,
            face_width_mm: 10.0,
        })
        .unwrap();
        assert!((b.pinion_pitch_angle_deg + b.gear_pitch_angle_deg - 90.0).abs() < 1e-9);
        assert!((b.pinion_pitch_diameter_mm - 40.0).abs() < 1e-9);

        let w = worm_drive_params(&WormDriveOptions {
            worm_starts: 1,
            wheel_teeth: 40,
            axial_module_mm: 2.0,
            worm_pitch_diameter_mm: 20.0,
            friction_coefficient: 0.15,
        })
        .unwrap();
        assert_eq!(w.gear_ratio, 40.0);
        assert!(w.self_locking);

        let sp = roller_chain_sprocket(&SprocketOptions::default()).unwrap();
        assert_eq!(sp.len(), 2);
        assert_eq!(sp[0].len(), 180);

        let htd5m = timing_pulley_standard(PulleyStandard::Htd5M, 24, 8.0);
        let htd_loops = timing_pulley_profile(&htd5m).unwrap();
        assert_eq!(htd_loops.len(), 2);
    }

    #[test]
    fn inserts_keyways_grooves_fits_and_springs() {
        let m3_insert = heat_set_insert_spec(3.0).unwrap();
        assert!((m3_insert.recommended_hole_diameter_mm - 4.0).abs() < 1e-6);

        let kw = shaft_keyway_din6885(20.0).unwrap();
        assert_eq!(kw.key_width_b_mm, 6.0);

        let circlip = retaining_ring_groove(20.0, false).unwrap();
        assert!(circlip.groove_diameter_d2_mm < 20.0);

        let oring = o_ring_groove_spec(2.5, false).unwrap();
        assert!(oring.groove_depth_h_mm < 2.5);
        assert!(oring.groove_width_b_mm > 2.5);

        let fit_g6 = iso286_shaft_housing_fit(20.0, BearingFitClass::ClearanceH7g6).unwrap();
        assert!(fit_g6.min_clearance_um > 0.0);
        let fit_p6 = iso286_shaft_housing_fit(20.0, BearingFitClass::InterferenceH7p6).unwrap();
        assert!(fit_p6.max_clearance_um < 0.0);

        let disc = disc_spring_load(&DiscSpringOptions {
            outer_diameter_mm: 20.0,
            inner_diameter_mm: 10.2,
            thickness_mm: 1.0,
            free_cone_height_mm: 0.55,
            deflection_mm: 0.3,
            young_mpa: 206_000.0,
            poisson: 0.3,
        })
        .unwrap();
        assert!(disc.axial_load_n > 100.0);
        assert!(disc.flat_load_n > disc.axial_load_n);

        let spring = helical_spring_analysis(&HelicalSpringOptions {
            wire_diameter_mm: 2.0,
            mean_coil_diameter_mm: 16.0,
            active_coils: 8.0,
            shear_modulus_mpa: 79_300.0,
            axial_load_n: 50.0,
        })
        .unwrap();
        assert_eq!(spring.spring_index, 8.0);
        assert!(spring.wahl_factor > 1.15);
        assert!(spring.deflection_mm > 0.0);

        let gland = oring_gland_spec(2.5, 20.0, true).unwrap();
        assert_eq!(gland.gland_inner_diameter_mm, 20.0);
        assert!(gland.gland_outer_diameter_mm > 23.0);
        assert!(gland.gland_fill_ratio > 0.65 && gland.gland_fill_ratio < 0.90);

        let pset = planetary_gearset_params(18, 18, 3, 1.5).unwrap();
        assert_eq!(pset.ring_teeth, 54);
        assert!((pset.sun_to_carrier_ratio - 4.0).abs() < 1e-9);
        assert!((pset.center_distance_mm - 27.0).abs() < 1e-9);
        assert!(planetary_gearset_params(17, 18, 3, 1.5).is_err()); // 17 + 53 = 70 not divisible by 3
    }
