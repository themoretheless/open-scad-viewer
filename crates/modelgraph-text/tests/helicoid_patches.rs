#[test]
fn helicoid_patches_require_motion_and_budget_fields() {
 let source=include_str!("../../../examples/rush/helicoid-patches.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in [",outer_radius: 2mm",",turns: 2",",max_deviation: 0.0001mm"] {
  assert!(modelgraph_text::compile(&source.replace(field, "")).is_err());
 }
}
