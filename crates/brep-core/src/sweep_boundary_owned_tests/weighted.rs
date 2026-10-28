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
