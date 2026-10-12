use super::*;
fn document(nodes: Value, root: &str) -> Value {
    json!({"language":"modelgraph/1","units":"mm","parameters":[],"nodes":nodes,"root":root})
}

#[test]
fn uses_dynamic_box_and_transform_vectors_with_correct_units() {
    let size = json!({"op":"match","input":3,"arms":[{"pattern":{"kind":"bind","name":"n"},"body":{"op":"list","items":[{"local":"n"},4,5]}}]});
    for (op, unit, result) in [
        ("translate", "mm", "translate([2,0,0])"),
        ("rotate", "deg", "rotate([2,0,0])"),
        ("scale", "", "scale([2,1,1])"),
    ] {
        let first = if unit.is_empty() {
            json!(2)
        } else {
            json!({"op":"quantity","value":2,"unit":unit})
        };
        let remaining = if op == "scale" { 1 } else { 0 };
        let nodes = json!([
            {"id":"base","op":"box","size":size},
            {"id":"shape","op":op,"input":"base","vector":{"op":"list","items":[first,remaining,remaining]}}
        ]);
        let output = crate::compile(document(nodes, "shape")).unwrap();
        let source = output["source"].as_str().unwrap();
        assert!(source.contains("cube([3,4,5]"));
        assert!(source.contains(result), "{source}");
    }
    let nodes = json!([
        {"id":"profile","op":"rectangle","size":{"op":"list","items":[3,4]}},
        {"id":"shape","op":"extrude","input":"profile","height":2}
    ]);
    assert!(
        crate::compile(document(nodes, "shape")).unwrap()["source"]
            .as_str()
            .unwrap()
            .contains("square([3,4]")
    );
}

#[test]
fn rejects_wrong_vector_lengths_scalar_results_and_dimensions() {
    for vector in [
        json!({"op":"list","items":[1,2]}),
        json!({"op":"list","items":[1,2,3,4]}),
        json!(2),
    ] {
        let nodes = json!([{"id":"shape","op":"box","size":vector}]);
        assert_eq!(
            crate::compile(document(nodes, "shape")).unwrap_err().code,
            "type_error"
        );
    }
    let nodes = json!([
        {"id":"base","op":"box","size":[1,2,3]},
        {"id":"shape","op":"translate","input":"base","vector":{"op":"list","items":[{"op":"quantity","value":2,"unit":"deg"},0,0]}}
    ]);
    assert_eq!(
        crate::compile(document(nodes, "shape")).unwrap_err().code,
        "unit_mismatch"
    );
}
