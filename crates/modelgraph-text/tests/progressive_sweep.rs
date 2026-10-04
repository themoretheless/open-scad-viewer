#[test]
fn lowers_progressive_sweep_examples_options_and_laws() {
    for source in [
        include_str!("../../../examples/rush/progressive-sweep.r"),
        include_str!("../../../examples/rush/closed-progressive-sweep.r"),
        include_str!("../../../examples/rush/arc-length-progressive-sweep.r"),
        include_str!("../../../examples/rush/multi-profile-progressive-sweep.r"),
        include_str!("../../../examples/rush/progressive-hollow-body.r"),
        include_str!("../../../examples/rush/affine-progressive-sweep.r"),
        include_str!("../../../examples/rush/affine-hollow-body.r"),
        include_str!("../../../examples/rush/closed-progressive-hollow-body.r"),
    ] {
        let graph = modelgraph_text::compile(source).unwrap();
        assert!(graph.to_string().contains("progressive_sweep"));
        assert!(
            modelgraph_text::compile(
                &source
                    .replace("normal: [1,0,0],", "")
                    .replace("normal: [0,0,1],", "")
            )
            .is_err()
        );
        assert!(
            modelgraph_text::compile(
                &source
                    .replace("max_sections: 257", "max_sections: 257,unexpected: 1")
                    .replace("max_sections: 3", "max_sections: 3,unexpected: 1")
                    .replace("max_sections: 129", "max_sections: 129,unexpected: 1")
            )
            .is_err()
        );
    }
}
