
    use super::*;
    #[test]
#[cfg(feature = "codec")]
    fn scaled_retained_loft_preserves_holes_and_rational_sections() {
        let mut hole=crate::sketch::circle_wire(1.).unwrap();
        hole=hole.iter().rev().map(Curve::reverse).collect::<nurbs_core::Result<Vec<_>>>().unwrap();
        let loops=vec![crate::sketch::circle_wire(3.).unwrap(),hole];
        let before=value_codec::to_value(&loops).unwrap();
        let loft=loft_scaled(&loops,0.,10.,2.,[5.,7.]).unwrap();
        assert_eq!(loft.bodies.len(),1);
        assert_eq!(loft.faces.iter().filter(|face|face.holes.len()==1).count(),2);
        assert_eq!(value_codec::to_value(&loops).unwrap(),before);
        let side=&loft.faces[0].surface;
        let point=side.evaluate(0.37,0.5).unwrap().point;
        let source=loops[0][0].evaluate(0.37).unwrap().point;
        assert!((point[0]-(1.5*source[0]+2.5)).abs()<1e-12);
        assert!((point[1]-(1.5*source[1]+3.5)).abs()<1e-12);
        assert!((point[2]-5.).abs()<1e-12);
        assert!(side.weights.iter().flatten().any(|w| *w!=1.));
        let (step,_,_)=crate::step_interchange_v3::export_step_v9(&loft).unwrap();
        let (restored,_,_)=crate::step_interchange_v3::import_step_v9(&step).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.bodies.len(),1);
        assert_eq!(restored.faces.iter().filter(|face|face.holes.len()==1).count(),2);
        for scale in [0.,-1.,f64::NAN,f64::INFINITY] {
            assert!(loft_scaled(&loops,0.,10.,scale,[0.,0.]).is_err());
        }
        assert!(loft_scaled(&loops,0.,10.,2.,[1e6,0.]).is_err());
    }

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
