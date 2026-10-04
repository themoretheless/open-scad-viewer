#[test]
fn variable_pipe_requires_a_radius_curve_and_explicit_budget() {
 let source=include_str!("../../../examples/rush/variable-pipe-surface.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in ["radius_law: radius,",",sections: 32",",max_deviation: 0.02mm"] {
  assert!(modelgraph_text::compile(&source.replace(field, "")).is_err());
 }
 assert!(modelgraph_text::compile(include_str!("../../../examples/rush/closed-variable-pipe-surface.r")).is_ok());
}
