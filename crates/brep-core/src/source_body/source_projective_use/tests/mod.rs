use super::*;
use crate::source_boundary_fragment::{Endpoint, Role};
use nurbs_core::surface::Surface;
#[test]
fn original_root_endpoint_is_retained_under_projective_mapping() {
    let surface = Surface {
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
    };
    let world = Curve {
        degree: 2,
        knots: vec![-5., -5., -5., 3., 3., 3.],
        control_points: vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![1., 1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let pc = Curve {
        degree: 2,
        knots: vec![10., 10., 10., 18., 18., 18.],
        control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]],
        weights: vec![1., 2., 4.],
        periodic: false,
    };
    let cutter = Curve::from_polyline(vec![vec![0.5, 0.], vec![0.5, 1.]]).unwrap();
    let root = crate::source_contact_point::qualify(
        &surface,
        &pc,
        &cutter,
        [[12.5, 12.8], [0.2, 0.3]],
        10000,
    )
    .unwrap();
    assert!(root.point.is_some(), "{}", root.reason);
    let fragment = Fragment::new(
        &surface,
        &pc,
        Endpoint::Parameter(10.),
        Endpoint::Crossing {
            point: root.point.unwrap(),
            role: Role::Boundary,
        },
    )
    .unwrap();
    let range = [[0., 1.], [1., 1.]];
    let mapped = qualify(&world, &fragment, range, [1., 2.], 100000000)
        .unwrap()
        .mapped
        .unwrap();
    assert_eq!(mapped.world(), &world);
    assert_eq!(mapped.fragment().curve(), &pc);
    assert!(matches!(
        mapped.fragment().endpoints()[1],
        Endpoint::Crossing { .. }
    ));
    let b = mapped.parameter_bounds().unwrap();
    assert!(b[0][0] <= -5. && -5. <= b[0][1]);
    assert!(b[1][0] <= -1. && -1. <= b[1][1]);
    assert!(!mapped.reversed());
    let q = mapped.map_normalized(I::point(14.)).unwrap();
    assert!(q.lo <= 2. / 3. && 2. / 3. <= q.hi);
    let back = mapped.inverse_normalized_unclipped(q).unwrap();
    assert!(back.lo <= 14. && 14. <= back.hi);
    let outside = mapped.inverse_normalized_unclipped(I::point(-0.1)).unwrap();
    assert!(
        outside.hi < 10.,
        "Unclipped inverse must expose unsupported tails"
    );

    let definition = mapped.definition();
    let restored = restore(definition.clone(), 10000, 100000000)
        .unwrap()
        .mapped
        .unwrap();
    assert_eq!(restored.definition(), definition);
    assert_eq!(
        restored.parameter_bounds().unwrap(),
        mapped.parameter_bounds().unwrap()
    );
    assert!(matches!(
        restored.fragment().endpoints()[1],
        Endpoint::Crossing { .. }
    ));
    let mut forged = definition.clone();
    forged["weights"] = json!([1., 1.]);
    assert!(restore(forged, 10000, 100000000).unwrap().mapped.is_none());
    let mut forged = definition.clone();
    forged["range"] = json!([[0., 1.], [3., 4.]]);
    assert!(restore(forged, 10000, 100000000).unwrap().mapped.is_none());
    let mut forged = definition.clone();
    forged["version"] = json!(2);
    assert!(restore(forged, 10000, 100000000).is_err());
    assert!(restore(definition.clone(), 0, 100000000).is_err());
    assert!(
        restore(definition.clone(), 10000, 1)
            .unwrap()
            .mapped
            .is_none()
    );

    assert!(
        same_parameter_endpoint([&mapped, &mapped], [0, 0], [None; 2], 100000000)
            .unwrap()
            .0
    );
    let mut mixed_pc = pc.clone();
    mixed_pc.weights = vec![1., 3., 9.];
    let mixed_root = crate::source_contact_point::qualify(
        &surface,
        &mixed_pc,
        &cutter,
        [[11.9, 12.1], [0.2, 0.3]],
        10000,
    )
    .unwrap()
    .point
    .unwrap();
    let mixed_fragment = Fragment::new(
        &surface,
        &mixed_pc,
        Endpoint::Parameter(10.),
        Endpoint::Crossing {
            point: mixed_root,
            role: Role::Boundary,
        },
    )
    .unwrap();
    let mixed = qualify(&world, &mixed_fragment, range, [1., 3.], 100000000)
        .unwrap()
        .mapped
        .unwrap();
    let mut fixed_pc = pc.clone();
    fixed_pc.weights = vec![1.; 3];
    let fixed_fragment = Fragment::new(
        &surface,
        &fixed_pc,
        Endpoint::Parameter(14.),
        Endpoint::Parameter(10.),
    )
    .unwrap();
    let fixed = qualify(&world, &fixed_fragment, range, [1., 1.], 100000000)
        .unwrap()
        .mapped
        .unwrap();
    assert!(
        same_parameter_endpoint(
            [&mixed, &fixed],
            [1, 0],
            [Some([12., 0.25]), None],
            100000000
        )
        .unwrap()
        .0
    );
    assert_eq!(
        same_parameter_endpoint_counted(
            [&mixed, &fixed],
            [1, 0],
            [Some([12., 0.25]), None],
            100000000
        )
        .unwrap()
        .2,
        1
    );
    assert!(
        !same_parameter_endpoint(
            [&mixed, &fixed],
            [1, 0],
            [Some([12.01, 0.25]), None],
            100000000
        )
        .unwrap()
        .0
    );
    assert!(
        !same_parameter_endpoint([&mixed, &fixed], [1, 0], [Some([12., 0.25]), None], 1)
            .unwrap()
            .0
    );

    let mut other_surface = surface.clone();
    for row in &mut other_surface.control_points {
        for point in row {
            point.swap(0, 1);
        }
    }
    let mut other_pc = pc.clone();
    for point in &mut other_pc.control_points {
        point.swap(0, 1);
    }
    let other_cutter = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
    let other_root = crate::source_contact_point::qualify(
        &other_surface,
        &other_pc,
        &other_cutter,
        [[12.5, 12.8], [0.2, 0.3]],
        10000,
    )
    .unwrap()
    .point
    .unwrap();
    let other_fragment = Fragment::new(
        &other_surface,
        &other_pc,
        Endpoint::Crossing {
            point: other_root,
            role: Role::Boundary,
        },
        Endpoint::Parameter(10.),
    )
    .unwrap();
    let other = qualify(&world, &other_fragment, range, [1., 2.], 100000000)
        .unwrap()
        .mapped
        .unwrap();
    assert!(other.reversed());
    let plane = [[0.5, 0., 0.], [0.5, 1., 0.], [0.5, 0., 1.]];
    let edge_report = crate::source_projective_edge::qualify(
        &world,
        [&fragment, &other_fragment],
        [range; 2],
        [[1., 2.]; 2],
        [None, Some(plane)],
        [[None; 2]; 2],
        100000000,
        10000,
    )
    .unwrap();
    let edge = edge_report.edge.unwrap();
    let restriction = crate::source_edge_restriction::Restriction::from_edge(&edge);
    let bounds = restriction.parameter_bounds().unwrap();
    assert!(bounds[0][0] <= -5. && -5. <= bounds[0][1]);
    assert!(bounds[1][0] <= -1. && -1. <= bounds[1][1]);
    let boxes = restriction.endpoint_boxes(100).unwrap();
    for (a, value) in [0.5, 0.25, 0.].into_iter().enumerate() {
        assert!(boxes[1][a][0] <= value && value <= boxes[1][a][1]);
    }
    let bytes = value_codec::encode_binary(&restriction.definition()).unwrap();
    let restriction_replay = crate::source_edge_restriction::restore_projective(
        value_codec::decode_binary(&bytes).unwrap(),
        crate::source_shared_edge_restore::Limits {
            mapping_cells_per_use: 10000,
            exact_work: 100000000,
            driver_cells: 10000,
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(restriction_replay.definition(), restriction.definition());
    assert_eq!(restriction_replay.parameter_bounds().unwrap(), bounds);
    assert_eq!(restriction_replay.endpoint_boxes(100).unwrap(), boxes);
    assert_eq!(
        restriction.display_segments(16).unwrap(),
        restriction_replay.display_segments(16).unwrap()
    );
    assert!(restriction.endpoint_boxes(0).is_err());
    assert!(restriction.display_segments(0).is_err());

    let seam_limits = || crate::source_seam_tangency::Limits {
        max_sine_squared: 1e-3,
        cells: 10000,
        curve_spans: 20000,
        normal_spans: 20000,
    };
    let seam = crate::source_seam_tangency::qualify(&edge, seam_limits()).unwrap();
    assert!(seam.seam.is_some(), "{}", seam.reason);
    assert!(seam.seam.unwrap().sine_squared_bounds()[1] <= 1e-3);
    let synchronized = crate::source_seam_tangency::qualify_with_synchronized_boundaries(
        &edge,
        seam_limits(),
        100000,
    )
    .unwrap();
    assert_eq!(
        synchronized.synchronized_reason,
        "synchronized-boundary-unsupported"
    );
    assert!(synchronized.polynomial_work <= 100000);
    assert!(synchronized.report.seam.is_some());
    assert_eq!(
        synchronized.report.seam.unwrap().edge().definition(),
        edge.definition()
    );
    // The same parabola lies on z=v-u^2, whose tangent plane differs
    // from z=0 everywhere on this retained interval.
    let oblique_surface = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|u| {
                (0..2)
                    .map(|v| {
                        vec![
                            u as f64 * 0.5,
                            v as f64,
                            v as f64 - if u == 2 { 1. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let oblique_root = crate::source_contact_point::qualify(
        &oblique_surface,
        &pc,
        &cutter,
        [[12.5, 12.8], [0.2, 0.3]],
        10000,
    )
    .unwrap()
    .point
    .unwrap();
    let oblique_fragment = Fragment::new(
        &oblique_surface,
        &pc,
        Endpoint::Crossing {
            point: oblique_root,
            role: Role::Boundary,
        },
        Endpoint::Parameter(10.),
    )
    .unwrap();
    let oblique = crate::source_projective_edge::qualify(
        &world,
        [&fragment, &oblique_fragment],
        [range; 2],
        [[1., 2.]; 2],
        [None, Some(plane)],
        [[None; 2]; 2],
        100000000,
        10000,
    )
    .unwrap()
    .edge
    .unwrap();
    let failed = crate::source_seam_tangency::qualify(&oblique, seam_limits()).unwrap();
    assert!(failed.seam.is_none());
    assert_eq!(failed.reason, "source-seam-angular-envelope-oblique");
    let synchronized_oblique =
        crate::source_seam_tangency::qualify_with_synchronized_boundaries(
            &oblique,
            seam_limits(),
            100000,
        )
        .unwrap();
    assert!(synchronized_oblique.report.seam.is_none());
    assert_eq!(
        synchronized_oblique.report.reason,
        "source-seam-angular-envelope-oblique"
    );
    let endpoint = edge.sample_endpoint(1, 100).unwrap().endpoint.unwrap();
    assert!(endpoint.canonical_bounds()[0] <= 0.5 && 0.5 <= endpoint.canonical_bounds()[1]);
    let sample = edge.sample_interior(0.25, 100).unwrap().sample.unwrap();
    assert!(sample.world_bounds()[0][0] <= 0.25 && 0.25 <= sample.world_bounds()[0][1]);
    for (side, expected) in [[0.25, 0.0625], [0.0625, 0.25]].iter().enumerate() {
        for axis in 0..2 {
            let b = sample.uv_bounds()[side][axis];
            assert!(b[0] <= expected[axis] && expected[axis] <= b[1]);
            assert!(
                b[1] - b[0] < 1e-10,
                "Interior UV enclosure should resolve the rational inverse"
            );
        }
    }

    let chain = edge
        .sample_interior_chain(&[0., 0.125, 0.25, 0.375, 0.5, 0.75], 1000)
        .unwrap();
    assert_eq!(chain.points.len(), 6);
    assert!(chain.pending >= 3);
    assert!(edge.sample_interior(0.25, 1).unwrap().sample.is_none());
    let mesh = edge
        .prepare_boundary_mesh(crate::source_shared_edge::BoundaryMeshLimits {
            max_deviation_mm: 0.1,
            cells: 1000,
            spans: 20000,
            max_depth: 12,
        })
        .unwrap();
    assert!(mesh.pending.is_empty());
    assert!(!mesh.segments.is_empty());
    assert!(
        mesh.segments
            .iter()
            .all(|segment| segment.deviation_upper_mm <= 0.1)
    );
    let saved = edge.definition();
    let text = value_codec::to_string(&saved).unwrap();
    let decoded: Value = value_codec::from_str_strict(&text).unwrap();
    assert_eq!(decoded, saved);
    let bytes = value_codec::encode_binary(&saved).unwrap();
    assert_eq!(value_codec::decode_binary(&bytes).unwrap(), saved);
    let replay = crate::source_projective_edge::restore(decoded, 10000, 100000000, 10000)
        .unwrap()
        .edge
        .unwrap();
    assert_eq!(replay.definition(), saved);
    assert_eq!(replay.world(), &world);
    assert!(replay.uses()[0].reversed() != replay.uses()[1].reversed());
    let mut wrong = saved.clone();
    wrong["uses"][1]["weights"] = json!([1., 1.]);
    assert!(
        crate::source_projective_edge::restore(wrong, 10000, 100000000, 10000)
            .unwrap()
            .edge
            .is_none()
    );
    let mut wrong = saved.clone();
    wrong["planes"][1] = json!([[0.6, 0., 0.], [0.6, 1., 0.], [0.6, 0., 1.]]);
    assert!(
        crate::source_projective_edge::restore(wrong, 10000, 100000000, 10000)
            .unwrap()
            .edge
            .is_none()
    );
    assert!(
        crate::source_projective_edge::restore(saved.clone(), 10000, 1, 10000)
            .unwrap()
            .edge
            .is_none()
    );
    assert!(
        crate::source_projective_edge::restore(saved, 10000, 100000000, 0)
            .unwrap()
            .edge
            .is_none()
    );
    let (same, report) =
        same_crossing_endpoint([&mapped, &other], [1, 0], plane, 100000000, 10000).unwrap();
    assert!(same, "{}", report.reason);
    assert!(
        !same_crossing_endpoint(
            [&mapped, &other],
            [1, 0],
            [[0.6, 0., 0.], [0.6, 1., 0.], [0.6, 0., 1.]],
            100000000,
            10000
        )
        .unwrap()
        .0
    );
    assert!(
        !same_crossing_endpoint([&mapped, &other], [1, 0], plane, 1, 10000)
            .unwrap()
            .0
    );

    assert!(
        qualify(&world, &fragment, range, [1., 1.], 100000000)
            .unwrap()
            .mapped
            .is_none()
    );
    assert!(
        qualify(&world, &fragment, range, [1., 2.], 1)
            .unwrap()
            .mapped
            .is_none()
    );
}
