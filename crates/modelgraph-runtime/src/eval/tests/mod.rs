use super::*;
fn document() -> Json {
    json!({"parameters":[],"functions":[],"root":"Root"})
}
fn evaluate(expression: &Json) -> Result<Value<'_>> {
    // Results borrow the expression; the leaked empty document is avoided by
    // placing it in static storage for these expression-only cases.
    static DOCUMENT: std::sync::OnceLock<Json> = std::sync::OnceLock::new();
    Evaluator::new(DOCUMENT.get_or_init(document))?.resolve(
        expression,
        &Scope::default(),
        "/test",
        0,
    )
}
fn scalars(value: &Value<'_>) -> Vec<f64> {
    sequence(value, "/")
        .unwrap()
        .iter()
        .map(|item| numeric(item, "/").unwrap())
        .collect()
}

#[test]
fn arithmetic_units_and_lazy_branches_match_contract() {
    let q = json!({"op":"add","args":[{"op":"quantity","value":2,"unit":"cm"},{"op":"quantity","value":3,"unit":"mm"}]});
    assert_eq!(
        numeric_value(&evaluate(&q).unwrap(), "/").unwrap(),
        Numeric {
            value: 23.0,
            dimension: units::LENGTH
        }
    );
    let mismatch = json!({"op":"add","args":[1,{"op":"quantity","value":3,"unit":"mm"}]});
    let error = evaluate(&mismatch).unwrap_err();
    assert_eq!(
        (error.code.as_str(), error.path.as_str()),
        ("unit_mismatch", "/test")
    );
    for expr in [
        json!({"op":"if","condition":1,"then":7,"else":{"local":"missing"}}),
        json!({"op":"or","args":[1,{"local":"missing"}]}),
        json!({"op":"and","args":[0,{"local":"missing"}]}),
    ] {
        assert!(evaluate(&expr).is_ok());
    }
    let squared = json!({"op":"sqrt","value":{"op":"multiply","args":[{"op":"quantity","value":3,"unit":"mm"},{"op":"quantity","value":3,"unit":"mm"}]}});
    assert_eq!(
        numeric_value(&evaluate(&squared).unwrap(), "/").unwrap(),
        Numeric {
            value: 3.0,
            dimension: units::LENGTH
        }
    );
    let invalid_dimension =
        json!({"op":"sqrt","value":{"op":"quantity","value":4,"unit":"mm"}});
    assert_eq!(
        evaluate(&invalid_dimension).unwrap_err().code,
        "dimension_limit"
    );
}

#[test]
fn closures_capture_lexical_scope_and_shadow_safely() {
    let expr = json!({"op":"let","name":"x","value":10,"body":{"op":"let","name":"f","value":{"op":"lambda","parameters":["n"],"body":{"op":"add","args":[{"local":"x"},{"local":"n"}]}},"body":{"op":"let","name":"x","value":20,"body":{"op":"apply","function":{"local":"f"},"args":[3]}}}});
    assert_eq!(numeric(&evaluate(&expr).unwrap(), "/").unwrap(), 13.0);
    let captured =
        json!({"op":"let","name":"x","value":{"op":"list","items":[1,2]},"body":{"local":"x"}});
    assert_eq!(scalars(&evaluate(&captured).unwrap()), vec![1.0, 2.0]);
}

#[test]
fn sequences_support_flatten_filter_reduce_and_exact_endpoints() {
    let interval = json!({"op":"interval","start":0,"end":0.3,"step":0.1,"inclusive":true});
    let list = evaluate(&interval).unwrap();
    let samples = scalars(&list);
    assert_eq!(samples.len(), 4);
    assert!((samples[3] - 0.3).abs() < 1e-15);
    let counted = json!({"op":"interval","start":0,"end":1,"count":4,"inclusive":true});
    assert_eq!(scalars(&evaluate(&counted).unwrap())[3], 1.0);
    let expr = json!({"op":"reduce","initial":0,"input":{"op":"filter","input":{"op":"flatmap","input":{"op":"range","count":3,"start":1,"step":1},"function":{"op":"lambda","parameters":["x"],"body":{"op":"list","items":[{"local":"x"},{"local":"x"}]}}},"function":{"op":"lambda","parameters":["x"],"body":{"op":"lt","args":[1,{"local":"x"}]}}},"function":{"op":"lambda","parameters":["sum","x"],"body":{"op":"add","args":[{"local":"sum"},{"local":"x"}]}}});
    assert_eq!(numeric(&evaluate(&expr).unwrap(), "/").unwrap(), 10.0);
    let zip =
        json!({"op":"zip","inputs":[{"op":"list","items":[1,2]},{"op":"list","items":[3]}]});
    assert_eq!(evaluate(&zip).unwrap_err().code, "length_mismatch");
}

#[test]
fn budget_checks_count_logical_allocations_despite_shared_storage() {
    let inner = json!({"op":"range","count":256,"start":0,"step":1});
    let expr = json!({"op":"map","input":inner,"function":{"op":"lambda","parameters":["x"],"body":{"op":"range","count":256,"start":0,"step":1}}});
    let error = evaluate(&expr).unwrap_err();
    assert_eq!(error.code, "allocation_limit");
    assert_eq!(error.path, "/test[62]/apply");
    let mut deep = json!(1);
    for _ in 0..33 {
        deep = json!({"op":"negate","value":deep});
    }
    assert_eq!(evaluate(&deep).unwrap_err().code, "evaluation_limit");
    let doc = document();
    let literal = json!(1);
    let mut evaluator = Evaluator::new(&doc).unwrap();
    for _ in 0..100_000 {
        evaluator
            .resolve(&literal, &Scope::default(), "/test", 0)
            .unwrap();
    }
    assert_eq!(
        evaluator
            .resolve(&literal, &Scope::default(), "/test", 0)
            .unwrap_err()
            .code,
        "evaluation_limit"
    );
}

#[test]
fn typed_values_and_strict_fields_are_not_coerced() {
    let rounded = json!({"op":"typed","type":"f32","value":0.1});
    assert_eq!(
        numeric(&evaluate(&rounded).unwrap(), "/").unwrap(),
        0.1f32 as f64
    );
    let fractional = json!({"op":"typed","type":"int","value":1.1});
    assert_eq!(
        evaluate(&fractional).unwrap_err().message,
        "Expected int (signed 32-bit integer)"
    );
    assert_eq!(
        units::field(0.0.into(), units::LENGTH, "/", true).unwrap(),
        0.0
    );
    assert_eq!(
        units::field(1.0.into(), units::LENGTH, "/", true)
            .unwrap_err()
            .code,
        "unit_mismatch"
    );
    assert_eq!(
        units::unary("sin", 90.0.into(), "/", true)
            .unwrap_err()
            .code,
        "unit_mismatch"
    );
}

#[test]
fn function_arguments_bind_without_caller_locals_and_geometry_is_deferred() {
    let doc = json!({"parameters":[{"id":"Width","value":4}],"functions":[{"id":"Add","kind":"scalar","parameters":["x"],"body":{"op":"add","args":[{"local":"x"},{"param":"Width"}]}},{"id":"Shape","kind":"geometry","parameters":["r"],"nodes":[],"root":"Sphere"}]});
    let call = json!({"op":"call","function":"Add","args":{"x":2}});
    let shape = json!({"op":"geometry","function":"Shape","args":{"r":2}});
    let wrong = json!({"op":"call","function":"Add","args":{"z":2}});
    let mut evaluator = Evaluator::new(&doc).unwrap();
    assert_eq!(
        evaluator
            .evaluate(&call, &Scope::default(), "/", 0)
            .unwrap(),
        6.0
    );
    ::std::assert_matches!(
        evaluator
            .resolve(&shape, &Scope::default(), "/", 0)
            .unwrap(),
        Value::Geometry {
            function: "Shape",
            ..
        }
    );
    assert_eq!(
        evaluator
            .resolve(&wrong, &Scope::default(), "/", 0)
            .unwrap_err()
            .code,
        "invalid_arguments"
    );
}

#[test]
fn reports_retain_paths_dimensions_and_all_failed_constraints() {
    let doc = json!({"parameters":[],"constraints":[{"id":"A","left":1,"relation":"eq","right":2,"message":"A failed"},{"id":"B","left":3,"relation":"lt","right":2,"message":"B failed"}]});
    let error = Evaluator::new(&doc)
        .unwrap()
        .validate_checks()
        .err()
        .unwrap();
    assert_eq!(error.code, "constraint_failed");
    assert!(
        error
            .message
            .contains("A failed (actual: 1.0; required: == 2.0")
    );
    assert!(
        error
            .message
            .contains("B failed (actual: 3.0; required: < 2.0")
    );
    let report = error.details.unwrap();
    assert_eq!(report.as_array().unwrap().len(), 2);
    assert_eq!(report[1]["path"], "/constraints/1");
    assert_eq!(report[0]["dimension"], json!([0, 0]));
    let doc = json!({"parameters":[],"root":"Root","geometry_assertions":[{"id":"Size","target":"Root","check":"width","expected":{"op":"quantity","value":2,"unit":"cm"},"message":"Width"}]});
    let checks = Evaluator::new(&doc).unwrap().validate_checks().unwrap();
    assert_eq!(checks.geometry_assertions[0]["expected"], 20.0);
    assert_eq!(checks.geometry_assertions[0]["tolerance"], 0.0);
}
