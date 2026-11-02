use brep_core::shell_thickness::planar_network::{self, Limits};
use nurbs_core::surface::Surface;
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize};
fn patch(left: bool, degree: usize, lambda: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: degree,
        knots_u: vec![2., 2., 5., 5.],
        knots_v: [vec![-4.; degree + 1], vec![8.; degree + 1]].concat(),
        control_points: (0..2)
            .map(|i| {
                (0..=degree)
                    .map(|j| {
                        let seam = if j == 0 || j == degree { 1. } else { 1.5 };
                        let x = if left {
                            if i == 0 { 0. } else { seam }
                        } else if i == 0 {
                            seam
                        } else {
                            2.
                        };
                        vec![x, j as f64 / degree as f64, 0.]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![(0..=degree).map(|j| lambda.powi(j as i32)).collect(); 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn pair(degree: usize, lambda: f64) -> Vec<Surface> {
    let mut b = patch(false, degree, lambda);
    b.knots_v = [vec![3.; degree + 1], vec![11.; degree + 1]].concat();
    vec![patch(true, degree, 1.), b]
}
#[test]
fn whole_curved_seams_with_different_positive_mobius_clocks_keep_original_cap_carriers() {
    for lambda in [2., 0.5] {
        let s = pair(2, lambda);
        let saved = s.clone();
        let r =
            planar_network::thicken(&s, &[false, false], 0.25, 1e-7, Limits::default()).unwrap();
        assert!(r.cuts().is_empty());
        assert_eq!(r.exposed_boundaries().len(), 6);
        assert_eq!(r.model().faces.len(), 10);
        for i in 0..2 {
            assert_eq!(r.model().faces[i].surface, s[i]);
        }
        assert!(
            r.model().loops.iter().flat_map(|l| &l.coedges).any(|c| c
                .pcurve
                .weights
                .windows(2)
                .any(|w| w[0] != w[1]))
        );
        assert!((r.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-6);
        assert!(r.reverify().unwrap().trim_regions_valid);
        #[cfg(feature = "codec")]
        {
            assert_eq!(
                brep_core::Model::from_value(r.model().to_value()).unwrap(),
                *r.model()
            );
        }
        assert_eq!(s, saved);
    }
}
#[test]
fn cubic_projective_clocks_with_transposed_charts_and_rounded_uv_weights_are_requalified() {
    let mut s = pair(3, 3.);
    s[1] = nurbs_core::surface_edit::transpose(&s[1]).unwrap();
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false, true], -0.25, 1e-7, Limits::default()).unwrap();
    assert!(r.cuts().is_empty());
    for i in 0..2 {
        assert_eq!(r.model().faces[i].surface, s[i]);
    }
    assert!(
        r.model().loops.iter().flat_map(|l| &l.coedges).any(|c| c
            .pcurve
            .weights
            .iter()
            .any(|w| *w == 1. / 3.))
    );
    assert!((r.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-5);
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn partial_curved_seams_with_independent_neighbor_mobius_clocks_retain_the_original_t_witness() {
    let main = patch(true, 2, 1.);
    let right = patch(false, 2, 1.);
    let mut a = right.trim([2., 5., -4., 2.]).unwrap();
    let mut b = right.trim([2., 5., 2., 8.]).unwrap();
    for row in &mut a.weights {
        for (j, w) in row.iter_mut().enumerate() {
            *w *= 2f64.powi(j as i32);
        }
    }
    for row in &mut b.weights {
        for (j, w) in row.iter_mut().enumerate() {
            *w *= 0.5f64.powi(j as i32);
        }
    }
    let s = vec![main, a, b];
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.cuts().len(), 1);
    assert_eq!(r.model().loops[r.model().faces[0].outer].coedges.len(), 5);
    for i in 0..3 {
        assert_eq!(r.model().faces[i].surface, s[i]);
    }
    assert!((r.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-6);
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert_eq!(s, saved);
}
#[test]
fn coincident_curved_poles_with_nonprojective_weights_do_not_authorize_a_shared_seam() {
    let mut s = pair(2, 1.);
    s[1].weights = vec![vec![1., 2., 3.]; 2];
    let saved = s.clone();
    let e = planar_network::thicken(&s, &[false, false], 0.25, 1e-7, Limits::default())
        .err()
        .unwrap();
    assert!(e.to_string().contains("connected"), "{e}");
    assert_eq!(s, saved);
}
#[test]
fn uniform_weight_gauges_still_use_the_exact_full_boundary_geometry() {
    let mut s = pair(2, 1.);
    s[1].weights = vec![vec![2.; 3]; 2];
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false, false], 0.25, 1e-7, Limits::default()).unwrap();
    assert!(r.cuts().is_empty());
    assert!(
        r.model().loops.iter().flat_map(|l| &l.coedges).all(|c| c
            .pcurve
            .weights
            .iter()
            .all(|w| *w == c.pcurve.weights[0]))
    );
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn a_nonpolynomial_rational_seam_with_projective_neighbor_clock_allows_later_native_splits() {
    let mut s = pair(2, 1.);
    s[0].weights = vec![vec![1., 0.75, 1.]; 2];
    s[1].weights = vec![vec![1., 1.5, 4.]; 2];
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false, false], 0.25, 1e-7, Limits::default()).unwrap();
    let left = &r.model().loops[r.model().faces[0].outer].coedges;
    let right = &r.model().loops[r.model().faces[1].outer].coedges;
    let edge = left
        .iter()
        .find(|c| right.iter().any(|other| other.edge == c.edge))
        .unwrap()
        .edge;
    let split = brep_core::feature_ops::edge_partition::split(
        r.model(),
        edge,
        &[2.],
        "after-projective-sew",
        brep_core::feature_ops::edge_partition::Limits::default(),
    )
    .unwrap();
    assert!(split.reverify().unwrap().trim_regions_valid);
    assert!((split.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-5);
    assert_eq!(s, saved);
}
