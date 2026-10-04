#[test]
fn toroidal_constructors_require_explicit_parameters() {
 for (source,fields) in [
  (include_str!("../../../examples/rush/toroidal-spiral-extrusion.r"),[",minor_turns: 0.375",",max_deviation: 0.0001mm"]),
  (include_str!("../../../examples/rush/torus-knot-extrusion.r"),[",q: 3",",max_deviation: 0.0001mm"]),
 ] {
  assert!(modelgraph_text::compile(source).is_ok());
  for field in fields {assert!(modelgraph_text::compile(&source.replace(field, "")).is_err());}
 }
}
