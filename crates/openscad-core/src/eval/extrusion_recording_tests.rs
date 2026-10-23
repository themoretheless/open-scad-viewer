
    use super::*;
    #[test]
    fn records_profile_and_deferred_policy_with_shared_parameters() {
        let evaluator = Evaluator::new(
            &[],
            &[],
            LanguageProfile::Stable2021,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        );
        evaluator
            .emit_profile(
                "unused",
                geometry_ops::profile_program::Node::Circle {
                    radius: 9.,
                    segments: 12,
                },
                0,
            )
            .unwrap();
        let sections = evaluator
            .emit_profile(
                "square",
                geometry_ops::profile_program::Node::Rectangle {
                    size: [2.; 2],
                    center: true,
                },
                0,
            )
            .unwrap();
        let values = HashMap::from([
            ("height".to_string(), Value::Number(3.)),
            ("twist".to_string(), Value::Number(360.)),
            ("slices".to_string(), Value::Number(3.9)),
        ]);
        let shapes = evaluator
            .record_stable_extrusion(sections, &values, [12., 12., 2.], 0)
            .unwrap();
        assert_eq!(shapes[0].geometry, Some(0));
        assert_eq!(shapes[0].dimension, 3);
        let nodes = evaluator.geometry.borrow();
        let geometry_ops::solid_program::Node::ExtrudeProfile {
            profile,
            slice_policy,
            height,
            twist,
            ..
        } = &nodes[0]
        else {
            panic!("Expected profile extrusion")
        };
        assert_eq!(*height, 3.);
        assert_eq!(*twist, 360.);
        assert_eq!(profile.roots, vec![0]);
        assert_eq!(profile.nodes.len(), 1);
        assert_eq!(slice_policy.as_ref().unwrap().explicit, Some(3.));
        assert_eq!(slice_policy.as_ref().unwrap().fragments, [12., 12., 2.]);
    }
