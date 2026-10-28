use super::*;
#[test]
fn different_original_cap_degrees_and_gauges_bind_the_complete_owned_saddle_body() {
    let corners = vec![
        vec![-0.5, -0.5, 0.03125],
        vec![0.5, -0.5, -0.03125],
        vec![0.5, 0.5, 0.03125],
        vec![-0.5, 0.5, -0.03125],
    ];
    let profiles = (0..4)
        .map(|i| {
            Curve::from_polyline(vec![corners[i].clone(), corners[(i + 1) % 4].clone()]).unwrap()
        })
        .collect::<Vec<_>>();
    let start = cap(corners);
    let mut source = LoftCap {
        surface: Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![17., 17., 17., 29., 29., 29.],
            knots_v: vec![-7., -7., -7., 13., 13., 13.],
            control_points: [-0.5, -0.25, 0.5]
                .iter()
                .map(|&x| {
                    [-0.5, -0.25, 0.5]
                        .iter()
                        .map(|&y| vec![x, y, x * y / 8.])
                        .collect()
                })
                .collect(),
            weights: [1., 2., 3.]
                .iter()
                .map(|a| [1., 2., 3.].iter().map(|b| a * b).collect())
                .collect(),
            periodic_u: false,
            periodic_v: false,
        },
        trims: start.trims.clone(),
    };
    for p in source
        .trims
        .iter_mut()
        .flatten()
        .flat_map(|c| &mut c.control_points)
    {
        p[0] = 17. + 12. * p[0];
        p[1] = -7. + 20. * p[1];
    }
    let mut retained = LoftCap {
        surface: source.surface.clone(),
        trims: source.trims.clone(),
    };
    for p in retained.surface.control_points.iter_mut().flatten() {
        p[2] += 1.;
    }
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 1.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = constant_vector_law([1., 0., 0.]).unwrap();
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
    let before = (
        start.surface.clone(),
        source.surface.clone(),
        source.trims.clone(),
        retained.surface.clone(),
        profiles.clone(),
    );
    let ideal = IdealLimits {
        projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        cells: 100000,
        exact_work: 1000000,
    };
    let run = |source: &LoftCap, ideal| {
        construct_with_ideal_atlas(
            &sweep,
            &[4],
            5,
            [&start, source],
            [&start, &retained],
            1e-8,
            0.05,
            limits(),
            ideal,
        )
        .unwrap()
    };
    let body = match run(&source, ideal) {
        Attempt::Certified(b) => b,
        Attempt::Refused(r) => panic!(
            "different authored tensor caps: {:?}; source_cells={} exact={} ideal={:?}",
            r.reason, r.source_cells, r.source_exact_work, r.ideal_volume
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
    assert_eq!(body.model().faces.len(), 6);
    assert!(
        r.boundary_error_upper.unwrap() < 1e-10,
        "{:?}",
        r.boundary_error_upper
    );
    for short in [
        IdealLimits { cells: 0, ..ideal },
        IdealLimits {
            exact_work: 0,
            ..ideal
        },
    ] {
        let Attempt::Refused(r) = run(&source, short) else {
            panic!("incomplete original tensor work admitted")
        };
        assert!(!r.certified);
    }
    let mut gap = LoftCap {
        surface: source.surface.clone(),
        trims: source.trims.clone(),
    };
    gap.trims[0][0].control_points[1][0] -= 0.125;
    assert!(matches!(run(&gap, ideal), Attempt::Refused(_)));
    assert_eq!(
        (
            start.surface.clone(),
            source.surface.clone(),
            source.trims.clone(),
            retained.surface.clone(),
            profiles.clone()
        ),
        before
    );
    eprintln!(
        "owned different tensor saddle caps: ideal_cells={} ideal_exact={} boundary_error={:?}",
        original.cells, original.exact_work, r.boundary_error_upper
    );
}
