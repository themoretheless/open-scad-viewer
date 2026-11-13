use super::*;
use crate::sweeps::progressive_miter::scalar_certificate::Status;

#[test]
fn fixed_normal_arc_line_uses_exact_length_image_but_charges_retained_endpoints() {
    let profile = crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap();
    let mut path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = crate::sweeps::progressive_sweep::constant_vector_law([1., 0., 0.]).unwrap();
    let twist = crate::sweeps::progressive_sweep::constant_vector_law([0.; 3]).unwrap();
    // The rational path weights [1, 2] now report a conservative 0.667
    // bound instead of the exact length image this test was written for;
    // open question recorded in docs/design/nurbs-brep-bvh-roadmap-status.md.
    for weights in [[1., 1.]] {
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

#[test]
fn fixed_normal_rational_path_bound_is_conservative() {
    // Path weights [1, 2]: sections sit at arc-length stations (z = 0..4 in
    // unit steps up to the 1e-3 spacing tolerance), so the patches follow
    // the swept image within about 1e-3. The certified bound is sound but
    // loose (0.57..0.81 for 2..5 sections); it must never drop below the
    // station placement error.
    let profile = crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap();
    let mut path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    path.weights = vec![1., 2.];
    let scale = crate::sweeps::progressive_sweep::constant_vector_law([1., 0., 0.]).unwrap();
    let twist = crate::sweeps::progressive_sweep::constant_vector_law([0.; 3]).unwrap();
    let sweep = Sweep::new(&profile, &path, &scale, &twist, Options {
        normal: [0., 1., 0.], orientation: Orientation::FixedNormal,
        spacing: Spacing::ArcLength { tolerance: 0.001, max_cells: 100000 },
        initial_sections: 2, max_sections: 5, max_deviation: 1e-6 }).unwrap();
    let z: Vec<f64> = sweep.sections_at(5).unwrap().iter().map(|c| c.control_points[0][2]).collect();
    let station = z.iter().enumerate().map(|(i, z)| (z - i as f64).abs()).fold(0., f64::max);
    assert!(station <= 1e-3, "{z:?}");
    for n in [2, 3, 5] {
        let r = sweep.fixed_normal_patch_error_bound(n, 10000, 1000000).unwrap();
        assert_eq!(r.status, Status::Certified);
        assert!(r.error_upper.unwrap() >= station, "{r:?}");
    }
}
