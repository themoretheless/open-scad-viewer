use super::*;
use value_codec::json;

fn document(node: Value) -> Value {
    json!({"language":"rush/ir-1","units":"mm","parameters":[],"nodes":[node],"root":"shape"})
}
#[test]
fn matches_original_zod_schema_corpus() {
    let corpus: Value =
        value_codec::from_str(include_str!("../../tests/fixtures/schema-parity.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let result = validate(case["input"].clone());
        if case["error"] == true {
            assert!(result.is_err(), "{} unexpectedly accepted", case["name"]);
        } else {
            assert_eq!(
                result.unwrap_or_else(|error| panic!("{}: {:?}", case["name"], error)),
                case["expected"],
                "{} defaults differ",
                case["name"]
            );
        }
    }
}
#[test]
fn reports_exact_nested_paths_and_budget_error_codes() {
    let error = validate(document(
        json!({"id":"shape","op":"box","size":[1,{"param":"bad-id"},3]}),
    ))
    .unwrap_err();
    assert_eq!(error.code, "invalid_document");
    assert_eq!(error.path, "/nodes/0/size/1/param");
    let error = validate(Value::Array(vec![Value::Null; 20_000])).unwrap_err();
    assert_eq!(error.code, "input_limit");
    assert_eq!(error.path, "/");
}
#[test]
fn inserts_defaults_without_changing_expressions() {
    let input = document(
        json!({"id":"shape","op":"gear","teeth":{"param":"teeth"},"module":1,"thickness":5}),
    );
    let output = validate(input).unwrap();
    assert_eq!(output["segments"], json!(48));
    assert_eq!(output["nodes"][0]["teeth"], json!({"param":"teeth"}));
    assert_eq!(output["nodes"][0]["pressure_angle"], json!(20));
    assert_eq!(output["nodes"][0]["internal"], json!(false));
    assert_eq!(output["nodes"][0]["backlash"], json!(0.15));
}
#[test]
fn rejects_unknown_keys_in_expressions_and_components() {
    let mut input = document(
        json!({"id":"shape","op":"sphere","radius":{"op":"add","args":[1,2],"extra":3}}),
    );
    assert!(validate(input.clone()).is_err());
    input["nodes"][0]["radius"] = json!({"param":"r","local":"r"});
    assert!(validate(input).is_err());
    let input = document(
        json!({"id":"shape","op":"assembly","components":[{"id":"part","input":"other","anchors":[],"placement":{"origin":[0,0,0],"rotation":[0,0,0],"extra":0}}]}),
    );
    assert!(validate(input).is_err());
}
#[test]
fn validates_nested_function_and_sequence_variants() {
    let mut input = document(
        json!({"id":"shape","op":"evaluate","value":{"op":"reduce","input":{"op":"zip","inputs":[{"op":"interval","start":0,"end":5,"inclusive":false},{"op":"range","start":1,"step":2,"count":5}]},"function":{"op":"lambda","parameters":["acc","x"],"body":{"op":"add","args":[{"local":"acc"},{"op":"at","input":{"local":"x"},"index":0}]}},"initial":0}}),
    );
    input["functions"] = json!([{"id":"make","kind":"geometry","parameters":[],"nodes":[{"id":"box","op":"box","size":[1,2,3]}],"root":"box"}]);
    let output = validate(input).unwrap();
    assert_eq!(output["functions"][0]["nodes"][0]["center"], false);
}
#[test]
fn enforces_input_budgets_before_schema_errors() {
    let mut value = Value::Null;
    for _ in 0..66 {
        value = Value::Array(vec![value]);
    }
    assert!(validate(value).is_err());
    assert!(validate(Value::Array(vec![Value::Null; 20_000])).is_err());
}
#[test]
fn rejects_out_of_range_and_invalid_shapes() {
    for radius in [
        json!(1_000_001),
        json!(true),
        json!({"op":"bogus"}),
        json!({"local":"bad-id"}),
    ] {
        assert!(
            validate(document(
                json!({"id":"shape","op":"sphere","radius":radius})
            ))
            .is_err()
        );
    }
    assert!(validate(document(json!({"id":"shape","op":"box","size":[1,2]}))).is_err());
    let mut input = document(json!({"id":"shape","op":"sphere","radius":1}));
    input["segments"] = json!(12.5);
    assert!(validate(input).is_err());
}
#[test]
fn rejects_null_optional_values_and_checks_message_code_point_length() {
    let mut input = document(json!({"id":"shape","op":"sphere","radius":1}));
    input["type_policy"] = Value::Null;
    assert!(validate(input.clone()).is_err());
    input.as_object_mut().unwrap().remove("type_policy");
    input["assertions"] = json!([{"condition":1,"message":"😀".repeat(257)}]);
    assert!(validate(input).is_err());
}
#[test]
fn pattern_schema_rejects_ambiguous_bindings_and_unknown_keys() {
    for pattern in [
        json!({"kind":"list","prefix":[{"kind":"bind","name":"x"},{"kind":"bind","name":"x"}],"suffix":[]}),
        json!({"kind":"as","name":"x","pattern":{"kind":"bind","name":"x"}}),
        json!({"kind":"or","patterns":[{"kind":"bind","name":"x"},{"kind":"wildcard"}]}),
        json!({"kind":"wildcard","ignored":1}),
        json!({"kind":"literal","value":{"local":"x"}}),
        json!({"kind":"type","name":"unknown","pattern":{"kind":"wildcard"}}),
        json!({"kind":"list","prefix":[],"suffix":[],"rest":"bad-name"}),
    ] {
        let input = document(
            json!({"id":"shape","op":"sphere","radius":{"op":"match","input":1,"arms":[{"pattern":pattern,"body":2}]}}),
        );
        assert!(validate(input).is_err());
    }
}

#[test]
fn pattern_schema_enforces_arms_nesting_and_total_size_limits() {
    let valid = json!({"pattern":{"kind":"wildcard"},"body":1});
    let input = document(
        json!({"id":"shape","op":"sphere","radius":{"op":"match","input":1,"arms":vec![valid;33]}}),
    );
    assert!(validate(input).is_err());
    let mut deep = json!({"kind":"wildcard"});
    for _ in 0..18 {
        deep = json!({"kind":"type","name":"int","pattern":deep});
    }
    let wide = json!({"kind":"list","prefix":vec![json!({"kind":"wildcard"});256],"suffix":[]});
    for pattern in [deep, wide] {
        let input = document(
            json!({"id":"shape","op":"sphere","radius":{"op":"match","input":1,"arms":[{"pattern":pattern,"body":2}]}}),
        );
        assert!(validate(input).is_err());
    }
}
#[test]
fn type_descriptors_are_strict_and_bounded() {
    let mut deep = json!({"name":"int"});
    for _ in 0..18 {
        deep = json!({"name":"Vec","args":[deep]});
    }
    let mut wide = json!({"name":"Wide","fields":{}});
    for index in 0..33 {
        wide["fields"][&format!("f{index}")] = json!({"name":"int"});
    }
    for descriptor in [
        deep,
        wide,
        json!({"name":"Vec"}),
        json!({"name":"Unknown"}),
        json!({"name":"str","fields":{}}),
        json!({"name":"int","args":[{"name":"int"}]}),
        json!({"name":"int","unknown":1}),
        json!({"name":"Point","fields":{"bad-name":{"name":"int"}}}),
    ] {
        let input = document(
            json!({"id":"shape","op":"sphere","radius":{"op":"typed_value","value":1,"type":descriptor}}),
        );
        assert!(validate(input).is_err());
    }
}

#[test]
fn dynamic_vectors_do_not_relax_numeric_or_other_surface_shapes() {
    for node in [
        json!({"id":"shape","op":"sphere","radius":{"op":"add","args":{"op":"list","items":[1,2]}}}),
        json!({"id":"shape","op":"mirror","input":"other","normal":{"op":"list","items":[1,0,0]}}),
        json!({"id":"shape","op":"polygon","points":{"op":"list","items":[]}}),
    ] {
        assert!(validate(document(node)).is_err());
    }
}
