use modelgraph_text::compile;
#[test]
fn authored_loft_examples_lower_references_without_a_javascript_host() {
    for source in [
        include_str!("../../../examples/rush/guided-loft-surface.r"),
        include_str!("../../../examples/rush/auto-guided-loft.r"),
        include_str!("../../../examples/rush/g2-loft-surface.r"),
        include_str!("../../../examples/rush/natural-loft-solid.r"),
    ] {
        let lowered = compile(source).unwrap();
        assert!(lowered["nodes"].as_array().unwrap().len() >= 4);
    }
}
#[test]
fn automatic_parameter_tolerance_is_lowered_as_a_named_option() {
    let source = include_str!("../../../examples/rush/auto-guided-loft.r").replace(
        "budget: 0.000001mm",
        "budget: 0.000001mm,parameter_tolerance: 0.00000001",
    );
    let lowered = compile(&source).unwrap();
    let node = lowered["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["op"].as_str() == Some("auto_guided_loft_surface"))
        .unwrap();
    assert_eq!(node["parameter_tolerance"].as_f64(), Some(1e-8));
}
