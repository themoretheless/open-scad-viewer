use super::*;
use crate::rings::area;

#[test]
fn solid_rect_stroke_area() {
    let path = BezierPath::from_rect([0., 0.], [10., 0.]).unwrap(); // degenerate height - use line
    let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
    let outlines = outline_stroke(
        &line,
        &StrokeOptions {
            width: 2.,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(outlines.len(), 1);
    let ring = outlines[0].to_ring(0.05).unwrap();
    let a: f64 = area(&ring).abs();
    assert!((a - 20.).abs() < 1.5, "area={a}");
    let _ = path;
}

#[test]
fn dashed_produces_multiple() {
    let line = BezierPath::from_polyline(&[[0., 0.], [20., 0.]], false).unwrap();
    let outs = outline_stroke(
        &line,
        &StrokeOptions {
            width: 1.,
            dash: Some(vec![4., 4.]),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(outs.len() >= 2, "spans={}", outs.len());
}

#[test]
fn round_cap_closed_ok() {
    let line = BezierPath::from_polyline(&[[0., 0.], [5., 0.]], false).unwrap();
    let outs = outline_stroke(
        &line,
        &StrokeOptions {
            width: 2.,
            cap: LineCap::Round,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(outs[0].closed);
}

#[test]
fn arrow_and_bar_are_closed() {
    let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
    let marks = path_arrow_markers(&line, ArrowMarker::Arrow, ArrowMarker::Bar, 1.).unwrap();
    assert_eq!(marks.len(), 2);
    assert!(marks.iter().all(|p| p.closed));
}

#[test]
fn chevron_marker_is_closed() {
    let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
    let marks = path_arrow_markers(&line, ArrowMarker::None, ArrowMarker::Chevron, 1.).unwrap();
    assert_eq!(marks.len(), 1);
    assert!(marks[0].closed);
    assert!(marks[0].segments.len() >= 3);
}

#[test]
fn dash_spans_alternate_on_line() {
    let line = BezierPath::from_polyline(&[[0., 0.], [20., 0.]], false).unwrap();
    let spans = path_dash_spans(&line, &[4., 4.], 0., 0.25).unwrap();
    assert!(spans.len() >= 2, "spans={}", spans.len());
    assert!(spans.iter().all(|s| s.len() >= 2));
}
fn region_area(paths: &[BezierPath]) -> f64 {
    paths.iter().map(|p| area(&p.to_ring(0.001).unwrap())).sum()
}

#[test]
fn closed_stroke_keeps_hole_and_has_no_seam_gap() {
    let rect = BezierPath::from_rect([0., 0.], [10., 10.]).unwrap();
    let out = outline_stroke(
        &rect,
        &StrokeOptions {
            width: 2.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out.len(), 2);
    assert!((region_area(&out) - 80.).abs() < 1e-8);
    let rings = out.iter().map(|p| p.to_ring(0.01).unwrap()).collect();
    assert!(!crate::rings::inside([5., 5.], &rings));
    for p in [[0., 5.], [5., 0.], [10., 5.], [5., 10.], [-0.5, -0.5]] {
        assert!(crate::rings::inside(p, &rings), "seam/join gap at {p:?}");
    }
}

#[test]
fn caps_have_correct_area_and_round_chord_accuracy() {
    let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
    for (cap, expected) in [
        (LineCap::Butt, 20.),
        (LineCap::Square, 24.),
        (LineCap::Round, 20. + std::f64::consts::PI),
    ] {
        let out = outline_stroke_tol(
            &line,
            &StrokeOptions {
                width: 2.,
                cap,
                ..Default::default()
            },
            0.0001,
        )
        .unwrap();
        assert!(
            (region_area(&out) - expected).abs() < 0.001,
            "{cap:?}: {}",
            region_area(&out)
        );
    }
}

#[test]
fn joins_distinguish_bevel_round_and_miter() {
    let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.], [10., 10.]], false).unwrap();
    for (join, expected) in [
        (LineJoin::Bevel, 39.5),
        (LineJoin::Round, 39. + std::f64::consts::FRAC_PI_4),
        (LineJoin::Miter, 40.),
    ] {
        let out = outline_stroke_tol(
            &line,
            &StrokeOptions {
                width: 2.,
                join,
                ..Default::default()
            },
            0.0001,
        )
        .unwrap();
        assert!(
            (region_area(&out) - expected).abs() < 0.001,
            "{join:?}: {}",
            region_area(&out)
        );
    }
}

#[test]
fn acute_miter_is_clipped_to_bevel_at_limit() {
    let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.], [0., 1.]], false).unwrap();
    let outline = |limit| {
        outline_stroke(
            &line,
            &StrokeOptions {
                width: 2.,
                miter_limit: limit,
                ..Default::default()
            },
        )
        .unwrap()
    };
    let short = outline(2.);
    let long = outline(100.);
    assert!(region_area(&long) > region_area(&short) + 10.);
    let rightmost = |paths: &[BezierPath]| {
        paths
            .iter()
            .flat_map(|p| p.anchors())
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max)
    };
    assert!(rightmost(&short) < 11.);
    assert!(rightmost(&long) > 25.);
}

#[test]
fn dash_keeps_intermediate_corners_and_repeats_odd_pattern() {
    let spans = dash_spans(&[[0., 0.], [3., 0.], [3., 4.]], false, &[5., 1.], 0.).unwrap();
    assert_eq!(spans[0], vec![[0., 0.], [3., 0.], [3., 2.]]);
    let spans = dash_spans(&[[0., 0.], [12., 0.]], false, &[1., 2., 3.], 0.).unwrap();
    assert_eq!(
        spans,
        vec![
            vec![[0., 0.], [1., 0.]],
            vec![[3., 0.], [6., 0.]],
            vec![[7., 0.], [9., 0.]]
        ]
    );
}

#[test]
fn dash_closed_seam_joins_without_extra_caps() {
    let spans = dash_polylines(
        &[[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
        true,
        &[3., 2.],
        0.,
    )
    .unwrap();
    assert_eq!(spans.len(), 1);
    assert!(spans[0].0.windows(3).any(|p| p[1] == [0., 0.]));
    let solid = dash_polylines(
        &[[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
        true,
        &[100., 2.],
        0.,
    )
    .unwrap();
    assert!(solid[0].1);
    assert_eq!(solid[0].0.len(), 4);
}

#[test]
fn invalid_dash_options_fail_before_sampling() {
    let points = [[0., 0.], [1., 0.]];
    for pattern in [&[-1., 2.][..], &[f64::NAN, 2.], &[0., 0.]] {
        assert!(dash_spans(&points, false, pattern, 0.).is_err());
    }
    assert!(dash_spans(&points, false, &[1., 1.], f64::INFINITY).is_err());
}
#[test]
fn convex_route_matches_general_union_for_all_join_styles() {
    let polygon = [[0., 0.], [7., 0.], [10., 4.], [6., 9.], [1., 8.]];
    let path = BezierPath::from_polyline(&polygon, true).unwrap();
    for join in [LineJoin::Miter, LineJoin::Bevel, LineJoin::Round] {
        for width in [0.2, 2., 12.] {
            let options = StrokeOptions {
                width,
                join,
                ..Default::default()
            };
            let actual = outline_stroke_tol(&path, &options, 0.002).unwrap();
            let mut pieces = vec![];
            stroke_pieces(&polygon, true, &options, 0.002, &mut pieces).unwrap();
            let expected = crate::rings::nonzero(&pieces).unwrap();
            let actual: crate::rings::Rings =
                actual.iter().map(|p| p.to_ring(0.002).unwrap()).collect();
            let area =
                |r: &crate::rings::Rings| r.iter().map(|r| crate::rings::area(r)).sum::<f64>();
            assert!(
                (area(&actual) - area(&expected)).abs() < 1e-7,
                "{join:?}, width={width}"
            );
            for x in -4..16 {
                for y in -4..16 {
                    let p = [x as f64 + 0.237, y as f64 + 0.419];
                    assert_eq!(
                        crate::rings::inside(p, &actual),
                        crate::rings::inside(p, &expected)
                    );
                }
            }
        }
    }
    let star: Vec<_> = (0..5)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU * 2. / 5.;
            [a.cos() * 10., a.sin() * 10.]
        })
        .collect();
    assert!(
        convex_closed_stroke(&star, &StrokeOptions::default(), 0.01)
            .unwrap()
            .is_none()
    );
}
