    use super::*;
    #[test]
    fn root_owner_replay_uses_its_original_source_not_the_inner_cached_source() {
        let root = root_chart(Spec::default(), RootPart::CrestCylinder, [-0.15, 0.15]).unwrap();
        let mut grid = fit_root_chart(&root, 1e-4, 128).unwrap();
        grid.reverify().unwrap();
        // The contact proof is phase invariant. The NURBS ownership proof is
        // not: changed source phase must be tested against the actual poles.
        grid.root = root_chart(
            Spec {
                axial_shift: 0.3,
                ..Spec::default()
            },
            RootPart::CrestCylinder,
            [-0.15, 0.15],
        )
        .unwrap();
        grid.root.reverify().unwrap();
        assert!(grid.reverify().is_err());
    }
