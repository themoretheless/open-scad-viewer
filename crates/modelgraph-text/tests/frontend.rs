use modelgraph_text::compile;
use value_codec::json;

#[test]
fn builds_graph_without_a_javascript_host() {
    let result = compile("param radius: 2 range 1..8\nshow circle(radius).extrude(4)").unwrap();
    assert_eq!(result["root"], "n2");
    assert_eq!(
        result["nodes"][0],
        json!({"id":"n1","op":"circle","radius":{"param":"radius"}})
    );
    assert_eq!(
        result["nodes"][1],
        json!({"id":"n2","op":"extrude","input":"n1","height":4.0})
    );
}
#[test]
fn customizer_offsets_are_utf16_not_utf8() {
    let source = "// 🧱 русское описание\nparam size: -12mm range -20mm..20mm\nshow box([1,2,3])";
    let result = compile(source).unwrap();
    let start = source.find("-12").unwrap();
    let offset = source[..start].encode_utf16().count();
    assert_eq!(result["customizer"][0]["valueStart"], offset);
    assert_eq!(result["customizer"][0]["valueEnd"], offset + 3);
}
#[test]
fn closure_capture_is_lexical_and_calls_preserve_parameter_checks() {
    let result = compile(
        "param radius: 2 range 1..8\nfn make[T] x: T -> T\n  ret x\nshow sphere(make<int>(radius))",
    )
    .unwrap();
    let radius = &result["nodes"][0]["radius"];
    assert_eq!(radius["op"], "checked");
    assert_eq!(radius["checks"][0]["type"], "int");
    assert_eq!(radius["checks"][0]["value"]["param"], "radius");
}
#[test]
fn rejects_invalid_layout_and_bounded_resource_abuse() {
    for (source, error) in [
        (
            "fn f x: int -> int\nret x\nshow sphere(1)",
            "indented function body",
        ),
        ("show sphere(1) |> translate([1,2,3])", "Use .method"),
        (
            "struct Tree { child: Tree }\nshow sphere(1)",
            "Recursive value structures",
        ),
    ] {
        assert!(compile(source).unwrap_err().message.contains(error));
    }
    assert!(
        compile(&" ".repeat(262145))
            .unwrap_err()
            .message
            .contains("256 KiB")
    );
    let deep = format!("show sphere({}1{})", "(".repeat(70), ")".repeat(70));
    assert!(
        compile(&deep)
            .unwrap_err()
            .message
            .contains("nesting exceeds 64")
    );
}
#[test]
fn geometry_collection_remains_a_parameterized_collect() {
    let result =
        compile("param n: 3 range 1..8\nshow [for i in 0..<n => sphere(1).translate([i*3,0,0])]")
            .unwrap();
    let collect = result["nodes"].as_array().unwrap().last().unwrap();
    assert_eq!(collect["op"], "collect");
    assert_eq!(collect["values"]["end"]["param"], "n");
}
