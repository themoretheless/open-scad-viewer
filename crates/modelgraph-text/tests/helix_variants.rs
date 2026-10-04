#[test]
fn compiles_helix_variants_and_requires_budget_and_radii(){
 for source in [include_str!("../../../examples/rush/elliptic-helix-extrusion.r"),include_str!("../../../examples/rush/conical-helix-extrusion.r")]{
  assert!(modelgraph_text::compile(source).is_ok());
  assert!(modelgraph_text::compile(&source.replace(",max_deviation: 0.0001mm","")).is_err());
 }
 let elliptic=include_str!("../../../examples/rush/elliptic-helix-extrusion.r");
 assert!(modelgraph_text::compile(&elliptic.replace(",radius_y: 3mm","")).is_err());
 let conical=include_str!("../../../examples/rush/conical-helix-extrusion.r");
 assert!(modelgraph_text::compile(&conical.replace(",end_radius: 2mm","")).is_err());
}
