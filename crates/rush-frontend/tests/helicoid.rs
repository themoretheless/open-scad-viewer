#[test]
fn helicoid_requires_radii_height_and_budget() {
 let source=include_str!("../../../examples/rush/helicoid-surface.r");
 assert!(rush_frontend::compile(source).is_ok());
 for field in [",outer_radius: 2mm",",height: 3mm",",max_deviation: 0.0001mm"] {
  assert!(rush_frontend::compile(&source.replace(field, "")).is_err());
 }
}
