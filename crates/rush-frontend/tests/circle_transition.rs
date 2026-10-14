#[test]
fn circle_transition_requires_both_complete_sections() {
 let source=include_str!("../../../examples/rush/circle-transition-surface.r");
 assert!(rush_frontend::compile(source).is_ok());
 for field in ["start_center: [0mm,0mm,0mm],",",start_seam: [1,0,0]",",end_normal: [1,0,0]",",end_radius: 3mm"] {
  assert!(rush_frontend::compile(&source.replace(field, "")).is_err());
 }
}
