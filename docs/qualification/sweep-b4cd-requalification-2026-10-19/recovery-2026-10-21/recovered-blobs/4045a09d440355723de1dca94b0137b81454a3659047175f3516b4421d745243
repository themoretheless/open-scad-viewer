use nurbs_core::{
    curve::Curve,
    planar_toolpath::{self, Limits, Spec, Stop},
};
fn square(reverse: bool) -> Vec<Curve> {
    let mut p = vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
    if reverse {
        p.reverse();
    }
    (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
        .collect()
}
fn spec() -> Spec {
    Spec {
        tool_radius: 1.,
        stock_allowance: 0.2,
        stepover: 0.5,
        passes: 3,
        tolerance: 0.02,
    }
}
fn segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t =
        (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1])).clamp(0., 1.);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}
#[test]
fn square_toolpaths_have_global_clearance_and_correct_side_in_both_orientations() {
    for reverse in [false, true] {
        let source = square(reverse);
        let before = source.clone();
        let s = spec();
        let report = planar_toolpath::generate(&source, s, Limits::default()).unwrap();
        assert_eq!(report.stop, Stop::Complete, "{:?}", report.stop);
        assert_eq!(report.qualified_passes, 3);
        let paths = report.paths.unwrap();
        for (i, path) in paths.iter().enumerate() {
            assert!(path.outside_proven);
            assert!(path.minimum_clearance_bounds[0] >= s.tool_radius + s.stock_allowance);
            assert!(path.source_offset_error_upper <= s.tolerance * 0.125);
            assert!(path.nominal_offset_error_upper <= s.tolerance);
            if i > 0 {
                assert!(
                    (paths[i - 1].offset_distance - path.offset_distance - s.stepover).abs()
                        < 1e-12
                );
            }
            let first = path.curves[0].control_points.first().unwrap();
            let last = path.curves.last().unwrap().control_points.last().unwrap();
            assert_eq!(first, last);
            for curve in &path.curves {
                for k in 0..=100 {
                    let d = curve.domain();
                    let p = curve
                        .evaluate(d[0] + (d[1] - d[0]) * k as f64 / 100.)
                        .unwrap()
                        .point;
                    assert!(p[0] < 0. || p[0] > 10. || p[1] < 0. || p[1] > 10.);
                    let corners = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
                    let clearance = (0..4)
                        .map(|j| segment_distance([p[0], p[1]], corners[j], corners[(j + 1) % 4]))
                        .fold(f64::INFINITY, f64::min);
                    assert!(clearance + 1e-11 >= s.tool_radius + s.stock_allowance);
                }
            }
        }
        assert_eq!(source, before);
    }
}
#[test]
fn rational_circular_contour_follows_independent_radius_and_clearance() {
    let points = [
        [[3., 0.], [3., 3.], [0., 3.]],
        [[0., 3.], [-3., 3.], [-3., 0.]],
        [[-3., 0.], [-3., -3.], [0., -3.]],
        [[0., -3.], [3., -3.], [3., 0.]],
    ];
    let source: Vec<_> = points
        .iter()
        .map(|p| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: p.iter().map(|q| q.to_vec()).collect(),
            weights: vec![1., 0.5_f64.sqrt(), 1.],
            periodic: false,
        })
        .collect();
    let s = Spec {
        passes: 1,
        ..spec()
    };
    let report = planar_toolpath::generate(&source, s, Limits::default()).unwrap();
    assert_eq!(report.stop, Stop::Complete, "{:?}", report.stop);
    let paths = report.paths.unwrap();
    let path = &paths[0];
    for curve in &path.curves {
        for i in 0..=100 {
            let d = curve.domain();
            let p = curve
                .evaluate(d[0] + (d[1] - d[0]) * i as f64 / 100.)
                .unwrap()
                .point;
            let r = p[0].hypot(p[1]);
            assert!(r >= 3. + s.tool_radius + s.stock_allowance - 1e-11);
            assert!((r - (3. + path.offset_distance)).abs() < s.tolerance);
            assert!((r - (3. + s.tool_radius + s.stock_allowance)).abs() <= s.tolerance);
        }
    }
}
#[test]
fn work_refusals_and_bad_inputs_never_return_an_unqualified_plan() {
    let source = square(false);
    let s = spec();
    for limits in [
        Limits {
            offset_cells: 1,
            ..Limits::default()
        },
        Limits {
            clearance_cells: 1,
            ..Limits::default()
        },
        Limits {
            clearance_pairs: 1,
            ..Limits::default()
        },
        Limits {
            region_pairs: 1,
            ..Limits::default()
        },
    ] {
        let report = planar_toolpath::generate(&source, s, limits).unwrap();
        assert!(report.paths.is_none());
        assert_ne!(report.stop, Stop::Complete);
        assert!(report.offset_cells_charged <= limits.offset_cells);
        assert!(report.clearance_cells_charged <= limits.clearance_cells);
    }
    for bad in [
        Spec {
            tool_radius: 0.,
            ..s
        },
        Spec { passes: 0, ..s },
        Spec {
            tolerance: f64::NAN,
            ..s
        },
    ] {
        assert!(planar_toolpath::generate(&source, bad, Limits::default()).is_err());
    }
}

#[test]
fn cutter_that_cannot_fit_a_concave_slot_is_refused() {
    let p = [
        [0., 0.],
        [10., 0.],
        [10., 10.],
        [6., 10.],
        [6., 2.],
        [4., 2.],
        [4., 10.],
        [0., 10.],
    ];
    let source: Vec<_> = (0..p.len())
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % p.len()].to_vec()]).unwrap())
        .collect();
    let s = Spec {
        tool_radius: 1.5,
        passes: 1,
        ..spec()
    };
    let report = planar_toolpath::generate(&source, s, Limits::default()).unwrap();
    assert_eq!(report.source_region.valid, Some(true));
    assert!(report.paths.is_none());
    assert_eq!(report.stop, Stop::UnsafeClearance);
}
