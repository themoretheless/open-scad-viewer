    use super::*;
    #[test]
    #[cfg(feature = "codec")]
    fn recognizes_trimmed_step_carriers_without_admitting_deformed_sides() {
        let model: Model = value_codec::from_str(include_str!(
            "../../../../docs/qualification/cad-roadmap-2026-09-28/parts-history/imported-flange.json"
        ))
        .unwrap();
        let recognized = recognize(&model).unwrap().unwrap();
        assert_eq!(
            (recognized.z_min, recognized.z_max, recognized.loops.len()),
            (0., 7., 2)
        );
        let usage = model.shells[0]
            .faces
            .iter()
            .find(|u| model.faces[u.face].surface.control_points.len() == 9)
            .unwrap()
            .clone();
        assert!(side_is_proven(&model, &usage, 0., 7.).unwrap());
        let mut deformed = model.clone();
        deformed.faces[usage.face].surface.control_points[1][1][0] += 0.01;
        assert!(!side_is_proven(&deformed, &usage, 0., 7.).unwrap());
        let mut trimmed = model.clone();
        let wire = trimmed.faces[usage.face].outer;
        trimmed.loops[wire].coedges[0].pcurve.control_points[0][0] += 0.01;
        assert!(!side_is_proven(&trimmed, &usage, 0., 7.).unwrap());
        let top = model
            .faces
            .iter()
            .position(|f| planar_cap_z(&f.surface) == Some(7.))
            .unwrap();
        let result = crate::operations::push_planar_face(&model, top, 1.).unwrap();
        let volume = crate::analysis::mass_properties(&result, 1e-7, 200_000)
            .unwrap()
            .signed_volume_mm3;
        assert!((volume - 3000. * std::f64::consts::PI).abs() < 1e-5);
    }
