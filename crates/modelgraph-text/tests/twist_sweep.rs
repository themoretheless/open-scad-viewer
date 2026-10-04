#[test]
fn compiles_twist_sweep_example_and_requires_rotation_fields(){
 let source=include_str!("../../../examples/rush/twist-sweep.r");
 assert!(modelgraph_text::compile(source).unwrap().to_string().contains("twist_sweep"));
 assert!(modelgraph_text::compile(&source.replace(",axis: [0,0,1]","")).is_err());
 assert!(modelgraph_text::compile(&source.replace("profile,path,origin","profile,origin")).is_err());
}
