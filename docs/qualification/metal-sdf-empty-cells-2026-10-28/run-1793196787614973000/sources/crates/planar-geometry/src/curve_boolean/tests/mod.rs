use super::*;
use crate::path::BezierPath;

fn rectangle(x: f64, y: f64, w: f64, h: f64) -> BezPath {
    BezPath::from_bezier_paths(&[BezierPath::from_rect([x, y], [x + w, y + h]).unwrap()])
}
fn ellipse(x: f64, y: f64, rx: f64, ry: f64) -> BezPath {
    BezPath::from_bezier_paths(&[BezierPath::from_ellipse([x, y], rx, ry).unwrap()])
}
fn reverse(path: &BezPath) -> BezPath {
    BezPath::from_bezier_paths(
        &path
            .to_bezier_paths()
            .unwrap()
            .iter()
            .map(BezierPath::reverse)
            .collect::<Vec<_>>(),
    )
}
fn combine(a: &BezPath, b: &BezPath, op: BinaryOp) -> BezPath {
    binary_op_with_eps(a, b, FillRule::NonZero, op, 0.05)
        .unwrap_or_else(|e| panic!("{op:?} {:?} with {:?}: {e:?}", a.elements(), b.elements()))
        .to_path()
}
fn expected(a: bool, b: bool, op: BinaryOp) -> bool {
    match op {
        BinaryOp::Union => a || b,
        BinaryOp::Intersection => a && b,
        BinaryOp::Difference => a && !b,
        BinaryOp::Xor => a != b,
    }
}
fn covered(path: &BezPath, point: Point, rule: FillRule) -> bool {
    let winding = path.winding(point);
    match rule {
        FillRule::NonZero => winding != 0,
        FillRule::EvenOdd => winding % 2 != 0,
    }
}
fn assert_grid(
    a: &BezPath,
    b: &BezPath,
    out: &BezPath,
    rule_a: FillRule,
    rule_b: FillRule,
    op: BinaryOp,
) {
    let bb = a.bounding_box().union(b.bounding_box());
    for iy in 0..33 {
        for ix in 0..33 {
            // Irrational-looking offsets avoid sampling exactly on source edges.
            let p = Point::new(
                bb.x0 + (bb.width() + 0.7) * (ix as f64 + 0.371) / 33. - 0.35,
                bb.y0 + (bb.height() + 0.7) * (iy as f64 + 0.619) / 33. - 0.35,
            );
            assert_eq!(
                covered(out, p, FillRule::NonZero),
                expected(covered(a, p, rule_a), covered(b, p, rule_b), op),
                "{op:?} region mismatch at {p:?}"
            );
        }
    }
    for contour in out.to_bezier_paths().unwrap() {
        assert!(contour.closed);
    }
}
fn all_ops_grid(a: &BezPath, b: &BezPath) {
    for op in [
        BinaryOp::Union,
        BinaryOp::Intersection,
        BinaryOp::Difference,
        BinaryOp::Xor,
    ] {
        let out = combine(a, b, op);
        assert_grid(a, b, &out, FillRule::NonZero, FillRule::NonZero, op);
    }
}
#[test]
fn four_rectangle_arrangement_is_a_complete_disjoint_partition() {
    let mut inputs = [
        rectangle(0., 0., 10., 10.),
        rectangle(4., 0., 10., 10.),
        rectangle(0., 4., 10., 10.),
        rectangle(4., 4., 10., 10.),
    ];
    for _ in 0..4 {
        let fold = |indices: Vec<usize>, op| {
            let mut iter = indices.into_iter();
            let mut result = inputs[iter.next().unwrap()].clone();
            for index in iter {
                result = combine(&result, &inputs[index], op);
            }
            result
        };
        let mut cells = Vec::new();
        // This is Curvex Divide's complete subset arrangement: intersect each
        // subset, then subtract the union of every input outside that subset.
        for mask in 1..16 {
            let included = (0..4).filter(|i| mask & (1 << i) != 0).collect();
            let excluded: Vec<_> = (0..4).filter(|i| mask & (1 << i) == 0).collect();
            let intersection = fold(included, BinaryOp::Intersection);
            let cell = if excluded.is_empty() {
                intersection
            } else {
                combine(
                    &intersection,
                    &fold(excluded, BinaryOp::Union),
                    BinaryOp::Difference,
                )
            };
            cells.push(cell);
        }
        assert!((cells.iter().map(Shape::area).sum::<f64>() - 196.).abs() < 1e-8);
        assert!((cells[14].area() - 36.).abs() < 1e-8);
        for yi in -1..15 {
            for xi in -1..15 {
                let point = Point::new(xi as f64 + 0.5, yi as f64 + 0.5);
                let actual = cells.iter().filter(|c| c.winding(point) != 0).count();
                let expected = usize::from((0..14).contains(&xi) && (0..14).contains(&yi));
                assert_eq!(actual, expected, "partition coverage at {point:?}");
            }
        }
        inputs.rotate_left(1);
    }
}

#[test]
fn almost_axis_aligned_edges_keep_crossings_on_both_axes() {
    for transpose in [false, true] {
        let polygon = |vertices: &[(f64, f64)]| {
            let mut path = BezPath::new();
            for (index, &(x, y)) in vertices.iter().enumerate() {
                let point = if transpose {
                    Point::new(y, x)
                } else {
                    Point::new(x, y)
                };
                if index == 0 {
                    path.move_to(point);
                } else {
                    path.line_to(point);
                }
            }
            path.close_path();
            path
        };
        // The tiny positive x overlap of the first shape's left edge and
        // the cutter's bottom edge still contains an interior crossing.
        let source = polygon(&[(4., 0.), (10., 0.), (10., 10.), (4. + 1e-9, 10.)]);
        let cutter = polygon(&[(0., 4.), (14., 4.), (14., 14.), (0., 14.)]);
        let result = combine(&source, &cutter, BinaryOp::Difference);
        assert!((result.area() - 24.).abs() < 1e-7);
        all_ops_grid(&source, &cutter);
    }
}

#[test]
fn overlapping_ellipses_preserve_cubic_boundaries_and_region() {
    let a = ellipse(0., 0., 10., 7.);
    let b = ellipse(8., 2., 9., 6.);
    all_ops_grid(&a, &b);
    let union = combine(&a, &b, BinaryOp::Union);
    assert!(
        union
            .segments()
            .all(|s| matches!(s, CurveSegment::Cubic(..))),
        "only original cubic spans belong on the ellipse boundary"
    );
    assert_eq!(union.to_bezier_paths().unwrap().len(), 1);
}
#[test]
fn rounded_protrusion_preserves_original_controls() {
    let a = rectangle(0., 0., 10., 10.);
    let b = ellipse(10., 5., 4., 3.);
    let union = combine(&a, &b, BinaryOp::Union);
    all_ops_grid(&a, &b);
    let retained: Vec<_> = b
        .segments()
        .filter(|s| s.start().x >= 10. && s.end().x >= 10.)
        .collect();
    assert!(!retained.is_empty());
    for source in retained {
        assert!(
            union
                .segments()
                .any(|out| out == source || out == source.reverse()),
            "unaffected rounded arcs must retain their actual controls"
        );
    }
    assert!((union.bounding_box().max_x() - 14.).abs() < 1e-8);
}
#[test]
fn contained_disjoint_equal_and_reversed_curves() {
    let outer = ellipse(0., 0., 10., 10.);
    for other in [
        ellipse(0., 0., 3., 2.),
        ellipse(30., 0., 3., 2.),
        outer.clone(),
        reverse(&outer),
    ] {
        all_ops_grid(&outer, &other);
    }
    let union = combine(&outer, &reverse(&outer), BinaryOp::Union);
    assert_eq!(union.to_bezier_paths().unwrap().len(), 1);
    assert!((union.area().abs() - outer.area().abs()).abs() < 1e-7);
}
#[test]
fn evenodd_compound_keeps_holes_and_nested_islands() {
    let mut a = ellipse(0., 0., 10., 10.);
    a.extend(ellipse(0., 0., 7., 7.).iter());
    a.extend(ellipse(0., 0., 2., 2.).iter());
    let b = rectangle(-11., -1., 22., 2.);
    for op in [
        BinaryOp::Union,
        BinaryOp::Intersection,
        BinaryOp::Difference,
        BinaryOp::Xor,
    ] {
        let out = binary_op(&a, &b, FillRule::EvenOdd, FillRule::NonZero, op, 0.05)
            .unwrap()
            .to_path();
        assert_grid(&a, &b, &out, FillRule::EvenOdd, FillRule::NonZero, op);
    }
}
#[test]
fn self_intersecting_cubic_normalizes_each_fill_rule() {
    let mut a = BezPath::new();
    a.move_to((0., 0.));
    a.curve_to((6., 10.), (-6., 10.), (3., 0.));
    a.close_path();
    let empty = BezPath::new();
    for rule in [FillRule::EvenOdd, FillRule::NonZero] {
        let out = binary_op_with_eps(&a, &empty, rule, BinaryOp::Union, 0.05)
            .unwrap()
            .to_path();
        assert_grid(&a, &empty, &out, rule, rule, BinaryOp::Union);
        assert!(out.segments().any(|s| matches!(s, CurveSegment::Cubic(..))));
    }
}
#[test]
fn tangent_and_nearly_coincident_inputs_remain_closed() {
    let a = ellipse(0., 0., 10., 10.);
    for other in [
        ellipse(20., 0., 10., 10.),
        ellipse(19.999, 0., 10., 10.),
        ellipse(20.001, 0., 10., 10.),
        ellipse(0.001, 0., 10., 10.),
    ] {
        all_ops_grid(&a, &other);
    }
    all_ops_grid(&rectangle(0., 0., 10., 10.), &rectangle(10., 0., 10., 10.));
}
#[test]
fn subdivided_coincident_cubic_spans_do_not_double_the_boundary() {
    let a = ellipse(0., 0., 10., 10.);
    let mut b = BezPath::new();
    b.move_to(a.segments().next().unwrap().start());
    for source in a.segments() {
        let (left, right) = source.split(0.317);
        b.push(left.to_path_el());
        b.push(right.to_path_el());
    }
    b.close_path();
    all_ops_grid(&a, &b);
    let union = combine(&a, &b, BinaryOp::Union);
    assert_eq!(union.to_bezier_paths().unwrap().len(), 1);
    assert!((union.area() - a.area().abs()).abs() < 1e-6);
}
#[test]
fn partial_coincident_arc_preserves_the_remaining_regions() {
    let source = CurveSegment::Cubic(
        Point::new(0., 0.),
        Point::new(0., 10.),
        Point::new(10., 10.),
        Point::new(10., 0.),
    );
    let mut a = BezPath::new();
    a.move_to(source.start());
    a.push(source.to_path_el());
    a.close_path();
    let cut = source.subsegment(0.23, 0.81);
    let mut b = BezPath::new();
    b.move_to(cut.start());
    b.push(cut.to_path_el());
    b.close_path();
    all_ops_grid(&a, &b);
    all_ops_grid(&reverse(&a), &b);
}
#[test]
fn stationary_cubic_midpoint_does_not_drop_a_valid_region() {
    // x=(t-.5)^2, y=(t-.5)^3 has a cusp where its derivative vanishes.
    let mut cusp = BezPath::new();
    cusp.move_to((0.25, -0.125));
    cusp.curve_to((-1. / 12., 0.125), (-1. / 12., -0.125), (0.25, 0.125));
    cusp.close_path();
    let empty = BezPath::new();
    let out = combine(&cusp, &empty, BinaryOp::Union);
    assert_grid(
        &cusp,
        &empty,
        &out,
        FillRule::NonZero,
        FillRule::NonZero,
        BinaryOp::Union,
    );
    assert!((out.area() - cusp.area().abs()).abs() < 1e-10);
    assert!(out.segments().any(|s| matches!(s, CurveSegment::Cubic(..))));
}
#[test]
fn varied_curved_arrangements_match_independent_input_winding() {
    for i in 0..20 {
        let t = i as f64;
        let a = ellipse(
            t.sin() * 4.,
            t.cos() * 2.,
            2. + (t * 0.7).sin().abs() * 8.,
            1. + (t * 0.9).cos().abs() * 4.,
        );
        let b = ellipse(
            (t * 0.6).cos() * 7.,
            (t * 0.3).sin() * 3.,
            1. + (t * 0.13).cos().abs() * 5.,
            1. + (t * 0.41).sin().abs() * 6.,
        );
        all_ops_grid(&a, &b);
    }
}
#[test]
fn cubic_equivalent_of_a_line_can_share_polygon_edges() {
    let a = rectangle(0., 0., 10., 10.);
    let mut b = BezPath::new();
    b.move_to((0., 0.));
    b.curve_to((0., 0.), (0., 0.), (10., 0.));
    b.line_to((10., 10.));
    b.line_to((0., 10.));
    b.close_path();
    all_ops_grid(&a, &b);
    assert_eq!(
        combine(&a, &b, BinaryOp::Union)
            .to_bezier_paths()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn shared_span_still_intersects_a_different_part_of_a_loop() {
    let curve = CurveSegment::Cubic(
        Point::new(0., 0.),
        Point::new(6., 10.),
        Point::new(-6., 10.),
        Point::new(3., 0.),
    );
    let mut a = BezPath::new();
    a.move_to(curve.start());
    a.push(curve.to_path_el());
    a.close_path();
    let partial = curve.subsegment(0., 0.2);
    let mut b = BezPath::new();
    b.move_to(partial.start());
    b.push(partial.to_path_el());
    b.close_path();
    all_ops_grid(&a, &b);
}
#[test]
fn translated_geometry_beyond_former_coordinate_limit_is_preserved() {
    let a = ellipse(0., 0., 10., 7.);
    let b = ellipse(8., 2., 9., 6.);
    let shift = Point::new(1e7, -1e7);
    let translate = |path: &BezPath| {
        BezPath::from_vec(
            path.iter()
                .map(|el| match el {
                    PathEl::MoveTo(p) => PathEl::MoveTo(p + shift),
                    PathEl::LineTo(p) => PathEl::LineTo(p + shift),
                    PathEl::QuadTo(a, b) => PathEl::QuadTo(a + shift, b + shift),
                    PathEl::CurveTo(a, b, c) => {
                        PathEl::CurveTo(a + shift, b + shift, c + shift)
                    }
                    PathEl::ClosePath => PathEl::ClosePath,
                })
                .collect(),
        )
    };
    let ta = translate(&a);
    let tb = translate(&b);
    for op in [
        BinaryOp::Union,
        BinaryOp::Intersection,
        BinaryOp::Difference,
        BinaryOp::Xor,
    ] {
        let original = combine(&a, &b, op);
        let translated = combine(&ta, &tb, op);
        assert!(
            (translated.area() - original.area()).abs() < 1e-4,
            "translated {op:?} changed area"
        );
        assert_grid(
            &ta,
            &tb,
            &translated,
            FillRule::NonZero,
            FillRule::NonZero,
            op,
        );
    }
}
#[test]
fn five_thousand_simple_vertices_are_not_rejected_by_old_segment_cap() {
    let count = 5000;
    let mut path = BezPath::new();
    for i in 0..count {
        let angle = std::f64::consts::TAU * i as f64 / count as f64;
        let p = (100. * angle.cos(), 100. * angle.sin());
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    let output = combine(&path, &BezPath::new(), BinaryOp::Union);
    assert_eq!(output.segments().count(), count);
    let expected = count as f64 * 0.5 * 10000. * (std::f64::consts::TAU / count as f64).sin();
    assert!((output.area() - expected).abs() < 1e-5);
    assert_eq!(output.winding(Point::ZERO), 1);
}
