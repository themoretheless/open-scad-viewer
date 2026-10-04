#[test]
fn circular_rolling_requires_both_radii_and_budget(){
 for source in [include_str!("../../../examples/rush/epicycloid-extrusion.r"),include_str!("../../../examples/rush/hypocycloid-extrusion.r")]{
  assert!(modelgraph_text::compile(source).is_ok());
  for field in [",fixed_radius: 3mm",",rolling_radius: 1mm",",max_deviation: 0.0001mm"]{
   assert!(modelgraph_text::compile(&source.replace(field,"")).is_err());
  }
 }
}
