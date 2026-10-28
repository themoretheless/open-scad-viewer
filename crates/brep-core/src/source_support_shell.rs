//! Rebuild source regions and shared incidence from authored support topology.
//! Closed flags and Body records in the input confer no geometric authority.
use crate::source_shell_incidence::{Address, Pair, Pole};
use std::collections::BTreeMap;
pub struct Limits {
    pub tolerance_uv: f64,
    pub faces: usize,
    pub uses: usize,
    /// Independently bounded original-region work per face.
    pub regions: crate::trimmed_face_recipe::Limits,
    pub exact_work: u64,
}
pub fn prepare(
    model: &crate::Model,
    limits: Limits,
) -> crate::Result<crate::source_shell_incidence::Report> {
    prepare_using(model,limits,|regions,pairs,poles,work| {
        crate::source_shell_incidence::assemble_regions_with_poles(regions,pairs,poles,work)
    })
}
/// Rebuild authored support sheets using exact projective edge correspondence.
/// Pole contractions are proved from original surfaces; flags grant no authority.
pub fn prepare_projective(model:&crate::Model,limits:Limits)
    ->crate::Result<crate::source_shell_incidence::Report<crate::source_projective_edge::Edge>> {
    prepare_using(model,limits,|regions,pairs,poles,work| {
        crate::source_shell_incidence::assemble_regions_projective_with_poles(regions,pairs,&[],&[],&[],poles,work,0)
    })
}
fn prepare_using<E>(model:&crate::Model,limits:Limits,
    assemble:impl FnOnce(&[crate::source_contour_proposal::SourceRegion],&[Pair],&[Pole],u64)->nurbs_core::Result<crate::source_shell_incidence::Report<E>>,
)->crate::Result<crate::source_shell_incidence::Report<E>> {
    if !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || !(1..=4096).contains(&limits.faces)
        || !(1..=100000).contains(&limits.uses)
        || !(1..=100_000_000).contains(&limits.exact_work)
    {
        return Err(crate::invalid(
            "Choose bounded source support work and positive UV tolerance",
        ));
    }
    if model.faces.len() > limits.faces
        || model.shells.len() != 1
        || model.shells[0].faces.len() != model.faces.len()
    {
        return Err(crate::invalid(
            "Source support assembly requires one complete connected shell",
        ));
    }
    let count = model.loops.iter().map(|w| w.coedges.len()).sum::<usize>();
    if count > limits.uses {
        return Err(crate::invalid("Source support use limit"));
    }
    model.validate()?;
    // Propose consistently oriented charts. Every proposed lift and shared
    // carrier is freshly checked below; reflected floats grant no authority.
    let oriented = crate::face_senses::canonical_face_senses(model)?;
    let model = &oriented;
    let mut regions = Vec::new();
    let mut pairs = Vec::new();
    let mut poles = Vec::new();
    let mut pending = BTreeMap::new();
    for (face, support) in model.faces.iter().enumerate() {
        let mut wires = Vec::new();
        for (wire, index) in std::iter::once(support.outer)
            .chain(support.holes.iter().copied())
            .enumerate()
        {
            let mut boundaries = Vec::new();
            for (edge, use_) in model.loops[index].coedges.iter().enumerate() {
                let address = Address { face, wire, edge };
                let original = &model.edges[use_.edge];
                let pcurve = use_.pcurve.clone();
                let reversed = use_.reversed;
                boundaries.push(crate::trimmed_face_recipe::Boundary {
                    curve: original.curve.clone(),
                    pcurve,
                    reversed,
                });
                if original.degenerate {
                    poles.push(Pole {
                        use_: address,
                        point: model.vertices[original.vertices[0]].point,
                    });
                } else if let Some((other, reversed)) = pending.remove(&use_.edge) {
                    pairs.push(Pair {
                        uses: [other, address],
                        world: original.curve.clone(),
                        world_reversed: [reversed, use_.reversed],
                        cutters: [None, None],
                    });
                } else {
                    pending.insert(use_.edge, (address, reversed));
                }
            }
            wires.push(boundaries);
        }
        let qualified = crate::source_contour_proposal::qualify_original_region(
            &cad_predicates::ToleranceContext::default_valid(),
            &support.surface,
            &wires,
            limits.tolerance_uv,
            crate::trimmed_face_recipe::Limits {
                pairs: limits.regions.pairs,
                region_cells: limits.regions.region_cells,
                domain_cells: limits.regions.domain_cells,
                agreement_cells: limits.regions.agreement_cells,
            },
        )?;
        let Some(region) = qualified.region else {
            return Err(crate::invalid(format!(
                "Source support face {face}: {}",
                qualified.audit.reason
            )));
        };
        regions.push(region);
    }
    if !pending.is_empty() {
        return Err(crate::invalid(
            "Source supports retain unpaired ordinary edges",
        ));
    }
    assemble(
        &regions,
        &pairs,
        &poles,
        limits.exact_work,
    )
    .map_err(|e| crate::Error::new(e.code, e.message))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            tolerance_uv: 1e-8,
            faces: 4096,
            uses: 100000,
            regions: crate::trimmed_face_recipe::Limits {
                pairs: 10000,
                region_cells: 10000,
                domain_cells: 10000,
                agreement_cells: 10000,
            },
            exact_work: 100_000_000,
        }
    }
    fn geometry_limits() -> crate::source_shell_geometry::Limits {
        crate::source_shell_geometry::Limits {
            tolerance_uv: 1e-8,
            corners: 10000,
            spans: 100000,
            linear_cells: 100000,
            exact_work: 100_000_000,
            driver_cells: 100000,
            pairs: crate::face_contacts::Limits {
                pairs: 10000,
                cells: 10000,
                domain_cells: 100000,
                cells_per_pair: 32,
                domain_cells_per_pair: 512,
            },
        }
    }
    #[test]
    fn projective_body_material_chord_and_automatic_thin_witness_keep_source_ownership() {
        let model = crate::cuboid([0., 0., 0.], [10., 20., 30.]).unwrap();
        let shell = prepare_projective(&model, limits()).unwrap().shell.unwrap();
        let geometry =
            crate::source_shell_geometry::qualify_projective(shell, geometry_limits()).unwrap();
        let geometry = geometry.geometry.expect(geometry.reason);
        let volume = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 0,
                origin: 0.,
                absolute_error: 0.01,
                tolerance_uv: 1e-8,
                cells: 10000,
                spans: 100000,
                domain_cells: 100000,
            },
        )
        .unwrap();
        let body = volume.body.expect(volume.reason);
        let original = body.definition().unwrap();
        let chord_limits = || crate::source_material_chord::Limits {
            cells: 10000,
            domain_cells: 10000,
            normal_spans: 10000,
            max_sine_squared: 1e-12,
        };
        let r = crate::source_material_chord::qualify(
            &body,
            [-1., 7., 11.],
            [12., 0., 0.],
            1e-8,
            chord_limits(),
        )
        .unwrap();
        assert!(r.boundary.unresolved.is_empty());
        assert_eq!(r.boundary.contacts.len(), 2);
        let proof = r.certificate.expect(r.reason);
        assert!(std::ptr::eq(proof.body(), &body));
        assert!(proof.length_mm()[0] <= 10. && proof.length_mm()[1] >= 10.);
        assert!(proof.length_mm()[1] - proof.length_mm()[0] < 1e-6);
        let found =
            crate::source_wall_scan::search(&body, 11., 1, 6, 1e-8, chord_limits()).unwrap();
        assert_eq!(found.attempts, 6);
        assert_eq!(found.faces_visited, 6);
        assert!(found.proposals_exhausted);
        let thin = found
            .thin_witness()
            .expect("A certified 10 mm chord lies below 11 mm");
        assert!(std::ptr::eq(thin.body(), &body));
        assert!(thin.length_mm()[1] < 11.);
        let no_thin =
            crate::source_wall_scan::search(&body, 9., 1, 6, 1e-8, chord_limits()).unwrap();
        assert!(no_thin.best().is_some());
        assert!(no_thin.thin_witness().is_none());
        let limited =
            crate::source_wall_scan::search(&body, 11., 1, 1, 1e-8, chord_limits()).unwrap();
        assert_eq!(limited.attempts, 1);
        assert!(!limited.proposals_exhausted);
        let refused = crate::source_material_chord::qualify(
            &body,
            [-1., 7., 11.],
            [12., 0., 0.],
            1e-8,
            crate::source_material_chord::Limits {
                cells: 1,
                domain_cells: 1,
                normal_spans: 1,
                max_sine_squared: 1e-12,
            },
        )
        .unwrap();
        assert!(refused.certificate.is_none());
        assert!(!refused.boundary.unresolved.is_empty());
        for (minimum, pairs, normals, accepted) in [
            (9.99, 21, 6, true),
            (10.01, 21, 6, false),
            (9.99, 20, 6, false),
            (9.99, 21, 1, false),
        ] {
            let lower = crate::source_wall_coverage::qualify(
                &body,
                minimum,
                crate::source_wall_coverage::Limits {
                    pairs,
                    plane_controls: 10000,
                    normal_spans: normals,
                    gap: crate::source_face_gap::Limits {
                        cells: 10000,
                        spans: 20000,
                    },
                    max_sine_squared: 1e-6,
                },
            )
            .unwrap();
            assert_eq!(lower.total_pairs, 21);
            assert_eq!(lower.enumeration_complete, pairs == 21);
            assert_eq!(lower.certificate.is_some(), accepted, "{}", lower.reason);
            if let Some(proof) = lower.certificate {
                assert!(std::ptr::eq(proof.body(), &body));
                assert_eq!(proof.minimum_mm(), minimum);
            }
        }
        let opposing = proof.faces();
        let first = [opposing[0]];
        let second = [opposing[1]];
        let bounded = crate::source_material_wall::qualify(
            &body,
            [&first, &second],
            9.99,
            0.02,
            [-1., 7., 11.],
            [12., 0., 0.],
            1e-8,
            crate::source_material_wall::Limits {
                gap: crate::source_face_gap::Limits {
                    cells: 10000,
                    spans: 20000,
                },
                chord: chord_limits(),
            },
        )
        .unwrap();
        assert!(bounded.converged);
        let interval = bounded.certificate.unwrap();
        assert!(std::ptr::eq(interval.body(), &body));
        assert!(interval.interval_mm()[0] <= 10. && interval.interval_mm()[1] >= 10.);
        let refined = crate::source_material_wall::search_and_refine(
            &body,
            [&first, &second],
            9.,
            1e-4,
            1e-8,
            1,
            1,
            crate::source_material_wall::Limits {
                gap: crate::source_face_gap::Limits {
                    cells: 10000,
                    spans: 20000,
                },
                chord: chord_limits(),
            },
        )
        .unwrap();
        assert!(refined.result.converged);
        let refined = refined.result.certificate.unwrap();
        assert!(std::ptr::eq(refined.body(), &body));
        assert!(refined.interval_mm()[1] - refined.interval_mm()[0] <= 1e-4);
        let self_limits = || crate::source_wall_self_coverage::Limits {
            plane_controls: 10000,
            cells: 1,
            spans: 1,
            cells_per_face: 1,
            spans_per_face: 1,
        };
        for (minimum, angle, accepted) in [
            (9.99, 1e-6, true),
            (10.01, 1e-6, false),
            (9.99, 2e-6, false),
        ] {
            let own = crate::source_wall_self_coverage::qualify(&body, 9.99, 1e-6, self_limits())
                .unwrap();
            assert!(own.all_self_pairs_qualified());
            assert_eq!(own.certificates().len(), 6);
            let covered = crate::source_wall_coverage::qualify_with_self_facts(
                &body,
                minimum,
                crate::source_wall_coverage::Limits {
                    pairs: 21,
                    plane_controls: 10000,
                    normal_spans: 6,
                    gap: crate::source_face_gap::Limits {
                        cells: 10000,
                        spans: 20000,
                    },
                    max_sine_squared: angle,
                },
                own.into_certificates(),
            );
            if accepted {
                assert_eq!(
                    covered
                        .unwrap()
                        .certificate
                        .unwrap()
                        .adaptive_self_certificates()
                        .len(),
                    6
                );
            } else {
                assert_eq!(
                    covered.err().expect("Stale threshold or angle").code,
                    "BREP_SOURCE_SELF_WALL_FACT"
                );
            }
        }
        assert_eq!(body.definition().unwrap(), original);
    }
    #[test]
    fn projective_spherical_end_junctions_have_synchronized_whole_seam_tangency() {
        let spans = crate::linear_canal::construct(
            [[0., 0., 0.], [0., 0., 8.]],
            [0.5, 1.25],
            [1., 0., 0.],
            std::f64::consts::TAU,
        )
        .unwrap();
        let model = crate::linear_canal::to_capped_region(&spans, 1e-7).unwrap();
        let shell = prepare_projective(&model, limits()).unwrap().shell.unwrap();
        let mut joins = 0;
        for edge in shell.edges() {
            let degrees = edge
                .uses()
                .each_ref()
                .map(|u| u.fragment().surface().degree_u);
            if degrees[0] == degrees[1] {
                continue;
            }
            let r = crate::source_seam_tangency::qualify_with_synchronized_boundaries(
                edge,
                crate::source_seam_tangency::Limits {
                    max_sine_squared: 1e-10,
                    cells: 1,
                    curve_spans: 2,
                    normal_spans: 2,
                },
                100000,
            )
            .unwrap();
            assert!(
                r.report.seam.is_some(),
                "{} / {} {:?}",
                r.report.reason,
                r.synchronized_reason,
                r.report.uncertain_canonical
            );
            assert!(r.polynomial_work <= 100000);
            assert_eq!(r.report.accepted_cells, 1);
            assert!(r.report.seam.unwrap().sine_squared_bounds()[1] <= 1e-10);
            joins += 1;
        }
        assert_eq!(joins, 8);
    }
    #[test]
    fn projective_generated_end_caps_keep_exact_poles_and_replay() {
        let spans = crate::linear_canal::construct(
            [[0., 0., 0.], [0., 0., 8.]],
            [0.5, 1.25],
            [1., 0., 0.],
            std::f64::consts::TAU,
        )
        .unwrap();
        let model = crate::linear_canal::to_capped_region(&spans, 1e-7).unwrap();
        let report = prepare_projective(&model, limits()).unwrap();
        let shell = report.shell.expect(report.reason);
        assert!(!shell.poles().is_empty());
        let links = crate::source_vertex_links::inspect(&shell, 10000).unwrap();
        assert!(links.all_manifold);
        assert_eq!(links.genus, Some(0));
        assert!(
            shell
                .inspect_face_charts(100000, 100000)
                .unwrap()
                .all_injective
        );
        let definition = shell.definition().unwrap();
        let bytes = value_codec::encode_binary(&definition).unwrap();
        let restored = crate::source_projective_shell_restore::restore(
            value_codec::decode_binary(&bytes).unwrap(),
            &crate::source_projective_shell_restore::Limits {
                regions: crate::source_region_restore::test_limits(),
                exact_work: 100000000,
                driver_cells: 10000,
            },
        )
        .unwrap();
        let replay = restored.shell.expect(restored.reason);
        assert_eq!(replay.definition().unwrap(), definition);
        assert_eq!(replay.vertices(), shell.vertices());
        assert_eq!(replay.poles().len(), shell.poles().len());
        let embedded =
            crate::source_shell_geometry::qualify_projective(replay, geometry_limits()).unwrap();
        assert!(
            embedded.geometry.is_some(),
            "{} pairs {} next {:?}",
            embedded.reason,
            embedded.pairs,
            embedded.next_pair
        );
        let geometry = embedded.geometry.unwrap();
        assert!(geometry.contacts().all_pairs_qualified);
        assert!(
            geometry
                .contacts()
                .pairs
                .iter()
                .any(|p| p.pole_paired.is_some())
        );
        let volume = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.25,
                tolerance_uv: 1e-8,
                cells: 50000,
                spans: 100000,
                domain_cells: 100000,
            },
        )
        .unwrap();
        let body = volume.body.expect(volume.reason);
        let r = [0.5_f64, 1.25];
        let length = 8.;
        let slope = (r[1] - r[0]) / length;
        let k2 = 1. - slope * slope;
        let h = [r[0] * (1. - slope), r[1] * (1. + slope)];
        let expected = std::f64::consts::PI
            * (length * k2 * k2 * (r[0] * r[0] + r[0] * r[1] + r[1] * r[1]) / 3.
                + h[0] * h[0] * (r[0] - h[0] / 3.)
                + h[1] * h[1] * (r[1] - h[1] / 3.));
        let bounds = body.volume();
        assert!(
            bounds[0] <= expected && expected <= bounds[1],
            "{expected} outside {bounds:?}"
        );
        assert!(bounds[0] > 0. && bounds[1] - bounds[0] <= 0.25);
        assert!(body.reverse_orientation());
        assert_eq!(body.geometry().contacts().pairs.len(), 120);
        let transferred = crate::source_body_model::convert(&body, 1e-7).unwrap();
        assert_eq!(transferred.bodies.len(), 1);
        assert!(transferred.shells[0].closed);
        assert_eq!(transferred.faces.len(), model.faces.len());
        assert_eq!(
            transferred.edges.iter().filter(|e| e.degenerate).count(),
            body.geometry().shell().poles().len()
        );
        let (step, _, _) = crate::export_step_v6(&transferred).unwrap();
        if let Some(path) = std::env::var_os("CAD_PROJECTIVE_CAPPED_STEP_OUTPUT") {
            std::fs::write(path, &step).unwrap();
        }
        let (imported, _, _) = crate::import_step_v6(&step).unwrap();
        crate::source_body_model::assert_step_identity(&transferred, &imported);
        let body_definition = body.definition().unwrap();
        let exchange = crate::source_exchange_step::prepare(
            &body,
            1e-7,
            crate::source_exchange_endpoints::Limits {
                root_checks: 10000,
                mapping_cells: 100000,
                replay_mapping_per_use: 10000,
                exact_work: 100000000,
                driver_cells: 10000,
                spans: 100000,
                endpoints: 10000,
            },
            1000000,
        )
        .unwrap()
        .expect("Projective capped source STEP preparation");
        assert!(exchange.endpoint_error_upper <= 1e-7);
        assert_eq!(exchange.faces, model.faces.len());
        assert_eq!(
            exchange.edges,
            body.geometry().shell().edges().len() + body.geometry().shell().poles().len()
        );
        assert_eq!(body.definition().unwrap(), body_definition);
        if let Some(path) = std::env::var_os("CAD_PROJECTIVE_CAPPED_SOURCE_STEP_OUTPUT") {
            std::fs::write(path, &exchange.text).unwrap();
        }
        let body_bytes = value_codec::encode_binary(&body_definition).unwrap();
        let restored_body = crate::source_projective_body_restore::restore(
            value_codec::decode_binary(&body_bytes).unwrap(),
            crate::source_projective_body_restore::Limits {
                shell: crate::source_projective_shell_restore::Limits {
                    regions: crate::source_region_restore::test_limits(),
                    exact_work: 100000000,
                    driver_cells: 10000,
                },
                embedding: geometry_limits(),
                volume: crate::source_volume::Limits {
                    axis: 2,
                    origin: 0.,
                    absolute_error: 0.25,
                    tolerance_uv: 1e-8,
                    cells: 50000,
                    spans: 100000,
                    domain_cells: 100000,
                },
            },
        )
        .unwrap();
        let recovered = restored_body.body().expect(restored_body.reason());
        assert_eq!(recovered.definition().unwrap(), body_definition);
        assert_eq!(recovered.volume(), body.volume());
        assert_eq!(
            recovered.geometry().shell().vertices(),
            body.geometry().shell().vertices()
        );
        assert_eq!(
            crate::source_body_model::convert(recovered, 1e-7).unwrap(),
            transferred
        );
        for admitted in [&body, recovered] {
            let mut tangent_joins = 0;
            for (index, edge) in admitted.geometry().shell().edges().iter().enumerate() {
                let degrees = edge
                    .uses()
                    .each_ref()
                    .map(|u| u.fragment().surface().degree_u);
                if degrees[0] == degrees[1] {
                    continue;
                }
                let proof = admitted
                    .qualify_edge_tangency_synchronized(
                        index,
                        crate::source_seam_tangency::Limits {
                            max_sine_squared: 1e-10,
                            cells: 1,
                            curve_spans: 2,
                            normal_spans: 2,
                        },
                        100000,
                    )
                    .unwrap();
                assert!(proof.report.seam.is_some(), "{}", proof.report.reason);
                assert_eq!(proof.report.accepted_cells, 1);
                tangent_joins += 1;
            }
            assert_eq!(tangent_joins, 8);
            assert!(
                admitted
                    .qualify_edge_tangency_synchronized(
                        usize::MAX,
                        crate::source_seam_tangency::Limits {
                            max_sine_squared: 1e-10,
                            cells: 1,
                            curve_spans: 2,
                            normal_spans: 2
                        },
                        100000
                    )
                    .is_err()
            );
        }

        let mut bad = definition.clone();
        let point = bad["poles"][0]["point"][0].as_f64().unwrap();
        bad["poles"][0]["point"][0] = value_codec::json!(point + 1e-12);
        assert!(
            crate::source_projective_shell_restore::restore(
                bad,
                &crate::source_projective_shell_restore::Limits {
                    regions: crate::source_region_restore::test_limits(),
                    exact_work: 100000000,
                    driver_cells: 10000
                }
            )
            .unwrap()
            .shell
            .is_none()
        );
    }
    #[test]
    fn cylinder_straight_shared_edge_has_owned_paired_contact() {
        let model = crate::analytic::cylinder(10., 30.).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        for faces in [[0, 1], [1, 0]] {
            let r = crate::source_fiber_contact::certify_with_linear_chart(
                &shell, faces, 100000000, 100000, 100000, 100000,
            )
            .unwrap();
            eprintln!(
                "cylinder fiber {faces:?}: {} boundary {:?} work={} spans={} drivers={} linear={}",
                r.reason,
                r.uncertain_boundary,
                r.exact_work,
                r.spans,
                r.driver_cells,
                r.linear_cells
            );
            assert!(
                r.exact_work <= 100000000
                    && r.spans <= 100000
                    && r.driver_cells <= 100000
                    && r.linear_cells <= 100000
            );
            let p = crate::source_paired_fiber_contact::certify(&shell, faces, 100000000, 100000)
                .unwrap();
            eprintln!("cylinder paired {faces:?}: {}", p.reason);
            let certificate = p.certificate.expect(p.reason);
            assert_eq!(certificate.faces(), faces);
            assert!(!certificate.edges().is_empty());
            assert!(p.exact_work <= 100000000 && p.driver_cells <= 100000);
        }
        let admitted = crate::source_shell_geometry::qualify(shell, geometry_limits()).unwrap();
        assert!(
            admitted.geometry.is_some(),
            "{} pair {:?}",
            admitted.reason,
            admitted.next_pair
        );
    }
    #[test]
    fn partial_canonical_carrier_replays_a_body_and_keeps_owned_chord_maps() {
        use value_codec::{Serialize, json};
        for case in 0..3 {
            let smooth = case != 0;
            let model = if case == 2 {
                crate::analytic::cylinder(10., 30.).unwrap()
            } else if smooth {
                let points = [[0., 0.], [5., 0.], [10., 0.], [10., 20.], [0., 20.]];
                let curves = (0..points.len())
                    .map(|i| {
                        nurbs_core::curve::Curve::from_polyline(vec![
                            points[i].to_vec(),
                            points[(i + 1) % points.len()].to_vec(),
                        ])
                        .unwrap()
                    })
                    .collect();
                crate::prism::extrude(&[curves], 0., 30.).unwrap()
            } else {
                crate::cuboid([0., 0., 0.], [10., 20., 30.]).unwrap()
            };
            let shell = prepare(&model, limits()).unwrap().shell.unwrap();
            if smooth {
                let baseline = prepare(&model, limits()).unwrap().shell.unwrap();
                let baseline =
                    crate::source_shell_geometry::qualify(baseline, geometry_limits()).unwrap();
                assert!(
                    baseline.geometry.is_some(),
                    "unmodified split-side prism: {} pair {:?}",
                    baseline.reason,
                    baseline.next_pair
                );
            }
            let index = if case == 2 {
                shell
                    .edges()
                    .iter()
                    .position(|e| e.world().degree == 1 && e.world().control_points.len() == 2)
                    .unwrap()
            } else if smooth {
                shell
                    .edges()
                    .iter()
                    .position(|e| {
                        e.world().degree == 1
                            && e.world()
                                .control_points
                                .iter()
                                .all(|p| p[0] == 5. && p[1] == 0.)
                    })
                    .expect("shared coplanar side seam")
            } else {
                0
            };
            let original = &shell.edges()[index];
            let addresses = shell.uses()[index];
            let faces = addresses.map(|a| a.face);
            let mut world = original.world().clone();
            assert_eq!(world.degree, 1);
            assert_eq!(world.control_points.len(), 2);
            let old = world.control_points.clone();
            for k in 0..3 {
                world.control_points[0][k] = 2. * old[0][k] - old[1][k];
                world.control_points[1][k] = 2. * old[1][k] - old[0][k];
            }
            let ranges = crate::source_seam_tangency::ranges(original)
                .map(|ends| ends.map(|r| [r[0] + r[1], 3. * r[1]]));
            let mut definition = shell.definition().unwrap();
            definition["pairs"][index]["edge"]["world"] = world.to_value();
            let planes = [None::<[[f64; 3]; 3]>; 2];
            let candidates = [[None::<[f64; 2]>; 2]; 2];
            definition["pairs"][index]["edge"]["recipe"] = json!({"kind":"mapped","ranges":ranges,
            "planes":planes,"candidates":candidates});
            let replay = crate::source_shell_restore::restore(
                definition,
                &crate::source_shell_restore::Limits {
                    regions: crate::source_region_restore::test_limits(),
                    exact_work: 100000000,
                    driver_cells: 100000,
                },
            )
            .unwrap();
            let shell = replay.shell.expect(replay.reason);
            assert!(!shell.edges()[index].covers_complete_canonical_source());
            let geometry = crate::source_shell_geometry::qualify(shell, geometry_limits()).unwrap();
            let geometry = geometry
                .geometry
                .unwrap_or_else(|| panic!("{} pair {:?}", geometry.reason, geometry.next_pair));
            let volume = crate::source_volume::qualify(
                geometry,
                crate::source_volume::Limits {
                    axis: 2,
                    origin: 0.,
                    absolute_error: 0.02,
                    tolerance_uv: 1e-8,
                    cells: 100000,
                    spans: 100000,
                    domain_cells: 1000000,
                },
            )
            .unwrap();
            let body = volume.body.expect(volume.reason);
            let shared = crate::source_shared_chord_map::prepare_for_chords(&body, faces, 100)
                .unwrap()
                .unwrap();
            assert_eq!(shared.edge(), index);
            assert!(
                crate::source_shared_chord_map::prepare(&body, faces, 100)
                    .unwrap()
                    .is_none()
            );
            let axes = shared.fixed_axes();
            let fixed = shared.fixed_values();
            let ends = shared.free_endpoints();
            let rectangles = std::array::from_fn(|side| {
                std::array::from_fn(|axis| {
                    if axis == axes[side] {
                        [fixed[side]; 2]
                    } else {
                        [ends[side][0] * 0.5 + ends[side][1] * 0.5; 2]
                    }
                })
            });
            for side in 0..2 {
                let canonical = shared.canonical_bounds(side, rectangles[side]).unwrap();
                assert!(canonical[0] <= 0.5 && 0.5 <= canonical[1]);
            }
            assert_eq!(shared.tangent_bounds(rectangles, 0).unwrap(), (None, 0));
            let mut outside = rectangles[0];
            outside[1 - axes[0]] = [1e10; 2];
            assert!(shared.canonical_bounds(0, outside).is_err());
            let (tangent, spent) = shared.tangent_bounds(rectangles, 100).unwrap();
            assert!(spent > 0);
            let tangent = tangent.unwrap();
            for k in 0..3 {
                let exact = 3. * (old[1][k] - old[0][k]);
                assert!(tangent[k][0] <= exact && exact <= tangent[k][1]);
            }
            let expected_volume = if case == 2 {
                3000. * std::f64::consts::PI
            } else {
                6000.
            };
            assert!(body.volume()[0] <= expected_volume && expected_volume <= body.volume()[1]);
            let specs = [crate::source_profile_body::FaceSpec {
                face: faces[0],
                anchor: [0, 0],
            }];
            let proposed = crate::source_profile_body::prepare(
                &body,
                &specs,
                crate::source_profile_body::Limits {
                    controls: 100,
                    fragments: 100,
                    position_tolerance: 1e-8,
                },
            )
            .unwrap();
            let proposal = proposed.proposal.expect(proposed.reason);
            let sharp = proposal
                .qualify_edge_interval(index, [0.4, 0.6], 1e-6, 100, 1000)
                .unwrap();
            assert_eq!(sharp.angular.aligned, Some(smooth));
            assert_eq!(sharp.seam.is_some(), smooth);
            if let Some(seam) = &sharp.seam {
                assert_eq!(seam.edge(), index);
                assert!(std::ptr::eq(seam.proposal(), &proposal));
            }
            let outside = proposal
                .qualify_edge_interval(index, [0., 1.], 1e-6, 100, 1000)
                .unwrap();
            assert_eq!(outside.angular.reason, "profile-seam-carrier-unproven");
            assert!(outside.seam.is_none());
            let axis = 1 - axes[0];
            let source = shared.surfaces()[0];
            let high = if axes[0] == 0 {
                fixed[0] == source.knots_u[source.control_points.len()]
            } else {
                fixed[0] == source.knots_v[source.control_points[0].len()]
            };
            let parameter = (ends[0][0] + ends[0][1]) * 0.5;
            let cut = proposal
                .cut_endpoint(faces[0], axis, parameter, high, index, 100)
                .unwrap();
            let cut = cut.endpoint.expect(cut.reason);
            let q = cut.canonical_bounds();
            assert!(q[0] <= 0.5 && 0.5 <= q[1]);
            let stopped = proposal
                .cut_endpoint(faces[0], axis, parameter, high, index, 0)
                .unwrap();
            assert!(stopped.endpoint.is_none());
            assert_eq!(stopped.spans, 0);

            let report_variable = if case == 2 {
                "CAD_PARTIAL_CARRIER_CYLINDER_REPORT_PATH"
            } else if smooth {
                "CAD_PARTIAL_CARRIER_SMOOTH_REPORT_PATH"
            } else {
                "CAD_PARTIAL_CARRIER_BODY_REPORT_PATH"
            };
            if let Some(path) = std::env::var_os(report_variable) {
                let report = json!({"definition":body.definition().unwrap(),"faces":faces,"edge":shared.edge(),
                "rectangles":rectangles,"tangent":tangent,"spans":spent,"volume":body.volume(),
                "partialCanonicalSource":true,"wholeWallQualified":false,"smoothControl":smooth,
                "cutCanonicalBounds":q,"sharpSeamAligned":sharp.angular.aligned,
                "sharpSeamReason":sharp.angular.reason,"outsideSeamReason":outside.angular.reason});
                std::fs::write(path, value_codec::to_string_pretty(&report).unwrap()).unwrap();
            }
        }
    }
    #[test]
    fn source_retained_gap_covers_owned_opposite_faces_and_preserves_exhaustion() {
        let model = crate::cuboid([0., 0., 0.], [10., 20., 30.]).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let geometry = crate::source_shell_geometry::qualify(shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
        let volume = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.02,
                tolerance_uv: 1e-8,
                cells: 100000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap();
        let body = volume.body.unwrap();
        let options = || crate::source_retained_face_gap::Limits {
            cells: 8,
            spans: 16,
            domain_cells: 1,
            domain_cells_per_query: 1,
            tolerance_uv: 1e-8,
        };
        let proved =
            crate::source_retained_face_gap::qualify(&body, [0, 1], 29.99, options()).unwrap();
        let c = proved.certificate.unwrap();
        assert!(std::ptr::eq(c.body(), &body));
        assert_eq!(c.faces(), [0, 1]);
        assert_eq!(c.lower_mm(), 29.99);
        assert!(proved.unresolved_products.is_empty());
        assert_eq!(proved.cells, 1);
        assert_eq!(proved.spans, 2);
        assert!(proved.domain_cells <= 1);
        let mixed_options = || crate::source_wall_pair::Limits {
            normal_spans: 1000,
            max_sine_squared: 1e-6,
            cells: 64,
            spans: 128,
            domain_cells: 1000,
            domain_cells_per_query: 100,
            tolerance_uv: 1e-8,
        };
        let perpendicular =
            crate::source_wall_pair::qualify(&body, [0, 2], 100., mixed_options()).unwrap();
        let proof = perpendicular.certificate.unwrap();
        assert!(std::ptr::eq(proof.body(), &body));
        assert_eq!(proof.faces(), [0, 2]);
        assert_eq!(proof.lower_mm(), 100.);
        assert_eq!(proof.max_sine_squared(), 1e-6);
        assert_eq!(perpendicular.normal_products, 1);
        assert!(perpendicular.unresolved_products.is_empty());
        // Adjacent faces have zero gap, but their orthogonal normal directions
        // exclude the aligned chords required by this wall criterion.
        assert!(
            crate::source_retained_face_gap::qualify(&body, [0, 2], 100., options())
                .unwrap()
                .certificate
                .is_none()
        );
        let too_thin =
            crate::source_wall_pair::qualify(&body, [0, 1], 30.01, mixed_options()).unwrap();
        assert!(too_thin.certificate.is_none());
        assert!(!too_thin.unresolved_products.is_empty());
        // UV resolution is a refusal boundary, never a thickness certificate.
        let mut coarse = mixed_options();
        coarse.tolerance_uv = 100.;
        let stopped = crate::source_wall_pair::qualify(&body, [0, 1], 30.01, coarse).unwrap();
        assert!(stopped.certificate.is_none());
        assert_eq!(stopped.reason, "source-wall-pair-resolution-limit");
        assert_eq!(stopped.cells, 1);
        assert_eq!(stopped.unresolved_products.len(), 1);

        assert!(
            crate::source_wall_pair::qualify(&body, [0, 1], 29.99, mixed_options())
                .unwrap()
                .certificate
                .is_some()
        );
        let coverage_options = || crate::source_wall_coverage::Limits {
            pairs: 21,
            plane_controls: 10000,
            normal_spans: 10000,
            gap: crate::source_face_gap::Limits {
                cells: 10000,
                spans: 20000,
            },
            max_sine_squared: 1e-6,
        };
        let no_products = || crate::source_wall_coverage::NormalProductLimits {
            cells: 0,
            cells_per_pair: 0,
        };
        let no_factors = || crate::source_wall_coverage::FactoredNormalLimits {
            exact_work: 0,
            queries: 0,
            cells: 0,
            cells_per_pair: 0,
        };
        let rejected = crate::source_wall_coverage::qualify_with_retained_gap_facts(
            &body,
            30.01,
            coverage_options(),
            Vec::new(),
            no_products(),
            no_factors(),
            vec![c],
        );
        assert!(rejected.is_err());
        let c1 = crate::source_retained_face_gap::qualify(&body, [0, 1], 29.99, options())
            .unwrap()
            .certificate
            .unwrap();
        let c2 = crate::source_retained_face_gap::qualify(&body, [1, 0], 29.99, options())
            .unwrap()
            .certificate
            .unwrap();
        assert!(
            crate::source_wall_coverage::qualify_with_retained_gap_facts(
                &body,
                9.99,
                coverage_options(),
                Vec::new(),
                no_products(),
                no_factors(),
                vec![c1, c2]
            )
            .is_err()
        );
        let owned = crate::source_retained_face_gap::qualify(&body, [0, 1], 29.99, options())
            .unwrap()
            .certificate
            .unwrap();
        let coverage = crate::source_wall_coverage::qualify_with_retained_gap_facts(
            &body,
            9.99,
            coverage_options(),
            Vec::new(),
            no_products(),
            no_factors(),
            vec![owned],
        )
        .unwrap();
        let complete = coverage.certificate.unwrap();
        assert_eq!(complete.retained_gap_certificates().len(), 1);
        assert!(std::ptr::eq(complete.body(), &body));
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let geometry = crate::source_shell_geometry::qualify(shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
        let other = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.02,
                tolerance_uv: 1e-8,
                cells: 100000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap()
        .body
        .unwrap();
        macro_rules! mixed_coverage {
            ($target:expr,$minimum:expr,$angle:expr,$proofs:expr) => {{
                let mut l = coverage_options();
                l.max_sine_squared = $angle;
                crate::source_wall_coverage::qualify_with_mixed_pair_facts(
                    $target,
                    $minimum,
                    l,
                    Vec::new(),
                    no_products(),
                    no_factors(),
                    Vec::new(),
                    None,
                    $proofs,
                )
            }};
        }
        let mixed = crate::source_wall_pair::qualify(&body, [0, 2], 100., mixed_options())
            .unwrap()
            .certificate
            .unwrap();
        let complete = mixed_coverage!(&body, 9.99, 1e-6, vec![mixed])
            .unwrap()
            .certificate
            .unwrap();
        assert_eq!(complete.mixed_pair_certificates().len(), 1);
        let mixed = crate::source_wall_pair::qualify(&body, [0, 2], 100., mixed_options())
            .unwrap()
            .certificate
            .unwrap();
        assert!(mixed_coverage!(&body, 100.01, 1e-6, vec![mixed]).is_err());
        let mixed = crate::source_wall_pair::qualify(&body, [0, 2], 100., mixed_options())
            .unwrap()
            .certificate
            .unwrap();
        assert!(mixed_coverage!(&body, 9.99, 1e-5, vec![mixed]).is_err());
        let mixed = crate::source_wall_pair::qualify(&other, [0, 2], 100., mixed_options())
            .unwrap()
            .certificate
            .unwrap();
        assert!(mixed_coverage!(&body, 9.99, 1e-6, vec![mixed]).is_err());
        let first = crate::source_wall_pair::qualify(&body, [0, 2], 100., mixed_options())
            .unwrap()
            .certificate
            .unwrap();
        let reversed = crate::source_wall_pair::qualify(&body, [2, 0], 100., mixed_options())
            .unwrap()
            .certificate
            .unwrap();
        assert!(mixed_coverage!(&body, 9.99, 1e-6, vec![first, reversed]).is_err());
        let foreign = crate::source_retained_face_gap::qualify(&other, [0, 1], 29.99, options())
            .unwrap()
            .certificate
            .unwrap();
        assert!(
            crate::source_wall_coverage::qualify_with_retained_gap_facts(
                &body,
                9.99,
                coverage_options(),
                Vec::new(),
                no_products(),
                no_factors(),
                vec![foreign]
            )
            .is_err()
        );
        let pass = crate::source_wall_coverage::RetainedGapLimits {
            cells: 1,
            spans: 2,
            domain_cells: 1,
            cells_per_pair: 1,
            spans_per_pair: 2,
            domain_cells_per_pair: 1,
            domain_cells_per_query: 1,
            tolerance_uv: 1e-8,
        };
        let bounded = crate::source_wall_coverage::qualify_with_retained_gap_pass(
            &body,
            30.01,
            coverage_options(),
            Vec::new(),
            no_products(),
            no_factors(),
            Vec::new(),
            Some(pass),
        )
        .unwrap();
        assert!(bounded.certificate.is_none());
        let extra = bounded.retained_gap.unwrap();
        assert_eq!(extra.cells, 1);
        assert!(extra.spans <= 2);
        assert!(extra.domain_cells <= 1);
        assert_eq!(extra.pairs.len(), 1);
        assert!(!extra.pairs[0].qualified);
        assert!(!extra.pairs[0].unresolved_products.is_empty());
        let mixed_pass = crate::source_wall_coverage::MixedPairLimits {
            domains: pass,
            normal_spans: 1,
            normal_spans_per_pair: 1,
        };
        let limited = crate::source_wall_coverage::qualify_with_mixed_pair_pass(
            &body,
            30.01,
            coverage_options(),
            Vec::new(),
            no_products(),
            no_factors(),
            Vec::new(),
            None,
            Vec::new(),
            Some(mixed_pass),
        )
        .unwrap();
        assert!(limited.certificate.is_none());
        for old in bounded.pairs.iter().filter(|p| p.proven) {
            assert!(
                limited
                    .pairs
                    .iter()
                    .find(|p| p.faces == old.faces)
                    .unwrap()
                    .proven
            );
        }
        let diagnostics = limited.mixed.unwrap();
        assert_eq!(diagnostics.pairs.len(), 1);
        assert_eq!(diagnostics.cells, 1);
        assert!(diagnostics.spans <= 2);
        assert!(diagnostics.domain_cells <= 1);
        assert!(diagnostics.normal_spans <= 1);
        assert!(!diagnostics.pairs[0].retained.qualified);
        assert!(!diagnostics.pairs[0].retained.unresolved_products.is_empty());
        let mut invalid_mixed = mixed_pass;
        invalid_mixed.normal_spans_per_pair = 0;
        assert!(
            crate::source_wall_coverage::qualify_with_mixed_pair_pass(
                &body,
                9.99,
                coverage_options(),
                Vec::new(),
                no_products(),
                no_factors(),
                Vec::new(),
                None,
                Vec::new(),
                Some(invalid_mixed)
            )
            .is_err()
        );
        let mut invalid = pass;
        invalid.cells_per_pair = 0;
        assert!(
            crate::source_wall_coverage::qualify_with_retained_gap_pass(
                &body,
                9.99,
                coverage_options(),
                Vec::new(),
                no_products(),
                no_factors(),
                Vec::new(),
                Some(invalid)
            )
            .is_err()
        );
        let reversed =
            crate::source_retained_face_gap::qualify(&body, [1, 0], 29.99, options()).unwrap();
        assert!(reversed.certificate.is_some());
        let refused =
            crate::source_retained_face_gap::qualify(&body, [0, 1], 30.01, options()).unwrap();
        assert!(refused.certificate.is_none());
        assert!(!refused.unresolved_products.is_empty());
        assert_eq!(refused.cells, 8);
        assert!(refused.spans <= 16);
        assert!(refused.domain_cells <= 1);
        let repeated = crate::source_retained_face_gap::qualify(
            &body,
            [0, 1],
            30.01,
            crate::source_retained_face_gap::Limits {
                cells: 8,
                spans: 16,
                domain_cells: 10000,
                domain_cells_per_query: 1000,
                tolerance_uv: 1e-8,
            },
        )
        .unwrap();
        assert!(repeated.certificate.is_none());
        assert!(
            repeated.domain_queries < 2 * repeated.cells,
            "Repeated rectangles should reuse bounded classifications: {} queries for {} cells",
            repeated.domain_queries,
            repeated.cells
        );
        assert!(crate::source_retained_face_gap::qualify(&body, [0, 0], 29.99, options()).is_err());
        assert!(crate::source_retained_face_gap::qualify(&body, [0, 6], 29.99, options()).is_err());
        let mut invalid = options();
        invalid.domain_cells = 0;
        assert!(crate::source_retained_face_gap::qualify(&body, [0, 1], 29.99, invalid).is_err());
    }
    #[test]
    fn original_cuboid_whole_wall_coverage_requires_all_pairs() {
        let model = crate::cuboid([0., 0., 0.], [10., 20., 30.]).unwrap();
        let shell = prepare(&model, limits())
            .unwrap()
            .shell
            .expect("original cuboid incidence");
        let admission = crate::source_shell_geometry::qualify(shell, geometry_limits()).unwrap();
        let geometry = admission.geometry.expect(admission.reason);
        let volume = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.02,
                tolerance_uv: 1e-8,
                cells: 100000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap();
        let body = volume.body.expect(volume.reason);
        if let Some(path) = std::env::var_os("CAD_CUBOID_SOURCE_BODY_OUTPUT") {
            std::fs::write(
                path,
                value_codec::to_string_pretty(&body.definition().unwrap()).unwrap(),
            )
            .unwrap();
        }
        for (minimum, pairs, normal_spans, success) in [
            (9.99, 21, 10000, true),
            (10.01, 21, 10000, false),
            (9.99, 20, 10000, false),
            (9.99, 21, 6, true),
            (9.99, 21, 1, false),
        ] {
            let report = crate::source_wall_coverage::qualify(
                &body,
                minimum,
                crate::source_wall_coverage::Limits {
                    pairs,
                    plane_controls: 10000,
                    normal_spans,
                    gap: crate::source_face_gap::Limits {
                        cells: 10000,
                        spans: 20000,
                    },
                    max_sine_squared: 1e-6,
                },
            )
            .unwrap();
            eprintln!(
                "original cuboid wall minimum={minimum} reason={} pairs={}/{} cells={} normals={}",
                report.reason,
                report.pairs.len(),
                report.total_pairs,
                report.cells,
                report.normal_spans
            );
            let factored =
                crate::source_wall_coverage::qualify_with_self_facts_and_factored_normals(
                    &body,
                    minimum,
                    crate::source_wall_coverage::Limits {
                        pairs,
                        plane_controls: 10000,
                        normal_spans,
                        gap: crate::source_face_gap::Limits {
                            cells: 10000,
                            spans: 20000,
                        },
                        max_sine_squared: 1e-6,
                    },
                    Vec::new(),
                    crate::source_wall_coverage::NormalProductLimits {
                        cells: 0,
                        cells_per_pair: 0,
                    },
                    crate::source_wall_coverage::FactoredNormalLimits {
                        exact_work: 1,
                        queries: 1,
                        cells: 1,
                        cells_per_pair: 1,
                    },
                )
                .unwrap();
            assert_eq!(factored.certificate.is_some(), report.certificate.is_some());
            assert_eq!(factored.pairs.len(), report.pairs.len());
            assert_eq!(factored.normal_spans, report.normal_spans);
            assert!(factored.factored_exact_work <= 1);
            assert_eq!(factored.factored_queries, 0);
            assert_eq!(factored.factored_product_cells, 0);
            for (old, new) in report.pairs.iter().zip(&factored.pairs) {
                assert_eq!(old.faces, new.faces);
                assert_eq!(old.proven, new.proven);
                assert_eq!(old.reason, new.reason);
            }
            assert_eq!(report.total_pairs, 21);
            assert_eq!(report.enumeration_complete, pairs == 21);
            assert_eq!(report.certificate.is_some(), success, "{}", report.reason);
            assert!(
                report.normal_spans <= normal_spans.min(6),
                "whole-chart normals must be paid once per original face"
            );
            if let Some(certificate) = report.certificate {
                assert!(std::ptr::eq(certificate.body(), &body));
                assert_eq!(certificate.minimum_mm(), minimum);
                assert_eq!(certificate.max_sine_squared(), 1e-6);
                assert!(!certificate.gap_certificates().is_empty());
            }
        }
        for (minimum, angle, accepted) in [
            (9.99, 1e-6, true),
            (10.01, 1e-6, false),
            (9.99, 2e-6, false),
        ] {
            let self_facts = crate::source_wall_self_coverage::qualify(
                &body,
                9.99,
                1e-6,
                crate::source_wall_self_coverage::Limits {
                    plane_controls: 10000,
                    cells: 1,
                    spans: 1,
                    cells_per_face: 1,
                    spans_per_face: 1,
                },
            )
            .unwrap();
            assert!(self_facts.all_self_pairs_qualified());
            assert_eq!(self_facts.certificates().len(), 6);
            let result = crate::source_wall_coverage::qualify_with_self_facts(
                &body,
                minimum,
                crate::source_wall_coverage::Limits {
                    pairs: 21,
                    plane_controls: 10000,
                    normal_spans: 10000,
                    gap: crate::source_face_gap::Limits {
                        cells: 10000,
                        spans: 20000,
                    },
                    max_sine_squared: angle,
                },
                self_facts.into_certificates(),
            );
            if accepted {
                let report = result.unwrap();
                let proof = report
                    .certificate
                    .expect("original self facts plus distinct gaps qualify the cuboid");
                assert_eq!(proof.adaptive_self_certificates().len(), 6);
                assert!(std::ptr::eq(proof.body(), &body));
            } else {
                assert_eq!(
                    result
                        .err()
                        .expect("stale threshold or angle must refuse")
                        .code,
                    "BREP_SOURCE_SELF_WALL_FACT"
                );
            }
        }
        let twin_shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let twin_geometry = crate::source_shell_geometry::qualify(twin_shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
        let twin = crate::source_volume::qualify(
            twin_geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.02,
                tolerance_uv: 1e-8,
                cells: 100000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap()
        .body
        .unwrap();
        assert!(!std::ptr::eq(&body, &twin));
        let extension_limits = crate::source_wall_self_coverage::Limits {
            plane_controls: 10000,
            cells: 1,
            spans: 1,
            cells_per_face: 1,
            spans_per_face: 1,
        };
        let mut extension =
            crate::source_wall_self_coverage::qualify(&body, 9.99, 1e-6, extension_limits).unwrap();
        assert!(
            crate::source_wall_self_coverage::extend_with_pole_facts(
                &twin,
                9.99,
                1e-6,
                &mut extension,
                extension_limits,
                1
            )
            .is_err()
        );
        assert!(
            crate::source_wall_self_coverage::extend_with_pole_facts(
                &body,
                10.01,
                1e-6,
                &mut extension,
                extension_limits,
                1
            )
            .is_err()
        );
        assert!(
            crate::source_wall_self_coverage::extend_with_pole_facts(
                &body,
                9.99,
                2e-6,
                &mut extension,
                extension_limits,
                1
            )
            .is_err()
        );
        assert!(
            crate::source_wall_self_coverage::extend_with_pole_facts(
                &body,
                9.99,
                1e-6,
                &mut extension,
                extension_limits,
                100000001
            )
            .is_err()
        );
        crate::source_wall_self_coverage::extend_with_pole_facts(
            &body,
            9.99,
            1e-6,
            &mut extension,
            extension_limits,
            1,
        )
        .unwrap();
        assert!(extension.all_self_pairs_qualified());
        assert_eq!(
            (
                extension.pole_exact_work,
                extension.pole_cells,
                extension.pole_spans
            ),
            (0, 0, 0)
        );
        assert!(
            crate::source_wall_self_coverage::refine_unqualified(
                &twin,
                9.99,
                1e-6,
                &mut extension,
                extension_limits
            )
            .is_err()
        );
        assert!(
            crate::source_wall_self_coverage::refine_unqualified(
                &body,
                10.01,
                1e-6,
                &mut extension,
                extension_limits
            )
            .is_err()
        );
        assert!(
            crate::source_wall_self_coverage::refine_unqualified(
                &body,
                9.99,
                2e-6,
                &mut extension,
                extension_limits
            )
            .is_err()
        );
        crate::source_wall_self_coverage::refine_unqualified(
            &body,
            9.99,
            1e-6,
            &mut extension,
            extension_limits,
        )
        .unwrap();
        assert!(extension.all_self_pairs_qualified());
        assert_eq!(
            (extension.refinement_cells, extension.refinement_spans),
            (0, 0)
        );
        assert!(
            crate::source_wall_self_coverage::refine_unqualified(
                &body,
                9.99,
                1e-6,
                &mut extension,
                extension_limits
            )
            .is_err()
        );

        for wrong_owner in [true, false] {
            let make_facts = || {
                crate::source_wall_self_coverage::qualify(
                    &body,
                    9.99,
                    1e-6,
                    crate::source_wall_self_coverage::Limits {
                        plane_controls: 10000,
                        cells: 1,
                        spans: 1,
                        cells_per_face: 1,
                        spans_per_face: 1,
                    },
                )
                .unwrap()
                .into_certificates()
            };
            let mut facts = make_facts();
            if !wrong_owner {
                facts.push(make_facts().remove(0));
            }
            let target = if wrong_owner { &twin } else { &body };
            let result = crate::source_wall_coverage::qualify_with_self_facts(
                target,
                9.99,
                crate::source_wall_coverage::Limits {
                    pairs: 21,
                    plane_controls: 10000,
                    normal_spans: 10000,
                    gap: crate::source_face_gap::Limits {
                        cells: 10000,
                        spans: 20000,
                    },
                    max_sine_squared: 1e-6,
                },
                facts,
            );
            assert_eq!(result.err().expect("identical geometry does not grant Body identity; repeated face facts refuse").code,"BREP_SOURCE_SELF_WALL_FACT");
        }
    }
    #[test]
    fn exact_annular_profile_projection_is_globally_injective() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let geometry = crate::source_shell_geometry::qualify(shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
        let body = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 20.,
                tolerance_uv: 1e-8,
                cells: 100000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap()
        .body
        .unwrap();
        let specs = [
            crate::source_profile_body::FaceSpec {
                face: 0,
                anchor: [5, 0],
            },
            crate::source_profile_body::FaceSpec {
                face: 5,
                anchor: [0, 0],
            },
        ];
        let report = crate::source_profile_body::prepare(
            &body,
            &specs,
            crate::source_profile_body::Limits {
                controls: 1000,
                fragments: 100,
                position_tolerance: 1e-7,
            },
        )
        .unwrap();
        let proposal = report.proposal.unwrap();
        let profile = proposal.profile(0).unwrap();
        let r =
            nurbs_core::profile_projection_jacobian::certify(profile, [0, 1], 20000000).unwrap();
        eprintln!(
            "annular exact profile factors={:?} orientation={} work={} exactReason={:?} pending={:?} filtered={} signs={:?}",
            profile.factors(),
            r.certificate.is_some(),
            r.exact_work,
            r.exact_reason,
            r.pending_coefficient,
            r.filtered_coefficients,
            r.signs
        );
        assert!(r.exact_work <= 20000000);
        let orientation = r
            .certificate
            .expect("actual factorized annular profile orientation");
        assert!(std::ptr::eq(orientation.profile(), profile));
        assert!(r.filtered_coefficients > 0);
        let jordan =
            nurbs_core::profile_projected_jordan::certify(profile, [0, 1], 1e-8, 20000000, 10000)
                .unwrap();
        eprintln!(
            "annular exact profile Jordan qualified={} work={} cells={} reason={}",
            jordan.certificate.is_some(),
            jordan.exact_work,
            jordan.boundary_cells,
            jordan.reason
        );
        let certificate = jordan
            .certificate
            .expect("actual factorized annular profile Jordan boundary");
        assert!(std::ptr::eq(certificate.profile(), profile));
        assert_eq!(certificate.axes(), [0, 1]);
    }
    #[test]
    fn adjacent_annular_curved_faces_have_owned_opposite_plane_fibers() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        for ids in [[0, 5], [5, 0]] {
            let report =
                crate::source_paired_fiber_contact::certify(&shell, ids, 10000000, 10000).unwrap();
            eprintln!(
                "adjacent curved source pair={ids:?} qualified={} work={} drivers={} reason={}",
                report.certificate.is_some(),
                report.exact_work,
                report.driver_cells,
                report.reason
            );
            let proof = report.certificate.expect(report.reason);
            assert_eq!(proof.faces(), ids);
            assert_eq!(proof.fibers()[0].plane(), proof.fibers()[1].plane());
            assert_ne!(proof.fibers()[0].side(), proof.fibers()[1].side());
            assert!(!proof.edges().is_empty());
            assert!(report.exact_work <= 10000000 && report.driver_cells <= 10000);
            let selected = ids.iter().position(|&i| i == 0).unwrap();
            let fiber = &proof.fibers()[selected];
            let (profile, _) =
                nurbs_core::normal_alignment::qualify_weight_profile(fiber.surface(), [5, 0], 1000)
                    .unwrap();
            let profile = profile.unwrap();
            let (axis, upper) = fiber.boundary();
            let transfer = crate::source_profile_boundary_identity::certify_plane_image(
                &profile,
                fiber.plane(),
                axis,
                upper,
                10000000,
            )
            .unwrap();
            let transfer = transfer.certificate.expect(transfer.reason);
            assert!(std::ptr::eq(transfer.boundary().profile(), &profile));
            assert_eq!(transfer.image().plane(), fiber.plane());
            assert_eq!(transfer.image().side(), fiber.side());
            assert!(transfer.image().poles().is_empty());
            assert!(
                crate::source_paired_fiber_contact::certify(&shell, ids, 1, 10000)
                    .unwrap()
                    .certificate
                    .is_none()
            );
        }
    }

    #[test]
    fn annular_raw_profile_hull_uses_only_fresh_neighbor_material() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let mut qualified = Vec::new();
        for neighbor in [11, 15, 19] {
            for (ids, raw) in [([0, neighbor], 0), ([neighbor, 0], 1)] {
                let report = crate::source_hull_separation::certify_shell_with_raw_side(
                    &shell, ids, raw, 10000000,
                )
                .unwrap();
                eprintln!(
                    "raw profile hull pair={ids:?} qualified={} work={} reason={}",
                    report.certificate.is_some(),
                    report.exact_work,
                    report.reason
                );
                assert!(report.exact_work <= 10000000);
                if let Some(proof) = report.certificate {
                    assert!(proof.material_hulls()[raw].is_none());
                    assert!(proof.material_hulls()[1 - raw].is_some());
                    assert_ne!(proof.sides()[0], proof.sides()[1]);
                    assert_eq!(
                        proof.regions()[raw].loops()[0][0].surface(),
                        shell.regions().unwrap()[0].loops()[0][0].surface()
                    );
                    qualified.push((neighbor, raw));
                }
                assert!(
                    crate::source_hull_separation::certify_shell_with_raw_side(&shell, ids, raw, 1)
                        .unwrap()
                        .certificate
                        .is_none()
                );
            }
        }
        assert_eq!(
            qualified,
            vec![(11, 0), (11, 1), (15, 0), (15, 1), (19, 0), (19, 1)]
        );
        eprintln!("raw profile hull qualified cases={qualified:?}");
        assert!(
            crate::source_hull_separation::certify_shell_with_raw_side(&shell, [0, 11], 2, 100000)
                .is_err()
        );
    }

    #[test]
    fn annular_profile_boundary_plane_transfer_keeps_exact_source_owner() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let source = shell.regions().unwrap()[0].loops()[0][0].surface();
        let (profile, _) =
            nurbs_core::normal_alignment::qualify_weight_profile(source, [5, 0], 1000).unwrap();
        let profile = profile.unwrap();
        for axis in 0..2 {
            for upper in [false, true] {
                let report =
                    crate::source_profile_boundary_identity::certify(&profile, axis, upper, 100000)
                        .unwrap();
                eprintln!(
                    "actual profile boundary axis={axis} upper={upper} reason={} work={}",
                    report.reason, report.exact_work
                );
                let proof = report.certificate.expect(report.reason);
                assert!(std::ptr::eq(proof.profile(), &profile));
            }
        }
        for neighbor in [1, 5] {
            let original =
                crate::source_pole_planar_contact::certify(&shell, [0, neighbor], 10000000, 10000)
                    .unwrap();
            eprintln!(
                "actual profile source pole neighbor={neighbor} reason={}",
                original.reason
            );
            if neighbor == 5 {
                // Cap support is not qualified by the boundary/pole protocol.
                assert!(original.certificate.is_none());
                continue;
            }
            let original = original.certificate.expect(original.reason);
            assert_eq!(original.fiber().surface(), source);
            let (axis, upper) = original.fiber().boundary();
            let report = crate::source_profile_boundary_identity::certify_plane_image(
                &profile,
                original.fiber().plane(),
                axis,
                upper,
                10000000,
            )
            .unwrap();
            let proof = report.certificate.expect(report.reason);
            assert!(std::ptr::eq(proof.boundary().profile(), &profile));
            assert_eq!(proof.image().poles(), original.fiber().poles());
            assert_eq!(proof.image().side(), original.fiber().side());
            assert!(report.exact_work <= 10000000);
        }
    }

    #[test]
    fn annular_authored_source_radius_baseline() {
        let authored = crate::circular_blend::plane_cylinder_transition(
            20.,
            6.,
            0.,
            1.25,
            0.,
            std::f64::consts::FRAC_PI_2 / 4.,
        )
        .unwrap();
        let report = authored.qualify_radius(1e-6, 100, 100000000).unwrap();
        eprintln!(
            "annular source radius reason={} error={:?} work={}",
            report.reason, report.error_upper, report.work
        );
        let certificate = report
            .certificate
            .expect("original annular radius baseline");
        assert_eq!(certificate.surface(), &authored.surface);
        assert_eq!(certificate.centers(), &authored.centers);
    }

    #[test]
    fn retained_profile_recipe_replays_native_admission() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let geometry = crate::source_shell_geometry::qualify(shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
        let body = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 20.,
                tolerance_uv: 1e-8,
                cells: 100000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap()
        .body
        .unwrap();
        let work = || crate::source_profile_body::Limits {
            controls: 1000,
            fragments: 100,
            position_tolerance: 1e-7,
        };
        let original = crate::source_profile_body::prepare(
            &body,
            &[crate::source_profile_body::FaceSpec {
                face: 0,
                anchor: [5, 0],
            }],
            work(),
        )
        .unwrap()
        .proposal
        .unwrap();
        let recipe = original.definition().unwrap();
        let rebuilt = crate::source_profile_body::restore(&body, recipe.clone(), work())
            .unwrap()
            .proposal
            .unwrap();
        assert_eq!(rebuilt.definition().unwrap(), recipe);
        assert!(std::ptr::eq(rebuilt.original_body(), &body));
        let limited = crate::source_profile_body::restore(
            &body,
            recipe.clone(),
            crate::source_profile_body::Limits {
                controls: 0,
                fragments: 0,
                position_tolerance: 1e-7,
            },
        )
        .unwrap();
        assert!(limited.proposal.is_none());
        for field in [
            "originalBody",
            "profiles",
            "positionTolerance",
            "version",
            "kind",
        ] {
            let mut changed = recipe.clone();
            changed[field] = value_codec::Value::Null;
            assert!(
                crate::source_profile_body::restore(&body, changed, work()).is_err(),
                "{field}"
            );
        }
        let mut forged = recipe.clone();
        forged["profiles"][0]["face"] = value_codec::json!(9999);
        assert!(crate::source_profile_body::restore(&body, forged, work()).is_err());
        let mut forged = recipe.clone();
        forged["certificate"] = value_codec::json!({"complete":true});
        let limited = crate::source_profile_body::restore(
            &body,
            forged,
            crate::source_profile_body::Limits {
                controls: 0,
                fragments: 0,
                position_tolerance: 1e-7,
            },
        )
        .unwrap();
        assert!(limited.proposal.is_none());
        let a = rebuilt
            .cut_endpoint(0, 0, 0.75, true, 1, 16)
            .unwrap()
            .endpoint
            .unwrap();
        let b = rebuilt
            .cut_endpoint(0, 0, 0.9, true, 1, 16)
            .unwrap()
            .endpoint
            .unwrap();
        let pa = a.split_source_use(100).unwrap().partition.unwrap();
        let pb = b.split_source_use(100).unwrap().partition.unwrap();
        let seam = rebuilt
            .qualify_root_ended_seam([&pa, &pb], 1e-6, 100000, 100000)
            .unwrap()
            .seam
            .unwrap();
        let seam_recipe = seam.definition().unwrap();
        let replay_work = || crate::source_profile_body::RootSeamReplayLimits {
            cut_spans: 32,
            mapping_cells: 200,
            max_sine_squared: 1e-6,
            angular_cells: 100000,
            seam_spans: 100000,
        };
        let replay = crate::source_profile_body::restore_root_ended_seam(
            &rebuilt,
            seam_recipe.clone(),
            replay_work(),
        )
        .unwrap();
        let replayed = replay.seam.expect(replay.reason);
        assert_eq!(replayed.definition().unwrap(), seam_recipe);
        assert_eq!(
            replayed.restricted().canonical_interval(),
            seam.restricted().canonical_interval()
        );
        assert!(std::ptr::eq(replayed.restricted().proposal(), &rebuilt));
        let mut zero = replay_work();
        zero.cut_spans = 0;
        assert!(
            crate::source_profile_body::restore_root_ended_seam(
                &rebuilt,
                seam_recipe.clone(),
                zero
            )
            .unwrap()
            .seam
            .is_none()
        );
        let mut zero = replay_work();
        zero.mapping_cells = 0;
        assert!(
            crate::source_profile_body::restore_root_ended_seam(
                &rebuilt,
                seam_recipe.clone(),
                zero
            )
            .unwrap()
            .seam
            .is_none()
        );
        for field in ["proposal", "ends", "version", "kind"] {
            let mut changed = seam_recipe.clone();
            changed[field] = value_codec::Value::Null;
            assert!(
                crate::source_profile_body::restore_root_ended_seam(
                    &rebuilt,
                    changed,
                    replay_work()
                )
                .is_err(),
                "{field}"
            );
        }
        let contour_replay = crate::source_profile_body::restore_incident_contours(
            &rebuilt,
            seam_recipe.clone(),
            replay_work(),
            100,
            100000000,
            100000,
        )
        .unwrap();
        let replayed_contours = contour_replay.assembly.expect(contour_replay.reason);
        let original_contours = seam
            .propose_incident_contours(100)
            .unwrap()
            .proposal
            .unwrap();
        for (actual, expected) in replayed_contours
            .contours
            .contours()
            .iter()
            .zip(original_contours.contours())
        {
            assert_eq!(actual.face(), expected.face());
            assert_eq!(actual.middle(), expected.middle());
            for (wire, expected_wire) in actual.loops().iter().zip(expected.loops()) {
                assert_eq!(wire.len(), expected_wire.len());
                for (fragment, expected_fragment) in wire.iter().zip(expected_wire) {
                    assert_eq!(fragment.definition(), expected_fragment.definition());
                }
                assert!((0..wire.len()).all(|i| wire[i].joins(&wire[(i + 1) % wire.len()])));
            }
        }
        let original_edge = seam
            .qualify_incident_edge(100000000, 100000)
            .unwrap()
            .edge
            .unwrap();
        assert_eq!(
            replayed_contours.edge.definition(),
            original_edge.definition()
        );
        assert!(
            crate::source_profile_body::restore_incident_contours(
                &rebuilt,
                seam_recipe.clone(),
                replay_work(),
                0,
                100000000,
                100000
            )
            .unwrap()
            .assembly
            .is_none()
        );
        assert!(
            crate::source_profile_body::restore_incident_contours(
                &rebuilt,
                seam_recipe.clone(),
                replay_work(),
                100,
                1,
                1
            )
            .unwrap()
            .assembly
            .is_none()
        );
        let mut duplicate = seam_recipe.clone();
        duplicate["ends"][1] = duplicate["ends"][0].clone();
        assert!(
            crate::source_profile_body::restore_root_ended_seam(&rebuilt, duplicate, replay_work())
                .unwrap()
                .seam
                .is_none()
        );
        let mut mismatched = seam_recipe.clone();
        mismatched["ends"][1]["highFreeEnd"] = value_codec::json!(false);
        assert!(
            crate::source_profile_body::restore_root_ended_seam(
                &rebuilt,
                mismatched,
                replay_work()
            )
            .unwrap()
            .seam
            .is_none()
        );
        let mut forged = seam_recipe.clone();
        forged["certificate"] = value_codec::json!({"complete":true});
        let mut zero = replay_work();
        zero.seam_spans = 0;
        assert!(
            crate::source_profile_body::restore_root_ended_seam(&rebuilt, forged, zero)
                .unwrap()
                .seam
                .is_none()
        );
        let mut bounds = a.definition();
        bounds["worldBounds"] = value_codec::json!([[999., 999.], [999., 999.], [999., 999.]]);
        let fresh = crate::source_profile_body::restore_cut_endpoint(&rebuilt, bounds, 16)
            .unwrap()
            .endpoint
            .unwrap();
        assert_eq!(fresh.world_bounds(), a.world_bounds());
        let bridge_recipe = value_codec::json!({"version":1,"kind":"exact-profile-section-bridge",
            "proposal":recipe,"sections":[{"face":0,"axis":0,"parameter":0.75},
            {"face":0,"axis":0,"parameter":0.9}],"scales":[0.9-0.75,0.9-0.75]});
        let restored_bridge =
            crate::source_profile_body::restore_section_bridge(&rebuilt, bridge_recipe.clone())
                .unwrap();
        let authored = nurbs_core::normal_alignment::ProfileSectionBridge::new(
            original.profile(0).unwrap().section(0, 0.75).unwrap(),
            original.profile(0).unwrap().section(0, 0.9).unwrap(),
            [0.9 - 0.75; 2],
        )
        .unwrap();
        let left_a = rebuilt
            .cut_endpoint(0, 0, 0.75, false, 0, 16)
            .unwrap()
            .endpoint
            .unwrap();
        let left_b = rebuilt
            .cut_endpoint(0, 0, 0.9, false, 0, 16)
            .unwrap()
            .endpoint
            .unwrap();
        let left_pa = left_a.split_source_use(100).unwrap().partition.unwrap();
        let left_pb = left_b.split_source_use(100).unwrap().partition.unwrap();
        let left = rebuilt
            .qualify_root_ended_seam([&left_pa, &left_pb], 1e-6, 100000, 100000)
            .unwrap()
            .seam
            .unwrap();
        let constrained = restored_bridge
            .constrain_original_rails(1000)
            .unwrap()
            .0
            .unwrap();
        let left_side = left
            .qualify_transition_side(&constrained, 1000, 1000)
            .unwrap()
            .side
            .unwrap();
        let right_side = seam
            .qualify_transition_side(&constrained, 1000, 1000)
            .unwrap()
            .side
            .unwrap();
        let boundary =
            crate::source_profile_body::propose_transition_boundary([&left_side, &right_side])
                .unwrap();
        let boundary_recipe = boundary.definition().unwrap();
        let boundary_work = || crate::source_profile_body::TransitionBoundaryReplayLimits {
            seam: replay_work(),
            controls: 1000,
            side_cells: 1000,
            side_spans: 1000,
        };
        let replay=crate::source_profile_body::with_restored_transition_boundary(&rebuilt,boundary_recipe.clone(),
            boundary_work(),|fresh| {
                assert_eq!(fresh.definition().unwrap(),boundary_recipe);
                for edge in 0..4 {
                    assert_eq!(fresh.enclose_edge(edge,[0.,1.],100).unwrap(),boundary.enclose_edge(edge,[0.,1.],100).unwrap());
                }
                for high in [false,true] {
                    assert!(fresh.replay_section_contours(high,0,200,100).unwrap().section.is_none());
                    assert!(fresh.replay_section_contours(high,32,0,100).unwrap().section.is_none());
                    assert!(fresh.replay_section_contours(high,32,200,0).unwrap().section.is_none());
                    let section=fresh.replay_section_contours(high,32,200,100).unwrap();
                    let section=section.section.expect(section.reason);
                    assert!(std::ptr::eq(section.boundary(),fresh));
                    let joined=section.qualify_join(1000,1000,10000).unwrap();
                    assert!(joined.join.is_some(),"{}",joined.reason);
                    let exterior=section.qualify_exterior(1000,1000).unwrap();
                    let exterior=exterior.region.expect(exterior.reason);
                    let retained=exterior.qualify_face(1,16,256,2048,12288).unwrap();
                    assert!(retained.face.is_some(),"{}",retained.reason);
                    assert!(exterior.qualify_face(1,16,255,2048,12288).unwrap().face.is_none());
                }
                let jet=restored_bridge.derivative_bounds([0.5;2],[0.5;2],8).unwrap().0.unwrap();
                let vectors=jet.map(|r|r.map(|b|b[0]*0.5+b[1]*0.5));
                let determinant=vectors[0][0]*vectors[1][1]-vectors[1][0]*vectors[0][1];
                let projection=[[vectors[1][1]/determinant,-vectors[1][0]/determinant,0.],
                    [-vectors[0][1]/determinant,vectors[0][0]/determinant,0.]];
                // The candidate matrix grants nothing: complete interval coverage below does.
                let face=fresh.qualify_face_from_rails(projection,1000,2000,100000).unwrap();
                assert!(face.face.is_some(),"{}",face.reason);
                assert!(face.interior.as_ref().unwrap().complete_regular);
                assert!(fresh.qualify_face_from_rails(projection,1000,0,0).unwrap().face.is_none());
                let sections=[fresh.replay_section_contours(false,32,200,100).unwrap().section.unwrap(),
                    fresh.replay_section_contours(true,32,200,100).unwrap().section.unwrap()];
                let exteriors=sections.each_ref().map(|s|s.qualify_exterior(1000,1000).unwrap().region.unwrap());
                let retained=exteriors.each_ref().map(|e|e.qualify_face(1,16,256,2048,12288).unwrap().face.unwrap());
                let face=face.face.unwrap();
                let rails=face.qualify_rail_incidence(100000000,100000,100).unwrap().incidence.unwrap();
                let patch=rails.qualify_sections([&exteriors[0],&exteriors[1]]).unwrap().patch.unwrap();
                assert!(patch.qualify_outer_edges([&retained[0],&retained[1]],1,1).unwrap().assembly.is_none());
                let outer=patch.qualify_outer_edges([&retained[0],&retained[1]],100000000,100000).unwrap().assembly.unwrap();
                assert_eq!(outer.edges().len(),4);assert_eq!(outer.retained_factored(),[None,Some(1)]);
                assert!(outer.assemble_shell(0).unwrap().shell.is_none());
                let assembled=outer.assemble_shell(1000).unwrap();
                let shell=assembled.shell.expect(assembled.reason);
                assert_eq!(shell.faces().len(),29);assert_eq!(shell.components(),1);
                assert!(outer.assemble_shell(assembled.uses-1).unwrap().shell.is_none());
                for pair in shell.pairs() {
                    assert_ne!(pair.uses()[0],pair.uses()[1]);
                }
                let region=face.qualify_profile_xy_region(1e-8,20000000,10000).unwrap().certificate.unwrap();
                let audit_limits=crate::source_profile_body::ContactAuditLimits{cells:16384,cells_per_pair:8,
                    controls_per_enclosure:1000,spans:100000,domain_cells:100000,domain_cells_per_pair:256,
                    exact_work:100000000,driver_cells:100000,tolerance_uv:1e-8};
                let contacts=shell.inspect_reconstructed_contacts(audit_limits,&region,10000,256,32).unwrap();
                assert_eq!(contacts.pairs().len(),406);
                assert!(std::ptr::eq(contacts.owners(),shell.faces()));
                let (cells,spans,domains,work,drivers)=contacts.work();
                assert!(cells<=audit_limits.cells && spans<=audit_limits.spans && domains<=audit_limits.domain_cells
                    && work<=audit_limits.exact_work && drivers<=audit_limits.driver_cells);
                eprintln!("restored contact pipeline pending={} work={:?}",contacts.pending_pairs(),contacts.work());
                assert_eq!(shell.qualify_embedding(&contacts).is_some(),contacts.pending_pairs()==0);
                assert_eq!(contacts.pending_pairs(),0,"restored fixture must requalify all contacts");
                let geometry=shell.qualify_embedding(&contacts).unwrap();
                let volume_limits=|cells|crate::source_profile_body::ExpressionVolumeLimits {
                    axis:2,origin:0.,absolute_error:20.,tolerance_uv:1e-8,controls_per_enclosure:1000,
                    cells,spans:200000,domain_cells:100000,domain_cells_per_rectangle:256};
                assert!(geometry.qualify_volume(volume_limits(0)).unwrap().certificate.is_none());
                let volume=geometry.qualify_volume(volume_limits(32768)).unwrap();
                let proof=volume.certificate.expect(volume.report.reason);
                assert!(std::ptr::eq(proof.geometry(),&geometry));
                let bounds=proof.positive_bounds();
                assert!(bounds[0]>0. && bounds[1]-bounds[0]<=20.);
                eprintln!("restored volume bounds={bounds:?} cells={} spans={}",volume.report.cells,volume.report.spans);
                if let Ok(path)=std::env::var("BREP_RESTORED_VOLUME_PATH") {
                    std::fs::write(path,value_codec::json!({"complete":volume.report.complete,
                        "signedBounds":proof.signed_bounds(),"positiveBounds":bounds,
                        "cells":volume.report.cells,"spans":volume.report.spans,"domainCells":volume.report.domain_cells,
                        "reason":volume.report.reason}).to_string()).unwrap();
                }

                if let Ok(path)=std::env::var("BREP_RESTORED_CONTACT_PATH") {
                    std::fs::write(path,contacts.diagnostics().to_string()).unwrap();
                }
                Ok(fresh.corners().iter().map(|p|p.definition()).collect::<Vec<_>>())
            }).unwrap();
        assert!(replay.result.is_some());
        let jets=restored_bridge.derivative_bounds([0.5;2],[0.5;2],8).unwrap().0.unwrap();
        let vectors=jets.map(|r|r.map(|b|b[0]*0.5+b[1]*0.5));
        let determinant=vectors[0][0]*vectors[1][1]-vectors[1][0]*vectors[0][1];
        let projection=[[vectors[1][1]/determinant,-vectors[1][0]/determinant,0.],
            [-vectors[0][1]/determinant,vectors[0][0]/determinant,0.]];
        let shell_work=||crate::source_profile_body::ShellReplayLimits{
            boundary:boundary_work(),projection,cut_spans_per_section:32,mapping_cells_per_section:200,
            fragments_per_section:100,controls:1000,join_cells:1000,join_spans:10000,exterior_drivers:1000,
            collapsed_ends:[1,1],subdivisions:16,quotient_cells:256,retained_cells:2048,retained_spans:12288,
            transition_cells:2000,transition_spans:100000,exact_work_per_rail:100000000,
            drivers_per_rail:100000,fragments_per_rail:100,outer_exact_work:100000000,outer_drivers:100000,shell_uses:1000};
        if let Ok(path)=std::env::var("BREP_EXPRESSION_REPLAY_REQUEST_PATH") {
            let request=value_codec::json!({"op":"cad_expression_shell_restore",
                "proposalDefinition":rebuilt.definition().unwrap(),"boundaryDefinition":boundary_recipe,
                "profileLimits":{"controls":1000,"fragments":100,"positionTolerance":1e-7},
                "sourceLimits":{"shell":{"exactWork":100000000,"driverCells":100000,"regions":{
                    "region":{"pairs":10000,"regionCells":10000,"domainCells":10000,"agreementCells":10000},
                    "searchCells":100000,"pointChecks":100000,"mappingCells":100000,"driverCells":100000,
                    "membershipCells":100000,"controls":100000,"windingCells":100000,"steps":64}},
                    "embedding":{"toleranceUv":1e-8,"corners":10000,"spans":100000,"linearCells":100000,
                        "exactWork":100000000,"driverCells":100000,"pairs":{"pairs":10000,"cells":10000,
                            "domainCells":100000,"cellsPerPair":32,"domainCellsPerPair":512}},
                    "volume":{"axis":2,"origin":0.,"absoluteError":20.,"toleranceUv":1e-8,
                        "cells":100000,"spans":100000,"domainCells":1000000}},
                "shellLimits":{"boundary":{"seam":{"cutSpans":32,"mappingCells":200,"maxSineSquared":1e-6,
                        "angularCells":100000,"seamSpans":100000},"controls":1000,"sideCells":1000,"sideSpans":1000},
                    "projection":projection,"cutSpansPerSection":32,"mappingCellsPerSection":200,"fragmentsPerSection":100,
                    "controls":1000,"joinCells":1000,"joinSpans":10000,"exteriorDrivers":1000,"collapsedEnds":[1,1],
                    "subdivisions":16,"quotientCells":256,"retainedCells":2048,"retainedSpans":12288,
                    "transitionCells":2000,"transitionSpans":100000,"exactWorkPerRail":100000000,
                    "driversPerRail":100000,"fragmentsPerRail":100,"outerExactWork":100000000,
                    "outerDrivers":100000,"shellUses":1000}});
            std::fs::write(path,request.to_string()).unwrap();
        }
        let single_call=crate::source_profile_body::with_restored_expression_shell(&rebuilt,boundary_recipe.clone(),
            shell_work(),|shell| {
                let mut ids=std::collections::BTreeSet::new();
                for (face,f) in shell.faces().iter().enumerate() {for (wire,w) in f.loops().iter().enumerate() {
                    for edge in 0..w.len() {
                        let a=crate::source_shell_incidence::Address{face,wire,edge};
                        let next=crate::source_shell_incidence::Address{edge:(edge+1)%w.len(),..a};
                        ids.insert(shell.boundary_vertex(a,0).unwrap());
                        assert_eq!(shell.boundary_vertex(a,1),shell.boundary_vertex(next,0));
                        assert!(shell.boundary_vertex(a,2).is_none());
                    }
                }}
                let boundary_mesh=shell.prepare_source_boundary_mesh(crate::source_shared_edge::BoundaryMeshLimits {
                    max_deviation_mm:0.05,cells:4096,spans:100000,max_depth:24}).unwrap();
                assert_eq!(boundary_mesh.partitions.len(),shell.pairs().len());
                assert!(boundary_mesh.cells<=4096 && boundary_mesh.spans<=100000);
                for partition in &boundary_mesh.partitions {
                    let a=shell.pairs()[partition.pair].uses()[0];
                    assert_eq!(partition.endpoints,[shell.boundary_vertex(a,0).unwrap(),shell.boundary_vertex(a,1).unwrap()]);
                    if let Some(mesh)=&partition.mesh {for segment in &mesh.segments {assert!(segment.deviation_upper_mm<=0.05);}}
                }
                assert_eq!(boundary_mesh.partitions.iter().filter(|p|p.mesh.is_none()).count(),0);
                assert_eq!(boundary_mesh.partitions.iter().filter(|p|p.native_chart_uses==[false,true]).count(),2);
                for partition in &boundary_mesh.partitions {
                    let pair=&shell.pairs()[partition.pair];
                    if let crate::source_profile_body::ShellPairKind::Section(end) = pair.kind() {
                        let mesh=partition.mesh.as_ref().unwrap();
                        let charts=partition.shell_uv.as_ref().unwrap();
                        assert_eq!(charts.len(),mesh.vertices.len());
                        for (uv,vertex) in charts.iter().zip(&mesh.vertices) {
                            assert_eq!(uv[1][1],[end as f64;2]);
                            let (position,_)=shell.enclose_support(pair.uses()[1].face,uv[1],1000,1000)?;
                            let position=position.expect("section transition enclosure");
                            for axis in 0..3 {assert!(position[axis][0]<=vertex.world[axis][1] && vertex.world[axis][0]<=position[axis][1]);}
                        }
                    }
                    if let crate::source_profile_body::ShellPairKind::Rail(_) = pair.kind() {
                        let mapped=partition.shell_uv.as_ref().unwrap();
                        let mesh=partition.mesh.as_ref().unwrap();
                        assert_eq!(mapped.len(),mesh.vertices.len());
                        for (uv,vertex) in mapped.iter().zip(&mesh.vertices) {
                            assert!(uv[0][1][0]>=0. && uv[0][1][1]<=1.);
                            let (position,_)=shell.enclose_support(pair.uses()[0].face,uv[0],1000,1000)?;
                            let position=position.expect("transition rail UV enclosure");
                            for axis in 0..3 {assert!(position[axis][0]<=vertex.world[axis][1] && vertex.world[axis][0]<=position[axis][1]);}
                        }
                        let b=pair.uses()[1];
                        let ids=[shell.boundary_vertex(b,0).unwrap(),shell.boundary_vertex(b,1).unwrap()];
                        assert!(partition.mesh_endpoints==ids || partition.mesh_endpoints==[ids[1],ids[0]]);
                    } else {assert_eq!(partition.mesh_endpoints,partition.endpoints);}
                }
                let graph=shell.prepare_boundary_graph(crate::source_shared_edge::BoundaryMeshLimits {
                    max_deviation_mm:0.05,cells:4096,spans:100000,max_depth:24})?;
                assert!(graph.vertices.len()>=shell.vertices());
                assert!(graph.vertices[..shell.vertices()].iter().all(|v|v.point.is_some()));
                let loops=shell.order_boundary_loops(&graph);
                assert_eq!(loops.len(),shell.faces().iter().map(|f|f.loops().len()).sum::<usize>());
                assert!(loops.iter().any(|l|l.pending.is_empty()));
                for loop_ in loops.iter().filter(|l|l.pending.is_empty()) {
                    let ends=|s:&crate::source_profile_body::BoundaryLoopSegment| {
                        let v=graph.segments[s.segment].vertices;if s.reversed {[v[1],v[0]]}else{v}
                    };
                    for i in 0..loop_.segments.len(){assert_eq!(ends(&loop_.segments[i])[1],ends(&loop_.segments[(i+1)%loop_.segments.len()])[0]);}
                }
                let candidates=shell.propose_boundary_face_triangles(&graph);
                let mut bounded=0;
                for triangle in candidates.triangles.iter().take(25) {
                    let (error,spent)=shell.bound_candidate_triangle(triangle,&graph,1000,1000)?;
                    assert!(spent<=1000);
                    if let Some(error)=error {assert!(error.is_finite() && error>=0.);bounded+=1;}
                    assert!(shell.bound_candidate_triangle(triangle,&graph,1000,0)?.0.is_none());
                    assert_eq!(shell.bound_candidate_triangle_rectangle_center(triangle,&graph,1000,0)?,(None,0));
                    assert_eq!(shell.bound_candidate_triangle_partitioned(triangle,&graph,1000,0)?,(None,0));
                }
                assert!(bounded>0);
                let incidence_report=candidates.inspect_incidence();
                assert_eq!(incidence_report.orientation_conflicts,0);
                assert_eq!(incidence_report.duplicate_triangles,0);
                assert_eq!(incidence_report.degenerate_triangles,0);
                eprintln!("candidate topology triangles={} pendingFaces={} openEdges={} nonmanifoldEdges={} invalidVertexLinks={}",
                    candidates.triangles.len(),candidates.pending.len(),incidence_report.open_edges,
                    incidence_report.nonmanifold_edges,incidence_report.invalid_vertex_links);
                assert!(!candidates.triangles.is_empty());
                let admitted_faces=candidates.triangles.iter().map(|t|t.face).collect::<std::collections::BTreeSet<_>>();
                let pending_faces=candidates.pending.iter().map(|p|p.face).collect::<std::collections::BTreeSet<_>>();
                assert!(admitted_faces.is_disjoint(&pending_faces));
                assert_eq!(admitted_faces.len()+pending_faces.len(),shell.faces().len());
                for &face in &admitted_faces {
                    let mut incidence=std::collections::BTreeMap::new();
                    for t in candidates.triangles.iter().filter(|t|t.face==face) {
                        assert!(t.vertices.iter().all(|&id|graph.vertices[id].point.is_some()));
                        for i in 0..3 {let a=t.vertices[i];let b=t.vertices[(i+1)%3];*incidence.entry((a.min(b),a.max(b))).or_insert(0usize)+=1;}
                    }
                    for loop_ in loops.iter().filter(|l|l.face==face) {for constraint in &loop_.segments {
                        let [a,b]=graph.segments[constraint.segment].vertices;
                        assert_eq!(incidence.get(&(a.min(b),a.max(b))),Some(&1));
                    }}
                }
                assert!(!graph.segments.is_empty());
                for segment in &graph.segments {
                    assert!(segment.deviation_upper_mm<=0.05);
                    assert!(segment.vertices.iter().all(|&v|graph.vertices[v].point.is_some()));
                }
                assert!(graph.pending.iter().all(|p|p.reason!="boundary-shared-endpoint-enclosure-conflict"));
                assert_eq!(ids.len(),shell.vertices());
                for pair in shell.pairs() {let [a,b]=pair.uses();
                    assert_eq!(shell.boundary_vertex(a,0),shell.boundary_vertex(b,1));
                    assert_eq!(shell.boundary_vertex(a,1),shell.boundary_vertex(b,0));
                }
                for &pole in shell.poles() {assert_eq!(shell.boundary_vertex(pole,0),shell.boundary_vertex(pole,1));}
                let mut refinement_graph=graph;
                let before_disabled=refinement_graph.vertices.len();
                let disabled_candidates=shell.propose_boundary_face_triangles(&refinement_graph);
                let disabled_count=disabled_candidates.triangles.len();
                let disabled=shell.bisect_candidate_deviation_edges(&mut refinement_graph,disabled_candidates,100,1000,10000,0.05,0)?;
                assert_eq!((disabled.splits,disabled.spans,disabled.deviation_queries),(0,0,0));
                assert_eq!(disabled.candidates.triangles.len(),disabled_count);
                assert_eq!(refinement_graph.vertices.len(),before_disabled);
                assert!(disabled.candidates.inspect_incidence().closed_oriented());
                let disabled_trim=shell.bisect_candidate_trimmed_edges(&mut refinement_graph,disabled.candidates,crate::source_profile_body::AdaptiveInteriorLimits {
                    max_splits:100,controls:1000,spans:10000,spans_per_query:0,target_mm:0.05,
                    trim_cells:10000,trim_cells_per_query:64,tolerance_uv:1e-8})?;
                assert_eq!((disabled_trim.splits,disabled_trim.spans,disabled_trim.deviation_queries,disabled_trim.trim_queries,disabled_trim.trim_cells),(0,0,0,0,0));
                assert_eq!(refinement_graph.vertices.len(),before_disabled);
                assert!(disabled_trim.candidates.inspect_incidence().closed_oriented());
                let interior_splits=std::env::var("CAD_BOUNDARY_INTERIOR_SPLITS").ok()
                    .map(|v|v.parse::<usize>().expect("finite native split count")).unwrap_or(32);
                let trim_adaptive=std::env::var("CAD_BOUNDARY_TRIM_ADAPTIVE").is_ok();
                let adaptive=trim_adaptive || std::env::var("CAD_BOUNDARY_ADAPTIVE").is_ok();
                let bisection=if trim_adaptive {shell.bisect_candidate_trimmed_edges(&mut refinement_graph,candidates,crate::source_profile_body::AdaptiveInteriorLimits {
                    max_splits:interior_splits,controls:1000,spans:100000,spans_per_query:64,target_mm:0.05,
                    trim_cells:100000,trim_cells_per_query:64,tolerance_uv:1e-8})?} else if adaptive {shell.bisect_candidate_deviation_edges(&mut refinement_graph,candidates,interior_splits,1000,100000,0.05,64)?}
                    else {shell.bisect_candidate_interior_edges(&mut refinement_graph,candidates,interior_splits,1000,10000)?};
                if adaptive {assert!(bisection.splits<=interior_splits);}else{assert_eq!(bisection.splits,interior_splits);}
                assert!(bisection.spans<=if adaptive {100000}else{10000});
                assert!(bisection.trim_cells<=100000);
                assert!(bisection.candidates.inspect_incidence().closed_oriented());

                let extended_refinement=std::env::var("CAD_TRIANGLE_REFINEMENT_EXTENDED").is_ok();
                let candidates=bisection.candidates;
                let mut transition_budget_probes=Vec::new();
                if std::env::var("CAD_TRANSITION_BOUND_PROBES").is_ok() {
                    for triangle in candidates.triangles.iter().filter(|t|shell.faces()[t.face].kind()==crate::source_profile_body::ShellFaceKind::Transition).take(3) {
                        for budget in [64,128,256,512] {
                            let (bound,spent)=shell.bound_candidate_triangle(triangle,&refinement_graph,1000,budget)?;
                            assert!(spent<=budget);
                            if let Some(bound)=bound {assert!(bound.is_finite() && bound>=0.);}
                            transition_budget_probes.push(value_codec::json!({"face":triangle.face,"vertices":triangle.vertices,"uv":triangle.uv,"spansPerQuery":budget,"spent":spent,"deviationUpperMm":bound}));
                            let (center_bound,center_spent)=shell.bound_candidate_triangle_rectangle_center(triangle,&refinement_graph,1000,budget)?;
                            assert!(center_spent<=budget);
                            if let Some(bound)=center_bound {assert!(bound.is_finite() && bound>=0.);}
                            transition_budget_probes.push(value_codec::json!({"anchor":"rectangle-center","face":triangle.face,"vertices":triangle.vertices,"uv":triangle.uv,"spansPerQuery":budget,"spent":center_spent,"deviationUpperMm":center_bound}));
                            let (partition_bound,partition_spent)=shell.bound_candidate_triangle_partitioned(triangle,&refinement_graph,1000,budget)?;
                            assert!(partition_spent<=budget);
                            if let Some(bound)=partition_bound {assert!(bound.is_finite() && bound>=0.);}
                            transition_budget_probes.push(value_codec::json!({"anchor":"rectangle-partition","face":triangle.face,"vertices":triangle.vertices,"uv":triangle.uv,"spansPerQuery":budget,"spent":partition_spent,"deviationUpperMm":partition_bound}));
                        }
                    }
                    assert!(!transition_budget_probes.is_empty());
                }
                let vertices_before=refinement_graph.vertices.len();
                let mut no_query=shell.refine_face_candidates(&mut refinement_graph,&candidates,crate::source_profile_body::TriangleRefinementLimits {
                    max_deviation_mm:0.05,cells:100,spans:10000,spans_per_query:0,controls:1000,domain_cells:10000,
                    domain_cells_per_triangle:64,tolerance_uv:1e-8,max_depth:4})?;
                let no_regional=shell.qualify_pending_transition_regions(&refinement_graph,&mut no_query,0)?;
                assert_eq!((no_regional.spans,no_regional.queries,no_regional.accepted),(0,0,0));
                assert!(no_query.accepted.is_empty());
                assert_eq!(no_query.pending.len(),candidates.triangles.len());
                assert_eq!((no_query.cells,no_query.spans,no_query.domain_cells,no_query.split_triangles,no_query.initial_checked),(0,0,0,0,0));
                assert_eq!(refinement_graph.vertices.len(),vertices_before);
                assert_eq!(no_query.trim_inherited,0);
                assert_eq!(no_query.initial_domain_cells,0);
                assert_eq!(no_query.trim_cache_hits,0);
                assert_eq!((no_query.trim_triangle_queries,no_query.trim_triangle_proven),(0,0));
                assert_eq!((no_query.contour_enclosure_queries,no_query.contour_enclosure_cache_hits),(0,0));
                assert!(no_query.numeric_failures.is_empty());
                assert!(no_query.inspect_leaf_incidence().closed_oriented());
                let mut refinement=shell.refine_face_candidates(&mut refinement_graph,&candidates,crate::source_profile_body::TriangleRefinementLimits {
                    max_deviation_mm:0.05,cells:if extended_refinement {8192}else{64},spans:if extended_refinement {100000}else{10000},spans_per_query:64,controls:1000,domain_cells:if extended_refinement {100000}else{10000},
                    domain_cells_per_triangle:if extended_refinement {128}else{64},tolerance_uv:1e-8,max_depth:if extended_refinement {12}else{4}})?;
                let mut transition_point_probes=Vec::new();
                if std::env::var("CAD_TRANSITION_POINT_PROBES").is_ok() {
                    let probe_limit=std::env::var("CAD_TRANSITION_POINT_PROBE_LIMIT").ok().map(|v|v.parse::<usize>().expect("bounded diagnostic probe limit")).unwrap_or(16);
                    assert!((1..=16).contains(&probe_limit));
                    for pending in refinement.pending.iter().filter(|p|shell.faces()[p.triangle.face].kind()==crate::source_profile_body::ShellFaceKind::Transition).take(probe_limit) {
                        let triangle=&pending.triangle;
                        let query=std::array::from_fn(|k|triangle.uv[0][k]*0.5+triangle.uv[1][k]*0.5);
                        let (disabled,spent)=shell.candidate_point_deviation_lower(triangle,&refinement_graph,query,1000,0)?;
                        assert_eq!((disabled,spent),(None,0));
                        let (lower,spent)=shell.candidate_point_deviation_lower(triangle,&refinement_graph,query,1000,64)?;
                        assert!(spent<=64);
                        if let Some(lower)=lower {assert!(lower.is_finite() && lower>=0.);}
                        transition_point_probes.push(value_codec::json!({"face":triangle.face,"vertices":triangle.vertices,"uv":triangle.uv,"queryUv":query,"deviationLowerMm":lower,"spent":spent,"scope":"Point correspondence to affine mesh plane; material membership and whole-triangle admission independent"}));
                        for depth in [1,2,3] {
                            let budget=if depth==1 {512}else if depth==2 {2048}else{8192};
                            let (upper,used)=shell.bound_candidate_triangle_partition_depth(triangle,&refinement_graph,1000,budget,depth)?;
                            assert!(used<=budget);
                            if let Some(upper)=upper { assert!(upper.is_finite() && upper>=0.); }
                            let (target_bound,target_used)=shell.bound_candidate_triangle_to_target(triangle,&refinement_graph,1000,budget,depth,0.05)?;
                            assert!(target_used<=used);
                            let (tight,tight_used)=shell.bound_candidate_triangle_to_target(triangle,&refinement_graph,1000,budget,depth,1e-6)?;
                            assert!(tight.is_none());
                            assert!(tight_used<=used);
                            assert!(shell.bound_candidate_triangle_to_target(triangle,&refinement_graph,1000,0,0,0.05).is_err());
                            if upper.is_some_and(|value|value<=0.05) {assert_eq!((target_bound,target_used),(upper,used));}
                            else { assert!(target_bound.is_none()); }
                            assert!(shell.bound_candidate_triangle_to_target(triangle,&refinement_graph,1000,0,depth,f64::NAN).is_err());
                            assert_eq!(shell.bound_candidate_triangle_to_target(triangle,&refinement_graph,1000,0,depth,0.05)?,(None,0));
                            transition_point_probes.push(value_codec::json!({"face":triangle.face,"vertices":triangle.vertices,"uv":triangle.uv,"partitionDepth":depth,"deviationUpperMm":upper,"spent":used,"targetUpperMm":target_bound,"targetSpent":target_used,"tightTargetSpent":tight_used}));
                        }
                    }
                    assert!(!transition_point_probes.is_empty());
                }
                let regional=if let Ok(value)=std::env::var("CAD_REGIONAL_TRANSITION_BUDGET") {
                    let budget=value.parse::<usize>().expect("bounded regional transition budget");
                    let result=shell.qualify_pending_transition_regions(&refinement_graph,&mut refinement,budget)?;
                    assert!(result.spans<=result.max_spans);
                    Some(value_codec::json!({"maxSpans":result.max_spans,"spans":result.spans,"queries":result.queries,"accepted":result.accepted}))
                } else {None};
                assert!(refinement.cells<=if extended_refinement {8192}else{64});
                assert!(refinement.initial_checked<=candidates.triangles.len());
                assert!(refinement.trim_triangle_proven<=refinement.trim_triangle_queries);
                assert!(refinement.initial_domain_cells<=refinement.domain_cells);
                assert!(refinement.trim_triangle_queries<=refinement.cells);
                if refinement.initial_checked<candidates.triangles.len() {assert_eq!(refinement.split_triangles,0);}
                assert!(refinement.spans<=if extended_refinement {100000}else{10000});
                assert!(refinement.domain_cells<=if extended_refinement {100000}else{10000});
                assert!(!refinement.accepted.is_empty() || !refinement.pending.is_empty());
                assert_eq!(refinement.accepted.len()+refinement.pending.len(),refinement.initial_triangles+2*refinement.split_triangles);
                assert!(refinement.inspect_leaf_incidence().closed_oriented());
                assert_eq!(refinement.numeric_failures.len(),refinement.pending.iter().filter(|p|p.reason=="triangle-support-numeric-unproven" || p.reason=="triangle-midpoint-numeric-unproven").count());
                assert!(refinement.numeric_failures.iter().all(|p|p.code=="NURBS_NUMERIC_ERROR" && p.uv.iter().flatten().all(|v|v.is_finite())));
                for t in &refinement.accepted {assert!(t.deviation_upper_mm<=0.05);}
                if let Ok(path)=std::env::var("CAD_BOUNDARY_TRIANGLE_REPORT") {
                    let report=value_codec::json!({"version":1,"fixture":"annular-root-ended-expression",
                        "targetDeviationMm":0.05,"solidQualified":false,
                        "refinementLimits":{"maxDeviationMm":refinement.limits.max_deviation_mm,"cells":refinement.limits.cells,"spans":refinement.limits.spans,"spansPerQuery":refinement.limits.spans_per_query,"controls":refinement.limits.controls,"domainCells":refinement.limits.domain_cells,"domainCellsPerTriangle":refinement.limits.domain_cells_per_triangle,"toleranceUv":refinement.limits.tolerance_uv,"maxDepth":refinement.limits.max_depth},
                        "transitionBudgetProbes":transition_budget_probes,
                        "transitionPointProbes":transition_point_probes,
                        "regionalTransitionLimits":regional.as_ref().map(|work|value_codec::json!({"maxSpans":work["maxSpans"].clone(),"spansPerQuery":2048,"partitionDepth":2,"earlyUnproven":true,"strictTriangleSeparation":true})),
                        "regionalTransitionWork":regional,
                        "faceKinds":shell.faces().iter().enumerate().map(|(face,support)|value_codec::json!({"face":face,"kind":format!("{:?}",support.kind())})).collect::<Vec<_>>(),
                        "interiorBisection":{"splits":bisection.splits,"spans":bisection.spans,"refused":bisection.refused,"adaptive":adaptive,"deviationQueries":bisection.deviation_queries,"deviationUnproven":bisection.deviation_unproven,"trimAdaptive":trim_adaptive,"trimQueries":bisection.trim_queries,"trimCells":bisection.trim_cells,"trimUnproven":bisection.trim_unproven},
                        "candidate":{"triangles":candidates.triangles.len(),"pendingFaces":candidates.pending.iter().map(|p|
                            value_codec::json!({"face":p.face,"reason":p.reason})).collect::<Vec<_>>(),
                            "openEdges":incidence_report.open_edges,"nonmanifoldEdges":incidence_report.nonmanifold_edges,
                            "orientationConflicts":incidence_report.orientation_conflicts,"invalidVertexLinks":incidence_report.invalid_vertex_links},
                        "boundaryPending":refinement_graph.pending.iter().map(|p|value_codec::json!({"pair":p.pair,"reason":p.reason})).collect::<Vec<_>>(),
                        "refinement":{"initialTriangles":refinement.initial_triangles,"initialChecked":refinement.initial_checked,"initialDomainCells":refinement.initial_domain_cells,"trimInherited":refinement.trim_inherited,"trimCacheHits":refinement.trim_cache_hits,"trimTriangleQueries":refinement.trim_triangle_queries,"trimTriangleProven":refinement.trim_triangle_proven,"contourEnclosureQueries":refinement.contour_enclosure_queries,"contourEnclosureCacheHits":refinement.contour_enclosure_cache_hits,"numericFailures":refinement.numeric_failures.iter().map(|p|value_codec::json!({"face":p.face,"vertices":p.vertices,"uv":p.uv,"stage":p.stage,"code":p.code,"message":p.message})).collect::<Vec<_>>(),"splits":refinement.split_triangles,"leafIncidenceClosed":refinement.inspect_leaf_incidence().closed_oriented(),"cells":refinement.cells,"spans":refinement.spans,"domainCells":refinement.domain_cells,
                            "accepted":refinement.accepted.len(),"pending":refinement.pending.iter().map(|p|
                                value_codec::json!({"face":p.triangle.face,"vertices":p.triangle.vertices,"uv":p.triangle.uv,"reason":p.reason})).collect::<Vec<_>>()}});
                    std::fs::write(path,report.to_string()).unwrap();
                }

                Ok((shell.faces().len(),shell.components(),shell.pairs().len()))
            }).unwrap();
        assert_eq!(single_call.result.map(|(faces,components,_)|(faces,components)),Some((29,1)));
        let mut no_work=shell_work();no_work.boundary.controls=0;
        assert!(crate::source_profile_body::with_restored_expression_shell(&rebuilt,boundary_recipe.clone(),no_work,
            |_|->nurbs_core::Result<()> {panic!("no recipe proof must not invoke shell inspection")}).unwrap().result.is_none());
        let mut excessive=shell_work();excessive.shell_uses=usize::MAX;
        assert!(crate::source_profile_body::with_restored_expression_shell(&rebuilt,boundary_recipe.clone(),excessive,
            |_|Ok(())).is_err());

        let mut zero=boundary_work();zero.controls=0;
        let no_call=crate::source_profile_body::with_restored_transition_boundary(&rebuilt,boundary_recipe.clone(),zero,
            |_|->nurbs_core::Result<()> {panic!("incomplete owners must not reach inspection")}).unwrap();
        assert!(no_call.result.is_none());
        let mut invalid_boundary=boundary_recipe.clone();invalid_boundary["sides"][1]=invalid_boundary["sides"][0].clone();
        assert!(crate::source_profile_body::with_restored_transition_boundary(&rebuilt,invalid_boundary,boundary_work(),
            |_|Ok(())).is_err());
        assert_eq!(restored_bridge.enclose([0.,1.],[0.,1.],100).unwrap(),authored.enclose([0.,1.],[0.,1.],100).unwrap());
        assert!(restored_bridge.constrain_original_rails(1000).unwrap().0.is_some());
        for field in ["proposal","sections","scales","version","kind"] {
            let mut changed=bridge_recipe.clone();changed[field]=value_codec::Value::Null;
            assert!(crate::source_profile_body::restore_section_bridge(&rebuilt,changed).is_err(),"{field}");
        }
        let mut changed=bridge_recipe.clone();changed["sections"][0]["face"]=value_codec::json!(5);
        assert!(crate::source_profile_body::restore_section_bridge(&rebuilt,changed).is_err());
        let mut changed=bridge_recipe.clone();changed["scales"]=value_codec::json!([0.,0.]);
        assert!(crate::source_profile_body::restore_section_bridge(&rebuilt,changed).is_err());
        for uv in [[0.75,0.25],[0.9,0.75]] {
            assert_eq!(original.profile(0).unwrap().enclose([[uv[0];2],[uv[1];2]],100).unwrap(),
                rebuilt.profile(0).unwrap().enclose([[uv[0];2],[uv[1];2]],100).unwrap());
        }
    }

    #[test]
    fn annular_corrected_expression_coefficient_radius_cell() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let body=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:20.,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap().body.unwrap();
        let proposal=crate::source_profile_body::prepare(&body,
            &[crate::source_profile_body::FaceSpec{face:0,anchor:[5,0]}],
            crate::source_profile_body::Limits{controls:1000,fragments:100,position_tolerance:1e-7}).unwrap().proposal.unwrap();
        let profile=proposal.profile(0).unwrap();
        let bridge=nurbs_core::normal_alignment::ProfileSectionBridge::new(
            profile.section(0,0.75).unwrap(),profile.section(0,0.9).unwrap(),[0.9-0.75;2]).unwrap();
        let bridge=bridge.constrain_original_rails(1000).unwrap().0.unwrap();
        let authored=crate::circular_blend::plane_cylinder_transition(20.,6.,0.,1.25,0.,std::f64::consts::FRAC_PI_2/4.).unwrap();
        let centers=authored.centers.reverse().unwrap();let law=authored.radius_curve().reverse().unwrap();
        // Rounded evaluations only define the ray; coefficient isolation proves its roots.
        let u=0.75*0.5+0.9*0.5;
        let center=centers.evaluate(u).unwrap().point;
        let target=profile.source().evaluate(u,0.5).unwrap().point;
        let origin=[center[0],center[1],center[2]];
        let direction=std::array::from_fn(|k|target[k]-origin[k]);
        let line=bridge.inspect_line_intersections([[0.,1.];2],origin,direction,1e-5,1000,100000,100000000).unwrap();
        let roots=line.intersections.expect("annular coefficient line enclosure");
        eprintln!("annular coefficient roots {:?}",roots);
        assert!(roots.unresolved.is_empty(),"annular line roots remain unresolved");
        assert_eq!(roots.roots.len(),1);
        let report=bridge.inspect_radius_coefficients(&centers,&law,[[0.75,0.9];2],[[0.,1.];2],100000,100000000).unwrap();
        eprintln!("annular coefficient radius reason={} error={:?} residual={:?} spans={} work={}",
            report.reason,report.error_upper,report.squared_residual,report.spans,report.coefficient_work);
        assert!(report.error_upper.is_some(),"{}",report.reason);
        assert!(report.error_upper.unwrap()<=1e-4,"whole annular radius exceeds requested tolerance");
        assert!(report.squared_residual.is_some());
        assert!(report.spans<=100000 && report.coefficient_work<=100000000);
        // The correlated coefficient proof must remain tied to the supplied
        // law: changing its radius cannot reuse the successful original bound.
        let mut changed_law=law.clone();
        for point in &mut changed_law.control_points {point[0]+=0.25;}
        let changed=bridge.inspect_radius_coefficients(&centers,&changed_law,
            [[0.75,0.9];2],[[0.,1.];2],100000,100000000).unwrap();
        assert!(changed.error_upper.map_or(true,|error|error>1e-4),
            "a changed radius law must not pass the original tolerance");
        assert!(changed.error_lower.unwrap()>1e-4,
            "the changed law must have a certified radial mismatch");
        assert!(report.error_lower.unwrap()<=report.error_upper.unwrap());
        for (spans,work) in [(0,100000000),(100000,0)] {
            let unproven=bridge.inspect_radius_coefficients(&centers,&changed_law,
                [[0.75,0.9];2],[[0.,1.];2],spans,work).unwrap();
            assert!(unproven.error_lower.is_none() && unproven.error_upper.is_none(),
                "exhausted work cannot establish a radial mismatch");
        }
        assert!(changed.spans<=100000 && changed.coefficient_work<=100000000);
        if let Ok(path)=std::env::var("BREP_COEFFICIENT_RADIUS_PATH") {
            std::fs::write(path,value_codec::json!({"domain":[[0.,1.],[0.,1.]],"lawDomains":[[0.75,0.9],[0.75,0.9]],
                "errorUpper":report.error_upper,"squaredResidual":report.squared_residual,
                "spans":report.spans,"coefficientWork":report.coefficient_work,"reason":report.reason,
                "changedLaw":{"radiusDeltaMm":0.25,"errorUpper":changed.error_upper,
                    "errorLower":changed.error_lower,"spans":changed.spans,"coefficientWork":changed.coefficient_work,
                    "reason":changed.reason,"passesTolerance":false}}).to_string()).unwrap();
        }
    }

    #[test]
    fn annular_corrected_expression_authored_radius() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let body=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:20.,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap().body.unwrap();
        let proposal=crate::source_profile_body::prepare(&body,
            &[crate::source_profile_body::FaceSpec{face:0,anchor:[5,0]}],
            crate::source_profile_body::Limits{controls:1000,fragments:100,position_tolerance:1e-7}).unwrap().proposal.unwrap();
        let profile=proposal.profile(0).unwrap();
        let bridge=nurbs_core::normal_alignment::ProfileSectionBridge::new(
            profile.section(0,0.75).unwrap(),profile.section(0,0.9).unwrap(),[0.9-0.75;2]).unwrap();
        let bridge=bridge.constrain_original_rails(1000).unwrap().0.unwrap();
        let authored=crate::circular_blend::plane_cylinder_transition(20.,6.,0.,1.25,0.,std::f64::consts::FRAC_PI_2/4.).unwrap();
        let mut reversed=authored.surface.clone();reversed.control_points.reverse();reversed.weights.reverse();
        assert_eq!(profile.source(),&reversed);
        let centers=authored.centers.reverse().unwrap();let law=authored.radius_curve().reverse().unwrap();
        let report=bridge.inspect_radius_on_domains(&centers,&law,[[0.75,0.9];2],1e-4,10000,1000000).unwrap();
        eprintln!("annular exact radius complete={} cells={} spans={} pending={} reason={} violation={:?}",
            report.complete,report.cells,report.spans,report.pending.len(),report.reason,
            report.violation.as_ref().map(|v|(v.domain,v.error_bounds)));
        assert!(report.cells<=10000 && report.spans<=1000000);
        assert_eq!(report.complete,report.pending.is_empty());
        if report.complete {assert!(report.accepted_error_upper<=1e-4);}
        if let Ok(path)=std::env::var("BREP_FOCUSED_EXPRESSION_RADIUS_PATH") {
            std::fs::write(path,value_codec::json!({"complete":report.complete,"tolerance":1e-4,
                "cells":report.cells,"spans":report.spans,"acceptedCells":report.accepted_cells,
                "domain":report.domain,"pendingParameterFraction":report.pending_parameter_fraction(),
                "acceptedErrorUpper":report.accepted_error_upper,"pending":report.pending,"reason":report.reason,
                "violation":report.violation.map(|v|value_codec::json!({"domain":v.domain,"errorBounds":v.error_bounds,
                    "squaredResidual":v.squared_residual}))}).to_string()).unwrap();
        }
    }

    #[test]
    fn exact_profile_owned_curved_seam_limits() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let body=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:20.,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap().body.unwrap();
        let proposal=crate::source_profile_body::prepare(&body,
            &[crate::source_profile_body::FaceSpec{face:0,anchor:[5,0]}],
            crate::source_profile_body::Limits{controls:1000,fragments:100,position_tolerance:1e-7}).unwrap().proposal.unwrap();
        for edge in [0,1] {
            let report=proposal.inspect_edge_pole_limit_angles(0,edge,true,1e-6,1000,10000).unwrap();
            eprintln!("curved seam edge={edge} aligned={:?} pending={:?} reason={} cells={} spans={}",
                report.aligned,report.pending,report.reason,report.cells,report.spans);
            if edge==0 {
                assert_eq!(report.aligned,Some(true),"{}",report.reason);
                assert!(report.pending.is_empty());
            } else {
                assert_eq!(report.aligned,None);
                assert!(!report.pending.is_empty());
                assert_eq!(report.reason,"profile-path-work-limit");
                let refined=proposal.inspect_edge_pole_limit_angles(0,edge,true,1e-6,10000,100000).unwrap();
                eprintln!("refined curved seam edge={edge} aligned={:?} pending={:?} reason={} cells={} spans={} upper={:?}",
                    refined.aligned,refined.pending,refined.reason,refined.cells,refined.spans,refined.upper_sine_squared);
                assert_eq!(refined.aligned,Some(true),"{}",refined.reason);
            }
        }
    }

    #[test]
    fn restricted_profile_seams_match_owned_neighbor_supports() {
        restricted_profile_seam_control(false,false);
    }
    #[test]
    fn partial_rational_multispan_carrier_transfers_owned_transition_contact() {
        restricted_profile_seam_control(true,false);
    }
    #[test]
    fn partial_rational_transition_has_owned_oriented_volume() {
        restricted_profile_seam_control(true,true);
    }
    fn restricted_profile_seam_control(partial_carrier:bool,volume_only:bool) {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let mut shell=prepare(&model,limits()).unwrap().shell.unwrap();
        if partial_carrier {
            use value_codec::Serialize;
            let index=shell.uses().iter().position(|uses|uses.iter().all(|a|[0,5].contains(&a.face))).expect("retained paired contact edge");
            let edge=&shell.edges()[index];
            let mut world=edge.world().clone();
            let degree=world.degree;assert_eq!(world.control_points.len(),degree+1);
            let domain=world.domain();assert_eq!(domain,[0.,1.]);
            let first=world.control_points[0].clone();let last=world.control_points[degree].clone();
            let first_weight=world.weights[0];let last_weight=world.weights[degree];
            // Keep the original rational Bezier span literally unchanged.
            // Constant outer spans avoid rounding new homogeneous quotients.
            for _ in 0..degree {world.control_points.insert(0,first.clone());world.weights.insert(0,first_weight);world.control_points.push(last.clone());world.weights.push(last_weight);}
            world.knots=std::iter::repeat_n(-1.,degree+1).chain(std::iter::repeat_n(0.,degree)).chain(std::iter::repeat_n(1.,degree)).chain(std::iter::repeat_n(2.,degree+1)).collect();
            let ranges=crate::source_seam_tangency::ranges(edge).map(|ends|ends.map(|r|[r[0]+r[1],3.*r[1]]));
            let mut definition=shell.definition().unwrap();
            definition["pairs"][index]["edge"]["world"]=world.to_value();
            let planes=[None::<[[f64;3];3]>;2];let candidates=[[None::<[f64;2]>;2];2];
            definition["pairs"][index]["edge"]["recipe"]=value_codec::json!({"kind":"mapped","ranges":ranges,"planes":planes,"candidates":candidates});
            let replay=crate::source_shell_restore::restore(definition,&crate::source_shell_restore::Limits {
                regions:crate::source_region_restore::test_limits(),exact_work:100000000,driver_cells:100000,
            }).unwrap();
            shell=replay.shell.expect(replay.reason);
            assert!(!shell.edges()[index].covers_complete_canonical_source());
        }
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let body=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:20.,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap().body.unwrap();
        let specs=[crate::source_profile_body::FaceSpec{face:0,anchor:[5,0]},crate::source_profile_body::FaceSpec{face:5,anchor:[0,0]}];
        let report=crate::source_profile_body::prepare(&body,&specs,
            crate::source_profile_body::Limits{controls:1000,fragments:100,position_tolerance:1e-7}).unwrap();
        let proposal=report.proposal.unwrap();
        let mut rooted_seams=Vec::new();
        let mut whole_seam_diagnostics=Vec::new();
        let mut pole_diagnostics=Vec::new();
        let mut section_join_diagnostics=Vec::new();
        // Source pole limits are diagnostic only: they cannot admit the changed
        // profile, nor replace an undefined regular endpoint normal.
        let source_profile=proposal.profile(0).unwrap().source();
        for end in [false,true] {
            let factored=nurbs_core::factored_surface_normals::prepare(source_profile,end,100_000_000).unwrap();
            if let Some(proof)=factored.certificate {
                let limits=proof.boundary_limits(1e-6).unwrap();
                eprintln!("original selected pole end={end} collapsed={:?} distinctLimits={} sine2={:?}",
                    limits.collapsed_point,limits.distinct_tangent_limits,limits.sine_squared_interval);
            }
            let (limits,spans)=proposal.profile(0).unwrap().factored_pole_limits(end,1e-6,16).unwrap();
            if let Some(limits)=limits {
                eprintln!("exact changed profile pole end={end} point={:?} distinctLimits={} sine2={:?}",
                    limits.point,limits.distinct_tangent_limits,limits.sine_squared_interval);
                pole_diagnostics.push(value_codec::json!({"face":0,"high":end,"point":limits.point,
                    "normalLimits":limits.components,"distinctTangentLimits":limits.distinct_tangent_limits,
                    "sineSquaredInterval":limits.sine_squared_interval,"spans":spans,
                    "regularPoleCertified":false}));
            } else {
                pole_diagnostics.push(value_codec::json!({"face":0,"high":end,"spans":spans,
                    "reason":"profile-pole-factor-recipe-or-budget-unproven","regularPoleCertified":false}));
            }
        }
        for (edge,canonical,faces) in [(0,[0.1,0.25],[0,1]),(1,[0.75,0.9],[0,2])] {
            let limits=Some(proposal.inspect_edge_pole_limit_angles(0,edge,true,1e-6,1000,10000).unwrap());
            let r=limits.as_ref().unwrap();
            eprintln!("owned curved seam limit angles edge={edge} aligned={:?} pending={} upper={:?} reason={}",
                r.aligned,r.pending.len(),r.upper_sine_squared,r.reason);
            let whole=proposal.qualify_edge(edge,100,1e-6,100000,100000).unwrap();
            eprintln!("whole retained fillet seam edge={edge} qualified={} reason={}",whole.seam.is_some(),whole.reason);
            whole_seam_diagnostics.push(value_codec::json!({
                "edge":edge,"faces":faces,"canonicalInterval":[0.,1.],
                "qualified":whole.seam.is_some(),"reason":whole.reason,
                "upperSineSquared":whole.seam.as_ref().map(|seam|seam.upper_sine_squared()),
                "pending":whole.angular.as_ref().map(|angular|&angular.pending),
                "cells":whole.angular.as_ref().map(|angular|angular.cells),
                "spans":whole.angular.as_ref().map(|angular|angular.spans),
                "factoredLimitAngles":limits.as_ref().map(|r|value_codec::json!({"aligned":r.aligned,
                    "pending":r.pending,"cells":r.cells,"spans":r.spans,"upperSineSquared":r.upper_sine_squared,"reason":r.reason})),
                "carrierMapping":"owned-original-pcurve",
            }));
            if let Some(seam) = whole.seam.as_ref() {
                assert!(std::ptr::eq(seam.proposal(), &proposal));
                assert_eq!(seam.edge(), edge);
                assert_eq!(seam.faces(), faces);
                assert!(seam.upper_sine_squared() <= seam.threshold());
            }
            let report = proposal
                .qualify_edge_interval(edge, canonical, 1e-6, 100000, 100000)
                .unwrap();
            let seam = report.seam.expect(report.angular.reason);
            assert!(std::ptr::eq(seam.proposal(), &proposal));
            assert_eq!(seam.edge(), edge);
            assert_eq!(seam.faces(), faces);
            assert_eq!(seam.canonical_interval(), canonical);
            assert_eq!(report.angular.aligned, Some(true));
            assert!(report.angular.pending.is_empty());
            assert!(seam.upper_sine_squared() <= seam.threshold());
            eprintln!(
                "restricted seam edge={edge} interval={canonical:?} cells={} spans={} upper={}",
                report.angular.cells,
                report.angular.spans,
                seam.upper_sine_squared()
            );
            let high = edge == 1;
            let a = proposal
                .cut_endpoint(0, 0, 0.75, high, edge, 16)
                .unwrap()
                .endpoint
                .unwrap();
            let b = proposal
                .cut_endpoint(0, 0, 0.9, high, edge, 16)
                .unwrap()
                .endpoint
                .unwrap();
            let pa = a.split_source_use(100).unwrap().partition.unwrap();
            let pb = b.split_source_use(100).unwrap().partition.unwrap();
            let rooted = proposal
                .qualify_root_ended_seam([&pa, &pb], 1e-6, 100000, 100000)
                .unwrap();
            let rooted_seam = rooted.seam.expect(rooted.reason);
            assert!(std::ptr::eq(rooted_seam.restricted().proposal(), &proposal));
            assert_eq!(rooted_seam.restricted().edge(), edge);
            assert!(
                rooted_seam
                    .propose_incident_contours(0)
                    .unwrap()
                    .proposal
                    .is_none()
            );
            assert!(rooted_seam.propose_incident_contours(100001).is_err());
            let contours = rooted_seam.propose_incident_contours(100).unwrap();
            let contours = contours.proposal.expect(contours.reason);
            let incident = rooted_seam.incident_fragments().unwrap();
            for (side, contour) in contours.contours().iter().enumerate() {
                let original = &body.geometry().shell().regions().unwrap()[contour.face()];
                assert_eq!(contour.loops().len(), original.loops().len());
                let [wire, index] = contour.middle();
                assert_eq!(
                    contour.loops()[wire].len(),
                    original.loops()[wire].len() + 2
                );
                assert_eq!(
                    contour.loops()[wire][index].definition(),
                    incident[side].definition()
                );
                for (loop_index, loop_) in original.loops().iter().enumerate() {
                    for (edge_index, fragment) in loop_.iter().enumerate() {
                        if loop_index == wire && edge_index == index - 1 {
                            continue;
                        }
                        let shifted = if loop_index == wire && edge_index >= index {
                            edge_index + 2
                        } else {
                            edge_index
                        };
                        assert_eq!(
                            contour.loops()[loop_index][shifted].definition(),
                            fragment.definition()
                        );
                    }
                }
                for loop_ in contour.loops() {
                    assert!(
                        (0..loop_.len()).all(|i| loop_[i].joins(&loop_[(i + 1) % loop_.len()]))
                    );
                }
                for fragment in &contour.loops()[wire][index - 1..=index + 1] {
                    let restored =
                        crate::source_boundary_fragment::restore(fragment.definition(), 100)
                            .unwrap();
                    assert_eq!(restored.definition(), fragment.definition());
                }
            }
            let shared = rooted_seam
                .qualify_incident_edge(100000000, 100000)
                .unwrap();
            let shared_edge = shared.edge.expect(shared.reason);
            assert!(shared_edge.uses().iter().all(|u| {
                u.endpoints()
                    .iter()
                    .all(|e| matches!(e, crate::source_boundary_fragment::Endpoint::Carrier { .. }))
            }));
            let fresh_shared = crate::source_shared_edge_restore::restore(
                shared_edge.definition(),
                crate::source_shared_edge_restore::Limits {
                    mapping_cells_per_use: 100,
                    exact_work: 100000000,
                    driver_cells: 100000,
                },
            )
            .unwrap();
            let fresh_shared = fresh_shared.edge.expect(fresh_shared.reason);
            assert_eq!(fresh_shared.definition(), shared_edge.definition());

            assert!(
                rooted_seam
                    .qualify_incident_edge(1, 1)
                    .unwrap()
                    .edge
                    .is_none()
            );
            eprintln!(
                "two-root retained edge={edge} work={} drivers={} rootChecks={}",
                shared.work_used, shared.driver_cells, shared.root_checks
            );
            for fragment in rooted_seam.incident_fragments().unwrap() {
                assert!(fragment.endpoints().iter().all(|e| matches!(
                    e,
                    crate::source_boundary_fragment::Endpoint::Carrier { .. }
                )));
                let restored =
                    crate::source_boundary_fragment::restore(fragment.definition(), 100).unwrap();
                assert_eq!(restored.definition(), fragment.definition());
                assert_eq!(restored.reversed(), fragment.reversed());
                let native=fragment.requalify(100).unwrap();
                assert!(native.same_definition(&restored));
                assert_eq!(native.parameter_bounds(),restored.parameter_bounds());
                assert!(fragment.requalify(1).is_err());
                assert!(
                    crate::source_boundary_fragment::restore(fragment.definition(), 1).is_err()
                );
            }

            for point in rooted_seam.points() {
                let q = point.canonical_bounds();
                let whole = rooted_seam.restricted().canonical_interval();
                assert!(whole[0] <= q[0] && q[1] <= whole[1]);
                let fresh = crate::source_carrier_point::restore(
                    point.definition(),
                    crate::source_carrier_point::RestoreLimits {
                        edge: crate::source_shared_edge_restore::Limits {
                            mapping_cells_per_use: 16,
                            exact_work: 100000000,
                            driver_cells: 10000,
                        },
                        root_mapping_cells: 16,
                        spans: 3,
                    },
                )
                .unwrap()
                .point
                .unwrap();
                assert_eq!(fresh.definition(), point.definition());
            }
            let twin = crate::source_profile_body::prepare(
                &body,
                &specs,
                crate::source_profile_body::Limits {
                    controls: 1000,
                    fragments: 100,
                    position_tolerance: 1e-7,
                },
            )
            .unwrap()
            .proposal
            .unwrap();
            fn make_bridge<'p, 'b>(
                profile: &'p nurbs_core::normal_alignment::WeightProfile<'b>,
                end: f64,
            ) -> nurbs_core::normal_alignment::ProfileSectionBridge<'p, 'b, 'p, 'b> {
                nurbs_core::normal_alignment::ProfileSectionBridge::new(
                    profile.section(0, 0.75).unwrap(),
                    profile.section(0, end).unwrap(),
                    [end - 0.75; 2],
                )
                .unwrap()
            }
            let original_bridge = make_bridge(proposal.profile(0).unwrap(), 0.9);
            let original_bridge = original_bridge
                .constrain_original_rails(1000)
                .unwrap()
                .0
                .unwrap();
            for section_high in [false, true] {
                let join = original_bridge
                    .inspect_transverse_corrected_section_join(section_high, [0., 1.], 1000, 10000)
                    .unwrap();
                eprintln!(
                    "transition section join high={section_high} qualified={} cells={} spans={} pending={:?}",
                    join.complete_regular, join.cells, join.spans, join.pending
                );
                assert!(join.complete_regular, "{}", join.reason);
                if edge == 0 {
                    section_join_diagnostics.push(value_codec::json!({"high":section_high,
                        "parameter":if section_high {0.9}else{0.75},"qualified":join.complete_regular,
                        "cells":join.cells,"spans":join.spans,"pending":join.pending,"reason":join.reason}));
                }
            }
            let bound = rooted_seam
                .qualify_transition_side(&original_bridge, 1000, 1000)
                .unwrap();
            let bound = bound.side.expect(bound.reason);
            assert!(std::ptr::eq(bound.rooted(), &rooted_seam));
            assert_eq!(bound.high_free_end(), high);
            assert_eq!(bound.parameters(), [0.75, 0.9]);
            assert_eq!(bound.axis(), 0);
            assert_eq!(bound.face(), 0);
            let wrong_bridge = make_bridge(proposal.profile(0).unwrap(), 0.85);
            let wrong_bridge = wrong_bridge
                .constrain_original_rails(1000)
                .unwrap()
                .0
                .unwrap();
            assert!(
                rooted_seam
                    .qualify_transition_side(&wrong_bridge, 1000, 1000)
                    .unwrap()
                    .side
                    .is_none()
            );
            let foreign_bridge = make_bridge(twin.profile(0).unwrap(), 0.9);
            let foreign_bridge = foreign_bridge
                .constrain_original_rails(1000)
                .unwrap()
                .0
                .unwrap();
            assert!(
                rooted_seam
                    .qualify_transition_side(&foreign_bridge, 1000, 1000)
                    .unwrap()
                    .side
                    .is_none()
            );
            assert!(
                rooted_seam
                    .qualify_transition_side(&original_bridge, 0, 1000)
                    .unwrap()
                    .side
                    .is_none()
            );

            assert!(
                twin.qualify_root_ended_seam([&pa, &pb], 1e-6, 100, 100)
                    .unwrap()
                    .seam
                    .is_none()
            );
            assert!(
                proposal
                    .qualify_root_ended_seam([&pa, &pa], 1e-6, 100, 100)
                    .unwrap()
                    .seam
                    .is_none()
            );
            assert!(
                proposal
                    .qualify_root_ended_seam([&pa, &pb], 1e-6, 100, 1)
                    .unwrap()
                    .seam
                    .is_none()
            );
            eprintln!(
                "root-ended seam edge={edge} carrierSpans={} canonical={:?}",
                rooted.carrier_spans,
                rooted_seam.restricted().canonical_interval()
            );
            let limited = proposal
                .qualify_edge_interval(edge, canonical, 1e-6, 1, 1)
                .unwrap();
            assert!(limited.seam.is_none());
            assert!(!limited.angular.pending.is_empty());
            rooted_seams.push(rooted_seam);
        }
        if let Ok(path) = std::env::var("BREP_EXPRESSION_WHOLE_SEAMS_PATH") {
            std::fs::write(
                path,
                value_codec::json!({"seams":whole_seam_diagnostics,"poles":pole_diagnostics,
                "sectionJoins":section_join_diagnostics})
                .to_string(),
            )
            .unwrap();
        }
        let bridge = nurbs_core::normal_alignment::ProfileSectionBridge::new(
            proposal.profile(0).unwrap().section(0, 0.75).unwrap(),
            proposal.profile(0).unwrap().section(0, 0.9).unwrap(),
            [0.9 - 0.75; 2],
        )
        .unwrap();
        let jet = bridge
            .derivative_bounds([0.5; 2], [0.5; 2], 8)
            .unwrap()
            .0
            .unwrap();
        let vectors = jet.map(|r| r.map(|b| b[0] * 0.5 + b[1] * 0.5));
        let dot = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        let aa = dot(vectors[0], vectors[0]);
        let bb = dot(vectors[1], vectors[1]);
        let ab = dot(vectors[0], vectors[1]);
        let determinant = aa * bb - ab * ab;
        assert!(determinant > 0.);
        let projection = [
            std::array::from_fn(|k| (bb * vectors[0][k] - ab * vectors[1][k]) / determinant),
            std::array::from_fn(|k| (aa * vectors[1][k] - ab * vectors[0][k]) / determinant),
        ];
        let bridge = bridge.constrain_original_rails(1000).unwrap().0.unwrap();
        let sides = rooted_seams
            .iter()
            .map(|root| {
                root.qualify_transition_side(&bridge, 1000, 1000)
                    .unwrap()
                    .side
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let boundary =
            crate::source_profile_body::propose_transition_boundary([&sides[1], &sides[0]])
                .unwrap();
        assert!(
            crate::source_profile_body::propose_transition_boundary([&sides[0], &sides[0]])
                .is_err()
        );
        let other_boundary =
            crate::source_profile_body::propose_transition_boundary([&sides[0], &sides[1]])
                .unwrap();
        let mut other_sections = Vec::new();
        let mut section_bindings = Vec::new();
        for high in [false, true] {
            let parameter = if high { 0.9 } else { 0.75 };
            let ends = [
                proposal
                    .cut_endpoint(0, 0, parameter, false, 0, 16)
                    .unwrap()
                    .endpoint
                    .unwrap(),
                proposal
                    .cut_endpoint(0, 0, parameter, true, 1, 16)
                    .unwrap()
                    .endpoint
                    .unwrap(),
            ];
            let parts = ends
                .each_ref()
                .map(|end| end.split_source_use(100).unwrap().partition.unwrap());
            let section = boundary
                .propose_section_contours(high, [&parts[0], &parts[1]], 100)
                .unwrap();
            let section = section.section.expect(section.reason);
            let join = section.qualify_join(1000, 1000, 10000).unwrap();
            let joined = join.join.expect(join.reason);
            assert!(std::ptr::eq(joined.section(), &section));
            assert!(join.regularity.unwrap().pending.is_empty());
            assert!(section.qualify_join(0, 1000, 10000).unwrap().join.is_none());
            assert!(section.qualify_join(1000, 0, 10000).unwrap().join.is_none());
            assert!(std::ptr::eq(section.boundary(), &boundary));
            assert_eq!(section.edge_index(), if high { 1 } else { 3 });
            assert!(section.qualify_exterior(0, 0).unwrap().region.is_none());
            let exterior = section.qualify_exterior(1000, 1000).unwrap();
            let exterior = exterior.region.expect(exterior.reason);
            assert!(std::ptr::eq(exterior.section(), &section));
            assert!(!exterior.kept_loop().is_empty());
            assert!(!exterior.removed_loop().is_empty());
            let injectivity = exterior.inspect_quotient_injectivity(1, 16, 256).unwrap();
            assert!(injectivity.proven, "owned profile must qualify globally");
            let retained = exterior.qualify_face(1, 16, 256, 2048, 12288).unwrap();
            let retained_face = retained.face.expect(retained.reason);
            assert!(std::ptr::eq(retained_face.exterior(), &exterior));
            assert_eq!(
                retained_face.factored_boundary(),
                if high { Some(1) } else { None }
            );
            assert!(
                exterior
                    .qualify_face(1, 16, 255, 2048, 12288)
                    .unwrap()
                    .face
                    .is_none()
            );
            assert!(
                exterior
                    .qualify_face(1, 16, 256, 0, 12288)
                    .unwrap()
                    .face
                    .is_none()
            );
            eprintln!(
                "retained profile quotient high={high} proven={} cells={} reason={} bounds={:?} margins={:?}",
                injectivity.proven,
                injectivity.cells,
                injectivity.reason,
                injectivity.weighted_bounds,
                injectivity.band_margins_lower
            );
            let cover = exterior.inspect_regularity(128, 256).unwrap();
            let area: f64 = cover
                .regular
                .iter()
                .chain(&cover.pending)
                .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
                .sum();
            assert!((area - if high { 0.1 } else { 0.75 }).abs() < 1e-12);
            assert!(cover.cells <= 128 && cover.spans <= 256);
            if !high {
                assert!(cover.complete_regular);
                assert!(cover.pending.is_empty());
            }
            eprintln!(
                "retained profile high={high} regular={} pending={} cells={} spans={}",
                cover.regular.len(),
                cover.pending.len(),
                cover.cells,
                cover.spans
            );
            if high {
                let profile = proposal.profile(0).unwrap();
                let mut factored_regular = 0;
                for domain in &cover.pending {
                    let (normal, spent) = profile
                        .factored_pole_normal_bounds(*domain, true, 7)
                        .unwrap();
                    assert!((6..=7).contains(&spent));
                    let normal = normal.unwrap();
                    assert_eq!(normal.point, [20., 0., 6.]);
                    if normal.components.iter().any(|r| r[0] > 0. || r[1] < 0.) {
                        factored_regular += 1;
                    }
                }
                eprintln!(
                    "retained terminal factored normal accepted={} pending={}",
                    factored_regular,
                    cover.pending.len() - factored_regular
                );
                let factored = exterior
                    .inspect_factored_pole_regularity(true, 2048, 12288)
                    .unwrap();
                assert_eq!(factored.point, Some([20., 0., 6.]));
                assert!(
                    factored.complete_factored,
                    "entire retained terminal factored cover must qualify"
                );
                assert!(factored.pending.is_empty());
                assert_eq!(factored.power, Some(2));
                let area: f64 = factored
                    .regular
                    .iter()
                    .chain(&factored.pending)
                    .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
                    .sum();
                assert!((area - 0.1).abs() < 1e-12);
                eprintln!(
                    "terminal factored cover complete={} regular={} pending={} cells={} spans={} power={:?}",
                    factored.complete_factored,
                    factored.regular.len(),
                    factored.pending.len(),
                    factored.cells,
                    factored.spans,
                    factored.power
                );
                for free in [0., 0.5, 1.] {
                    let normal = profile
                        .factored_pole_normal_bounds([[1.; 2], [free; 2]], true, 7)
                        .unwrap()
                        .0
                        .unwrap();
                    eprintln!(
                        "pole scaled normal free={free} components={:?}",
                        normal.components
                    );
                }
                eprintln!(
                    "factored pending sample={:?}",
                    &factored.pending[..factored.pending.len().min(3)]
                );
            }
            let refused = exterior.inspect_regularity(0, 256).unwrap();
            assert!(!refused.complete_regular);
            assert_eq!(refused.pending.len(), 1);
            let expected = if high { 1. } else { 0. };
            assert!(
                exterior.kept_loop().iter().any(|f| f
                    .curve()
                    .control_points
                    .iter()
                    .all(|p| p[0] == expected))
            );
            assert!(
                !exterior.removed_loop().iter().any(|f| f
                    .curve()
                    .control_points
                    .iter()
                    .all(|p| p[0] == expected))
            );
            for loop_ in section.contours().loops() {
                assert!((0..loop_.len()).all(|i| loop_[i].joins(&loop_[(i + 1) % loop_.len()])));
            }
            assert!(
                boundary
                    .propose_section_contours(!high, [&parts[0], &parts[1]], 100)
                    .unwrap()
                    .section
                    .is_none()
            );
            assert!(
                boundary
                    .propose_section_contours(high, [&parts[1], &parts[0]], 100)
                    .unwrap()
                    .section
                    .is_none()
            );
            assert!(
                boundary
                    .propose_section_contours(high, [&parts[0], &parts[1]], 0)
                    .unwrap()
                    .section
                    .is_none()
            );
            other_sections.push(
                other_boundary
                    .propose_section_contours(high, [&parts[0], &parts[1]], 100)
                    .unwrap()
                    .section
                    .unwrap(),
            );
            section_bindings.push(section);
        }
        assert!(boundary.enclose_edge(4, [0., 1.], 1).is_err());
        assert!(boundary.enclose_edge(0, [-0.1, 0.5], 1).is_err());
        assert!(boundary.enclose_edge(0, [0.75, 0.25], 1).is_err());

        let refused = boundary.qualify_face([[0.; 3]; 2], 1000, 0, 0).unwrap();
        assert!(refused.face.is_none());
        let interior = refused.interior.unwrap();
        assert!(!interior.complete_regular);
        assert_eq!(interior.pending.len(), 1);
        assert!(
            boundary
                .qualify_face([[f64::NAN; 3]; 2], 1000, 1, 100)
                .is_err()
        );
        assert!(
            boundary
                .qualify_face([[0.; 3]; 2], 0, 1, 100)
                .unwrap()
                .face
                .is_none()
        );
        assert!(
            boundary
                .qualify_face_from_rails(projection, 1000, 0, 0)
                .unwrap()
                .face
                .is_none()
        );
        assert!(
            boundary
                .qualify_face_from_rails([[f64::NAN; 3]; 2], 1000, 1, 100)
                .is_err()
        );
        // This candidate only preconditions XY. Midpoint estimates choose
        // a matrix; fresh whole-expression interval coverage grants authority.
        let xy_det = vectors[0][0] * vectors[1][1] - vectors[1][0] * vectors[0][1];
        assert!(xy_det.is_finite() && xy_det != 0.);
        let xy_projection = [
            [vectors[1][1] / xy_det, -vectors[1][0] / xy_det, 0.],
            [-vectors[0][1] / xy_det, vectors[0][0] / xy_det, 0.],
        ];
        // Diagnose the exact boundary separately from coarse interval cells.
        // A candidate projection may degenerate there even when 3D immersion
        // is proven; point diagnostics never grant contact authority.
        for across in [[0., 0.], [0.5, 0.5], [1., 1.], [0., 1.]] {
            let (rail, spent) = bridge
                .first_free_from_rail_bounds([1., 1.], across, 1000)
                .unwrap();
            let projected = rail.map(|jet| {
                xy_projection.map(|row| {
                    let mut v = nurbs_core::interval_eval::Interval::point(0.);
                    for k in 0..3 {
                        if row[k] != 0. {
                            v = v
                                .add(
                                    nurbs_core::interval_eval::Interval::point(row[k])
                                        .mul(
                                            nurbs_core::interval_eval::Interval::new(
                                                jet[k][0], jet[k][1],
                                            )
                                            .unwrap(),
                                        )
                                        .unwrap(),
                                )
                                .unwrap();
                        }
                    }
                    [v.lo, v.hi]
                })
            });
            eprintln!(
                "owned XY high rail across={across:?} projectedFree={projected:?} spans={spent}"
            );
            if across == [0., 1.] {
                let bounds = projected.expect("bounded full source rail");
                assert!(bounds[0][0] > 0., "whole high rail first principal minor");
                assert_eq!(spent, 1);
            }
        }
        for domain in [
            [[0.984375, 1.], [0., 0.015625]],
            [[0.984375, 1.], [0.5, 0.515625]],
        ] {
            let (second, spent) = bridge
                .second_free_derivatives_with_transverse_correction(domain[0], domain[1], 10000)
                .unwrap();
            eprintln!(
                "owned XY boundary path second domain={domain:?} bounds={second:?} spans={spent}"
            );
            let (jet, spent) = bridge
                .derivatives_with_transverse_correction(domain[0], domain[1], 10000)
                .unwrap();
            eprintln!("owned XY boundary full jet domain={domain:?} bounds={jet:?} spans={spent}");
        }
        let direct_xy = bridge
            .inspect_transverse_corrected_injectivity(
                [0., 1.],
                [0., 1.],
                xy_projection,
                1000,
                100000,
            )
            .unwrap();
        let area = |regions: &Vec<[[f64; 2]; 2]>| {
            regions
                .iter()
                .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
                .sum::<f64>()
        };
        eprintln!(
            "owned transition direct XY pendingArea={} accepted={} pending={} cells={} spans={}",
            area(&direct_xy.pending),
            direct_xy.regular.len(),
            direct_xy.pending.len(),
            direct_xy.cells,
            direct_xy.spans
        );
        let xy_report = bridge
            .inspect_transverse_corrected_injectivity_from_rails(
                [0., 1.],
                [0., 1.],
                xy_projection,
                2000,
                100000,
            )
            .unwrap();
        eprintln!(
            "owned transition XY injectivity qualified={} accepted={} pending={} cells={} spans={} projection={:?}",
            xy_report.complete_regular,
            xy_report.regular.len(),
            xy_report.pending.len(),
            xy_report.cells,
            xy_report.spans,
            xy_projection
        );
        eprintln!(
            "owned transition rail XY pendingArea={} pendingDomains={:?}",
            area(&xy_report.pending),
            xy_report.pending
        );
        assert!(xy_report.cells <= 2000 && xy_report.spans <= 100000);
        // Budget exhaustion must preserve the entire requested domain. A
        // positive determinant on visited cells cannot hide an omitted region.
        for report in [&direct_xy, &xy_report] {
            let covered = area(&report.regular) + area(&report.pending);
            assert!(
                (covered - 1.).abs() <= 32. * f64::EPSILON,
                "injectivity cover lost parameter area: {covered}"
            );
            for r in report.regular.iter().chain(&report.pending) {
                assert!(r.iter().all(|v| v[0] >= 0. && v[1] <= 1. && v[0] < v[1]));
            }
        }
        assert!(
            xy_report.complete_regular && xy_report.pending.is_empty(),
            "whole XY transition injectivity must be qualified"
        );
        let xy_owned = boundary
            .qualify_face_from_rails(xy_projection, 1000, 2000, 100000)
            .unwrap();
        let xy_face = xy_owned.face.expect("fresh owned XY transition admission");
        let support_join = xy_face
            .qualify_support_tangency(1000, 10000, 100000)
            .unwrap();
        assert!(
            support_join.certificate.is_some(),
            "{}",
            support_join.reason
        );
        assert_eq!(support_join.boundaries.len(), 4);
        assert!(
            support_join
                .boundaries
                .iter()
                .all(|r| r.complete_regular && r.pending.is_empty())
        );
        assert!(std::ptr::eq(
            support_join.certificate.unwrap().face(),
            &xy_face
        ));
        assert!(support_join.cells <= 10000 && support_join.spans <= 100000);
        for (controls, cells, spans) in [
            (0, 10000, 100000),
            (1000, 0, 100000),
            (1000, 10000, 0),
            (1000, 1, 1),
        ] {
            assert!(
                xy_face
                    .qualify_support_tangency(controls, cells, spans)
                    .unwrap()
                    .certificate
                    .is_none()
            );
        }

        let wrong_centers = nurbs_core::curve::Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let wrong_radius = nurbs_core::curve::Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![1., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let radius = xy_face
            .qualify_radius(&wrong_centers, &wrong_radius, 0.1, 1000, 100, 10000)
            .unwrap();
        assert!(radius.certificate.is_none());
        assert_eq!(radius.reason, "expression-radius-law-mismatch");
        assert!(radius.coverage.unwrap().violation.unwrap().error_bounds[0] > 0.1);
        assert!(
            xy_face
                .qualify_radius(&wrong_centers, &wrong_radius, 0.1, 0, 100, 10000)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            xy_face
                .qualify_radius(&wrong_centers, &wrong_radius, 0.1, 1000, 0, 10000)
                .unwrap()
                .certificate
                .is_none()
        );
        let authored = crate::circular_blend::plane_cylinder_transition(
            20.,
            6.,
            0.,
            1.25,
            0.,
            std::f64::consts::FRAC_PI_2 / 4.,
        )
        .unwrap();
        let mut reversed = authored.surface.clone();
        reversed.control_points.reverse();
        reversed.weights.reverse();
        assert_eq!(
            proposal.profile(0).unwrap().source(),
            &reversed,
            "radius law must match the actual original parameter orientation"
        );
        let centers = authored.centers.reverse().unwrap();
        let law = authored.radius_curve().reverse().unwrap();
        let law_domains = [[0.75, 0.9]; 2];
        for (controls, spans, work) in [
            (0, 100000, 100000000),
            (1000, 0, 100000000),
            (1000, 100000, 0),
        ] {
            assert!(
                xy_face
                    .qualify_radius_coefficients(
                        &centers,
                        &law,
                        law_domains,
                        1e-4,
                        controls,
                        spans,
                        work
                    )
                    .unwrap()
                    .certificate
                    .is_none()
            );
        }
        let owned_radius = xy_face
            .qualify_radius_coefficients(&centers, &law, law_domains, 1e-4, 1000, 100000, 100000000)
            .unwrap();
        let radius_proof = owned_radius.certificate.expect(owned_radius.reason);
        assert!(std::ptr::eq(radius_proof.face(), &xy_face));
        assert_eq!(radius_proof.centers(), &centers);
        assert_eq!(radius_proof.radius(), &law);
        assert_eq!(radius_proof.law_domains(), law_domains);
        assert!(radius_proof.error_upper() <= 1e-4);
        let coefficient_bound = owned_radius.bound.unwrap();
        eprintln!(
            "owned coefficient radius error={} spans={} work={}",
            radius_proof.error_upper(),
            coefficient_bound.spans,
            coefficient_bound.coefficient_work
        );
        if let Ok(path) = std::env::var("BREP_OWNED_COEFFICIENT_RADIUS_PATH") {
            std::fs::write(path,value_codec::json!({"qualified":true,"tolerance":1e-4,"lawDomains":law_domains,
                "errorUpper":radius_proof.error_upper(),"squaredResidual":coefficient_bound.squared_residual,
                "spans":coefficient_bound.spans,"coefficientWork":coefficient_bound.coefficient_work,
                "reason":owned_radius.reason}).to_string()).unwrap();
        }
        let radius = xy_face
            .qualify_radius_on_domains(&centers, &law, law_domains, 1e-4, 1000, 1000, 100000)
            .unwrap();
        let coverage = radius.coverage.unwrap();
        eprintln!(
            "owned transition authored radius qualified={} reason={} cells={} spans={} pending={} acceptedError={}",
            radius.certificate.is_some(),
            radius.reason,
            coverage.cells,
            coverage.spans,
            coverage.pending.len(),
            coverage.accepted_error_upper
        );
        if let Ok(path) = std::env::var("BREP_EXPRESSION_RADIUS_PATH") {
            std::fs::write(path,value_codec::json!({"qualified":radius.certificate.is_some(),"reason":radius.reason,
                "tolerance":1e-4,"lawDomains":law_domains,"cells":coverage.cells,"spans":coverage.spans,
                "acceptedErrorUpper":coverage.accepted_error_upper,"pending":coverage.pending,
                "violation":coverage.violation.map(|v|value_codec::json!({"domain":v.domain,"errorBounds":v.error_bounds,
                    "squaredResidual":v.squared_residual}))}).to_string()).unwrap();
        }
        assert!(std::ptr::eq(xy_face.boundary(), &boundary));
        assert_eq!(xy_face.projection(), xy_projection);
        assert!(
            xy_face
                .qualify_original_xy_region(1e-8, 0, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(xy_face.qualify_original_xy_region(f64::NAN, 1, 1).is_err());
        let exact_profile = proposal.profile(0).unwrap();
        let profile_orientation =
            nurbs_core::profile_projection_jacobian::certify(exact_profile, [0, 1], 20000000)
                .unwrap();
        eprintln!(
            "exact factorized profile XY orientation qualified={} work={} reason={}",
            profile_orientation.certificate.is_some(),
            profile_orientation.exact_work,
            profile_orientation.reason
        );
        if let Some(certificate) = &profile_orientation.certificate {
            assert!(std::ptr::eq(certificate.profile(), exact_profile));
            assert_eq!(certificate.axes(), [0, 1]);
        }
        let profile_region = xy_face
            .qualify_profile_xy_region(1e-8, 20000000, 10000)
            .unwrap();
        eprintln!(
            "owned exact profile XY region qualified={} work={} cells={} reason={}",
            profile_region.certificate.is_some(),
            profile_region.exact_work,
            profile_region.boundary_cells,
            profile_region.reason
        );
        let profile_region = profile_region
            .certificate
            .expect("actual factorized profile region must qualify");
        assert!(std::ptr::eq(profile_region.face(), &xy_face));
        assert!(std::ptr::eq(
            profile_region.profile().profile(),
            exact_profile
        ));
        assert_eq!(profile_region.source_domain(), [[0.75, 0.9], [0., 1.]]);
        assert!(
            xy_face
                .qualify_profile_xy_region(1e-8, 0, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(xy_face.qualify_profile_xy_region(f64::NAN, 1, 1).is_err());
        let original_region = xy_face
            .qualify_original_xy_region(1e-8, 20000000, 10000)
            .unwrap();
        eprintln!(
            "owned transition original XY region qualified={} work={} cells={} reason={}",
            original_region.certificate.is_some(),
            original_region.exact_work,
            original_region.boundary_cells,
            original_region.reason
        );
        if let Some(region) = &original_region.certificate {
            assert!(std::ptr::eq(region.face(), &xy_face));
            assert_eq!(region.original().axes(), [0, 1]);
            assert_eq!(region.source_domain(), [[0.75, 0.9], [0., 1.]]);
        }
        let xy_interior = xy_owned.interior.unwrap();
        assert!(xy_interior.complete_regular && xy_interior.pending.is_empty());
        let admitted = boundary
            .qualify_face(projection, 1000, 1000, 61000)
            .unwrap();
        let face = admitted
            .face
            .expect("complete interior admission for the owned annular transition");
        assert!(std::ptr::eq(face.boundary(), &boundary));
        assert_eq!(face.projection(), projection);
        assert!(!std::ptr::eq(&face, profile_region.face()));
        assert!(std::ptr::eq(
            face.boundary(),
            profile_region.face().boundary()
        ));
        assert_ne!(face.projection(), profile_region.face().projection());
        let from_rails = boundary
            .qualify_face_from_rails(projection, 1000, 2000, 100000)
            .unwrap();
        let rail_face = from_rails
            .face
            .expect("fresh rail-policy admission for owned 3D projection");
        assert!(std::ptr::eq(rail_face.boundary(), &boundary));
        assert_eq!(rail_face.projection(), projection);
        let rail_interior = from_rails.interior.unwrap();
        assert!(rail_interior.complete_regular && rail_interior.pending.is_empty());
        assert!(
            face.qualify_rail_incidence(1, 1, 100)
                .unwrap()
                .incidence
                .is_none()
        );
        assert!(
            face.qualify_rail_incidence(100000000, 100000, 0)
                .unwrap()
                .incidence
                .is_none()
        );
        let sewn = face.qualify_rail_incidence(100000000, 100000, 100).unwrap();
        let sewn = sewn.incidence.expect(sewn.reason);
        assert!(std::ptr::eq(sewn.face(), &face));
        let exterior = section_bindings
            .iter()
            .map(|s| s.qualify_exterior(1000, 1000).unwrap().region.unwrap())
            .collect::<Vec<_>>();
        assert!(
            sewn.qualify_sections([&exterior[0], &exterior[0]])
                .unwrap()
                .patch
                .is_none()
        );
        let other_exterior = other_sections
            .iter()
            .map(|s| s.qualify_exterior(1000, 1000).unwrap().region.unwrap())
            .collect::<Vec<_>>();
        assert!(
            sewn.qualify_sections([&other_exterior[0], &other_exterior[1]])
                .unwrap()
                .patch
                .is_none()
        );
        let patch = sewn.qualify_sections([&exterior[1], &exterior[0]]).unwrap();
        let patch = patch.patch.expect(patch.reason);
        assert!(std::ptr::eq(patch.rails(), &sewn));
        let retained_faces = exterior
            .iter()
            .map(|e| {
                e.qualify_face(1, 16, 256, 2048, 12288)
                    .unwrap()
                    .face
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(
            patch
                .qualify_outer_edges([&retained_faces[0], &retained_faces[0]], 100000000, 100000)
                .unwrap()
                .assembly
                .is_none()
        );
        assert!(
            patch
                .qualify_outer_edges([&retained_faces[0], &retained_faces[1]], 1, 1)
                .unwrap()
                .assembly
                .is_none()
        );
        let outer = patch
            .qualify_outer_edges([&retained_faces[1], &retained_faces[0]], 100000000, 100000)
            .unwrap();
        eprintln!(
            "outer retained edges work={} drivers={} reason={}",
            outer.work, outer.driver_cells, outer.reason
        );
        let outer = outer.assembly.expect(outer.reason);
        assert!(std::ptr::eq(outer.patch(), &patch));
        assert_eq!(outer.edges().len(), 4);
        assert_eq!(outer.retained_factored(), [None, Some(1)]);
        assert!(outer.assemble_shell(0).unwrap().shell.is_none());
        let assembled = outer.assemble_shell(1000).unwrap();
        eprintln!(
            "expression shell uses={} reason={}",
            assembled.uses, assembled.reason
        );
        let expression = assembled.shell.expect(assembled.reason);
        let neighbor_join = expression
            .qualify_transition_neighbor_tangency(1e-6, 1000, 10000, 100000)
            .unwrap();
        assert!(
            neighbor_join.certificate.is_some(),
            "{}",
            neighbor_join.reason
        );
        assert!(neighbor_join.cells <= 10000 && neighbor_join.spans <= 100000);
        assert!(neighbor_join.cells > 0 && neighbor_join.spans > 0);
        let neighbor_certificate = neighbor_join.certificate.unwrap();
        assert!(std::ptr::eq(neighbor_certificate.shell(), &expression));
        assert!(
            neighbor_certificate
                .upper_sine_squared()
                .iter()
                .all(|v| *v <= 1e-6)
        );
        assert_eq!(&neighbor_certificate.upper_sine_squared()[..2], &[0., 0.]);
        assert!(
            expression
                .qualify_transition_neighbor_tangency(1e-6, 1000, 0, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_transition_neighbor_tangency(f64::NAN, 1000, 10000, 100000)
                .is_err()
        );
        let limited_neighbor = expression
            .qualify_transition_neighbor_tangency(1e-6, 1000, 1, 1)
            .unwrap();
        assert!(limited_neighbor.certificate.is_none());
        assert!(limited_neighbor.cells <= 1 && limited_neighbor.spans <= 1);
        assert!(
            expression
                .qualify_transition_neighbor_tangency(1e-6, 0, 10000, 100000)
                .unwrap()
                .certificate
                .is_none()
        );

        for face in 0..expression.faces().len() {
            let (derivatives, spent) = expression
                .enclose_support_derivatives(
                    face,
                    expression.support_domain(face).unwrap(),
                    1000,
                    0,
                )
                .unwrap();
            let (normal, normal_spent) = expression
                .enclose_support_normals(face, expression.support_domain(face).unwrap(), 1000, 0)
                .unwrap();
            assert!(
                normal.is_none() && normal_spent == 0,
                "zero normal budget stays unproven on every expression face"
            );
            assert_eq!(
                expression
                    .enclose_support_normal_directions(
                        face,
                        expression.support_domain(face).unwrap(),
                        1000,
                        0
                    )
                    .unwrap(),
                (None, 0)
            );
            if matches!(
                expression.faces()[face].kind(),
                crate::source_profile_body::ShellFaceKind::Retained(_)
            ) {
                let domain = expression.support_domain(face).unwrap();
                assert_eq!(
                    expression
                        .enclose_support_normal_directions(face, domain, 1000, 100)
                        .unwrap(),
                    expression
                        .enclose_support_normals(face, domain, 1000, 100)
                        .unwrap(),
                    "a pole-touching retained domain must keep actual-normal uncertainty"
                );
            }
            assert_eq!(
                expression
                    .inspect_support_trim_outside(
                        face,
                        expression.support_domain(face).unwrap(),
                        1e-5,
                        0
                    )
                    .unwrap(),
                (None, 0)
            );
            assert!(derivatives.is_none() && spent == 0);
        }

        let zero_line = expression
            .inspect_line_support_roots([1000.; 3], [1., 0., 0.], 1e-5, 1000, 0, 100000, 100000000)
            .unwrap();
        assert!(zero_line.roots.is_empty());
        assert_eq!(zero_line.pending.len(), expression.faces().len());
        let outside_line = expression
            .inspect_line_support_roots(
                [1000.; 3],
                [1., 0., 0.],
                1e-5,
                1000,
                1000,
                100000,
                100000000,
            )
            .unwrap();
        assert!(outside_line.roots.is_empty() && outside_line.pending.is_empty());
        assert!(outside_line.spans <= 100000 && outside_line.cells <= 1000);
        let trimmed_line = expression
            .inspect_trimmed_line_roots(
                [0., 0., 3.],
                [30., 0., 0.],
                1e-5,
                1000,
                10000,
                100000,
                100000000,
                100000,
            )
            .unwrap();
        assert!(!trimmed_line.roots.is_empty() || !trimmed_line.pending.is_empty());
        assert!(
            trimmed_line.cells <= 10000
                && trimmed_line.spans <= 100000
                && trimmed_line.coefficient_work <= 100000000
                && trimmed_line.domain_cells <= 100000
        );
        assert!(
            trimmed_line
                .roots
                .windows(2)
                .all(|r| r[0].1.parameter[1] < r[1].1.parameter[0])
        );
        eprintln!(
            "expression trimmed line roots={} pending={} cells={} spans={} coefficientWork={} domainCells={}",
            trimmed_line.roots.len(),
            trimmed_line.pending.len(),
            trimmed_line.cells,
            trimmed_line.spans,
            trimmed_line.coefficient_work,
            trimmed_line.domain_cells
        );
        let outside_segment = expression
            .inspect_segment_supports([1000.; 3], [1., 0., 0.], 0.01, 1000, 1000, 10000)
            .unwrap();
        assert!(outside_segment.boundary_excluded);
        assert!(outside_segment.candidates.is_empty() && outside_segment.pending.is_empty());
        assert_eq!(outside_segment.cells, expression.faces().len());
        let pending_segment = expression
            .inspect_segment_supports([0.; 3], [30., 0., 0.], 0.01, 1000, 0, 10000)
            .unwrap();
        assert!(!pending_segment.boundary_excluded);
        assert_eq!(pending_segment.pending.len(), expression.faces().len());
        let touching_segment = expression
            .inspect_segment_supports([0., 0., 3.], [30., 0., 0.], 0.05, 1000, 1000, 10000)
            .unwrap();
        assert!(!touching_segment.boundary_excluded);
        assert!(!touching_segment.candidates.is_empty() || !touching_segment.pending.is_empty());
        eprintln!(
            "expression segment outside={} outsideCells={} touchingCandidates={} touchingPending={} touchingSpans={}",
            outside_segment.boundary_excluded,
            outside_segment.cells,
            touching_segment.candidates.len(),
            touching_segment.pending.len(),
            touching_segment.spans
        );
        assert!(
            expression
                .inspect_segment_supports([0.; 3], [0.; 3], 0.01, 1000, 1000, 10000)
                .is_err()
        );
        let (gap, gap_certificate) = expression
            .qualify_face_clearance([28, 3], 13.5, 1000, 10000, 100000)
            .unwrap();
        eprintln!(
            "exact expression clearance complete={} lower={:?} cells={} spans={} pending={}",
            gap.complete,
            gap.lower_mm,
            gap.cells,
            gap.spans,
            gap.pending.len()
        );
        assert_eq!(gap.complete, gap.pending.is_empty());
        assert!(
            gap.complete,
            "whole transition/inner-face clearance must qualify"
        );
        assert_eq!(gap.complete, gap_certificate.is_some());
        if let Some(proof) = gap_certificate {
            assert!(std::ptr::eq(proof.shell(), &expression));
            assert_eq!(proof.faces(), [28, 3]);
            assert_eq!(Some(proof.lower_mm()), gap.lower_mm);
        }
        if gap.complete {
            assert!(gap.lower_mm.unwrap() >= 13.5);
        }
        if let Ok(path) = std::env::var("BREP_EXPRESSION_GAP_PATH") {
            std::fs::write(
                path,
                value_codec::json!({"faces":gap.faces,"requestedLowerMm":gap.requested_lower_mm,
                "complete":gap.complete,"lowerMm":gap.lower_mm,"cells":gap.cells,"spans":gap.spans,
                "cacheHits":gap.cache_hits,"pending":gap.pending,"reason":gap.reason})
                .to_string(),
            )
            .unwrap();
        }
        let (refused, refused_certificate) = expression
            .qualify_face_clearance([28, 3], 13.5, 1000, 0, 10000)
            .unwrap();
        assert!(refused_certificate.is_none());
        assert!(!refused.complete && refused.lower_mm.is_none() && !refused.pending.is_empty());
        assert!(
            expression
                .inspect_face_clearance([28, 28], 13.5, 1000, 1000, 10000)
                .is_err()
        );
        if let Some(region) = original_region.certificate.as_ref() {
            assert!(
                expression
                    .qualify_transition_ruled_contact(1, region, 1000, 0, 1000)
                    .unwrap()
                    .certificate
                    .is_none()
            );
            let ruled = expression
                .qualify_transition_ruled_contact(1, region, 1000, 10000000, 100000)
                .unwrap();
            eprintln!(
                "owned transition ruled contact qualified={} work={} reason={}",
                ruled.certificate.is_some(),
                ruled.exact_work,
                ruled.reason
            );
            let ruled = ruled
                .certificate
                .expect("whole ruled trace contact confined to owned high rail");
            assert_eq!(ruled.rail(), 1);
            assert_eq!(ruled.trace().domain(), [0., 3.]);
            let owners = ruled.owners();
            assert!(
                owners
                    .iter()
                    .all(|owner| expression.faces().iter().any(|f| std::ptr::eq(*owner, f)))
            );
        } else {
            assert_eq!(
                original_region.reason,
                "profile-original-weight-identity-unproven"
            );
        }
        assert!(std::ptr::eq(expression.assembly(), &outer));
        assert_eq!(
            expression.faces().len(),
            body.geometry().shell().faces().len() + 2
        );
        assert_eq!(
            expression.pairs().len(),
            body.geometry().shell().edges().len() + 6
        );
        assert_eq!(
            expression.poles().len(),
            body.geometry().shell().poles().len()
        );
        assert_eq!(expression.components(), 1);
        assert!(
            outer
                .assemble_shell(assembled.uses - 1)
                .unwrap()
                .shell
                .is_none()
        );
        let exact_budget = outer.assemble_shell(assembled.uses).unwrap();
        assert!(exact_budget.shell.is_some());
        assert!(
            expression
                .pairs()
                .iter()
                .all(|p| p.reversed()[0] != p.reversed()[1])
        );

        use crate::source_profile_body::ShellFaceKind;
        let profile = proposal.profile(0).unwrap();
        for (id, face) in expression.faces().iter().enumerate() {
            match face.kind() {
                ShellFaceKind::Original(index) => {
                    let surface = body.geometry().shell().faces()[index][0].edges()[0].surface();
                    let u = (surface.knots_u[surface.degree_u]
                        + surface.knots_u[surface.control_points.len()])
                        / 2.;
                    let v = (surface.knots_v[surface.degree_v]
                        + surface.knots_v[surface.control_points[0].len()])
                        / 2.;
                    let domain = [[u; 2], [v; 2]];
                    assert!(
                        expression
                            .enclose_support(id, domain, 1000, 0)
                            .unwrap()
                            .0
                            .is_none()
                    );
                    let bounds = expression
                        .enclose_support(id, domain, 1000, 100)
                        .unwrap()
                        .0
                        .unwrap();
                    let sample = surface.evaluate(u, v).unwrap().point;
                    for k in 0..3 {
                        assert!(
                            sample[k] >= bounds[k][0] - 1e-12 && sample[k] <= bounds[k][1] + 1e-12
                        );
                    }
                }
                ShellFaceKind::Retained(end) => {
                    let parameter = if end == 0 { 0.5 } else { 0.95 };
                    let domain = [[parameter; 2], [0.5; 2]];
                    assert_eq!(
                        expression.enclose_support(id, domain, 1000, 100).unwrap(),
                        profile.enclose(domain, 100).unwrap()
                    );
                }
                ShellFaceKind::Transition => {
                    assert!(
                        expression
                            .enclose_support(id, [[0.5; 2], [0.; 2]], 0, 100)
                            .unwrap()
                            .0
                            .is_none()
                    );
                    let entry = expression
                        .enclose_support(id, [[0.5; 2], [0.; 2]], 1000, 100)
                        .unwrap();
                    assert_eq!(
                        entry,
                        profile
                            .section(0, 0.75)
                            .unwrap()
                            .enclose([0.5; 2], 100)
                            .unwrap()
                    );
                    assert!(
                        expression
                            .enclose_support(id, [[0.5; 2], [0.5; 2]], 1000, 100)
                            .unwrap()
                            .0
                            .is_some()
                    );
                }
            }
        }
        assert!(
            expression
                .enclose_support(expression.faces().len(), [[0.; 2]; 2], 1000, 100)
                .is_err()
        );

        let retained_ids = [0, 1].map(|end| {
            expression
                .faces()
                .iter()
                .position(|f| f.kind() == ShellFaceKind::Retained(end))
                .unwrap()
        });
        let derivative_domain = [[0.2, 0.3], [0.2, 0.3]];
        for id in retained_ids {
            let support = expression.support_domain(id).unwrap();
            let derivative_domain =
                support.map(|r| [r[0] * 0.75 + r[1] * 0.25, r[0] * 0.5 + r[1] * 0.5]);
            let (direct_profile_derivatives, direct_profile_spans) =
                profile.derivative_bounds(derivative_domain, 10000).unwrap();
            let (owned, spent) = expression
                .enclose_support_derivatives(id, derivative_domain, 1000, 10000)
                .unwrap();
            assert_eq!(
                owned,
                direct_profile_derivatives
                    .map(|b| std::array::from_fn(|axis| std::array::from_fn(|k| b[k][axis])))
            );
            assert_eq!(spent, direct_profile_spans);
            let (position, position_spans) = expression
                .enclose_support(id, derivative_domain, 1000, 10000)
                .unwrap();
            let direct_flux = nurbs_core::surface_flux::bound_expression(
                position.unwrap(),
                owned.unwrap(),
                derivative_domain,
                2,
                0.,
                false,
            )
            .unwrap();
            let (flux, flux_spans) = expression
                .enclose_support_flux(id, derivative_domain, 2, 0., 1000, 10000)
                .unwrap();
            let flux = flux.unwrap();
            assert_eq!(flux.density, direct_flux.density);
            assert_eq!(flux.integral, direct_flux.integral);
            assert_eq!(flux_spans, position_spans + spent);
            let masked = expression
                .enclose_material_flux(id, derivative_domain, 2, 0., 1e-8, 1000, 10000, 0)
                .unwrap();
            let masked_bounds = masked.bound.unwrap();
            assert!(masked_bounds[0] <= 0. && masked_bounds[1] >= 0.);
            assert_eq!(masked.domain_cells, 0);
            assert_eq!(masked.spans, flux_spans);
            let classified = expression
                .enclose_material_flux(id, derivative_domain, 2, 0., 1e-8, 1000, 10000, 1000)
                .unwrap();
            let classified_bounds = classified.bound.unwrap();
            assert!(
                masked_bounds[0] <= classified_bounds[0]
                    && classified_bounds[1] <= masked_bounds[1]
            );
            assert!(classified.domain_cells <= 1000);
        }
        assert!(
            expression
                .enclose_support_flux(retained_ids[0], derivative_domain, 3, 0., 1000, 10000)
                .is_err()
        );
        assert!(
            expression
                .enclose_support_flux(retained_ids[0], derivative_domain, 2, f64::NAN, 1000, 10000)
                .is_err()
        );
        assert!(
            expression
                .enclose_support_derivatives(usize::MAX, derivative_domain, 1000, 10000)
                .is_err()
        );
        assert!(
            expression
                .enclose_support_derivatives(
                    retained_ids[0],
                    [[f64::NAN; 2], [0., 1.]],
                    1000,
                    10000
                )
                .is_err()
        );
        assert!(
            expression
                .enclose_support_derivatives(retained_ids[0], derivative_domain, 1000, 100001)
                .is_err()
        );
        let original_whole = expression
            .faces()
            .iter()
            .enumerate()
            .find_map(|(id, f)| match f.kind() {
                ShellFaceKind::Original(source)
                    if body.geometry().shell().regions().unwrap()[source]
                        .whole_chart_material() =>
                {
                    Some((id, source))
                }
                _ => None,
            })
            .expect("control has an unchanged whole material chart");
        let original_domain = expression
            .support_domain(original_whole.0)
            .unwrap()
            .map(|r| [r[0] * 0.75 + r[1] * 0.25, r[0] * 0.5 + r[1] * 0.5]);
        let original_surface =
            body.geometry().shell().faces()[original_whole.1][0].edges()[0].surface();
        let direct_original_flux =
            nurbs_core::surface_flux::bound(original_surface, original_domain, 2, 0., 10000)
                .unwrap();
        let native_original_flux = expression
            .enclose_material_flux(
                original_whole.0,
                original_domain,
                2,
                0.,
                1e-8,
                1000,
                10000,
                0,
            )
            .unwrap();
        let direct_original_integral = direct_original_flux.bound.unwrap().integral;
        let mut direct_original_bound = nurbs_core::interval_eval::Interval::new(
            direct_original_integral[0],
            direct_original_integral[1],
        )
        .unwrap();
        if body.geometry().shell().regions().unwrap()[original_whole.1].chart_winding() == -1 {
            direct_original_bound = direct_original_bound
                .mul(nurbs_core::interval_eval::Interval::point(-1.))
                .unwrap();
        }
        assert_eq!(
            native_original_flux.bound,
            Some([direct_original_bound.lo, direct_original_bound.hi])
        );
        assert_eq!(native_original_flux.spans, direct_original_flux.spans);
        assert_eq!(native_original_flux.domain_cells, 0);
        let support_pair = expression
            .inspect_support_pair(
                retained_ids,
                [[[0., 0.75], [0., 1.]], [[0.9, 1.], [0., 1.]]],
                32,
                1000,
                256,
            )
            .unwrap();
        eprintln!(
            "retained support pair separated={} pending={} cells={} spans={}",
            support_pair.separated.len(),
            support_pair.pending.len(),
            support_pair.cells,
            support_pair.spans
        );
        assert!(support_pair.complete_separated());
        assert!(support_pair.cells <= 32 && support_pair.spans <= 256);
        assert!(!support_pair.separated.is_empty() || !support_pair.pending.is_empty());
        assert!(
            expression
                .inspect_support_pair([retained_ids[0]; 2], [[[0.; 2]; 2]; 2], 1, 100, 100)
                .is_err()
        );

        let removed_product = [[[0.8; 2], [0.5; 2]]; 2];
        let trim_removed = expression
            .inspect_trimmed_pair(retained_ids, removed_product, 1e-8, 1, 1000, 100, 1000)
            .unwrap();
        eprintln!(
            "removed strip product outside={} pending={} domainCells={}",
            trim_removed.outside.len(),
            trim_removed.pending.len(),
            trim_removed.domain_cells
        );
        assert!(trim_removed.complete_excluded());
        assert!(!trim_removed.complete_separated());
        assert_eq!(trim_removed.outside, vec![removed_product]);
        let trim_no_work = expression
            .inspect_trimmed_pair(retained_ids, removed_product, 1e-8, 1, 1000, 100, 0)
            .unwrap();
        assert_eq!(trim_no_work.pending, vec![removed_product]);

        let no_pair_work = expression
            .inspect_shell_supports(0, 2, 1000, 10000)
            .unwrap();
        let count = expression.faces().len();
        assert_eq!(no_pair_work.pairs.len(), count * (count - 1) / 2);
        assert_eq!(no_pair_work.pending_pairs(), no_pair_work.pairs.len());
        for pair in &no_pair_work.pairs {
            assert_eq!(
                pair.inspection.pending,
                vec![pair.faces.map(|i| expression.support_domain(i).unwrap())]
            );
        }
        let all_supports = expression
            .inspect_shell_supports(1024, 2, 1000, 10000)
            .unwrap();
        assert_eq!(all_supports.pairs.len(), count * (count - 1) / 2);
        assert!(all_supports.cells <= 1024 && all_supports.spans <= 10000);
        for pair in &all_supports.pairs {
            assert!(!pair.inspection.pending.is_empty() || !pair.inspection.separated.is_empty());
            if !pair.shared_edges.is_empty() {
                assert!(!pair.inspection.complete_separated());
            }
        }
        eprintln!(
            "all expression supports pairs={} separated={} pending={} cells={} spans={}",
            all_supports.pairs.len(),
            all_supports.pairs.len() - all_supports.pending_pairs(),
            all_supports.pending_pairs(),
            all_supports.cells,
            all_supports.spans
        );

        let support_matched = expression
            .inspect_shell_supports(4096, 8, 1000, 20000)
            .unwrap();
        eprintln!(
            "matched support-only pairs={} excluded={} pending={} cells={} spans={}",
            support_matched.pairs.len(),
            support_matched.pairs.len() - support_matched.pending_pairs(),
            support_matched.pending_pairs(),
            support_matched.cells,
            support_matched.spans
        );
        let trimmed_shell = expression
            .inspect_shell_trimmed(4096, 8, 1000, 20000, 1e-8, 100000, 256)
            .unwrap();
        assert_eq!(trimmed_shell.pairs.len(), count * (count - 1) / 2);
        assert!(
            trimmed_shell.cells <= 4096
                && trimmed_shell.spans <= 20000
                && trimmed_shell.domain_cells <= 100000
        );
        for pair in &trimmed_shell.pairs {
            if !pair.shared_edges.is_empty() {
                assert!(!pair.inspection.complete_excluded());
            }
            assert!(
                !pair.inspection.pending.is_empty()
                    || !pair.inspection.separated.is_empty()
                    || !pair.inspection.outside.is_empty()
            );
        }
        eprintln!(
            "all trimmed expression pairs={} excluded={} pending={} outsideProducts={} cells={} spans={} domainCells={}",
            trimmed_shell.pairs.len(),
            trimmed_shell.pairs.len() - trimmed_shell.pending_pairs(),
            trimmed_shell.pending_pairs(),
            trimmed_shell
                .pairs
                .iter()
                .map(|p| p.inspection.outside.len())
                .sum::<usize>(),
            trimmed_shell.cells,
            trimmed_shell.spans,
            trimmed_shell.domain_cells
        );
        let zero_trim = expression
            .inspect_shell_trimmed(0, 8, 1000, 20000, 1e-8, 0, 256)
            .unwrap();
        assert_eq!(zero_trim.pending_pairs(), count * (count - 1) / 2);

        let mut original_planar = 0;
        let mut planar_work = 0;
        let mut planar_spans = 0;
        let mut original_fiber = 0;
        let mut fiber_work = 0;
        let mut fiber_spans = 0;
        let mut fiber_drivers = 0;
        let mut additional_count = 0;
        let mut additional_work = 0;
        let mut additional_spans = 0;
        let mut additional_drivers = 0;
        for pair in &trimmed_shell.pairs {
            if pair.inspection.complete_excluded() {
                continue;
            }
            let contact = expression
                .qualify_original_planar_contact(
                    pair.faces,
                    100000000 - planar_work,
                    100000 - planar_spans,
                )
                .unwrap();
            planar_work += contact.exact_work;
            planar_spans += contact.spans;
            if let Some(certificate) = contact.certificate {
                original_planar += 1;
                assert_eq!(certificate.faces(), pair.faces);
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        certificate.owners()[side],
                        &expression.faces()[pair.faces[side]]
                    ));
                }
                assert!(
                    expression
                        .qualify_original_planar_contact(pair.faces, 0, 0)
                        .unwrap()
                        .certificate
                        .is_none()
                );
            } else {
                let fiber = expression
                    .qualify_original_fiber_contact(
                        pair.faces,
                        100000000 - fiber_work,
                        100000 - fiber_spans,
                        100000 - fiber_drivers,
                    )
                    .unwrap();
                fiber_work += fiber.exact_work;
                fiber_spans += fiber.spans;
                fiber_drivers += fiber.driver_cells;
                if let Some(certificate) = fiber.certificate {
                    original_fiber += 1;
                    assert_eq!(certificate.faces(), pair.faces);
                    let source_faces = match certificate.proof() {
                        crate::source_profile_body::OriginalFiberProof::Boundary(proof) => {
                            proof.faces()
                        }
                        crate::source_profile_body::OriginalFiberProof::Paired(proof) => {
                            proof.faces()
                        }
                    };
                    let original = pair.faces.map(|id| match expression.faces()[id].kind() {
                        ShellFaceKind::Original(index) => index,
                        _ => panic!("changed face cannot inherit"),
                    });
                    assert!(source_faces == original || source_faces == [original[1], original[0]]);
                    for side in 0..2 {
                        assert!(std::ptr::eq(
                            certificate.owners()[side],
                            &expression.faces()[pair.faces[side]]
                        ));
                    }
                    assert!(
                        expression
                            .qualify_original_fiber_contact(pair.faces, 0, 0, 0)
                            .unwrap()
                            .certificate
                            .is_none()
                    );
                } else {
                    let additional = expression
                        .qualify_original_additional_contact(
                            pair.faces,
                            100000000 - additional_work,
                            100000 - additional_spans,
                            100000 - additional_drivers,
                        )
                        .unwrap();
                    additional_work += additional.exact_work;
                    additional_spans += additional.spans;
                    additional_drivers += additional.driver_cells;
                    if let Some(certificate) = additional.certificate {
                        additional_count += 1;
                        assert_eq!(certificate.faces(), pair.faces);
                        let source_faces = match certificate.proof() {
                            crate::source_profile_body::OriginalAdditionalProof::Vertex(proof) => {
                                proof.faces()
                            }
                            crate::source_profile_body::OriginalAdditionalProof::Interior(
                                proof,
                            ) => proof.faces(),
                        };
                        let original = pair.faces.map(|id| match expression.faces()[id].kind() {
                            ShellFaceKind::Original(index) => index,
                            _ => panic!("changed face cannot inherit"),
                        });
                        assert!(
                            source_faces == original || source_faces == [original[1], original[0]]
                        );
                        for side in 0..2 {
                            assert!(std::ptr::eq(
                                certificate.owners()[side],
                                &expression.faces()[pair.faces[side]]
                            ));
                        }
                        assert!(
                            expression
                                .qualify_original_additional_contact(pair.faces, 0, 0, 0)
                                .unwrap()
                                .certificate
                                .is_none()
                        );
                    }
                }
            }
        }
        eprintln!(
            "expression fresh unchanged planar contacts={} exactWork={} spans={}",
            original_planar, planar_work, planar_spans
        );
        assert_eq!(original_planar, 12);
        eprintln!(
            "expression fresh unchanged fiber contacts={} exactWork={} spans={} drivers={}",
            original_fiber, fiber_work, fiber_spans, fiber_drivers
        );
        assert_eq!(original_fiber, 23);
        eprintln!(
            "expression fresh vertex/interior contacts={} exactWork={} spans={} drivers={}",
            additional_count, additional_work, additional_spans, additional_drivers
        );
        assert_eq!(additional_count, 54);
        assert!(
            expression
                .qualify_original_additional_contact([0, 0], 100, 100, 100)
                .is_err()
        );
        assert!(
            expression
                .qualify_original_additional_contact(retained_ids, 100000000, 100000, 100000)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_original_fiber_contact(retained_ids, 100000000, 100000, 100000)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_original_planar_contact([0, 0], 100, 100)
                .is_err()
        );
        let transition_id = expression
            .faces()
            .iter()
            .position(|f| f.kind() == ShellFaceKind::Transition)
            .unwrap();
        assert!(
            expression
                .qualify_original_planar_contact(
                    [retained_ids[0], transition_id],
                    100000000,
                    100000
                )
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_original_planar_contact(retained_ids, 100000000, 100000)
                .unwrap()
                .certificate
                .is_none()
        );

        for rail in 0..2 {
            let plane = expression
                .qualify_transition_plane_contact(rail, 100000000, 1000, 4096, 100000)
                .unwrap();
            eprintln!(
                "transition plane rail={} qualified={} pending={} cells={} spans={} exactWork={} reason={}",
                rail,
                plane.certificate.is_some(),
                plane.pending.len(),
                plane.cells,
                plane.spans,
                plane.exact_work,
                plane.reason
            );
            eprintln!(
                "transition plane rail={} anchorOrder={:?}",
                rail, plane.anchor_order
            );
            eprintln!(
                "transition plane rail={} pendingDomains={:?}",
                rail, plane.pending
            );
            assert!(plane.cells <= 4096 && plane.spans <= 100000 && plane.exact_work <= 100000000);
            assert!(
                expression
                    .qualify_transition_plane_contact(rail, 0, 0, 0, 0)
                    .unwrap()
                    .certificate
                    .is_none()
            );
            if rail == 0 {
                assert!(plane.certificate.is_some() && plane.pending.is_empty());
            }
            if let Some(certificate) = plane.certificate {
                assert_eq!(certificate.rail(), rail);
                assert!(!certificate.accepted().is_empty());
                assert!(
                    certificate
                        .owners()
                        .iter()
                        .any(|f| std::ptr::eq(*f, &expression.faces()[transition_id]))
                );
            }
        }
        use crate::source_profile_body::{ContactAuditLimits, PairContactProof};
        let audit_limits = ContactAuditLimits {
            cells: 4096,
            cells_per_pair: 8,
            controls_per_enclosure: 1000,
            spans: 100000,
            domain_cells: 100000,
            domain_cells_per_pair: 256,
            exact_work: 100000000,
            driver_cells: 100000,
            tolerance_uv: 1e-8,
        };
        let ruled_profile = expression
            .qualify_transition_profile_ruled_contact(1, &profile_region, 1000, 1000000, 10000)
            .unwrap();
        eprintln!(
            "exact profile ruled contact qualified={} work={} drivers={} reason={}",
            ruled_profile.certificate.is_some(),
            ruled_profile.exact_work,
            ruled_profile.driver_cells,
            ruled_profile.reason
        );
        assert!(
            ruled_profile.certificate.is_some(),
            "exact profile boundary must match owned wall"
        );
        assert!(
            expression
                .qualify_transition_profile_ruled_contact(1, &profile_region, 1000, 0, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        for end in 0..2 {
            let section = expression
                .qualify_transition_section_contact(end, &profile_region, 1000)
                .unwrap();
            let certificate = section.certificate.expect(section.reason);
            assert_eq!(certificate.end(), end);
            assert_eq!(certificate.owners()[0].kind(), ShellFaceKind::Retained(end));
            assert_eq!(certificate.owners()[1].kind(), ShellFaceKind::Transition);
            assert!(
                expression
                    .qualify_transition_section_contact(end, &profile_region, 0)
                    .unwrap()
                    .certificate
                    .is_none()
            );
            assert!(
                expression
                    .qualify_transition_section_contact(end, &profile_region, 1)
                    .unwrap()
                    .certificate
                    .is_none()
            );
        }
        assert!(
            expression
                .qualify_transition_section_contact(2, &profile_region, 1000)
                .is_err()
        );
        let audit = expression
            .inspect_contacts_with_profile_region(audit_limits, &profile_region)
            .unwrap();
        assert!(std::ptr::eq(audit.owners(), expression.faces()));
        assert_eq!(audit.pairs().len(), 406);
        assert_eq!(audit.pending_pairs(), 150);
        let mut kinds = [0usize; 20];
        for pair in audit.pairs() {
            let (id, owners) = match pair.proof() {
                PairContactProof::TransitionXyPlane(c) => (19, Some(c.owners())),
                PairContactProof::ProfilePaired(c) => (18, Some(c.owners())),
                PairContactProof::ProfileRuled(c) => (17, Some(c.image().owners())),
                PairContactProof::ProfileBoundary(c) => (16, Some(c.owners())),
                PairContactProof::Hull(c) => (8, Some(c.owners())),
                PairContactProof::Pole(c) => (9, Some(c.owners())),
                PairContactProof::ProfileHull(c) => (10, Some(c.owners())),
                PairContactProof::Boundary(c) => (11, Some(c.owners())),
                PairContactProof::TransitionXyHull(c) => (12, Some(c.owners())),
                PairContactProof::TransitionXyBounds(c) => (13, Some(c.owners())),
                PairContactProof::ProfileVertex(c) => (14, Some(c.owners())),
                PairContactProof::TransitionCoordinate(c) => (15, Some(c.owners())),
                PairContactProof::Pending => (0, None),
                PairContactProof::Excluded => (1, None),
                PairContactProof::Planar(c) => (2, Some(c.owners())),
                PairContactProof::Fiber(c) => (3, Some(c.owners())),
                PairContactProof::VertexOrInterior(c) => (4, Some(c.owners())),
                PairContactProof::TransitionPlane(c) => (5, Some([c.owners()[1], c.owners()[0]])),
                PairContactProof::TransitionRuled(c) => (6, Some([c.owners()[1], c.owners()[0]])),
                PairContactProof::TransitionSection(c) => (7, Some(c.owners())),
            };
            kinds[id] += 1;
            if let Some(owners) = owners {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        owners[side],
                        &expression.faces()[pair.support().faces[side]]
                    ));
                }
            }
        }
        assert_eq!(
            kinds,
            [
                150, 163, 12, 23, 54, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ]
        );
        let diagnostics = audit.diagnostics();
        assert_eq!(diagnostics["pendingPairs"].as_u64(), Some(150));
        if let Ok(path) = std::env::var("BREP_CONTACT_AUDIT_PATH") {
            std::fs::write(path, diagnostics.to_string())
                .expect("write requested contact audit diagnostics");
        }
        assert_eq!(diagnostics["pairs"].as_array().unwrap().len(), 406);
        let binary = value_codec::encode_binary(&diagnostics).unwrap();
        assert_eq!(value_codec::decode_binary(&binary).unwrap(), diagnostics);
        let json = value_codec::from_str::<value_codec::Value>(&diagnostics.to_string()).unwrap();
        assert_eq!(json["pendingPairs"].as_u64(), Some(150));
        assert_eq!(json["pairs"].as_array().unwrap().len(), 406);
        eprintln!(
            "expression audit diagnostics binaryBytes={} jsonBytes={}",
            binary.len(),
            diagnostics.to_string().len()
        );

        let (cells, spans, domains, work, drivers) = audit.work();
        assert!(
            cells <= audit_limits.cells
                && spans <= audit_limits.spans
                && domains <= audit_limits.domain_cells
                && work <= audit_limits.exact_work
                && drivers <= audit_limits.driver_cells
        );
        eprintln!(
            "combined expression audit counts={:?} cells={} spans={} domains={} exactWork={} drivers={}",
            kinds, cells, spans, domains, work, drivers
        );
        let exhausted = expression
            .inspect_contacts(ContactAuditLimits {
                cells: 0,
                spans: 0,
                domain_cells: 0,
                exact_work: 0,
                driver_cells: 0,
                ..audit_limits
            })
            .unwrap();
        assert_eq!(exhausted.pairs().len(), 406);
        assert_eq!(exhausted.pending_pairs(), 406);

        let refined = expression
            .inspect_contacts_with_profile_region_refined(audit_limits, &profile_region, 32)
            .unwrap();
        assert_eq!(refined.pairs().len(), 406);
        assert!(refined.pending_pairs() <= audit.pending_pairs());
        for (old, new) in audit.pairs().iter().zip(refined.pairs()) {
            assert_eq!(old.support().faces, new.support().faces);
            if !matches!(old.proof(), PairContactProof::Pending) {
                assert_eq!(old.proof().kind(), new.proof().kind());
            } else {
                assert!(matches!(
                    new.proof(),
                    PairContactProof::Pending | PairContactProof::Excluded
                ));
            }
        }
        let (cells, spans, domains, work, drivers) = refined.work();
        assert!(
            cells <= audit_limits.cells
                && spans <= audit_limits.spans
                && domains <= audit_limits.domain_cells
                && work <= audit_limits.exact_work
                && drivers <= audit_limits.driver_cells
        );
        let refined_diagnostics = refined.diagnostics();
        eprintln!(
            "refined contact audit pending={} cells={} spans={} domains={} reasons={} faceKinds={}",
            refined.pending_pairs(),
            cells,
            spans,
            domains,
            refined_diagnostics["pendingByReason"],
            refined_diagnostics["pendingByFaceKinds"]
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_REFINED_AUDIT_PATH") {
            std::fs::write(path, refined_diagnostics.to_string())
                .expect("write requested refined contact diagnostics");
        }

        let all_refined = expression
            .inspect_all_contacts_with_profile_region_refined(audit_limits, &profile_region, 32)
            .unwrap();
        assert_eq!(all_refined.pairs().len(), 406);
        assert!(all_refined.pending_pairs() <= audit.pending_pairs());
        for (old, new) in audit.pairs().iter().zip(all_refined.pairs()) {
            assert_eq!(old.support().faces, new.support().faces);
            if !matches!(old.proof(), PairContactProof::Pending) {
                assert_eq!(old.proof().kind(), new.proof().kind());
            } else {
                assert!(matches!(
                    new.proof(),
                    PairContactProof::Pending | PairContactProof::Excluded
                ));
            }
        }
        let (cells, spans, domains, work, drivers) = all_refined.work();
        assert!(
            cells <= audit_limits.cells
                && spans <= audit_limits.spans
                && domains <= audit_limits.domain_cells
                && work <= audit_limits.exact_work
                && drivers <= audit_limits.driver_cells
        );
        let all_diagnostics = all_refined.diagnostics();
        eprintln!(
            "all-face refined pending={} cells={} spans={} domains={} groups={}",
            all_refined.pending_pairs(),
            cells,
            spans,
            domains,
            all_diagnostics["pendingByFaceKinds"]
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_ALL_REFINED_AUDIT_PATH") {
            std::fs::write(path, all_diagnostics.to_string()).unwrap();
        }

        let mut resumed = refined;
        assert!(expression.qualify_embedding(&resumed).is_none());
        let previous = resumed.diagnostics();
        let previous_work = resumed.work();
        assert!(
            expression
                .resume_contact_audit(
                    &mut resumed,
                    ContactAuditLimits {
                        cells: 0,
                        ..audit_limits
                    },
                    32
                )
                .is_err()
        );
        assert_eq!(resumed.diagnostics(), previous);
        expression
            .resume_contact_audit(&mut resumed, audit_limits, 0)
            .unwrap();
        assert_eq!(resumed.diagnostics(), previous);
        let cumulative = ContactAuditLimits {
            cells: 8192,
            ..audit_limits
        };
        expression
            .resume_contact_audit(&mut resumed, cumulative, 64)
            .unwrap();
        assert!(resumed.pending_pairs() <= 129);
        for (old, new) in previous["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
        }
        let (cells, spans, domains, work, drivers) = resumed.work();
        assert!(cells >= previous_work.0 && spans >= previous_work.1 && domains >= previous_work.2);
        assert!(
            cells <= cumulative.cells
                && spans <= cumulative.spans
                && domains <= cumulative.domain_cells
                && work <= cumulative.exact_work
                && drivers <= cumulative.driver_cells
        );
        eprintln!(
            "resumed audit pending={} cells={} spans={} domains={}",
            resumed.pending_pairs(),
            cells,
            spans,
            domains
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_RESUMED_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_linear = resumed.diagnostics();
        expression
            .qualify_remaining_original_contacts(&mut resumed, cumulative, 0, 256)
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_linear);
        assert!(
            expression
                .qualify_remaining_original_contacts(&mut resumed, cumulative, 100001, 256)
                .is_err()
        );
        assert_eq!(resumed.diagnostics(), before_linear);
        let prior_pending = resumed.pending_pairs();
        expression
            .qualify_remaining_original_contacts(&mut resumed, cumulative, 10000, 256)
            .unwrap();
        assert!(resumed.pending_pairs() <= prior_pending && resumed.linear_cells() <= 10000);
        for (old, new) in before_linear["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
        }
        let (cells, spans, domains, work, drivers) = resumed.work();
        assert!(
            cells <= cumulative.cells
                && spans <= cumulative.spans
                && domains <= cumulative.domain_cells
                && work <= cumulative.exact_work
                && drivers <= cumulative.driver_cells
        );
        eprintln!(
            "linear-chart contact audit pending={} linearCells={} spans={} work={} drivers={}",
            resumed.pending_pairs(),
            resumed.linear_cells(),
            spans,
            work,
            drivers
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_LINEAR_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_hulls = resumed.diagnostics();
        let prior_pending = resumed.pending_pairs();
        let spent_work = resumed.work().3;
        expression
            .qualify_remaining_original_hulls(
                &mut resumed,
                ContactAuditLimits {
                    exact_work: spent_work,
                    ..cumulative
                },
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_hulls);
        assert!(
            expression
                .qualify_original_hull_contact([0, 0], 1000)
                .is_err()
        );
        assert!(
            expression
                .qualify_original_hull_contact([1, 2], 0)
                .unwrap()
                .certificate
                .is_none()
        );
        expression
            .qualify_remaining_original_hulls(&mut resumed, cumulative)
            .unwrap();
        assert!(
            resumed.pending_pairs() <= prior_pending && resumed.work().3 <= cumulative.exact_work
        );
        for (old, new) in before_hulls["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::Hull(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
            }
        }
        eprintln!(
            "fresh hull contact audit pending={} work={}",
            resumed.pending_pairs(),
            resumed.work().3
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_HULL_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_poles = resumed.diagnostics();
        let prior_pending = resumed.pending_pairs();
        let spent_work = resumed.work().3;
        expression
            .qualify_remaining_original_poles(
                &mut resumed,
                ContactAuditLimits {
                    exact_work: spent_work,
                    ..cumulative
                },
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_poles);
        assert!(
            expression
                .qualify_original_pole_contact([1, 1], 1000, 1000)
                .is_err()
        );
        assert!(
            expression
                .qualify_original_pole_contact([1, 2], 1000, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        expression
            .qualify_remaining_original_poles(&mut resumed, cumulative)
            .unwrap();
        assert!(
            resumed.pending_pairs() <= prior_pending
                && resumed.work().3 <= cumulative.exact_work
                && resumed.work().4 <= cumulative.driver_cells
        );
        for (old, new) in before_poles["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::Pole(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
            }
        }
        eprintln!(
            "fresh pole contact audit pending={} work={} drivers={}",
            resumed.pending_pairs(),
            resumed.work().3,
            resumed.work().4
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_POLE_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_profile_hulls = resumed.diagnostics();
        let prior_pending = resumed.pending_pairs();
        let spent_work = resumed.work().3;
        expression
            .qualify_remaining_profile_hulls(
                &mut resumed,
                ContactAuditLimits {
                    exact_work: spent_work,
                    ..cumulative
                },
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_profile_hulls);
        assert!(
            expression
                .qualify_profile_hull_contact([0, 0], 1000)
                .is_err()
        );
        assert!(
            expression
                .qualify_profile_hull_contact([0, 1], 0)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_profile_hull_contact([1, 2], 1000)
                .unwrap()
                .certificate
                .is_none()
        );
        expression
            .qualify_remaining_profile_hulls(&mut resumed, cumulative)
            .unwrap();
        assert!(
            resumed.pending_pairs() <= prior_pending && resumed.work().3 <= cumulative.exact_work
        );
        for (old, new) in before_profile_hulls["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::ProfileHull(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                    if matches!(
                        expression.faces()[new.support().faces[side]].kind(),
                        ShellFaceKind::Retained(_)
                    ) {
                        assert!(c.source().material_hulls()[side].is_none());
                    }
                }
            }
        }
        eprintln!(
            "profile control-hull audit pending={} work={}",
            resumed.pending_pairs(),
            resumed.work().3
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_PROFILE_HULL_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let mut profile_hull_positive = false;
        for (id, face) in expression.faces().iter().enumerate() {
            if !matches!(face.kind(), ShellFaceKind::Original(_)) {
                continue;
            }
            let report = expression
                .qualify_profile_hull_contact([0, id], 100000)
                .unwrap();
            assert!(report.exact_work <= 100000);
            if let Some(c) = report.certificate {
                assert!(std::ptr::eq(c.owners()[0], &expression.faces()[0]));
                assert!(std::ptr::eq(c.owners()[1], face));
                assert!(c.source().material_hulls().iter().all(Option::is_none));
                assert!(
                    expression
                        .qualify_profile_hull_contact([id, 0], 100000)
                        .unwrap()
                        .certificate
                        .is_some()
                );
                profile_hull_positive = true;
                break;
            }
        }
        assert!(
            profile_hull_positive,
            "full positive control hulls must separate at least one actual original/profile pair"
        );

        let before_boundaries = resumed.diagnostics();
        let prior_pending = resumed.pending_pairs();
        let spent_work = resumed.work().3;
        expression
            .qualify_remaining_original_boundaries(
                &mut resumed,
                ContactAuditLimits {
                    exact_work: spent_work,
                    ..cumulative
                },
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_boundaries);
        assert!(
            expression
                .qualify_original_boundary_contact([10, 12], f64::NAN, 1000, 1000)
                .is_err()
        );
        assert!(
            expression
                .qualify_original_boundary_contact([10, 12], 1e-8, 1000, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        expression
            .qualify_remaining_original_boundaries(&mut resumed, cumulative)
            .unwrap();
        assert!(
            resumed.pending_pairs() <= prior_pending
                && resumed.work().3 <= cumulative.exact_work
                && resumed.work().4 <= cumulative.driver_cells
        );
        for (old, new) in before_boundaries["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::Boundary(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
            }
        }
        eprintln!(
            "fresh boundary contact audit pending={} work={} drivers={}",
            resumed.pending_pairs(),
            resumed.work().3,
            resumed.work().4
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_BOUNDARY_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_xy = resumed.diagnostics();
        let prior_pending = resumed.pending_pairs();
        let spent_work = resumed.work().3;
        expression
            .qualify_remaining_transition_xy_hulls(
                &mut resumed,
                ContactAuditLimits {
                    exact_work: spent_work,
                    ..cumulative
                },
                &profile_region,
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_xy);
        assert!(
            expression
                .qualify_transition_xy_hull_contact([0, 0], &profile_region, 1000)
                .is_err()
        );
        assert!(
            expression
                .qualify_transition_xy_hull_contact([1, transition_id], &profile_region, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_transition_xy_hull_contact([0, 1], &profile_region, 1000)
                .unwrap()
                .certificate
                .is_none()
        );
        let mut xy_positive = false;
        let mut xy_work = 0u64;
        let mut xy_reasons = std::collections::BTreeMap::new();
        for (id, face) in expression.faces().iter().enumerate() {
            if !matches!(face.kind(), ShellFaceKind::Original(_)) {
                continue;
            }
            let report = expression
                .qualify_transition_xy_hull_contact([id, transition_id], &profile_region, 100000)
                .unwrap();
            assert!(report.exact_work <= 100000);
            xy_work += report.exact_work;
            *xy_reasons.entry(report.reason).or_insert(0usize) += 1;
            if let Some(c) = report.certificate {
                assert!(std::ptr::eq(c.owners()[0], face));
                assert!(std::ptr::eq(
                    c.owners()[1],
                    &expression.faces()[transition_id]
                ));
                assert!(c.source().material_hulls().iter().all(Option::is_none));
                assert!(
                    expression
                        .qualify_transition_xy_hull_contact(
                            [transition_id, id],
                            &profile_region,
                            100000
                        )
                        .unwrap()
                        .certificate
                        .is_some()
                );
                xy_positive = true;
                break;
            }
        }
        eprintln!(
            "transition full XY hull positive={xy_positive} work={xy_work} reasons={xy_reasons:?}"
        );
        assert!(
            xy_work > 0,
            "actual owned image must reach fresh hull predicates"
        );
        expression
            .qualify_remaining_transition_xy_hulls(&mut resumed, cumulative, &profile_region)
            .unwrap();
        assert!(
            resumed.pending_pairs() <= prior_pending && resumed.work().3 <= cumulative.exact_work
        );
        for (old, new) in before_xy["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::TransitionXyHull(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
            }
        }
        eprintln!(
            "transition XY hull audit pending={} work={}",
            resumed.pending_pairs(),
            resumed.work().3
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_TRANSITION_XY_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_bounds = resumed.diagnostics();
        let prior_pending = resumed.pending_pairs();
        let spent_spans = resumed.work().1;
        expression
            .qualify_remaining_transition_xy_bounds(
                &mut resumed,
                ContactAuditLimits {
                    spans: spent_spans,
                    ..cumulative
                },
                &profile_region,
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_bounds);
        assert!(
            expression
                .qualify_transition_xy_bounds_contact([0, 0], &profile_region, 100)
                .is_err()
        );
        assert!(
            expression
                .qualify_transition_xy_bounds_contact([1, transition_id], &profile_region, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_transition_xy_bounds_contact([0, 1], &profile_region, 100)
                .unwrap()
                .certificate
                .is_none()
        );
        expression
            .qualify_remaining_transition_xy_bounds(&mut resumed, cumulative, &profile_region)
            .unwrap();
        assert!(resumed.pending_pairs() <= prior_pending && resumed.work().1 <= cumulative.spans);
        for (old, new) in before_bounds["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::TransitionXyBounds(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
                let b = c.bounds();
                let a = c.axis();
                assert!(b[0][a][1] < b[1][a][0] || b[1][a][1] < b[0][a][0]);
                assert!(
                    expression
                        .qualify_transition_xy_bounds_contact(
                            [new.support().faces[1], new.support().faces[0]],
                            &profile_region,
                            1000
                        )
                        .unwrap()
                        .certificate
                        .is_some()
                );
            }
        }
        eprintln!(
            "transition clipped XY bounds audit pending={} spans={}",
            resumed.pending_pairs(),
            resumed.work().1
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_TRANSITION_XY_BOUNDS_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_material = resumed.diagnostics();
        let prior_pending = resumed.pending_pairs();
        let spent_work = resumed.work().3;
        expression
            .qualify_remaining_transition_xy_material_bounds(
                &mut resumed,
                ContactAuditLimits {
                    exact_work: spent_work,
                    ..cumulative
                },
                &profile_region,
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_material);
        assert!(
            expression
                .qualify_transition_xy_material_bounds_contact(
                    [1, transition_id],
                    &profile_region,
                    100,
                    100000001
                )
                .is_err()
        );
        expression
            .qualify_remaining_transition_xy_material_bounds(
                &mut resumed,
                cumulative,
                &profile_region,
            )
            .unwrap();
        assert!(
            resumed.pending_pairs() <= prior_pending
                && resumed.work().1 <= cumulative.spans
                && resumed.work().3 <= cumulative.exact_work
        );
        for (old, new) in before_material["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if old["proof"].as_str() == Some("pending") {
                if let PairContactProof::TransitionXyBounds(c) = new.proof() {
                    assert!(c.material_hull().is_some());
                    for side in 0..2 {
                        assert!(std::ptr::eq(
                            c.owners()[side],
                            &expression.faces()[new.support().faces[side]]
                        ));
                    }
                    let b = c.bounds();
                    let a = c.axis();
                    assert!(b[0][a][1] < b[1][a][0] || b[1][a][1] < b[0][a][0]);
                    let reverse = expression
                        .qualify_transition_xy_material_bounds_contact(
                            [new.support().faces[1], new.support().faces[0]],
                            &profile_region,
                            1000,
                            10000000,
                        )
                        .unwrap();
                    assert!(reverse.certificate.is_some());
                    assert!(reverse.exact_work <= 10000000);
                }
            }
        }
        eprintln!(
            "transition material XY bounds audit pending={} spans={} work={}",
            resumed.pending_pairs(),
            resumed.work().1,
            resumed.work().3
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_TRANSITION_XY_MATERIAL_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let expanded = ContactAuditLimits {
            cells: 16384,
            ..cumulative
        };
        let before_coordinate = resumed.diagnostics();
        let spent_spans = resumed.work().1;
        expression
            .qualify_remaining_transition_coordinates(
                &mut resumed,
                ContactAuditLimits {
                    spans: spent_spans,
                    ..expanded
                },
                256,
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_coordinate);
        assert!(
            expression
                .qualify_transition_coordinate_contact([4, transition_id], 0, 1000, 100)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_transition_coordinate_contact([0, 0], 1, 1, 1)
                .is_err()
        );
        expression
            .qualify_remaining_transition_coordinates(&mut resumed, expanded, 256)
            .unwrap();
        assert!(
            resumed.pending_pairs() <= before_coordinate["pendingPairs"].as_u64().unwrap() as usize
        );
        assert!(resumed.work().0 <= expanded.cells && resumed.work().1 <= expanded.spans);
        for (old, new) in before_coordinate["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::TransitionCoordinate(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
                assert!(!c.cover().is_empty());
                for (_, b) in c.cover() {
                    assert!(b[1] < c.value() || b[0] > c.value());
                }
                assert!(
                    expression
                        .qualify_transition_coordinate_contact(
                            [new.support().faces[1], new.support().faces[0]],
                            256,
                            1000,
                            10000
                        )
                        .unwrap()
                        .certificate
                        .is_some()
                );
            }
        }
        eprintln!(
            "coordinate transition audit pending={} cells={} spans={}",
            resumed.pending_pairs(),
            resumed.work().0,
            resumed.work().1
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_TRANSITION_COORDINATE_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }
        let before_transition_refinement = resumed.diagnostics();
        expression
            .resume_transition_contact_audit(&mut resumed, expanded, 256)
            .unwrap();
        assert!(
            resumed.pending_pairs()
                <= before_transition_refinement["pendingPairs"]
                    .as_u64()
                    .unwrap() as usize
        );
        assert!(
            resumed.work().0 <= expanded.cells
                && resumed.work().1 <= expanded.spans
                && resumed.work().2 <= expanded.domain_cells
        );
        for (old, new) in before_transition_refinement["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if !new.support().faces.contains(&transition_id) {
                assert_eq!(old["reason"].as_str(), Some(new.reason()));
            }
        }
        eprintln!(
            "focused transition support audit pending={} cells={} spans={}",
            resumed.pending_pairs(),
            resumed.work().0,
            resumed.work().1
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_TRANSITION_SUPPORT_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_vertices = resumed.diagnostics();
        let spent_work = resumed.work().3;
        expression
            .qualify_remaining_profile_vertices(
                &mut resumed,
                ContactAuditLimits {
                    exact_work: spent_work,
                    ..expanded
                },
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_vertices);
        assert!(
            expression
                .qualify_profile_vertex_contact([0, 0], 1000)
                .is_err()
        );
        assert!(
            expression
                .qualify_profile_vertex_contact([0, 1], 0)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_profile_vertex_contact([1, transition_id], 1000)
                .unwrap()
                .certificate
                .is_none()
        );
        expression
            .qualify_remaining_profile_vertices(&mut resumed, expanded)
            .unwrap();
        assert!(
            resumed.pending_pairs() <= before_vertices["pendingPairs"].as_u64().unwrap() as usize
                && resumed.work().3 <= expanded.exact_work
        );
        for (old, new) in before_vertices["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::ProfileVertex(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
                assert_eq!(c.source().point(), c.image().point());
                if let Some(domain) = c.excluded_domain() {
                    let retained = new
                        .support()
                        .faces
                        .iter()
                        .copied()
                        .find(|&i| {
                            matches!(expression.faces()[i].kind(), ShellFaceKind::Retained(_))
                        })
                        .unwrap();
                    assert_eq!(domain, expression.support_domain(retained).unwrap());
                    assert_eq!(new.proof().kind(), "profile-vertex-excluded");
                }
                assert!(
                    expression
                        .qualify_profile_vertex_contact(
                            [new.support().faces[1], new.support().faces[0]],
                            10000000
                        )
                        .unwrap()
                        .certificate
                        .is_some()
                );
            }
        }
        assert!(
            expression
                .qualify_profile_ruled_image([0, 0], 1e-8, 1000, 1000000, 1000)
                .is_err()
        );
        assert!(
            expression
                .qualify_profile_ruled_image([0, 2], 1e-8, 1000, 0, 1000)
                .unwrap()
                .certificate
                .is_none()
        );
        for ids in [[0, 2], [2, 27]] {
            for order in [ids, [ids[1], ids[0]]] {
                let report = expression
                    .qualify_profile_ruled_image(order, 1e-8, 1000, 10000000, 10000)
                    .unwrap();
                eprintln!(
                    "retained wall image pair={order:?} qualified={} work={} reason={}",
                    report.certificate.is_some(),
                    report.exact_work,
                    report.reason
                );
                let proof = report.certificate.expect(report.reason);
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        proof.owners()[side],
                        &expression.faces()[order[side]]
                    ));
                }
                assert_eq!(proof.natural(), 2);
                assert_eq!(proof.profile().axes(), [0, 1]);
                assert!(report.exact_work <= 10000000 && report.boundary_cells <= 10000);
            }
        }
        assert!(
            expression
                .qualify_profile_ruled_contact([0, 2], 1e-8, 1000, 0, 10000, 10000)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_profile_ruled_contact([0, 2], 1e-8, 1000, 10000000, 10000, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        for ids in [[0, 2], [2, 27]] {
            for order in [ids, [ids[1], ids[0]]] {
                let report = expression
                    .qualify_profile_ruled_contact(order, 1e-8, 1000, 10000000, 10000, 10000)
                    .unwrap();
                eprintln!(
                    "retained ruled contact pair={order:?} qualified={} work={} drivers={} reason={}",
                    report.certificate.is_some(),
                    report.exact_work,
                    report.driver_cells,
                    report.reason
                );
                let proof = report.certificate.expect(report.reason);
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        proof.image().owners()[side],
                        &expression.faces()[order[side]]
                    ));
                }
                assert!(std::ptr::eq(
                    proof.boundary().profile(),
                    proof.image().profile().profile()
                ));
                for addresses in proof.shared() {
                    assert!(expression.pairs().iter().any(|p| p.uses() == *addresses));
                }
                assert!(
                    report.exact_work <= 10000000
                        && report.boundary_cells <= 10000
                        && report.driver_cells <= 10000
                );
            }
        }
        assert!(
            expression
                .qualify_profile_paired_contact([0, 0], 1000, 100)
                .is_err()
        );
        assert!(
            expression
                .qualify_profile_paired_contact([0, 5], 0, 100)
                .unwrap()
                .certificate
                .is_none()
        );
        for ids in [[0, 5], [5, 0]] {
            let report = expression
                .qualify_profile_paired_contact(ids, 10000000, 10000)
                .unwrap();
            eprintln!(
                "retained paired contact pair={ids:?} qualified={} work={} drivers={} reason={}",
                report.certificate.is_some(),
                report.exact_work,
                report.driver_cells,
                report.reason
            );
            let proof = report.certificate.expect(report.reason);
            for side in 0..2 {
                assert!(std::ptr::eq(
                    proof.owners()[side],
                    &expression.faces()[ids[side]]
                ));
            }
            for addresses in proof.shared() {
                assert!(expression.pairs().iter().any(|p| p.uses() == *addresses));
            }
            assert!(proof.image().image().poles().is_empty());
            assert!(report.exact_work <= 10000000 && report.driver_cells <= 10000);
        }
        assert!(
            expression
                .qualify_profile_boundary_contact([0, 0], 1000, 100)
                .is_err()
        );
        assert!(
            expression
                .qualify_profile_boundary_contact([0, 1], 0, 100)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_profile_boundary_contact([0, 1], 1000, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        for ids in [[0, 1], [1, 27]] {
            let report = expression
                .qualify_profile_boundary_contact(ids, 10000000, 10000)
                .unwrap();
            eprintln!(
                "retained boundary pair={ids:?} qualified={} work={} drivers={} reason={}",
                report.certificate.is_some(),
                report.exact_work,
                report.driver_cells,
                report.reason
            );
            let proof = report.certificate.expect(report.reason);
            for side in 0..2 {
                assert!(std::ptr::eq(
                    proof.owners()[side],
                    &expression.faces()[ids[side]]
                ));
            }
            assert!(!proof.shared().is_empty());
            assert_eq!(
                proof.source().fiber().plane(),
                proof.image().image().plane()
            );
            assert!(report.exact_work <= 10000000 && report.driver_cells <= 10000);
            assert!(
                expression
                    .qualify_profile_boundary_contact([ids[1], ids[0]], 10000000, 10000)
                    .unwrap()
                    .certificate
                    .is_some()
            );
        }
        eprintln!(
            "profile vertex audit pending={} work={}",
            resumed.pending_pairs(),
            resumed.work().3
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_PROFILE_VERTEX_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_profile_boundary = resumed.diagnostics();
        let prior_pending = resumed.pending_pairs();
        let spent_profile_boundary = ContactAuditLimits {
            exact_work: resumed.work().3,
            driver_cells: resumed.work().4,
            ..cumulative
        };
        expression
            .qualify_remaining_profile_boundaries(&mut resumed, spent_profile_boundary)
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_profile_boundary);
        expression
            .qualify_remaining_profile_boundaries(&mut resumed, cumulative)
            .unwrap();
        assert!(
            resumed.pending_pairs() <= prior_pending
                && resumed.work().3 <= cumulative.exact_work
                && resumed.work().4 <= cumulative.driver_cells
        );
        for (old, new) in before_profile_boundary["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::ProfileBoundary(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
                for addresses in c.shared() {
                    assert!(expression.pairs().iter().any(|p| p.uses() == *addresses));
                }
                assert_eq!(c.source().fiber().plane(), c.image().image().plane());
                assert!(
                    expression
                        .qualify_profile_boundary_contact(
                            [new.support().faces[1], new.support().faces[0]],
                            10000000,
                            10000
                        )
                        .unwrap()
                        .certificate
                        .is_some()
                );
            }
        }
        eprintln!(
            "profile boundary audit pending={} work={} drivers={}",
            resumed.pending_pairs(),
            resumed.work().3,
            resumed.work().4
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_PROFILE_BOUNDARY_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_profile_ruled = resumed.diagnostics();
        let spent_profile_ruled = ContactAuditLimits {
            exact_work: resumed.work().3,
            driver_cells: resumed.work().4,
            domain_cells: resumed.work().2,
            ..cumulative
        };
        expression
            .qualify_remaining_profile_ruled_contacts(&mut resumed, spent_profile_ruled)
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_profile_ruled);
        let prior_pending = resumed.pending_pairs();
        expression
            .qualify_remaining_profile_ruled_contacts(&mut resumed, cumulative)
            .unwrap();
        assert!(
            resumed.pending_pairs() <= prior_pending
                && resumed.work().3 <= cumulative.exact_work
                && resumed.work().4 <= cumulative.driver_cells
                && resumed.work().2 <= cumulative.domain_cells
        );
        for (old, new) in before_profile_ruled["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::ProfileRuled(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.image().owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
                assert!(std::ptr::eq(
                    c.boundary().profile(),
                    c.image().profile().profile()
                ));
                for addresses in c.shared() {
                    assert!(expression.pairs().iter().any(|p| p.uses() == *addresses));
                }
            }
        }
        eprintln!(
            "profile ruled audit pending={} work={} drivers={}",
            resumed.pending_pairs(),
            resumed.work().3,
            resumed.work().4
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_PROFILE_RULED_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_profile_paired = resumed.diagnostics();
        let spent_profile_paired = ContactAuditLimits {
            exact_work: resumed.work().3,
            driver_cells: resumed.work().4,
            ..cumulative
        };
        expression
            .qualify_remaining_profile_paired_contacts(&mut resumed, spent_profile_paired)
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_profile_paired);
        let prior_pending = resumed.pending_pairs();
        expression
            .qualify_remaining_profile_paired_contacts(&mut resumed, cumulative)
            .unwrap();
        assert!(
            resumed.pending_pairs() <= prior_pending
                && resumed.work().3 <= cumulative.exact_work
                && resumed.work().4 <= cumulative.driver_cells
        );
        for (old, new) in before_profile_paired["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::ProfilePaired(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
                for addresses in c.shared() {
                    assert!(expression.pairs().iter().any(|p| p.uses() == *addresses));
                }
                assert!(c.image().image().poles().is_empty());
            }
        }
        assert!(
            expression
                .qualify_transition_xy_material_plane(
                    [6, transition_id],
                    &profile_region,
                    0,
                    10000000
                )
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            expression
                .qualify_transition_xy_material_plane([6, transition_id], &profile_region, 10000, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        for ids in [[6, transition_id], [transition_id, 6]] {
            let report = expression
                .qualify_transition_xy_material_plane(ids, &profile_region, 10000, 10000000)
                .unwrap();
            eprintln!(
                "transition material XY plane pair={ids:?} qualified={} work={} spans={} reason={}",
                report.certificate.is_some(),
                report.exact_work,
                report.spans,
                report.reason
            );
            let proof = report.certificate.expect(report.reason);
            for side in 0..2 {
                assert!(std::ptr::eq(
                    proof.owners()[side],
                    &expression.faces()[ids[side]]
                ));
            }
            assert_eq!(proof.domain(), profile_region.source_domain());
            assert_eq!(
                [proof.plane()[0][0], proof.plane()[0][1]],
                [proof.plane()[2][0], proof.plane()[2][1]]
            );
            assert!(report.exact_work <= 10000000 && report.spans <= 10000);
        }
        eprintln!(
            "profile paired audit pending={} work={} drivers={}",
            resumed.pending_pairs(),
            resumed.work().3,
            resumed.work().4
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_PROFILE_PAIRED_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        let before_xy_plane = resumed.diagnostics();
        let spent_xy_exact = ContactAuditLimits {
            exact_work: resumed.work().3,
            ..cumulative
        };
        expression
            .qualify_remaining_transition_xy_material_planes(
                &mut resumed,
                spent_xy_exact,
                &profile_region,
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_xy_plane);
        let spent_xy_spans = ContactAuditLimits {
            spans: resumed.work().1,
            ..cumulative
        };
        expression
            .qualify_remaining_transition_xy_material_planes(
                &mut resumed,
                spent_xy_spans,
                &profile_region,
            )
            .unwrap();
        assert_eq!(resumed.diagnostics(), before_xy_plane);
        let prior_pending = resumed.pending_pairs();
        expression
            .qualify_remaining_transition_xy_material_planes(
                &mut resumed,
                cumulative,
                &profile_region,
            )
            .unwrap();
        assert!(
            resumed.pending_pairs() <= prior_pending
                && resumed.work().3 <= cumulative.exact_work
                && resumed.work().1 <= cumulative.spans
        );
        for (old, new) in before_xy_plane["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .zip(resumed.pairs())
        {
            if old["proof"].as_str() != Some("pending") {
                assert_eq!(old["proof"].as_str(), Some(new.proof().kind()));
            }
            if let PairContactProof::TransitionXyPlane(c) = new.proof() {
                for side in 0..2 {
                    assert!(std::ptr::eq(
                        c.owners()[side],
                        &expression.faces()[new.support().faces[side]]
                    ));
                }
                assert_eq!(c.domain(), profile_region.source_domain());
                assert_eq!(
                    [c.plane()[0][0], c.plane()[0][1]],
                    [c.plane()[2][0], c.plane()[2][1]]
                );
                assert!(
                    expression
                        .qualify_transition_xy_material_plane(
                            [new.support().faces[1], new.support().faces[0]],
                            &profile_region,
                            10000,
                            10000000
                        )
                        .unwrap()
                        .certificate
                        .is_some()
                );
            }
        }
        eprintln!(
            "transition material XY plane audit pending={} work={} spans={}",
            resumed.pending_pairs(),
            resumed.work().3,
            resumed.work().1
        );
        if let Ok(path) = std::env::var("BREP_CONTACT_TRANSITION_XY_PLANE_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }
        assert_eq!(
            resumed.complete_cross_face_contacts(),
            resumed.pending_pairs() == 0
        );
        assert!(
            resumed.complete_cross_face_contacts(),
            "Transition acceptance requires every cross-face contact: {} pairs remain",
            resumed.pending_pairs()
        );
        if resumed.complete_cross_face_contacts() {
            let geometry = expression
                .qualify_embedding(&resumed)
                .expect("complete owned chart/contact embedding");
            assert!(std::ptr::eq(geometry.shell(), &expression));
            assert!(std::ptr::eq(geometry.contacts(), &resumed));
            if volume_only {
                let qualification = geometry
                    .qualify_volume(crate::source_profile_body::ExpressionVolumeLimits {
                        axis: 2,
                        origin: 0.,
                        absolute_error: 20.,
                        tolerance_uv: 1e-8,
                        controls_per_enclosure: 1000,
                        cells: 32768,
                        spans: 200000,
                        domain_cells: 100000,
                        domain_cells_per_rectangle: 256,
                    })
                    .unwrap();
                let certificate = qualification
                    .certificate
                    .expect(qualification.report.reason);
                assert!(std::ptr::eq(certificate.geometry(), &geometry));
                let bounds = certificate.positive_bounds();
                assert!(bounds[0] > 0. && bounds[0] <= bounds[1]);
                assert!(bounds[1] - bounds[0] <= 20.);
                if let Some(path) = std::env::var_os("CAD_PARTIAL_RATIONAL_VOLUME_REPORT_PATH") {
                    let r = &qualification.report;
                    let report = value_codec::json!({"kind":"partial-rational-transition-oriented-volume","complete":r.complete,
                        "signedBounds":certificate.signed_bounds(),"positiveBounds":bounds,"reverseOrientation":certificate.reverse_orientation(),
                        "cells":r.cells,"spans":r.spans,"domainCells":r.domain_cells,"reason":r.reason,
                        "crossFacePairs":resumed.pairs().len(),"pendingPairs":resumed.pending_pairs(),"wholeWallQualified":false});
                    std::fs::write(path, report.to_string()).unwrap();
                }
                return;
            }
            let wall = geometry
                .qualify_minimum_aligned_chords(1., 1e-4, 0.05, 1000, 0, 100000)
                .unwrap();
            assert!(
                wall.certificate.is_none(),
                "exhausted coverage cannot admit a minimum"
            );
            let wall = wall.report;
            assert_eq!(
                wall.pending.len(),
                wall.budget_pending + wall.enclosure_pending + wall.resolution_pending
            );
            eprintln!(
                "expression wall pending reasons budget={} enclosure={} resolution={}",
                wall.budget_pending, wall.enclosure_pending, wall.resolution_pending
            );
            assert!(!wall.complete && wall.cells == 0 && wall.spans == 0);
            assert_eq!(
                wall.total_pairs,
                expression.faces().len() * (expression.faces().len() + 1) / 2
            );
            assert_eq!(wall.pending.len(), wall.total_pairs);
            for face in 0..expression.faces().len() {
                assert!(wall.pending.iter().any(|p| p.faces == [face, face]));
            }
            let wall_cells = std::env::var("BREP_EXPRESSION_WALL_CELLS")
                .ok()
                .map(|v| {
                    v.parse::<usize>()
                        .expect("wall cell budget must be an integer")
                })
                .unwrap_or(wall.total_pairs);
            assert!(wall_cells >= wall.total_pairs && wall_cells <= 100000);
            let trim_cells = std::env::var("BREP_EXPRESSION_WALL_TRIM_CELLS")
                .ok()
                .map(|v| v.parse::<usize>().expect("trim budget must be an integer"))
                .unwrap_or(0);
            let wall_spans = std::env::var("BREP_EXPRESSION_WALL_SPANS")
                .ok()
                .map(|v| {
                    v.parse::<usize>()
                        .expect("wall span budget must be an integer")
                })
                .unwrap_or(1000000);
            assert!((100000..=1000000).contains(&wall_spans));
            let wall = geometry
                .qualify_trimmed_minimum_aligned_chords(
                    1., 1e-4, 0.05, 1000, wall_cells, wall_spans, trim_cells,
                )
                .unwrap();
            assert!(wall.report.spans <= wall_spans && wall.report.cells <= wall_cells);
            assert!(wall.report.domain_cells <= trim_cells);
            assert_eq!(wall.certificate.is_some(), wall.report.complete);
            if let Some(proof) = &wall.certificate {
                assert!(std::ptr::eq(proof.geometry(), &geometry));
                assert_eq!(proof.minimum_mm(), 1.);
                assert_eq!(proof.max_sine_squared(), 1e-4);
            }
            let wall = wall.report;
            assert_eq!(
                wall.spans,
                wall.intrinsic_spans
                    + wall.profile_transition_spans
                    + wall.span_stages.position
                    + wall.span_stages.normal
                    + wall.span_stages.shared_corner
                    + wall.span_stages.original_pair
                    + wall.span_stages.retained_pair
                    + wall.span_stages.corrected_pair,
                "all span work must be attributed exactly once"
            );
            eprintln!(
                "expression whole-wall initial={} total={} pending={} cells={} spans={} distanceExcluded={} angleExcluded={} planarExcluded={} intrinsicExcluded={} cacheHits={}",
                wall.initial_pairs_checked,
                wall.total_pairs,
                wall.pending.len(),
                wall.cells,
                wall.spans,
                wall.distance_excluded,
                wall.angle_excluded,
                wall.planar_excluded,
                wall.intrinsic_excluded,
                wall.cache_hits
            );
            assert_eq!(
                wall.pending.len(),
                wall.budget_pending + wall.enclosure_pending + wall.resolution_pending,
                "every unresolved product needs one pending reason"
            );
            for (reason, count) in [
                (
                    crate::source_profile_body::ExpressionWallPendingReason::Budget,
                    wall.budget_pending,
                ),
                (
                    crate::source_profile_body::ExpressionWallPendingReason::Enclosure,
                    wall.enclosure_pending,
                ),
                (
                    crate::source_profile_body::ExpressionWallPendingReason::Resolution,
                    wall.resolution_pending,
                ),
            ] {
                assert_eq!(
                    wall.pending
                        .iter()
                        .filter(|p| p.pending_reason == Some(reason))
                        .count(),
                    count
                );
            }
            eprintln!(
                "expression refined wall pending reasons budget={} enclosure={} resolution={}",
                wall.budget_pending, wall.enclosure_pending, wall.resolution_pending
            );
            if let Ok(path) = std::env::var("BREP_EXPRESSION_WALL_REPORT_PATH") {
                assert!(wall.pending.iter().all(|p| p.pending_reason.is_some()));
                let report = value_codec::Serialize::to_value(&wall);
                let request = value_codec::json!({
                    "kind":"expression-wall-native-request", "partialCarrier":partial_carrier,
                    "minimumMm":1., "maxSineSquared":1e-4, "toleranceUv":0.05,
                    "controls":1000, "cells":wall_cells, "spans":wall_spans,
                    "domainCells":trim_cells,
                    "scope":"endpoint-normal-aligned-chords"
                });
                // Keep the wire report unchanged while making every budget
                // explicit for future comparisons of native qualification runs.
                let output = std::path::Path::new(&path);
                if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent).expect("create wall report output directory");
                }
                std::fs::write(
                    format!("{path}.request.json"),
                    value_codec::to_string(&request).unwrap(),
                )
                .unwrap();
                std::fs::write(path, value_codec::to_string(&report).unwrap()).unwrap();
            }
            assert!(wall.profile_transition_checked <= wall.cells);
            assert!(wall.profile_transition_excluded <= wall.profile_transition_checked);
            assert!(wall.profile_transition_excluded <= wall.angle_excluded);
            assert!(wall.profile_transition_spans <= wall.spans);
            assert_eq!(
                wall.initial_pairs_checked, wall.total_pairs,
                "visit every original pair before refinement"
            );
            assert!(
                wall.planar_excluded > 0,
                "exact planar self/coplanar products must be excluded"
            );
            assert!(
                wall.cache_hits > 0,
                "reuse paid rectangles only within this owner-bound audit"
            );
            assert!(wall.plane_controls <= expression.faces().len() * 1000);

            assert_eq!(
                wall.complete,
                wall.pending.is_empty(),
                "coverage cannot hide pending rectangles"
            );
            if wall_cells == wall.total_pairs {
                assert!(
                    !wall.complete,
                    "the initial-pair budget cannot cover all refinements"
                );
            }

            let material_qualification = geometry
                .qualify_material_line(
                    [-30., 2., 3.],
                    [60., 0., 0.],
                    1e-5,
                    1000,
                    10000,
                    200000,
                    100000000,
                    100000,
                )
                .unwrap();
            let material = material_qualification.report;
            assert_eq!(
                material_qualification.certificate.is_some(),
                material.intervals.is_some()
            );
            if let Some(certificate) = material_qualification.certificate {
                assert!(std::ptr::eq(certificate.geometry(), &geometry));
                assert_eq!(certificate.origin(), [-30., 2., 3.]);
                assert_eq!(certificate.direction(), [60., 0., 0.]);
                assert_eq!(
                    certificate.intervals().len(),
                    material.intervals.as_ref().unwrap().len()
                );
                let lengths = certificate.lengths_mm().unwrap();
                assert_eq!(lengths.len(), certificate.intervals().len());
                assert!(
                    lengths
                        .iter()
                        .all(|d| d[0] > 0. && d[0] <= d[1] && d[1].is_finite())
                );
                let strict = certificate
                    .inspect_endpoint_normals(1e-4, 1000, 10000)
                    .unwrap();
                assert!(
                    !strict.complete,
                    "oblique chords must not qualify at strict normal tolerance"
                );
                assert_eq!(strict.normals.len(), certificate.intervals().len());
                let no_normals = certificate.inspect_endpoint_normals(1e-4, 1000, 0).unwrap();
                assert!(!no_normals.complete && no_normals.spans == 0);
                assert!(no_normals.normals.iter().flatten().all(|r| r.is_none()));
                let partial_normals = certificate.inspect_endpoint_normals(1e-4, 1000, 1).unwrap();
                assert!(!partial_normals.complete && partial_normals.spans <= 1);

                eprintln!(
                    "expression material lengths={:?} strictNormalsComplete={} normalSpans={} angles={:?}",
                    lengths,
                    strict.complete,
                    strict.spans,
                    strict
                        .normals
                        .iter()
                        .map(|pair| pair
                            .iter()
                            .map(|r| r.as_ref().and_then(|r| r.sine_squared_interval))
                            .collect::<Vec<_>>())
                        .collect::<Vec<_>>()
                );
            }
            eprintln!(
                "expression material line reason={} intervals={:?} roots={} pending={} spans={}",
                material.reason,
                material.intervals.as_ref().map(|v| v.len()),
                material.boundary.roots.len(),
                material.boundary.pending.len(),
                material.boundary.spans
            );
            assert!(
                material.boundary.spans <= 200000
                    && material.boundary.cells <= 10000
                    && material.boundary.domain_cells <= 100000
            );
            if let Ok(path) = std::env::var("BREP_EXPRESSION_MATERIAL_LINE_PATH") {
                let root_json = |face: usize, root: &nurbs_core::ray_surface::Root| {
                    value_codec::json!({
                    "face":face,"uv":root.uv,"parameter":root.parameter})
                };
                let roots = material
                    .boundary
                    .roots
                    .iter()
                    .map(|(face, root)| root_json(*face, root))
                    .collect::<Vec<_>>();
                let pending = material
                    .boundary
                    .pending
                    .iter()
                    .map(|(face, uv)| value_codec::json!({"face":face,"uv":uv}))
                    .collect::<Vec<_>>();
                let intervals = material.intervals.as_ref().map(|v| {
                    v.iter()
                        .map(|i| {
                            value_codec::json!({
                    "entry":root_json(i.entry.0,&i.entry.1),"exit":root_json(i.exit.0,&i.exit.1)})
                        })
                        .collect::<Vec<_>>()
                });
                std::fs::write(path,value_codec::json!({"origin":[-30.,2.,3.],"direction":[60.,0.,0.],
                    "reason":material.reason,"roots":roots,"pending":pending,"intervals":intervals,
                    "cells":material.boundary.cells,"spans":material.boundary.spans,
                    "coefficientWork":material.boundary.coefficient_work,"domainCells":material.boundary.domain_cells}).to_string()).unwrap();
            }
            let intervals = material
                .intervals
                .as_ref()
                .expect("complete owned outside-seeded material line");
            assert!(!intervals.is_empty());
            for interval in intervals {
                assert!(interval.entry.1.parameter[1] < interval.exit.1.parameter[0]);
            }
            let refused_line = geometry
                .inspect_material_line(
                    [-30., 2., 3.],
                    [60., 0., 0.],
                    1e-5,
                    1000,
                    0,
                    200000,
                    100000000,
                    100000,
                )
                .unwrap();
            assert!(refused_line.intervals.is_none());
            let radial = geometry
                .qualify_material_line(
                    [-30., -6., 3.],
                    [60., 12., 0.],
                    1e-5,
                    1000,
                    10000,
                    200000,
                    100000000,
                    100000,
                )
                .unwrap();
            let radial = radial
                .certificate
                .expect("outside-seeded radial material intervals");
            let radial_normals = radial.inspect_endpoint_normals(1e-4, 1000, 10000).unwrap();
            eprintln!(
                "expression radial lengths={:?} normalComplete={} angles={:?}",
                radial.lengths_mm().unwrap(),
                radial_normals.complete,
                radial_normals
                    .normals
                    .iter()
                    .map(|pair| pair
                        .iter()
                        .map(|r| r.as_ref().and_then(|r| r.sine_squared_interval))
                        .collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            );
            assert!(
                radial_normals.complete,
                "radial material intervals require both endpoint normals"
            );

            let volume_limits = |cells| crate::source_profile_body::ExpressionVolumeLimits {
                axis: 2,
                origin: 0.,
                absolute_error: 20.,
                tolerance_uv: 1e-8,
                controls_per_enclosure: 1000,
                cells,
                spans: 200000,
                domain_cells: 100000,
                domain_cells_per_rectangle: 256,
            };
            let refused_qualification = geometry.qualify_volume(volume_limits(0)).unwrap();
            assert!(refused_qualification.certificate.is_none());
            let refused = refused_qualification.report;
            assert!(!refused.complete && refused.signed_bounds.is_none() && refused.cells == 0);
            let qualification = geometry.qualify_volume(volume_limits(32768)).unwrap();
            let volume = &qualification.report;
            let sign_is_enclosed = volume
                .signed_bounds
                .is_some_and(|[lo, hi]| lo > 0. || hi < 0.);
            assert_eq!(
                qualification.certificate.is_some(),
                volume.complete && sign_is_enclosed,
                "a complete nonzero oriented volume must issue its owned certificate"
            );
            if let Some(certificate) = &qualification.certificate {
                assert!(std::ptr::eq(certificate.geometry(), &geometry));
                assert_eq!(certificate.signed_bounds(), volume.signed_bounds.unwrap());
                let bounds = certificate.positive_bounds();
                assert!(bounds[0] > 0. && bounds[0] <= bounds[1]);
                assert_eq!(
                    certificate.reverse_orientation(),
                    certificate.signed_bounds()[1] < 0.
                );
            }
            eprintln!(
                "expression volume bounds={:?} complete={} cells={} spans={} domains={} planarFaces={} reason={}",
                volume.signed_bounds,
                volume.complete,
                volume.cells,
                volume.spans,
                volume.domain_cells,
                volume.planar_faces,
                volume.reason
            );
            assert!(
                volume.cells <= 32768 && volume.spans <= 200000 && volume.domain_cells <= 100000
            );
            if let Some([lo, hi]) = volume.signed_bounds {
                assert!(lo.is_finite() && hi.is_finite() && lo <= hi);
            }
            if volume.complete {
                let [lo, hi] = volume.signed_bounds.unwrap();
                assert!(hi - lo <= 20.);
            }
            if let Ok(path) = std::env::var("BREP_EXPRESSION_VOLUME_PATH") {
                std::fs::write(path,value_codec::json!({"signedBounds":volume.signed_bounds,"complete":volume.complete,"cells":volume.cells,"spans":volume.spans,"domainCells":volume.domain_cells,"planarFaces":volume.planar_faces,"reason":volume.reason,"uncertainFace":volume.uncertain_face,"uncertainDomain":volume.uncertain_domain}).to_string()).unwrap();
            }
        }
        if let Ok(path) = std::env::var("BREP_CONTACT_TRANSITION_XY_PLANE_AUDIT_PATH") {
            std::fs::write(path, resumed.diagnostics().to_string()).unwrap();
        }

        for edge in outer.edges() {
            assert_ne!(edge.reversed()[0], edge.reversed()[1]);
            let restored = crate::source_shared_edge_restore::restore(
                edge.definition(),
                crate::source_shared_edge_restore::Limits {
                    mapping_cells_per_use: 100,
                    exact_work: 100000000,
                    driver_cells: 100000,
                },
            )
            .unwrap();
            let restored = restored.edge.expect(restored.reason);
            assert_eq!(restored.definition(), edge.definition());
        }
        for index in 0..2 {
            assert_eq!(
                patch.retained()[index]
                    .iter()
                    .map(|f| f.definition())
                    .collect::<Vec<_>>(),
                exterior[index]
                    .kept_loop()
                    .iter()
                    .map(|f| f.definition())
                    .collect::<Vec<_>>()
            );
        }
        for rail in sewn.rails() {
            assert_ne!(
                rail.transition_reversed() ^ sewn.reverse_face(),
                rail.edge().reversed()[rail.neighbor_side()]
            );
            let contour = rail.neighbor_contour();
            let [wire, index] = contour.middle();
            assert_eq!(
                contour.loops()[wire][index].definition(),
                rail.edge().uses()[rail.neighbor_side()].definition()
            );
        }
        let interior = admitted.interior.unwrap();
        assert!(interior.complete_regular);
        assert!(interior.pending.is_empty());
        let area: f64 = interior
            .regular
            .iter()
            .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
            .sum();
        assert_eq!(area, 1.);
        eprintln!(
            "owned transition face accepted={} cells={} spans={}",
            interior.regular.len(),
            interior.cells,
            interior.spans
        );
        for edge in 0..4 {
            assert!(
                boundary
                    .enclose_edge(edge, [0., 1.], 1)
                    .unwrap()
                    .0
                    .is_some()
            );
            for (t, corner) in [(0., edge), (1., (edge + 1) % 4)] {
                let bounds = boundary.enclose_edge(edge, [t; 2], 1).unwrap().0.unwrap();
                let root = boundary.corners()[corner].world_bounds();
                for k in 0..3 {
                    assert!(bounds[k][0] <= root[k][1] && root[k][0] <= bounds[k][1]);
                }
                let next = boundary
                    .enclose_edge((edge + 1) % 4, [0.; 2], 1)
                    .unwrap()
                    .0
                    .unwrap();
                if t == 1. {
                    assert_eq!(bounds, next);
                }
            }
        }
        for canonical in [[0.5, 0.5], [-0.1, 0.5], [0.5, 1.1], [f64::NAN, 1.]] {
            assert!(
                proposal
                    .qualify_edge_interval(0, canonical, 1e-6, 100, 100)
                    .is_err()
            );
        }
        assert!(
            proposal
                .qualify_edge_interval(usize::MAX, [0., 1.], 1e-6, 100, 100)
                .is_err()
        );
    }
    #[test]
    fn corrected_seam_endpoint_preflight_preserves_work_refusals() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let geometry = crate::source_shell_geometry::qualify(shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
        let body = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 20.,
                tolerance_uv: 1e-8,
                cells: 100000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap()
        .body
        .unwrap();
        let specs = [
            crate::source_profile_body::FaceSpec {
                face: 0,
                anchor: [5, 0],
            },
            crate::source_profile_body::FaceSpec {
                face: 5,
                anchor: [0, 0],
            },
        ];
        let report = crate::source_profile_body::prepare(
            &body,
            &specs,
            crate::source_profile_body::Limits {
                controls: 1000,
                fragments: 100,
                position_tolerance: 1e-7,
            },
        )
        .unwrap();
        let proposal = report.proposal.unwrap();
        let (pole, spent) = proposal
            .profile(0)
            .unwrap()
            .pole_limits(true, 1e-6, 4)
            .unwrap();
        let pole = pole.expect("exact corrected pole limit diagnostic");
        assert_eq!(spent, 4);
        assert_eq!(pole.point, [20., 0., 6.]);
        assert!(pole.distinct_tangent_limits);
        eprintln!(
            "corrected profile pole distinct={} sine={:?} normals={:?}",
            pole.distinct_tangent_limits, pole.sine_squared_interval, pole.components
        );
        let cut = proposal.profile(0).unwrap().section(0, 0.75).unwrap();
        let transition = proposal
            .profile(0)
            .unwrap()
            .section(0, 0.75)
            .unwrap()
            .end_transition([20., 0., 6.], 0.25)
            .unwrap();
        for i in 0..64 {
            let free = [i as f64 / 64., (i + 1) as f64 / 64.];
            assert_eq!(transition.regular_join(free, 1).unwrap().0, Some(true));
            assert_eq!(
                transition.enclose(free, [0., 0.], 1).unwrap(),
                cut.enclose(free, 1).unwrap()
            );
            assert!(transition.enclose(free, [0., 1.], 2).unwrap().0.is_some());
        }
        for high in [false, true] {
            let rail = transition.rail(high);
            assert_eq!(
                rail.enclose([0., 0.], 1).unwrap(),
                cut.enclose([rail.free_parameter(); 2], 1).unwrap()
            );
            assert_eq!(
                rail.enclose([1., 1.], 0).unwrap().0.unwrap(),
                [[20., 20.], [0., 0.], [6., 6.]]
            );
            assert_eq!(
                rail.tangent_bounds([1., 1.], 0).unwrap().0.unwrap(),
                [[0., 0.]; 3]
            );
            for i in 0..64 {
                let range = [i as f64 / 64., (i + 1) as f64 / 64.];
                assert!(rail.enclose(range, 2).unwrap().0.is_some());
                assert!(rail.tangent_bounds(range, 2).unwrap().0.is_some());
            }
        }
        let mut regular_cells = 0;
        let mut pending_cells = Vec::new();
        for free_index in 0..8 {
            for across_index in 0..8 {
                let free = [free_index as f64 / 8., (free_index + 1) as f64 / 8.];
                let across = [across_index as f64 / 8., (across_index + 1) as f64 / 8.];
                let (bounds, spent) = transition.normal_bounds(free, across, 4).unwrap();
                assert_eq!(spent, 4);
                let bounds = bounds.unwrap();
                if bounds.iter().any(|r| r[0] > 0. || r[1] < 0.) {
                    regular_cells += 1;
                } else {
                    pending_cells.push([free, across]);
                }
            }
        }
        assert_eq!(regular_cells + pending_cells.len(), 64);
        eprintln!(
            "transition Jacobian regular={} pending={} domains={:?}",
            regular_cells,
            pending_cells.len(),
            pending_cells
        );
        assert!(
            transition
                .normal_bounds([0.2, 0.3], [0.2, 0.3], 3)
                .unwrap()
                .0
                .is_none()
        );
        let refined = transition
            .inspect_regularity([0., 1.], [0., 1.], 1000, 4000)
            .unwrap();
        let area: f64 = refined
            .regular
            .iter()
            .chain(&refined.pending)
            .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
            .sum();
        assert_eq!(area, 1.);
        assert!(!refined.complete_regular);
        assert!(refined.pending.iter().any(|r| r[1][1] == 1.));
        eprintln!(
            "adaptive transition regular={} pending={} cells={} spans={}",
            refined.regular.len(),
            refined.pending.len(),
            refined.cells,
            refined.spans
        );
        assert!(refined.regular.iter().all(|r| r[1][1] < 1.));
        let pending_area: f64 = refined
            .pending
            .iter()
            .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
            .sum();
        eprintln!("adaptive transition pendingArea={pending_area:.17}");
        let (tip, tip_spans) = transition.tip_limits(1e-6, 8).unwrap();
        let tip = tip.unwrap();
        assert_eq!(tip_spans, 8);
        assert_eq!(tip.point, [20., 0., 6.]);
        assert!(tip.distinct_tangent_limits);
        assert!(tip.sine_squared_interval.unwrap()[0] > 0.999999);
        eprintln!(
            "transition tip distinct={} sine={:?} normals={:?}",
            tip.distinct_tangent_limits, tip.sine_squared_interval, tip.components
        );
        assert!(transition.tip_limits(1e-6, 7).unwrap().0.is_none());
        let bridge = nurbs_core::normal_alignment::ProfileSectionBridge::new(
            proposal.profile(0).unwrap().section(0, 0.75).unwrap(),
            proposal.profile(0).unwrap().section(0, 0.9).unwrap(),
            [0.15, 0.15],
        )
        .unwrap();
        for high in [false, true] {
            for i in 0..64 {
                let free = [i as f64 / 64., (i + 1) as f64 / 64.];
                assert_eq!(bridge.regular_join(high, free, 1).unwrap().0, Some(true));
                assert_eq!(
                    bridge
                        .enclose(free, [if high { 1. } else { 0. }; 2], 1)
                        .unwrap(),
                    bridge.section(high).enclose(free, 1).unwrap()
                );
            }
        }
        for high in [false, true] {
            let rail = bridge.rail(high);
            let free = rail.free_parameter();
            for across in [0., 0.5, 1.] {
                assert_eq!(
                    rail.enclose([across; 2], 4).unwrap(),
                    bridge.enclose([free; 2], [across; 2], 4).unwrap()
                );
                assert!(rail.tangent_bounds([across; 2], 8).unwrap().0.is_some());
            }
        }
        let boundary = bridge.rail(true).enclose([0.5; 2], 4).unwrap().0.unwrap();
        use nurbs_core::interval_eval::Interval as RailInterval;
        let x = RailInterval::new(boundary[0][0], boundary[0][1]).unwrap();
        let y = RailInterval::new(boundary[1][0], boundary[1][1]).unwrap();
        let residual = x
            .mul(x)
            .unwrap()
            .add(y.mul(y).unwrap())
            .unwrap()
            .sub(RailInterval::point(400.))
            .unwrap();
        assert!(
            residual.hi < 0.,
            "Hermite rail must not be admitted as the unchanged cylinder boundary"
        );
        eprintln!(
            "bridge cylinder rail midpoint squared-radius residual=[{},{}]",
            residual.lo, residual.hi
        );
        assert!(
            bridge.constrain_original_rails(1000).unwrap().0.is_none(),
            "rounded 0.15 scale is not the exact difference of section coordinates"
        );
        let exact_width = 0.9 - 0.75;
        let exact_bridge = nurbs_core::normal_alignment::ProfileSectionBridge::new(
            proposal.profile(0).unwrap().section(0, 0.75).unwrap(),
            proposal.profile(0).unwrap().section(0, 0.9).unwrap(),
            [exact_width; 2],
        )
        .unwrap();
        let constrained = exact_bridge
            .constrain_original_rails(1000)
            .unwrap()
            .0
            .unwrap();
        let corrected = constrained
            .enclose([1., 1.], [0.5; 2], 1)
            .unwrap()
            .0
            .unwrap();
        let x = RailInterval::new(corrected[0][0], corrected[0][1]).unwrap();
        let y = RailInterval::new(corrected[1][0], corrected[1][1]).unwrap();
        let residual = x
            .mul(x)
            .unwrap()
            .add(y.mul(y).unwrap())
            .unwrap()
            .sub(RailInterval::point(400.))
            .unwrap();
        assert!(residual.lo.abs() < 1e-8 && residual.hi.abs() < 1e-8);
        eprintln!(
            "constrained cylinder rail residual=[{},{}]",
            residual.lo, residual.hi
        );
        assert!(
            constrained
                .enclose([0.4, 0.6], [0.4, 0.6], 14)
                .unwrap()
                .0
                .is_some()
        );
        assert!(
            constrained
                .enclose([0.4, 0.6], [0.4, 0.6], 13)
                .unwrap()
                .0
                .is_none()
        );
        assert!(
            constrained
                .derivative_bounds([0.5; 2], [0.5; 2], 36)
                .unwrap()
                .0
                .is_some()
        );
        assert!(
            constrained
                .derivative_bounds([0.5; 2], [0.5; 2], 35)
                .unwrap()
                .0
                .is_none()
        );
        for high in [false, true] {
            let report = constrained
                .inspect_support_rail_normals(high, [0.5; 2], 1e-6, 37)
                .unwrap();
            assert_eq!(report.aligned, Some(true));
            if high {
                assert!(
                    report.sine_squared_interval.unwrap()[0] > 1e-10,
                    "position-only correction still needs a tangent-field correction"
                );
            }
            assert_eq!(
                constrained
                    .inspect_support_rail_normals(high, [0.5; 2], 1e-6, 36)
                    .unwrap()
                    .aligned,
                None
            );
            eprintln!(
                "constrained rail high={} aligned={:?} sine={:?} spans={}",
                high, report.aligned, report.sine_squared_interval, report.spans
            );
        }
        for sample in [0.25, 0.5, 0.75] {
            let (normal, spans) = constrained
                .normals_with_transverse_correction([sample; 2], [sample; 2], 61)
                .unwrap();
            let normal = normal.unwrap();
            assert!(normal.iter().any(|r| r[0] > 0. || r[1] < 0.));
            eprintln!("full Hermite point={sample} normal={normal:?} spans={spans}");
        }
        for high in [false, true] {
            let join = constrained
                .inspect_transverse_corrected_side_join(high, [0., 1.], 1000, 1000)
                .unwrap();
            assert!(join.complete_regular);
            eprintln!(
                "full Hermite side={high} cells={} spans={}",
                join.cells, join.spans
            );
        }
        let full_cover = constrained
            .inspect_transverse_corrected_regularity([0., 1.], [0., 1.], 1000, 72000)
            .unwrap();
        assert!(
            full_cover.complete_regular,
            "full corrected bridge regularity must cover the entire rectangle"
        );
        let full_area: f64 = full_cover
            .regular
            .iter()
            .chain(&full_cover.pending)
            .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
            .sum();
        assert_eq!(full_area, 1.);
        eprintln!(
            "full Hermite regular={} pending={} cells={} spans={}",
            full_cover.regular.len(),
            full_cover.pending.len(),
            full_cover.cells,
            full_cover.spans
        );
        let mut bridge_regular = 0;
        for i in 0..8 {
            for j in 0..8 {
                let n = bridge
                    .normal_bounds(
                        [i as f64 / 8., (i + 1) as f64 / 8.],
                        [j as f64 / 8., (j + 1) as f64 / 8.],
                        8,
                    )
                    .unwrap()
                    .0
                    .unwrap();
                if n.iter().any(|r| r[0] > 0. || r[1] < 0.) {
                    bridge_regular += 1;
                }
            }
        }
        eprintln!(
            "noncollapsed bridge coarse regular={bridge_regular} pending={}",
            64 - bridge_regular
        );
        let cover = bridge
            .inspect_regularity([0., 1.], [0., 1.], 1000, 8000)
            .unwrap();
        assert!(
            cover.complete_regular,
            "{} pending={}",
            cover.reason,
            cover.pending.len()
        );
        let area: f64 = cover
            .regular
            .iter()
            .chain(&cover.pending)
            .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
            .sum();
        assert_eq!(area, 1.);
        eprintln!(
            "noncollapsed bridge adaptive regular={} pending={} cells={} spans={}",
            cover.regular.len(),
            cover.pending.len(),
            cover.cells,
            cover.spans
        );
        let jet = bridge
            .derivative_bounds([0.5, 0.5], [0.5, 0.5], 8)
            .unwrap()
            .0
            .unwrap();
        let vectors = jet.map(|r| r.map(|b| b[0] * 0.5 + b[1] * 0.5));
        let dot = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        let aa = dot(vectors[0], vectors[0]);
        let bb = dot(vectors[1], vectors[1]);
        let ab = dot(vectors[0], vectors[1]);
        let determinant = aa * bb - ab * ab;
        assert!(determinant > 0.);
        let projection = [
            std::array::from_fn(|k| (bb * vectors[0][k] - ab * vectors[1][k]) / determinant),
            std::array::from_fn(|k| (aa * vectors[1][k] - ab * vectors[0][k]) / determinant),
        ];
        let injectivity = bridge
            .inspect_injectivity([0., 1.], [0., 1.], projection, 4000, 32000)
            .unwrap();
        assert!(
            injectivity.complete_regular,
            "{} pending={}",
            injectivity.reason,
            injectivity.pending.len()
        );
        let area: f64 = injectivity
            .regular
            .iter()
            .chain(&injectivity.pending)
            .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
            .sum();
        assert_eq!(area, 1.);
        eprintln!(
            "bridge injectivity qualified={} accepted={} pending={} cells={} spans={} projection={:?}",
            injectivity.complete_regular,
            injectivity.regular.len(),
            injectivity.pending.len(),
            injectivity.cells,
            injectivity.spans,
            projection
        );

        let full_injective = constrained
            .inspect_transverse_corrected_injectivity([0., 1.], [0., 1.], projection, 1000, 61000)
            .unwrap();
        let full_injective_area: f64 = full_injective
            .regular
            .iter()
            .chain(&full_injective.pending)
            .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
            .sum();
        assert_eq!(full_injective_area, 1.);
        assert!(
            full_injective.complete_regular,
            "full corrected transition injectivity must cover the entire rectangle"
        );
        eprintln!(
            "full Hermite injectivity={} accepted={} pending={} cells={} spans={}",
            full_injective.complete_regular,
            full_injective.regular.len(),
            full_injective.pending.len(),
            full_injective.cells,
            full_injective.spans
        );
        let middle = cut.enclose([0.5, 0.5], 1).unwrap().0.unwrap();
        assert!(
            middle
                .iter()
                .zip(pole.point)
                .any(|(r, p)| r[1] < p - 0.01 || r[0] > p + 0.01)
        );
        for i in 0..64 {
            let r = cut
                .support_normals([i as f64 / 64., (i + 1) as f64 / 64.], 1)
                .unwrap()
                .0
                .unwrap();
            for bounds in [
                cut.tangent_bounds([i as f64 / 64., (i + 1) as f64 / 64.], 1)
                    .unwrap()
                    .0
                    .unwrap(),
                cut.transverse_bounds([i as f64 / 64., (i + 1) as f64 / 64.], 1)
                    .unwrap()
                    .0
                    .unwrap(),
            ] {
                assert!(bounds.iter().any(|r| r[0] > 0. || r[1] < 0.));
            }
            assert!(
                r.iter().any(|r| r[0] > 0. || r[1] < 0.),
                "cut support normal unresolved at {i}: {r:?}"
            );
        }
        eprintln!(
            "corrected pole cut U=0.75 midpoint={:?} regularIntervals=64",
            middle
        );
        let low = proposal
            .cut_endpoint(0, 0, 0.75, false, 0, 16)
            .unwrap()
            .endpoint
            .unwrap();
        let high = proposal
            .cut_endpoint(0, 0, 0.75, true, 1, 16)
            .unwrap()
            .endpoint
            .unwrap();
        let low_partition = low.split_source_use(100).unwrap().partition.unwrap();
        let high_partition = high.split_source_use(100).unwrap().partition.unwrap();
        let contours =
            crate::source_profile_body::propose_cut_contours(&low_partition, &high_partition, 100)
                .unwrap();
        let contour_cost = contours.fragments;
        let contours = contours.proposal.expect(contours.reason);
        for wire in contours.loops() {
            for i in 0..wire.len() {
                assert!(wire[i].joins(&wire[(i + 1) % wire.len()]));
            }
            let restored = wire
                .iter()
                .map(|f| crate::source_boundary_fragment::restore(f.definition(), 100).unwrap())
                .collect::<Vec<_>>();
            for i in 0..wire.len() {
                assert!(restored[i].joins(&restored[(i + 1) % wire.len()]));
            }
        }
        assert!(
            crate::source_profile_body::propose_cut_contours(
                &low_partition,
                &high_partition,
                contour_cost - 1
            )
            .unwrap()
            .proposal
            .is_none()
        );
        assert!(
            crate::source_profile_body::propose_cut_contours(&low_partition, &low_partition, 100)
                .unwrap()
                .proposal
                .is_none()
        );
        let different = proposal
            .cut_endpoint(0, 0, 0.7, true, 1, 16)
            .unwrap()
            .endpoint
            .unwrap();
        let different_partition = different.split_source_use(100).unwrap().partition.unwrap();
        assert!(
            crate::source_profile_body::propose_cut_contours(
                &low_partition,
                &different_partition,
                100
            )
            .unwrap()
            .proposal
            .is_none()
        );
        for (high, index, neighbor) in [(false, 0, 1), (true, 1, 2)] {
            let report = proposal.cut_endpoint(0, 0, 0.75, high, index, 16).unwrap();
            let endpoint = report.endpoint.expect(report.reason);
            assert!(std::ptr::eq(endpoint.proposal(), &proposal));
            assert_eq!(endpoint.faces(), [0, neighbor]);
            assert_eq!(endpoint.edge(), index);
            assert_eq!(endpoint.definition()["parameter"], value_codec::json!(0.75));
            let own = cut
                .enclose([if high { 1. } else { 0. }; 2], 1)
                .unwrap()
                .0
                .unwrap();
            let world = endpoint.world_bounds();
            for k in 0..3 {
                assert!(own[k][0] <= world[k][1] && world[k][0] <= own[k][1]);
            }
            eprintln!(
                "owned cut endpoint edge={} canonical={:?} neighborUV={:?} world={:?} spans={}",
                index,
                endpoint.canonical_bounds(),
                endpoint.neighbor_uv_bounds(),
                world,
                report.spans
            );
            let split = endpoint.split_source_use(16).unwrap();
            let partition = split.partition.expect(split.reason);
            assert!(std::ptr::eq(partition.endpoint(), &endpoint));
            let fragments = partition.selected_fragments();
            assert!(fragments[0].joins(&fragments[1]));
            assert_eq!(fragments[0].curve(), fragments[1].curve());
            let restored = fragments
                .each_ref()
                .map(|f| crate::source_boundary_fragment::restore(f.definition(), 16).unwrap());
            assert!(restored[0].joins(&restored[1]));
            for i in 0..2 {
                assert_eq!(restored[i].definition(), fragments[i].definition());
            }

            let neighbor = partition.neighbor_ranges();
            assert_eq!(neighbor[0][1], neighbor[1][0]);
            let point = partition.point();
            assert_eq!(point.surface(), fragments[0].surface());
            assert!(matches!(
                &fragments[0].endpoints()[1],
                crate::source_boundary_fragment::Endpoint::Crossing { .. }
            ));
            assert!(matches!(
                &fragments[1].endpoints()[0],
                crate::source_boundary_fragment::Endpoint::Crossing { .. }
            ));
            assert!(endpoint.split_source_use(0).is_err());
            let original = &body.geometry().shell().edges()[index];
            let addresses = &body.geometry().shell().uses()[index];
            let side = usize::from(addresses[0].face != 0);
            let transported =
                crate::source_carrier_point::qualify(original, point, side, 3).unwrap();
            let carrier = transported.point.expect(transported.reason);
            assert_eq!(transported.spans, 3);
            assert_eq!(carrier.root().definition(), point.definition());
            for use_side in 0..2 {
                let parts = carrier.retained_parts(use_side).unwrap();
                let fragments = original.uses()[use_side]
                    .split_at_carrier(&carrier, use_side)
                    .unwrap();
                assert!(fragments[0].joins(&fragments[1]));
                for fragment in &fragments {
                    let restored =
                        crate::source_boundary_fragment::restore(fragment.definition(), 100)
                            .unwrap();
                    assert_eq!(restored.definition(), fragment.definition());
                    assert_eq!(restored.parameter_bounds(), fragment.parameter_bounds());
                    assert!(
                        crate::source_boundary_fragment::restore(fragment.definition(), 0).is_err()
                    );
                    let mut foreign = fragment.definition();
                    for end in foreign["endpoints"].as_array_mut().unwrap() {
                        if end["kind"].as_str() == Some("carrier") {
                            end["side"] = value_codec::json!(1 - use_side);
                        }
                    }
                    assert!(crate::source_boundary_fragment::restore(foreign, 100).is_err());
                }
                let mut nested = carrier.definition();
                nested["edge"]["uses"][0]["endpoints"][0]["kind"] = value_codec::json!("carrier");
                assert!(
                    crate::source_carrier_point::restore(
                        nested,
                        crate::source_carrier_point::RestoreLimits {
                            edge: crate::source_shared_edge_restore::Limits {
                                mapping_cells_per_use: 100,
                                exact_work: 1000000,
                                driver_cells: 100
                            },
                            root_mapping_cells: 100,
                            spans: 100
                        }
                    )
                    .is_err()
                );
                assert!(parts[0].joins(&parts[1]));
                assert!(!parts[1].joins(&parts[0]));
                assert_eq!(
                    parts[0].parameter_bounds()[1],
                    parts[1].parameter_bounds()[0]
                );
                assert_eq!(
                    parts[0].source().definition(),
                    original.uses()[use_side].definition()
                );
                assert_eq!(
                    parts[1].parameter_bounds()[1],
                    original.uses()[use_side].parameter_bounds()[1]
                );
                assert!(!parts[0].enclose(0).unwrap().complete);
                assert!(parts[0].enclose(100).unwrap().complete);
                assert!(parts[1].enclose(100).unwrap().complete);
                let foreign_parts = carrier.retained_parts(1 - use_side).unwrap();
                assert!(!parts[0].joins(&foreign_parts[1]));
            }
            let transported_partition = partition
                .qualify_transported_neighbor(3, 100000000, 100000)
                .unwrap();
            assert_eq!(transported_partition.carrier_spans, 3);
            assert!(
                transported_partition.partition.is_some(),
                "{}",
                transported_partition.reason
            );
            assert!(
                partition
                    .qualify_transported_neighbor(2, 100000000, 100000)
                    .unwrap()
                    .partition
                    .is_none()
            );
            assert!(
                partition
                    .qualify_transported_neighbor(3, 1, 100)
                    .unwrap()
                    .partition
                    .is_none()
            );
            let neighbor_fragments = original.uses()[1 - side]
                .split_at_carrier(&carrier, 1 - side)
                .unwrap();
            let selected_fragments = partition.selected_fragments();
            let carrier_selected = original.uses()[side]
                .split_at_carrier(&carrier, side)
                .unwrap();
            let maps = crate::source_seam_tangency::ranges(original);
            for half in 0..2 {
                let pair = [&selected_fragments[half], &neighbor_fragments[1 - half]];
                let ranges = [maps[side], maps[1 - side]];
                let report = crate::source_mapped_edge::qualify(
                    original.world(),
                    pair,
                    ranges,
                    [None, None],
                    100000000,
                    100000,
                )
                .unwrap();
                let admitted = report.edge.expect(report.reason);
                assert!(!admitted.covers_complete_canonical_source());
                let restored = crate::source_shared_edge_restore::restore(
                    admitted.definition(),
                    crate::source_shared_edge_restore::Limits {
                        mapping_cells_per_use: 100,
                        exact_work: 100000000,
                        driver_cells: 100000,
                    },
                )
                .unwrap();
                assert_eq!(
                    restored.edge.expect(restored.reason).definition(),
                    admitted.definition()
                );
                let endpoint_index = if half == 0 { 1 } else { 0 };
                let ends = [
                    &pair[0].endpoints()[endpoint_index],
                    &pair[1].endpoints()[1 - endpoint_index],
                ];
                assert!(crate::source_carrier_point::same_mapped_endpoint(
                    original.world(),
                    pair,
                    ranges,
                    ends
                ));
                let both_pair = [&carrier_selected[half], &neighbor_fragments[1 - half]];
                let both_ends = [
                    &both_pair[0].endpoints()[endpoint_index],
                    &both_pair[1].endpoints()[1 - endpoint_index],
                ];
                assert!(crate::source_carrier_point::same_mapped_endpoint(
                    original.world(),
                    both_pair,
                    ranges,
                    both_ends
                ));
                assert!(!crate::source_carrier_point::same_mapped_endpoint(
                    original.world(),
                    both_pair,
                    ranges,
                    [both_ends[0], both_ends[0]]
                ));
                assert!(
                    crate::source_mapped_edge::qualify(
                        original.world(),
                        both_pair,
                        ranges,
                        [None, None],
                        100000000,
                        100000
                    )
                    .unwrap()
                    .edge
                    .is_some()
                );
                let mut changed = original.world().clone();
                changed.control_points[0][0] += 0.01;
                assert!(!crate::source_carrier_point::same_mapped_endpoint(
                    &changed, pair, ranges, ends
                ));
                let mut changed_ranges = ranges;
                changed_ranges[0][0][0] += 0.01;
                assert!(!crate::source_carrier_point::same_mapped_endpoint(
                    original.world(),
                    pair,
                    changed_ranges,
                    ends
                ));
                assert!(
                    crate::source_mapped_edge::qualify(
                        original.world(),
                        pair,
                        ranges,
                        [None, None],
                        1,
                        100
                    )
                    .unwrap()
                    .edge
                    .is_none()
                );
            }
            assert!(carrier.retained_parts(2).is_err());
            let b = carrier.parameter_bounds()[1 - side];
            assert!(b[0] < if high { 0.75 } else { 0.25 } && b[1] > if high { 0.75 } else { 0.25 });
            assert!(
                crate::source_carrier_point::qualify(original, point, side, 2)
                    .unwrap()
                    .point
                    .is_none()
            );
            assert!(
                crate::source_carrier_point::qualify(original, point, 1 - side, 3)
                    .unwrap()
                    .point
                    .is_none()
            );
            let restore_limits = || crate::source_carrier_point::RestoreLimits {
                edge: crate::source_shared_edge_restore::Limits {
                    mapping_cells_per_use: 16,
                    exact_work: 100000000,
                    driver_cells: 10000,
                },
                root_mapping_cells: 16,
                spans: 3,
            };
            let mut definition = carrier.definition();
            definition["canonical"] = value_codec::json!([0., 0.]);
            let restored =
                crate::source_carrier_point::restore(definition, restore_limits()).unwrap();
            let restored = restored.point.expect(restored.reason);
            assert_eq!(restored.definition(), carrier.definition());
            let native=carrier.requalify(restore_limits()).unwrap().point.unwrap();
            assert!(native.same_definition(&restored));
            assert_eq!(native.canonical_bounds(),restored.canonical_bounds());
            assert_eq!(native.parameter_bounds(),restored.parameter_bounds());
            assert_eq!(native.world_bounds(),restored.world_bounds());
            let mut foreign = carrier.definition();
            foreign["sourceSide"] = value_codec::json!(1 - side);
            assert!(
                crate::source_carrier_point::restore(foreign, restore_limits())
                    .unwrap()
                    .point
                    .is_none()
            );
            eprintln!(
                "carrier root edge={} side={} target={:?} canonical={:?} spans=3",
                index,
                side,
                b,
                carrier.canonical_bounds()
            );

            if high {
                let cutter =
                    nurbs_core::curve::Curve::from_polyline(vec![vec![0.75, -1.], vec![0.75, 2.]])
                        .unwrap();
                let joined = partition
                    .qualify_neighbor(&cutter, [[0.7, 0.8], [0.61, 0.72]], 16, 100000000, 10000)
                    .unwrap();
                eprintln!(
                    "neighbor cut edge={} reason={} work={} roots={}",
                    index, joined.reason, joined.exact_work, joined.root_checks
                );
                let halves = joined.partition.expect(joined.reason);
                assert!(halves.fragments()[0].joins(&halves.fragments()[1]));
                assert!(
                    halves
                        .edges()
                        .iter()
                        .all(|e| !e.covers_complete_canonical_source())
                );
                for half in halves.edges() {
                    let restored = crate::source_shared_edge_restore::restore(
                        half.definition(),
                        crate::source_shared_edge_restore::Limits {
                            mapping_cells_per_use: 16,
                            exact_work: 100000000,
                            driver_cells: 10000,
                        },
                    )
                    .unwrap();
                    let restored = restored.edge.expect(restored.reason);
                    assert_eq!(restored.definition(), half.definition());
                    assert!(!restored.covers_complete_canonical_source());
                }

                let wrong =
                    nurbs_core::curve::Curve::from_polyline(vec![vec![0.8, -1.], vec![0.8, 2.]])
                        .unwrap();
                assert!(
                    partition
                        .qualify_neighbor(&wrong, [[0.7, 0.8], [0.61, 0.72]], 16, 100000000, 10000)
                        .unwrap()
                        .partition
                        .is_none()
                );
                assert!(
                    partition
                        .qualify_neighbor(&cutter, [[0.7, 0.8], [0.61, 0.72]], 16, 1, 1)
                        .unwrap()
                        .partition
                        .is_none()
                );
            }

            eprintln!(
                "owned cut partition edge={} roots={:?} neighborRanges={:?} cells={}",
                index,
                point.root_parameters(),
                neighbor,
                split.mapping_cells
            );
            let short = proposal.cut_endpoint(0, 0, 0.75, high, index, 1).unwrap();
            assert!(short.endpoint.is_none());
            assert_eq!(short.reason, "profile-cut-work-limit");
        }
        assert!(
            proposal
                .cut_endpoint(0, 0, 0.75, false, 5, 16)
                .unwrap()
                .endpoint
                .is_none()
        );
        assert!(proposal.cut_endpoint(0, 2, 0.75, false, 0, 16).is_err());

        let report = proposal.qualify_edge(1, 100, 1e-6, 2, 4).unwrap();
        assert!(report.seam.is_none());
        assert_eq!(report.reason, "profile-seam-endpoint-normal-unresolved");
        let r = report.angular.unwrap();
        assert_eq!((r.cells, r.spans, r.accepted_cells), (2, 4, 1));
        assert_eq!(r.pending, vec![[0., 1.], [1., 1.]]);
        let report = proposal.qualify_edge(1, 100, 1e-6, 1, 1).unwrap();
        assert!(report.seam.is_none());
        assert_eq!(report.reason, "profile-path-work-limit");
        assert!(!report.angular.unwrap().pending.is_empty());
    }
    #[test]
    fn actual_original_neighbor_angles_ignore_proposed_neighbor_profile() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let geometry = crate::source_shell_geometry::qualify(shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
        let body = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 20.,
                tolerance_uv: 1e-8,
                cells: 100000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap()
        .body
        .unwrap();
        let specs = [
            crate::source_profile_body::FaceSpec {
                face: 0,
                anchor: [5, 0],
            },
            crate::source_profile_body::FaceSpec {
                face: 5,
                anchor: [0, 0],
            },
        ];
        let work = || crate::source_profile_body::Limits {
            controls: 1000,
            fragments: 100,
            position_tolerance: 1e-7,
        };
        let single = crate::source_profile_body::prepare(&body, &specs[..1], work())
            .unwrap()
            .proposal
            .unwrap();
        let both = crate::source_profile_body::prepare(&body, &specs, work())
            .unwrap()
            .proposal
            .unwrap();
        assert!(single.profile(5).is_none() && both.profile(5).is_some());
        let baseline = single
            .inspect_original_neighbor_interval(0, 5, 5, 1e-6, 50000, 100000, [0., 1.])
            .unwrap();
        let actual = both
            .inspect_original_neighbor_interval(0, 5, 5, 1e-6, 50000, 100000, [0., 1.])
            .unwrap();
        assert_eq!(baseline.aligned, Some(true));
        assert_eq!(actual.aligned, baseline.aligned);
        assert_eq!(actual.upper_sine_squared, baseline.upper_sine_squared);
        assert_eq!(actual.cells, baseline.cells);
        assert_eq!(actual.spans, baseline.spans);
        assert!(actual.pending.is_empty() && baseline.pending.is_empty());
        assert!(
            both.inspect_original_neighbor_interval(0, 0, 5, 1e-6, 50000, 100000, [0., 1.])
                .is_err()
        );
        assert!(
            both.inspect_original_neighbor_interval(0, 1, 5, 1e-6, 50000, 100000, [0., 1.])
                .is_err()
        );
    }
    #[test]
    fn admitted_annular_body_checks_original_radius_patches() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let geometry = crate::source_shell_geometry::qualify(shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
        let body = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 20.,
                tolerance_uv: 1e-8,
                cells: 100000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap()
        .body
        .unwrap();
        if let Some(path) = std::env::var_os("CAD_ANNULAR_SOURCE_BODY_OUTPUT") {
            std::fs::write(
                path,
                value_codec::to_string_pretty(&body.definition().unwrap()).unwrap(),
            )
            .unwrap();
        }
        let combined = crate::source_wall_pair::qualify(
            &body,
            [4, 5],
            5.99,
            crate::source_wall_pair::Limits {
                normal_spans: 10000,
                max_sine_squared: 1e-6,
                cells: 4096,
                spans: 10000,
                domain_cells: 10000,
                domain_cells_per_query: 256,
                tolerance_uv: 1e-8,
            },
        )
        .unwrap();
        assert!(combined.normal_products > 0);
        assert!(combined.spans > 0);
        assert!(combined.unresolved_products.is_empty());
        let mixed = combined.certificate.unwrap();
        assert!(std::ptr::eq(mixed.body(), &body));
        let aggregate = crate::source_wall_coverage::qualify_with_mixed_pair_facts(
            &body,
            5.99,
            crate::source_wall_coverage::Limits {
                pairs: 378,
                plane_controls: 10000,
                normal_spans: 1000,
                gap: crate::source_face_gap::Limits {
                    cells: 1000,
                    spans: 2000,
                },
                max_sine_squared: 1e-6,
            },
            Vec::new(),
            crate::source_wall_coverage::NormalProductLimits {
                cells: 0,
                cells_per_pair: 0,
            },
            crate::source_wall_coverage::FactoredNormalLimits {
                exact_work: 0,
                queries: 0,
                cells: 0,
                cells_per_pair: 0,
            },
            Vec::new(),
            None,
            vec![mixed],
        )
        .unwrap();
        let pair = aggregate.pairs.iter().find(|p| p.faces == [4, 5]).unwrap();
        assert!(pair.proven);
        assert_eq!(pair.reason, "source-wall-mixed-product-qualified");
        assert!(aggregate.certificate.is_none());

        for faces in [[0, 7], [2, 5], [5, 12], [7, 10]] {
            let end = crate::source_wall_pair::qualify(
                &body,
                faces,
                5.99,
                crate::source_wall_pair::Limits {
                    normal_spans: 10000,
                    max_sine_squared: 1e-6,
                    cells: 4096,
                    spans: 10000,
                    domain_cells: 10000,
                    domain_cells_per_query: 256,
                    tolerance_uv: 1e-8,
                },
            )
            .unwrap();
            assert!(
                end.unresolved_products.is_empty(),
                "original corner product {faces:?}"
            );
            assert!(end.spans <= 10000);
            assert!(end.normal_spans <= 10000);
            let proof = end
                .certificate
                .expect("continuous source-bound corner exclusion");
            assert!(std::ptr::eq(proof.body(), &body));
            assert_eq!(proof.faces(), faces);
            assert_eq!(proof.lower_mm(), 5.99);
        }
        let shared = crate::source_shared_chord_map::prepare(&body, [0, 2], 100000)
            .unwrap()
            .unwrap();
        assert!(std::ptr::eq(shared.body(), &body));
        assert_eq!(shared.faces(), [0, 2]);
        assert_eq!(shared.fixed_axes(), [1, 1]);
        assert_eq!(shared.fixed_values(), [1., 1.]);
        assert_eq!(shared.free_endpoints(), [[0., 1.], [3., 0.]]);
        let source = shared.surfaces()[0];
        let (profile, profile_work) =
            nurbs_core::normal_alignment::qualify_weight_profile(source, [5, 0], 18).unwrap();
        let profile = profile.unwrap();
        assert_eq!(profile_work, 18);
        assert!(std::ptr::eq(profile.source(), source));
        assert!(profile.within_position_tolerance(1e-7).unwrap());
        let (pole_box, pole_spans) = profile.enclose([[1., 1.], [0.5, 0.5]], 16).unwrap();
        assert!(pole_spans > 0);
        let pole_box = pole_box.unwrap();
        for (interval, position) in pole_box.into_iter().zip([20., 0., 6.]) {
            assert!(
                interval[0] <= position
                    && position <= interval[1]
                    && interval[1] - interval[0] < 1e-10
            );
        }

        eprintln!(
            "annular original exact weight-profile proposal axis upper {:?} weight upper {}",
            profile.axis_deviation_upper(),
            profile.weight_deviation_upper()
        );
        assert!(
            nurbs_core::normal_alignment::qualify_weight_profile(source, [5, 0], 17)
                .unwrap()
                .0
                .is_none()
        );
        let (profile_direction, direction_spans) = profile
            .boundary_direction([[1., 1.], [0.5, 0.5]], 1, 1., 16)
            .unwrap();
        let profile_direction = profile_direction.unwrap();
        assert_eq!(direction_spans, 1);
        assert!(std::ptr::eq(profile_direction.profile(), &profile));
        assert_eq!(profile_direction.factor(), Some((true, 2)));
        assert_eq!(profile_direction.boundary(), 1.);
        assert!(
            profile_direction
                .components()
                .iter()
                .any(|r| r[0] > 0. || r[1] < 0.)
        );
        eprintln!(
            "annular exact profile pole direction {:?}",
            profile_direction.components()
        );
        let proposal_limits = || crate::source_profile_body::Limits {
            controls: 1000,
            fragments: 100,
            position_tolerance: 1e-7,
        };
        let specs = [crate::source_profile_body::FaceSpec {
            face: 0,
            anchor: [5, 0],
        }];
        let report = crate::source_profile_body::prepare(&body, &specs, proposal_limits()).unwrap();
        let proposal = report.proposal.expect(report.reason);
        assert!(std::ptr::eq(proposal.original_body(), &body));
        assert!(proposal.profile(0).is_some() && proposal.profile(1).is_none());
        assert_eq!(proposal.specs()[0].anchor, [5, 0]);
        let definition = proposal.definition().unwrap();
        assert_eq!(definition["originalBody"], body.definition().unwrap());
        assert_eq!(definition["profiles"][0]["face"], value_codec::json!(0));
        eprintln!(
            "annular profile body proposal {} controls={} fragments={}",
            report.reason, report.controls, report.fragments
        );
        let mut short = proposal_limits();
        short.controls = 17;
        assert!(
            crate::source_profile_body::prepare(&body, &specs, short)
                .unwrap()
                .proposal
                .is_none()
        );
        assert!(
            crate::source_profile_body::prepare(&body, &[specs[0], specs[0]], proposal_limits())
                .is_err()
        );
        assert_eq!((report.controls, report.fragments), (44, 4));
        for missing in [0, 1, 3] {
            let mut limits = proposal_limits();
            limits.fragments = missing;
            let refused = crate::source_profile_body::prepare(&body, &specs, limits).unwrap();
            assert!(refused.proposal.is_none());
            assert_eq!(refused.fragments, missing);
            assert_eq!(refused.reason, "profile-body-work-limit");
        }
        let neighbor = body.geometry().shell().regions().unwrap()[5].loops()[0][0].surface();
        let profile = proposal.profile(0).unwrap();
        let seam = profile
            .inspect_original_normals(
                neighbor,
                [[[0., 0.], [0.49, 0.51]], [[1., 1.], [0.49, 0.51]]],
                1e-6,
                16,
            )
            .unwrap();
        eprintln!(
            "corrected profile middle seam aligned={:?} sine={:?} reason={}",
            seam.aligned, seam.sine_squared_interval, seam.reason
        );
        assert!(seam.normal_components.iter().all(|n| n.is_some()));
        assert_eq!(seam.aligned, None);
        let point = profile
            .inspect_original_normals(
                neighbor,
                [[[0., 0.], [0.5, 0.5]], [[1., 1.], [0.5, 0.5]]],
                1e-6,
                16,
            )
            .unwrap();
        eprintln!(
            "corrected profile seam midpoint aligned={:?} sine={:?}",
            point.aligned, point.sine_squared_interval
        );
        assert_eq!(point.aligned, Some(true));
        let owned = proposal
            .qualify_seam(0, 5, 100, 1e-6, 50000, 100000)
            .unwrap();
        let certificate = owned.seam.as_ref().expect(owned.reason);
        assert!(std::ptr::eq(certificate.proposal(), &proposal));
        assert_eq!(certificate.faces(), [0, 5]);
        assert_eq!(certificate.edge(), 5);
        let whole = owned.angular.unwrap();
        eprintln!(
            "owned corrected whole seam aligned={:?} upper={:?} cells={} spans={} accepted={} pending={} reason={}",
            whole.aligned,
            whole.upper_sine_squared,
            whole.cells,
            whole.spans,
            whole.accepted_cells,
            whole.pending.len(),
            whole.reason
        );
        assert_eq!(whole.aligned, Some(true));
        assert!(whole.pending.is_empty());
        assert!(certificate.upper_sine_squared() <= certificate.threshold());
        assert_eq!(whole.cells, 14699);
        assert_eq!(whole.spans, 29398);
        let both_specs = [
            specs[0],
            crate::source_profile_body::FaceSpec {
                face: 5,
                anchor: [0, 0],
            },
        ];
        let both_report =
            crate::source_profile_body::prepare(&body, &both_specs, proposal_limits()).unwrap();
        let both = both_report.proposal.expect(both_report.reason);
        let both_seam = both.qualify_edge(5, 100, 1e-6, 50000, 100000).unwrap();
        let proof = both_seam.seam.expect(both_seam.reason);
        assert!(std::ptr::eq(proof.proposal(), &both));
        assert_eq!(proof.edge(), 5);
        let work = both_seam.angular.unwrap();
        assert!(work.pending.is_empty());
        eprintln!(
            "two-profile owned seam cells={} spans={} accepted={} upper={:?}",
            work.cells, work.spans, work.accepted_cells, work.upper_sine_squared
        );
        assert!(
            crate::source_shared_chord_map::prepare_edge(&body, [0, 5], 1, 100)
                .unwrap()
                .is_none()
        );
        assert!(
            crate::source_shared_chord_map::prepare_edge(&body, [0, 5], 5, 5)
                .unwrap()
                .is_none()
        );
        assert!(both.qualify_edge(usize::MAX, 100, 1e-6, 1, 2).is_err());
        for (index, uses) in body.geometry().shell().uses().iter().enumerate() {
            if uses.iter().any(|u| both.profile(u.face).is_some()) {
                let audit = both.qualify_edge(index, 100, 1e-6, 1, 2).unwrap();
                assert!(audit.seam.is_none());
                eprintln!(
                    "incident corrected edge {} faces={:?} reason={} pending={:?}",
                    index,
                    [uses[0].face, uses[1].face],
                    audit.reason,
                    audit.angular.map(|r| r.pending)
                );
            }
        }
        for index in [1, 8, 15] {
            let report = both.qualify_edge(index, 100, 1e-6, 50000, 100000).unwrap();
            let angular = report.angular.expect("owned seam route");
            if index == 1 {
                assert!(report.seam.is_none());
                assert_eq!(angular.aligned, None);
                assert_eq!(report.reason, "profile-seam-endpoint-normal-unresolved");
                assert!(angular.cells <= 2);
                assert!(!angular.pending.is_empty());
            } else {
                assert!(report.seam.is_some());
                assert_eq!(angular.aligned, Some(true));
            }

            eprintln!(
                "remaining corrected edge {} aligned={:?} cells={} spans={} accepted={} pending={} reason={} upper={:?}",
                index,
                angular.aligned,
                angular.cells,
                angular.spans,
                angular.accepted_cells,
                angular.pending.len(),
                report.reason,
                angular.upper_sine_squared
            );
            if let Some(proof) = report.seam {
                assert_eq!(proof.edge(), index);
                assert!(std::ptr::eq(proof.proposal(), &both));
                assert!(angular.pending.is_empty());
                assert!(proof.upper_sine_squared() <= 1e-6);
            } else {
                assert!(!angular.pending.is_empty());
            }
        }
        for index in [0, 6] {
            let report = both.qualify_edge(index, 100, 1e-6, 10000, 40000).unwrap();
            let angular = report.angular.expect("original pcurve route");
            if index == 0 {
                assert!(report.seam.is_none());
                assert_eq!(angular.aligned, None);
                assert_eq!(report.reason, "profile-seam-endpoint-normal-unresolved");
                assert!(!angular.pending.is_empty());
            } else {
                let proof = report.seam.expect(report.reason);
                assert_eq!(proof.edge(), 6);
                assert!(std::ptr::eq(proof.proposal(), &both));
                assert_eq!(angular.aligned, Some(true));
                assert!(angular.pending.is_empty());
                assert!(proof.upper_sine_squared() <= 1e-6);
                assert_eq!(
                    (angular.cells, angular.spans, angular.accepted_cells),
                    (291, 1164, 147)
                );
            }

            eprintln!(
                "curved corrected edge {} aligned={:?} cells={} spans={} accepted={} pending={} reason={} upper={:?}",
                index,
                angular.aligned,
                angular.cells,
                angular.spans,
                angular.accepted_cells,
                angular.pending.len(),
                report.reason,
                angular.upper_sine_squared
            );
        }
        let limited = proposal.qualify_seam(0, 5, 0, 1e-6, 50000, 100000).unwrap();
        assert!(limited.seam.is_none() && limited.angular.is_none());
        let limited = proposal.qualify_seam(0, 5, 100, 1e-6, 1, 1).unwrap();
        assert!(limited.seam.is_none());
        assert!(!limited.angular.unwrap().pending.is_empty());
        let mut exact = proposal_limits();
        exact.controls = 44;
        exact.fragments = 4;
        assert!(
            crate::source_profile_body::prepare(&body, &specs, exact)
                .unwrap()
                .proposal
                .is_some()
        );
        let mut missing = proposal_limits();
        missing.controls = 43;
        let refused = crate::source_profile_body::prepare(&body, &specs, missing).unwrap();
        assert!(refused.proposal.is_none());
        assert_eq!(refused.controls, 43);

        let first = shared.canonical_bounds(0, [[0.25, 0.5], [0., 1.]]).unwrap();
        let second = shared.canonical_bounds(1, [[1.5, 2.25], [0., 1.]]).unwrap();
        assert!(first[0] <= 0.25 && first[1] >= 0.5);
        assert!(second[0] <= 0.25 && second[1] >= 0.5);
        assert!(
            shared
                .canonical_bounds(0, [[-0.01, 0.5], [0., 1.]])
                .is_err()
        );
        let (tangent, spans) = shared
            .tangent_bounds([[[0.25, 0.5], [0., 1.]], [[1.5, 2.25], [0., 1.]]], 64)
            .unwrap();
        assert!(spans > 0);
        assert!(
            tangent
                .unwrap()
                .iter()
                .all(|r| r[0].is_finite() && r[1].is_finite() && r[0] <= r[1])
        );
        let shared_rects = [[[0.25, 0.5], [0.8, 1.]], [[1.5, 2.25], [0.8, 1.]]];
        let (_, shared_spans, shared_normals) = shared
            .exclude_normal_chords(shared_rects, 1e-6, 64, 64)
            .unwrap();
        assert!(shared_spans > 0 && shared_spans <= 64 && shared_normals <= 64);
        assert_eq!(
            shared
                .exclude_normal_chords(shared_rects, 1e-6, 0, 64)
                .unwrap(),
            (false, 0, 0)
        );
        assert_eq!(
            shared
                .exclude_normal_chords(shared_rects, 1e-6, 64, 0)
                .unwrap(),
            (false, 0, 0)
        );
        assert!(
            shared
                .exclude_normal_chords(shared_rects, f64::NAN, 64, 64)
                .is_err()
        );
        let narrow = [
            [[0.4, 0.400000001], [0.999999999, 1.]],
            [[1.799999997, 1.8], [0.999999999, 1.]],
        ];
        let (excluded, used, normal_used) =
            shared.exclude_normal_chords(narrow, 1e-6, 64, 64).unwrap();
        assert!(
            excluded,
            "shared rolling boundary exclusion used {used}/{normal_used}"
        );
        assert!(!shared.exclude_normal_chords(narrow, 1e-6, 1, 64).unwrap().0);
        assert!(
            shared
                .exclude_normal_chords_with_mean(narrow, 1e-6, 64, 64)
                .unwrap()
                .0
        );
        assert!(
            !shared
                .exclude_normal_chords_with_mean(narrow, 1e-6, 8, 64)
                .unwrap()
                .0
        );
        assert!(
            shared
                .exclude_normal_chords_with_secant(narrow, 1e-6, 64, 64)
                .unwrap()
                .0
        );
        assert!(
            shared
                .exclude_normal_chords_with_factored_secant(narrow, 1e-6, 64, 64)
                .unwrap()
                .0
        );

        assert!(
            crate::source_shared_chord_map::prepare(&body, [0, 2], 0)
                .unwrap()
                .is_none()
        );
        let reverse = crate::source_shared_chord_map::prepare(&body, [2, 0], 100000)
            .unwrap()
            .unwrap();
        assert_eq!(reverse.edge(), shared.edge());
        assert_eq!(reverse.free_endpoints(), [[3., 0.], [0., 1.]]);
        let prepared = crate::source_wall_anchor::prepare(
            &body,
            [0, 1],
            [[1., 0.], [20., 0.]],
            [20., 0., 6.],
            10000000,
        )
        .unwrap();
        assert!(prepared.prepared.is_some());
        assert!(prepared.exact_work > 0 && prepared.exact_work <= 10000000);
        let automatic =
            crate::source_wall_anchor::prepare_boundary_anchor(&body, [0, 1], 2000000).unwrap();
        assert!(automatic.prepared.is_some());
        assert!(automatic.exact_work <= 2000000);
        assert!(
            crate::source_wall_anchor::prepare_boundary_anchor(&body, [0, 1], 0)
                .unwrap()
                .prepared
                .is_none()
        );
        let paths = prepared.prepared.unwrap();
        let product = [
            [[0.9999999925494194, 1.], [0., 7.450580596923828e-9]],
            [[19.999999990686774, 20.], [-9.313225746154785e-9, 0.]],
        ];
        let (proof, used) = paths
            .qualify_product(product, [-1., -1., -1.], 1e-6, 16)
            .unwrap();
        assert!(proof.is_some(), "charged product spans {used}");
        assert!(used <= 16);
        let projections = [[0., 0., 0.], [-1., -1., -1.]];
        let (batch, batch_spans) = paths
            .qualify_product_projections(product, &projections, 1e-6, 16)
            .unwrap();
        assert!(batch.is_some());
        assert!(batch_spans <= 16);
        assert!(
            paths
                .qualify_product_projections(product, &[], 1e-6, 16)
                .is_err()
        );
        assert!(
            paths
                .qualify_product_projections(
                    product,
                    &[[-1., -1., -1.], [f64::NAN, 0., 0.]],
                    1e-6,
                    16
                )
                .is_err()
        );
        assert!(
            paths
                .qualify_product_projections(product, &[[1., 1., 1.]; 27], 1e-6, 16)
                .is_err()
        );
        assert!(
            paths
                .qualify_product_projections(product, &projections, 1e-6, 1)
                .unwrap()
                .0
                .is_none()
        );

        let limits = || crate::source_wall_pair::Limits {
            normal_spans: 100,
            max_sine_squared: 1e-6,
            cells: 1,
            spans: 100,
            domain_cells: 100,
            domain_cells_per_query: 16,
            tolerance_uv: 1e-8,
        };
        assert!(
            crate::source_wall_pair::qualify_with_anchors(&body, [1, 0], 5.99, limits(), &paths)
                .is_err()
        );
        let partial =
            crate::source_wall_pair::qualify_with_anchors(&body, [0, 1], 5.99, limits(), &paths)
                .unwrap();
        assert!(partial.certificate.is_none());
        assert!(!partial.unresolved_products.is_empty());

        assert!(
            paths
                .qualify_product(product, [-1., -1., -1.], 1e-6, 1)
                .unwrap()
                .0
                .is_none()
        );

        assert!(
            crate::source_wall_anchor::prepare(
                &body,
                [0, 1],
                [[1., 0.], [20., 0.]],
                [20., 0., 6.],
                0
            )
            .unwrap()
            .prepared
            .is_none()
        );

        let numerical = crate::source_wall_pair::qualify(
            &body,
            [16, 20],
            5.99,
            crate::source_wall_pair::Limits {
                normal_spans: 10000,
                max_sine_squared: 1e-6,
                cells: 4096,
                spans: 10000,
                domain_cells: 10000,
                domain_cells_per_query: 256,
                tolerance_uv: 1e-8,
            },
        )
        .unwrap();
        assert!(numerical.certificate.is_none());
        // The UV stop now avoids the former underflow path; work remains unproven.
        assert_eq!(numerical.reason, "source-wall-pair-work-limit");
        assert_eq!(numerical.cells, 4096);
        assert!(!numerical.unresolved_products.is_empty());
        assert!(numerical.normal_spans <= 10000);
        let shell = body.geometry().shell();
        let flat_at = |z: f64| {
            shell
                .faces()
                .iter()
                .enumerate()
                .filter_map(|(i, w)| {
                    let s = w[0].edges()[0].surface();
                    s.control_points
                        .iter()
                        .flatten()
                        .all(|p| p[2] == z)
                        .then_some(i)
                })
                .collect::<Vec<_>>()
        };
        let bottom = flat_at(0.);
        let top = flat_at(6.);
        assert!(!bottom.is_empty() && !top.is_empty());
        let gap = crate::source_face_gap::qualify(
            &body,
            [&bottom, &top],
            5.99,
            crate::source_face_gap::Limits {
                cells: 1000,
                spans: 2000,
            },
        )
        .unwrap();
        eprintln!(
            "original annular flat face gap {} cells={} spans={} bottom={bottom:?} top={top:?}",
            gap.reason, gap.cells, gap.spans
        );
        let proof = gap
            .certificate
            .expect("all original flat top/bottom pairs need clearance");
        assert!(proof.lower_mm() >= 5.99 && proof.lower_mm() <= 6.);
        assert_eq!(proof.faces(), [bottom.as_slice(), top.as_slice()]);
        assert!(std::ptr::eq(proof.body(), &body));
        let too_large = crate::source_face_gap::qualify(
            &body,
            [&bottom, &top],
            7.,
            crate::source_face_gap::Limits {
                cells: 8,
                spans: 16,
            },
        )
        .unwrap();
        assert!(too_large.certificate.is_none());
        assert!(too_large.uncertain_faces.is_some() && too_large.uncertain_uv.is_some());
        assert!(too_large.cells <= 8 && too_large.spans <= 16);
        assert!(
            crate::source_face_gap::qualify(
                &body,
                [&bottom, &top],
                5.99,
                crate::source_face_gap::Limits { cells: 1, spans: 1 }
            )
            .unwrap()
            .certificate
            .is_none()
        );
        assert!(
            crate::source_face_gap::qualify(
                &body,
                [&bottom, &bottom],
                5.99,
                crate::source_face_gap::Limits {
                    cells: 1000,
                    spans: 2000
                }
            )
            .is_err()
        );
        let curved_gap = crate::source_face_gap::qualify(
            &body,
            [&[2, 7, 12, 16, 20, 24], &[3, 8, 13, 17, 21, 25]],
            14.5,
            crate::source_face_gap::Limits {
                cells: 10000,
                spans: 20000,
            },
        )
        .unwrap();
        eprintln!(
            "original annular curved gap {} cells={} spans={} uncertain={:?}",
            curved_gap.reason, curved_gap.cells, curved_gap.spans, curved_gap.uncertain_faces
        );
        let curved_proof = curved_gap
            .certificate
            .expect("all original cylindrical inner/outer pairs need clearance");
        assert!(curved_proof.lower_mm() >= 14.5 && curved_proof.lower_mm() <= 15.);
        assert!(curved_gap.cells >= 36 && curved_gap.cells <= 10000 && curved_gap.spans <= 20000);
        let outside_segment = crate::source_material_segment::inspect_boundary(
            &body,
            [30., 30., 1.],
            [0., 0., 1.],
            1e-7,
            10000,
            10000,
        )
        .unwrap();
        assert!(outside_segment.boundary_free);
        assert!(outside_segment.contacts.is_empty() && outside_segment.unresolved.is_empty());
        let crossing_segment = crate::source_material_segment::inspect_boundary(
            &body,
            [10., 2., -1.],
            [0., 0., 8.],
            1e-7,
            10000,
            10000,
        )
        .unwrap();
        eprintln!(
            "original source segment contacts={} unresolved={} cells={} domains={}",
            crossing_segment.contacts.len(),
            crossing_segment.unresolved.len(),
            crossing_segment.cells,
            crossing_segment.domain_cells
        );
        assert!(!crossing_segment.boundary_free);
        assert_eq!(crossing_segment.contacts.len(), 2);
        assert!(crossing_segment.unresolved.is_empty());
        assert!(
            crossing_segment.contacts[0].parameter[1] < crossing_segment.contacts[1].parameter[0]
                || crossing_segment.contacts[1].parameter[1]
                    < crossing_segment.contacts[0].parameter[0]
        );
        assert!(
            crate::source_material_segment::inspect_boundary(
                &body, [0.; 3], [0.; 3], 1e-7, 10000, 10000
            )
            .is_err()
        );
        let chord = crate::source_material_chord::qualify(
            &body,
            [10., 2., -1.],
            [0., 0., 8.],
            1e-7,
            crate::source_material_chord::Limits {
                cells: 10000,
                domain_cells: 10000,
                normal_spans: 1000,
                max_sine_squared: 1e-6,
            },
        )
        .unwrap();
        eprintln!(
            "source normal chord {} length={:?}",
            chord.reason,
            chord.certificate.as_ref().map(|c| c.length_mm())
        );
        let proof = chord
            .certificate
            .expect("original annular flat normal material chord");
        assert!(std::ptr::eq(proof.body(), &body));
        assert!(proof.length_mm()[0] <= 6. && proof.length_mm()[1] >= 6.);
        assert!(proof.length_mm()[1] - proof.length_mm()[0] < 1e-5);
        for (origin, direction, cells, normal_spans) in [
            ([10., 2., 1.], [0., 0., 4.], 10000, 1000),
            ([30., 30., 1.], [0., 0., 1.], 10000, 1000),
            ([10., 2., -1.], [0., 0., 8.], 1, 1000),
            ([10., 2., -1.], [1., 0., 8.], 10000, 1000),
            ([10., 2., -1.], [0., 0., 8.], 10000, 1),
        ] {
            let denied = crate::source_material_chord::qualify(
                &body,
                origin,
                direction,
                1e-7,
                crate::source_material_chord::Limits {
                    cells,
                    domain_cells: 10000,
                    normal_spans,
                    max_sine_squared: 1e-6,
                },
            )
            .unwrap();
            eprintln!("source chord refusal {}", denied.reason);
            assert!(denied.certificate.is_none());
        }
        for (groups, minimum, tolerance, expected) in [
            ([&bottom[..], &top[..]], 5.99, 0.02, true),
            ([&bottom[..], &top[..]], 5., 1e-16, false),
            (
                [&[2, 7, 12, 16, 20, 24][..], &[3, 8, 13, 17, 21, 25][..]],
                14.5,
                0.02,
                false,
            ),
        ] {
            let wall = crate::source_material_wall::qualify(
                &body,
                groups,
                minimum,
                tolerance,
                [10., 2., -1.],
                [0., 0., 8.],
                1e-7,
                crate::source_material_wall::Limits {
                    gap: crate::source_face_gap::Limits {
                        cells: 10000,
                        spans: 20000,
                    },
                    chord: crate::source_material_chord::Limits {
                        cells: 10000,
                        domain_cells: 10000,
                        normal_spans: 1000,
                        max_sine_squared: 1e-6,
                    },
                },
            )
            .unwrap();
            eprintln!(
                "source wall {} interval={:?} converged={}",
                wall.reason,
                wall.certificate.as_ref().map(|c| c.interval_mm()),
                wall.converged
            );
            assert_eq!(wall.converged, expected);
            if minimum == 14.5 {
                assert!(wall.certificate.is_none());
            } else {
                let proof = wall.certificate.unwrap();
                assert!(std::ptr::eq(proof.body(), &body));
                assert!(proof.interval_mm()[0] <= 6. && proof.interval_mm()[1] >= 6.);
            }
        }
        let search = crate::source_wall_search::search(
            &body,
            [&bottom, &top],
            3,
            54,
            1e-7,
            crate::source_material_chord::Limits {
                cells: 10000,
                domain_cells: 10000,
                normal_spans: 1000,
                max_sine_squared: 1e-6,
            },
        )
        .unwrap();
        eprintln!(
            "source wall search attempts={} refused={} exhausted={} best={:?}",
            search.attempts,
            search.refused,
            search.candidates_exhausted,
            search.best.as_ref().map(|c| c.length_mm())
        );
        assert_eq!(search.attempts, 54);
        assert!(search.candidates_exhausted);
        let best = search
            .best
            .expect("automatic original annular axial material chord");
        assert!(best.length_mm()[0] <= 6. && best.length_mm()[1] >= 6.);
        assert!(best.length_mm()[1] - best.length_mm()[0] < 1e-5);
        let limited = crate::source_wall_search::search(
            &body,
            [&bottom, &top],
            3,
            1,
            1e-7,
            crate::source_material_chord::Limits {
                cells: 10000,
                domain_cells: 10000,
                normal_spans: 1000,
                max_sine_squared: 1e-6,
            },
        )
        .unwrap();
        assert_eq!(limited.attempts, 1);
        assert!(!limited.candidates_exhausted);
        assert!(
            crate::source_wall_search::search(
                &body,
                [&bottom, &bottom],
                3,
                1,
                1e-7,
                crate::source_material_chord::Limits {
                    cells: 10000,
                    domain_cells: 10000,
                    normal_spans: 1000,
                    max_sine_squared: 1e-6
                }
            )
            .is_err()
        );
        for (cells, expected) in [(1000, true), (1, false)] {
            let automatic = crate::source_material_wall::search_and_qualify(
                &body,
                [&bottom, &top],
                5.99,
                0.02,
                1e-7,
                3,
                54,
                crate::source_material_wall::Limits {
                    gap: crate::source_face_gap::Limits { cells, spans: 2000 },
                    chord: crate::source_material_chord::Limits {
                        cells: 10000,
                        domain_cells: 10000,
                        normal_spans: 1000,
                        max_sine_squared: 1e-6,
                    },
                },
            )
            .unwrap();
            eprintln!(
                "source automatic wall {} converged={} interval={:?}",
                automatic.reason,
                automatic.converged,
                automatic.certificate.as_ref().map(|c| c.interval_mm())
            );
            assert_eq!(automatic.converged, expected);
            assert_eq!(automatic.certificate.is_some(), expected);
            if let Some(proof) = automatic.certificate {
                assert!(std::ptr::eq(proof.body(), &body));
                assert!(proof.interval_mm()[0] <= 6. && proof.interval_mm()[1] >= 6.);
            } else {
                assert!(
                    automatic.search.best.is_some(),
                    "witness alone must not admit minimum thickness"
                );
            }
        }
        let radial = crate::source_material_chord::qualify_between(
            &body,
            [&[2, 7, 12, 16, 20, 24], &[3, 8, 13, 17, 21, 25]],
            [25., 5., 3.],
            [-50., -10., 0.],
            1e-7,
            crate::source_material_chord::Limits {
                cells: 10000,
                domain_cells: 10000,
                normal_spans: 1000,
                max_sine_squared: 1e-6,
            },
        )
        .unwrap();
        eprintln!(
            "source cavity radial chord {} contacts={} length={:?}",
            radial.reason,
            radial.boundary.contacts.len(),
            radial.certificate.as_ref().map(|c| c.length_mm())
        );
        assert_eq!(radial.boundary.contacts.len(), 4);
        let proof = radial
            .certificate
            .expect("original annular radial material interval around cavity");
        assert!(proof.length_mm()[0] <= 15. && proof.length_mm()[1] >= 15.);
        assert!(proof.length_mm()[1] - proof.length_mm()[0] < 1e-5);
        assert_eq!(proof.line(), [[25., 5., 3.], [-50., -10., 0.]]);
        let endpoints = proof.endpoints();
        for (i, endpoint) in endpoints.iter().enumerate() {
            let crossing = &radial.boundary.contacts[i];
            assert_eq!(endpoint.face, crossing.face);
            assert_eq!(endpoint.uv, crossing.uv);
            assert_eq!(endpoint.parameter, crossing.parameter);
            let surface =
                body.geometry().shell().regions().unwrap()[endpoint.face].loops()[0][0].surface();
            assert_eq!(
                endpoint.world_mm.to_vec(),
                nurbs_core::surface_distance::rectangle_bounds(surface, endpoint.uv).unwrap()
            );
            assert!(
                endpoint
                    .world_mm
                    .iter()
                    .all(|v| v[0].is_finite() && v[0] <= v[1] && v[1].is_finite())
            );
        }
        assert!(endpoints[0].parameter[1] < endpoints[1].parameter[0]);
        // Independent analytic radial oracle; rounded oracle coordinates are
        // checked with an explicit 1e-9 mm tolerance, never used for admission.
        let radial_unit = [25. / 650f64.sqrt(), 5. / 650f64.sqrt(), 0.];
        for (end, radius) in endpoints.iter().zip([20., 5.]) {
            let expected = [radius * radial_unit[0], radius * radial_unit[1], 3.];
            for (bounds, value) in end.world_mm.iter().zip(expected) {
                assert!(bounds[0] - 1e-9 <= value && value <= bounds[1] + 1e-9);
                assert!(bounds[1] - bounds[0] < 1e-5);
            }
            let expected_t = (650f64.sqrt() - radius) / (2. * 650f64.sqrt());
            assert!(
                end.parameter[0] - 1e-11 <= expected_t && expected_t <= end.parameter[1] + 1e-11
            );
        }

        let scan = crate::source_wall_scan::search(
            &body,
            7.,
            3,
            243,
            1e-7,
            crate::source_material_chord::Limits {
                cells: 10000,
                domain_cells: 10000,
                normal_spans: 1000,
                max_sine_squared: 1e-6,
            },
        )
        .unwrap();
        eprintln!(
            "original all-face wall scan attempts={} refused={} visited={} exhausted={} best={:?}",
            scan.attempts,
            scan.refused,
            scan.faces_visited,
            scan.proposals_exhausted,
            scan.best().map(|c| c.length_mm())
        );
        assert_eq!(scan.attempts, 243);
        assert_eq!(scan.faces_visited, 27);
        assert!(scan.proposals_exhausted);
        let thin = scan
            .thin_witness()
            .expect("automatic native scan must find the 6 mm wall");
        assert!(thin.length_mm()[1] < 7.);
        assert!(std::ptr::eq(thin.body(), &body));
        let coarse_scan = crate::source_wall_scan::search(
            &body,
            7.,
            1,
            27,
            1e-7,
            crate::source_material_chord::Limits {
                cells: 10000,
                domain_cells: 10000,
                normal_spans: 1000,
                max_sine_squared: 1e-6,
            },
        )
        .unwrap();
        assert!(coarse_scan.proposals_exhausted);
        assert_eq!(coarse_scan.faces_visited, 27);
        assert!(
            coarse_scan.thin_witness().is_none(),
            "exhausted coarse proposals must not assert the missing 6 mm witness"
        );
        let coarse = coarse_scan.best().unwrap();
        assert!(coarse.length_mm()[0] <= 15. && coarse.length_mm()[1] >= 15.);
        for (minimum, grid, attempts) in [
            (f64::NAN, 3, 243),
            (0., 3, 243),
            (7., 0, 243),
            (7., 9, 243),
            (7., 3, 0),
            (7., 3, 257),
        ] {
            assert!(
                crate::source_wall_scan::search(
                    &body,
                    minimum,
                    grid,
                    attempts,
                    1e-7,
                    crate::source_material_chord::Limits {
                        cells: 10000,
                        domain_cells: 10000,
                        normal_spans: 1000,
                        max_sine_squared: 1e-6
                    }
                )
                .is_err()
            );
        }
        let limited_scan = crate::source_wall_scan::search(
            &body,
            1.,
            1,
            1,
            1e-7,
            crate::source_material_chord::Limits {
                cells: 10000,
                domain_cells: 10000,
                normal_spans: 1000,
                max_sine_squared: 1e-6,
            },
        )
        .unwrap();
        assert_eq!(limited_scan.attempts, 1);
        assert!(!limited_scan.proposals_exhausted);
        assert!(limited_scan.thin_witness().is_none());
        for pair_budget in [1, 378] {
            let coverage = crate::source_wall_coverage::qualify(
                &body,
                5.99,
                crate::source_wall_coverage::Limits {
                    pairs: pair_budget,
                    plane_controls: 10000,
                    normal_spans: 1000,
                    gap: crate::source_face_gap::Limits {
                        cells: 1000,
                        spans: 2000,
                    },
                    max_sine_squared: 1e-6,
                },
            )
            .unwrap();
            eprintln!(
                "original continuous wall coverage reason={} pairs={}/{} unresolved={} cells={} normals={}",
                coverage.reason,
                coverage.pairs.len(),
                coverage.total_pairs,
                coverage.pairs.iter().filter(|p| !p.proven).count(),
                coverage.cells,
                coverage.normal_spans
            );
            assert_eq!(coverage.total_pairs, 378);
            assert_eq!(coverage.enumeration_complete, pair_budget == 378);
            assert!(coverage.certificate.is_none());
            assert!(
                coverage.cells <= 1000 && coverage.spans <= 2000 && coverage.normal_spans <= 1000
            );
            if pair_budget == 378 {
                assert!(
                    coverage
                        .pairs
                        .iter()
                        .any(|p| p.faces[0] == p.faces[1] && !p.proven)
                );
            }
        }
        let self_coverage = crate::source_wall_self_coverage::qualify(
            &body,
            5.99,
            1e-6,
            crate::source_wall_self_coverage::Limits {
                plane_controls: 10000,
                cells: 10000,
                spans: 100000,
                cells_per_face: 512,
                spans_per_face: 4096,
            },
        )
        .unwrap();
        eprintln!(
            "original adaptive self coverage qualified={}/{} cells={} spans={} pending={}",
            self_coverage.certificates().len(),
            self_coverage.faces.len(),
            self_coverage.cells,
            self_coverage.spans,
            self_coverage.faces.iter().map(|f| f.pending).sum::<usize>()
        );
        assert_eq!(self_coverage.faces.len(), 27);
        assert!(self_coverage.certificates().len() >= 14);
        assert!(
            self_coverage.cells <= 10000
                && self_coverage.spans <= 100000
                && self_coverage.plane_controls <= 10000
        );
        for (i, face) in self_coverage.faces.iter().enumerate() {
            assert_eq!(face.face, i);
            assert!(face.cells <= 512 && face.spans <= 4096);
            assert_eq!(face.qualified, face.pending == 0);
            assert_eq!(face.qualified, face.uncertain.is_none());
            assert_eq!(face.pending, face.unresolved_pairs.len());
            assert_eq!(face.uncertain, face.unresolved_pairs.last().copied());
        }
        for c in self_coverage.certificates() {
            assert!(std::ptr::eq(c.body(), &body));
            assert_eq!(c.minimum_mm(), 5.99);
            assert_eq!(c.max_sine_squared(), 1e-6);
            if let Some(proof) = c.adaptive() {
                assert!(std::ptr::eq(
                    proof.surface(),
                    body.geometry().shell().regions().unwrap()[c.face()].loops()[0][0].surface()
                ));
            }
        }
        assert!(!self_coverage.all_self_pairs_qualified());
        let mut self_coverage = self_coverage;
        self_coverage.faces.retain(|f| f.qualified);
        assert!(
            !self_coverage.all_self_pairs_qualified(),
            "public diagnostics cannot change original face coverage"
        );
        let issued_self_faces: std::collections::BTreeSet<_> = self_coverage
            .certificates()
            .iter()
            .map(|c| c.face())
            .collect();
        assert!(
            issued_self_faces.len() >= 16,
            "retain the previously qualified face classes"
        );
        let reused = crate::source_wall_coverage::qualify_with_self_facts(
            &body,
            5.99,
            crate::source_wall_coverage::Limits {
                pairs: 378,
                plane_controls: 10000,
                normal_spans: 1000,
                gap: crate::source_face_gap::Limits {
                    cells: 1000,
                    spans: 2000,
                },
                max_sine_squared: 1e-6,
            },
            self_coverage.into_certificates(),
        )
        .unwrap();
        eprintln!(
            "original reused adaptive wall pairs={} unresolved={} cells={} normals={}",
            reused.pairs.len(),
            reused.pairs.iter().filter(|p| !p.proven).count(),
            reused.cells,
            reused.normal_spans
        );
        assert!(reused.enumeration_complete);
        assert!(reused.certificate.is_none());
        for product_cells in [1, 50000] {
            let refined = crate::source_wall_coverage::qualify_with_self_facts_and_normal_products(
                &body,
                5.99,
                crate::source_wall_coverage::Limits {
                    pairs: 378,
                    plane_controls: 10000,
                    normal_spans: 20000,
                    gap: crate::source_face_gap::Limits {
                        cells: 1000,
                        spans: 2000,
                    },
                    max_sine_squared: 1e-6,
                },
                Vec::new(),
                crate::source_wall_coverage::NormalProductLimits {
                    cells: product_cells,
                    cells_per_pair: 128,
                },
            )
            .unwrap();
            let unresolved = refined.pairs.iter().filter(|p| !p.proven).count();
            eprintln!(
                "native normal-product cells={} normals={} unresolved={}",
                refined.normal_product_cells, refined.normal_spans, unresolved
            );
            assert!(refined.enumeration_complete);
            assert!(refined.certificate.is_none());
            assert!(refined.normal_product_cells <= product_cells && refined.normal_spans <= 20000);
            if product_cells == 50000 {
                assert!(
                    unresolved < 257,
                    "full-product exclusions improve original whole-wall coverage"
                );
            }
            for previous in reused
                .pairs
                .iter()
                .filter(|p| p.proven && p.faces[0] != p.faces[1])
            {
                assert!(
                    refined
                        .pairs
                        .iter()
                        .any(|p| p.faces == previous.faces && p.proven),
                    "retain previously confirmed distinct pairs {:?}",
                    previous.faces
                );
            }
        }
        for pair in &reused.pairs {
            let adjacent = body.geometry().shell().uses().iter().any(|u| {
                (u[0].face == pair.faces[0] && u[1].face == pair.faces[1])
                    || (u[1].face == pair.faces[0] && u[0].face == pair.faces[1])
            });
            if adjacent {
                assert!(
                    !pair.reason.starts_with("source-face-gap-"),
                    "shared-boundary pairs cannot consume clearance subdivision"
                );
            }
        }

        let transported_self_faces: std::collections::BTreeSet<_> = reused
            .pairs
            .iter()
            .filter(|p| p.reason == "source-wall-self-certified")
            .map(|p| {
                assert!(p.proven && p.faces[0] == p.faces[1]);
                p.faces[0]
            })
            .collect();
        assert_eq!(
            transported_self_faces, issued_self_faces,
            "every issued source-owned self certificate must be reused exactly once"
        );
        assert!(reused.pairs.iter().filter(|p| !p.proven).count() <= 256);
        let cavity_faces = [
            radial.boundary.contacts[1].face,
            radial.boundary.contacts[2].face,
        ];
        let cavity = crate::source_material_chord::qualify_between(
            &body,
            [&[cavity_faces[0]], &[cavity_faces[1]]],
            [25., 5., 3.],
            [-50., -10., 0.],
            1e-7,
            crate::source_material_chord::Limits {
                cells: 10000,
                domain_cells: 10000,
                normal_spans: 1000,
                max_sine_squared: 1e-6,
            },
        )
        .unwrap();
        assert!(cavity.certificate.is_none());
        assert_eq!(cavity.reason, "material-pair-outside-groups");
        let automatic_radial = crate::source_material_wall::search_and_qualify(
            &body,
            [&[2, 7, 12, 16, 20, 24], &[3, 8, 13, 17, 21, 25]],
            14.5,
            0.6,
            1e-7,
            3,
            54,
            crate::source_material_wall::Limits {
                gap: crate::source_face_gap::Limits {
                    cells: 10000,
                    spans: 20000,
                },
                chord: crate::source_material_chord::Limits {
                    cells: 10000,
                    domain_cells: 10000,
                    normal_spans: 1000,
                    max_sine_squared: 1e-6,
                },
            },
        )
        .unwrap();
        eprintln!(
            "source automatic radial wall {} attempts={} refused={} interval={:?}",
            automatic_radial.reason,
            automatic_radial.search.attempts,
            automatic_radial.search.refused,
            automatic_radial
                .certificate
                .as_ref()
                .map(|c| c.interval_mm())
        );
        assert!(automatic_radial.converged);
        let proof = automatic_radial
            .certificate
            .expect("automatically found curved cavity wall");
        assert!(proof.interval_mm()[0] <= 15. && proof.interval_mm()[1] >= 15.);
        for (cells, spans, tolerance, expected) in
            [(50000, 100000, 0.02, true), (10000, 20000, 1e-16, false)]
        {
            let refined = crate::source_material_wall::search_and_refine(
                &body,
                [&[2, 7, 12, 16, 20, 24], &[3, 8, 13, 17, 21, 25]],
                14.5,
                tolerance,
                1e-7,
                3,
                54,
                crate::source_material_wall::Limits {
                    gap: crate::source_face_gap::Limits { cells, spans },
                    chord: crate::source_material_chord::Limits {
                        cells: 10000,
                        domain_cells: 10000,
                        normal_spans: 1000,
                        max_sine_squared: 1e-6,
                    },
                },
            )
            .unwrap();
            eprintln!(
                "source refined radial wall converged={} cells={} spans={} interval={:?} refinement={:?}",
                refined.result.converged,
                refined.cells,
                refined.spans,
                refined.result.certificate.as_ref().map(|c| c.interval_mm()),
                refined.refinement.as_ref().map(|r| r.reason)
            );
            assert!(refined.cells <= cells && refined.spans <= spans);
            assert_eq!(refined.result.converged, expected);
            let proof = refined
                .result
                .certificate
                .expect("retain valid original lower bounds even if refinement exhausts");
            assert!(proof.interval_mm()[0] <= 15. && proof.interval_mm()[1] >= 15.);
            if expected {
                assert!(proof.interval_mm()[1] - proof.interval_mm()[0] <= 0.02);
            }
        }
        let blended_gap = crate::source_face_gap::qualify(
            &body,
            [&[0, 2, 5, 7, 10, 12, 16, 20, 24], &[3, 8, 13, 17, 21, 25]],
            13.5,
            crate::source_face_gap::Limits {
                cells: 50000,
                spans: 100000,
            },
        )
        .unwrap();
        eprintln!(
            "original annular blend wall gap {} cells={} spans={} uncertain={:?}",
            blended_gap.reason, blended_gap.cells, blended_gap.spans, blended_gap.uncertain_faces
        );
        let blended_proof = blended_gap
            .certificate
            .expect("all original transition/cylinder versus inner wall pairs need clearance");
        assert!(blended_proof.lower_mm() >= 13.5 && blended_proof.lower_mm() <= 13.75);
        assert!(
            blended_gap.cells >= 54 && blended_gap.cells <= 50000 && blended_gap.spans <= 100000
        );
        let q = std::f64::consts::FRAC_PI_2;
        let spans = [
            crate::circular_blend::plane_cylinder_transition(20., 6., 0., 1.25, 0., q / 4.)
                .unwrap(),
            crate::circular_blend::plane_cylinder_rim(20., 6., 1.25, q / 4., q / 2.)
                .unwrap()
                .remove(0),
            crate::circular_blend::plane_cylinder_transition(20., 6., 1.25, 0., q * 0.75, q / 4.)
                .unwrap(),
        ];
        for (face, span) in [0, 5, 10].into_iter().zip(spans) {
            let actual = body.geometry().shell().faces()[face][0].edges()[0].surface();
            let mut centers = span.centers.clone();
            let mut law = span.radius_curve();
            if actual != &span.surface {
                let mut reversed = span.surface.clone();
                reversed.control_points.reverse();
                reversed.weights.reverse();
                assert_eq!(
                    actual, &reversed,
                    "original face {face} must match the authored parameter orientation"
                );
                centers = centers.reverse().unwrap();
                law = law.reverse().unwrap();
            }
            let tolerance = if face == 5 { 1e-9 } else { 1e-6 };
            let r = body
                .qualify_face_radius(face, &centers, &law, tolerance, 100, 100_000_000)
                .unwrap();
            eprintln!(
                "original body radius face={face} reason={} bound={:?} work={}",
                r.reason, r.error_upper, r.work
            );
            let certificate = r
                .certificate
                .expect("original body patch radius needs full-domain certificate");
            assert_eq!(certificate.surface(), actual);
            assert!(
                body.qualify_face_radius(face, &centers, &law, tolerance, 100, 1)
                    .unwrap()
                    .certificate
                    .is_none()
            );
            let mut wrong = law.clone();
            wrong.control_points[0][0] += 0.1;
            assert!(
                body.qualify_face_radius(face, &centers, &wrong, tolerance, 100, 100_000_000)
                    .unwrap()
                    .certificate
                    .is_none()
            );
        }
        for neighbor in [6, 7] {
            let edge = body
                .geometry()
                .shell()
                .uses()
                .iter()
                .position(|uses| uses.iter().all(|a| [5, neighbor].contains(&a.face)))
                .unwrap();
            let r = body
                .qualify_edge_tangency(
                    edge,
                    crate::source_seam_tangency::Limits {
                        max_sine_squared: 1e-6,
                        cells: 100000,
                        curve_spans: 100000,
                        normal_spans: 100000,
                    },
                )
                .unwrap();
            eprintln!(
                "original middle seam edge={edge} neighbor={neighbor} reason={} cells={} normals={}",
                r.reason, r.cells, r.normal_spans
            );
            let certificate = r.seam.expect("middle fillet rails need full interval G1");
            assert!(certificate.sine_squared_bounds()[1] <= 1e-6);
            eprintln!(
                "original middle seam edge={edge} sine2={:?}",
                certificate.sine_squared_bounds()
            );
            assert!(
                body.qualify_edge_tangency(
                    edge,
                    crate::source_seam_tangency::Limits {
                        max_sine_squared: 1e-6,
                        cells: 1,
                        curve_spans: 1,
                        normal_spans: 1,
                    }
                )
                .unwrap()
                .seam
                .is_none()
            );
        }
        for faces in [[0, 1], [0, 2], [10, 11], [10, 12]] {
            let edge = body
                .geometry()
                .shell()
                .uses()
                .iter()
                .position(|uses| uses.iter().all(|a| faces.contains(&a.face)))
                .unwrap();
            let r = body
                .qualify_edge_tangency(
                    edge,
                    crate::source_seam_tangency::Limits {
                        max_sine_squared: 1e-6,
                        cells: 1024,
                        curve_spans: 2048,
                        normal_spans: 2048,
                    },
                )
                .unwrap();
            eprintln!(
                "original pole seam edge={edge} faces={faces:?} reason={} cells={} uncertain={:?}",
                r.reason, r.cells, r.uncertain_canonical
            );
            assert!(
                r.seam.is_none(),
                "collapsed endpoint must not acquire a regular G1 certificate"
            );
            assert!(r.uncertain_canonical.is_some());
        }
    }
    #[test]
    fn annular_planar_flux_keeps_budget_and_curved_fallback_guards() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let geometry = crate::source_shell_geometry::qualify(shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
        assert!(
            crate::source_planar_flux::bound(&geometry, 0, 2, 0., 0.01, 1000)
                .unwrap()
                .bound
                .is_none()
        );
        assert!(
            crate::source_planar_flux::bound(&geometry, 1, 2, 0., 0.01, 1)
                .unwrap()
                .bound
                .is_none()
        );
        let zero = crate::source_planar_flux::bound(&geometry, 4, 2, 0., 0.01, 1).unwrap();
        assert_eq!(zero.cells, 0);
        let zero = zero.bound.unwrap();
        assert_eq!([zero.lo, zero.hi], [0., 0.]);
        let limited = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.25,
                tolerance_uv: 1e-8,
                cells: 1,
                spans: 1,
                domain_cells: 1,
            },
        )
        .unwrap();
        assert!(limited.body.is_none());
        assert!(limited.cells <= 1 && limited.spans <= 1);
    }
    #[test]
    fn annular_original_geometry_admits_volume_and_orientation() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let shell = prepare(&model, limits()).unwrap().shell.unwrap();
        let geometry = crate::source_shell_geometry::qualify(shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
        let r = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.25,
                tolerance_uv: 1e-8,
                cells: 100000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap();
        eprintln!(
            "annular volume {} bounds={:?} cells={} spans={} domain={} face={:?}",
            r.reason, r.signed_bounds, r.cells, r.spans, r.domain_cells, r.uncertain_face
        );
        let body = r
            .body
            .expect("embedded original annulus must admit bounded volume");
        eprintln!(
            "annular body volume {:?} reverse={}",
            body.volume(),
            body.reverse_orientation()
        );
        let independent = 7061.4573708387725_f64;
        assert!(body.volume()[0] <= independent && independent <= body.volume()[1]);
        assert!(!body.reverse_orientation());
        assert!(body.volume()[0] > 0. && body.volume()[1] - body.volume()[0] <= 0.25);
        let original = body.definition().unwrap();
        let mut saved = original.clone();
        saved["volume"] = value_codec::json!([1., 1.]);
        saved["success"] = value_codec::json!(true);
        saved["reverseOrientation"] = value_codec::json!(true);
        let replay = crate::source_body_restore::restore(
            saved,
            crate::source_body_restore::Limits {
                shell: crate::source_shell_restore::Limits {
                    regions: crate::source_region_restore::test_limits(),
                    exact_work: 100_000_000,
                    driver_cells: 100000,
                },
                embedding: geometry_limits(),
                volume: crate::source_volume::Limits {
                    axis: 2,
                    origin: 0.,
                    absolute_error: 0.25,
                    tolerance_uv: 1e-8,
                    cells: 100000,
                    spans: 100000,
                    domain_cells: 1000000,
                },
            },
        )
        .unwrap();
        let recovered = replay
            .body()
            .expect("original annular body must freshly restore all gates");
        assert_eq!(recovered.definition().unwrap(), original);
        assert_eq!(recovered.volume(), body.volume());
        assert_eq!(recovered.reverse_orientation(), body.reverse_orientation());
        assert_eq!(
            recovered.geometry().shell().vertices(),
            body.geometry().shell().vertices()
        );
        if let Some(path) = std::env::var_os("CAD_ANNULAR_ORIGINAL_FLUX_OUTPUT") {
            let faces=body.geometry().shell().regions().unwrap().iter().map(|r| {
                let loops=r.loops().iter().map(|wire|wire.iter().map(|f| {
                    let endpoints=f.endpoints().each_ref().map(|e|match e {
                        crate::source_boundary_fragment::Endpoint::Parameter(t)=>*t,
                        _=>panic!("independent fixture requires original literal endpoints"),
                    });
                    value_codec::json!({"curve":f.curve(),"parameters":endpoints})
                }).collect::<Vec<_>>()).collect::<Vec<_>>();
                value_codec::json!({"surface":r.loops()[0][0].surface(),"wholeChartMaterial":r.whole_chart_material(),"chartWinding":r.chart_winding(),"loops":loops})
            }).collect::<Vec<_>>();
            let fixture=value_codec::json!({"schema":"cad-original-annular-flux/1","faces":faces,"nativeVolumeBounds":body.volume()});
            std::fs::write(path,value_codec::to_string_pretty(&fixture).unwrap()).unwrap();
        }

    }
    #[test]
    fn annular_outer_wall_bottom_contact() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let r=crate::source_pole_planar_contact::certify(&shell,[2,4],100_000_000,100000).unwrap();
        eprintln!("wall/bottom {} work={} driver={}",r.reason,r.exact_work,r.driver_cells);

        assert!(r.certificate.is_some(),"{}",r.reason);
        assert!(crate::source_pole_planar_contact::certify(&shell,[4,2],100_000_000,100000).unwrap().certificate.is_some());
        assert!(crate::source_pole_planar_contact::certify(&shell,[2,4],1,100000).unwrap().certificate.is_none());
    }
    #[test]
    fn annular_top_and_distant_inner_wall_separation() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let r=crate::source_hull_separation::certify_shell(&shell,[1,13],100_000_000).unwrap();
        eprintln!("distant inner separation {} work={}",r.reason,r.exact_work);
        assert!(r.certificate.is_some(),"{}",r.reason);
        assert!(crate::source_hull_separation::certify_shell(&shell,[13,1],100_000_000).unwrap().certificate.is_some());
        assert!(crate::source_hull_separation::certify_shell(&shell,[1,13],1).unwrap().certificate.is_none());
    }
    #[test]
    fn annular_distant_top_material_separation() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let r=crate::source_hull_separation::certify_shell(&shell,[1,10],100_000_000).unwrap();
        eprintln!("distant material separation {} work={}",r.reason,r.exact_work);
        let c=r.certificate.expect("original material hull must separate distant transition");
        assert!(c.material_hulls()[0].is_some());
        assert!(crate::source_hull_separation::certify_shell(&shell,[10,1],100_000_000).unwrap().certificate.is_some());
        assert!(crate::source_hull_separation::certify_shell(&shell,[1,10],1).unwrap().certificate.is_none());
        assert!(crate::source_hull_separation::certify_shell(&shell,[1,6],100_000_000).unwrap().certificate.is_none());
    }
    #[test]
    fn annular_neighboring_top_material_contact() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let r=crate::source_allowed_contact::certify(&shell,[1,6],100_000_000,100000).unwrap();
        eprintln!("top contact {} work={}",r.reason,r.exact_work);
        assert!(r.certificate.is_some(),"{}",r.reason);
        assert!(crate::source_allowed_contact::certify(&shell,[6,1],100_000_000,100000).unwrap().certificate.is_some());
        assert!(crate::source_allowed_contact::certify(&shell,[1,6],1,100000).unwrap().certificate.is_none());
        assert!(crate::source_allowed_contact::certify(&shell,[1,11],100_000_000,100000).unwrap().certificate.is_none());
    }
    #[test]
    fn annular_top_and_ruled_wall_only_contact_at_owned_pole() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let report=crate::source_vertex_contact::certify(&shell,[1,2],100_000_000).unwrap();
        eprintln!("top/cylinder owned pole: {} work={}",report.reason,report.exact_work);
        let certificate=report.certificate.expect("original top/cylinder pole needs contact certificate");
        assert!(certificate.corner_image().is_some());
        assert_eq!(certificate.corner_image().unwrap().point(),certificate.point());
    }
    #[test]
    fn trimmed_plane_vertex_owns_an_exact_original_carrier_endpoint() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let regions=shell.regions().unwrap();
        let mut owned=std::collections::BTreeMap::new();
        for face in [0,6] {
            for (wire,fragments) in regions[face].loops().iter().enumerate() {
                for (edge,fragment) in fragments.iter().enumerate() {
                    for end in 0..2 {
                        let address=crate::source_shell_incidence::Address {face,wire,edge};
                        if let Some(point)=crate::source_vertex_contact::carrier_corner(&shell,address,end) {
                            if face==6 {assert!(crate::source_vertex_contact::corner(fragment,end).is_none());}
                            owned.entry(shell.vertices()[face][wire][edge][end]).or_insert_with(Vec::new).push((face,point));
                        }
                    }
                }
            }
        }
        let shared=owned.values().filter(|ends|ends.iter().any(|e|e.0==0)&&ends.iter().any(|e|e.0==6)).collect::<Vec<_>>();
        assert_eq!(shared.len(),1);
        assert!(shared[0].iter().all(|e|e.1==shared[0][0].1));
        let report=crate::source_vertex_contact::certify(&shell,[0,6],100_000_000).unwrap();
        eprintln!("trimmed planar vertex: {} work={}",report.reason,report.exact_work);
        assert!(report.certificate.is_some(),"{}",report.reason);
        assert!(report.certificate.unwrap().material_hulls()[1].is_some());
    }
    #[test]
    fn original_annular_transition_cylinder_contact_is_owned() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let proof=crate::source_ruled_projection_contact::certify(&shell,[0,2],1e-8,100_000_000,100000).unwrap();
        eprintln!("original annular ruled contact: {} work={} driver={}",proof.reason,proof.exact_work,proof.driver_cells);
        assert!(proof.certificate.is_some(),"{}",proof.reason);
        let certificate=proof.certificate.unwrap();
        assert_eq!(certificate.faces(),[0,2]);
        assert_eq!(certificate.projection().surface(),shell.regions().unwrap()[0].loops()[0][0].surface());
        assert!(!certificate.edges().is_empty());
        assert!(crate::source_ruled_projection_contact::certify(&shell,[0,2],1e-8,1,100000).unwrap().certificate.is_none());
        assert!(crate::source_ruled_projection_contact::certify(&shell,[0,3],1e-8,100_000_000,100000).unwrap().certificate.is_none());
        let reverse=crate::source_ruled_projection_contact::certify(&shell,[2,0],1e-8,100_000_000,100000).unwrap();
        assert_eq!(reverse.certificate.unwrap().faces(),[2,0]);
    }
    #[test]
    fn canonical_original_annular_transition_has_jordan_projection() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let surface=shell.regions().unwrap()[0].loops()[0][0].surface();
        let report=nurbs_core::surface_projected_jordan::certify(surface,[0,1],1e-8,100_000_000,100000).unwrap();
        eprintln!("canonical original annular Jordan: {} work={} cells={}",report.reason,report.exact_work,report.boundary_cells);
        assert!(report.certificate.is_some(),"{}",report.reason);
        assert_eq!(report.certificate.unwrap().surface(),surface);
    }
    #[test]
    fn original_annular_projection_orientation_and_wall_rank_deficiency() {
        let model = crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        for face in [0,2] {
            let surface = &model.faces[face].surface;
            let report = nurbs_core::surface_projection_jacobian::certify(surface,[0,1],100_000_000).unwrap();
            let counts = report.signs.as_ref().map(|rows| {
                let mut counts=[0usize;3];
                for s in rows.iter().flatten() { counts[match s {
                    cad_predicates::Sign::Negative=>0,
                    cad_predicates::Sign::Zero=>1,
                    cad_predicates::Sign::Positive=>2,
                }]+=1; } counts
            });
            eprintln!("original annular face {face}: {} work={} exact={:?} signs={counts:?}",report.reason,report.exact_work,report.exact_reason);
            if face==0 {
                let certificate=report.certificate.as_ref().expect("corrected original transition needs orientation proof");
                assert_eq!(certificate.orientation(),cad_predicates::Sign::Negative);
                assert_eq!(certificate.surface(),surface);
                assert_eq!(counts,Some([77,13,0]));
                assert_eq!(report.opposite_v_boundary_signs,None);
                assert!(nurbs_core::surface_projection_jacobian::certify(surface,[0,1],1).unwrap().certificate.is_none());
            } else {
                assert!(report.signs.is_some(),"{}",report.reason);
                assert!(report.signs.as_ref().unwrap().iter().flatten().all(|s| *s==cad_predicates::Sign::Zero));
                assert!(report.certificate.is_none());
            }
        }
    }
    #[test]
    fn annular_incidence_and_support_contacts_qualify_embedded_geometry() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let report = prepare(&model, limits());
        match report {
            Ok(report) => {
                eprintln!(
                    "annular source incidence: {} pair={:?}",
                    report.reason, report.uncertain_pair
                );
                assert!(report.shell.is_some(), "{}", report.reason);
                let shell = report.shell.unwrap();
                assert_eq!(shell.faces().len(), 27);
                assert_eq!(shell.poles().len(), 2);
                let contact =
                    crate::source_pole_planar_contact::certify(&shell, [0, 1], 100_000_000, 100000)
                        .unwrap();
                assert!(contact.certificate.is_some(), "{}", contact.reason);
                assert!(
                    crate::source_pole_planar_contact::certify(&shell, [0, 1], 1, 100000)
                        .unwrap()
                        .certificate
                        .is_none()
                );
                let geometry = crate::source_shell_geometry::qualify(
                    shell,
                    geometry_limits(),
                )
                .unwrap();
                eprintln!(
                    "annular source geometry: {} face={:?} pair={:?}",
                    geometry.reason, geometry.uncertain_face, geometry.next_pair
                );
                eprintln!("annular resources exact={} spans={} driver={} linear={} pairs={}",geometry.exact_work,geometry.spans,geometry.driver_cells,geometry.linear_cells,geometry.pairs);
                assert!(geometry.exact_work<=100_000_000 && geometry.driver_cells<=100000);
                assert_eq!(geometry.reason,"source-shell-embedded-geometry-qualified");
                assert_eq!(geometry.next_pair,None);
                let geometry=geometry.geometry.expect("all original annular contacts must qualify");
                assert!(geometry.contacts().all_pairs_qualified);
                assert_eq!(geometry.contacts().pairs.len(),351);
                assert_eq!(geometry.contacts().total_pairs,351);
                assert!(geometry.inverse_shear().is_none());
            }
            Err(e) => panic!("annular source support: {}: {}", e.code, e.message),
        }
    }
    #[test]
    fn source_support_claims_do_not_authorize_geometric_lifts() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let snapshot = format!("{model:?}");
        let original = prepare(&model, limits()).unwrap().shell.unwrap();
        let mut without_claims = model.clone();
        without_claims.0.bodies.clear();
        without_claims.0.shells[0].closed = false;
        without_claims.rebuild_topology_ids();
        let fresh = prepare(&without_claims, limits()).unwrap().shell.unwrap();
        assert_eq!(fresh.definition().unwrap(), original.definition().unwrap());
        assert_eq!(format!("{model:?}"), snapshot);
        let mut damaged = model.clone();
        let wire = damaged.faces[1].outer;
        damaged.0.loops[wire].coedges[0].pcurve.control_points[2][0] += 0.01;
        assert!(prepare(&damaged, limits()).is_err());
        let mut low = limits();
        low.faces = 26;
        assert!(prepare(&model, low).is_err());
    }
}
