#[test]
fn catenoid_patches_requires_profile_conditions_and_budget() {
 let source=include_str!("../../../examples/rush/catenoid-patches.r");
 assert!(rush_frontend::compile(source).is_ok());
 for field in [",scale: 1mm",",end_z: 5mm",",max_deviation: 0.0001mm"] {
  assert!(rush_frontend::compile(&source.replace(field, "")).is_err());
 }
}
