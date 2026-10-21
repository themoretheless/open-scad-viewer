#[path = "../examples/support/interior_runout_fixture.rs"]
mod fixture;
#[test]
fn tenth_mm_interior_runout_is_root_bound_trimmed_and_closed() {
    let (source, out) = fixture::build();
    assert_eq!(
        (
            out.model.faces.len(),
            out.model.edges.len(),
            out.model.vertices.len()
        ),
        (7, 15, 10)
    );
    assert_eq!(source.faces.len(), 6);
    #[cfg(feature = "codec")]
    {
        let saved = value_codec::to_string(&out.model).unwrap();
        let restored: brep_core::Model = value_codec::from_str(&saved).unwrap();
        assert_eq!(restored, out.model);
        assert_eq!(
            brep_core::watertight::inspect_with_exact(&restored, 100000, 1000000)
                .unwrap()
                .closure
                .status,
            brep_core::watertight::Closure::WithinTolerance
        );
    }
}
