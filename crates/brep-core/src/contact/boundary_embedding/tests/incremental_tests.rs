use super::*;
fn limits() -> Limits {
    Limits {
        exact_work: 1000000,
        trim_pairs: 1000,
        trim_cells: 10000,
        trim_domain_cells: 100000,
        spans: 100,
        contacts: face_contacts::Limits {
            pairs: 100,
            cells: 10000,
            domain_cells: 100000,
            cells_per_pair: 16,
            domain_cells_per_pair: 1000,
        },
    }
}
fn report_identity(r: &Report) -> String {
    format!(
        "{:?}",
        (
            r.proven,
            &r.hull_contacts,
            (
                r.agreement.all_equal,
                &r.agreement.joins,
                r.agreement.all_joins_exact,
                r.agreement.work,
                r.agreement
                    .uses
                    .iter()
                    .map(|u| (
                        u.face,
                        u.wire,
                        u.coedge,
                        u.edge,
                        u.decision.as_ref().map(|d| (&d.outcome, d.work_used))
                    ))
                    .collect::<Vec<_>>()
            ),
            (
                r.trim
                    .faces
                    .iter()
                    .map(|face| face.as_ref().map(|f| (
                        f.valid,
                        f.reason,
                        &f.winding,
                        f.pairs,
                        f.cells,
                        f.domain_cells,
                        f.problem_loops,
                        f.loops
                            .iter()
                            .map(|l| (
                                l.proven_simple,
                                l.exact_joins,
                                &l.injective,
                                l.pairs
                                    .iter()
                                    .map(|p| (p.curves, p.proven))
                                    .collect::<Vec<_>>(),
                                l.total_pairs,
                                l.cells
                            ))
                            .collect::<Vec<_>>()
                    )))
                    .collect::<Vec<_>>(),
                r.trim.all_valid,
                r.trim.pairs,
                r.trim.cells,
                r.trim.domain_cells
            ),
            &r.intersections,
            &r.cap_contacts
        )
    )
}
#[test]
fn resumable_boundary_preserves_joint_prerequisites_and_all_stage_reports() {
    let cube = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut gap = cube.clone();
    gap.edges[0].curve.control_points[0][2] += 1e-12;
    let cap_budget = crate::sweep_cap_contacts::Budgets {
        max_walls: 1024,
        max_exact_work: 1000000,
        max_chart_cells: 1000,
        max_trim_pairs: 100000,
        max_trim_cells: 100000,
        max_trim_domain_cells: 1000000,
    };
    for model in [cube, gap] {
        for sweep in [false, true] {
            for caps in [None, Some((vec![0, 1], cap_budget))] {
                if !sweep && caps.is_some() {
                    continue;
                }
                for limits in [
                    limits(),
                    Limits {
                        spans: 1,
                        ..limits()
                    },
                    Limits {
                        contacts: face_contacts::Limits {
                            cells: 1,
                            ..limits().contacts
                        },
                        ..limits()
                    },
                ] {
                    let expected = frozen_inspect_impl(
                        &model,
                        1e-8,
                        limits,
                        10000,
                        caps.as_ref().map(|(c, b)| (c.as_slice(), *b)),
                        &[],
                        sweep,
                    )
                    .unwrap();
                    for batch in [1, 2, 64] {
                        let mut work = Inspector::new(
                            std::sync::Arc::new(model.clone()),
                            1e-8,
                            limits,
                            10000,
                            caps.clone(),
                            vec![],
                            sweep,
                        )
                        .unwrap();
                        let mut calls = 0;
                        while !work.finished() {
                            assert!(work.advance(batch).unwrap() <= batch);
                            calls += 1;
                            assert!(calls < 1000);
                        }
                        assert_eq!(work.advance(batch).unwrap(), 0);
                        assert_eq!(
                            report_identity(&work.finish().unwrap()),
                            report_identity(&expected)
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn resumable_boundary_refuses_unfinished_result() {
    let mut work = Inspector::new(
        std::sync::Arc::new(crate::cuboid([0.; 3], [1.; 3]).unwrap()),
        1e-8,
        limits(),
        10000,
        None,
        vec![],
        true,
    )
    .unwrap();
    assert!(work.advance(0).is_err());
    assert!(!work.finished());
    assert!(work.finish().is_err());
}
#[test]
fn exact_quadrant_sweep_admits_contacts_only_after_joint_prerequisites() {
    let section = |z| {
        vec![
            [
                [[0.5, 0.], [0.5, 0.5], [0., 0.5]],
                [[0., 0.5], [-0.5, 0.5], [-0.5, 0.]],
                [[-0.5, 0.], [-0.5, -0.5], [0., -0.5]],
                [[0., -0.5], [0.5, -0.5], [0.5, 0.]],
            ]
            .into_iter()
            .map(|p| nurbs_core::curve::Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p.into_iter().map(|xy| vec![xy[0], xy[1], z]).collect(),
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            })
            .collect(),
        ]
    };
    let model = crate::rational_section_loft(&[section(0.), section(5.)]).unwrap();
    let mut l = limits();
    l.contacts.pairs = 1;
    let report = inspect_with_linear(&model, 1e-8, l, 10000).unwrap();
    assert!(report.agreement.all_equal && report.agreement.all_joins_exact);
    assert!(report.trim.all_valid);
    assert!(report.intersections.faces.all_faces_injective);
    assert!(!report.hull_contacts.is_empty());
    assert!(report.intersections.pairs.pairs[0].boundary.is_some());
    assert!(report.intersections.pairs.next_pair.is_some());
    assert!(
        !report.proven,
        "one classified pair cannot certify the body"
    );
    let full = inspect_with_linear(&model, 1e-8, limits(), 10000).unwrap();
    assert!(full.intersections.pairs.next_pair.is_none());
    assert_eq!(full.intersections.pairs.pairs.len(), 15);
    // Exact hull contacts now cover these caps without an optional sweep
    // certificate. Keep the earlier one-pair budget refusal above: a
    // successful contact is still insufficient without complete coverage.
    assert!(full.proven);
    assert!(
        full.intersections
            .pairs
            .pairs
            .iter()
            .all(|p| p.reason != "pair-unresolved")
    );
    for pair in &full.intersections.pairs.pairs {
        println!(
            "retained pair {:?}: {} boundary={}",
            pair.faces,
            pair.reason,
            pair.boundary.is_some()
        );
    }
    println!("retained embedded boundary proven={}", full.proven);
    let cap_budget = crate::sweep_cap_contacts::Budgets {
        max_walls: 1024,
        max_exact_work: 1000000,
        max_chart_cells: 1000,
        max_trim_pairs: 100000,
        max_trim_cells: 100000,
        max_trim_domain_cells: 1000000,
    };
    let joined = inspect_sweep(&model, 1e-8, limits(), 10000, &[4, 5], cap_budget).unwrap();
    assert!(
        joined.proven,
        "joint cap contacts must complete this exact fixture"
    );
    assert_eq!(joined.cap_contacts.len(), 2);
    assert_eq!(
        joined
            .intersections
            .pairs
            .pairs
            .iter()
            .filter(|p| matches!(
                p.boundary,
                Some(face_contacts::SharedBoundary::SweepCap { .. })
            ))
            .count(),
        8
    );
    let short = inspect_sweep(
        &model,
        1e-8,
        limits(),
        10000,
        &[4, 5],
        crate::sweep_cap_contacts::Budgets {
            max_exact_work: 0,
            ..cap_budget
        },
    )
    .unwrap();
    // Independent hull proofs remain visible, but cannot replace the
    // explicitly selected cap evidence or manufacture a cap receipt.
    assert!(!short.proven);
    assert!(full.proven);
    assert!(short.intersections.pairs.next_pair.is_none());
    assert!(short.cap_contacts.iter().all(|(_, r)| !r.cap_certified));
    assert!(!short.intersections.pairs.pairs.iter().any(|p| matches!(
        p.boundary,
        Some(face_contacts::SharedBoundary::SweepCap { .. })
    )));
}
#[test]
fn oblique_chart_proof_does_not_bypass_exact_boundary_prerequisites() {
    let section = |z| {
        vec![vec![
            nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
        ]]
    };
    let model = crate::rational_section_loft(&[section(0.), section(5.)]).unwrap();
    let mut l = limits();
    l.exact_work = 1;
    l.contacts.pairs = 1;
    let report = inspect_with_linear(&model, 1e-8, l, 10000).unwrap();
    assert!(report.intersections.faces.all_faces_injective);
    assert!(report.intersections.faces.linear_cells > 0);
    assert!(!report.agreement.all_equal);
    assert!(report.hull_contacts.is_empty());
    assert!(!report.proven);
    assert!(report.intersections.pairs.next_pair.is_some());
}
#[test]
fn cap_cover_queries_share_the_existing_contact_budget() {
    let model=crate::cuboid([0.;3],[1.;3]).unwrap();
    let budget=crate::sweep_cap_contacts::Budgets {max_walls:1024,max_exact_work:1000000,
        max_chart_cells:1000,max_trim_pairs:100000,max_trim_cells:100000,max_trim_domain_cells:1000000};
    let full=inspect_sweep(&model,1e-8,limits(),10000,&[0,1],budget).unwrap();
    assert!(full.proven);
    let (_,preparation)=crate::face_contact_groups::Cover::prepare(&model,10000);
    assert!(full.intersections.pairs.group_cells>=preparation);
    for cells in [1,preparation-1,preparation,preparation+1] {
        let restricted=inspect_sweep(&model,1e-8,Limits {contacts:face_contacts::Limits {cells,..limits().contacts},..limits()},10000,&[0,1],budget).unwrap();
        assert!(!restricted.proven);
        assert!(restricted.intersections.pairs.cells<=cells);
        assert!(restricted.intersections.pairs.group_cells<=cells);
    }
    let mut gap=model.clone();
    gap.edges[0].curve.control_points[0][2]+=1e-12;
    let refused=inspect_sweep(&gap,1e-8,limits(),10000,&[0,1],budget).unwrap();
    assert!(!refused.proven);
    assert!(!refused.agreement.all_equal);
}
#[test]
fn cube_requires_every_stage_and_preserves_input() {
    let m = crate::cuboid([0.;3], [1.;3]).unwrap();
    let before = format!("{m:?}");
    assert!(inspect(&m,1e-8,limits()).unwrap().proven);
    for l in [Limits { exact_work: 1, ..limits() },
        Limits { trim_pairs: 1, ..limits() }, Limits { trim_domain_cells: 1, ..limits() },
        Limits { spans: 1, ..limits() },
        Limits { contacts: face_contacts::Limits { pairs: 1, ..limits().contacts }, ..limits() }] {
        let r = inspect(&m,1e-8,l).unwrap();
        assert!(!r.proven);
        assert_eq!(r.agreement.uses.len(),24);
        assert_eq!(r.trim.faces.len(),6);
        assert_eq!(r.intersections.faces.faces.len(),6);
    }
    assert_eq!(format!("{m:?}"),before);
}
#[test]
fn tolerance_accepted_gap_is_not_an_exact_embedded_boundary() {
    let mut m = crate::cuboid([0.;3], [1.;3]).unwrap();
    m.edges[0].curve.control_points[0][2] += 1e-12;
    m.validate().unwrap();
    let r = inspect(&m,1e-8,limits()).unwrap();
    assert!(!r.agreement.all_equal);
    assert!(!r.proven);
}
