    use super::*;
    #[test]
    fn physical_y_owners_replay_exposed_sources_instead_of_inner_cached_sources() {
        let s = Spec::default();
        let shifted = Spec {
            axial_shift: 0.3,
            ..s
        };
        let root = axial_root_chart(s, RootPart::CrestCylinder, [-2., 2.]).unwrap();
        let mut root_grid = fit_axial_root(&root, 1e-4, 256).unwrap();
        root_grid.root = axial_root_chart(shifted, RootPart::CrestCylinder, [-2., 2.]).unwrap();
        root_grid.root.reverify().unwrap();
        assert!(root_grid.reverify().is_err());
        let d = axial_stock_intersection(s, Flank::Descending, [-2., 2.], 21., 128).unwrap();
        let a = axial_stock_intersection(s, Flank::Ascending, [-2., 2.], 21., 128).unwrap();
        let sd = axial_stock_intersection(shifted, Flank::Descending, [-2., 2.], 21., 128).unwrap();
        let sa = axial_stock_intersection(shifted, Flank::Ascending, [-2., 2.], 21., 128).unwrap();
        let mut stock_grid = fit_axial_stock(&d, 1e-4, 256).unwrap();
        stock_grid.stock = sd.clone();
        stock_grid.stock.reverify().unwrap();
        assert!(stock_grid.reverify().is_err());
        let tip = axial_tip_chart(&d, &a, 32).unwrap();
        let mut tip_grid = fit_axial_tip(&tip, 1e-4, 256).unwrap();
        tip_grid.tip = axial_tip_chart(&sd, &sa, 32).unwrap();
        tip_grid.tip.reverify().unwrap();
        assert!(tip_grid.reverify().is_err());
    }
