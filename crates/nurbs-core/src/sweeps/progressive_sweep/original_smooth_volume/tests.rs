fn run<'s, 'a>(
    sweep: &'s MultiSweep<'a>,
    c: &'s Surface,
    l: OriginalSmoothVolumeLimits,
) -> OriginalSmoothVolumeAttempt<'s, 'a> {
    sweep
        .certify_original_smooth_volume_atlas([c, c], &[1], projection(), l)
        .unwrap()
}
use super::*;
fn limits(order: usize) -> OriginalSmoothVolumeLimits {
    OriginalSmoothVolumeLimits {
        order,
        cells: 1000000,
        exact_work: 1000000,
        products: 1000000,
    }
}
fn projection() -> [[f64; 3]; 3] {
    [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
}
fn cap(mode: usize) -> Surface {
    let xs = [-0.5, -0.25, 0., 0.25, 0.5];
    let mut zs = [0., 0., 0.25, 0.5, 1.];
    if mode == 1 {
        zs[4] += 0.25;
    }
    if mode == 2 {
        zs[3] += 0.0625;
        zs[4] += 0.125;
    }
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![-3., -3., -3., 1., 1., 5., 5., 5.],
        knots_v: vec![7., 7., 7., 11., 11., 11.],
        control_points: (0..5)
            .map(|i| {
                (0..3)
                    .map(|j| vec![xs[i], [-0.5, 0., 0.5][j], zs[i] / 8. + [0., 0., 0.0625][j]])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., 2., 3.]; 5],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn complete_original_c2_embedding_owns_filled_cap_knots_motion_and_one_budget() {
    let c = cap(0);
    let before = c.clone();
    let profiles = [crate::primitives::line([-0.5, -0.5, 0.], [0.5, -0.5, 0.125]).unwrap()];
    let path = crate::primitives::line([0.; 3], [0., 0., 3.]).unwrap();
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
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let OriginalSmoothVolumeAttempt::Certified(full) = run(&sweep, &c, limits(2)) else {
        panic!("complete original C2 embedding refused")
    };
    let r = full.report();
    assert_eq!(full.loop_sizes(), &[1]);
    assert!(r.certified && r.cap_continuity_certified && r.motion_continuity_certified);
    assert!(
        r.volume.embedding_certified
            && r.volume.regularity_certified
            && r.volume.parameter_topology_preserved
    );
    assert!(r.closed_boundary.is_none());
    assert!(std::ptr::eq(full.sweep(), &sweep) && std::ptr::eq(full.caps()[0], &c));
    let exact = OriginalSmoothVolumeLimits {
        cells: r.cells,
        exact_work: r.exact_work,
        ..limits(2)
    };
    assert!(matches!(
        run(&sweep, &c, exact),
        OriginalSmoothVolumeAttempt::Certified(_)
    ));
    for l in [
        OriginalSmoothVolumeLimits { cells: 0, ..exact },
        OriginalSmoothVolumeLimits {
            cells: r.cells - 1,
            ..exact
        },
        OriginalSmoothVolumeLimits {
            exact_work: 0,
            ..exact
        },
        OriginalSmoothVolumeLimits {
            exact_work: r.exact_work - 1,
            ..exact
        },
    ] {
        let OriginalSmoothVolumeAttempt::Refused(q) = run(&sweep, &c, l) else {
            panic!("incomplete smooth-volume work admitted")
        };
        assert!(!q.certified);
        assert!(q.cells <= l.cells && q.exact_work <= l.exact_work);
    }
    let jump = cap(1);
    assert!(matches!(
        run(&sweep, &jump, limits(1)),
        OriginalSmoothVolumeAttempt::Certified(_)
    ));
    let OriginalSmoothVolumeAttempt::Refused(q) = run(&sweep, &jump, limits(2)) else {
        panic!("C1 cap acquired C2")
    };
    assert!(q.volume.embedding_certified && !q.cap_continuity_certified);
    assert_eq!(q.reason, Some("original-filled-cap-continuity-unproved"));
    let crease = cap(2);
    let OriginalSmoothVolumeAttempt::Refused(q) = run(&sweep, &crease, limits(1)) else {
        panic!("authored C0 cap acquired C1")
    };
    assert!(q.volume.embedding_certified && !q.certified);
    let mut singular = c.clone();
    for p in singular.control_points.iter_mut().flatten() {
        p[0] = 0.;
        p[2] = 0.;
    }
    let OriginalSmoothVolumeAttempt::Refused(q) = run(
        &sweep,
        &singular,
        OriginalSmoothVolumeLimits {
            cells: 1000,
            ..limits(2)
        },
    ) else {
        panic!("smooth but rank-deficient source admitted")
    };
    assert!(!q.volume.regularity_certified && !q.certified);
    assert_eq!(c, before);
    eprintln!(
        "complete original C2 volume: cells={} exact={} cap_and_motion=true",
        r.cells, r.exact_work
    );
}
#[test]
fn smooth_frame_does_not_hide_an_original_translation_crease() {
    let c = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-0.5, -0.5, 0.], vec![-0.5, 0.5, 0.]],
            vec![vec![0.5, -0.5, 0.], vec![0.5, 0.5, 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let profiles = [crate::primitives::line([-0.5, -0.5, 0.], [0.5, -0.5, 0.]).unwrap()];
    let path =
        Curve::from_polyline(vec![vec![0.; 3], vec![0., 0., 1.], vec![0.25, 0., 3.]]).unwrap();
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
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    assert!(
        sweep.sweeps[0]
            .certify_original_frame_smoothness(2, 100000, 1000000)
            .unwrap()
            .source_frame_smoothness_certified
    );
    let OriginalSmoothVolumeAttempt::Refused(q) = sweep
        .certify_original_smooth_volume_atlas([&c, &c], &[1], projection(), limits(1))
        .unwrap()
    else {
        panic!("frame-only C1 credit escaped")
    };
    assert!(
        q.volume.embedding_certified
            && q.cap_continuity_certified
            && !q.motion_continuity_certified,
        "{q:?}"
    );
    assert_eq!(
        q.reason,
        Some("original-smooth-volume-translation-unproved")
    );
}
use crate as nurbs;
#[path = "../../../../tests/support/general_frame_source.rs"]
mod source;
#[test]
fn complete_closed_spatial_original_c2_embedding_includes_caps_and_periodic_motion() {
    let path = source::path();
    let (a, n, twist) = source::laws();
    let (profiles, mut c, _) = source::material();
    c.control_points.insert(
        1,
        (0..2)
            .map(|j| {
                (0..3)
                    .map(|k| (c.control_points[0][j][k] + c.control_points[1][j][k]) * 0.5)
                    .collect()
            })
            .collect(),
    );
    c.weights = vec![vec![1.; 2]; 3];
    c.knots_u = vec![-3., -3., 1., 5., 5.];
    let before = c.clone();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::Fixed,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 33,
            max_sections: 129,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&a, &n)
    .unwrap();
    let OriginalSmoothVolumeAttempt::Certified(body) = sweep
        .certify_original_smooth_volume_atlas([&c, &c], &[4], projection(), limits(2))
        .unwrap()
    else {
        panic!("closed original C2 embedding refused")
    };
    let r = body.report();
    assert!(
        r.certified
            && r.cap_continuity_certified
            && r.motion_continuity_certified
            && r.volume.embedding_certified
    );
    assert!(
        r.closed_boundary
            .as_ref()
            .unwrap()
            .differential_compatibility_certified
            && r.closed_boundary.as_ref().unwrap().order == 2
    );
    let OriginalSmoothVolumeAttempt::Refused(short) = sweep
        .certify_original_smooth_volume_atlas(
            [&c, &c],
            &[4],
            projection(),
            OriginalSmoothVolumeLimits {
                products: 0,
                ..limits(2)
            },
        )
        .unwrap()
    else {
        panic!("closed profile product exhaustion admitted");
    };
    assert!(!short.certified && short.products == 0 && short.volume.embedding_certified);
    assert_eq!(short.reason, Some("original-profile-extraction-work-limit"));
    assert_eq!(c, before);
    eprintln!(
        "complete closed spatial original C2 volume: cells={} exact={} products={}",
        r.cells, r.exact_work, r.products
    );
}
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-0.5, -0.5, 0.], vec![-0.5, 0.5, 0.]],
            vec![vec![0.5, -0.5, 0.], vec![0.5, 0.5, 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn smooth_frame_does_not_hide_an_original_scale_crease() {
    let c = plane();
    let profiles = [crate::primitives::line([-0.5, -0.5, 0.], [0.5, -0.5, 0.]).unwrap()];
    let path = crate::primitives::line([0.; 3], [0., 0., 3.]).unwrap();
    let scale =
        Curve::from_polyline(vec![vec![1., 0., 0.], vec![1.5, 0., 0.], vec![1., 0., 0.]]).unwrap();
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
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let OriginalSmoothVolumeAttempt::Refused(q) = sweep
        .certify_original_smooth_volume_atlas([&c, &c], &[1], projection(), limits(1))
        .unwrap()
    else {
        panic!("scale crease acquired C1")
    };
    assert!(
        q.volume.embedding_certified && q.cap_continuity_certified && !q.certified,
        "{q:?}"
    );
    assert_eq!(
        q.reason,
        Some("original-smooth-volume-law-continuity-unproved")
    );
}
#[test]
fn positive_arc_clock_proves_complete_c2_volume_despite_a_raw_speed_jump() {
    let c = plane();
    let profiles = [crate::primitives::line([-0.5, -0.5, 0.], [0.5, -0.5, 0.]).unwrap()];
    let path = Curve::from_polyline(vec![vec![0.; 3], vec![0., 0., 1.], vec![0., 0., 3.]]).unwrap();
    let before = path.clone();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = constant_vector_law([1., 0., 0.]).unwrap();
    assert!(
        !super::super::endpoint_jets::certify_knots(&path, 1, 1000000)
            .unwrap()
            .certified
    );
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let body = match sweep
        .certify_original_smooth_volume_atlas([&c, &c], &[1], projection(), limits(2))
        .unwrap()
    {
        OriginalSmoothVolumeAttempt::Certified(b) => b,
        OriginalSmoothVolumeAttempt::Refused(r) => panic!("geometric clock original C2: {r:?}"),
    };
    assert!(
        body.report().volume.embedding_certified
            && body.report().cap_continuity_certified
            && body.report().motion_continuity_certified
    );
    assert_eq!(path, before);
    eprintln!(
        "complete original geometric-clock C2 volume: cells={} exact={}",
        body.report().cells,
        body.report().exact_work
    );
}
