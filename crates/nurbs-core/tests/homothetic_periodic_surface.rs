use nurbs_core::surface_injectivity::certify_periodic_extrusion;
#[path = "common/periodic_extrusion.rs"]
mod fixture;
fn source(scale: f64) -> nurbs_core::surface::Surface {
    let mut s = fixture::source(1);
    for row in &mut s.control_points {
        row[1][0] *= scale;
        row[1][1] *= scale;
    }
    s
}
#[test]
fn a_positive_taper_has_exact_scale_and_a_regular_periodic_quotient() {
    let s = source(0.5);
    let saved = s.clone();
    let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
    assert!(r.proven, "{}", r.reason);
    assert!(r.regular_interior);
    assert_eq!(r.profile_scale, Some([0.5, 0.5]));
    assert_eq!(r.collapsed_end, None);
    assert_eq!(s, saved);
}
#[test]
fn either_collapsed_end_has_a_declared_cone_quotient_and_regular_interior() {
    let s = source(0.);
    let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
    assert!(r.proven, "{}", r.reason);
    assert!(r.regular_interior);
    assert_eq!(r.collapsed_end, Some(1));
    assert_eq!(r.profile_scale, Some([0., 0.]));
    let mut inverse = s.clone();
    for row in &mut inverse.control_points {
        row.swap(0, 1);
    }
    for row in &mut inverse.weights {
        row.swap(0, 1);
    }
    let r = certify_periodic_extrusion(&inverse, 0, 1000, 10000).unwrap();
    assert!(r.proven, "{}", r.reason);
    assert!(r.regular_interior);
    assert_eq!(r.collapsed_end, Some(0));
}
#[test]
fn an_interior_scale_collapse_or_nonhomothetic_row_cannot_borrow_the_theorem() {
    let s = source(-0.5);
    let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
    assert!(!r.proven);
    assert_eq!(r.reason, "interior-profile-scale-collapse");
    let mut s = source(0.5);
    s.control_points[3][1][0] = s.control_points[3][1][0].next_up();
    assert!(
        !certify_periodic_extrusion(&s, 0, 1000, 10000)
            .unwrap()
            .proven
    );
}
#[test]
fn a_positive_rational_gauge_preserves_the_taper_and_apex_theorem() {
    for scale in [0., 0.5] {
        let mut s = source(scale);
        for row in &mut s.weights {
            row[1] *= 2.;
        }
        let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
        assert!(r.proven, "{}", r.reason);
        assert!(r.regular_interior);
        assert!(!certify_periodic_extrusion(&s, 0, 1000, 1).unwrap().proven);
    }
}
