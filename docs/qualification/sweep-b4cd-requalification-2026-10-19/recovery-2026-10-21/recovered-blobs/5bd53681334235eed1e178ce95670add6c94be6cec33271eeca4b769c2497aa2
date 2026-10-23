#[test]
fn compiles_twist_sweep_example_and_requires_rotation_fields(){
 let source=include_str!("../../../examples/rush/twist-sweep.r");
 assert!(rush_frontend::compile(source).unwrap().to_string().contains("twist_sweep"));
 assert!(rush_frontend::compile(&source.replace(",axis: [0,0,1]","")).is_err());
 assert!(rush_frontend::compile(&source.replace("profile,path,origin","profile,origin")).is_err());
}
