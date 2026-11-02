use brep_core::shell_thickness::planar_network::{self, Limits};
use nurbs_core::surface::Surface;
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize};
fn patch(x: f64, y: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![x, y, 0.], vec![x, y + 1., 0.]],
            vec![vec![x + 1., y, 0.], vec![x + 1., y + 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn two_original_patches_keep_their_internal_seam_and_receive_no_internal_wall() {
    let s = vec![patch(0., 0.), patch(1., 0.)];
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false, false], 0.25, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.model().faces.len(), 10);
    assert_eq!(r.exposed_boundaries().len(), 6);
    for i in 0..2 {
        assert_eq!(r.model().faces[i].surface, s[i]);
    }
    assert!(r.normal_offset_error_upper() <= 1e-7);
    assert!(r.evidence().audit.ok);
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert!((r.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-6);
    #[cfg(feature = "codec")]
    {
        assert_eq!(
            brep_core::Model::from_value(r.model().to_value()).unwrap(),
            *r.model()
        );
    }
    assert_eq!(s, saved);
}
#[test]
fn a_connected_patch_ring_retains_its_through_hole_and_both_thickness_directions() {
    let s = (0..3)
        .flat_map(|x| {
            (0..3).filter_map(move |y| (x != 1 || y != 1).then(|| patch(x as f64, y as f64)))
        })
        .collect::<Vec<_>>();
    let saved = s.clone();
    for thickness in [0.25, -0.25] {
        let r =
            planar_network::thicken(&s, &[false; 8], thickness, 1e-7, Limits::default()).unwrap();
        assert_eq!(r.exposed_boundaries().len(), 16);
        assert_eq!(r.model().faces.len(), 32);
        assert_eq!(r.model().shells.len(), 1);
        assert_eq!(r.model().bodies.len(), 1);
        assert!(r.evidence().audit.ok);
        assert!((r.evidence().numerical_volume_mm3 - 2.).abs() < 1e-6);
        assert!(r.reverify().unwrap().boundary_complete);
    }
    assert_eq!(s, saved);
}
#[test]
fn independently_parameterized_and_transposed_sources_share_the_actual_complete_boundary() {
    let mut a = patch(0., 0.);
    a.knots_u = vec![2., 2., 5., 5.];
    a.knots_v = vec![-3., -3., 7., 7.];
    let b = nurbs_core::surface_edit::transpose(&patch(1., 0.)).unwrap();
    let s = vec![a, b];
    let r = planar_network::thicken(&s, &[false, true], 0.5, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.exposed_boundaries().len(), 6);
    assert_eq!(r.model().faces[0].surface, s[0]);
    assert_eq!(r.model().faces[1].surface, s[1]);
    assert!((r.evidence().numerical_volume_mm3 - 1.).abs() < 1e-6);
    assert!(r.reverify().unwrap().trim_regions_valid);
}
#[test]
fn a_curved_original_internal_seam_survives_while_only_exposed_boundaries_are_walled() {
    let p = |left: bool| Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..2)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let seam = [1., 1.5, 1.][j];
                        let x = if left {
                            if i == 0 { 0. } else { seam }
                        } else if i == 0 {
                            seam
                        } else {
                            2.
                        };
                        vec![x, j as f64 / 2., 0.]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let s = vec![p(true), p(false)];
    let r = planar_network::thicken(&s, &[false, false], 0.25, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.exposed_boundaries().len(), 6);
    assert_eq!(r.model().faces.len(), 10);
    assert_eq!(r.model().faces[0].surface, s[0]);
    assert!((r.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-6);
    assert!(r.reverify().unwrap().boundary_complete);
}
#[test]
fn an_oblique_original_plane_has_continuous_rounding_inclusive_normal_offset_bounds() {
    let mut s = vec![patch(0., 0.), patch(1., 0.)];
    for surface in &mut s {
        for p in surface.control_points.iter_mut().flatten() {
            let [x, y, z] = [p[0], p[1], p[2]];
            p.copy_from_slice(&[x + z + 5., y + z - 3., x + y - z + 2.]);
        }
    }
    let saved = s.clone();
    for thickness in [0.25, -0.25] {
        let r = planar_network::thicken(&s, &[false, false], thickness, 1e-7, Limits::default())
            .unwrap();
        assert!(r.normal_offset_error_upper() > 0. && r.normal_offset_error_upper() < 1e-7);
        assert!((r.evidence().numerical_volume_mm3 - 0.5 * 3f64.sqrt()).abs() < 1e-6);
        let bounds = r.cap_distance_bounds();
        assert!(
            bounds[0] > 0.
                && bounds[0] <= 0.25
                && bounds[1] >= 0.25
                && bounds[1] - bounds[0] < 1e-7
        );
        assert!(r.reverify().unwrap().boundary_complete);
    }
    assert_eq!(s, saved);
}
#[test]
fn positive_rational_source_weights_remain_original_and_shared_on_both_skins() {
    let mut s = vec![patch(0., 0.), patch(1., 0.)];
    s[0].weights = vec![vec![1., 1.25], vec![1.5, 1.75]];
    s[1].weights = vec![vec![1.5, 1.75], vec![1.25, 1.]];
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false, false], 0.25, 1e-7, Limits::default()).unwrap();
    for i in 0..2 {
        assert_eq!(r.model().faces[i].surface, s[i]);
        assert_eq!(r.model().faces[i + 2].surface.weights, s[i].weights);
    }
    assert!((r.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-5);
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn bent_incoherent_disconnected_nonmanifold_and_exhausted_inputs_cannot_publish_a_skin() {
    let s = vec![patch(0., 0.), patch(1., 0.)];
    let saved = s.clone();
    let mut bent = s.clone();
    bent[1].control_points[1][1][2] = 1e-12;
    assert!(
        planar_network::thicken(&bent, &[false, false], 0.25, 1e-7, Limits::default())
            .err()
            .unwrap()
            .to_string()
            .contains("exact plane")
    );
    assert!(
        planar_network::thicken(&s, &[false, true], 0.25, 1e-7, Limits::default())
            .err()
            .unwrap()
            .to_string()
            .contains("coherently oriented")
    );
    assert!(
        planar_network::thicken(
            &[patch(0., 0.), patch(3., 0.)],
            &[false, false],
            0.25,
            1e-7,
            Limits::default()
        )
        .err()
        .unwrap()
        .to_string()
        .contains("connected")
    );
    assert!(
        planar_network::thicken(
            &[s[0].clone(), s[1].clone(), s[1].clone()],
            &[false; 3],
            0.25,
            1e-7,
            Limits::default()
        )
        .is_err()
    );
    assert!(
        planar_network::thicken(
            &s,
            &[false, false],
            0.25,
            1e-7,
            Limits {
                exact_work: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let limits = Limits {
        proof: brep_core::feature_ops::region_pattern::Limits {
            max_cells: 1,
            ..Limits::default().proof
        },
        ..Limits::default()
    };
    assert!(planar_network::thicken(&s, &[false, false], 0.25, 1e-7, limits).is_err());
    assert!(planar_network::thicken(&s, &[false, false], 0., 1e-7, Limits::default()).is_err());
    assert_eq!(s, saved);
}

#[test]
fn rounded_zero_displacement_cannot_pass_as_positive_cap_thickness() {
    let mut s = patch(0., 0.);
    for p in s.control_points.iter_mut().flatten() {
        p[2] = 1e6;
    }
    let saved = s.clone();
    let e = planar_network::thicken(&[s.clone()], &[false], 1e-12, 1e-7, Limits::default())
        .err()
        .unwrap();
    assert!(e.to_string().contains("cap separation"), "{e}");
    assert_eq!(s, saved);
}
