use super::*;
fn sphere() -> J {
    json!({"id":"Shape","op":"sdf_sphere","center":[0,0,0],"radius":10})
}
fn document(nodes: Vec<J>, root: &str) -> J {
    json!({"language":"rush/nurbs-1","units":"mm","nodes":nodes,"root":root})
}

#[test]
fn curve_extrusion_references_participate_in_graph_and_conditional_traversal() {
    let curve = json!({"id":"Curve","op":"curve","degree":1,"knots":[0,0,1,1],"control_points":[[0,0],[1,0]],"weights":[1,1]});
    let mut body = json!({"id":"Body","op":"brep_extrude_curves","loops":[["Curve"]],"z_min":-2,"z_max":3});
    assert!(compile(document(vec![curve.clone(), body.clone()], "Body")).is_ok());
    body["loops"] = json!([["Missing"]]);
    assert_eq!(
        compile(document(vec![curve.clone(), body.clone()], "Body"))
            .unwrap_err()
            .code,
        "unknown_node"
    );
    body["loops"] = json!([["Body"]]);
    assert_eq!(
        compile(document(vec![body.clone()], "Body"))
            .unwrap_err()
            .code,
        "cycle"
    );
    body["loops"] = json!([vec!["Curve"; 128], vec!["Curve"; 127]]);
    assert_eq!(
        compile(document(vec![curve.clone(), body.clone()], "Body"))
            .unwrap_err()
            .code,
        "reference_limit"
    );
    body["loops"] = json!([["Chosen"]]);
    let choice = json!({"id":"Chosen","op":"if","condition":1,"then":"Curve","else":"Missing"});
    let prepared = compile_text(vec![curve, choice, body], &[], "Body".into()).unwrap();
    assert_eq!(
        prepared["document"]["nodes"][0]["loops"],
        json!([["Curve"]])
    );
    assert_eq!(prepared["document"]["nodes"].as_array().unwrap().len(), 2);
    let body = json!({"id":"Empty","op":"brep_extrude_curves","loops":[],"z_min":{"op":"quantity","value":-2,"unit":"mm"},"z_max":{"op":"quantity","value":3,"unit":"mm"}});
    assert_eq!(
        compile_text(vec![body.clone()], &[], "Empty".into()).unwrap()["document"]["nodes"][0]
            ["z_min"],
        -2
    );
    let mut bad = body;
    bad["z_max"]["unit"] = json!("deg");
    assert!(compile_text(vec![bad], &[], "Empty".into()).is_err());
}

#[test]
fn text_curve_periodicity_survives_canonical_compilation() {
    let curve = json!({"id":"Curve","op":"nurbs_curve","degree":2,
        "knots":[0,1,2,3,4,5,6,7,8],
        "control_points":[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],
        "weights":[1,1,1,1,1,1],"periodic":true});
    let body = json!({"id":"Body","op":"surface_extrude","input":"Curve","vector":[0,0,10]});
    let compiled = compile_text(vec![curve.clone(),body.clone()], &[], "Body".into()).unwrap();
    assert_eq!(compiled["document"]["nodes"][0]["periodic"], true);
    assert_eq!(compiled["resolved_document"]["nodes"][0]["periodic"], true);
    let mut invalid = curve;
    invalid["periodic"] = json!("true");
    assert!(compile_text(vec![invalid,body], &[], "Body".into()).is_err());
}

#[test]
fn validates_defaults_and_resolves_parameters_without_changing_authoring_document() {
    let input = json!({"language":"rush/nurbs-1","units":"mm","parameters":[{"id":"Radius","value":5}],"nodes":[{"id":"Curve","op":"curve","degree":1,"knots":[0,0,1,1],"control_points":[[0,0],[{"param":"Radius"},1]],"weights":[1,1]},{"id":"Body","op":"surface_extrude","input":"Curve","vector":[0,0,{"param":"Radius"}]}],"root":"Body"});
    let result = compile(input).unwrap();
    assert_eq!(result["document"]["nodes"][0]["periodic"], false);
    assert_eq!(
        result["document"]["nodes"][0]["control_points"][1][0],
        json!({"param":"Radius"})
    );
    assert_eq!(
        result["resolved_document"]["nodes"][0]["control_points"][1][0],
        5
    );
    assert_eq!(
        result["resolved_document"]["nodes"][1]["vector"],
        json!([0, 0, 5])
    );
    assert_eq!(
        compile(document(vec![sphere()], "Shape")).unwrap()["document"]["parameters"],
        json!([])
    );
}

#[test]
fn rejects_bad_shapes_enums_and_scalar_records_without_union_backtracking() {
    let mut shape = sphere();
    shape["radius"] = json!({"param":"Radius","extra":1});
    assert_eq!(
        compile(document(vec![shape], "Shape")).unwrap_err().path,
        "/nodes/0/radius"
    );
    let mut shape = sphere();
    shape["center"] = json!([0, 0]);
    assert_eq!(
        compile(document(vec![shape], "Shape")).unwrap_err().code,
        "invalid_document"
    );
    let mut shape = sphere();
    shape["id"] = json!("invalid-id");
    assert_eq!(
        compile(document(vec![shape], "Shape")).unwrap_err().path,
        "/nodes/0/id"
    );
    let mut shape = sphere();
    shape["op"] = json!("unknown");
    assert_eq!(
        compile(document(vec![shape], "Shape")).unwrap_err().path,
        "/nodes/0/op"
    );
    let mut shape = sphere();
    shape["radius"] = json!(1000001);
    assert_eq!(
        compile(document(vec![shape], "Shape")).unwrap_err().code,
        "invalid_document"
    );
    let input = document(
        vec![
            json!({"id":"A","op":"curve","degree":1,"knots":[0,0,1,1],"control_points":[[0,0,0,0],[1,1]],"weights":[1,1]}),
        ],
        "A",
    );
    assert_eq!(compile(input).unwrap_err().code, "invalid_document");
}

#[test]
fn rejects_duplicate_unknown_and_unreachable_references_with_original_paths() {
    let mut input = document(vec![sphere()], "Shape");
    input["parameters"] = json!([{"id":"R","value":1},{"id":"R","value":2}]);
    assert_eq!(compile(input).unwrap_err().code, "duplicate_parameter");
    let mut shape = sphere();
    shape["radius"] = json!({"param":"Missing"});
    let error = compile(document(vec![shape], "Shape")).unwrap_err();
    assert_eq!(
        (error.code.as_str(), error.path.as_str()),
        ("unknown_parameter", "/nodes/0/radius")
    );
    assert_eq!(
        compile(document(vec![sphere(), sphere()], "Shape"))
            .unwrap_err()
            .code,
        "duplicate_node"
    );
    let unused = json!({"id":"Other","op":"sdf_offset","input":"Shape","distance":1});
    assert_eq!(
        compile(document(vec![sphere(), unused], "Shape"))
            .unwrap_err()
            .code,
        "unreachable_node"
    );
    let cycle = json!({"id":"Cycle","op":"sdf_offset","input":"Cycle","distance":1});
    assert_eq!(
        compile(document(vec![cycle], "Cycle")).unwrap_err().path,
        "/nodes/Cycle"
    );
    assert_eq!(
        compile(document(vec![sphere()], "Missing"))
            .unwrap_err()
            .code,
        "unknown_node"
    );
}

#[test]
fn graph_depth_budget_counts_cached_shared_branches() {
    let mut nodes = vec![sphere()];
    let mut input = "Shape".to_owned();
    for i in 0..31 {
        let id = format!("N{i}");
        nodes.push(json!({"id":id,"op":"sdf_offset","input":input,"distance":1}));
        input = id;
    }
    compile(document(nodes.clone(), &input)).unwrap();
    nodes.push(json!({"id":"Join","op":"sdf_union","inputs":["Shape",input]}));
    assert_eq!(
        compile(document(nodes, "Join")).unwrap_err().code,
        "depth_limit"
    );
}

#[test]
fn value_and_raw_document_budgets_still_apply() {
    let vertices = vec![json!([0, 0, 0]); 6144];
    let triangles = vec![json!([0, 1, 2]); 2048];
    let input = document(
        vec![
            json!({"id":"Mesh","op":"triangle_mesh","vertices":vertices,"triangles":triangles}),
        ],
        "Mesh",
    );
    assert_eq!(compile(input).unwrap_err().code, "value_limit");
    let mut input = document(vec![sphere()], "Shape");
    input["extra"] = json!("x".repeat(250001));
    assert_eq!(compile(input).unwrap_err().code, "document_limit");
}

#[test]
fn text_choices_are_lazy_and_unit_errors_preserve_text_diagnostics() {
    let mut bad = sphere();
    bad["id"] = json!("Bad");
    bad["radius"] = json!({"op":"quantity","value":3,"unit":"deg"});
    let choice = json!({"id":"Choice","op":"if","condition":{"param":"Pick"},"then":"Shape","else":"Bad"});
    let output = compile_text(
        vec![sphere(), bad.clone(), choice],
        &[json!({"id":"Pick","value":1})],
        "Choice".into(),
    )
    .unwrap();
    assert_eq!(output["document"]["root"], "Shape");
    assert_eq!(output["document"]["nodes"].as_array().unwrap().len(), 1);
    let error = compile_text(vec![bad], &[], "Bad".into()).unwrap_err();
    assert_eq!(error.code, "text_error");
    assert!(
        error
            .message
            .starts_with("Rush radius: Expected length^1 angle^0")
    );
    let mut arithmetic = sphere();
    arithmetic["radius"] =
        json!({"op":"add","args":[{"op":"quantity","value":1,"unit":"mm"},1]});
    assert!(
        compile_text(vec![arithmetic], &[], "Shape".into())
            .unwrap_err()
            .message
            .starts_with("Rush radius: Incompatible dimensions")
    );
}

#[test]
fn scaled_sweep_scale_law_is_dimensionless_and_origin_is_length() {
    let profile=json!({"id":"Profile","op":"line_curve","start":[1,0,0],"end":[2,0,0]});
    let path=json!({"id":"Path","op":"line_curve","start":[0,0,0],"end":[0,0,4]});
    let sweep=json!({"id":"Sweep","op":"scaled_sweep","inputs":["Profile","Path"],
        "origin":[{"op":"quantity","value":1,"unit":"cm"},0,0],
        "scale":{"degree":1,"knots":[0,0,1,1],"values":[1,2],"weights":[1,1]}});
    let output=compile_text(vec![profile.clone(),path.clone(),sweep.clone()],&[],"Sweep".into()).unwrap();
    let nodes=output["document"]["nodes"].as_array().unwrap();
    let law=nodes.iter().find(|n|n["op"]=="scaled_sweep").unwrap();
    assert_eq!(law["origin"],json!([10.0,0.0,0.0]));
    assert_eq!(law["scale"]["values"],json!([1.0,2.0]));
    let mut bad=sweep;
    bad["scale"]["values"][1]=json!({"op":"quantity","value":2,"unit":"mm"});
    let error=compile_text(vec![profile,path,bad],&[],"Sweep".into()).unwrap_err();
    assert!(error.message.contains("scale/values/1"));
}

#[test]
fn text_matrix_translation_uses_lengths_and_linear_part_uses_scalars() {
    let transform = json!({"id":"Moved","op":"transform","input":"Shape","matrix":[[1,0,0,{"op":"quantity","value":2,"unit":"cm"}],[0,1,0,0],[0,0,1,0],[0,0,0,1]]});
    let result = compile_text(vec![sphere(), transform.clone()], &[], "Moved".into()).unwrap();
    assert_eq!(result["document"]["nodes"][1]["matrix"][0][3], 20.0);
    let mut bad = transform;
    bad["matrix"][0][0] = json!({"op":"quantity","value":1,"unit":"mm"});
    assert_eq!(
        compile_text(vec![sphere(), bad], &[], "Moved".into())
            .unwrap_err()
            .path,
        "matrix/0/0"
    );
}
