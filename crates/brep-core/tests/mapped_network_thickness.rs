use brep_core::shell_thickness::mapped_network::{Limits, OffsetSettings, thicken_with_caps};
use nurbs_core::{
    surface::{Axis, Surface},
    surface_edit,
};
fn cylinder() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 0., 1.]],
            vec![vec![1., 1., 0.], vec![1., 1., 1.]],
            vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        ],
        weights: vec![
            vec![1.; 2],
            vec![std::f64::consts::FRAC_1_SQRT_2; 2],
            vec![1.; 2],
        ],
        periodic_u: false,
        periodic_v: false,
    }
}
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 1., 0.], vec![0., 1., 1.]],
            vec![vec![0., 2., 0.], vec![0., 2., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn candidates() -> (Vec<Surface>, Vec<Surface>, Vec<[[f64; 2]; 2]>) {
    let sources = vec![cylinder(), plane()];
    let mut radial = sources[0].clone();
    for p in radial.control_points.iter_mut().flatten() {
        p[0] *= 1.1;
        p[1] *= 1.1;
    }
    // Numerical contact station is only a proposal. Whole cap/UV/closed-model
    // proofs below must independently admit or refuse the actual stored data.
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..60 {
        let t = (lo + hi) * 0.5;
        if radial.evaluate(t, 0.).unwrap().point[0] > 0.1 {
            lo = t;
        } else {
            hi = t;
        }
    }
    let end = (lo + hi) * 0.5;
    let arc = radial.trim([0., end, 0., 1.]).unwrap();
    let mut wall = plane();
    wall.control_points[0] = arc.control_points.last().unwrap().clone();
    let x = wall.control_points[0][0][0];
    for p in &mut wall.control_points[1] {
        p[0] = x;
    }
    let start = wall.control_points[0][0][1] - 1.;
    (
        sources,
        vec![arc, wall],
        vec![[[0., end], [0., 1.]], [[start, 1.], [0., 1.]]],
    )
}
#[test]
fn cylinder_plane_offset_intersection_keeps_originals_and_closes_without_internal_walls() {
    let (sources, caps, maps) = candidates();
    let saved = sources.clone();
    let r = thicken_with_caps(
        &sources,
        &caps,
        &maps,
        &[false; 2],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 0.01,
            normal_cells: 100000,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
    )
    .unwrap();
    assert_ne!(caps[0].knots_u, sources[0].knots_u);
    assert_ne!(caps[0].weights, sources[0].weights);
    assert_eq!(r.model().faces.len(), 10);
    assert_eq!(r.exposed_boundaries().len(), 6);
    assert!(r.mapped_normal_offset_error_upper() <= 0.01);
    assert_eq!(r.source_windows(), maps);
    assert_eq!(r.cap_patches(), caps);
    for i in 0..2 {
        assert_eq!(r.model().faces[i].surface, sources[i]);
    }
    assert!(r.evidence().charts_injective && r.evidence().boundary_complete);
    assert!(
        r.model().loops.iter().flat_map(|w| &w.coedges).any(|c| c
            .pcurve
            .weights
            .windows(2)
            .any(|w| w[0] != w[1]))
    );
    assert_eq!(r.model().validate().unwrap().boundary_edge_count, 0);
    assert!(r.reverify().unwrap().audit.ok);
    assert_eq!(sources, saved);
    #[cfg(feature = "codec")]
    {
        use value_codec::{Deserialize, Serialize};
        assert_eq!(
            brep_core::Model::from_value(r.model().to_value()).unwrap(),
            *r.model()
        );
    }
}
#[test]
fn changed_cap_degree_uses_native_aligned_ruled_walls() {
    let source = plane();
    let mut cap = surface_edit::elevate(&source, Axis::U, 2).unwrap();
    for p in cap.control_points.iter_mut().flatten() {
        p[0] += 0.1;
    }
    let r = thicken_with_caps(
        &[source],
        &[cap],
        &[[[0., 1.], [0., 1.]]],
        &[false],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 1e-7,
            normal_cells: 4096,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(r.model().faces.len(), 6);
    assert!(r.reverify().unwrap().audit.ok);
}
#[test]
fn incorrect_maps_gapped_intersections_and_budget_exhaustion_refuse() {
    let (sources, caps, maps) = candidates();
    assert!(
        thicken_with_caps(
            &sources,
            &caps,
            &maps,
            &[false; 2],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.01,
                normal_cells: 1,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    let mut bad = maps.clone();
    bad[1][0][0] = 0.;
    assert!(
        thicken_with_caps(
            &sources,
            &caps,
            &bad,
            &[false; 2],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.01,
                normal_cells: 100000,
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
            &maps,
            &[false; 2],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.01,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
}

#[test]
fn native_caps_from_contact_windows_receive_fresh_full_geometry_admission() {
    use brep_core::shell_thickness::mapped_network::thicken_from_windows;
    let (sources, _, maps) = candidates();
    let r = thicken_from_windows(
        &sources,
        &maps,
        &[false; 2],
        0.1,
        0.01,
        100000,
        1e-7,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(r.model().faces.len(), 10);
    assert!(r.mapped_normal_offset_error_upper() <= 0.01);
    assert!(r.reverify().unwrap().audit.ok);
    let mut wrong = maps.clone();
    wrong[1][0][0] = 0.;
    assert!(
        thicken_from_windows(
            &sources,
            &wrong,
            &[false; 2],
            0.1,
            0.01,
            100000,
            1e-7,
            Limits::default()
        )
        .is_err()
    );
}
