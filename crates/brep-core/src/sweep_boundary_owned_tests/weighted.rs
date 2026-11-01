use super::*;
#[test]
fn moving_tilted_frame_and_large_nonplanar_caps_bind_the_complete_original_body() {
    let corners = vec![
        vec![-0.5, -1., 0.5],
        vec![0.5, -1., -0.5],
        vec![0.5, 1., 0.5],
        vec![-0.5, 1., -0.5],
    ];
    let profiles = (0..4)
        .map(|i| {
            Curve::from_polyline(vec![corners[i].clone(), corners[(i + 1) % 4].clone()]).unwrap()
        })
        .collect::<Vec<_>>();
    let source = cap(corners);
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 1.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0.5, 0., 1.]).unwrap();
    let normal = nurbs_core::primitives::line([1., 0., 0.], [1., 0.125, 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 5,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let rows = sweep.sections_at(5).unwrap();
    let retained = [
        cap(rows
            .iter()
            .map(|r| r[0].control_points[0].clone())
            .collect()),
        cap(rows
            .iter()
            .map(|r| r[4].control_points[0].clone())
            .collect()),
    ];
    let before = (
        profiles.clone(),
        source.surface.clone(),
        retained[0].surface.clone(),
        retained[1].surface.clone(),
    );
    let ideal = IdealLimits {
        projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        cells: 100000,
        exact_work: 1000000,
    };
    let attempt = construct_with_ideal_atlas(
        &sweep,
        &[4],
        5,
        [&source, &source],
        [&retained[0], &retained[1]],
        1e-8,
        0.05,
        limits(),
        ideal,
    )
    .unwrap();
    let body = match attempt {
        Attempt::Certified(body) => body,
        Attempt::Refused(r) => panic!(
            "tilted saddle body: {:?}; wall={} caps={} source_cells={}",
            r.reason,
            r.wall.continuous_bound,
            r.caps.len(),
            r.source_cells
        ),
    };
    let r = body.report();
    let original = r.ideal_volume.as_ref().unwrap();
    assert!(r.certified && r.continuous_bound && r.within_budget);
    assert!(
        original.embedding_certified
            && original.regularity_certified
            && original.parameter_topology_preserved
    );
    assert!(r.regularity.as_ref().unwrap().all_faces_regular && r.volume.as_ref().unwrap().proven);
    assert_eq!(body.model().bodies.len(), 1);
    assert!(r.boundary_error_upper.unwrap() <= 0.05);
    let strong = construct_with_original_smooth_volume(
        &sweep,
        &[4],
        5,
        [&source, &source],
        [&retained[0], &retained[1]],
        1e-8,
        0.05,
        limits(),
        ideal.projection,
        nurbs_core::progressive_sweep::OriginalSmoothVolumeLimits {
            order: 2,
            cells: 100000,
            exact_work: 1000000,
            products: 1000000,
        },
    )
    .unwrap();
    let strong = match strong {
        OriginalSmoothBodyAttempt::Certified(body) => body,
        OriginalSmoothBodyAttempt::Refused(r) => panic!(
            "moving tilted original smooth body: {}; original={:?}",
            r.reason, r.original
        ),
    };
    assert!(strong.retained_report().certified && strong.original().report().certified);
    assert_eq!(strong.original().report().order, 2);
    assert_eq!(strong.cap_wall_continuity(), "C0");
    assert!(
        strong
            .original()
            .report()
            .volume
            .parameter_topology_preserved
    );
    assert!(strong.retained_report().volume.as_ref().unwrap().proven);
    assert_eq!(strong.model(), body.model());
    eprintln!(
        "moving tilted original C2 body: cells={} exact={} boundary_error={:?}",
        strong.work().cells,
        strong.work().exact_work,
        strong.retained_report().boundary_error_upper
    );
    // Independent authored cap representation: exact positive affine UV
    // domains, a common homogeneous weight scale, and correspondingly authored
    // trims. The owner must establish BOTH the trim pullback and the normalized
    // ideal carrier identity. Retained Cartesian controls and original profiles
    // are fixed; the endpoint retained cap uses the same authored UV domain.
    let mut equivalent = LoftCap {
        surface: source.surface.clone(),
        trims: source.trims.clone(),
    };
    equivalent.surface.knots_u = vec![-3., -3., 5., 5.];
    equivalent.surface.knots_v = vec![7., 7., 11., 11.];
    equivalent
        .surface
        .weights
        .iter_mut()
        .flatten()
        .for_each(|w| *w *= 2.);
    for curve in equivalent.trims.iter_mut().flatten() {
        curve.knots = vec![13., 13., 19., 19.];
        curve.weights.iter_mut().for_each(|w| *w *= 4.);
        for p in &mut curve.control_points {
            p[0] = -3. + 8. * p[0];
            p[1] = 7. + 4. * p[1];
        }
    }
    let equivalent_before = (equivalent.surface.clone(), equivalent.trims.clone());
    let mut equivalent_retained = LoftCap {
        surface: retained[1].surface.clone(),
        trims: equivalent.trims.clone(),
    };
    equivalent_retained.surface.knots_u = equivalent.surface.knots_u.clone();
    equivalent_retained.surface.knots_v = equivalent.surface.knots_v.clone();
    equivalent_retained.surface.weights = equivalent.surface.weights.clone();
    let retained_before = (
        equivalent_retained.surface.clone(),
        equivalent_retained.trims.clone(),
    );
    let run = |other: &LoftCap, budget| {
        construct_with_ideal_atlas(
            &sweep,
            &[4],
            5,
            [&source, other],
            [&retained[0], &equivalent_retained],
            1e-8,
            0.05,
            limits(),
            budget,
        )
        .unwrap()
    };
    let Attempt::Certified(normalized) = run(&equivalent, ideal) else {
        panic!("exact normalized source caps lost the owned original body");
    };
    let normalized_report = normalized.report();
    let normalized_ideal = normalized_report.ideal_volume.as_ref().unwrap();
    assert!(
        normalized_report.certified
            && normalized_report.continuous_bound
            && normalized_report.within_budget
    );
    assert!(
        normalized_ideal.embedding_certified
            && normalized_ideal.regularity_certified
            && normalized_ideal.parameter_topology_preserved
    );
    assert!(
        normalized_report
            .regularity
            .as_ref()
            .unwrap()
            .all_faces_regular
    );
    assert!(normalized_report.volume.as_ref().unwrap().proven);
    assert_eq!(normalized.model().bodies.len(), 1);
    assert!(normalized_report.boundary_error_upper.unwrap() <= 0.05);
    let mut wrong_trim = LoftCap {
        surface: equivalent.surface.clone(),
        trims: equivalent.trims.clone(),
    };
    wrong_trim.trims[0][0].control_points[1][0] -= 0.125;
    let Attempt::Refused(wrong_trim) = run(&wrong_trim, ideal) else {
        panic!("carrier identity bypassed original trim-region ownership");
    };
    assert!(!wrong_trim.certified);
    assert_eq!(
        (equivalent.surface.clone(), equivalent.trims.clone()),
        equivalent_before
    );
    assert_eq!(
        (
            equivalent_retained.surface.clone(),
            equivalent_retained.trims.clone()
        ),
        retained_before
    );
    eprintln!(
        "owned affine-normalized moving saddle caps: ideal_cells={} ideal_exact={} boundary_error={:?}",
        normalized_ideal.cells, normalized_ideal.exact_work, normalized_report.boundary_error_upper
    );
    let short = construct_with_ideal_atlas(
        &sweep,
        &[4],
        5,
        [&source, &source],
        [&retained[0], &retained[1]],
        1e-8,
        0.05,
        limits(),
        IdealLimits { cells: 0, ..ideal },
    )
    .unwrap();
    let Attempt::Refused(short) = short else {
        panic!("zero original-source budget admitted");
    };
    assert!(!short.certified);
    assert_eq!(
        (
            profiles,
            source.surface,
            retained[0].surface.clone(),
            retained[1].surface.clone()
        ),
        before
    );
    eprintln!(
        "owned tilted saddle ideal body: cells={} exact={} boundary_error={:?}",
        original.cells, original.exact_work, r.boundary_error_upper
    );
}
