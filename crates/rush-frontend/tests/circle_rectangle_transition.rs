#[test]
fn circle_rectangle_requires_circle_frame_and_rectangle_half_edges() {
 let source=include_str!("../../../examples/rush/circle-rectangle-transition.r");
 assert!(rush_frontend::compile(source).is_ok());
 for field in [",circle_seam: [1,0,0]",",circle_radius: 2mm",",rectangle_axis_v: [0mm,1mm,0mm]"] {
  assert!(rush_frontend::compile(&source.replace(field, "")).is_err());
 }
}
