#[test]
fn screw_surface_requires_motion_parameters_and_budget() {
 let source=include_str!("../../../examples/rush/screw-surface.r");
 assert!(rush_frontend::compile(source).is_ok());
 for field in [",height: 4mm",",turns: 1",",max_deviation: 0.0001mm"] {
  assert!(rush_frontend::compile(&source.replace(field, "")).is_err());
 }
}
