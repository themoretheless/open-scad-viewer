use super::*;
fn fixture(count: usize, spatial: bool, weighted: bool) -> Vec<Vec<Vec<Curve>>> {
    let quantum = 1. / 1024.;
    let snap = |x: f64| (x / quantum).round() * quantum;
    let mut rows = Vec::new();
    for station in 0..count - 1 {
        let angle = std::f64::consts::TAU * station as f64 / (count - 1) as f64;
        let (s, c) = angle.sin_cos();
        let z = if spatial {
            0.125 * (2. * angle).sin()
        } else {
            0.
        };
        let dz = if spatial {
            0.25 * (2. * angle).cos()
        } else {
            0.
        };
        let center = [snap(3. * c), snap(3. * s), snap(z)];
        let radial = [snap(c / 8.), snap(s / 8.), 0.];
        let binormal = [snap(-dz * s / 8.), snap(dz * c / 8.), -0.125];
        let points: Vec<Vec<f64>> = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)]
            .into_iter()
            .map(|(a, b)| {
                (0..3)
                    .map(|k| center[k] + a * radial[k] + b * binormal[k])
                    .collect()
            })
            .collect();
        rows.push(vec![
            (0..4)
                .map(|i| Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: vec![points[i].clone(), points[(i + 1) % 4].clone()],
                    weights: if weighted { vec![1., 2.] } else { vec![1., 1.] },
                    periodic: false,
                })
                .collect(),
        ]);
    }
    rows.push(rows[0].clone());
    rows
}
fn limits() -> crate::volume_validity::Limits {
    crate::volume_validity::Limits {
        boundary: crate::boundary_embedding::Limits {
            exact_work: 1_000_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 1000,
            contacts: crate::face_contacts::Limits {
                pairs: 10_000,
                cells: 100_000,
                domain_cells: 1_000_000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10_000,
            },
        },
        nesting_pairs: 1000,
        nesting_cells: 100_000,
        nesting_domain_cells: 1_000_000,
        orientation_cells: 100_000,
        orientation_domain_cells: 1_000_000,
        orientation_spans: 100,
    }
}

#[test]
fn independent_exact_station_jets_close_outside_fixed_station_modes() {
    for count in [6, 7, 10, 12] {
        for spatial in [false, true] {
            for weighted in [false, true] {
                let source = fixture(count, spatial, weighted);
                let untouched = source.clone();
                let result =
                    smooth_polygon_station_walls(&source, &[], true, 1. / 1024., 1., 100_000)
                        .unwrap();
                assert_eq!(source, untouched);
                assert!(result.sides.is_some(), "{}", result.reason);
                assert!(result.station_orders.iter().all(|x| *x == 2));
                let sides = result.sides.as_ref().unwrap();
                for station in 0..count - 1 {
                    for edge in 0..4 {
                        let a = &sides[0][station * 4 + edge];
                        let b = &sides[0][((station + 1) % (count - 1)) * 4 + edge];
                        let proof = nurbs_core::continuity::inspect_surface_exact_strip_jets(
                            a, b, "vMax", "vMin", 2, 1., 100_000,
                        )
                        .unwrap();
                        assert!(
                            proof.exact_identity,
                            "{count}/{spatial}/{weighted}/{station}/{edge}: {}",
                            proof.reason
                        );
                        assert!(
                            proof.certified,
                            "{count}/{spatial}/{weighted}: {}",
                            proof.reason
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn polygon_quintic_faces_require_full_weighted_projection_proofs() {
    for count in [6, 7, 10, 12] {
        for spatial in [false, true] {
            let result = smooth_polygon_station_walls(
                &fixture(count, spatial, false),
                &[],
                true,
                1. / 1024.,
                1.,
                100_000,
            )
            .unwrap();
            for (face, s) in result.sides.as_ref().unwrap()[0].iter().enumerate() {
                let proof =
                    nurbs_core::surface_linear_monotonicity::inspect_candidate(s, 10_000).unwrap();
                assert!(proof.certified, "{count}/{spatial}/{face}: {proof:?}");
            }
        }
    }
}

#[test]
fn closed_quintic_polygon_body_requires_fresh_whole_shell_proof() {
    for count in [6, 7, 10, 12] {
        for spatial in [false, true] {
            for weighted in [false, true] {
                let result = smooth_polygon_station_walls(
                    &fixture(count, spatial, weighted),
                    &[],
                    true,
                    1. / 1024.,
                    1.,
                    100_000,
                )
                .unwrap();
                let model = section_loft_surfaces(
                    result.sections.as_ref().unwrap(),
                    result.sides.as_ref().unwrap(),
                    true,
                )
                .unwrap();
                model.validate().unwrap();
                let station = crate::miter_seams::inspect(&model, &[], 1_000_000, true).unwrap();
                assert!(
                    station.g2.certified,
                    "{count}/{spatial}/{weighted}: actual station G2"
                );
                let proof = crate::volume_validity::inspect_sweep(
                    &model,
                    1e-8,
                    limits(),
                    100_000,
                    &[],
                    crate::sweep_cap_contacts::Budgets {
                        max_walls: 1024,
                        max_exact_work: 1_000_000,
                        max_chart_cells: 1000,
                        max_trim_pairs: 100_000,
                        max_trim_cells: 100_000,
                        max_trim_domain_cells: 1_000_000,
                    },
                )
                .unwrap();
                assert!(
                    proof.boundary.agreement.all_equal && proof.boundary.agreement.all_joins_exact,
                    "{count}/{spatial}/{weighted}: exact agreement"
                );
                assert!(
                    proof.boundary.intersections.faces.all_faces_injective,
                    "{count}/{spatial}/{weighted}: injectivity"
                );
                assert!(
                    proof.boundary.proven,
                    "{count}/{spatial}/{weighted}: boundary embedding; trim={} pending={:?} bad={:?}",
                    proof.boundary.trim.all_valid,
                    proof.boundary.intersections.pairs.next_pair,
                    proof
                        .boundary
                        .intersections
                        .pairs
                        .pairs
                        .iter()
                        .filter(|p| p.reason != "shared-boundary" && p.reason != "pair-disjoint")
                        .map(|p| (p.faces, p.reason, p.source_chart_cells))
                        .collect::<Vec<_>>()
                );
                assert!(proof.proven, "{count}/{spatial}/{weighted}: shell volume");
            }
        }
    }
}

#[test]
fn polygon_station_refusals_do_not_publish_partial_geometry_or_orders() {
    let source = fixture(6, true, true);
    let full = smooth_polygon_station_walls(&source, &[], true, 1. / 1024., 1., 100_000).unwrap();
    for max in [0, 1, full.work - 1] {
        let denied = smooth_polygon_station_walls(&source, &[], true, 1. / 1024., 1., max).unwrap();
        assert!(
            denied.sides.is_none() && denied.sections.is_none() && denied.station_orders.is_empty()
        );
    }
    let denied = smooth_polygon_station_walls(&source, &[], true, 1. / 1024., 0., 100_000).unwrap();
    assert_eq!(denied.reason, "displacement-budget");
    assert!(denied.sides.is_none());
    let mut gap = source.clone();
    gap[1][0][1].control_points[0][2] += 2_f64.powi(-40);
    assert_eq!(
        smooth_polygon_station_walls(&gap, &[], true, 1. / 1024., 1., 100_000)
            .unwrap()
            .reason,
        "authored-profile-gap"
    );
    let mut weights = source.clone();
    weights[1][0][0].weights[1] = 3.;
    assert_eq!(
        smooth_polygon_station_walls(&weights, &[], true, 1. / 1024., 1., 100_000)
            .unwrap()
            .reason,
        "incompatible-section-basis"
    );
    let sharp =
        smooth_polygon_station_walls(&source, &[0, 2], true, 1. / 1024., 1., 100_000).unwrap();
    assert_eq!(sharp.station_orders, vec![0, 2, 0, 2, 2, 0]);
}
