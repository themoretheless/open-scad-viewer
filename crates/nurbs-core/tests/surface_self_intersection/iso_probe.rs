use super::*;
use nurbs_core::intersection::{
    intersect_surface_iso_at_certified_report as at,
    intersect_surface_iso_certified_report as midpoint,
};
pub(super) fn quarter_surface() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 2., 3., 3.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [[0., 0.], [1., 1.], [0., 1.], [1., 0.]]
            .into_iter()
            .enumerate()
            .map(|(i, [x, y])| {
                let z = if i == 3 { 1. } else { 0. };
                vec![vec![x, y, -0.25 * z], vec![x, y, 1. + 0.75 * z]]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn off_center_contact_is_found_without_claiming_midpoint_coverage() {
    let s = quarter_surface();
    let center = midpoint(&s, 0, 1e-7, 1000).unwrap();
    assert!(center.crossings.is_empty());
    assert!(!center.complete);
    let r = at(&s, 0, 0.25, 1e-7, 1000).unwrap();
    assert_eq!(r.crossings.len(), 1);
    let c = &r.crossings[0];
    for (i, t) in [0.5, 2.5].into_iter().enumerate() {
        assert!(c.parameters[i][0] <= t && t <= c.parameters[i][1]);
    }
    for (i, x) in [0.5, 0.5, 0.25].into_iter().enumerate() {
        assert!(c.position_bounds[i][0] <= x && x <= c.position_bounds[i][1]);
    }
    assert!(!r.complete);
    assert!(!r.unresolved.is_empty());
}
#[test]
fn normalized_probe_is_mapped_to_a_nonunit_domain_exactly() {
    let mut s = quarter_surface();
    s.knots_v = vec![0.1, 0.1, 3.1, 3.1];
    let r = at(&s, 0, 0.25, 1e-7, 1000).unwrap();
    assert_eq!(r.crossings.len(), 1);
    let b = r.crossings[0].position_bounds[2];
    assert!(b[0] <= 0.25 && 0.25 <= b[1]);
}
#[test]
fn v_axis_probe_preserves_original_parameter_domains() {
    let mut s = quarter_surface();
    s.control_points = (0..2)
        .map(|j| (0..4).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..4).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = at(&s, 1, 0.25, 1e-7, 1000).unwrap();
    assert_eq!(r.crossings.len(), 1);
    for (i, t) in [0.5, 2.5].into_iter().enumerate() {
        assert!(r.crossings[0].parameters[i][0] <= t && t <= r.crossings[0].parameters[i][1]);
    }
}
#[test]
fn probe_coordinates_are_validated_without_clamping() {
    let s = quarter_surface();
    for t in [0., 1., -0.1, 1.1, f64::NAN, f64::INFINITY] {
        assert!(at(&s, 0, t, 1e-7, 100).is_err());
    }
    let r = at(&s, 0, 0.25, 1e-7, 1).unwrap();
    assert!(r.crossings.is_empty());
    assert!(!r.complete);
    assert_eq!(r.cells, 1);
}

#[test]
fn simple_selected_iso_can_coexist_with_contact_on_another_iso() {
    let mut s = super::within_span::loop_surface();
    for (i, row) in s.control_points.iter_mut().enumerate() {
        row[1] = vec![i as f64, 0., 1.];
    }
    let contact = at(&s, 0, 0.125, 1e-7, 10000).unwrap();
    assert!(!contact.crossings.is_empty());
    let simple = at(&s, 0, 0.75, 1e-7, 10000).unwrap();
    assert!(simple.complete);
    assert!(simple.crossings.is_empty());
}
