#[test]
fn variable_pitch_requires_both_endpoint_pitches_and_budget(){
 let source=include_str!("../../../examples/rush/variable-pitch-helix-extrusion.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in [",start_pitch: 24mm",",end_pitch: 72mm",",max_deviation: 0.0001mm"]{
  assert!(modelgraph_text::compile(&source.replace(field,"")).is_err());
 }
}
