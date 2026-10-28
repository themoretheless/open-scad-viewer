use super::*;
fn report() -> IdealVolumeReport {
    IdealVolumeReport {
        embedding_certified: false,
        regularity_certified: false,
        parameter_topology_preserved: false,
        cells: 0,
        exact_work: 0,
        boxes: 0,
        monotonicity_lower: None,
        projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        reason: None,
    }
}
#[test]
fn nonplanar_cap_and_moving_tilted_frame_have_an_original_global_pair_certificate() {
    let cap = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-0.5, -1., 0.5], vec![-0.5, 1., -0.5]],
            vec![vec![0.5, -1., -0.5], vec![0.5, 1., 0.5]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let corners = [
        cap.control_points[0][0].clone(),
        cap.control_points[1][0].clone(),
        cap.control_points[1][1].clone(),
        cap.control_points[0][1].clone(),
    ];
    let profiles = (0..4)
        .map(|i| {
            Curve::from_polyline(vec![corners[i].clone(), corners[(i + 1) % 4].clone()]).unwrap()
        })
        .collect::<Vec<_>>();
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut binding_work = 0;
    for i in 0..4 {
        let p = Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()]).unwrap();
        let r = crate::curve_surface_agreement::verify_exact_algebraic(
            &profiles[i],
            &p,
            &cap,
            false,
            1000000 - binding_work,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r.outcome, cad_predicates::BezierIdentity::Equal));
        binding_work += r.work_used;
    }
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 1.]).unwrap();
    let axis = constant_vector_law([0.5, 0., 1.]).unwrap();
    let normal = crate::primitives::line([1., 0., 0.], [1., 0.125, 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let before = (
        cap.clone(),
        profiles.clone(),
        path.clone(),
        axis.clone(),
        normal.clone(),
    );
    let zero = constant_vector_law([0.; 3]).unwrap();
    let s = &sweep.sweeps[0];
    let smooth = s
        .certify_original_frame_smoothness(2, 100000, 1000000)
        .unwrap();
    assert!(smooth.source_frame_smoothness_certified, "{smooth:?}");

    let initial = super::super::super::super::profile_domain::original_frame(
        s,
        &zero,
        [0., 0.],
        100000,
        true,
    )
    .unwrap()
    .unwrap();
    assert_eq!(initial.status, Status::Certified);
    let basis = [
        decode(initial.transverse.unwrap()).unwrap(),
        decode(initial.binormal.unwrap()).unwrap(),
        decode(initial.longitudinal.unwrap()).unwrap(),
    ];
    let c = cell::Context {
        s,
        caps: [&cap, &cap],
        basis,
        start: [I::point(0.); 3],
        zero,
        offset: None,
        anchor: None,
        transport: None,
        arc: None,
        guide_arc: None,
        shared_guide: false,
        poses: Some(Default::default()),
    };
    let mut local_report = report();
    let Enclosed::Value(e) = c
        .enclose([[0., 1.], [0., 1.], [0., 0.125]], &mut local_report, 100000)
        .unwrap()
    else {
        panic!("original source jets missing");
    };
    // Independently retain the former local-coordinate enclosure from the
    // SAME freshly paid source pose, without using it as geometry authority.
    let jets = cap_jets(&cap, [[0., 1.]; 2], &mut local_report.cells, 100000)
        .unwrap()
        .unwrap();
    let mut old_q = jets.map(|v| local(v, basis).unwrap());
    old_q[0] = iv::sub(old_q[0], local(c.start, basis).unwrap()).unwrap();
    let cached = c
        .poses
        .as_ref()
        .unwrap()
        .borrow()
        .values()
        .next()
        .copied()
        .unwrap();
    let legacy = cached.local.enclose(old_q, [I::point(0.); 3]).unwrap();
    let columns = legacy.columns;
    assert!(regular(e.columns, report().projection).unwrap());
    let det = iv::dot_tight(columns[0], iv::cross(columns[1], columns[2]).unwrap()).unwrap();
    assert!(det.lo <= 0. && det.hi >= 0.);
    let a = scaled(columns, coordinate_scale(columns).unwrap()).unwrap();
    let r = inverse_proposal(a).unwrap();
    assert!(!frobenius_invertible(a, r).unwrap());
    assert!(certify(a, r).unwrap());
    let projection = report().projection;
    let audit = |max| {
        sweep
            .certify_ideal_volume_atlas([&cap, &cap], projection, max, 1000000)
            .unwrap()
    };
    let global = audit(100000);
    eprintln!(
        "weighted original moving-frame saddle global: {global:?}; local_cells={} binding_work={binding_work}",
        local_report.cells
    );
    assert!(
        global.embedding_certified
            && global.regularity_certified
            && global.parameter_topology_preserved,
        "{global:?}"
    );
    assert!(audit(global.cells).embedding_certified);
    for max in [0, 1, global.cells - 1] {
        let short = audit(max);
        assert!(
            !short.embedding_certified
                && !short.regularity_certified
                && !short.parameter_topology_preserved,
            "{short:?}"
        );
        assert!(short.cells <= max);
    }
    let mut bad = cap.clone();
    for j in 0..2 {
        bad.control_points[1][j][0] = bad.control_points[0][j][0];
    }
    let refused = sweep
        .certify_ideal_volume_atlas([&bad, &bad], projection, 1000, 1000000)
        .unwrap();
    assert!(
        !refused.embedding_certified
            && !refused.regularity_certified
            && !refused.parameter_topology_preserved
    );
    assert_eq!((cap, profiles, path, axis, normal), before);
}
