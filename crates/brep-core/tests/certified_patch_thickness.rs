use brep_core::shell_thickness::{Limits, Stop, thicken_patch_with_cap};
use nurbs_core::surface::Surface;
fn patch() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.2]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn cap(s: &Surface, d: f64) -> Surface {
    let mut c = s.clone();
    for p in c.control_points.iter_mut().flatten() {
        p[2] += d;
    }
    c
}
#[test]
fn nonplanar_skin_separates_normal_and_sewing_tolerances() {
    for d in [-0.1, 0.1] {
        let s = patch();
        let c = cap(&s, d);
        let r = thicken_patch_with_cap(&s, &c, d, 0.05, 4096, 1e-7, Limits::default()).unwrap();
        assert_eq!(r.stop, Stop::ConstructedCertifiedNormalDisplacement);
        let model = r.model.as_ref().unwrap();
        assert_eq!(model.faces[0].surface, s);
        assert_eq!(model.faces[1].surface, c);
        assert_eq!(model.faces.len(), 6);
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        assert!(r.continuous_thickness_error_upper.unwrap() <= 0.05);
        assert!(r.rounding_inclusive_thickness_certified);
        assert!(r.max_sampled_thickness_deviation.is_none());
        assert!(!r.solid_geometry_certified);
        assert!(r.agreement.unwrap().complete);
    }
}
#[test]
fn rejected_and_unresolved_caps_never_return_models() {
    let s = patch();
    let c = cap(&s, 0.1);
    let r = thicken_patch_with_cap(&s, &c, 0.1, 1e-12, 1, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.stop, Stop::NormalOffsetUnresolved);
    assert!(r.model.is_none());
    let c = cap(&s, -0.1);
    let r = thicken_patch_with_cap(&s, &c, 0.1, 0.01, 4096, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.stop, Stop::NormalOffsetExceedsTolerance);
    assert!(r.model.is_none());
}
