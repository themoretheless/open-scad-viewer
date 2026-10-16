use super::*;
use crate as nurbs;
#[path = "../../../../tests/support/arc_bishop_source.rs"]
mod source;
#[test]
fn guided_positive_homothety_keeps_the_declared_constructor_arc_clock() {
    let path = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 5., 5., 5.],
        control_points: vec![vec![0.; 3], vec![0., 0., 0.5], vec![0., 1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut guide = path.clone();
    for point in &mut guide.control_points {
        for x in point.iter_mut() {
            *x *= 2.;
        }
        point[0] += 1.;
    }
    let profile = crate::primitives::line([0.01, 0., 0.], [0.02, 0., 0.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = Sweep::new(
        &profile,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 5,
            max_sections: 17,
            max_deviation: 1.,
        },
    )
    .unwrap()
    .with_orientation_guide(&guide)
    .unwrap();
    let report =
        super::super::authored_error::guided_section_interpolation(&sweep, 5, 100000).unwrap();
    assert_eq!(report.status, Status::Certified, "{report:?}");
    assert!(report.error_upper.is_some());
    let short =
        super::super::authored_error::guided_section_interpolation(&sweep, 5, report.cells - 1)
            .unwrap();
    assert_eq!(short.status, Status::Unresolved);
    assert!(short.error_upper.is_none());
}
#[test]
fn guided_arc_wall_error_owns_exact_phase_relative_values_and_original_chords() {
    let path = source::path(true);
    let mut guide = path.clone();
    for point in &mut guide.control_points {
        point[2] += 0.25;
    }
    let (profiles, _, _) = source::material();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 5,
            max_sections: 65,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_orientation_guide(&guide)
    .unwrap();
    let report = sweep
        .preview_at_with_error_limits(65, 100000, 1000000)
        .unwrap()
        .report;
    assert!(
        report.continuous_bound && report.continuous_error_upper.unwrap() <= 0.05,
        "{report:?}"
    );
    assert!(report.error_certificate_cells <= 100000);
    eprintln!("guided original wall error: {report:?}");
    for budget in [0, report.error_certificate_cells - 1] {
        let short = sweep
            .preview_at_with_error_limits(65, budget, 1000000)
            .unwrap()
            .report;
        assert!(
            !short.continuous_bound && short.continuous_error_upper.is_none(),
            "{short:?}"
        );
        assert!(short.error_certificate_cells <= budget);
    }
}
#[test]
#[cfg(feature = "transport")]
fn geometric_guided_arc_transport_recomputes_source_jets_and_preserves_frame_only_scope() {
    let path = source::path(true);
    let mut guide = path.clone();
    for point in &mut guide.control_points {
        point[2] += 0.25;
    }
    let profile = crate::primitives::line([0., 0., 0.01], [0., 0., 0.02]).unwrap();
    let request = value_codec::json!({
        "op":"surface_progressive_sweep_closed_guided_frame_smoothness",
        "profiles":[profile],"path":path,"orientation_guide":guide,
        "scale":constant_vector_law([1.,0.,0.]).unwrap(),
        "twist":constant_vector_law([0.;3]).unwrap(),"normal":[0.,0.,1.],
        "orientation":"rmf","spacing":"arc_length","length_tolerance":0.001,
        "length_max_cells":100000,"initial_sections":5,"max_sections":17,
        "max_deviation":1.,"order":2,"maxCells":100000,"maxExactWork":1000000
    });
    let input = request.clone();
    let proof = crate::transport::dispatch(request.clone()).unwrap();
    assert_eq!(request, input);
    assert_eq!(proof["closedSourceFrameSmoothnessCertified"], true);
    assert_eq!(proof["scope"], "closed-original-guided-frame-only");
    for key in [
        "pathSeamCertified",
        "retainedSeamsCertified",
        "profileJoinsCertified",
        "capJoinsCertified",
        "continuousBound",
        "solidCertified",
    ] {
        assert_eq!(proof[key], false, "{key}");
    }
    for (field, value) in [
        ("maxCells", 0),
        ("maxExactWork", 0),
        ("maxCells", proof["cells"].as_u64().unwrap() - 1),
        ("maxExactWork", proof["exactWork"].as_u64().unwrap() - 1),
    ] {
        let mut short = request.clone();
        short[field] = value_codec::json!(value);
        short["closedSourceFrameSmoothnessCertified"] = value_codec::json!(true);
        assert_eq!(
            crate::transport::dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],
            false
        );
    }
    let mut raw = request;
    raw["spacing"] = value_codec::json!("parameter");
    assert_eq!(
        crate::transport::dispatch(raw).unwrap()["closedSourceFrameSmoothnessCertified"],
        false
    );
}
#[test]
fn original_arc_guided_seam_keeps_geometric_jets_when_raw_speeds_differ() {
    let path = source::path(true);
    let mut guide = path.clone();
    for point in &mut guide.control_points {
        point[2] += 0.25;
    }
    let profile = crate::primitives::line([0., 0., 0.01], [0., 0., 0.02]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = Options {
        normal: [0., 0., 1.],
        orientation: Orientation::RotationMinimizing,
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
        initial_sections: 5,
        max_sections: 17,
        max_deviation: 1.,
    };
    let audit = |p: &Curve, g: &Curve, spacing, order, cells, work| {
        Sweep::new(&profile, p, &scale, &twist, Options { spacing, ..options })
            .unwrap()
            .with_orientation_guide(g)
            .unwrap()
            .certify_closed_guided_frame_smoothness(order, cells, work)
            .unwrap()
    };
    for order in [1, 2] {
        let proof = audit(&path, &guide, options.spacing, order, 100000, 1000000);
        assert!(
            proof.closed_source_frame_smoothness_certified,
            "{order}/{proof:?}"
        );
        assert!(proof.cells > 0 && proof.exact_work > 0);
        eprintln!("geometric guided source frame order={order}: {proof:?}");
        assert!(
            !audit(&path, &guide, Spacing::Parameter, order, 100000, 1000000)
                .closed_source_frame_smoothness_certified
        );
        for (cells, work) in [
            (0, 1000000),
            (proof.cells - 1, 1000000),
            (100000, 0),
            (100000, proof.exact_work - 1),
        ] {
            let short = audit(&path, &guide, options.spacing, order, cells, work);
            assert!(!short.closed_source_frame_smoothness_certified, "{short:?}");
            assert!(short.cells <= cells && short.exact_work <= work);
        }
        // Equal endpoint positions cannot conceal a geometric tangent or
        // guide second-jet mismatch, even at one stored binary64 ULP.
        let mut corner = path.clone();
        corner.control_points[1][2] = 2_f64.powi(-20);
        assert!(
            !audit(&corner, &guide, options.spacing, order, 100000, 1000000)
                .closed_source_frame_smoothness_certified
        );
        if order == 2 {
            let mut damaged = guide.clone();
            damaged.control_points[2][2] = damaged.control_points[2][2].next_up();
            assert!(
                !audit(&path, &damaged, options.spacing, order, 100000, 1000000)
                    .closed_source_frame_smoothness_certified
            );
        }
    }
}

#[test]
fn original_arc_guided_cover_owns_independent_source_domains() {
    let path = source::path(true);
    let mut guide = path.clone();
    guide.knots = [vec![13.; 10], vec![29.; 10]].concat();
    for point in &mut guide.control_points {
        point[2] += 0.25;
    }
    let profile = crate::primitives::line([0., 0., 0.01], [0., 0., 0.02]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = Sweep::new(
        &profile,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 5,
            max_sections: 17,
            max_deviation: 1.,
        },
    )
    .unwrap()
    .with_orientation_guide(&guide)
    .unwrap();
    let proof = sweep
        .certify_closed_guided_frame_smoothness(2, 100000, 1000000)
        .unwrap();
    assert!(proof.closed_source_frame_smoothness_certified, "{proof:?}");
    eprintln!("independent original guide domain: {proof:?}");
    assert!(
        !sweep
            .certify_closed_guided_frame_smoothness(2, proof.cells - 1, 1000000)
            .unwrap()
            .closed_source_frame_smoothness_certified
    );
}

#[test]
fn original_guided_arc_joint_cover_can_replace_a_failed_cartesian_product() {
    let path = source::path(false);
    let mut guide = path.clone();
    for point in &mut guide.control_points {
        point[2] += 0.25;
    }
    let profile = crate::primitives::line([0., 0., 0.01], [0., 0., 0.02]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = Sweep::new(
        &profile,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 5,
            max_sections: 17,
            max_deviation: 1.,
        },
    )
    .unwrap()
    .with_orientation_guide(&guide)
    .unwrap();
    let raw = super::super::super::endpoint_jets::certify(&path, 3, 1000000).unwrap();
    assert!(raw.certified);
    let (strong, work) =
        frame::certify_independent_guided_cover(&path, &guide, &twist, 4096).unwrap();
    assert_eq!(strong, Status::Unresolved);
    assert_eq!(work, 4096);
    let proof = sweep
        .certify_closed_guided_frame_smoothness(2, 100000, 1000000)
        .unwrap();
    assert!(proof.closed_source_frame_smoothness_certified, "{proof:?}");
    assert!(proof.cells > work && proof.exact_work > raw.exact_work);
    assert!(
        !sweep
            .certify_closed_guided_frame_smoothness(2, proof.cells - 1, 1000000)
            .unwrap()
            .closed_source_frame_smoothness_certified
    );
    eprintln!("failed Cartesian product and original station cover share work: {proof:?}");
}

#[test]
fn original_arc_guided_closed_body_has_global_embedding_and_g2_differentials() {
    let path = source::path(true);
    let mut guide = path.clone();
    for point in &mut guide.control_points {
        point[2] += 0.25;
    }
    let (profiles, cap, _) = source::material();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 5,
            max_sections: 65,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_orientation_guide(&guide)
    .unwrap();
    let boundary = sweep
        .certify_original_closed_boundary_smoothness(
            &[4],
            OriginalBoundarySmoothnessLimits {
                order: 2,
                cells: 100000,
                exact_work: 1000000,
                products: 1000000,
            },
        )
        .unwrap();
    assert!(
        boundary.differential_compatibility_certified
            && boundary.frame_certified
            && boundary.laws_certified
            && boundary.profiles_certified,
        "{boundary:?}"
    );
    let global = sweep
        .certify_ideal_volume_atlas(
            [&cap, &cap],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            1000000,
            1000000,
        )
        .unwrap();
    assert!(
        global.embedding_certified
            && global.regularity_certified
            && global.parameter_topology_preserved,
        "{global:?}"
    );
    eprintln!("original guided geometric-clock global body: {global:?}; boundary={boundary:?}");
}
