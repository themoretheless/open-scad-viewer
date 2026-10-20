//! Whole retained-body regression for shared-generator endpoint projection.
//! Source error, saved profile jets, and material admission are independent.
use brep_core::{
    analytic::{EndpointCapCorrection, progressive_profile_body_with_automatic_caps},
    boundary_embedding, face_contacts,
    retained_annular_cap::reference_filling,
    sweep_cap_contacts, volume_validity,
};
use nurbs_core::{
    curve::Curve,
    primitives::circle,
    progressive_sweep::{Options, Orientation, Spacing, constant_vector_law},
};

fn qualify(spacing: Spacing) {
    let path = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![0.16666666666666666, 0., 0.],
            vec![0.5, 0.16666666666666666, 0.],
            vec![0.8333333333333334, 0.6666666666666666, 0.5],
            vec![1., 1., 1.125],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let loops = vec![
        vec![circle([0.; 3], [1., 0., 0.], 0.1).unwrap()],
        vec![circle([0.; 3], [-1., 0., 0.], 0.05).unwrap()],
    ];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let before = (path.clone(), loops.clone(), scale.clone(), twist.clone());
    let options = Options {
        orientation: Orientation::CorrectedFrenet,
        normal: [0., 0., 1.],
        spacing,
        initial_sections: 3,
        max_sections: 129,
        max_deviation: 0.05,
    };
    let contacts = face_contacts::Limits {
        pairs: 10_000,
        cells: 100_000,
        domain_cells: 1_000_000,
        cells_per_pair: 1000,
        domain_cells_per_pair: 10_000,
    };
    let policy = reference_filling::Limits {
        exact_work: 1_000_000,
        products: 100_000,
        trim_pairs: 10_000,
        trim_cells: 100_000,
        trim_domain_cells: 1_000_000,
        spans: 1024,
        linear_cells: 100_000,
        contacts,
    };
    let correction = EndpointCapCorrection {
        quantum: 2_f64.powi(-40),
        tolerance: 1e-9,
        max_work: 1_000_000,
    };
    let body = progressive_profile_body_with_automatic_caps(
        &loops,
        &path,
        &scale,
        &twist,
        None,
        None,
        None,
        options,
        Some(correction),
        Some(correction),
        None,
        policy,
    )
    .unwrap();
    let boundary = body
        .complete_boundary()
        .expect("Complete original wall/cap union");
    assert!(boundary.error_upper.unwrap() <= options.max_deviation);
    assert!(
        boundary
            .filled_cap_error_upper
            .unwrap()
            .into_iter()
            .all(|v| v <= options.max_deviation)
    );
    assert!(body.evidence().cap_correction_error_upper.unwrap() <= correction.tolerance);
    assert_eq!(body.planar_cap_faces().len(), 2);

    // The former independent pole rounding left four genuinely non-G1 seams.
    // Audit every saved profile strip, including the closing quadrants.
    let smooth = body.inspect_smoothness(2_000_000).unwrap();
    let profile = &smooth.walls.profile;
    assert!(profile.extraction_complete);
    assert!(profile.unclassified_faces.is_empty() && profile.unpaired_edges.is_empty());
    assert!(profile.g1_certified && profile.g2_certified);
    assert_eq!(
        profile.edge_ids.len(),
        8 * (body
            .evidence()
            .approximation
            .levels
            .last()
            .unwrap()
            .sections
            - 1)
    );
    assert_eq!(profile.g2.seams.len(), profile.edge_ids.len());
    assert_eq!(smooth.walls.cap_continuity, "C0");
    assert!(smooth.exact_work <= 2_000_000);
    assert!(
        !body
            .inspect_smoothness(0)
            .unwrap()
            .walls
            .profile
            .g2_certified
    );

    let volume_policy = volume_validity::Limits {
        boundary: boundary_embedding::Limits {
            exact_work: policy.exact_work,
            trim_pairs: policy.trim_pairs,
            trim_cells: policy.trim_cells,
            trim_domain_cells: policy.trim_domain_cells,
            spans: policy.spans,
            contacts: face_contacts::Limits {
                pairs: 20_000,
                cells: 200_000,
                ..contacts
            },
        },
        nesting_pairs: 1000,
        nesting_cells: 100_000,
        nesting_domain_cells: 1_000_000,
        orientation_cells: 100_000,
        orientation_domain_cells: 1_000_000,
        orientation_spans: 100,
    };
    let cap_policy = sweep_cap_contacts::Budgets {
        max_walls: 1024,
        max_exact_work: 1_000_000,
        max_chart_cells: 1000,
        max_trim_pairs: 100_000,
        max_trim_cells: 100_000,
        max_trim_domain_cells: 1_000_000,
    };
    let caps = body.planar_cap_faces();
    let volume = volume_validity::inspect_sweep(
        body.model(),
        1e-8,
        volume_policy,
        100_000,
        &caps,
        cap_policy,
    )
    .unwrap();
    assert!(volume.proven && volume.boundary.proven);
    assert!(volume.boundary.intersections.faces.all_faces_injective);
    assert!(volume.boundary.intersections.pairs.all_pairs_classified);
    assert!(volume.boundary.intersections.pairs.next_pair.is_none());
    assert_eq!(
        volume.nesting.as_ref().unwrap().roles_consistent,
        Some(true)
    );
    assert!(
        volume
            .orientations
            .iter()
            .all(|s| s.outward == Some(s.expected_outward))
    );

    let mut changed = body.model().clone();
    let coordinate = changed.faces[0].surface.control_points[0][0]
        .iter_mut()
        .find(|v| **v != 0.)
        .unwrap();
    *coordinate = coordinate.next_up();
    assert!(
        !volume_validity::inspect_sweep(&changed, 1e-8, volume_policy, 100_000, &caps, cap_policy)
            .is_ok_and(|v| v.proven)
    );
    assert!(
        progressive_profile_body_with_automatic_caps(
            &loops,
            &path,
            &scale,
            &twist,
            None,
            None,
            None,
            options,
            Some(EndpointCapCorrection {
                max_work: 1,
                ..correction
            }),
            Some(correction),
            None,
            policy,
        )
        .is_err()
    );
    assert_eq!((path, loops, scale, twist), before);
    println!(
        "{spacing:?}: {} faces, bound {:?}, all {} saved profile seams G1/G2, fresh Solid and refusals",
        body.model().faces.len(),
        boundary.error_upper,
        profile.edge_ids.len()
    );
}

#[test]
fn parameter_c2_caps_preserve_saved_g2_and_material() {
    qualify(Spacing::Parameter);
}

#[test]
fn arc_c2_caps_preserve_saved_g2_and_material() {
    qualify(Spacing::ArcLength {
        tolerance: 0.001,
        max_cells: 100_000,
    });
}
