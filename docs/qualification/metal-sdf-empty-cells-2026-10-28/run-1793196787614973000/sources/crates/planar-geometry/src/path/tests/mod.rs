use super::*;

#[test]
fn ellipse_flattens_and_closes() {
    let path = BezierPath::from_circle([0., 0.], 10.).unwrap();
    assert!(path.closed);
    assert_eq!(path.segments.len(), 4);
    let ring = path.to_ring(0.1).unwrap();
    assert!(ring.len() >= 8);
    let area: f64 = (0..ring.len())
        .map(|i| {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            a[0] * b[1] - a[1] * b[0]
        })
        .sum::<f64>()
        / 2.;
    assert!((area.abs() - std::f64::consts::PI * 100.).abs() < 5.);
}

#[test]
fn de_casteljau_insert_preserves_endpoints() {
    let path = BezierPath::open(
        [0., 0.],
        vec![PathSegment::Cubic {
            c1: [1., 2.],
            c2: [2., 2.],
            to: [3., 0.],
        }],
    )
    .unwrap();
    let split = path.insert_anchor(0, 0.5).unwrap();
    assert_eq!(split.segments.len(), 2);
    assert_eq!(split.start, [0., 0.]);
    assert_eq!(split.segments[1].end(), [3., 0.]);
}

#[test]
fn reverse_roundtrip_endpoints() {
    let path = BezierPath::from_polyline(&[[0., 0.], [1., 0.], [1., 1.]], false).unwrap();
    let rev = path.reverse();
    assert_eq!(rev.start, [1., 1.]);
    assert_eq!(rev.segments.last().unwrap().end(), [0., 0.]);
}

#[test]
fn join_welds_coincident_ends() {
    let a = BezierPath::from_polyline(&[[0., 0.], [1., 0.]], false).unwrap();
    let b = BezierPath::from_polyline(&[[1., 0.], [2., 0.]], false).unwrap();
    let j = join_paths(&a, false, &b, false, 1e-6).unwrap();
    assert_eq!(j.segments.len(), 2);
    assert_eq!(j.segments.last().unwrap().end(), [2., 0.]);
}

#[test]
fn simplify_reduces_collinear() {
    let path =
        BezierPath::from_polyline(&[[0., 0.], [1., 0.], [2., 0.], [3., 0.], [3., 1.]], false)
            .unwrap();
    let s = path.simplify(0.01).unwrap();
    assert!(s.anchor_count() <= 4);
}

#[test]
fn outline_stroke_closed_rect() {
    let path = BezierPath::from_rect([0., 0.], [10., 10.]).unwrap();
    let outline = path.outline_stroke(2.).unwrap();
    assert!(outline.closed);
    assert!(outline.anchor_count() >= 8);
}

#[test]
fn convert_shapes_to_path() {
    let r = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
    let c = BezierPath::from_circle([1., 1.], 2.).unwrap();
    let p = BezierPath::from_polygon(&[[0., 0.], [1., 0.], [0.5, 1.]], true).unwrap();
    assert!(r.closed && c.closed && p.closed);
}

#[test]
fn split_closed_opens_at_node() {
    let path = BezierPath::from_rect([0., 0.], [1., 1.]).unwrap();
    let pieces = path.split_at_anchor(1).unwrap();
    assert_eq!(pieces.len(), 1);
    assert!(!pieces[0].closed);
}

#[test]
fn average_and_handle_link() {
    let path = BezierPath::from_polyline(&[[0., 0.], [2., 2.], [4., 0.]], false).unwrap();
    let avg = path
        .average_anchors(&[0, 2], AverageAxis::Horizontal)
        .unwrap();
    assert!((avg.start[0] - 2.).abs() < 1e-9);
    assert!((avg.segments[1].end()[0] - 2.).abs() < 1e-9);
    let cubic = BezierPath::open(
        [0., 0.],
        vec![
            PathSegment::Cubic {
                c1: [1., 1.],
                c2: [2., 1.],
                to: [3., 0.],
            },
            PathSegment::Cubic {
                c1: [4., 1.],
                c2: [5., 1.],
                to: [6., 0.],
            },
        ],
    )
    .unwrap();
    let edited = cubic
        .set_anchor_handle(1, HandleSide::Out, [3., 2.])
        .unwrap()
        .apply_handle_link(1, HandleSide::Out, HandleLink::Mirrored)
        .unwrap();
    let (inn, out) = edited.anchor_handles(1).unwrap();
    assert_eq!(out, Some([3., 2.]));
    assert_eq!(inn, Some([3., -2.]));
}

#[test]
fn offset_closed_grows_area() {
    let path = BezierPath::from_rect([0., 0.], [4., 4.]).unwrap();
    let out = path.offset(1., "Miter", 8).unwrap();
    assert_eq!(out.len(), 1);
    let a: f64 = crate::rings::area(&out[0].to_ring(0.05).unwrap()).abs();
    assert!((a - 36.).abs() < 1., "area={a}");
}

#[test]
fn smooth_makes_cubics() {
    let path = BezierPath::from_polyline(&[[0., 0.], [2., 1.], [4., 0.]], false).unwrap();
    let s = path.smooth().unwrap();
    assert!(
        s.segments
            .iter()
            .all(|seg| matches!(seg, PathSegment::Cubic { .. }))
    );
}

#[test]
fn open_path_drops_close() {
    let path = BezierPath::from_rect([0., 0.], [2., 2.]).unwrap();
    let open = path.open_path().unwrap();
    assert!(!open.closed);
}

#[test]
fn delete_segment_splits_open() {
    let path =
        BezierPath::from_polyline(&[[0., 0.], [1., 0.], [2., 0.], [3., 0.]], false).unwrap();
    let parts = path.delete_segments(&[1, 2]).unwrap();
    assert_eq!(parts.len(), 2);
    assert!(parts.iter().all(|p| !p.closed));
}
#[test]
fn flatten_meets_small_tolerance_beyond_old_sample_cap() {
    let path = BezierPath::open(
        [0., 0.],
        vec![PathSegment::Cubic {
            c1: [0., 100.],
            c2: [100., 100.],
            to: [100., 0.],
        }],
    )
    .unwrap();
    let tolerance = 0.00001;
    let points = path.flatten_tol(tolerance).unwrap();
    assert!(points.len() > 256);
    for i in 0..=1000 {
        let t = i as f64 / 1000.;
        let u = 1. - t;
        let p = [100. * (3. * u * t * t + t * t * t), 300. * u * t];
        let nearest = points
            .windows(2)
            .map(|ab| {
                let v = sub2(ab[1], ab[0]);
                let q = sub2(p, ab[0]);
                let parameter =
                    ((q[0] * v[0] + q[1] * v[1]) / (v[0] * v[0] + v[1] * v[1])).clamp(0., 1.);
                dist(p, lerp(ab[0], ab[1], parameter))
            })
            .fold(f64::INFINITY, f64::min);
        assert!(nearest <= tolerance, "error {nearest} exceeds {tolerance}");
    }
}
#[test]
fn flatten_preserves_collinear_overshoot_and_rejects_invalid_options() {
    let path = BezierPath::open(
        [0., 0.],
        vec![PathSegment::Cubic {
            c1: [10., 0.],
            c2: [10., 0.],
            to: [1., 0.],
        }],
    )
    .unwrap();
    let points = path.flatten_tol(0.01).unwrap();
    assert!(points.iter().any(|p| p[0] > 7.));
    for tolerance in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(path.flatten_tol(tolerance).is_err());
    }
    assert!(BezierPath::open([f64::NAN, 0.], vec![]).is_err());
}
