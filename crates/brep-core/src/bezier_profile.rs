//! Conservative admission of one closed polynomial Bézier boundary.
//! Definitions are retained. Injective spans and shared-endpoint separation are
//! proved from Bernstein control hulls; other pairs use interval intersection.
use crate::{Curve, Error, Result};
const MAX_CURVES: usize = 128;
#[path = "bezier_profile_boolean.rs"]
mod boolean_ops;
pub use boolean_ops::boolean;
#[path = "bezier_profile_normalize.rs"]
mod normalize_ops;
pub use normalize_ops::normalize;
fn refused(message: &str) -> Error {
    Error::new("BREP_UNSUPPORTED_BEZIER_PROFILE", message)
}
fn controls(curve: &Curve) -> Result<Vec<[f64; 2]>> {
    curve.validate()?;
    if ![1, 2, 3].contains(&curve.degree)
        || curve.control_points.len() != curve.degree + 1
        || curve
            .control_points
            .iter()
            .any(|p| p.len() != 2 || p.iter().any(|x| !x.is_finite() || x.abs() > 1e6))
        || (curve.degree != 2 && curve.weights.iter().any(|w| *w != curve.weights[0]))
        || curve.knots[..=curve.degree]
            .iter()
            .any(|t| *t != curve.domain()[0])
        || curve.knots[curve.degree + 1..]
            .iter()
            .any(|t| *t != curve.domain()[1])
    {
        return Err(refused(
            "Bézier admission requires clamped lines, cubic spans or circular rational arcs",
        ));
    }
    if curve.degree == 2 {
        crate::planar_trim::span_green_area(curve, [0., 0.], 1e-7)?;
    }
    Ok(curve.control_points.iter().map(|p| [p[0], p[1]]).collect())
}
fn injective(p: &[[f64; 2]], tolerance: f64) -> bool {
    // Nonnegative Bernstein derivative coefficients, with positive endpoint
    // difference, imply strict coordinate monotonicity in the open span.
    (0..2).any(|axis| {
        let delta = p.last().unwrap()[axis] - p[0][axis];
        delta.abs() > 8. * tolerance
            && p.windows(2).all(|q| {
                if delta > 0. {
                    q[1][axis] >= q[0][axis]
                } else {
                    q[1][axis] <= q[0][axis]
                }
            })
    })
}
fn hull_side(points: &[[f64; 2]], origin: [f64; 2], n: [f64; 2]) -> Option<i8> {
    let mut sign = 0;
    for p in points {
        if *p == origin {
            continue;
        }
        let value = (p[0] - origin[0]) * n[0] + (p[1] - origin[1]) * n[1];
        let error = 32.
            * f64::EPSILON
            * ((p[0].abs() + origin[0].abs()) * n[0].abs()
                + (p[1].abs() + origin[1].abs()) * n[1].abs());
        let next = if value > error {
            1
        } else if value < -error {
            -1
        } else {
            return None;
        };
        if sign != 0 && sign != next {
            return None;
        }
        sign = next;
    }
    (sign != 0).then_some(sign)
}
fn supporting_hulls_separate(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    (0..2).any(|k| {
        let bounds = |p: &[[f64; 2]]| {
            p.iter().fold([f64::INFINITY, f64::NEG_INFINITY], |r, q| {
                [r[0].min(q[k]), r[1].max(q[k])]
            })
        };
        let x = bounds(a);
        let y = bounds(b);
        (x[1] <= y[0] || y[1] <= x[0]) && x[0] < x[1] && y[0] < y[1]
    })
}
fn shared_hulls_separate(a: &[[f64; 2]], b: &[[f64; 2]], point: [f64; 2]) -> bool {
    if supporting_hulls_separate(a, b) {
        return true;
    }
    let other_a = if a[0] == point {
        *a.last().unwrap()
    } else {
        a[0]
    };
    let other_b = if b[0] == point {
        *b.last().unwrap()
    } else {
        b[0]
    };
    let mut candidates = vec![
        [1., 0.],
        [0., 1.],
        [1., 1.],
        [1., -1.],
        [other_b[0] - other_a[0], other_b[1] - other_a[1]],
    ];
    // Candidate normals only suggest a separator. Every control point must
    // still pass the outward-rounding side proof below.
    for x in a {
        for y in b {
            let u = [x[0] - point[0], x[1] - point[1]];
            let v = [y[0] - point[0], y[1] - point[1]];
            let un = u[0].hypot(u[1]);
            let vn = v[0].hypot(v[1]);
            if un > 0. && vn > 0. {
                candidates.push([u[0] / un - v[0] / vn, u[1] / un - v[1] / vn]);
            }
        }
    }
    candidates
        .into_iter()
        .any(|n| matches!((hull_side(a,point,n),hull_side(b,point,n)),(Some(x),Some(y)) if x!=y))
}
fn choose(n: usize, k: usize) -> f64 {
    let mut value = 1.;
    for i in 0..k {
        value *= (n - i) as f64 / (i + 1) as f64;
    }
    value
}
/// Green integral of polynomial Bernstein spans; sign is admitted only outside
/// a conservative floating-point cancellation allowance and the area floor.
pub fn signed_area(wire: &[Curve], tolerance: f64) -> Result<f64> {
    if wire.is_empty() {
        return Err(refused("Empty Bézier boundary"));
    }
    let origin = controls(&wire[0])?[0];
    let mut area = 0.;
    let mut magnitude = 0.;
    let mut scale = 0f64;
    let mut extent = 0f64;
    for curve in wire {
        let raw = controls(curve)?;
        for p in &raw {
            for k in 0..2 {
                scale = scale.max(p[k].abs());
                extent = extent.max((p[k] - origin[k]).abs());
            }
        }
        let p: Vec<_> = raw
            .iter()
            .map(|q| [q[0] - origin[0], q[1] - origin[1]])
            .collect();
        if curve.degree == 2 {
            let contribution = crate::planar_trim::span_green_area(curve, origin, tolerance)?;
            area += contribution;
            magnitude += contribution.abs() + extent * extent;
            continue;
        }
        let n = curve.degree;
        for i in 0..=n {
            for j in 0..n {
                let d = [p[j + 1][0] - p[j][0], p[j + 1][1] - p[j][1]];
                let coefficient = 0.25 * choose(n, i) * choose(n - 1, j) / choose(2 * n - 1, i + j);
                area += coefficient * (p[i][0] * d[1] - p[i][1] * d[0]);
                magnitude +=
                    coefficient.abs() * (p[i][0].abs() * d[1].abs() + p[i][1].abs() * d[0].abs());
            }
        }
    }
    let error = 128. * f64::EPSILON * (magnitude + scale * extent * wire.len() as f64);
    if !area.is_finite() || area.abs() <= error + 64. * tolerance * tolerance {
        return Err(refused("Bézier boundary orientation is unresolved"));
    }
    Ok(area)
}
pub fn validate_single_wire(wire: &[Curve], tolerance: f64) -> Result<f64> {
    if wire.len() < 3 || wire.len() > MAX_CURVES || !tolerance.is_finite() || tolerance <= 0. {
        return Err(refused(
            "Bézier extrusion requires 3–128 spans and a positive tolerance",
        ));
    }
    let points = wire.iter().map(controls).collect::<Result<Vec<_>>>()?;
    for (i, p) in points.iter().enumerate() {
        if !injective(p, tolerance) {
            return Err(refused(
                "Bézier span injectivity is unresolved; split the span at an extremum",
            ));
        }
        if *p.last().unwrap() != points[(i + 1) % points.len()][0] {
            return Err(refused("Bézier boundary endpoints must join exactly"));
        }
    }
    for i in 0..wire.len() {
        for j in i + 1..wire.len() {
            let shared = if j == i + 1 {
                Some(*points[i].last().unwrap())
            } else if i == 0 && j == wire.len() - 1 {
                Some(points[0][0])
            } else {
                None
            };
            if let Some(point) = shared {
                if !shared_hulls_separate(&points[i], &points[j], point) {
                    return Err(refused(
                        "Bézier joint control-hull separation is unresolved",
                    ));
                }
            } else {
                let lift = |curve: &Curve| {
                    let mut c = curve.clone();
                    for p in &mut c.control_points {
                        p.push(0.);
                    }
                    c
                };
                let audit = nurbs_core::intersection::intersect_curve_curve_report(
                    &lift(&wire[i]),
                    &lift(&wire[j]),
                    None,
                )?;
                if !audit.report.components.is_empty() || !audit.report.unresolved.is_empty() {
                    return Err(refused(
                        "Bézier boundary has an intersection or unresolved curve pair",
                    ));
                }
            }
        }
    }
    signed_area(wire, tolerance)
}

/// Containment is admitted by an inscribed convex chord polygon, never by
/// display samples or a bounding box nesting guess. All inner controls must
/// lie strictly inside that polygon; outer spans stay outside their chords.
fn contains_hull(
    outer: &[Vec<[f64; 2]>],
    inner: &[Vec<[f64; 2]>],
    sign: f64,
    tolerance: f64,
) -> bool {
    let vertices: Vec<_> = outer.iter().map(|p| p[0]).collect();
    let side = |a: [f64; 2], b: [f64; 2], p: [f64; 2]| {
        let u = [b[0] - a[0], b[1] - a[1]];
        let v = [p[0] - a[0], p[1] - a[1]];
        let value = sign * (u[0] * v[1] - u[1] * v[0]);
        let error = 64.
            * f64::EPSILON
            * ((b[0].abs() + a[0].abs()) * (p[1].abs() + a[1].abs())
                + (b[1].abs() + a[1].abs()) * (p[0].abs() + a[0].abs()));
        (value, error, 8. * tolerance * u[0].hypot(u[1]))
    };
    for (i, span) in outer.iter().enumerate() {
        let a = vertices[i];
        let b = vertices[(i + 1) % vertices.len()];
        for (j, p) in vertices.iter().enumerate() {
            if j != i && j != (i + 1) % vertices.len() {
                let (v, e, m) = side(a, b, *p);
                if v <= e + m {
                    return false;
                }
            }
        }
        for p in span.iter().skip(1).take(span.len() - 2) {
            // Exact collinearity of an authored control with an endpoint is
            // harmless. Otherwise its outward side must be resolved.
            if *p == a || *p == b {
                continue;
            }
            let (v, e, _) = side(a, b, *p);
            if v != 0. && v >= -e {
                return false;
            }
            // A rounded zero is not a collinearity certificate.
            if v == 0. && ((b[0] != a[0] && p[1] != a[1]) || (b[1] != a[1] && p[0] != a[0])) {
                return false;
            }
        }
        for p in inner.iter().flatten() {
            let (v, e, m) = side(a, b, *p);
            if v <= e + m {
                return false;
            }
        }
    }
    true
}
// A chord may replace a curve for the winding *decision* only when the
// complete control hull homotopy stays separated from the queried point.
// Output geometry always retains the original curves.
fn point_inside(curves: &[Curve], p: [f64; 2], tolerance: f64) -> Result<bool> {
    fn distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
        let u = [b[0] - a[0], b[1] - a[1]];
        let length = u[0] * u[0] + u[1] * u[1];
        let t = if length > 0. {
            ((p[0] - a[0]) * u[0] + (p[1] - a[1]) * u[1]) / length
        } else {
            0.
        }
        .clamp(0., 1.);
        (p[0] - a[0] - u[0] * t).hypot(p[1] - a[1] - u[1] * t)
    }
    let mut stack: Vec<_> = curves.iter().rev().map(|c| (c.clone(), 0usize)).collect();
    let mut winding = 0i32;
    let mut work = 0;
    while let Some((curve, depth)) = stack.pop() {
        let control = controls(&curve)?;
        work += 1;
        if work > 8192 || depth > 40 {
            return Err(refused("Bézier winding separation budget exceeded"));
        }
        let a = control[0];
        let b = *control.last().unwrap();
        let error = control
            .iter()
            .map(|q| distance(*q, a, b))
            .fold(0., f64::max);
        let scale = control
            .iter()
            .flatten()
            .chain(p.iter())
            .map(|v| v.abs())
            .fold(1., f64::max);
        if distance(p, a, b) > error + 8. * tolerance + 256. * f64::EPSILON * scale {
            if (a[1] <= p[1] && b[1] > p[1]) || (a[1] > p[1] && b[1] <= p[1]) {
                let cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
                let round = 128. * f64::EPSILON * scale * scale;
                if cross.abs() <= round {
                    return Err(refused("Bézier winding ray orientation is unresolved"));
                }
                if b[1] > a[1] && cross > 0. {
                    winding += 1;
                } else if b[1] < a[1] && cross < 0. {
                    winding -= 1;
                }
            }
        } else {
            if control.len() == 2 {
                return Err(refused("Bézier containment point is on a boundary"));
            }
            let [a, b] = curve.domain();
            let [left, right] = curve.split((a + b) * 0.5)?;
            stack.push((right, depth + 1));
            stack.push((left, depth + 1));
        }
    }
    Ok(winding.rem_euclid(2) == 1)
}
fn boundaries_separate(a: &[Curve], b: &[Curve], tolerance: f64) -> Result<bool> {
    let mut touching = false;
    let mut work = 0;
    for ca in a {
        for cb in b {
            let pa = controls(ca)?;
            let pb = controls(cb)?;
            if (0..2).any(|axis| {
                let bounds = |p: &Vec<[f64; 2]>| {
                    p.iter().fold([f64::INFINITY, f64::NEG_INFINITY], |r, q| {
                        [r[0].min(q[axis]), r[1].max(q[axis])]
                    })
                };
                let x = bounds(&pa);
                let y = bounds(&pb);
                x[1] + 8. * tolerance < y[0] || y[1] + 8. * tolerance < x[0]
            }) {
                continue;
            }
            if supporting_hulls_separate(&pa, &pb) {
                touching |= [pa[0], *pa.last().unwrap()].iter().any(|p| {
                    [pb[0], *pb.last().unwrap()]
                        .iter()
                        .any(|q| (p[0] - q[0]).hypot(p[1] - q[1]) <= 8. * tolerance)
                });
                continue;
            }
            work += 1;
            if work > 8192 {
                return Err(refused("Bézier boundary separation budget exceeded"));
            }
            let lift = |c: &Curve| {
                let mut c = c.clone();
                for p in &mut c.control_points {
                    p.push(0.);
                }
                c
            };
            let audit =
                nurbs_core::intersection::intersect_curve_curve_report(&lift(ca), &lift(cb), None)?;
            if !audit.report.unresolved.is_empty() {
                return Err(refused("Bézier boundary separation is unresolved"));
            }
            for event in audit.report.components {
                let endpoint = |c: &Curve, t: f64| {
                    c.domain()
                        .iter()
                        .any(|u| (t - u).abs() <= 1e-9 * (c.domain()[1] - c.domain()[0]))
                };
                if event.kind != nurbs_core::intersection::CurveCurveComponentKind::Point
                    || !endpoint(ca, event.first)
                    || !endpoint(cb, event.second)
                {
                    return Err(refused(
                        "Bézier boundaries intersect away from their endpoints",
                    ));
                }
                touching = true;
            }
        }
    }
    Ok(touching)
}

pub fn classify_components(
    wires: &[Vec<Curve>],
    tolerance: f64,
) -> Result<(Vec<(usize, Vec<usize>)>, Vec<usize>)> {
    if wires.is_empty() || wires.len() > 32 {
        return Err(refused("Bézier profiles require 1–32 boundaries"));
    }
    let areas = wires
        .iter()
        .map(|w| validate_single_wire(w, tolerance))
        .collect::<Result<Vec<_>>>()?;
    let points = wires
        .iter()
        .map(|w| w.iter().map(controls).collect::<Result<Vec<_>>>())
        .collect::<Result<Vec<_>>>()?;
    let mut inside = vec![vec![false; wires.len()]; wires.len()];
    for i in 0..wires.len() {
        for j in i + 1..wires.len() {
            let a = contains_hull(&points[i], &points[j], areas[i].signum(), tolerance);
            let b = contains_hull(&points[j], &points[i], areas[j].signum(), tolerance);
            if a && b {
                return Err(refused("Ambiguous Bézier containment"));
            }
            if a {
                inside[j][i] = true;
            } else if b {
                inside[i][j] = true;
            } else {
                let bounds = |p: &[Vec<[f64; 2]>], axis: usize| {
                    p.iter()
                        .flatten()
                        .fold([f64::INFINITY, f64::NEG_INFINITY], |mut b, q| {
                            b[0] = b[0].min(q[axis]);
                            b[1] = b[1].max(q[axis]);
                            b
                        })
                };
                let disjoint = (0..2).any(|axis| {
                    let a = bounds(&points[i], axis);
                    let b = bounds(&points[j], axis);
                    a[1] + 8. * tolerance < b[0] || b[1] + 8. * tolerance < a[0]
                });
                if !disjoint {
                    let touching = boundaries_separate(&wires[i], &wires[j], tolerance)?;
                    let representative = |wire: &[Curve]| -> Result<[f64; 2]> {
                        let c = &wire[0];
                        let [a, b] = c.domain();
                        let p = c.evaluate((a + b) * 0.5)?.point;
                        Ok([p[0], p[1]])
                    };
                    let a = point_inside(&wires[i], representative(&wires[j])?, tolerance)?;
                    let b = point_inside(&wires[j], representative(&wires[i])?, tolerance)?;
                    if touching && (a || b) {
                        return Err(refused("A hole cannot touch its containing boundary"));
                    }
                    if a && b {
                        return Err(refused("Ambiguous Bézier containment"));
                    }
                    inside[j][i] = a;
                    inside[i][j] = b;
                }
            }
        }
    }
    let depths: Vec<_> = inside
        .iter()
        .map(|row| row.iter().filter(|v| **v).count())
        .collect();
    let components = (0..wires.len())
        .filter(|i| depths[*i] % 2 == 0)
        .map(|i| {
            (
                i,
                (0..wires.len())
                    .filter(|j| inside[*j][i] && depths[*j] == depths[i] + 1)
                    .collect(),
            )
        })
        .collect();
    Ok((components, depths))
}

#[cfg(test)]
mod tests {
    use planar_geometry::path::{BezierPath, PathSegment};
    fn concave_path() -> BezierPath {
        BezierPath {
            start: [0., 0.],
            closed: true,
            segments: vec![
                PathSegment::Cubic {
                    c1: [2., 0.],
                    c2: [4., 0.],
                    to: [6., 0.],
                },
                PathSegment::Line { to: [6., 2.] },
                PathSegment::Line { to: [2., 2.] },
                PathSegment::Line { to: [2., 6.] },
                PathSegment::Line { to: [0., 6.] },
                PathSegment::Line { to: [0., 0.] },
            ],
        }
    }
    #[test]
    fn partial_collinear_cubic_overlap_keeps_union_and_difference_area() {
        let rectangle = |x: f64| {
            crate::sketch::bezier_wire(BezierPath {
                start: [x, 0.],
                closed: true,
                segments: vec![
                    PathSegment::Cubic {
                        c1: [x + 2., 0.],
                        c2: [x + 4., 0.],
                        to: [x + 6., 0.],
                    },
                    PathSegment::Line { to: [x + 6., 4.] },
                    PathSegment::Line { to: [x, 4.] },
                    PathSegment::Line { to: [x, 0.] },
                ],
            })
            .unwrap()
        };
        let a = vec![rectangle(0.)];
        let b = vec![rectangle(2.)];
        for (op, area) in [("union", 32.), ("intersection", 16.), ("difference", 8.)] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            assert!((super::validate_profile(&result, 1e-7).unwrap() - area).abs() < 1e-5);
        }
    }
    #[test]
    fn external_endpoint_tangency_has_empty_intersection() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 5.).unwrap()).unwrap(),
        ];
        let b = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([10., 0.], 5.).unwrap()).unwrap(),
        ];
        let united = super::boolean(&a, &b, "union", 1e-7).unwrap();
        assert_eq!(united.len(), 2);
        assert!(
            super::boolean(&a, &b, "intersection", 1e-7)
                .unwrap()
                .is_empty()
        );
        let result = super::boolean(&a, &b, "difference", 1e-7).unwrap();
        assert!(
            (super::validate_profile(&result, 1e-7).unwrap()
                - super::signed_area(&a[0], 1e-7).unwrap().abs())
            .abs()
                < 1e-5
        );
    }
    #[test]
    fn self_crossing_even_odd_profile_splits_into_retained_components() {
        let path = BezierPath {
            start: [0., 0.],
            closed: true,
            segments: vec![
                PathSegment::Cubic {
                    c1: [4. / 3., 4. / 3.],
                    c2: [8. / 3., 8. / 3.],
                    to: [4., 4.],
                },
                PathSegment::Line { to: [0., 4.] },
                PathSegment::Line { to: [4., 0.] },
                PathSegment::Line { to: [0., 0.] },
            ],
        };
        let wire = crate::sketch::bezier_wire(path).unwrap();
        let result = super::normalize(&[wire], 1e-7).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().flatten().any(|c| c.degree == 3));
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 8.).abs() < 1e-6);
        crate::prism::extrude(&result, 0., 2.)
            .unwrap()
            .validate()
            .unwrap();
    }
    #[test]
    fn mixed_cubic_and_rational_circle_boolean_retains_both_carriers() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 5.).unwrap()).unwrap(),
        ];
        let mut circle = crate::sketch::circle_wire(5.).unwrap();
        for c in &mut circle {
            for p in &mut c.control_points {
                p[0] += 3.;
            }
        }
        let b = vec![circle];
        for op in ["union", "intersection", "difference"] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            assert!(result.iter().flatten().any(|c| c.degree == 3));
            assert!(result.iter().flatten().any(|c| c.degree == 2));
            assert!(super::validate_profile(&result, 1e-7).unwrap() > 0.);
            crate::prism::extrude(&result, 0., 2.)
                .unwrap()
                .validate()
                .unwrap();
        }
    }
    #[test]
    fn transverse_bezier_booleans_retain_cubic_fragments_and_close_solids() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 5.).unwrap()).unwrap(),
        ];
        let b = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([3., 0.], 5.).unwrap()).unwrap(),
        ];
        let mut areas = Vec::new();
        for op in ["union", "intersection", "difference"] {
            let loops = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            let area = super::validate_profile(&loops, 1e-7).unwrap();
            areas.push(area);
            assert!(loops.iter().flatten().all(|c| c.degree == 3));
            let solid = crate::prism::extrude(&loops, 0., 2.).unwrap();
            solid.validate().unwrap();
        }
        let original = super::signed_area(&a[0], 1e-7).unwrap().abs();
        assert!((areas[0] + areas[1] - 2. * original).abs() < 1e-5);
        assert!((areas[2] + areas[1] - original).abs() < 1e-5);
        assert!(super::boolean(&a, &b, "xor", 1e-7).is_err());
    }
    #[test]
    fn identical_bezier_booleans_do_not_duplicate_boundaries() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 5.).unwrap()).unwrap(),
        ];
        assert_eq!(super::boolean(&a, &a, "union", 1e-7).unwrap()[0].len(), 4);
        assert!(
            super::boolean(&a, &a, "difference", 1e-7)
                .unwrap()
                .is_empty()
        );
        assert!(super::boolean(&a, &a, "xor", 1e-7).unwrap().is_empty());
    }
    #[test]
    fn concave_holes_use_separated_hull_winding_without_replacing_curves() {
        let hole = BezierPath::from_circle([1., 4.], 0.3).unwrap();
        let solid =
            crate::sketch::extrude_bezier_profiles(vec![concave_path(), hole], 2., 0., None)
                .unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.bodies.len(), 1);
        assert!(solid.faces.iter().any(|f| f.holes.len() == 1));
        assert_eq!(
            solid.edges.iter().filter(|e| e.curve.degree == 3).count(),
            10
        );
    }
    #[test]
    fn overlapping_bounds_do_not_imply_nested_or_intersecting_profiles() {
        let outside = BezierPath::from_circle([4., 4.], 0.5).unwrap();
        let solid =
            crate::sketch::extrude_bezier_profiles(vec![concave_path(), outside], 2., 0., None)
                .unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.bodies.len(), 2);
        assert!(solid.faces.iter().all(|f| f.holes.is_empty()));
    }
    #[test]
    fn edited_smooth_joint_uses_proved_adaptive_separator() {
        let path = BezierPath::from_polygon(&[[0., 0.], [20., 0.], [20., 20.], [0., 20.]], true)
            .unwrap()
            .smooth()
            .unwrap()
            .set_anchor_position(1, [30., 0.])
            .unwrap();
        let solid =
            crate::sketch::extrude(crate::sketch::Profile::Bezier(path), true, 5., 0., None)
                .unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.faces.len(), 6);
    }
    #[test]
    fn compound_holes_and_nested_islands_keep_exact_curves() {
        let paths = [10., 4., 1.].map(|r| BezierPath::from_circle([0., 0.], r).unwrap());
        let solid = crate::sketch::extrude_bezier_profiles(paths.into(), -3., 1., None).unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.bodies.len(), 2);
        assert_eq!(solid.faces.len(), 16);
        assert_eq!(
            solid.edges.iter().filter(|e| e.curve.degree == 3).count(),
            24
        );
        assert!(solid.faces.iter().any(|f| f.holes.len() == 1));
        assert!(
            solid
                .vertices
                .iter()
                .all(|v| v.point[2] == -2. || v.point[2] == 1.)
        );
    }
    #[test]
    fn compound_contacts_and_unproved_containment_are_refused() {
        let a = BezierPath::from_circle([0., 0.], 10.).unwrap();
        let b = BezierPath::from_circle([8., 0.], 3.).unwrap();
        assert!(crate::sketch::extrude_bezier_profiles(vec![a, b], 2., 0., None).is_err());
    }
    #[test]
    fn exact_cubic_extrusion_retains_controls_and_cap_trims() {
        let path = BezierPath::from_circle([0., 0.], 10.).unwrap().reverse();
        let solid = crate::sketch::extrude(
            crate::sketch::Profile::Bezier(path.clone()),
            true,
            5.,
            0.,
            None,
        )
        .unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.faces.len(), 6);
        let cubic = solid.edges.iter().find(|e| e.curve.degree == 3).unwrap();
        let PathSegment::Cubic { c1, c2, to } = path.segments[0] else {
            panic!()
        };
        assert_eq!(
            cubic.curve.control_points,
            vec![
                vec![path.start[0], path.start[1], 0.],
                vec![c1[0], c1[1], 0.],
                vec![c2[0], c2[1], 0.],
                vec![to[0], to[1], 0.]
            ]
        );
        assert!(
            solid
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .any(|e| e.pcurve.degree == 3)
        );
    }
    #[test]
    fn accepts_clockwise_path_and_negative_height() {
        let path = BezierPath::from_circle([0., 0.], 2.).unwrap();
        let solid =
            crate::sketch::extrude(crate::sketch::Profile::Bezier(path), true, -3., 1., None)
                .unwrap();
        assert!(
            solid
                .vertices
                .iter()
                .all(|v| v.point[2] == 1. || v.point[2] == -2.)
        );
    }
    #[test]
    fn rejects_crossing_and_unresolved_boundaries_without_chording() {
        let path = BezierPath::from_polygon(&[[0., 0.], [10., 10.], [0., 10.], [10., 0.]], true)
            .unwrap()
            .smooth()
            .unwrap();
        assert!(
            crate::sketch::extrude(crate::sketch::Profile::Bezier(path), true, 3., 0., None)
                .is_err()
        );
    }
}

/// Normalize nested polynomial boundaries to material-left winding.
pub fn orient_even_odd(wires: &[Vec<Curve>], tolerance: f64) -> Result<Vec<Vec<Curve>>> {
    if wires.is_empty() {
        return Ok(Vec::new());
    }
    let (_, depth) = classify_components(wires, tolerance)?;
    wires
        .iter()
        .zip(depth)
        .map(|(wire, depth)| {
            let area = validate_single_wire(wire, tolerance)?;
            if (area > 0.) != (depth % 2 == 0) {
                wire.iter().rev().map(Curve::reverse).collect()
            } else {
                Ok(wire.clone())
            }
        })
        .collect()
}
pub fn validate_profile(wires: &[Vec<Curve>], tolerance: f64) -> Result<f64> {
    if wires.is_empty() {
        return Ok(0.);
    }
    let (_, depth) = classify_components(wires, tolerance)?;
    let mut area = 0.;
    for (wire, depth) in wires.iter().zip(depth) {
        let a = validate_single_wire(wire, tolerance)?;
        if (a > 0.) != (depth % 2 == 0) {
            return Err(refused("Bézier profile requires material-left winding"));
        }
        area += a;
    }
    Ok(area)
}
