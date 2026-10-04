#[test]
fn compiles_scaled_sweep_example_and_requires_scale_law() {
    let source=include_str!("../../../examples/rush/scaled-sweep.r");
    let graph=modelgraph_text::compile(source).unwrap();
    assert!(graph.to_string().contains("scaled_sweep"));
    assert!(modelgraph_text::compile("show scaled_sweep(line_curve(start:[0,0,0],end:[1,0,0]),line_curve(start:[0,0,0],end:[0,0,1]),origin:[0,0,0])").is_err());
}
