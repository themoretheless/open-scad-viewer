#[test]
fn involute_requires_angles_radius_and_budget(){
 let source=include_str!("../../../examples/rush/involute-extrusion.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in [",end_degrees: 60deg",",max_deviation: 0.0001mm",",radius: 5mm"]{
  assert!(modelgraph_text::compile(&source.replace(field,"")).is_err());
 }
}
