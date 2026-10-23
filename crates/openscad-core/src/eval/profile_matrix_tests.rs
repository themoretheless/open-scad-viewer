
    use super::*;
    #[test]
    fn xy_matrix_ignores_z_collapse_and_preserves_profile_dimension() {
        let evaluator = Evaluator::new(
            &[],
            &[],
            LanguageProfile::Stable2021,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        );
        let shapes = evaluator
            .emit_profile(
                "square",
                geometry_ops::profile_program::Node::Rectangle {
                    size: [2.; 2],
                    center: true,
                },
                0,
            )
            .unwrap();
        let matrix = [
            2., 0., 0., 0., 0., 3., 0., 0., 0., 0., 0., 0., 4., 5., 6., 1.,
        ];
        let shapes = evaluator
            .record_stable_matrix(shapes, matrix, "scale", 0)
            .unwrap();
        assert_eq!(shapes[0].dimension, 2);
        assert_eq!(
            evaluator.profiles.borrow()[1],
            geometry_ops::profile_program::Node::Transform {
                input: 0,
                matrix: [[2., 0., 4.], [0., 3., 5.], [0., 0., 1.]]
            }
        );
        let mut collapsed = matrix;
        collapsed[0] = 0.;
        let empty = evaluator
            .record_stable_matrix(shapes, collapsed, "scale", 0)
            .unwrap();
        assert_eq!(empty[0].dimension, 2);
        assert_eq!(
            evaluator.profiles.borrow()[2],
            geometry_ops::profile_program::Node::Empty
        );
        assert!(evaluator.geometry.borrow().is_empty());
    }
