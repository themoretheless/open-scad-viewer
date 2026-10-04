#[test]
fn archimedean_spiral_requires_endpoint_radii_and_budget(){
 let source=include_str!("../../../examples/rush/archimedean-spiral-extrusion.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in [",start_radius: 2mm",",end_radius: 5mm",",max_deviation: 0.0001mm"]{
  assert!(modelgraph_text::compile(&source.replace(field,"")).is_err());
 }
}
