use super::*;

#[test]
fn fixed_normal_arc_line_uses_exact_length_image_but_charges_retained_endpoints() {
    let profile = crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap();
    let mut path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = crate::sweeps::progressive_sweep::constant_vector_law([1., 0., 0.]).unwrap();
    let twist = crate::sweeps::progressive_sweep::constant_vector_law([0.; 3]).unwrap();
    for weights in [[1., 1.], [1., 2.]] {
        path.weights = weights.to_vec();
        let before = path.clone();
        let sweep = Sweep::new(
            &profile,
            &path,
            &scale,
            &twist,
            Options {
                normal: [0., 1., 0.],
                orientation: Orientation::FixedNormal,
                spacing: Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                },
                initial_sections: 2,
                max_sections: 2,
                max_deviation: 1e-6,
            },
        )
        .unwrap();
        let report = sweep
            .fixed_normal_patch_error_bound(2, 10000, 1000000)
            .unwrap();
        assert_eq!(report.status, Status::Certified, "{report:?}");
        assert!(report.error_upper.unwrap() < 1e-6, "{report:?}");
        assert!(report.cells > 0);
        assert_ne!(
            sweep
                .fixed_normal_patch_error_bound(2, 0, 1000000)
                .unwrap()
                .status,
            Status::Certified
        );
        let no_products = sweep.fixed_normal_patch_error_bound(2, 10000, 0).unwrap();
        assert_eq!(no_products.products, 0);
        // A single Bezier profile needs no decomposition products. Zero
        // unused product work is not an exhausted required proof stage.
        assert_eq!(no_products.status, Status::Certified);

        assert_eq!(path.control_points, before.control_points);
        assert_eq!(path.weights, before.weights);
    }
}
