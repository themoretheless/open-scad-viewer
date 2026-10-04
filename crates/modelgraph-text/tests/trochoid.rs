#[test]
fn rolling_curves_require_budget_and_radii(){
 for source in [include_str!("../../../examples/rush/trochoid-extrusion.r"),include_str!("../../../examples/rush/cycloid-extrusion.r")]{
  assert!(modelgraph_text::compile(source).is_ok());
  assert!(modelgraph_text::compile(&source.replace(",max_deviation: 0.0001mm","")).is_err());
 }
 let source=include_str!("../../../examples/rush/trochoid-extrusion.r");
 assert!(modelgraph_text::compile(&source.replace(",tracing_radius: 7mm","")).is_err());
}
