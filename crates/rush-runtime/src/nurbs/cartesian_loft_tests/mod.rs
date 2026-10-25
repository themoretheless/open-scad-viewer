use super::*;
fn document(operation: &str) -> J {
    let mut loft=json!({"id":"loft","op":operation,"inputs":["a","b"],
        "parameters":[2.,7.],"guides":["g"],"construction":"cartesian",
        "max_cells":50000,"max_map_evaluations":200000,
        "start_tangents":[[0.,0.,0.4],[0.,0.,0.4]],
        "end_tangents":[[0.,0.,0.1],[0.,0.,0.1]]});
    if operation=="guided_loft_surface" {loft["guide_parameters"]=json!([0.]);loft["error_budget"]=json!(1e-6);}
    else {loft["budget"]=json!(1e-6);}
    json!({"language":"rush/nurbs-1","units":"mm","nodes":[
        {"id":"a","op":"line_curve","start":[0.,0.,0.],"end":[1.,0.,0.]},
        {"id":"b","op":"line_curve","start":[0.,0.,1.],"end":[1.,0.,1.]},
        {"id":"g","op":"line_curve","start":[0.,0.,0.],"end":[0.,0.,1.]},loft],"root":"loft"})
}
#[test]
fn cartesian_loft_graph_retains_audit_options_and_refuses_ignored_fields() {
    for operation in ["guided_loft_surface","auto_guided_loft_surface"] {
        let input=document(operation);
        assert!(compile(input.clone()).is_ok());
        let nodes=input["nodes"].as_array().unwrap().clone();
        assert!(compile_text(nodes,&[],"loft".into()).is_ok());
        let mut wrong=input.clone();
        wrong["nodes"][3]["construction"]=json!("homogeneous");
        assert!(compile(wrong).is_err());
        let mut depleted=input.clone();
        depleted["nodes"][3]["max_cells"]=json!(0);
        assert!(compile(depleted).is_err());
        let mut unpaired=input;
        unpaired["nodes"][3].as_object_mut().unwrap().remove("end_tangents");
        assert!(compile(unpaired).is_err());
    }
}
#[test]
fn section_maps_resolve_dimensionless_values_and_validate_nullable_shapes() {
    let mut input=document("guided_loft_surface");
    let leaf=json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,{"param":"q"},1.],"weights":[1.,1.,1.]}]});
    input["parameters"]=json!([{"id":"q","value":0.2}]);
    input["nodes"][3]["section_mappings"]=json!([J::Null,{"composition":[leaf.clone(),leaf.clone()]}]);
    let compiled=compile(input.clone()).unwrap();
    assert_eq!(compiled["resolved_document"]["nodes"][3]["section_mappings"][1]["composition"][0]["pieces"][0]["controlValues"][1],json!(0.2));
    let text=compile_text(input["nodes"].as_array().unwrap().clone(),input["parameters"].as_array().unwrap(),"loft".into()).unwrap();
    assert_eq!(text["document"]["nodes"][3]["section_mappings"][0],J::Null);
    let mut wrong=input.clone();
    wrong["nodes"][3]["section_mappings"]=json!([J::Null,{"unknown":1}]);
    assert!(compile(wrong).is_err());
    let mut wrong=input.clone();
    wrong["nodes"][3]["section_mappings"]=json!([leaf.clone()]);
    assert!(compile(wrong).is_err());
    let mut wrong=input;
    wrong["nodes"][3]["section_mappings"][1]["composition"][0]["pieces"][0]["controlValues"][1]=json!({"op":"quantity","value":0.2,"unit":"mm"});
    assert!(compile_text(wrong["nodes"].as_array().unwrap().clone(),&[],"loft".into()).is_err());
}
