#[test]
fn catenary_requires_scale_bounds_and_budget(){
 let source=include_str!("../../../examples/rush/catenary-extrusion.r");
 assert!(rush_frontend::compile(source).is_ok());
 for field in [",scale: 5mm",",end_x: 3mm",",max_deviation: 0.0001mm"]{
  assert!(rush_frontend::compile(&source.replace(field,"")).is_err());
 }
}
