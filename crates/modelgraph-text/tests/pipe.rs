#[test]
fn pipe_requires_radius_sections_and_budget() {
 let source=include_str!("../../../examples/rush/pipe-surface.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in ["radius: 1mm,",",sections: 32",",max_deviation: 0.005mm"] {
  assert!(modelgraph_text::compile(&source.replace(field, "")).is_err());
 }
 assert!(modelgraph_text::compile(include_str!("../../../examples/rush/closed-pipe-surface.r")).is_ok());
}
