    use super::*;
    fn limits() -> Limits {
        Limits {
            pairs: 100,
            cells: 10000,
            domain_cells: 100000,
            cells_per_pair: 16,
            domain_cells_per_pair: 1000,
        }
    }
    #[test]
    fn resumable_contacts_match_frozen_traversal_and_cumulative_budgets() {
        let cube = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let mut separated = cube.clone();
        let original = separated.faces[0].surface.clone();
        for (i, face) in separated.faces.iter_mut().enumerate() {
            face.surface = original.clone();
            for row in &mut face.surface.control_points {
                for point in row { point[0] += 10. * i as f64; }
            }
        }
        for model in [cube, separated] {
            for sweep in [false, true] {
                for joint in [false, true] {
                    for budget in [limits(), Limits { pairs: 1, ..limits() },
                        Limits { cells: 1, ..limits() }, Limits { domain_cells: 1, ..limits() }] {
                        let expected = frozen_inspect_impl(&model, 1e-8, budget, &[], &[], sweep, joint, None).unwrap();
                        for batch in [1, 2, 64] {
                            let owned = std::sync::Arc::new(model.clone());
                            let mut work = Inspector::new(owned.clone(), 1e-8, budget, vec![], vec![], sweep, joint, None).unwrap();
                            let mut calls = 0;
                            while !work.finished() {
                                assert!(work.advance(batch).unwrap() <= batch);
                                assert!(work.report.cells <= budget.cells);
                                assert!(work.report.domain_cells <= budget.domain_cells);
                                assert!(work.report.pairs.len() <= budget.pairs);
                                calls += 1;
                                assert!(calls <= 1000);
                            }
                            assert_eq!(work.advance(batch).unwrap(), 0);
                            assert_eq!(format!("{:?}", work.finish().unwrap()), format!("{expected:?}"));
                            assert_eq!(owned.faces.len(), model.faces.len());
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn resumable_contacts_refuse_pending_reports_and_invalid_steps() {
        let model = std::sync::Arc::new(crate::cuboid([0.; 3], [1.; 3]).unwrap());
        let mut work = Inspector::new(model, 1e-8, limits(), vec![], vec![], true, false, None).unwrap();
        assert!(work.advance(0).is_err());
        assert!(work.advance(100001).is_err());
        assert_eq!(work.cursor, 0);
        assert!(work.finish().is_err());
    }
    #[test]
    fn sweep_groups_partition_all_pairs_and_preserve_bounded_pending_suffix() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let original = model.faces[0].surface.clone();
        for (i, face) in model.faces.iter_mut().enumerate() {
            face.surface = original.clone();
            for row in &mut face.surface.control_points {
                for p in row {
                    p[0] += 10. * i as f64;
                }
            }
        }
        for limits in [
            limits(),
            Limits {
                pairs: 1,
                ..limits()
            },
            Limits {
                cells: 1,
                ..limits()
            },
        ] {
            let report =
                inspect_with_sweep_certificates(&model, 1e-8, limits, &[], &[], false).unwrap();
            assert!(report.cells <= limits.cells && report.pairs.len() <= limits.pairs);
            let mut covered = report.pairs.iter().map(|p| p.faces).collect::<Vec<_>>();
            for c in &report.disjoint_groups {
                assert!(c.range[0] > c.face && c.range[1] <= model.faces.len());
                for b in c.range[0]..c.range[1] {
                    covered.push([c.face, b]);
                }
            }
            assert_eq!(covered.len(), report.pairs.len() + report.grouped_pairs);
            covered.sort_unstable();
            let expected = (0..6)
                .flat_map(|a| (a + 1..6).map(move |b| [a, b]))
                .collect::<Vec<_>>();
            assert_eq!(covered, expected[..covered.len()]);
            assert_eq!(
                report.cells,
                report.group_cells
                    + report
                        .pairs
                        .iter()
                        .map(|p| p.boundary_cells + p.source_chart_cells + p.hull_cells + p.result.as_ref().map_or(0, |r| r.cells))
                        .sum::<usize>()
            );
            if let Some(next) = report.next_pair {
                assert_eq!(next, expected[covered.len()]);
                assert!(!report.all_pairs_classified);
            } else {
                assert_eq!(covered.len(), 15);
                assert!(report.all_pairs_classified);
                assert!(report.grouped_pairs > 0);
            }
        }
    }
    #[test]
    fn cube_visits_all_pairs_without_excusing_adjacent_contacts() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let r = inspect(&m, 1e-8, limits()).unwrap();
        assert_eq!(r.total_pairs, 15);
        assert_eq!(r.pairs.len(), 15);
        assert!(r.next_pair.is_none());
        assert!(!r.all_pairs_disjoint);
        assert!(
            r.pairs
                .iter()
                .all(|p| p.result.as_ref().is_none_or(|r| r.contact.is_none()))
        );
        assert!(r.pairs.iter().any(|p| p.reason == "pair-disjoint"));
        assert_eq!(
            r.pairs
                .iter()
                .filter(|p| p.reason == "shared-boundary")
                .count(),
            12
        );
        assert!(r.all_pairs_classified);
    }
    #[test]
    fn limits_preserve_exact_unvisited_suffix() {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        // Force actual UV work: an intact cube now needs only shared-edge
        // certificates and spatial exclusions, consuming no UV cells.
        for row in &mut m.faces[0].surface.control_points {
            for p in row {
                for x in p {
                    *x += 0.01;
                }
            }
        }
        for l in [
            Limits {
                pairs: 1,
                ..limits()
            },
            Limits {
                cells: 1,
                ..limits()
            },
            Limits {
                domain_cells: 1,
                ..limits()
            },
        ] {
            let r = inspect(&m, 1e-8, l).unwrap();
            assert!(!r.all_pairs_disjoint);
            assert!(r.next_pair.is_some());
            assert!(
                r.cells <= l.cells && r.domain_cells <= l.domain_cells && r.pairs.len() <= l.pairs
            );
            let expected: Vec<_> = (0..6)
                .flat_map(|a| (a + 1..6).map(move |b| [a, b]))
                .collect();
            assert_eq!(r.next_pair, Some(expected[r.pairs.len()]));
            assert_eq!(
                r.pairs.iter().map(|p| p.faces).collect::<Vec<_>>(),
                expected[..r.pairs.len()]
            );
        }
    }
    #[test]
    fn a_defect_does_not_stop_later_pairs_from_being_reported() {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        for (f, face) in m.faces.iter_mut().enumerate() {
            face.surface.control_points = (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if f == 1 {
                                vec![u as f64, 0.5, v as f64 - 0.5]
                            } else {
                                vec![u as f64, v as f64, f as f64 * 10.]
                            }
                        })
                        .collect()
                })
                .collect();
        }
        let r = inspect(&m, 1e-8, limits()).unwrap();
        assert_eq!(r.pairs.len(), 15);
        assert!(r.next_pair.is_none());
        assert_eq!(r.pairs[0].faces, [0, 1]);
        assert_eq!(r.pairs[0].reason, "interior-contact");
        assert!(r.pairs[1..].iter().all(|p| p.reason == "pair-disjoint"));
        assert!(!r.all_pairs_disjoint);
    }
