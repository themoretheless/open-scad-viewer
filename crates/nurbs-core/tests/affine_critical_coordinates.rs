//! Whole original charts under exact mixed world coordinates, including the
//! transposed source axes. A positional/injective proof never supplies rank.
use nurbs_core::{surface::Surface, surface_edit, surface_injectivity};
#[path = "support/gram_seam_map.rs"]
mod source;
#[test]
fn both_original_parameter_orders_own_the_same_fixed_functional_theorem() {
    let (mut surface, _) = source::fixture();
    let rows = [[1., 1., 1.], [1., 2., 1.], [1., 1., 2.]];
    let offset = [8., -4., 2.];
    for p in surface.control_points.iter_mut().flatten() {
        let before = p.clone();
        for k in 0..3 {
            p[k] = offset[k] + (0..3).map(|j| rows[k][j] * before[j]).sum::<f64>();
        }
    }
    let transpose = surface_edit::transpose(&surface).unwrap();
    for s in [&surface, &transpose] {
        let before = s.clone();
        let report = surface_injectivity::certify_chart(s, 1000).unwrap();
        assert!(report.proven, "{report:?}");
        assert_eq!(report.reason, "global-exact-affine-separable-monotonicity");
        assert!(report.linear_projection.is_some() && report.contraction_upper.is_none());
        assert!(!surface_injectivity::certify_chart(s, 1).unwrap().proven);
        assert!(
            !surface_injectivity::certify_contraction(s, 1000)
                .unwrap()
                .proven
        );
        let regular = nurbs_core::surface_regularity::inspect(s, 10000).unwrap();
        assert!(regular.spanwise_regular, "{regular:?}");
        assert!(regular.cells < 10000);
        assert!(
            nurbs_core::surface_regularity::inspect(s, regular.cells)
                .unwrap()
                .spanwise_regular
        );
        assert!(
            !nurbs_core::surface_regularity::inspect(s, regular.cells - 1)
                .unwrap()
                .spanwise_regular
        );
        assert_eq!(*s, before);
    }
}
