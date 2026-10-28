//! Public native geometry/replay/exchange path, runnable without `codec`.
use brep_core::{
    source_contour_proposal as region, source_shell_incidence as incidence, trimmed_face_recipe,
};
use nurbs_core::{curve::Curve, surface::Surface};
fn limits() -> trimmed_face_recipe::Limits {
    trimmed_face_recipe::Limits {
        pairs: 10000,
        region_cells: 100000,
        domain_cells: 100000,
        agreement_cells: 100000,
    }
}
fn replay_limits() -> region::ReplayLimits {
    region::ReplayLimits {
        region: limits(),
        search_cells: 100000,
        point_checks: 10000,
        mapping_cells: 10000,
        driver_cells: 10000,
        membership_cells: 100000,
        controls: 100000,
        winding_cells: 100000,
        steps: 64,
    }
}
fn cube(shift: f64) -> brep_core::source_volume::Body {
    let corners = [
        [[0., 1., 0.], [0., 0., 0.], [1., 1., 0.], [1., 0., 0.]],
        [[0., 0., 1.], [0., 1., 1.], [1., 0., 1.], [1., 1., 1.]],
        [[0., 0., 0.], [0., 0., 1.], [1., 0., 0.], [1., 0., 1.]],
        [[1., 1., 0.], [1., 1., 1.], [0., 1., 0.], [0., 1., 1.]],
        [[0., 1., 0.], [0., 1., 1.], [0., 0., 0.], [0., 0., 1.]],
        [[1., 0., 0.], [1., 0., 1.], [1., 1., 0.], [1., 1., 1.]],
    ];
    let uv = [vec![0., 0.], vec![1., 0.], vec![1., 1.], vec![0., 1.]];
    let mut regions = Vec::new();
    let mut boundaries = Vec::new();
    for controls in corners {
        let p = controls.map(|mut p| {
            p[0] += shift;
            p.to_vec()
        });
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![p[0].clone(), p[1].clone()],
                vec![p[2].clone(), p[3].clone()],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let wire = (0..4)
            .map(|i| {
                let a = &uv[i];
                let b = &uv[(i + 1) % 4];
                trimmed_face_recipe::Boundary {
                    curve: Curve::from_polyline(vec![
                        surface.evaluate(a[0], a[1]).unwrap().point.to_vec(),
                        surface.evaluate(b[0], b[1]).unwrap().point.to_vec(),
                    ])
                    .unwrap(),
                    pcurve: Curve::from_polyline(vec![a.clone(), b.clone()]).unwrap(),
                    reversed: false,
                }
            })
            .collect::<Vec<_>>();
        let r = region::qualify_original_region(
            &cad_predicates::ToleranceContext::default_valid(),
            &surface,
            &[wire.clone()],
            1e-8,
            limits(),
        )
        .unwrap();
        regions.push(r.region.expect(r.reason));
        boundaries.push(wire);
    }
    let mut pairs = Vec::new();
    for face in 0..6 {
        for edge in 0..4 {
            let curve = &boundaries[face][edge].curve;
            let a = &curve.control_points[0];
            let b = &curve.control_points[1];
            for other in face + 1..6 {
                for next in 0..4 {
                    let candidate = &boundaries[other][next].curve;
                    if a == &candidate.control_points[1] && b == &candidate.control_points[0] {
                        pairs.push(incidence::Pair {
                            uses: [
                                incidence::Address {
                                    face,
                                    wire: 0,
                                    edge,
                                },
                                incidence::Address {
                                    face: other,
                                    wire: 0,
                                    edge: next,
                                },
                            ],
                            world: curve.clone(),
                            world_reversed: [false, true],
                            cutters: [None, None],
                        });
                    }
                }
            }
        }
    }
    assert_eq!(pairs.len(), 12);
    let r = incidence::assemble_regions(&regions, &pairs, 100000000).unwrap();
    let r = brep_core::source_shell_geometry::qualify(
        r.shell.expect(r.reason),
        brep_core::source_shell_geometry::Limits {
            tolerance_uv: 1e-8,
            corners: 100,
            spans: 100000,
            linear_cells: 100000,
            pairs: brep_core::face_contacts::Limits {
                pairs: 15,
                cells: 100000,
                domain_cells: 100000,
                cells_per_pair: 10000,
                domain_cells_per_pair: 10000,
            },
            exact_work: 100000000,
            driver_cells: 100000,
        },
    )
    .unwrap();
    let r = brep_core::source_volume::qualify(
        r.geometry.expect(r.reason),
        brep_core::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 0.01,
            tolerance_uv: 1e-8,
            cells: 10000,
            spans: 100000,
            domain_cells: 1000000,
        },
    )
    .unwrap();
    r.body.expect(r.reason)
}
fn body_limits() -> brep_core::source_body_restore::Limits {
    brep_core::source_body_restore::Limits {
        shell: brep_core::source_shell_restore::Limits {
            regions: replay_limits(),
            exact_work: 100000000,
            driver_cells: 100000,
        },
        embedding: brep_core::source_shell_geometry::Limits {
            tolerance_uv: 1e-8,
            corners: 100,
            spans: 100000,
            linear_cells: 100000,
            pairs: brep_core::face_contacts::Limits {
                pairs: 15,
                cells: 100000,
                domain_cells: 100000,
                cells_per_pair: 10000,
                domain_cells_per_pair: 10000,
            },
            exact_work: 100000000,
            driver_cells: 100000,
        },
        volume: brep_core::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 0.01,
            tolerance_uv: 1e-8,
            cells: 10000,
            spans: 100000,
            domain_cells: 1000000,
        },
    }
}
fn edge_limits() -> brep_core::source_shared_edge::ReplayLimits {
    brep_core::source_shared_edge::ReplayLimits {
        mapping_cells_per_use: 10000,
        exact_work: 100000000,
        driver_cells: 100000,
    }
}
#[test]
fn source_body_replay_and_exchange_keep_native_provenance_without_wire_values() {
    let body = cube(0.);
    let replay = body.requalify(&body_limits()).unwrap();
    let replay = replay.body().expect(replay.reason());
    assert_eq!(
        replay.geometry().shell().vertices(),
        body.geometry().shell().vertices()
    );
    assert_eq!(replay.volume(), body.volume());
    let mut exhausted = body_limits();
    exhausted.shell.exact_work = 1;
    let stopped = body.requalify(&exhausted).unwrap();
    assert!(stopped.body().is_none());
    assert!(stopped.embedding.is_none());
    assert!(stopped.volume.is_none());
    let mut exhausted = body_limits();
    exhausted.embedding.pairs.pairs = 1;
    let stopped = body.requalify(&exhausted).unwrap();
    assert!(stopped.body().is_none());
    assert!(stopped.volume.is_none());
    let volume = body.volume();
    assert!(volume[0] <= 1. && volume[1] >= 1.);
    let shell = body.geometry().shell();
    let shell_limits = brep_core::source_shell_restore::Limits {
        regions: replay_limits(),
        exact_work: 100000000,
        driver_cells: 100000,
    };
    let replay = shell.requalify(&shell_limits).unwrap();
    let replay = replay.shell.expect(replay.reason);
    assert_eq!(replay.vertices(), shell.vertices());
    assert_eq!(replay.uses(), shell.uses());
    let exhausted = brep_core::source_shell_restore::Limits {
        regions: replay_limits(),
        exact_work: 1,
        driver_cells: 100000,
    };
    assert!(shell.requalify(&exhausted).unwrap().shell.is_none());
    let projective_pairs = shell
        .edges()
        .iter()
        .enumerate()
        .map(|(i, e)| incidence::Pair {
            uses: shell.uses()[i],
            world: e.world().clone(),
            world_reversed: [false; 2],
            cutters: [None, None],
        })
        .collect::<Vec<_>>();
    let projective_maps = shell
        .edges()
        .iter()
        .enumerate()
        .map(|(i, e)| incidence::ProjectiveMaps {
            pair: i,
            ranges: std::array::from_fn(|side| {
                if e.reversed()[side] ^ e.uses()[side].reversed() {
                    [[1., 1.], [0., 1.]]
                } else {
                    [[0., 1.], [1., 1.]]
                }
            }),
            weights: [[1.; 2]; 2],
        })
        .collect::<Vec<_>>();
    let projective = incidence::assemble_regions_projective(
        shell.regions().unwrap(),
        &projective_pairs,
        &[],
        &projective_maps,
        &[],
        100000000,
        100000,
    )
    .unwrap();
    let projective = projective.shell.expect(projective.reason);
    let replay = projective.requalify(&shell_limits).unwrap();
    let replay = replay.shell.expect(replay.reason);
    assert_eq!(replay.vertices(), projective.vertices());
    assert_eq!(replay.uses(), projective.uses());
    assert!(projective.requalify(&exhausted).unwrap().shell.is_none());
    let geometry =
        brep_core::source_shell_geometry::qualify_projective(projective, body_limits().embedding)
            .unwrap();
    let geometry = geometry.geometry.expect(geometry.reason);
    let volume = brep_core::source_volume::qualify(geometry, body_limits().volume).unwrap();
    let projected = volume.body.expect(volume.reason);
    let fresh = projected.requalify(&body_limits()).unwrap();
    let fresh = fresh.body().expect(fresh.reason());
    assert_eq!(
        fresh.geometry().shell().vertices(),
        projected.geometry().shell().vertices()
    );
    assert_eq!(fresh.volume(), projected.volume());
    let mut exhausted = body_limits();
    exhausted.embedding.pairs.pairs = 1;
    let stopped = projected.requalify(&exhausted).unwrap();
    assert!(stopped.body().is_none());
    assert!(stopped.volume.is_none());

    let original = &shell.regions().unwrap()[0];
    let split = original.split_boundary_parameter(0, 0, 0.5).unwrap();
    assert!(split.same_definition(&split.requalify(&replay_limits()).unwrap()));
    let edge = &shell.edges()[0];
    let fresh = edge.requalify(edge_limits()).unwrap();
    assert!(edge.same_definition(&fresh.edge.expect(fresh.reason)));
    let projective = brep_core::source_projective_edge::qualify(
        edge.world(),
        edge.uses().each_ref(),
        [[[0., 1.], [1., 1.]], [[1., 1.], [0., 1.]]],
        [[1.; 2]; 2],
        [None; 2],
        [[None; 2]; 2],
        100000000,
        100000,
    )
    .unwrap();
    let projective = projective.edge.expect(projective.reason);
    let fresh = projective.requalify(edge_limits()).unwrap();
    assert!(projective.same_definition(&fresh.edge.expect(fresh.reason)));
    let use_ = &edge.uses()[0];
    let contact = Curve::from_polyline(vec![vec![0.5, -0.25], vec![0.5, 0.25]]).unwrap();
    let point = brep_core::source_contact_point::qualify(
        use_.surface(),
        use_.curve(),
        &contact,
        [[0., 1.]; 2],
        10000,
    )
    .unwrap();
    let point = point.point.expect(point.reason);
    assert!(point.same_definition(&point.requalify(10000).unwrap().point.unwrap()));
    let carrier = brep_core::source_carrier_point::qualify(edge, &point, 0, 10000).unwrap();
    let carrier = carrier.point.expect(carrier.reason);
    let fresh = carrier
        .requalify(brep_core::source_carrier_point::RestoreLimits {
            edge: edge_limits(),
            root_mapping_cells: 10000,
            spans: 10000,
        })
        .unwrap();
    assert!(carrier.same_definition(&fresh.point.expect(fresh.reason)));
    let endpoint = brep_core::source_exchange_endpoints::prepare(
        &body,
        1e-7,
        brep_core::source_exchange_endpoints::Limits {
            root_checks: 10000,
            mapping_cells: 10000,
            replay_mapping_per_use: 10000,
            exact_work: 100000000,
            driver_cells: 100000,
            spans: 100000,
            endpoints: 1000,
        },
    )
    .unwrap();
    let endpoints = endpoint.prepared.expect(endpoint.reason);
    assert!(
        brep_core::source_exchange_trims::prepare(&body, &endpoints, 1e-7, 1000000)
            .unwrap()
            .is_some()
    );
    #[cfg(feature = "codec")]
    assert_eq!(endpoints.original(), &body.definition().unwrap());
    let same = cube(0.);
    assert!(
        brep_core::source_exchange_trims::prepare(&same, &endpoints, 1e-7, 1000000)
            .unwrap()
            .is_some()
    );
    #[cfg(feature = "codec")]
    assert_eq!(endpoints.original(), &same.definition().unwrap());
    let foreign = cube(0.25);
    #[cfg(feature = "codec")]
    assert_ne!(endpoints.original(), &foreign.definition().unwrap());
    assert!(
        brep_core::source_exchange_trims::prepare(&foreign, &endpoints, 1e-7, 1000000).is_err()
    );
    let projection = brep_core::source_body_topology::convert(&body, 1e-7, 100000, 1000).unwrap();
    assert_eq!(projection.topology().faces.len(), 6);
}
#[test]
fn capped_canal_body_admits_native_geometry_and_independent_volume_without_codec() {
    let spans = brep_core::linear_canal::construct(
        [[10., -7., 5.], [13., -3., 17.]],
        [0.5, 1.25],
        [1., 0., 0.],
        std::f64::consts::TAU,
    )
    .unwrap();
    let report = brep_core::linear_canal_body::qualify(
        &spans,
        brep_core::linear_canal_body::Limits {
            tolerance_mm: 1e-7,
            tolerance_uv: 1e-8,
            regions: limits(),
            incidence_exact_work: 100000000,
            embedding: brep_core::source_shell_geometry::Limits {
                tolerance_uv: 1e-8,
                corners: 10000,
                spans: 20000,
                linear_cells: 10000,
                pairs: brep_core::face_contacts::Limits {
                    pairs: 10000,
                    cells: 10000,
                    domain_cells: 100000,
                    cells_per_pair: 32,
                    domain_cells_per_pair: 512,
                },
                exact_work: 10000000,
                driver_cells: 10000,
            },
            volume: brep_core::source_volume::Limits {
                axis: 2,
                origin: 5.,
                absolute_error: 0.25,
                tolerance_uv: 1e-8,
                cells: 50000,
                spans: 100000,
                domain_cells: 100000,
            },
        },
    )
    .unwrap();
    let body = report.body().expect(report.reason());
    let mut replay_limits = body_limits();
    replay_limits.shell.driver_cells = 10000;
    replay_limits.embedding.corners = 10000;
    replay_limits.embedding.spans = 20000;
    replay_limits.embedding.linear_cells = 10000;
    replay_limits.embedding.pairs = brep_core::face_contacts::Limits {
        pairs: 10000,
        cells: 10000,
        domain_cells: 100000,
        cells_per_pair: 32,
        domain_cells_per_pair: 512,
    };
    replay_limits.embedding.exact_work = 10000000;
    replay_limits.embedding.driver_cells = 10000;
    replay_limits.volume.origin = 5.;
    replay_limits.volume.absolute_error = 0.25;
    replay_limits.volume.cells = 50000;
    replay_limits.volume.domain_cells = 100000;
    let replay = body.requalify(&replay_limits).unwrap();
    let replayed = replay.body().expect(replay.reason());
    assert_eq!(
        replayed.geometry().shell().vertices(),
        body.geometry().shell().vertices()
    );
    assert_eq!(replayed.volume(), body.volume());
    replay_limits.volume.cells = 1;
    let stopped = body.requalify(&replay_limits).unwrap();
    assert!(stopped.body().is_none());
    assert!(stopped.embedding.is_some());
    assert!(stopped.volume.is_some());

    let radii = [0.5_f64, 1.25];
    let length = 13.;
    let slope = (radii[1] - radii[0]) / length;
    let k2 = 1. - slope * slope;
    let h = [radii[0] * (1. - slope), radii[1] * (1. + slope)];
    let expected = std::f64::consts::PI
        * (length * k2 * k2 * (radii[0] * radii[0] + radii[0] * radii[1] + radii[1] * radii[1])
            / 3.
            + h[0] * h[0] * (radii[0] - h[0] / 3.)
            + h[1] * h[1] * (radii[1] - h[1] / 3.));
    let volume = body.volume();
    assert!(volume[0] <= expected && expected <= volume[1]);
    assert!(volume[0] > 0. && volume[1] - volume[0] <= 0.25);
    assert!(body.geometry().contacts().all_pairs_qualified);
}
