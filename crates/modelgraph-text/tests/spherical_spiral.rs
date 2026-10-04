#[test]
fn spherical_spiral_requires_explicit_rates_phases_and_budget() {
 let source=include_str!("../../../examples/rush/spherical-spiral-extrusion.r");
 assert!(modelgraph_text::compile(source).is_ok());
 for field in [",latitude_turns: 0.5",",latitude_phase_degrees: -90deg",",max_deviation: 0.0001mm"] {
  assert!(modelgraph_text::compile(&source.replace(field, "")).is_err());
 }
}
