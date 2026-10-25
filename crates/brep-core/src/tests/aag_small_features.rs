use super::*;

fn budget() -> Budget {
    Budget::new(10_000_000, 8, 60_000).unwrap()
}

fn graph_with_attrs(model: &Model) -> Aag {
    let mut graph = Aag::build(model, &budget()).unwrap();
    graph.attach_face_attrs(model, &budget()).unwrap();
    graph
}

fn filleted_box(radius: f64) -> Model {
    let model = crate::cuboid([0.; 3], [20.; 3]).unwrap();
    let edge = model
        .edges
        .iter()
        .enumerate()
        .find_map(|(index, edge)| {
            let a = model.vertices[edge.vertices[0]].point;
            let b = model.vertices[edge.vertices[1]].point;
            ((a[0] - b[0]).abs() < 1e-9
                && (a[1] - b[1]).abs() < 1e-9
                && (a[2] - b[2]).abs() > 1.)
                .then_some(index)
        })
        .unwrap();
    crate::analytic_features::analytic_fillet(&model, edge, radius)
        .unwrap()
        .0
}

fn boss_model(x0: f64, y0: f64, x1: f64, y1: f64) -> Model {
    crate::boolean(
        &crate::cuboid([0.; 3], [40., 30., 2.]).unwrap(),
        &crate::cuboid([x0, y0, 2.], [x1, y1, 12.]).unwrap(),
        "union",
    )
    .unwrap()
}

#[test]
fn small_feature_corpus_meets_recall_and_false_positive_bounds() {
    type Expectation = (SmallFeatureKind, f64, f64, Option<(f64, f64)>);
    let small_pocket = crate::boolean(
        &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &crate::cuboid([2., 2., 15.], [6., 6., 22.]).unwrap(),
        "difference",
    )
    .unwrap();
    let large_pocket = crate::boolean(
        &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &crate::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
        "difference",
    )
    .unwrap();
    let chamfer_source = crate::cuboid([0.; 3], [10.; 3]).unwrap();
    let short_face = crate::chamfer(&chamfer_source, 0, 0.01).unwrap();
    let cases: Vec<(&str, Model, Option<Expectation>)> = vec![
        (
            "small hole",
            crate::analytic::tube(5., 0.5, 6.).unwrap(),
            Some((
                SmallFeatureKind::Hole,
                1.,
                1e-4,
                Some((std::f64::consts::PI * 0.25 * 6., 1e-4)),
            )),
        ),
        (
            "large hole",
            crate::analytic::tube(5., 2., 6.).unwrap(),
            None,
        ),
        (
            "small fillet",
            filleted_box(0.5),
            Some((SmallFeatureKind::Fillet, 0.5, 0.005, None)),
        ),
        ("large fillet", filleted_box(3.), None),
        (
            "small pocket",
            small_pocket,
            Some((SmallFeatureKind::Pocket, 5., 1e-6, Some((80., 1e-6)))),
        ),
        ("large pocket", large_pocket, None),
        (
            "small boss",
            boss_model(15., 10., 25., 20.),
            Some((SmallFeatureKind::Boss, 10., 1e-6, Some((1000., 1e-6)))),
        ),
        ("large boss", boss_model(10., 5., 30., 25.), None),
        (
            "short face",
            short_face,
            Some((
                SmallFeatureKind::ShortFace,
                10. * 0.01 * std::f64::consts::SQRT_2,
                1e-5,
                Some((0., 0.)),
            )),
        ),
        (
            "plain cuboid",
            crate::cuboid([0.; 3], [20.; 3]).unwrap(),
            None,
        ),
    ];

    let (mut tp, mut fp, mut fn_) = (0usize, 0usize, 0usize);
    for (name, model, expected) in &cases {
        let graph = graph_with_attrs(model);
        let report = scan_small_features(model, &graph, &SmallFeatureOptions::default(), &budget())
            .unwrap();
        if let Some((kind, size, size_tol, delta)) = *expected {
            let matched = report
                .candidates
                .iter()
                .filter(|feature| feature.kind == kind)
                .count();
            tp += usize::from(matched > 0);
            fp += matched.saturating_sub(1) + report.candidates.len().saturating_sub(matched);
            fn_ += usize::from(matched == 0);
            if matched == 1 && report.candidates.len() == 1 {
                let feature = &report.candidates[0];
                assert!(
                    (feature.size - size).abs() <= size_tol,
                    "{name}: size {}, expected {size}: {feature:?}",
                    feature.size
                );
                if let Some((volume, volume_tol)) = delta {
                    assert!(
                        (feature.delta_volume - volume).abs() <= volume_tol,
                        "{name}: delta_volume {}, expected {volume}: {feature:?}",
                        feature.delta_volume
                    );
                }
                assert!(!feature.faces.is_empty(), "{name}: member faces reported");
            }
        } else {
            fp += report.candidates.len();
        }
    }
    let recall = tp as f64 / (tp + fn_).max(1) as f64;
    let false_positive_rate = fp as f64 / (tp + fp).max(1) as f64;
    assert!(
        recall >= 0.95 && false_positive_rate <= 0.05,
        "recall {recall}, FP rate {false_positive_rate} (tp={tp}, fp={fp}, fn={fn_})"
    );
}

#[test]
fn small_feature_defaults_scale_and_absolute_override_work() {
    let compact = crate::analytic::tube(5., 2., 6.).unwrap();
    let graph = graph_with_attrs(&compact);
    let compact_report =
        scan_small_features(&compact, &graph, &SmallFeatureOptions::default(), &budget()).unwrap();
    assert!(compact_report.candidates.is_empty(), "{compact_report:?}");

    // Keep the bore diameter fixed while growing the body tenfold: the
    // bbox-relative default now admits the same 4 mm hole.
    let enlarged = crate::analytic::tube(50., 2., 60.).unwrap();
    let graph = graph_with_attrs(&enlarged);
    let enlarged_report =
        scan_small_features(&enlarged, &graph, &SmallFeatureOptions::default(), &budget()).unwrap();
    assert!(enlarged_report.bbox_diagonal > 9.999 * compact_report.bbox_diagonal);
    assert!(enlarged_report.hole_diameter_max > 9.999 * compact_report.hole_diameter_max);
    assert_eq!(enlarged_report.candidates.len(), 1, "{enlarged_report:?}");
    assert_eq!(enlarged_report.candidates[0].kind, SmallFeatureKind::Hole);

    let options = SmallFeatureOptions {
        hole_diameter_max: Some(5.),
        ..SmallFeatureOptions::default()
    };
    let overridden = scan_small_features(&compact, &graph_with_attrs(&compact), &options, &budget())
        .unwrap();
    assert_eq!(overridden.hole_diameter_max, 5.);
    assert_eq!(overridden.candidates.len(), 1);
    assert_eq!(overridden.candidates[0].kind, SmallFeatureKind::Hole);
}

#[test]
fn small_feature_input_errors_and_budget_are_typed() {
    let model = crate::analytic::tube(5., 0.5, 6.).unwrap();
    let plain_graph = Aag::build(&model, &budget()).unwrap();
    let error = scan_small_features(
        &model,
        &plain_graph,
        &SmallFeatureOptions::default(),
        &budget(),
    )
    .unwrap_err();
    assert_eq!(error.code, "BREP_AAG_SMALL_FEATURES_INPUT");

    let graph = graph_with_attrs(&model);
    let mut stale_model = model.clone();
    stale_model.faces[0].surface.control_points[0][0][0] += 1.;
    let error = scan_small_features(
        &stale_model,
        &graph,
        &SmallFeatureOptions::default(),
        &budget(),
    )
    .unwrap_err();
    assert_eq!(error.code, "BREP_AAG_SMALL_FEATURES_INPUT");

    let graph = graph_with_attrs(&model);
    let error = scan_small_features(
        &model,
        &graph,
        &SmallFeatureOptions::default(),
        &Budget::with_iterations(2).unwrap(),
    )
    .unwrap_err();
    assert!(
        error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
        "budget exhaustion must be a resource error: {error:?}"
    );

    let invalid = SmallFeatureOptions {
        fillet_radius_max: Some(0.),
        ..SmallFeatureOptions::default()
    };
    let error = scan_small_features(&model, &graph, &invalid, &budget()).unwrap_err();
    assert_eq!(error.code, "BREP_AAG_SMALL_FEATURES_INPUT");
}

#[test]
fn small_feature_scan_scales_toward_ten_thousand_faces() {
    // MAX_FACES=1024 prevents a literal 10^4-face in-contract Model. Measure
    // the largest established synthetic instead: 150 disjoint cuboids = 900
    // faces. The no-concave fast path, attribute scan and candidate scan are
    // linear in faces/edges, so release time extrapolates by 10^4/900 ≈ 11.1.
    let unit = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    use brep_topology::Shell;
    let copies = 150;
    let mut model = Model(
        brep_topology::Model {
            vertices: vec![],
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: unit.tolerance_mm,
        },
        crate::TopologyIds::default(),
    );
    for _ in 0..copies {
        let (v, e, l, f, s) = (
            model.vertices.len(),
            model.edges.len(),
            model.loops.len(),
            model.faces.len(),
            model.shells.len(),
        );
        model.vertices.extend(unit.vertices.iter().cloned());
        model.edges.extend(unit.edges.iter().map(|edge| crate::Edge {
            degenerate: edge.degenerate,
            vertices: [edge.vertices[0] + v, edge.vertices[1] + v],
            curve: edge.curve.clone(),
        }));
        model.loops.extend(unit.loops.iter().map(|wire| crate::Loop {
            coedges: wire
                .coedges
                .iter()
                .map(|c| crate::Coedge::new(c.edge + e, c.reversed, c.pcurve.clone()))
                .collect(),
        }));
        model.faces.extend(unit.faces.iter().map(|face| crate::Face {
            surface: face.surface.clone(),
            outer: face.outer + l,
            holes: face.holes.iter().map(|hole| hole + l).collect(),
        }));
        model.shells.extend(unit.shells.iter().map(|shell| Shell {
            faces: shell
                .faces
                .iter()
                .map(|use_| brep_topology::FaceUse {
                    face: use_.face + f,
                    reversed: use_.reversed,
                })
                .collect(),
            closed: shell.closed,
        }));
        model.bodies.extend(unit.bodies.iter().map(|body| brep_topology::Body {
            outer_shell: body.outer_shell + s,
            inner_shells: body.inner_shells.iter().map(|shell| shell + s).collect(),
        }));
    }
    assert_eq!(model.faces.len(), copies * 6);

    let start = std::time::Instant::now();
    let mut graph = Aag::build(&model, &budget()).unwrap();
    graph.attach_face_attrs(&model, &budget()).unwrap();
    let scan_start = std::time::Instant::now();
    let report = scan_small_features(
        &model,
        &graph,
        &SmallFeatureOptions::default(),
        &budget(),
    )
    .unwrap();
    let scan_elapsed = scan_start.elapsed();
    let elapsed = start.elapsed();
    assert!(report.candidates.is_empty(), "{report:?}");
    let scan_extrapolation = scan_elapsed.mul_f64(10_000. / 900.);
    println!(
        "900-face total: {elapsed:?}; scan: {scan_elapsed:?}; linear 10^4-face scan estimate: {scan_extrapolation:?}"
    );
    let bound = if cfg!(debug_assertions) { 20.0 } else { 2.0 };
    assert!(
        elapsed.as_secs_f64() < bound && scan_elapsed.as_secs_f64() < bound,
        "900-face total {elapsed:?}, scan {scan_elapsed:?} (bound {bound} s)"
    );
    if !cfg!(debug_assertions) {
        assert!(
            scan_extrapolation.as_secs_f64() < 2.0,
            "linear 10^4-face scan estimate exceeds 2 s: {scan_extrapolation:?}"
        );
    }
}
