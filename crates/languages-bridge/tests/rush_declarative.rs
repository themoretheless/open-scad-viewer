use value_codec::Value;

#[test]
fn browser_dispatch_compiles_named_rush_ids_through_geometry_preparation() {
    let source = "// @rush/1\nparam size = 10mm range 1mm..20mm\nbody @id(\"stable-body\") = box([size,size,size])\nshow body";
    let compiled = languages_bridge::execute_rush_frontend(source);
    let result: Value = value_codec::from_str(&compiled).unwrap();
    assert_eq!(result["ok"].as_bool(),Some(true),"{compiled}");
    assert!(result["value"]["document"]["root"].as_str().unwrap().starts_with('r'));
    assert_eq!(result["value"]["customizer"][0]["name"].as_str(),Some("size"));
}
