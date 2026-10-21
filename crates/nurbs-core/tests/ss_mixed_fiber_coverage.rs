use nurbs_core::ss_intersection::{
    self as ss, CoverageLimits, FiberAxis, FiberCoverageStatus, FiberLimits, FiberSources,
};
#[path = "support/ss_mixed_fiber.rs"]
mod fixture;
use fixture::{family, sources};
#[test]
fn essential_mixed_axis_bridge_covers_every_original_root_and_closed_product_cell() {
    let fs = family();
    let result = ss::cover_mixed_fiber_surfaces(&fs, CoverageLimits::default()).unwrap();
    assert_eq!(
        result.report().status,
        FiberCoverageStatus::Complete,
        "{:?}",
        result.report()
    );
    let c = result.into_coverage().unwrap();
    c.reverify().unwrap();
    assert!(c.covers_entire_surfaces());
    // Independent analytic world-curve oracle; completeness comes from the
    // original interval cover, not these position samples.
    for u in [0_f64, 0.125, 0.375, 0.5, 0.625, 0.875, 1.] {
        let v = (0.25 - u * u / 16.).sqrt();
        let a = c.fibers()[0].source().first().evaluate(u, v).unwrap().point;
        let b = c.fibers()[0]
            .source()
            .second()
            .evaluate(0.25 + 0.25 * u, v)
            .unwrap()
            .point;
        assert!(a.iter().zip(b).all(|(&a, b)| (a - b).abs() < 1e-12));
    }

    assert!(c.report().excluded_cells > 0);
    assert!(
        c.nodes()
            .iter()
            .any(|n| matches!(n.kind(), Some(ss::MixedCoverageKind::Fiber { index: 2 })))
    );
    // Removing the central chart leaves a genuine open interval of original roots.
    let missing = ss::cover_mixed_fiber_surfaces(
        &fs[..2],
        CoverageLimits {
            max_depth: 12,
            max_cells: 2048,
            ..CoverageLimits::default()
        },
    )
    .unwrap();
    assert!(missing.coverage().is_none());
    assert_ne!(missing.report().status, FiberCoverageStatus::Complete);
    // This is root coverage only: no physical branch or chart-transition owner.
}
#[test]
fn mixed_cover_work_and_cell_exhaustion_do_not_authorize_partial_census() {
    let fs = family();
    for limits in [
        CoverageLimits {
            max_rectangles: 1,
            ..CoverageLimits::default()
        },
        CoverageLimits {
            max_cells: 1,
            ..CoverageLimits::default()
        },
        CoverageLimits {
            max_depth: 0,
            ..CoverageLimits::default()
        },
    ] {
        let r = ss::cover_mixed_fiber_surfaces(&fs, limits).unwrap();
        assert!(r.coverage().is_none());
        assert_ne!(r.report().status, FiberCoverageStatus::Complete);
        assert!(r.report().rectangles <= limits.max_rectangles);
    }
    let mut changed = sources().second().clone();
    changed.control_points[0][0][2] = 0.3;
    let altered = FiberSources::new(sources().first(), &changed).unwrap();
    if let Some(other) = altered
        .certify(
            [[0., 0.375], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
    {
        let mixed = vec![fs[0].clone(), other];
        assert!(ss::cover_mixed_fiber_surfaces(&mixed, CoverageLimits::default()).is_err());
    } else {
        panic!("altered original must retain the proposed qualified local fiber");
    }
}

#[test]
fn reordered_overlapping_axes_and_exact_shared_work_remain_source_owned() {
    let mut fs = family();
    fs.rotate_right(1);
    let r = ss::cover_mixed_fiber_surfaces(&fs, CoverageLimits::default()).unwrap();
    assert_eq!(
        r.report().status,
        FiberCoverageStatus::Complete,
        "{:?}",
        r.report()
    );
    let work = r.report().rectangles;
    let tight = ss::cover_mixed_fiber_surfaces(
        &fs,
        CoverageLimits {
            max_rectangles: work,
            ..CoverageLimits::default()
        },
    )
    .unwrap();
    tight.coverage().unwrap().reverify().unwrap();
    let short = ss::cover_mixed_fiber_surfaces(
        &fs,
        CoverageLimits {
            max_rectangles: work - 1,
            ..CoverageLimits::default()
        },
    )
    .unwrap();
    assert!(short.coverage().is_none());
    assert!(short.report().rectangles <= work - 1);
}
#[test]
fn a_missing_distinct_original_root_branch_never_inherits_complete_mixed_cover() {
    let original = sources();
    let mut a = original.first().clone();
    let mut b = original.second().clone();
    for row in &mut a.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = [0.1875, -0.3125, 0.1875][j];
        }
    }
    for row in &mut b.control_points {
        for p in row {
            p[2] = 0.;
        }
    }
    let source = FiberSources::new(&a, &b).unwrap();
    let fibers = [([0.2, 0.3], [0.1, 0.4]), ([0.7, 0.8], [0.6, 0.9])]
        .into_iter()
        .map(|(v, t)| {
            source
                .certify(
                    [[0., 1.], v, [0.2, 0.55], t],
                    FiberAxis::FirstU,
                    FiberLimits::default(),
                )
                .unwrap()
                .into_fiber()
                .unwrap()
        })
        .collect::<Vec<_>>();
    let limits = CoverageLimits {
        max_cells: 2048,
        max_depth: 12,
        ..CoverageLimits::default()
    };
    let full = ss::cover_mixed_fiber_surfaces(&fibers, limits).unwrap();
    assert_eq!(
        full.report().status,
        FiberCoverageStatus::Complete,
        "{:?}",
        full.report()
    );
    full.coverage().unwrap().reverify().unwrap();
    let missing = ss::cover_mixed_fiber_surfaces(&fibers[..1], limits).unwrap();
    assert!(missing.coverage().is_none());
    assert_ne!(missing.report().status, FiberCoverageStatus::Complete);
}
