#[test]
fn compiles_two_guide_example_and_requires_both_guides(){
    let source=include_str!("../../../examples/rush/two-guide-sweep.r");
    assert!(rush_frontend::compile(source).unwrap().to_string().contains("two_guide_sweep"));
    assert!(rush_frontend::compile(&source.replace("profile,a,b,width","profile,a,width")).is_err());
    assert!(rush_frontend::compile(&source.replace(",axis_z: [0,0,1]","")).is_err());
}
