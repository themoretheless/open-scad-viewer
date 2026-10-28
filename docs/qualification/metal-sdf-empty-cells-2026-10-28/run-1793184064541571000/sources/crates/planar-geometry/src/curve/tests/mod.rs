use super::*;
fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
fn square(x: f64, y: f64, side: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to((x, y));
    path.line_to((x + side, y));
    path.line_to((x + side, y + side));
    path.line_to((x, y + side));
    path.close_path();
    path
}
#[test]
fn polynomial_area_and_bounds_are_analytic() {
    let mut path = BezPath::new();
    path.move_to((0., 0.));
    path.curve_to((0., 2.), (2., 2.), (2., 0.));
    path.close_path();
    near(path.area(), -2.4);
    near(path.bounding_box().max_y(), 1.5);
    assert_eq!(path.winding(p(1., 1.)), -1);
    assert_eq!(path.winding(p(1., 1.6)), 0);
}
#[test]
fn quadratic_promotion_preserves_the_curve() {
    let mut path = BezPath::new();
    path.move_to((0., 0.));
    path.quad_to((3., 6.), (9., 0.));
    let native = path.to_bezier_paths().unwrap();
    let rebuilt = BezPath::from_bezier_paths(&native);
    let a = path.segments().next().unwrap();
    let b = rebuilt.segments().next().unwrap();
    for t in [0., 0.13, 0.5, 0.88, 1.] {
        near(a.eval(t).distance(b.eval(t)), 0.);
    }
    near(path.bounding_box().max_y(), 3.);
}
#[test]
fn exact_subdivision_and_reversal() {
    let curve = CurveSegment::Cubic(p(-2., 1.), p(4., 8.), p(-3., -6.), p(5., 2.));
    let cut = curve.subsegment(0.2, 0.73);
    let reverse = curve.subsegment(0.73, 0.2);
    for t in [0., 0.07, 0.25, 0.5, 0.9, 1.] {
        near(cut.eval(t).distance(curve.eval(0.2 + 0.53 * t)), 0.);
        near(reverse.eval(t).distance(cut.eval(1. - t)), 0.);
    }
    let (a, b) = curve.split(0.37);
    near(a.signed_area() + b.signed_area(), curve.signed_area());
}
#[test]
fn signed_winding_distinguishes_holes_and_double_winding() {
    let outer = square(0., 0., 10.);
    let inner = square(2., 2., 6.);
    let mut same = outer.clone();
    same.extend(inner.iter());
    assert_eq!(same.winding(p(5., 5.)), 2);
    near(same.area(), 136.);
    let native = inner.to_bezier_paths().unwrap();
    let reversed: Vec<_> = native.iter().map(BezierPath::reverse).collect();
    let mut hole = outer;
    hole.extend(BezPath::from_bezier_paths(&reversed).iter());
    assert_eq!(hole.winding(p(5., 5.)), 0);
    assert_eq!(hole.winding(p(1., 5.)), 1);
    near(hole.area(), 64.);
}
#[test]
fn closed_area_remains_accurate_far_from_origin() {
    near(square(1e12, -1e12, 2.).area(), 4.);
}
#[test]
fn tangencies_and_shared_vertices_do_not_create_crossings() {
    let mut path = BezPath::new();
    path.move_to((0., 0.));
    path.curve_to((0., 2.), (2., 2.), (2., 0.));
    path.close_path();
    assert_eq!(path.winding(p(-1., 1.5)), 0);
    assert_eq!(path.winding(p(1., 1.5)), 0);
    assert_eq!(square(0., 0., 2.).winding(p(-1., 0.)), 0);
    assert_eq!(square(0., 0., 2.).winding(p(1., 0.)), 1);
    assert_eq!(square(0., 0., 2.).winding(p(1., 2.)), 0);
}
#[test]
fn cubic_with_two_y_extrema_uses_every_monotone_piece() {
    let curve = CurveSegment::Cubic(p(0., 0.), p(1., 3.), p(2., -3.), p(3., 0.));
    assert_eq!(curve.winding(p(-1., 0.)), 0);
    assert_eq!(curve.winding(p(1., 0.)), -1);
    assert_eq!(curve.winding(p(2., 0.)), 0);
    let bb = curve.bounding_box();
    near(bb.max_y(), 3.0_f64.sqrt() / 2.0);
    near(bb.min_y(), -3.0_f64.sqrt() / 2.0);
}
#[test]
fn compounds_convert_without_losing_closures_or_curves() {
    let mut path = square(0., 0., 2.);
    path.move_to((4., 0.));
    path.curve_to((5., 2.), (6., 2.), (7., 0.));
    let contours = path.to_bezier_paths().unwrap();
    assert_eq!(contours.len(), 2);
    assert!(contours[0].closed);
    assert!(!contours[1].closed);
    let rebuilt = BezPath::from_bezier_paths(&contours);
    near(path.area(), rebuilt.area());
    assert!(matches!(
        rebuilt.segments().last(),
        Some(CurveSegment::Cubic(..))
    ));
    assert_eq!(
        path.indexed_segments().map(|(i, _)| i).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 6]
    );
}
#[test]
fn invalid_geometry_is_an_explicit_conversion_error() {
    let mut path = BezPath::new();
    path.move_to((f64::NAN, 0.));
    path.line_to((1., 1.));
    assert!(path.to_bezier_paths().is_err());
    let mut path = BezPath::new();
    path.move_to((0., 0.));
    path.curve_to((1., 1.), (2., f64::INFINITY), (3., 0.));
    assert!(path.to_bezier_paths().is_err());
}
#[test]
fn stroke_uses_native_width_caps_and_contour_holes() {
    let path = square(0., 0., 10.);
    let out = stroke_checked(path.iter(), &Stroke::new(2.), 0.05).unwrap();
    assert_eq!(out.to_bezier_paths().unwrap().len(), 2);
    assert_eq!(out.winding(p(5., 5.)), 0);
    let mut line = BezPath::new();
    line.move_to((0., 0.));
    line.line_to((10., 0.));
    let cap =
        stroke_checked(line.iter(), &Stroke::new(2.).with_caps(Cap::Square), 0.05).unwrap();
    near(cap.bounding_box().min_x(), -1.);
    near(cap.bounding_box().max_x(), 11.);
}
