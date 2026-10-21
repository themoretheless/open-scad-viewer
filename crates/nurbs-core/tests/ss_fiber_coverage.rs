use nurbs_core::{
    ss_intersection::{
        CoverageLimits, FiberAxis, FiberCoverageStatus, FiberLimits, FiberSources, cover_fiber_box,
        cover_fiber_surfaces,
    },
    surface::Surface,
};
fn pair(two: bool) -> (Surface, Surface) {
    let z = if two {
        [0.1875, -0.3125, 0.1875]
    } else {
        [0., 0., 1.]
    };
    let a = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: [0.2, 0.4]
            .iter()
            .map(|&x| (0..3).map(|j| vec![x, j as f64 / 2., z[j]]).collect())
            .collect(),
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let h = if two { 0. } else { 0.25 };
    let b = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., h], vec![0., 1., h]],
            vec![vec![1., 0., h], vec![1., 1., h]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    (a, b)
}
fn fiber(two: bool, free: [f64; 2]) -> nurbs_core::ss_intersection::QualifiedFiber {
    let (a, b) = pair(two);
    let sources = FiberSources::new(&a, &b).unwrap();
    let (v, t) = if two {
        ([0.2, 0.3], [0.1, 0.4])
    } else {
        ([0.4, 0.6], [0.3, 0.7])
    };
    sources
        .certify(
            [free, v, [0.1, 0.5], t],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap()
}
#[test]
fn whole_original_surfaces_have_one_fiber_and_every_complement_slab_is_excluded() {
    let f = fiber(false, [0., 1.]);
    let result = cover_fiber_surfaces(&f, CoverageLimits::default()).unwrap();
    assert_eq!(
        result.report().status,
        FiberCoverageStatus::Complete,
        "{:?}",
        result.report()
    );
    let coverage = result.into_coverage().unwrap();
    coverage.reverify().unwrap();
    assert!(coverage.covers_entire_surfaces());
    assert_eq!(coverage.domain(), [[0., 1.]; 4]);
    assert_eq!(coverage.nodes().len(), 6);
    assert_eq!(coverage.report().excluded_cells, 6);
    assert_eq!(coverage.report().rectangles, 16);
    let (a, b) = pair(false);
    for u in [0., 0.25, 0.5, 0.75, 1.] {
        let p = a.evaluate(u, 0.5).unwrap().point;
        let q = b.evaluate(0.2 + 0.2 * u, 0.5).unwrap().point;
        assert!(p.iter().zip(q).all(|(x, y)| (x - y).abs() < 1e-14));
    }
    // Independently: y equality gives t=v; z equality v²=1/4 gives
    // the sole admissible v=t=1/2, and x gives s=.2+.2u.
    assert!(
        coverage
            .fiber()
            .report()
            .support_regularity_proven
            .iter()
            .all(|v| *v)
    );
}
#[test]
fn a_second_real_branch_prevents_complete_coverage_of_only_the_first() {
    let f = fiber(true, [0., 1.]);
    let result = cover_fiber_surfaces(&f, CoverageLimits::default()).unwrap();
    assert_ne!(
        result.report().status,
        FiberCoverageStatus::Complete,
        "{:?}",
        result.report()
    );
    assert!(result.coverage().is_none());
    assert_eq!(
        result.report().status,
        FiberCoverageStatus::AdditionalRoots,
        "{:?}",
        result.report()
    );
    let additional = result.additional_fiber().unwrap();
    additional.reverify().unwrap();
    let v = additional.report().root_image.unwrap()[0];
    assert!(v[0] <= 0.75 && 0.75 <= v[1]);
    let (a, b) = pair(true);
    for v in [0.25, 0.75] {
        let p = a.evaluate(0.5, v).unwrap().point;
        let q = b.evaluate(0.3, v).unwrap().point;
        assert!(p.iter().zip(q).all(|(x, y)| (x - y).abs() < 1e-14));
    }
}
#[test]
fn missing_free_interval_or_proof_work_cannot_publish_whole_surface_authority() {
    let small = fiber(false, [0.2, 0.8]);
    let r = cover_fiber_surfaces(&small, CoverageLimits::default()).unwrap();
    assert_eq!(r.report().status, FiberCoverageStatus::Unresolved);
    assert_eq!(r.report().reason, "free-interval-incomplete");
    assert!(r.coverage().is_none());
    let r = cover_fiber_box(
        &small,
        [[0.2, 0.8], [0., 1.], [0., 1.], [0., 1.]],
        CoverageLimits::default(),
    )
    .unwrap();
    assert_eq!(r.report().status, FiberCoverageStatus::Complete);
    assert!(!r.coverage().unwrap().covers_entire_surfaces());
    r.coverage().unwrap().reverify().unwrap();
    let f = fiber(false, [0., 1.]);
    for limits in [
        CoverageLimits {
            max_cells: 5,
            ..Default::default()
        },
        CoverageLimits {
            max_rectangles: 3,
            ..Default::default()
        },
        CoverageLimits {
            max_rectangles: 5,
            ..Default::default()
        },
    ] {
        let r = cover_fiber_surfaces(&f, limits).unwrap();
        assert_eq!(r.report().status, FiberCoverageStatus::Unresolved);
        assert!(r.coverage().is_none());
    }
    assert!(
        cover_fiber_surfaces(
            &f,
            CoverageLimits {
                max_cells: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        cover_fiber_surfaces(
            &f,
            CoverageLimits {
                max_depth: 33,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn curved_empty_complement_requires_a_complete_subdivision_tree() {
    let (mut a, b) = pair(true);
    let z = [-15. / 128., 27. / 128., -59. / 128., 111. / 128.];
    a.degree_v = 3;
    a.knots_v = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    a.control_points = [0.2, 0.4]
        .iter()
        .map(|&x| (0..4).map(|j| vec![x, j as f64 / 3., z[j]]).collect())
        .collect();
    a.weights = vec![vec![1.; 4]; 2];
    // The exact dyadic z coefficients give
    // z(v)=3(v-1/2)((v-1/4)^2+1/64): one root, no hidden second branch.
    let sources = FiberSources::new(&a, &b).unwrap();
    let f = sources
        .certify(
            [[0., 1.], [0.49, 0.51], [0.1, 0.5], [0.4, 0.6]],
            FiberAxis::FirstU,
            Default::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let coarse = cover_fiber_surfaces(
        &f,
        CoverageLimits {
            max_depth: 0,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(coarse.report().status, FiberCoverageStatus::Unresolved);
    assert!(coarse.coverage().is_none());
    let r = cover_fiber_surfaces(&f, CoverageLimits::default()).unwrap();
    assert_eq!(
        r.report().status,
        FiberCoverageStatus::Complete,
        "{:?}",
        r.report()
    );
    let owner = r.into_coverage().unwrap();
    owner.reverify().unwrap();
    assert!(owner.covers_entire_surfaces());
    assert!(owner.nodes().iter().any(|n| matches!(
        n.kind(),
        Some(nurbs_core::ss_intersection::FiberCoverageKind::Split { .. })
    )));
    eprintln!(
        "complete nonlinear complement: {} nodes, {} excluded leaves, {} original rectangles",
        owner.report().cells,
        owner.report().excluded_cells,
        owner.report().rectangles
    );
}

#[test]
fn a_real_branch_on_the_natural_boundary_is_not_lost_by_closed_complement_partitioning() {
    let (mut a, b) = pair(true);
    for row in &mut a.control_points {
        for (j, z) in [0., -0.25, 0.5].into_iter().enumerate() {
            row[j][2] = z;
        }
    }
    // z=v(v-1/2) has a boundary branch v=t=0 and an interior branch v=t=1/2.
    let f = FiberSources::new(&a, &b)
        .unwrap()
        .certify(
            [[0., 1.], [0.4, 0.6], [0.1, 0.5], [0.3, 0.7]],
            FiberAxis::FirstU,
            Default::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let r = cover_fiber_surfaces(
        &f,
        CoverageLimits {
            max_cells: 256,
            max_depth: 8,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        r.report().status,
        FiberCoverageStatus::Unresolved,
        "{:?}",
        r.report()
    );
    assert!(r.coverage().is_none());
    let p = a.evaluate(0.25, 0.).unwrap().point;
    let q = b.evaluate(0.25, 0.).unwrap().point;
    assert!(p.iter().zip(q).all(|(x, y)| (x - y).abs() < 1e-14));
}

#[test]
fn two_disjoint_root_tubes_cover_both_branches_without_single_branch_authority() {
    use nurbs_core::ss_intersection::cover_fiber_atlas_surfaces;
    let first = fiber(true, [0., 1.]);
    let second = first
        .source()
        .certify(
            [[0., 1.], [0.7, 0.8], [0.1, 0.5], [0.6, 0.9]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let fibers = [first.clone(), second];
    let result = cover_fiber_atlas_surfaces(&fibers, CoverageLimits::default()).unwrap();
    assert_eq!(
        result.report().status,
        FiberCoverageStatus::Complete,
        "{:?}",
        result.report()
    );
    let owner = result.into_coverage().unwrap();
    assert_eq!(owner.fibers().count(), 2);
    assert!(owner.covers_entire_surfaces());
    owner.reverify().unwrap();
    assert!(cover_fiber_atlas_surfaces(&[], CoverageLimits::default()).is_err());
    assert!(
        cover_fiber_atlas_surfaces(&[first.clone(), first], CoverageLimits::default()).is_err()
    );
    let limited = cover_fiber_atlas_surfaces(
        &fibers,
        CoverageLimits {
            max_rectangles: 7,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(limited.report().status, FiberCoverageStatus::Unresolved);
    assert!(limited.coverage().is_none());
}

#[test]
fn atlas_rejects_mixed_sources_and_incomplete_second_free_interval() {
    use nurbs_core::ss_intersection::cover_fiber_atlas_surfaces;
    let first = fiber(true, [0., 1.]);
    let short = first
        .source()
        .certify(
            [[0.2, 0.8], [0.7, 0.8], [0.1, 0.5], [0.6, 0.9]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let result =
        cover_fiber_atlas_surfaces(&[first.clone(), short], CoverageLimits::default()).unwrap();
    assert_eq!(result.report().status, FiberCoverageStatus::Unresolved);
    assert!(result.coverage().is_none());
    assert!(
        cover_fiber_atlas_surfaces(&[first, fiber(false, [0., 1.])], CoverageLimits::default())
            .is_err()
    );
}
