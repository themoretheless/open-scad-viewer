use super::*;
use nurbs_core::progressive_sweep::OriginalSmoothVolumeLimits;
mod contact;
fn source(crease: bool) -> LoftCap {
    let corners = vec![
        vec![-3., -1., 0.],
        vec![3., -1., 0.],
        vec![3., 1., 0.],
        vec![-3., 1., 0.],
    ];
    let mut c = cap(corners);
    c.surface = Surface {
        degree_u: 3,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..7)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 - 3.,
                            j as f64 - 1.,
                            if j == 1 && i == if crease { 4 } else { 5 } {
                                1. / 128.
                            } else {
                                0.
                            },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 7],
        periodic_u: false,
        periodic_v: false,
    };
    c
}
#[test]
fn owned_c1_filled_cap_keeps_curvature_jump_and_c0_cap_wall_class_without_c2_credit() {
    let c = source(false);
    let mut end = LoftCap {
        surface: c.surface.clone(),
        trims: c.trims.clone(),
    };
    for p in end.surface.control_points.iter_mut().flatten() {
        p[2] += 2.;
    }
    let xyz = [[-3., -1., 0.], [3., -1., 0.], [3., 1., 0.], [-3., 1., 0.]];
    let profiles = (0..4)
        .map(|i| nurbs_core::primitives::line(xyz[i], xyz[(i + 1) % 4]).unwrap())
        .collect::<Vec<_>>();
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
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
        c.surface.clone(),
        c.trims.clone(),
        end.surface.clone(),
        profiles.clone(),
    );
    let p = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let requested = OriginalSmoothVolumeLimits {
        order: 1,
        cells: 100000,
        exact_work: 1000000,
        products: 1000000,
    };
    let run = |order| {
        construct_with_original_smooth_volume(
            &sweep,
            &[4],
            5,
            [&c, &c],
            [&c, &end],
            1e-8,
            0.05,
            limits(),
            p,
            OriginalSmoothVolumeLimits { order, ..requested },
        )
        .unwrap()
    };
    let body = match run(1) {
        OriginalSmoothBodyAttempt::Certified(b) => b,
        OriginalSmoothBodyAttempt::Refused(r) => panic!(
            "C1 filled original cap: {}; original={:?}",
            r.reason, r.original
        ),
    };
    assert!(body.retained_report().certified && body.original().report().certified);
    assert!(body.retained_report().volume.as_ref().unwrap().proven);
    assert_eq!(body.original().report().order, 1);
    assert_eq!(body.cap_wall_continuity(), "C0");
    assert_eq!(body.model().bodies.len(), 1);
    assert!(body.retained_report().boundary_error_upper.unwrap() < 1e-10);
    let OriginalSmoothBodyAttempt::Refused(r) = run(2) else {
        panic!("actual cap curvature jump acquired C2")
    };
    assert!(!r.certified && r.retained.certified);
    let original = r.original.as_ref().unwrap();
    assert!(
        original.volume.embedding_certified
            && !original.certified
            && !original.cap_continuity_certified
    );
    assert_eq!(r.reason, "original-filled-cap-continuity-unproved");
    assert!(r.work.cells <= requested.cells && r.work.exact_work <= requested.exact_work);
    let sharp = source(true);
    let mut sharp_end = LoftCap {
        surface: sharp.surface.clone(),
        trims: sharp.trims.clone(),
    };
    for p in sharp_end.surface.control_points.iter_mut().flatten() {
        p[2] += 2.;
    }
    let OriginalSmoothBodyAttempt::Refused(r) = construct_with_original_smooth_volume(
        &sweep,
        &[4],
        5,
        [&sharp, &sharp],
        [&sharp, &sharp_end],
        1e-8,
        0.05,
        limits(),
        p,
        requested,
    )
    .unwrap() else {
        panic!("actual C0 cap crease acquired C1")
    };
    assert!(!r.certified);
    assert_eq!(
        (
            c.surface.clone(),
            c.trims.clone(),
            end.surface.clone(),
            profiles.clone()
        ),
        before
    );
    let work = body.work();
    eprintln!(
        "owned C1 filled cap with actual curvature jump: cells={} exact={} products={} boundary_error={:?} cap_wall={}",
        work.cells,
        work.exact_work,
        work.products,
        body.retained_report().boundary_error_upper,
        body.cap_wall_continuity()
    );
}
