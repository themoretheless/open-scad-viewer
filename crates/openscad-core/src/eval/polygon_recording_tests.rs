
    use super::*;
    fn vector(values: Vec<Value<'static>>) -> Value<'static> {
        Value::vector(values)
    }
    #[test]
    fn records_even_odd_outlines_and_preserves_conversion_warnings() {
        let evaluator = Evaluator::new(
            &[],
            &[],
            LanguageProfile::Stable2021,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        );
        let points = vector(vec![
            vector(vec![Value::Number(0.), Value::Number(0.)]),
            vector(vec![Value::Number(2.), Value::Number(0.)]),
            vector(vec![Value::Number(0.), Value::Number(2.)]),
        ]);
        let values = HashMap::from([
            ("points".to_string(), points),
            (
                "paths".to_string(),
                vector(vec![vector(vec![
                    Value::Number(0.),
                    Value::Number(1.),
                    Value::Number(99.),
                    Value::Number(2.),
                ])]),
            ),
        ]);
        let shapes = evaluator.record_stable_polygon(&values, 0).unwrap();
        assert_eq!(shapes[0].dimension, 2);
        assert_eq!(
            evaluator.profiles.borrow()[0],
            geometry_ops::profile_program::Node::EvenOddRings(vec![vec![
                [0., 0.],
                [2., 0.],
                [0., 2.]
            ]])
        );
        assert_eq!(
            evaluator.warnings.borrow()[0],
            "polygon skipped a path entry whose point index is outside the point vector"
        );
        let invalid = HashMap::from([(
            "points".to_string(),
            vector(vec![vector(vec![Value::Number(0.)])]),
        )]);
        evaluator.record_stable_polygon(&invalid, 0).unwrap();
        assert_eq!(
            evaluator.profiles.borrow()[1],
            geometry_ops::profile_program::Node::Empty
        );
        assert_eq!(
            evaluator.warnings.borrow()[1],
            "polygon produced no outlines because a point failed exact vec2 conversion"
        );
    }
