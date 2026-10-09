use rush_frontend::compile;
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

#[test]
fn cartesian_authored_tangent_examples_preserve_units_and_budgets() {
    for source in [include_str!("../../../examples/rush/cartesian-control-tangent-loft.r"),
                   include_str!("../../../examples/rush/cartesian-auto-control-tangent-loft.r")] {
        let lowered = compile(source).unwrap();
        let node = lowered["nodes"].as_array().unwrap().iter()
            .find(|n| n["construction"] == "cartesian").unwrap();
        assert_eq!(node["max_cells"].as_f64(), Some(50000.));
        assert_eq!(node["max_map_evaluations"].as_f64(), Some(200000.));
        assert_eq!(node["parameters"], value_codec::json!([2.,7.]));
        assert_eq!(node["start_tangents"].as_array().unwrap().len(), 2);
    }
}

#[test]
fn mapped_loft_descriptors_lower_without_geometry_nodes_or_inline_records() {
    for source in [include_str!("../../../examples/rush/cartesian-mapped-loft.r"),
                   include_str!("../../../examples/rush/mapped-natural-loft.r")] {
        let lowered=compile(source).unwrap();
        let nodes=lowered["nodes"].as_array().unwrap();
        assert!(!nodes.iter().any(|n|n["op"]=="loft_parameter_map"));
        let node=nodes.iter().find(|n|n.get("section_mappings").is_some()).unwrap();
        assert_eq!(node["section_mappings"].as_array().unwrap().len(),2);
        assert_eq!(node["parameters"],value_codec::json!([2.,7.]));
    }
}

#[test]
fn authored_nonplanar_cap_example_lowers_carriers_and_uv_trim_references() {
    let lowered=compile(include_str!("../../../examples/rush/authored-nonplanar-cap-loft.r")).unwrap();
    let node=lowered["nodes"].as_array().unwrap().iter().find(|n|n["op"]=="brep_capped_loft").unwrap();
    assert_eq!(node["cap_surfaces"].as_array().unwrap().len(),2);
    assert_eq!(node["cap_trims"][0][0].as_array().unwrap().len(),4);
    assert_eq!(node["embedding_limits"]["facePairs"],value_codec::json!(10000));
}
