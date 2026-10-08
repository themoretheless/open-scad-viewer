#[test]
fn compiles_helix_example_and_requires_approximation_budget(){
 let source=include_str!("../../../examples/rush/helix-extrusion.r");
 assert!(rush_frontend::compile(source).unwrap().to_string().contains("helix_curve"));
 assert!(rush_frontend::compile(&source.replace(",max_deviation: 0.0001mm","")).is_err());
}
