#[test]
fn clothoid_requires_curvatures_length_and_budget() {
 let source=include_str!("../../../examples/rush/clothoid-extrusion.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in [",length: 2mm",",end_curvature: 2 / 1mm",",max_deviation: 0.0001mm"] {
  assert!(modelgraph_text::compile(&source.replace(field, "")).is_err());
 }
}
