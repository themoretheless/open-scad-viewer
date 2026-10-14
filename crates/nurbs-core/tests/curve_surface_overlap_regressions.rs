use nurbs_core::{
    curve::Curve,
    intersection::{CurveSurfaceComponent, intersect_curve_surface_report},
    surface::Surface,
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn line(a: [f64; 3], b: [f64; 3]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1.; 2],
        periodic: false,
    }
}
#[test]
fn tiny_gap_cannot_be_reported_as_exact_overlap() {
    let c = line([0.1, 0.2, 5e-10], [0.8, 0.7, 5e-10]);
    let r = intersect_curve_surface_report(&c, &plane(), None).unwrap();
    assert!(
        !r.components
            .iter()
            .any(|x| matches!(x, CurveSurfaceComponent::Overlap(_)))
    );
    assert!(!r.unresolved.is_empty());
}
#[test]
fn support_plane_extension_is_not_finite_surface_overlap() {
    let c = line([2., 0.2, 0.], [3., 0.7, 0.]);
    let r = intersect_curve_surface_report(&c, &plane(), None).unwrap();
    assert!(
        !r.components
            .iter()
            .any(|x| matches!(x, CurveSurfaceComponent::Overlap(_)))
    );
    assert!(!r.unresolved.is_empty());
}
#[test]
fn contained_exact_segment_keeps_overlap() {
    let c = line([0.1, 0.2, 0.], [0.8, 0.7, 0.]);
    let r = intersect_curve_surface_report(&c, &plane(), None).unwrap();
    assert!(
        r.components
            .iter()
            .any(|x| matches!(x, CurveSurfaceComponent::Overlap(_)))
    );
}

#[test]
fn rational_and_nonaffine_planar_uv_are_not_fabricated() {
    let c = line([0.1, 0.2, 0.], [0.8, 0.7, 0.]);
    let mut rational = plane();
    rational.weights = vec![vec![1., 2.], vec![3., 6.]];
    let mut trapezoid = plane();
    trapezoid.control_points[1][1][0] = 0.5;
    for s in [rational, trapezoid] {
        let r = intersect_curve_surface_report(&c, &s, None).unwrap();
        assert!(
            !r.components
                .iter()
                .any(|x| matches!(x, CurveSurfaceComponent::Overlap(_)))
        );
        assert!(!r.unresolved.is_empty());
    }
}
#[test]
fn overlap_enclosure_contains_entire_segment() {
    let c = line([0.1, 0.2, 0.], [0.8, 0.7, 0.]);
    let r = intersect_curve_surface_report(&c, &plane(), None).unwrap();
    let overlap = r
        .components
        .iter()
        .find_map(|x| {
            if let CurveSurfaceComponent::Overlap(o) = x {
                Some(o)
            } else {
                None
            }
        })
        .unwrap();
    for p in &c.control_points {
        for k in 0..3 {
            assert!(
                overlap.geometry_enclosure[k][0] <= p[k]
                    && p[k] <= overlap.geometry_enclosure[k][1]
            );
        }
    }
}

#[test]
fn rational_curve_mid_sample_follows_actual_parameter() {
    let mut c = line([0.1, 0.2, 0.], [0.8, 0.7, 0.]);
    c.weights = vec![1., 3.];
    let r = intersect_curve_surface_report(&c, &plane(), None).unwrap();
    let o = r
        .components
        .iter()
        .find_map(|x| {
            if let CurveSurfaceComponent::Overlap(o) = x {
                Some(o)
            } else {
                None
            }
        })
        .unwrap();
    assert!((o.samples[1][1] - 0.625).abs() < 1e-12);
    assert!((o.samples[1][2] - 0.575).abs() < 1e-12);
}

#[test]
fn piercing_support_plane_outside_panel_has_no_surface_point() {
    let c = line([2., 0.5, -1.], [2., 0.5, 1.]);
    let r = intersect_curve_surface_report(&c, &plane(), None).unwrap();
    assert!(
        !r.components
            .iter()
            .any(|x| matches!(x, CurveSurfaceComponent::Point(_)))
    );
    assert!(!r.unresolved.is_empty());
}
#[test]
fn rational_panel_does_not_get_affine_point_uv() {
    let c = line([0.3, 0.4, -1.], [0.3, 0.4, 1.]);
    let mut s = plane();
    s.weights = vec![vec![1., 2.], vec![3., 6.]];
    let r = intersect_curve_surface_report(&c, &s, None).unwrap();
    assert!(
        !r.components
            .iter()
            .any(|x| matches!(x, CurveSurfaceComponent::Point(_)))
    );
    assert!(!r.unresolved.is_empty());
}

#[test]
fn residual_candidates_are_not_an_existence_certificate() {
    let c = line([0.3, 0.4, -1.], [0.3, 0.4, 1.]);
    let r = intersect_curve_surface_report(&c, &plane(), None).unwrap();
    assert!(!r.certified);
    assert!(
        r.components
            .iter()
            .any(|x| matches!(x, CurveSurfaceComponent::Point(_)))
    );
    assert!(r.unresolved.is_empty());
    assert_eq!(r.krawczyk_isolated, 0);
}

#[test]
#[cfg(feature = "codec")]
fn host_report_separates_search_coverage_from_proof() {
    let c = line([0.3, 0.4, -1.], [0.3, 0.4, 1.]);
    let r = intersect_curve_surface_report(&c, &plane(), None).unwrap();
    let encoded = value_codec::Serialize::to_value(&r);
    assert_eq!(
        encoded["coverage"]["searchComplete"],
        value_codec::json!(true)
    );
    assert_eq!(encoded["coverage"]["complete"], value_codec::json!(false));
    assert_eq!(encoded["coverage"]["certified"], value_codec::json!(false));
    assert_eq!(
        encoded["rounding"],
        value_codec::json!("uncertified-binary64")
    );
}
