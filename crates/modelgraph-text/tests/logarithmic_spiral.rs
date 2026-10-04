#[test]
fn logarithmic_spiral_requires_growth_and_budget(){
 let source=include_str!("../../../examples/rush/logarithmic-spiral-extrusion.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in [",growth: 0.3",",max_deviation: 0.0001mm"]{
  assert!(modelgraph_text::compile(&source.replace(field,"")).is_err());
 }
}
