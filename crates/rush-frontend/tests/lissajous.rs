#[test]
fn lissajous_requires_frequencies_phases_and_budget(){
 let source=include_str!("../../../examples/rush/lissajous-extrusion.r");
 assert!(rush_frontend::compile(source).is_ok());
 for field in [",frequencies: [0.125,0.25,0.375]",",phases_degrees: [0deg,30deg,60deg]",",max_deviation: 0.0001mm"]{
  assert!(rush_frontend::compile(&source.replace(field,"")).is_err());
 }
}
