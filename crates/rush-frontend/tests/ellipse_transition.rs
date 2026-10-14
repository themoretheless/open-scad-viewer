#[test]
fn ellipse_transition_requires_both_complete_axis_frames() {
 let source=include_str!("../../../examples/rush/ellipse-transition-surface.r");
 assert!(rush_frontend::compile(source).is_ok());
 for field in ["start_center: [3mm,4mm,5mm],",",start_axis_u: [2mm,0mm,0mm]",",end_axis_v: [1mm,1mm,2mm]"] {
  assert!(rush_frontend::compile(&source.replace(field, "")).is_err());
 }
}
