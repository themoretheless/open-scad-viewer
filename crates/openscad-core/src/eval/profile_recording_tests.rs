
    use super::*;
    #[test]
    fn profile_booleans_keep_operand_order_without_solid_nodes() {
        let evaluator = Evaluator::new(
            &[],
            &[],
            LanguageProfile::Stable2021,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        );
        let first = evaluator
            .emit_profile(
                "square",
                geometry_ops::profile_program::Node::Rectangle {
                    size: [4.; 2],
                    center: true,
                },
                0,
            )
            .unwrap();
        let second = evaluator
            .emit_profile(
                "square",
                geometry_ops::profile_program::Node::Rectangle {
                    size: [2.; 2],
                    center: true,
                },
                0,
            )
            .unwrap();
        let shapes = evaluator
            .boolean_shapes([first, second].concat(), "difference", 0, "difference")
            .unwrap();
        assert_eq!(shapes[0].dimension, 2);
        assert_eq!(shapes[0].profile, Some(2));
        assert!(shapes[0].geometry.is_none());
        assert!(evaluator.geometry.borrow().is_empty());
        assert_eq!(
            evaluator.profiles.borrow()[2],
            geometry_ops::profile_program::Node::Boolean {
                operation: geometry_ops::solid_program::Boolean::Difference,
                inputs: vec![0, 1]
            }
        );
    }
