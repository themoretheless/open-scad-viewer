use super::*;

#[test]
fn geometry_match_keeps_bindings_local_and_builds_only_selected_arm() {
    let document = json!({"language":"modelgraph/1","units":"mm","parameters":[],"root":"choice","nodes":[
        {"id":"choice","op":"match","value":{"op":"record","fields":{"radius":3}},"arms":[
            {"pattern":{"kind":"record","exact":true,"fields":{"radius":{"kind":"bind","name":"r"}}},"guard":{"op":"lt","args":[0,{"local":"r"}]},"input":"good"},
            {"pattern":{"kind":"wildcard"},"input":"bad"}
        ]},
        {"id":"good","op":"sphere","radius":{"local":"r"}},
        {"id":"bad","op":"sphere","radius":-1}
    ]});
    let result = crate::compile(document.clone()).unwrap();
    assert!(result["source"].as_str().unwrap().contains("sphere(r=3);"));
    let mut invalid = document;
    invalid["nodes"][0]["arms"][1]["input"] = json!("missing");
    assert_eq!(crate::compile(invalid).unwrap_err().code, "unknown_node");
}
