use super::*;

fn quantity(value: f64) -> Value {
    json!({"op":"quantity","value":value,"unit":"mm"})
}
fn width(id: &str, target: &str, expected: Value) -> Value {
    json!({"id":id,"target":target,"check":"width","expected":expected,"message":"Width"})
}

#[test]
fn hidden_and_transformed_targets_have_separate_sources() {
    let document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"scaled","nodes":[
        {"id":"original","op":"box","size":[2,3,4]},
        {"id":"scaled","op":"scale","input":"original","vector":[5,1,1]},
        {"id":"hidden","op":"box","size":[7,8,9]}
    ],"geometry_assertions":[width("before","original",quantity(2.0)),width("after","scaled",quantity(10.0)),width("invisible","hidden",quantity(7.0))]});
    let result = crate::compile(document).unwrap();
    let checks = result["geometry_assertions"].as_array().unwrap();
    assert!(checks[0]["source"].as_str().unwrap().contains("[2,3,4]"));
    assert!(!checks[0]["source"].as_str().unwrap().contains("scale("));
    assert!(checks[1].get("source").is_none());
    assert!(checks[2]["source"].as_str().unwrap().contains("[7,8,9]"));
    assert!(!result["source"].as_str().unwrap().contains("[7,8,9]"));
    assert!(
        result["source_map"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["node_id"] != "hidden")
    );
}

#[test]
fn scalar_checks_capture_each_loop_scope_and_ignore_inactive_branches() {
    let checked = json!({"op":"assert_value","value":1,"geometry_assertions":[width("localWidth","part",json!({"op":"multiply","args":[{"local":"i"},quantity(1.0)]}))]});
    let document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"instances","nodes":[
        {"id":"instances","op":"collect","binding":"i","values":{"op":"list","items":[2,3]},"input":"selected"},
        {"id":"selected","op":"if","condition":checked,"then":"part","else":"inactive"},
        {"id":"part","op":"box","size":[{"local":"i"},1,1]},
        {"id":"inactive","op":"if","condition":{"op":"assert_value","value":1,"constraints":[{"id":"never","left":0,"relation":"eq","right":1,"message":"Inactive"}]},"then":"part","else":"part"}
    ]});
    let result = crate::compile(document).unwrap();
    let checks = result["geometry_assertions"].as_array().unwrap();
    assert_eq!(checks.len(), 2);
    assert_eq!(checks[0]["expected"], 2.0);
    assert_eq!(checks[1]["expected"], 3.0);
    assert!(checks[0]["source"].as_str().unwrap().contains("[2,1,1]"));
    assert!(checks[1]["source"].as_str().unwrap().contains("[3,1,1]"));
}

#[test]
fn geometry_checks_in_numeric_functions_keep_bound_arguments() {
    let document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"shown","nodes":[
        {"id":"shown","op":"sphere","radius":{"op":"call","function":"checkedRadius","args":{"r":3}}},
        {"id":"target","op":"sphere","radius":{"local":"r"}}
    ],"functions":[{"id":"checkedRadius","kind":"scalar","parameters":["r"],"body":{"op":"assert_value","value":{"local":"r"},"geometry_assertions":[width("diameter","target",quantity(6.0))]}}]});
    let result = crate::compile(document).unwrap();
    assert!(
        result["geometry_assertions"][0]["source"]
            .as_str()
            .unwrap()
            .contains("sphere(r=3)")
    );
    assert_eq!(result["geometry_assertions"].as_array().unwrap().len(), 1);
}

#[test]
fn scalar_assertions_return_values_and_preserve_all_failures_in_a_chain() {
    let constraints = json!([
        {"id":"low","left":{"local":"r"},"relation":"ge","right":4,"message":"Too small"},
        {"id":"high","left":{"local":"r"},"relation":"le","right":2,"message":"Too large"}
    ]);
    let document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"shown","nodes":[
        {"id":"shown","op":"sphere","radius":{"op":"let","name":"r","value":3,"body":{"op":"assert_value","value":{"local":"r"},"constraints":constraints}}}
    ]});
    let error = crate::compile(document).unwrap_err();
    assert_eq!(error.code, "constraint_failed");
    assert_eq!(error.details.unwrap().as_array().unwrap().len(), 2);
}

#[test]
fn geometry_effects_check_discarded_function_results_without_displaying_them() {
    let mut document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"shown","nodes":[
        {"id":"shown","op":"sphere","radius":{"op":"geometry_effects","input":"hidden","value":1}},
        {"id":"hidden","op":"if","condition":{"op":"assert_value","value":1,"constraints":[{"id":"local","left":1,"relation":"eq","right":1,"message":"Discarded local"}]},"then":"profile","else":"profile"},
        {"id":"profile","op":"circle","radius":2}
    ]});
    let result = crate::compile(document.clone()).unwrap();
    assert!(result["geometry_assertions"].as_array().unwrap().is_empty());
    assert_eq!(result["constraint_report"].as_array().unwrap().len(), 1);
    assert!(!result["source"].as_str().unwrap().contains("circle("));
    document["nodes"][1]["condition"]["constraints"][0]["right"] = json!(2);
    assert_eq!(
        crate::compile(document).unwrap_err().code,
        "constraint_failed"
    );
}

#[test]
fn memoized_assertions_preserve_one_hundred_distinct_loop_iterations() {
    let check = json!({"op":"memo","id":"checkedOnce","value":{"op":"assert_value","value":1,"geometry_assertions":[{"id":"local","target":"part","check":"hasBodies","expected":1,"message":"Each instance"}]}});
    let document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"instances","nodes":[
        {"id":"instances","op":"map","count":100,"index":"i","input":"shown"},
        {"id":"shown","op":"sphere","radius":{"op":"checked","checks":[check.clone(),check.clone()],"value":check}},
        {"id":"part","op":"box","size":[{"op":"add","args":[{"local":"i"},1]},1,1]}
    ]});
    let result = crate::compile(document).unwrap();
    assert_eq!(result["geometry_assertions"].as_array().unwrap().len(), 100);
    assert!(
        result["geometry_assertions"][99]["source"]
            .as_str()
            .unwrap()
            .contains("[100,1,1]")
    );
}

#[test]
fn document_checks_and_geometry_share_the_global_memo_scope() {
    let value = json!({"op":"memo","id":"global","value":{"op":"assert_value","value":1,"geometry_assertions":[{"id":"single","target":"shown","check":"hasBodies","expected":1,"message":"Global"}]}});
    let document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"shown","nodes":[{"id":"shown","op":"sphere","radius":value.clone()}],"constraints":[{"id":"rootConstraint","left":value,"relation":"eq","right":1,"message":"Global constraint"}]});
    let result = crate::compile(document).unwrap();
    assert_eq!(result["geometry_assertions"].as_array().unwrap().len(), 1);
}

#[test]
fn hidden_target_executes_its_embedded_assertions_exactly_once() {
    let mut document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"shown","nodes":[
        {"id":"shown","op":"sphere","radius":1},
        {"id":"hidden","op":"if","condition":{"op":"assert_value","value":1,"geometry_assertions":[width("inner","hidden",quantity(2.0))]},"then":"part","else":"part"},
        {"id":"part","op":"box","size":[2,3,4]}
    ],"geometry_assertions":[width("outer","hidden",quantity(2.0))]});
    let result = crate::compile(document.clone()).unwrap();
    assert_eq!(result["geometry_assertions"].as_array().unwrap().len(), 2);
    assert_eq!(
        result["geometry_assertions"][0]["source"],
        result["geometry_assertions"][1]["source"]
    );
    assert!(!result["source"].as_str().unwrap().contains("cube("));
    document["nodes"][1]["condition"]["constraints"] = json!([{"id":"fails","left":1,"relation":"eq","right":2,"message":"Hidden nested check"}]);
    assert_eq!(
        crate::compile(document).unwrap_err().code,
        "constraint_failed"
    );
}

#[test]
fn sibling_local_scalar_assertions_are_aggregated_before_the_return_value() {
    let document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"shown","nodes":[
        {"id":"shown","op":"sphere","radius":{"op":"checked","checks":[
            {"op":"assert_value","value":1,"constraints":[{"id":"first","left":1,"relation":"eq","right":2,"message":"First failure"}]},
            {"op":"assert_value","value":1,"constraints":[{"id":"second","left":3,"relation":"eq","right":4,"message":"Second failure"}]}
        ],"value":{"op":"divide","args":[1,0]}}}
    ]});
    let error = crate::compile(document).unwrap_err();
    assert_eq!(error.code, "constraint_failed");
    assert_eq!(error.details.unwrap().as_array().unwrap().len(), 2);
}

#[test]
fn expanded_geometry_assertions_are_bounded() {
    let document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"instances","nodes":[
        {"id":"instances","op":"map","index":"i","count":256,"input":"checked"},
        {"id":"checked","op":"if","condition":{"op":"assert_value","value":1,"geometry_assertions":[width("first","part",quantity(1.0)),width("second","part",quantity(1.0))]},"then":"part","else":"part"},
        {"id":"part","op":"box","size":[1,1,1]}
    ]});
    assert_eq!(
        crate::compile(document).unwrap_err().code,
        "assertion_limit"
    );
}

#[test]
fn unknown_and_ambiguous_pending_targets_fail_explicitly() {
    let mut document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"shown","nodes":[
        {"id":"shown","op":"sphere","radius":{"op":"assert_value","value":1,"geometry_assertions":[width("test","missing",quantity(1.0))]}}
    ]});
    assert_eq!(
        crate::compile(document.clone()).unwrap_err().code,
        "unknown_node"
    );
    document["nodes"][0]["radius"]["geometry_assertions"][0]["target"] = json!("shown");
    document["functions"] = json!([{"id":"other","kind":"geometry","parameters":[],"root":"shown","nodes":[{"id":"shown","op":"sphere","radius":2}]}]);
    assert_eq!(
        crate::compile(document).unwrap_err().code,
        "ambiguous_assertion_target"
    );
}
