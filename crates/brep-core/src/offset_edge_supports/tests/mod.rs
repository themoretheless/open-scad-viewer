use super::*;
fn limits() -> Limits {
    Limits {
        pairs: 10000,
        region_cells: 100000,
        domain_cells: 100000,
        agreement_cells: 100000,
    }
}
fn lift_limits() -> nurbs_core::curve_surface_lift::Limits {
    nurbs_core::curve_surface_lift::Limits {
        proposal_cells: 2047,
        agreement_cells: 8192,
        cells_per_attempt: 1,
    }
}
fn pcurve_limits() -> nurbs_core::offset_path_pcurves::Limits {
    nurbs_core::offset_path_pcurves::Limits {
        trims: nurbs_core::offset_path_trims::Limits {
            path: nurbs_core::offset_contact_path::Limits {
                cells: 255,
                spans: 16,
                iterations: 8,
                numerical_tolerance_mm: 1e-8,
                padding_fraction: 0.01,
            },
            membership_cells: 255,
            region_pairs: 10000,
            region_cells: 10000,
            domain_cells: 100000,
        },
        proposal_cells: 1023,
        correspondence_cells: 8192,
        cells_per_side: 1,
        station_queries: 8192,
        station_refinements: 4,
        domain_cells: 100000,
    }
}
#[test]
fn all_box_edges_bind_two_original_faces_curves_and_ids_without_mutation() {
    let source = crate::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let before = source.clone();
    for edge in 0..source.edges.len() {
        let r = prepare(&source, edge, 1e-8, limits()).unwrap();
        let prepared = r.prepared.expect(r.reason);
        assert_eq!(prepared.edge_id(), source.1.edges[edge]);
        assert_eq!(prepared.curve(), &source.edges[edge].curve);
        assert_ne!(
            prepared.supports()[0].face_index(),
            prepared.supports()[1].face_index()
        );
        for support in prepared.supports() {
            let coedge = &source.loops[support.loop_index()].coedges[support.cyclic_index()];
            assert_eq!(support.selected_pcurve(), &coedge.pcurve);
            assert_eq!(support.selected_reversed(), coedge.reversed);
            assert_eq!(support.face_id(), source.1.faces[support.face_index()]);
            assert_eq!(
                support.face().face().surface,
                source.faces[support.face_index()].surface
            );
            assert_eq!(support.edge_ids().len(), support.face().edges().len());
        }
    }
    assert_eq!(source, before);
}
#[test]
fn rational_edge_keeps_original_curved_and_planar_supports() {
    let arc = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1., 0.5f64.sqrt(), 1.],
        periodic: false,
    };
    let profile = vec![
        arc,
        Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap(),
        Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap(),
    ];
    let source = crate::prism::extrude(&[profile], 0., 2.).unwrap();
    let edge = source
        .edges
        .iter()
        .position(|e| e.curve.degree == 2)
        .unwrap();
    let report = prepare(&source, edge, 1e-8, limits()).unwrap();
    let prepared = report.prepared.expect(report.reason);
    assert_eq!(prepared.curve(), &source.edges[edge].curve);
    assert!(prepared
        .supports()
        .iter()
        .any(|s| s.face().face().surface.degree_u == 2));
    assert!(prepared
        .supports()
        .iter()
        .any(|s| s.face().face().surface.degree_u == 1));
    assert!(report.support_audits.iter().all(|r| {
        r.agreements
            .iter()
            .all(|a| a.status == nurbs_core::curve_surface_agreement::Status::WithinTolerance)
    }));
    let distances = std::array::from_fn(|i| {
        if prepared.supports()[i].face_reversed() {
            0.2
        } else {
            -0.2
        }
    });
    let section = propose_section(&prepared, 0.5, distances, 1e-8, 8, 1e-3, 16).unwrap();
    assert!(section.residual_mm <= 1e-8);
    assert!(
        matches!(
            section.certificate,
            Some(nurbs_core::surface_contact::Verdict::Witness(_))
        ),
        "{}",
        section.reason
    );
    let path = propose_path(
        &prepared,
        section.fixed_axis,
        [0.2, 0.8],
        distances,
        63,
        nurbs_core::offset_contact_path::Limits {
            cells: 255,
            spans: 16,
            iterations: 8,
            numerical_tolerance_mm: 1e-8,
            padding_fraction: 0.01,
        },
    )
    .unwrap();
    assert!(path.driver.monotonic_proven);
    assert!(
        path.path.as_ref().unwrap().continuous_path_proven,
        "{}",
        path.reason
    );
    let trimmed = propose_trimmed_path(
        &prepared,
        section.fixed_axis,
        [0.2, 0.8],
        distances,
        63,
        1e-8,
        nurbs_core::offset_path_trims::Limits {
            path: nurbs_core::offset_contact_path::Limits {
                cells: 255,
                spans: 16,
                iterations: 8,
                numerical_tolerance_mm: 1e-8,
                padding_fraction: 0.01,
            },
            membership_cells: 255,
            region_pairs: 10000,
            region_cells: 10000,
            domain_cells: 100000,
        },
    )
    .unwrap();
    assert!(
        trimmed
            .contact
            .as_ref()
            .unwrap()
            .trimmed_continuous_path_proven,
        "{}",
        trimmed.reason
    );
    let pc = propose_source_pcurves(
        &prepared,
        section.fixed_axis,
        [0.2, 0.8],
        distances,
        63,
        1e-4,
        pcurve_limits(),
    )
    .unwrap();
    assert!(
        pc.pcurves.as_ref().unwrap().source_pcurves_proven,
        "{}",
        pc.reason
    );
    let world = propose_world_contacts(
        &prepared,
        section.fixed_axis,
        [0.2, 0.8],
        distances,
        63,
        1e-4,
        pcurve_limits(),
        1e-5,
        1e-3,
        lift_limits(),
    )
    .unwrap();
    assert!(
        world.world_contacts_proven,
        "{}: {:?}",
        world.reason, world.contact_error_upper_mm
    );
    assert!(world
        .lifts
        .iter()
        .all(|r| r.source_lift_proven && r.shared_endpoints_proven));
    let tight = propose_world_contacts(
        &prepared,
        section.fixed_axis,
        [0.2, 0.8],
        distances,
        63,
        1e-4,
        pcurve_limits(),
        1e-5,
        1e-10,
        lift_limits(),
    )
    .unwrap();
    assert!(!tight.world_contacts_proven);
    assert!(tight.lifts.iter().all(|r| r.source_lift_proven));
    assert_eq!(tight.reason, "source-world-contact-bound-exceeds-tolerance");
    let mut work = lift_limits();
    work.proposal_cells = 1;
    let stopped = propose_world_contacts(
        &prepared,
        section.fixed_axis,
        [0.2, 0.8],
        distances,
        63,
        1e-4,
        pcurve_limits(),
        1e-5,
        1e-3,
        work,
    )
    .unwrap();
    assert!(
        stopped
            .pcurves
            .pcurves
            .as_ref()
            .unwrap()
            .source_pcurves_proven
    );
    assert!(!stopped.world_contacts_proven && stopped.lifts.len() == 2);
    assert!(stopped.lifts.iter().any(|r| !r.source_lift_proven));
}
#[test]
fn automatic_box_edge_stations_find_certified_centers_after_rotation() {
    let base = crate::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let c = 0.6;
    let t = 0.8;
    let source = crate::transform::affine(
        &base,
        [
            [c, -t, 0., 5.],
            [t, c, 0., -3.],
            [0., 0., 1., 2.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    for edge in 0..source.edges.len() {
        let prepared = prepare(&source, edge, 1e-8, limits())
            .unwrap()
            .prepared
            .unwrap();
        let distances = std::array::from_fn(|i| {
            if prepared.supports()[i].face_reversed() {
                0.2
            } else {
                -0.2
            }
        });
        let section = propose_section(&prepared, 0.5, distances, 1e-8, 8, 1e-3, 16).unwrap();
        assert!(section.residual_mm <= 1e-8);
        assert!(
            matches!(
                section.certificate,
                Some(nurbs_core::surface_contact::Verdict::Witness(_))
            ),
            "{}",
            section.reason
        );
        assert!(section.iterations <= 8 && section.line_search_evaluations <= 96);
        let path = propose_path(
            &prepared,
            section.fixed_axis,
            [0.2, 0.8],
            distances,
            16,
            nurbs_core::offset_contact_path::Limits {
                cells: 63,
                spans: 16,
                iterations: 8,
                numerical_tolerance_mm: 1e-8,
                padding_fraction: 0.01,
            },
        )
        .unwrap();
        assert!(path.driver.monotonic_proven);
        let world = propose_world_contacts(
            &prepared,
            section.fixed_axis,
            [0.2, 0.8],
            distances,
            16,
            1e-5,
            pcurve_limits(),
            1e-7,
            1e-5,
            lift_limits(),
        )
        .unwrap();
        assert!(
            world.world_contacts_proven,
            "edge {}: {}: {:?}",
            edge, world.reason, world.contact_error_upper_mm
        );

        let pc = propose_source_pcurves(
            &prepared,
            section.fixed_axis,
            [0.2, 0.8],
            distances,
            16,
            1e-5,
            pcurve_limits(),
        )
        .unwrap();
        assert!(
            pc.pcurves.as_ref().unwrap().source_pcurves_proven,
            "edge {}: {}",
            edge,
            pc.reason
        );

        let trimmed = propose_trimmed_path(
            &prepared,
            section.fixed_axis,
            [0.2, 0.8],
            distances,
            16,
            1e-8,
            nurbs_core::offset_path_trims::Limits {
                path: nurbs_core::offset_contact_path::Limits {
                    cells: 63,
                    spans: 16,
                    iterations: 8,
                    numerical_tolerance_mm: 1e-8,
                    padding_fraction: 0.01,
                },
                membership_cells: 127,
                region_pairs: 10000,
                region_cells: 10000,
                domain_cells: 100000,
            },
        )
        .unwrap();
        assert!(
            trimmed
                .contact
                .as_ref()
                .unwrap()
                .trimmed_continuous_path_proven,
            "edge {}: {}",
            edge,
            trimmed.reason
        );

        assert!(
            path.path.as_ref().unwrap().continuous_path_proven,
            "edge {}: {}",
            edge,
            path.reason
        );
    }
}
#[test]
fn numerical_center_does_not_gain_authority_when_interval_work_stops() {
    // C2 cubic carrier with two incident spans at the driving station.
    let line = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.],
        control_points: vec![
            vec![0., 0.],
            vec![1. / 3., 0.],
            vec![1., 0.],
            vec![5. / 3., 0.],
            vec![2., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let points = [vec![2., 0.], vec![2., 3.], vec![0., 3.], vec![0., 0.]];
    let mut profile = vec![line];
    profile.extend(
        points
            .windows(2)
            .map(|p| Curve::from_polyline(p.to_vec()).unwrap()),
    );
    let mut source = crate::prism::extrude(&[profile], 0., 4.).unwrap();
    // Extrusion splits profile spans into separate side faces. Retain a
    // multispan affine cap carrier so the root tube actually crosses a knot.
    for face in &mut source.faces {
        let surface = &mut face.surface;
        let z = surface.control_points[0][0][2];
        if surface.control_points.iter().flatten().all(|p| p[2] == z) {
            surface.degree_u = 3;
            surface.knots_u = vec![0., 0., 0., 0., 0.25, 1., 1., 1., 1.];
            surface.control_points = [0., 1. / 12., 5. / 12., 0.75, 1.]
                .map(|u| vec![vec![2. * u, 0., z], vec![2. * u, 3., z]])
                .to_vec();
            surface.weights = vec![vec![1.; 2]; 5];
        }
    }
    let edge = source
        .edges
        .iter()
        .position(|e| e.curve.degree == 3)
        .unwrap();
    let prepared = prepare(&source, edge, 1e-8, limits())
        .unwrap()
        .prepared
        .unwrap();
    let distances = std::array::from_fn(|i| {
        if prepared.supports()[i].face_reversed() {
            0.2
        } else {
            -0.2
        }
    });
    let section = propose_section(&prepared, 0.5, distances, 1e-8, 8, 1e-3, 1).unwrap();
    assert!(section.residual_mm <= 1e-8);
    assert!(
        matches!(
            section.certificate,
            Some(nurbs_core::surface_contact::Verdict::Unresolved)
        ),
        "{}: {:?}",
        section.reason,
        section.certificate
    );
    let sufficient = propose_section(&prepared, 0.5, distances, 1e-8, 8, 1e-3, 16).unwrap();
    assert!(
        matches!(
            sufficient.certificate,
            Some(nurbs_core::surface_contact::Verdict::Witness(_))
        ),
        "{}",
        sufficient.reason
    );
    assert!(propose_section(&prepared, -0.1, distances, 1e-8, 8, 1e-3, 16).is_err());
}
#[test]
fn face_work_stop_and_missing_material_ownership_cannot_prepare_an_edge() {
    let mut source = crate::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let mut work = limits();
    work.pairs = 1;
    work.region_cells = 1;
    work.domain_cells = 1;
    let r = prepare(&source, 0, 1e-8, work).unwrap();
    assert!(r.prepared.is_none());
    assert_eq!(r.support_audits.len(), 2);
    source.bodies.clear();
    source.rebuild_topology_ids();
    assert_eq!(
        prepare(&source, 0, 1e-8, limits()).err().unwrap().code,
        "BREP_OFFSET_EDGE_OWNERSHIP"
    );
}
