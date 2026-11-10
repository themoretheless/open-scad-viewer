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
    assert!(
        full.proven,
        "exact curved-trim compatibility certificates complete this fixture"
    );
    assert!(
        full.intersections
            .pairs
            .pairs
            .iter()
            .filter(|p| p.reason == "pair-unresolved")
            .all(|p| p.faces[0] < 4 && p.faces[1] >= 4)
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
    assert!(
        !short.proven,
        "selected caps require complete contact evidence"
    );
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
fn cube_requires_every_stage_and_preserves_input() {
    let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let before = format!("{m:?}");
    assert!(inspect(&m, 1e-8, limits()).unwrap().proven);
    for l in [
        Limits {
            exact_work: 1,
            ..limits()
        },
        Limits {
            trim_pairs: 1,
            ..limits()
        },
        Limits {
            trim_domain_cells: 1,
            ..limits()
        },
        Limits {
            spans: 1,
            ..limits()
        },
        Limits {
            contacts: face_contacts::Limits {
                pairs: 1,
                ..limits().contacts
            },
            ..limits()
        },
    ] {
        let r = inspect(&m, 1e-8, l).unwrap();
        assert!(!r.proven);
        assert_eq!(r.agreement.uses.len(), 24);
        assert_eq!(r.trim.faces.len(), 6);
        assert_eq!(r.intersections.faces.faces.len(), 6);
    }
    assert_eq!(format!("{m:?}"), before);
}
#[test]
fn tolerance_accepted_gap_is_not_an_exact_embedded_boundary() {
    let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    m.edges[0].curve.control_points[0][2] += 1e-12;
    m.validate().unwrap();
    let r = inspect(&m, 1e-8, limits()).unwrap();
    assert!(!r.agreement.all_equal);
    assert!(!r.proven);
}

#[test]
fn exact_sphere_boundary_embedding_covers_poles_equator_edges_and_vertices() {
    let model = crate::analytic::sphere(3.).unwrap();
    let before = format!("{model:?}");
    let mut l = limits();
    l.spans = 1000;
    let r = inspect(&model, 1e-8, l).unwrap();
    assert!(r.agreement.all_equal && r.agreement.all_joins_exact && r.trim.all_valid);
    assert!(r.intersections.faces.all_faces_injective);
    for pair in [[0, 2], [1, 3], [4, 6], [5, 7]] {
        assert!(
            r.hull_contacts
                .iter()
                .any(|c| c.faces == pair && c.vertex.is_some())
        );
    }
    assert!(r.proven);
    assert!(r.intersections.absence_proven);
    assert!(r.intersections.pairs.all_pairs_classified);
    assert_eq!(
        r.hull_contacts
            .iter()
            .filter(|c| c.vertex.is_some())
            .count(),
        12
    );
    let rounded = crate::analytic::sphere(2.).unwrap();
    let r = inspect(&rounded, 1e-8, l).unwrap();
    assert!(!r.proven);
    assert!(!r.agreement.all_equal);
    assert!(r.hull_contacts.is_empty());
    assert_eq!(format!("{model:?}"), before);
}
#[test]
fn trimmed_plane_hulls_require_complete_exact_boundary_prerequisites() {
    let model =
        crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
    let before = format!("{model:?}");
    let mut l = limits();
    l.spans = 4096;
    l.contacts.pairs = 400;
    l.contacts.cells = 150000;
    l.contacts.domain_cells = 1500000;
    l.contacts.cells_per_pair = 1024;
    l.contacts.domain_cells_per_pair = 100000;
    let r = inspect(&model, 1e-8, l).unwrap();
    assert!(r.agreement.all_equal && r.agreement.all_joins_exact && r.trim.all_valid);
    assert!(r.intersections.faces.all_faces_injective);
    assert!(
        r.hull_contacts
            .iter()
            .any(|c| c.faces == [0, 6] && c.vertex.is_some())
    );
    assert!(
        r.hull_contacts
            .iter()
            .any(|c| c.faces == [1, 23] && !c.edges.is_empty())
    );
    assert!(
        r.hull_contacts
            .iter()
            .any(|c| c.faces == [4, 9] && c.edges == vec![13])
    );
    assert!(
        r.hull_contacts
            .iter()
            .any(|c| c.faces == [1, 2] && c.vertex == Some(0))
    );
    assert!(
        r.intersections
            .pairs
            .pairs
            .iter()
            .any(|p| p.faces == [0, 1] && p.reason == "shared-boundary")
    );
    assert!(
        r.proven
            && r.intersections.absence_proven
            && r.intersections.pairs.all_pairs_classified
    );
    assert_eq!(
        r.hull_contacts
            .iter()
            .filter(|c| c.joined_proof.is_some())
            .count(),
        2
    );
    l.exact_work = 1;
    l.contacts.pairs = 1;
    let incomplete = inspect(&model, 1e-8, l).unwrap();
    assert!(!incomplete.agreement.all_equal && incomplete.hull_contacts.is_empty());
    assert!(!incomplete.proven);
    assert_eq!(format!("{model:?}"), before);
}
