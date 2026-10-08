#[test]
fn clothoid_requires_curvatures_length_and_budget() {
 let source=include_str!("../../../examples/rush/clothoid-extrusion.r");
 assert!(rush_frontend::compile(source).is_ok());
 for field in [",length: 2mm",",end_curvature: 2 / 1mm",",max_deviation: 0.0001mm"] {
  assert!(rush_frontend::compile(&source.replace(field, "")).is_err());
 }
}
