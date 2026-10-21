use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, Piece, Report},
};
fn sources() -> Vec<Curve> {
    [-1., 1.]
        .into_iter()
        .map(|sign| Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0.5, 0.5],
                vec![0.5 + sign * 0.25, 0.5],
                vec![0.5, 0.5 + sign * 0.25],
                vec![0.5, 0.5],
            ],
            weights: vec![1.; 4],
            periodic: false,
        })
        .collect()
}
fn partitions(src: &[Curve]) -> Vec<Report> {
    src.iter()
        .map(|c| Report {
            pieces: vec![Piece {
                source_domain: c.domain(),
                curve: c.clone(),
                translation: [0.; 2],
                tile: [0; 2],
            }],
            candidate_hulls_contained: false,
        })
        .collect()
}
fn limits() -> p::ChartAssemblyLimits {
    p::ChartAssemblyLimits {
        max_segments: 64,
        max_links: 32,
        max_edges: 32,
        max_images: 256,
        max_cells: 30000,
        tolerance_uv: 1e-8,
    }
}
#[test]
fn unchanged_nonlinear_original_contacts_reach_weighted_chart_and_exact_port_ownership() {
    let src = sources();
    let parts = partitions(&src);
    let chart = p::assemble_exact_source_chart(
        &src,
        &parts,
        &[[0, 0]; 2],
        [[0., 1.]; 2],
        [0.53, 0.53],
        limits(),
        1_000_000,
    )
    .unwrap();
    assert!(chart.graph().is_some(), "{:?}", chart.reason());
    let contacts = chart.contact_ownership().unwrap();
    assert!(contacts.complete, "{contacts:?}");
    assert_eq!(contacts.contacts.len(), 1);
    assert_eq!(contacts.contacts[0].pairs.len(), 2);
    assert_eq!(chart.identities().len(), 2);
    chart.reverify().unwrap();
    let calibration = chart.calibration().unwrap();
    assert_eq!(calibration.source.winding, Some(1));
    assert_eq!(calibration.candidate.as_ref().unwrap().winding, Some(1));
    let old = p::assemble_coherent_weighted_chart(
        &src,
        &parts,
        [[0., 1.]; 2],
        [0.53, 0.53],
        p::ApproximateChartLimits {
            isotopy: p::IsotopyLimits {
                tolerance_uv: 0.01,
                max_deviation_cells: 40000,
                max_jet_cells: 256,
                max_pairs: 20000,
            },
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            max_checks: 10000,
            tolerance_uv: 1e-8,
        },
    )
    .unwrap();
    assert!(!old.candidate_winding_preserved);
}
#[test]
fn source_damage_tiles_homology_and_shared_work_cannot_inherit_contact_chart() {
    let src = sources();
    let parts = partitions(&src);
    let complete = p::assemble_exact_source_chart(
        &src,
        &parts,
        &[[0, 0]; 2],
        [[0., 1.]; 2],
        [0.53, 0.53],
        limits(),
        1_000_000,
    )
    .unwrap();
    let exact_work = complete.work_used();
    assert!(exact_work > 1);
    let tight = p::assemble_exact_source_chart(
        &src,
        &parts,
        &[[0, 0]; 2],
        [[0., 1.]; 2],
        [0.53, 0.53],
        limits(),
        exact_work,
    )
    .unwrap();
    tight.reverify().unwrap();
    for budget in [0, 1, exact_work - 1] {
        let c = p::assemble_exact_source_chart(
            &src,
            &parts,
            &[[0, 0]; 2],
            [[0., 1.]; 2],
            [0.53, 0.53],
            limits(),
            budget,
        )
        .unwrap();
        assert!(c.graph().is_none());
        assert!(c.work_used() <= budget);
    }
    let mut changed = parts.clone();
    changed[0].pieces[0].curve.control_points[1][0] = 0.25_f64.next_up();
    let c = p::assemble_exact_source_chart(
        &src,
        &changed,
        &[[0, 0]; 2],
        [[0., 1.]; 2],
        [0.53, 0.53],
        limits(),
        1_000_000,
    )
    .unwrap();
    assert!(c.graph().is_none());
    let mut changed = parts.clone();
    changed[0].pieces[0].tile[0] = 1;
    let c = p::assemble_exact_source_chart(
        &src,
        &changed,
        &[[0, 0]; 2],
        [[0., 1.]; 2],
        [0.53, 0.53],
        limits(),
        1_000_000,
    )
    .unwrap();
    assert_eq!(c.reason(), Some("exact-source-tile-unproven"));
    assert!(c.graph().is_none());
    let c = p::assemble_exact_source_chart(
        &src,
        &parts,
        &[[1, 0], [0, 0]],
        [[0., 1.]; 2],
        [0.53, 0.53],
        limits(),
        1_000_000,
    )
    .unwrap();
    assert_eq!(c.reason(), Some("nonzero-total-source-homology"));
    assert!(c.graph().is_none());
}
