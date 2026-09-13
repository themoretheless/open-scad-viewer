//! Independent distance oracle for round caps/joins, including actual failures
//! found by the Curvex stress corpus. No reference tessellator is used.
use planar_geometry::{
    path::BezierPath,
    rings,
    stroke::{self, LineCap, LineJoin, StrokeOptions},
    tessellation::{self, FillRule},
};
fn cross(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}
fn distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t =
        (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1])).clamp(0., 1.);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}
fn assert_round_coverage(pts: &[[f64; 2]], width: f64, closed: bool, tolerance: f64) {
    let path = BezierPath::from_polyline(pts, closed).unwrap();
    let options = StrokeOptions {
        width,
        cap: LineCap::Round,
        join: LineJoin::Round,
        ..Default::default()
    };
    let mesh = stroke::tessellate_stroke(&path, &options, tolerance).unwrap();
    let outline = stroke::outline_stroke_tol(&path, &options, tolerance).unwrap();
    let rings = outline
        .iter()
        .map(|p| p.flatten_tol(tolerance).unwrap())
        .collect();
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for p in pts {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k] - width);
            hi[k] = hi[k].max(p[k] + width);
        }
    }
    for x in 0..31 {
        for y in 0..29 {
            let p = [
                lo[0] + (hi[0] - lo[0]) * (x as f64 + 0.317) / 31.,
                lo[1] + (hi[1] - lo[1]) * (y as f64 + 0.419) / 29.,
            ];
            let d = (0..if closed { pts.len() } else { pts.len() - 1 })
                .map(|i| distance(p, pts[i], pts[(i + 1) % pts.len()]))
                .fold(f64::INFINITY, f64::min);
            if (d - width * 0.5).abs() < 2. * tolerance {
                continue;
            }
            let expected = d < width * 0.5;
            let hits = mesh
                .indices
                .chunks_exact(3)
                .filter(|t| {
                    let [a, b, c] = [
                        mesh.positions[t[0] as usize],
                        mesh.positions[t[1] as usize],
                        mesh.positions[t[2] as usize],
                    ];
                    cross(a, b, p) > 0. && cross(b, c, p) > 0. && cross(c, a, p) > 0.
                })
                .count();
            assert_eq!(hits, usize::from(expected), "overlap or gap at {p:?}");
            assert_eq!(rings::inside(p, &rings), expected, "outline at {p:?}");
        }
    }
}
#[test]
fn tiny_translated_retraced_capsules_keep_closed_boundaries() {
    for (pts, width) in [
        (
            [
                [10000000.005458426, 9999999.978037074],
                [9999999.993027065, 10000000.010431845],
                [10000000.005458426, 9999999.978037074],
            ],
            0.003523256231776509,
        ),
        (
            [
                [9999999.99275131, 10000000.033101393],
                [10000000.032837505, 10000000.007858966],
                [9999999.99275131, 10000000.033101393],
            ],
            0.044139157949463714,
        ),
    ] {
        for closed in [false, true] {
            assert_round_coverage(&pts, width, closed, 0.00001);
        }
    }
}
#[test]
fn retraced_crossing_band_does_not_emit_negative_roundoff_triangle() {
    let pts = [
        [14.024069280061305, -28.32572332177773],
        [-38.82696157614702, -4.39799905237291],
        [14.024069280061305, -28.32572332177773],
        [-0.8091521259032959, 17.704829432603802],
        [-37.39899578121208, 0.44394102613762243],
        [0.29454980675684794, 19.189316922038834],
        [-25.767213430778586, -18.170881641207792],
        [29.73332917512016, 20.020955076864936],
        [-22.46517213266567, -5.9971704273189985],
    ];
    for scale in [0.001, 1., 1000.] {
        let points: Vec<_> = pts.iter().map(|p| [p[0] * scale, p[1] * scale]).collect();
        assert_round_coverage(&points, 4.555900719309697 * scale, true, 0.01 * scale);
    }
}
#[test]
fn dense_coincident_input_stops_before_quadratic_pair_processing() {
    let square = vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let error = rings::nonzero(&vec![square; 2000]).unwrap_err();
    assert!(
        error.to_string().contains("candidate pair budget"),
        "{error}"
    );
}
#[test]
fn invalid_and_over_budget_inputs_return_errors_without_panics() {
    assert!(
        tessellation::tessellate_rings(&vec![vec![[0., 0.]; 65_537]], FillRule::NonZero).is_err()
    );
    for width in [0., -1., f64::NAN, f64::INFINITY] {
        let path = BezierPath::from_polyline(&[[0., 0.], [1., 1.]], false).unwrap();
        assert!(
            stroke::tessellate_stroke(
                &path,
                &StrokeOptions {
                    width,
                    ..Default::default()
                },
                0.01
            )
            .is_err()
        );
    }
}
