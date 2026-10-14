#[test]
fn ribbon_requires_width_law_sections_and_budget() {
 let source=include_str!("../../../examples/rush/ribbon-surface.r");
 assert!(rush_frontend::compile(source).is_ok());
 for field in ["width_law: width,",",sections: 32",",max_deviation: 0.01mm"] {
  assert!(rush_frontend::compile(&source.replace(field, "")).is_err());
 }
 assert!(rush_frontend::compile(include_str!("../../../examples/rush/closed-ribbon-surface.r")).is_ok());
}
