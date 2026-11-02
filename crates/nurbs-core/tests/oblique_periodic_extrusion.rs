use nurbs_core::surface_injectivity::certify_periodic_extrusion;
#[path = "common/periodic_extrusion.rs"]
mod fixture;
#[test]
fn oblique_translation_does_not_require_world_axis_alignment() {
    let mut s = fixture::source(1);
    for row in &mut s.control_points {
        for k in 0..3 {
            row[1][k] = row[0][k] + [2., -1., 3.][k];
        }
    }
    let saved = s.clone();
    let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
    assert!(r.proven, "{}", r.reason);
    assert_eq!(r.axial_coordinate, Some(2));
    assert_eq!(r.projection_axes, Some([0, 1]));
    assert!(r.source_frame.is_some());
    assert_eq!(s, saved);
}
#[test]
fn an_exact_oblique_profile_plane_and_transverse_direction_have_a_source_frame_proof() {
    let mut s = fixture::source(1);
    for row in &mut s.control_points {
        for p in row {
            let [x, y, z] = [p[0], p[1], p[2]];
            *p = vec![x + z + 5., y + z - 3., x + 2. * y + 4. * z + 2.];
        }
    }
    let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
    assert!(r.proven, "{}", r.reason);
    assert_eq!(r.axial_coordinate, None);
    assert_eq!(r.projection_axes, Some([0, 1]));
    assert!(r.source_frame.is_some());
}
#[test]
fn near_planarity_and_near_constant_translation_are_not_exact_source_theorems() {
    let mut s = fixture::source(1);
    s.control_points[3][0][2] = f64::from_bits(1);
    let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
    assert!(!r.proven);
    assert_eq!(r.reason, "nonplanar-original-profile");
    let mut s = fixture::source(1);
    s.control_points[3][1][0] = s.control_points[3][1][0].next_up();
    let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
    assert!(!r.proven);
    assert_eq!(r.reason, "nonconstant-original-translation");
}
#[test]
fn tangent_direction_and_exhausted_source_frame_work_cannot_gain_quotient_authority() {
    let mut s = fixture::source(1);
    for row in &mut s.control_points {
        for k in 0..3 {
            row[1][k] = row[0][k] + [2., 1., 0.][k];
        }
    }
    let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
    assert!(!r.proven);
    assert_eq!(r.reason, "tangent-extrusion-direction");
    let s = fixture::source(1);
    assert!(!certify_periodic_extrusion(&s, 0, 1000, 1).unwrap().proven);
}
#[test]
fn a_constant_positive_row_gauge_keeps_the_exact_transverse_translation_family() {
    let mut s = fixture::source(1);
    for row in &mut s.weights {
        row[1] *= 2.;
    }
    let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
    assert!(r.proven, "{}", r.reason);
    assert_eq!(r.projection_axes, Some([0, 1]));
}
