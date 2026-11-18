use brep_core::shell_thickness::{freeform_network::thicken_with_caps, planar_network::{Limits, OffsetSettings}};
use nurbs_core::surface::Surface;
fn patch(x: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![x, 0., 0.], vec![x, 1., 0.1 * x]],
            vec![vec![x + 1., 0., 0.], vec![x + 1., 1., 0.1 * (x + 1.)]],
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
fn nonplanar_network_keeps_shared_seams_and_original_caps() {
    for d in [-0.1, 0.1] {
        let sources = vec![patch(0.), patch(1.)];
        let caps = sources.iter().map(|s| cap(s, d)).collect::<Vec<_>>();
        let r = thicken_with_caps(
            &sources,
            &caps,
            &[false, false],
            OffsetSettings {
                signed_thickness: d,
                normal_tolerance: 0.05,
                normal_cells: 32768,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .unwrap();
        assert_eq!(r.model().faces.len(), 10);
        assert_eq!(r.exposed_boundaries().len(), 6);
        assert_eq!(r.normal_certificates().len(), 2);
        assert!(r.normal_cells() <= 32768);
        assert!(r.normal_offset_error_upper() <= 0.05);
        assert!(r.evidence().audit.ok);
        assert_eq!(r.model().validate().unwrap().boundary_edge_count, 0);
        for i in 0..2 {
            assert_eq!(r.model().faces[i].surface, sources[i]);
            assert_eq!(r.model().faces[i + 2].surface, caps[i]);
        }
        assert!(r.reverify().unwrap().boundary_complete);
    }
}
#[test]
fn reversed_source_chart_uses_oriented_normal() {
    let a = patch(0.);
    let mut b = patch(1.);
    for row in &mut b.control_points {
        row.reverse();
    }
    for row in &mut b.weights {
        row.reverse();
    }
    let sources = vec![a, b];
    let caps = sources.iter().map(|s| cap(s, 0.1)).collect::<Vec<_>>();
    assert!(
        thicken_with_caps(
            &sources,
            &caps,
            &[false, true],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.05,
                normal_cells: 32768,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_ok()
    );
    assert!(
        thicken_with_caps(
            &sources,
            &caps,
            &[false, false],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.05,
                normal_cells: 32768,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
}
#[test]
fn invalid_caps_and_networks_do_not_produce_bodies() {
    let sources = vec![patch(0.), patch(1.)];
    let caps = sources.iter().map(|s| cap(s, 0.1)).collect::<Vec<_>>();
    assert!(
        thicken_with_caps(
            &sources,
            &caps,
            &[false, false],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.05,
                normal_cells: 1,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    let mut broken = caps.clone();
    for p in broken[1].control_points.iter_mut().flatten() {
        p[0] += 0.001;
    }
    assert!(
        thicken_with_caps(
            &sources,
            &broken,
            &[false, false],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.05,
                normal_cells: 32768,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    let disconnected = vec![patch(0.), patch(3.)];
    let c = disconnected.iter().map(|s| cap(s, 0.1)).collect::<Vec<_>>();
    assert!(
        thicken_with_caps(
            &disconnected,
            &c,
            &[false, false],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.05,
                normal_cells: 32768,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
}

#[test]
fn nonplanar_t_network_partitions_original_seam_without_an_internal_wall() {
    let mut left = patch(0.);
    for p in left.control_points.iter_mut().flatten() {
        p[1] *= 2.;
        p[2] *= 2.;
    }
    let lower = patch(1.);
    let mut upper = patch(1.);
    for p in upper.control_points.iter_mut().flatten() {
        p[1] += 1.;
        p[2] += 0.1 * p[0];
    }
    let sources = vec![left, lower, upper];
    let caps = sources.iter().map(|s| cap(s, 0.1)).collect::<Vec<_>>();
    let r = thicken_with_caps(
        &sources,
        &caps,
        &[false; 3],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 0.08,
            normal_cells: 32768,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
    )
    .unwrap();
    assert!(!r.cuts().is_empty());
    assert_eq!(r.exposed_boundaries().len(), 7);
    assert_eq!(r.model().faces.len(), 13);
    assert!(r.reverify().unwrap().audit.ok);
}

#[test]
fn rational_cylindrical_network_has_continuously_verified_radial_caps() {
    let cylinder = |z: f64| Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., z], vec![1., 0., z + 1.]],
            vec![vec![1., 1., z], vec![1., 1., z + 1.]],
            vec![vec![0., 1., z], vec![0., 1., z + 1.]],
        ],
        weights: vec![
            vec![1.; 2],
            vec![std::f64::consts::FRAC_1_SQRT_2; 2],
            vec![1.; 2],
        ],
        periodic_u: false,
        periodic_v: false,
    };
    let sources = vec![cylinder(0.), cylinder(1.)];
    let mut caps = sources.clone();
    for p in caps
        .iter_mut()
        .flat_map(|s| s.control_points.iter_mut().flatten())
    {
        p[0] *= 1.1;
        p[1] *= 1.1;
    }
    let r = thicken_with_caps(
        &sources,
        &caps,
        &[false; 2],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 0.01,
            normal_cells: 32768,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(r.model().faces.len(), 10);
    assert!(r.normal_offset_error_upper() <= 0.01);
    assert!(r.evidence().charts_injective && r.evidence().boundary_complete);
    assert!((r.evidence().numerical_volume_mm3 - 0.105 * std::f64::consts::PI).abs() < 1e-5);
}

#[test]
fn automatic_native_caps_are_admitted_only_by_continuous_network_proofs() {
    use brep_core::shell_thickness::freeform_network::thicken;
    let sources = vec![patch(0.), patch(1.)];
    let saved = sources.clone();
    let r = thicken(
        &sources,
        &[false; 2],
        0.1,
        0.05,
        32768,
        1e-7,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(r.model().faces.len(), 10);
    assert_eq!(r.normal_certificates().len(), 2);
    assert!(r.normal_offset_error_upper() <= 0.05);
    assert!(r.reverify().unwrap().audit.ok);
    assert_eq!(sources, saved);
}

#[test]
fn normal_allowance_is_shared_across_patches_and_saved_model_roundtrips() {
    let mut sources = vec![patch(0.), patch(1.)];
    for p in sources
        .iter_mut()
        .flat_map(|s| s.control_points.iter_mut().flatten())
    {
        p[2] = 0.;
    }
    let caps = sources.iter().map(|s| cap(s, 0.1)).collect::<Vec<_>>();
    assert!(
        thicken_with_caps(
            &sources,
            &caps,
            &[false; 2],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 1e-8,
                normal_cells: 1,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    let r = thicken_with_caps(
        &sources,
        &caps,
        &[false; 2],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 1e-8,
            normal_cells: 2,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(r.normal_cells(), 2);
    #[cfg(feature = "codec")]
    {
        use value_codec::{Deserialize, Serialize};
        assert_eq!(
            brep_core::Model::from_value(r.model().to_value()).unwrap(),
            *r.model()
        );
    }
    let mut low = Limits::default();
    low.exact_work = 1;
    assert!(thicken_with_caps(
        &sources,
        &caps,
        &[false; 2],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 1e-8,
            normal_cells: 2,
            sewing_tolerance: 1e-7,
        },
        low,
    ).is_err());
    let mut low = Limits::default();
    low.proof.max_cells = 1;
    assert!(thicken_with_caps(
        &sources,
        &caps,
        &[false; 2],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 1e-8,
            normal_cells: 2,
            sewing_tolerance: 1e-7,
        },
        low,
    ).is_err());
}
