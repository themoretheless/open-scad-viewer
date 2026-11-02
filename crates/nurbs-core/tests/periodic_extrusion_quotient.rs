use nurbs_core::{surface::Surface, surface_injectivity::certify_periodic_extrusion};
#[path = "common/periodic_extrusion.rs"]
mod fixture;
use fixture::source;
#[test]
fn a_complete_original_rational_profile_has_only_the_proved_periodic_shore_identification() {
    let s = source(1);
    let saved = s.clone();
    let r = certify_periodic_extrusion(&s, 0, 1000, 10000).unwrap();
    assert!(r.proven, "{}", r.reason);
    assert_eq!(r.axial_coordinate, Some(2));
    assert_eq!(r.pairs, 6);
    assert!(r.cells <= 10000);
    assert_eq!(s, saved);
}
#[test]
fn transposed_storage_and_permuted_world_axes_keep_the_same_original_quotient_proof() {
    let mut s = source(1);
    for row in &mut s.control_points {
        for p in row {
            p.swap(1, 2);
        }
    }
    let n = s.control_points.len();
    let points = (0..2)
        .map(|j| (0..n).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    let weights = (0..2)
        .map(|j| (0..n).map(|i| s.weights[i][j]).collect())
        .collect();
    let t = Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: true,
    };
    let r = certify_periodic_extrusion(&t, 1, 1000, 10000).unwrap();
    assert!(r.proven, "{}", r.reason);
    assert_eq!(r.axial_coordinate, Some(1));
}
#[test]
fn double_cover_and_perturbed_tensor_weights_cannot_borrow_a_periodic_flag() {
    let double = source(2);
    let r = certify_periodic_extrusion(&double, 0, 1000, 10000).unwrap();
    assert!(!r.proven);
    assert_eq!(r.reason, "profile-jordan-unproven");
    let mut bad = source(1);
    bad.weights[3][1] = bad.weights[3][1].next_up();
    assert!(
        !certify_periodic_extrusion(&bad, 0, 1000, 10000)
            .unwrap()
            .proven
    );
}
#[test]
fn original_seam_and_resource_limits_remain_mandatory() {
    let s = source(1);
    assert!(!certify_periodic_extrusion(&s, 0, 1000, 1).unwrap().proven);
    assert!(!certify_periodic_extrusion(&s, 0, 1, 10000).unwrap().proven);
    let mut bad = s.clone();
    bad.knots_u[0] = (-1f64).next_up();
    assert!(
        !certify_periodic_extrusion(&bad, 0, 1000, 10000)
            .unwrap()
            .proven
    );
    assert!(certify_periodic_extrusion(&s, 2, 1000, 10000).is_err());
}
