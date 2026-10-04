#[test]
fn catenoid_requires_scale_bounds_and_budget(){
 let source=include_str!("../../../examples/rush/catenoid-surface.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in [",scale: 2mm",",end_z: 3mm",",max_deviation: 0.0001mm"]{
  assert!(modelgraph_text::compile(&source.replace(field,"")).is_err());
 }
}
