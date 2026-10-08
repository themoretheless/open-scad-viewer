#[test]
fn lowers_profile_sweep_examples_and_rejects_missing_or_extra_options() {
    for source in [
        include_str!("../../../examples/rush/profile-sweep.r"),
        include_str!("../../../examples/rush/closed-profile-sweep.r"),
    ] {
        let graph = rush_frontend::compile(source).unwrap();
        assert!(graph.to_string().contains("profile_sweep"));
        assert!(rush_frontend::compile(&source.replace("normal: [0,0,1],", "")).is_err());
        assert!(
            rush_frontend::compile(&source.replace("sections: 32", "sections: 32,unexpected: 1"))
                .is_err()
        );
    }
}
