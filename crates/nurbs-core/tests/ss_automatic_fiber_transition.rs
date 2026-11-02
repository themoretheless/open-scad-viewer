use nurbs_core::ss_intersection::{self as ss, FiberAxis, FiberLimits, FiberSources};
#[path = "support/ss_mixed_fiber.rs"]
mod fixture;
fn charts() -> (ss::QualifiedFiber, ss::QualifiedFiber) {
    let s = fixture::sources();
    let first = s
        .certify(
            [[0.5, 0.75], [0.4, 0.6], [0.3, 0.5], [0.4, 0.6]],
            FiberAxis::FirstU,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let second = s
        .certify(
            [
                [0.5, 0.8],
                [0.465, 0.48],
                [0.35, 0.45],
                [0.46875, 0.4765625],
            ],
            FiberAxis::SecondV,
            FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    (first, second)
}
#[test]
fn original_nonlinear_reverse_transition_is_found_without_a_bridge_box() {
    let (a, b) = charts();
    let r = ss::discover_fiber_transition(&a, &b, Default::default()).unwrap();
    let found = r.transition().expect(r.reason());
    assert!(found.attempts().len() > 1);
    assert_eq!(
        found.rectangles(),
        found.attempts().iter().map(|a| a.rectangles).sum::<usize>()
    );
    assert_eq!(
        found.proof().orientation(),
        ss::TransitionOrientation::Reverse
    );
    assert_eq!(found.proof().first().domain(), a.domain());
    assert_eq!(found.proof().second().domain(), b.domain());
    for (i, u) in found.proof().bridge().domain()[0].into_iter().enumerate() {
        let exact = (0.25 - u * u / 16.).sqrt();
        let image = found.proof().endpoint_images()[i];
        assert!(image[0] <= exact && exact <= image[1]);
    }
    found.reverify().unwrap();
}
#[test]
fn failed_proposals_and_low_work_never_grant_overlap_or_separation() {
    let (a, b) = charts();
    for l in [
        ss::FiberTransitionDiscoveryLimits {
            max_rectangles: 1,
            ..Default::default()
        },
        ss::FiberTransitionDiscoveryLimits {
            max_proposals: 1,
            ..Default::default()
        },
        ss::FiberTransitionDiscoveryLimits {
            max_depth: 0,
            ..Default::default()
        },
    ] {
        let r = ss::discover_fiber_transition(&a, &b, l).unwrap();
        assert!(r.transition().is_none());
        assert!(r.rectangles() <= l.max_rectangles);
        assert!(r.attempts().len() <= l.max_proposals);
    }
    let family = fixture::family();
    let r = ss::discover_fiber_transition(&family[0], &family[1], Default::default()).unwrap();
    assert!(r.transition().is_none());
    assert_eq!(r.reason(), "no-open-chart-box");
    assert!(
        ss::discover_fiber_transition(
            &a,
            &b,
            ss::FiberTransitionDiscoveryLimits {
                max_rectangles: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn complete_mixed_parameter_components_no_longer_require_manual_bridge_specs() {
    let cover = ss::cover_mixed_fiber_surfaces(&fixture::family(), Default::default())
        .unwrap()
        .into_coverage()
        .unwrap();
    let r = ss::connect_mixed_fiber_cover_automatic(&cover, Default::default()).unwrap();
    let proof = r.components().expect(r.reason());
    assert_eq!(proof.proof().components(), &[vec![0, 1, 2]]);
    assert_eq!(proof.proof().transitions().len(), 2);
    assert_eq!(proof.searches().len(), 2);
    assert!(proof.searches().iter().all(|s| !s.attempts.is_empty()));
    assert_eq!(
        proof.rectangles(),
        proof.searches().iter().map(|s| s.rectangles).sum::<usize>() + proof.proof().rectangles()
    );
    proof.reverify().unwrap();
    for l in [
        ss::MixedComponentDiscoveryLimits {
            max_pairs: 1,
            ..Default::default()
        },
        ss::MixedComponentDiscoveryLimits {
            max_bridges: 1,
            ..Default::default()
        },
        ss::MixedComponentDiscoveryLimits {
            max_rectangles: 1,
            ..Default::default()
        },
    ] {
        let short = ss::connect_mixed_fiber_cover_automatic(&cover, l).unwrap();
        assert!(short.components().is_none());
        assert!(short.rectangles() <= l.max_rectangles);
    }
}
#[test]
fn distinct_original_components_are_separated_without_fabricated_transition_searches() {
    let source = fixture::sources();
    let mut a = source.first().clone();
    let mut b = source.second().clone();
    for row in &mut a.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = [0.1875, -0.3125, 0.1875][j];
        }
    }
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 0.;
    }
    let source = FiberSources::new(&a, &b).unwrap();
    let fs = [([0.2, 0.3], [0.1, 0.4]), ([0.7, 0.8], [0.6, 0.9])]
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
    let cover = ss::cover_mixed_fiber_surfaces(&fs, Default::default())
        .unwrap()
        .into_coverage()
        .unwrap();
    let r = ss::connect_mixed_fiber_cover_automatic(
        &cover,
        ss::MixedComponentDiscoveryLimits {
            max_pairs: 0,
            max_bridges: 0,
            ..Default::default()
        },
    )
    .unwrap();
    let proof = r.components().expect(r.reason());
    assert_eq!(proof.proof().components(), &[vec![0], vec![1]]);
    assert!(proof.searches().is_empty());
    assert_eq!(proof.proof().separations().len(), 1);
    proof.reverify().unwrap();
}
#[test]
fn different_ordered_original_surfaces_cannot_share_a_discovered_transition() {
    let (a, b) = charts();
    let source = a.source();
    let mut changed = source.first().clone();
    for p in changed.control_points.iter_mut().flatten() {
        p[2] += 0.001;
    }
    let source = FiberSources::new(&changed, source.second()).unwrap();
    let other = source
        .certify(b.domain(), b.free_axis(), FiberLimits::default())
        .unwrap()
        .into_fiber()
        .unwrap();
    assert!(ss::discover_fiber_transition(&a, &other, Default::default()).is_err());
}

#[test]
fn a_closed_original_circle_has_complete_coverage_and_automatic_mixed_transitions() {
    use nurbs_core::surface::Surface;
    let a = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let mut b = a.clone();
    b.degree_u = 2;
    b.degree_v = 2;
    b.knots_u = vec![0., 0., 0., 1., 1., 1.];
    b.knots_v = b.knots_u.clone();
    let q = [0.25, -0.25, 0.25];
    b.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| vec![i as f64 / 2., j as f64 / 2., q[i] + q[j] - 0.0625])
                .collect()
        })
        .collect();
    b.weights = vec![vec![1.; 3]; 3];
    let sources = FiberSources::new(&a, &b).unwrap();
    let mut charts = vec![];
    for i in 0..32 {
        let angle = i as f64 * std::f64::consts::TAU / 32.;
        let half = std::f64::consts::PI / 24.;
        let free = if angle.sin().abs() >= angle.cos().abs() {
            0
        } else {
            1
        };
        let points = [angle - half, angle, angle + half]
            .map(|t| [0.5 + 0.25 * t.cos(), 0.5 + 0.25 * t.sin()]);
        let bounds: [[f64; 2]; 2] = std::array::from_fn(|k| {
            [
                points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min),
                points
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max),
            ]
        });
        let mut domain: [[f64; 2]; 4] =
            std::array::from_fn(|k| [bounds[k % 2][0] - 0.05, bounds[k % 2][1] + 0.05]);
        domain[free] = bounds[free];
        domain[2 + free] = [bounds[free][0] - 0.01, bounds[free][1] + 0.01];
        let axis = if free == 0 {
            FiberAxis::FirstU
        } else {
            FiberAxis::FirstV
        };
        let proof = sources.certify_contracted(domain, axis, 100000).unwrap();
        let reason = proof.reason();
        charts.push(
            proof
                .into_fiber()
                .unwrap_or_else(|| panic!("chart {i}: {reason}")),
        );
    }
    assert!(charts.iter().any(|f| f.free_axis() == FiberAxis::FirstU));
    assert!(charts.iter().any(|f| f.free_axis() == FiberAxis::FirstV));
    let cover = ss::cover_mixed_fiber_surfaces(&charts, Default::default()).unwrap();
    let report = cover.report().clone();
    let cover = cover
        .into_coverage()
        .unwrap_or_else(|| panic!("circle coverage: {report:?}"));
    assert!(cover.covers_entire_surfaces());
    let result = ss::connect_mixed_fiber_cover_automatic(&cover, Default::default()).unwrap();
    let proof = result.components().unwrap_or_else(|| {
        let searches = result
            .searches()
            .iter()
            .map(|s| {
                (
                    s.charts,
                    &s.reason,
                    s.rectangles,
                    s.attempts.len(),
                    s.attempts.last().map(|a| &a.reason),
                )
            })
            .collect::<Vec<_>>();
        if let Some(s) = result.searches().iter().find(|s| s.charts == [4, 5]) {
            eprintln!("SWITCH: {:?}", &s.attempts[..s.attempts.len().min(10)]);
        }
        panic!(
            "{} work={} searches={searches:?}",
            result.reason(),
            result.rectangles()
        );
    });
    assert_eq!(proof.proof().components().len(), 1);
    assert_eq!(proof.proof().components()[0].len(), 32);
    assert!(
        proof
            .proof()
            .transitions()
            .iter()
            .any(|t| t.first().free_axis() != t.second().free_axis())
    );
    proof.reverify().unwrap();
    println!(
        "closed-circle: charts={} coverage_cells={} coverage_rectangles={} transitions={} total_rectangles={}",
        charts.len(),
        report.cells,
        report.rectangles,
        proof.proof().transitions().len(),
        proof.rectangles()
    );
    // A connected parameter root cover with cyclic geometric input is not
    // upgraded here to an embedded world loop or a generated BRep trim.
}

#[path = "support/ss_discovered_sources.rs"]
mod discovered_fixture;
#[test]
fn automatically_discovered_original_atlas_feeds_the_mixed_component_owner() {
    let sources = discovered_fixture::sources(&[0.1875, -0.3125, 0.1875], 0.);
    let found = ss::discover_fiber_surfaces(&sources, Default::default()).unwrap();
    let report = found.report().clone();
    let atlas = found.into_atlas().unwrap_or_else(|| panic!("{report:?}"));
    assert!(atlas.covers_entire_surfaces());
    atlas.reverify().unwrap();
    let covered =
        ss::cover_mixed_fiber_box(atlas.fibers(), atlas.domain(), Default::default()).unwrap();
    let report = covered.report().clone();
    let covered = covered
        .into_coverage()
        .unwrap_or_else(|| panic!("{report:?}"));
    let linked = ss::connect_mixed_fiber_cover_automatic(&covered, Default::default()).unwrap();
    let proof = linked.components().expect(linked.reason());
    assert_eq!(proof.proof().components().len(), 2);
    proof.reverify().unwrap();
}

#[test]
fn singleton_root_connections_require_actual_source_proofs_and_are_not_oriented_transitions() {
    let charts = fixture::family();
    let cover = ss::cover_mixed_fiber_surfaces(&charts, Default::default())
        .unwrap()
        .into_coverage()
        .unwrap();
    let specs = [(0, 0.3), (1, 0.7)].map(|(i, u)| {
        let a = charts[i].domain();
        let b = charts[2].domain();
        let mut domain = std::array::from_fn(|k| [a[k][0].max(b[k][0]), a[k][1].min(b[k][1])]);
        domain[0] = [u; 2];
        ss::FiberBridgeSpec {
            charts: [i, 2],
            domain,
        }
    });
    let linked = ss::connect_mixed_fiber_cover(&cover, &specs, 100000).unwrap();
    let proof = linked.components().expect(linked.reason());
    assert_eq!(proof.components(), &[vec![0, 1, 2]]);
    assert!(proof.transitions().is_empty());
    assert_eq!(proof.point_connections().len(), 2);
    for p in proof.point_connections() {
        p.point().reverify().unwrap();
    }
    proof.reverify().unwrap();
    let mut wrong = specs;
    wrong[0].domain[0] = [0.13; 2];
    let missing = ss::connect_mixed_fiber_cover(&cover, &wrong, 100000).unwrap();
    assert!(missing.components().is_none());
    assert_eq!(missing.reason(), "original-point-bridge-unproven");
    assert!(
        ss::connect_mixed_fiber_cover(&cover, &specs, 1)
            .unwrap()
            .components()
            .is_none()
    );
}
