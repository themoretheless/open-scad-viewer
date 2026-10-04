#[test]
fn helicoid_requires_radii_height_and_budget() {
 let source=include_str!("../../../examples/rush/helicoid-surface.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in [",outer_radius: 2mm",",height: 3mm",",max_deviation: 0.0001mm"] {
  assert!(modelgraph_text::compile(&source.replace(field, "")).is_err());
 }
}
