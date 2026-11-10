use super::*;

#[test]
fn retained_caps_share_budget_and_reject_changed_source_weights() {
    use nurbs_core::curve::Curve;
    let ring = |z: f64| {
        let points = [[0., 0., z], [1., 0., z], [1., 1., z], [0., 1., z]];
        (0..4)
            .map(|i| Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            })
            .collect::<Vec<_>>()
    };
    let mut endpoints = vec![vec![ring(0.)], vec![ring(2.)]];
    let before = endpoints.clone();
    let model = crate::rational_section_loft(&endpoints).unwrap();
    let budget = Budgets {
        max_walls: 1024,
        max_exact_work: 1_000_000,
        max_chart_cells: 1000,
        max_trim_pairs: 100000,
        max_trim_cells: 100000,
        max_trim_domain_cells: 1000000,
    };
    let complete = inspect_retained_caps(&model, &endpoints, budget, 1024).unwrap();
    assert!(complete.exact, "{:?}", complete);
    assert!(complete.exact_work > 1);
    let cut = inspect_retained_caps(
        &model,
        &endpoints,
        Budgets {
            max_exact_work: 1,
            ..budget
        },
        1024,
    )
    .unwrap();
    assert!(!cut.exact);
    assert!(cut.exact_work <= 1);
    assert_eq!(endpoints, before);
    endpoints[1][0][0].weights[0] = f64::from_bits(1_f64.to_bits() + 1);
    assert!(
        !inspect_retained_caps(&model, &endpoints, budget, 1024)
            .unwrap()
            .exact
    );
}

#[test]
fn retained_oblique_plane_requires_exact_original_controls_and_shared_work() {
    let mut surface = nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 1.]],
            vec![vec![1., 0., 1.], vec![1., 1., 2.]],
        ],
        weights: vec![vec![1., 0.5], vec![2., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate().unwrap();
    let mut work = 0;
    assert!(planar_control_hull(&surface, &mut work, 1000000));
    assert!(work > 0 && work <= 1000000);
    let mut exhausted = work;
    assert!(!planar_control_hull(&surface, &mut exhausted, work));
    assert_eq!(exhausted, work);
    surface.control_points[1][1][2] = f64::from_bits(2f64.to_bits() + 1);
    assert!(!planar_control_hull(&surface, &mut 0, 1000000));
    surface.control_points[1] = surface.control_points[0].clone();
    assert!(!planar_control_hull(&surface, &mut 0, 1000000));
}

#[test]
fn corrected_spatial_cap_excludes_all_wall_interiors_with_shared_exact_work() {
    let profiles = vec![
        nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap(),
        nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.2)
            .unwrap()
            .reverse()
            .unwrap(),
    ];
    let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]];
    let mut sections =
        nurbs_core::paths::miter_sections(&profiles, &points, [1., 0., 0.], 2.).unwrap();
    let last = sections.len() - 1;
    sections[last] = nurbs_core::section_projection::project(
        &sections[last],
        1,
        [0., -0.5],
        17.5,
        2_f64.powi(-40),
        1e-9,
        1000000,
    )
    .unwrap()
    .curves
    .unwrap();
    let sections = sections
        .into_iter()
        .map(|row| row.into_iter().map(|c| vec![c]).collect())
        .collect::<Vec<_>>();
    let model = crate::rational_section_loft(&sections).unwrap();
    let budgets = Budgets {
        max_walls: 24,
        max_exact_work: 1000000,
        max_chart_cells: 10000,
        max_trim_pairs: 10000,
        max_trim_cells: 100000,
        max_trim_domain_cells: 1000000,
    };
    let report = inspect(&model, 25, &[24, 25], budgets).unwrap();
    assert!(
        report.cap_certified
            && report.planar_control_hull_certified
            && report.all_cap_wall_contacts_certified,
        "{report:?}"
    );
    assert_eq!(report.allowed_boundaries.len(), 8);
    assert_eq!(report.separated_walls.len(), 16);
    assert!(report.exact_work <= budgets.max_exact_work);
    let joint = crate::boundary_embedding::inspect_sweep(
        &model,
        1e-8,
        crate::boundary_embedding::Limits {
            exact_work: 1000000,
            trim_pairs: 10000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
            spans: 1000,
            contacts: crate::face_contacts::Limits {
                pairs: 10000,
                cells: 100000,
                domain_cells: 1000000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10000,
            },
        },
        20000,
        &[24, 25],
        budgets,
    )
    .unwrap();
    assert!(joint.agreement.all_equal && joint.intersections.faces.all_faces_injective);
    assert!(
        joint
            .cap_contacts
            .iter()
            .all(|(_, r)| r.all_cap_wall_contacts_certified)
    );
    let unresolved = joint
        .intersections
        .pairs
        .pairs
        .iter()
        .filter(|p| p.reason == "pair-unresolved")
        .map(|p| p.faces)
        .collect::<Vec<_>>();
    println!(
        "spatial remaining pairs: {unresolved:?}; boundary proven: {}",
        joint.proven
    );
    assert!(joint.proven);
    for pair in [[9, 18], [15, 20]] {
        assert!(
            crate::boundary_hull_contact::certify(&model, pair)
                .unwrap()
                .vertex
                .is_some()
        );
    }
    let volume_limits = crate::volume_validity::Limits {
        boundary: crate::boundary_embedding::Limits {
            exact_work: 1000000,
            trim_pairs: 10000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
            spans: 1000,
            contacts: crate::face_contacts::Limits {
                pairs: 10000,
                cells: 100000,
                domain_cells: 1000000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10000,
            },
        },
        nesting_pairs: 1000,
        nesting_cells: 100000,
        nesting_domain_cells: 1000000,
        orientation_cells: 100000,
        orientation_domain_cells: 1000000,
        orientation_spans: 100,
    };
    let volume = crate::volume_validity::inspect_sweep(
        &model,
        1e-8,
        volume_limits,
        20000,
        &[24, 25],
        budgets,
    )
    .unwrap();
    println!(
        "spatial volume proven: {}; orientations: {:?}",
        volume.proven,
        volume
            .orientations
            .iter()
            .map(|o| (o.shell, o.outward, o.attempts.len()))
            .collect::<Vec<_>>()
    );
    assert!(volume.proven);
    let mut inward = model.clone();
    for face in &mut inward.shells[0].faces {
        face.reversed = !face.reversed;
    }
    let wrong = crate::volume_validity::inspect_sweep(
        &inward,
        1e-8,
        volume_limits,
        20000,
        &[24, 25],
        budgets,
    )
    .unwrap();
    assert!(wrong.boundary.proven);
    assert!(!wrong.proven);
    assert_eq!(wrong.orientations[0].outward, Some(false));
    for pair in [[16, 19], [17, 18], [20, 23], [21, 22]] {
        assert!(
            joint
                .intersections
                .pairs
                .pairs
                .iter()
                .any(|p| p.faces == pair && p.boundary.is_some())
        );
    }
    for pair in [
        [16, 20],
        [16, 21],
        [16, 22],
        [16, 23],
        [18, 20],
        [18, 21],
        [18, 22],
        [18, 23],
        [21, 23],
    ] {
        assert!(
            joint
                .intersections
                .pairs
                .pairs
                .iter()
                .any(|p| p.faces == pair && p.reason == "pair-disjoint")
        );
    }
    for pair in [[9, 16], [10, 19], [13, 20], [14, 23]] {
        let certificate = crate::boundary_hull_contact::certify(&model, pair).unwrap();
        assert!(certificate.vertex.is_some());
        assert!(certificate.edges.is_empty());
    }
    let vertex = crate::boundary_hull_contact::certify(&model, [9, 16])
        .unwrap()
        .vertex
        .unwrap();
    let extra = model.faces[9]
        .surface
        .control_points
        .iter()
        .flatten()
        .find(|p| p.as_slice() != model.vertices[vertex].point)
        .unwrap()
        .clone();
    let mut overlap = model.clone();
    overlap.faces[16].surface.control_points[0][0] = extra;
    assert!(crate::boundary_hull_contact::certify(&overlap, [9, 16]).is_none());
    let limited = inspect(
        &model,
        25,
        &[24, 25],
        Budgets {
            max_exact_work: 0,
            ..budgets
        },
    )
    .unwrap();
    assert!(!limited.all_cap_wall_contacts_certified);
    let mut cap = model.faces[25].surface.clone();
    cap.control_points[0][0][1] = cap.control_points[0][0][1].next_up();
    let mut work = 0;
    assert_eq!(
        oblique_plane_contact(
            &cap,
            &model.faces[16].surface,
            Some(Boundary::VMax),
            &mut work,
            1000000
        ),
        (false, false)
    );
}
