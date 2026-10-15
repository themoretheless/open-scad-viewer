//! Conservative admission of positive-weight rational Bézier boundaries.
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
#[path = "bezier_profile_coincidence.rs"]
mod coincidence_ops;
#[path = "bezier_profile_departure.rs"]
mod departure_ops;
#[path = "bezier_profile_joint.rs"]
mod joint_ops;
#[path = "bezier_profile_area.rs"]
mod area_ops;
/// Keep the specialized line/circle arrangement where it applies; route
/// polynomial quadratics, general conics and rational cubics to this kernel.
pub fn requires_general(curve: &Curve, tolerance: f64) -> bool {
    curve.degree == 3 || (curve.degree == 2 &&
        crate::planar_trim::span_green_area(curve, [0.,0.], tolerance).is_err())
}
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
        || curve.periodic
        || curve.knots[..=curve.degree]
            .iter()
            .any(|t| *t != curve.domain()[0])
        || curve.knots[curve.degree + 1..]
            .iter()
            .any(|t| *t != curve.domain()[1])
    {
        return Err(refused(
            "Bézier admission requires clamped positive-weight planar spans of degree 1..3",
        ));
    }
    Ok(curve.control_points.iter().map(|p| [p[0], p[1]]).collect())
}
fn injective(c: &Curve, tolerance: f64) -> bool {
    (0..2).any(|axis| {
        let delta = c.control_points.last().unwrap()[axis] - c.control_points[0][axis];
        delta.abs() > 8. * tolerance && nurbs_core::intersection::planar_scalar::monotone(c, axis)
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
    // A positive-weight Bernstein span with one endpoint off a supporting
    // plane has its complete open interior off that plane, even at a cusp.
    for k in 0..2 {
        for (flat, curved, other) in [(a, b, other_b), (b, a, other_a)] {
            let plane = point[k];
            if flat.iter().all(|p| p[k] == plane)
                && ((other[k] > plane && curved.iter().all(|p| p[k] >= plane))
                    || (other[k] < plane && curved.iter().all(|p| p[k] <= plane)))
            {
                return true;
            }
        }
    }
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
        || joint_ops::shared_line_separates(a, b, point)
        || joint_ops::opposite_halfplanes(a, b, point)
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
    if wire.is_empty() || !tolerance.is_finite() || tolerance <= 0. {
        return Err(refused("Empty Bézier boundary"));
    }
    let origin = controls(&wire[0])?[0];
    let mut area = 0.;
    let mut magnitude = 0.;
    let mut scale = 0f64;
    let mut extent = 0f64;
    let mut integration_error = 0.;
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
        if curve.degree > 1 && curve.weights.iter().any(|w| *w != curve.weights[0]) {
            if curve.degree == 2 {
                if let Ok(contribution) = crate::planar_trim::span_green_area(curve, origin, tolerance) {
                    area += contribution;
                    magnitude += contribution.abs() + extent * extent;
                    continue;
                }
            }
            let bounds = area_ops::integral(curve,origin,
                (tolerance * extent.max(1.) * 0.125).max(512.*f64::EPSILON*extent.max(1.).powi(2)))?;
            let contribution = bounds.lo + (bounds.hi-bounds.lo)*0.5;
            area += contribution;
            integration_error += (contribution-bounds.lo).max(bounds.hi-contribution).next_up();
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
    let error = integration_error + 128. * f64::EPSILON * (magnitude + scale * extent * wire.len() as f64);
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
        if !injective(&wire[i], tolerance) {
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
                if !shared_hulls_separate(&points[i], &points[j], point)
                    && !joint_ops::exact_endpoint_only(&wire[i], &wire[j], point, tolerance)?
                {
                    return Err(refused(
                        "Bézier joint control-hull separation is unresolved",
                    ));
                }
            } else {
                if let Some(nurbs_core::intersection::planar_scalar::Relation::Points(points)) =
                    nurbs_core::intersection::planar_scalar::intersect(
                        &wire[i], &wire[j], tolerance,
                    )?
                {
                    if points.is_empty() {
                        continue;
                    }
                    return Err(refused("Bézier boundary has an intersection"));
                }
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
            if [pa[0], *pa.last().unwrap()].iter().any(|p| {
                [pb[0], *pb.last().unwrap()]
                    .iter()
                    .any(|q| p == q && shared_hulls_separate(&pa, &pb, *p))
            }) {
                touching = true;
                continue;
            }
            work += 1;
            if work > 8192 {
                return Err(refused("Bézier boundary separation budget exceeded"));
            }
            let endpoint = |c: &Curve, t: f64| {
                c.domain()
                    .iter()
                    .any(|u| (t - u).abs() <= 1e-9 * (c.domain()[1] - c.domain()[0]))
            };
            if let Some(relation) =
                nurbs_core::intersection::planar_scalar::intersect(ca, cb, tolerance)?
            {
                match relation {
                    nurbs_core::intersection::planar_scalar::Relation::Points(points) => {
                        if points
                            .iter()
                            .any(|p| !endpoint(ca, p.first) || !endpoint(cb, p.second))
                        {
                            return Err(refused(
                                "Bézier boundaries intersect away from their endpoints",
                            ));
                        }
                        touching |= !points.is_empty();
                        continue;
                    }
                    _ => return Err(refused("Bézier boundaries overlap")),
                }
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
    fn curved_partial_overlap_boolean_retains_source_cubics() {
        let top = crate::Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 2.], vec![2., 1.], vec![3., 3.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let region = |c: crate::Curve| {
            let a = c.control_points[0].clone();
            let b = c.control_points.last().unwrap().clone();
            vec![
                crate::Curve::from_polyline(vec![vec![a[0], -2.], vec![b[0], -2.]]).unwrap(),
                crate::Curve::from_polyline(vec![vec![b[0], -2.], b]).unwrap(),
                c.reverse().unwrap(),
                crate::Curve::from_polyline(vec![a.clone(), vec![a[0], -2.]]).unwrap(),
            ]
        };
        let a = vec![region(top.clone())];
        let b = vec![region(top.trim(0.25, 0.75).unwrap())];
        let aa = super::validate_profile(&a, 1e-7).unwrap();
        let bb = super::validate_profile(&b, 1e-7).unwrap();
        for (op, expected) in [("union", aa), ("intersection", bb), ("difference", aa - bb)] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            assert!((super::validate_profile(&result, 1e-7).unwrap() - expected).abs() < 1e-5);
            assert!(result.iter().flatten().any(|c| c.degree == 3));
        }
        let normalized = super::normalize(&[a[0].clone(), b[0].clone()], 1e-7).unwrap();
        assert_eq!(normalized.len(), 2);
        assert!((super::validate_profile(&normalized, 1e-7).unwrap() - (aa - bb)).abs() < 1e-5);
    }
    #[test]
    fn partial_curved_coincidence_with_different_injective_axes_keeps_material() {
        let c = crate::Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![6., -2.], vec![11., -1.], vec![7., 11.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let ac = c.trim(0., 0.625).unwrap();
        let bc = c.trim(0.375, 1.).unwrap();
        let line = |a: Vec<f64>, b: Vec<f64>| crate::Curve::from_polyline(vec![a, b]).unwrap();
        let ap = ac.control_points[0].clone();
        let az = ac.control_points.last().unwrap().clone();
        let bp = bc.control_points[0].clone();
        let bz = bc.control_points.last().unwrap().clone();
        let a = vec![vec![
            line(vec![ap[0], -3.], vec![az[0], -3.]),
            line(vec![az[0], -3.], az),
            ac.reverse().unwrap(),
            line(ap.clone(), vec![ap[0], -3.]),
        ]];
        let b = vec![vec![
            line(vec![-2., bp[1]], bp),
            bc,
            line(bz.clone(), vec![-2., bz[1]]),
            line(vec![-2., bz[1]], vec![-2., -0.5625]),
        ]];
        let aa = super::validate_profile(&a, 1e-7).unwrap();
        let bb = super::validate_profile(&b, 1e-7).unwrap();
        let union = super::boolean(&a, &b, "union", 1e-7).unwrap();
        let intersection = super::boolean(&a, &b, "intersection", 1e-7).unwrap();
        let difference = super::boolean(&a, &b, "difference", 1e-7).unwrap();
        let ia = super::validate_profile(&intersection, 1e-7).unwrap();
        assert!((super::validate_profile(&union, 1e-7).unwrap() + ia - aa - bb).abs() < 1e-5);
        assert!((super::validate_profile(&difference, 1e-7).unwrap() + ia - aa).abs() < 1e-5);
        crate::prism::extrude(&union, 0., 2.)
            .unwrap()
            .validate()
            .unwrap();
        let xy = |t: f64| {
            [
                18. * t - 3. * t * t - 8. * t * t * t,
                -6. * t + 9. * t * t + 8. * t * t * t,
            ]
        };
        let inverse = |axis: usize, value: f64, mut lo: f64, mut hi: f64| {
            for _ in 0..60 {
                let mid = (lo + hi) * 0.5;
                if xy(mid)[axis] < value {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            (lo + hi) * 0.5
        };
        for i in 0..20 {
            for j in 0..20 {
                let p = [-2. + i as f64 * 0.57 + 0.137, -3. + j as f64 * 0.69 + 0.193];
                let ina = p[0] > 0.
                    && p[0] < 8.125
                    && p[1] > -3.
                    && p[1] < xy(inverse(0, p[0], 0., 0.625))[1];
                let inb = p[1] > -0.5625
                    && p[1] < 11.
                    && p[0] > -2.
                    && p[0] < xy(inverse(1, p[1], 0.375, 1.))[0];
                for (result, expected) in [
                    (&union, ina || inb),
                    (&intersection, ina && inb),
                    (&difference, ina && !inb),
                ] {
                    let spans = result.iter().flatten().cloned().collect::<Vec<_>>();
                    assert_eq!(
                        super::point_inside(&spans, p, 1e-7).unwrap(),
                        expected,
                        "{p:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn tangent_cusp_components_use_departure_order_instead_of_equal_tangents() {
        let lobe = |sign: f64| {
            crate::sketch::bezier_wire(BezierPath {
                start: [0., 0.],
                closed: true,
                segments: vec![
                    PathSegment::Cubic {
                        c1: [sign * 2. / 3., 0.],
                        c2: [sign * 4. / 3., 4. / 3.],
                        to: [sign * 2., 4.],
                    },
                    PathSegment::Line {
                        to: [sign * 2., -4.],
                    },
                    PathSegment::Cubic {
                        c1: [sign * 4. / 3., -4. / 3.],
                        c2: [sign * 2. / 3., 0.],
                        to: [0., 0.],
                    },
                ],
            })
            .unwrap()
        };
        let mut source = lobe(1.);
        source.extend(lobe(-1.));
        let result = super::normalize(&[source], 1e-7).unwrap();
        assert_eq!(result.len(), 2);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 32. / 3.).abs() < 1e-5);
        assert!(result.iter().flatten().filter(|c| c.degree == 3).count() >= 4);
    }
    #[test]
    fn zero_first_jets_at_self_touching_cusps_keep_separate_regions() {
        let lobe = |sign: f64| {
            crate::sketch::bezier_wire(BezierPath {
                start: [0., 0.],
                closed: true,
                segments: vec![
                    PathSegment::Cubic {
                        c1: [0., 0.],
                        c2: [sign, 0.],
                        to: [sign * 2., 4.],
                    },
                    PathSegment::Line {
                        to: [sign * 2., -4.],
                    },
                    PathSegment::Cubic {
                        c1: [sign, 0.],
                        c2: [0., 0.],
                        to: [0., 0.],
                    },
                ],
            })
            .unwrap()
        };
        let mut source = lobe(1.);
        source.extend(lobe(-1.));
        let result = super::normalize(&[source], 1e-7).unwrap();
        assert_eq!(result.len(), 2);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 11.2).abs() < 1e-5);
        crate::prism::extrude(&result, 0., 2.)
            .unwrap()
            .validate()
            .unwrap();
    }
    #[test]
    fn diagonal_self_touching_cusps_keep_the_same_topology_after_affine_transform() {
        let lobe = |sign: f64| {
            crate::sketch::bezier_wire(BezierPath {
                start: [0., 0.],
                closed: true,
                segments: vec![
                    PathSegment::Cubic {
                        c1: [0., 0.],
                        c2: [sign, 0.],
                        to: [sign * 2., 4.],
                    },
                    PathSegment::Line {
                        to: [sign * 2., -4.],
                    },
                    PathSegment::Cubic {
                        c1: [sign, 0.],
                        c2: [0., 0.],
                        to: [0., 0.],
                    },
                ],
            })
            .unwrap()
        };
        let mut wire = lobe(1.);
        wire.extend(lobe(-1.));
        for c in &mut wire {
            for p in &mut c.control_points {
                let [x, y] = [p[0], p[1]];
                p[0] = 17. + x - y;
                p[1] = -9. + x + y;
            }
        }
        let result = super::normalize(&[wire], 1e-7).unwrap();
        assert_eq!(result.len(), 2);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 22.4).abs() < 1e-5);
    }
    #[test]
    fn monotone_cubic_with_reversing_controls_keeps_exact_profile_and_overlap() {
        use crate::Curve;
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.1], vec![0.9, 0.8], vec![1.9, 2.1]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let region = |c: &Curve| {
            let a = c.control_points[0].clone();
            let b = c.control_points.last().unwrap().clone();
            let line = |a, b| Curve::from_polyline(vec![a, b]).unwrap();
            vec![
                line(vec![a[0], -1.], vec![b[0], -1.]),
                line(vec![b[0], -1.], b),
                c.reverse().unwrap(),
                line(a.clone(), vec![a[0], -1.]),
            ]
        };
        let a = vec![region(&c)];
        assert!((super::validate_profile(&a, 1e-7).unwrap() - 3.811).abs() < 1e-10);
        let normalized = super::normalize(&a, 1e-7).unwrap();
        assert!((super::validate_profile(&normalized, 1e-7).unwrap() - 3.811).abs() < 1e-10);
        crate::prism::extrude(&normalized, 0., 1.5)
            .unwrap()
            .validate()
            .unwrap();
        let b = vec![region(&c.trim(0.25, 0.75).unwrap())];
        let aa = super::validate_profile(&a, 1e-7).unwrap();
        let bb = super::validate_profile(&b, 1e-7).unwrap();
        for (op, area) in [("union", aa), ("intersection", bb), ("difference", aa - bb)] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap();
            assert!((super::validate_profile(&result, 1e-7).unwrap() - area).abs() < 1e-6);
            assert!(result.iter().flatten().any(|c| c.degree == 3));
        }
    }
    #[test]
    fn affine_curved_tangency_boolean_preserves_both_retained_carriers() {
        use crate::Curve;
        let region = |ys: [f64; 4]| {
            let c = Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
                control_points: (0..4).map(|i| vec![10. * i as f64, ys[i]]).collect(),
                weights: vec![1.; 4],
                periodic: false,
            };
            let line = |a, b| Curve::from_polyline(vec![a, b]).unwrap();
            let mut wire = vec![
                line(vec![0., -2.], vec![30., -2.]),
                line(vec![30., -2.], vec![30., ys[3]]),
                c.reverse().unwrap(),
                line(vec![0., ys[0]], vec![0., -2.]),
            ];
            for c in &mut wire {
                for p in &mut c.control_points {
                    let [x, y] = [p[0], p[1]];
                    p[0] = 17. + x - y;
                    p[1] = -9. + x + y;
                }
            }
            wire
        };
        let a = vec![region([0., 0., 1., 3.])];
        let b = vec![region([1., -1., 1., 7.])];
        assert!((super::validate_profile(&a, 1e-7).unwrap() - 180.).abs() < 1e-6);
        assert!((super::validate_profile(&b, 1e-7).unwrap() - 240.).abs() < 1e-6);
        for (operation, area) in [("union", 240.), ("intersection", 180.), ("difference", 0.)] {
            let result = super::boolean(&a, &b, operation, 1e-7).unwrap();
            assert!((super::validate_profile(&result, 1e-7).unwrap() - area).abs() < 1e-5);
            if area > 0. {
                assert!(result.iter().flatten().any(|c| c.degree == 3));
            }
        }
    }
    #[test]
    fn interior_semicubical_cusp_splits_original_curve_without_chords() {
        let c = crate::Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![3., -1.], vec![-1., 1.], vec![-1., -1.], vec![3., 1.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let line = crate::Curve::from_polyline(vec![vec![3., 1.], vec![3., -1.]]).unwrap();
        let result = super::normalize(&[vec![c, line]], 1e-7).unwrap();
        assert_eq!(result.len(), 1);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 2.4).abs() < 1e-5);
        assert_eq!(result[0].iter().filter(|c| c.degree == 3).count(), 2);
        crate::prism::extrude(&result, 0., 2.)
            .unwrap()
            .validate()
            .unwrap();
    }
    #[test]
    fn non_dyadic_cusp_keeps_its_material_area() {
        let c = crate::Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![1., -1.], vec![-1., 2.], vec![0., -4.], vec![4., 8.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let line = crate::Curve::from_polyline(vec![vec![4., 8.], vec![1., -1.]]).unwrap();
        let result = super::normalize(&[vec![c, line]], 1e-7).unwrap();
        assert_eq!(result.len(), 1);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 2.7).abs() < 1e-5);
    }
    #[test]
    fn a_retraced_spur_cancels_without_removing_the_curved_region() {
        let mut wire =
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 2.).unwrap()).unwrap();
        let expected = super::validate_single_wire(&wire, 1e-7).unwrap().abs();
        let spur = crate::Curve::from_polyline(vec![vec![2., 0.], vec![3., 1.]]).unwrap();
        wire.extend([spur.clone(), spur.reverse().unwrap()]);
        let result = super::normalize(&[wire], 1e-7).unwrap();
        assert_eq!(result.len(), 1);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - expected).abs() < 1e-5);
    }
    #[test]
    fn constant_cubic_passes_have_no_even_odd_material() {
        let point = crate::Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![2., 0.]; 4],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(
            super::normalize(&[vec![point.clone()]], 1e-7)
                .unwrap()
                .is_empty()
        );
        let mut circle =
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 2.).unwrap()).unwrap();
        let expected = super::validate_single_wire(&circle, 1e-7).unwrap().abs();
        circle.push(point);
        let result = super::normalize(&[circle], 1e-7).unwrap();
        assert!((super::validate_profile(&result, 1e-7).unwrap() - expected).abs() < 1e-5);
    }
    #[test]
    fn retraced_straight_cubic_with_nonlinear_speed_cancels_a_line() {
        let a = crate::sketch::bezier_wire(BezierPath {
            start: [0., 0.],
            closed: true,
            segments: vec![
                PathSegment::Cubic {
                    c1: [0.5, 0.],
                    c2: [2.5, 0.],
                    to: [3., 0.],
                },
                PathSegment::Line { to: [3., 2.] },
                PathSegment::Line { to: [0., 2.] },
                PathSegment::Line { to: [0., 0.] },
            ],
        })
        .unwrap();
        let b = crate::sketch::bezier_wire(
            BezierPath::from_polygon(&[[0., 0.], [3., 0.], [3., 2.], [0., 2.]], true).unwrap(),
        )
        .unwrap();
        assert!(
            super::normalize(&[a.clone(), b.clone()], 1e-7)
                .unwrap()
                .is_empty()
        );
        let result = super::normalize(&[a, b.clone(), b], 1e-7).unwrap();
        assert_eq!(result.len(), 1);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 6.).abs() < 1e-5);
    }
    #[test]
    fn repeated_curved_boundaries_cancel_by_even_odd_multiplicity() {
        let a = crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 2.).unwrap()).unwrap();
        let expected = super::validate_single_wire(&a, 1e-7).unwrap().abs();
        assert!(
            super::normalize(&[a.clone(), a.clone()], 1e-7)
                .unwrap()
                .is_empty()
        );
        let reversed = a
            .iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect::<Vec<_>>();
        let result = super::normalize(&[a.clone(), reversed, a.clone()], 1e-7).unwrap();
        assert_eq!(result.len(), 1);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - expected).abs() < 1e-5);
    }
    #[test]
    fn third_order_contact_boolean_preserves_cubic_cusp_and_area() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath {
                start: [0., -2.],
                closed: true,
                segments: vec![
                    PathSegment::Line { to: [3., -2.] },
                    PathSegment::Line { to: [3., 1.] },
                    PathSegment::Cubic {
                        c1: [2., -1.],
                        c2: [1., 1.],
                        to: [0., -1.],
                    },
                    PathSegment::Line { to: [0., -2.] },
                ],
            })
            .unwrap(),
        ];
        let b = vec![
            crate::sketch::bezier_wire(
                BezierPath::from_polygon(&[[0., -2.], [3., -2.], [3., 0.], [0., 0.]], true)
                    .unwrap(),
            )
            .unwrap(),
        ];
        for (op, expected) in [
            ("union", 6.375),
            ("intersection", 5.625),
            ("difference", 0.375),
        ] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            let area = super::validate_profile(&result, 1e-7).unwrap();
            assert!((area - expected).abs() < 1e-5, "{op}: {area}");
            assert!(result.iter().flatten().any(|c| c.degree == 3));
        }
    }
    #[test]
    fn partial_line_cubic_overlap_deduplicates_different_degrees() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath {
                start: [0., 0.],
                closed: true,
                segments: vec![
                    PathSegment::Cubic {
                        c1: [2., 0.],
                        c2: [4., 0.],
                        to: [6., 0.],
                    },
                    PathSegment::Line { to: [6., 4.] },
                    PathSegment::Line { to: [0., 4.] },
                    PathSegment::Line { to: [0., 0.] },
                ],
            })
            .unwrap(),
        ];
        let b = vec![
            crate::sketch::bezier_wire(
                BezierPath::from_polygon(&[[2., 0.], [8., 0.], [8., 4.], [2., 4.]], true).unwrap(),
            )
            .unwrap(),
        ];
        for (op, expected) in [("union", 32.), ("intersection", 16.), ("difference", 8.)] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            assert!((super::validate_profile(&result, 1e-7).unwrap() - expected).abs() < 1e-5);
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

#[cfg(test)]
mod implicit_joint_tests {
    use super::*;
    fn cubic(p: Vec<Vec<f64>>) -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: p,
            weights: vec![1.; 4],
            periodic: false,
        }
    }
    #[test]
    fn same_side_curved_cusp_lens_with_nonlinear_speed_keeps_exact_carriers() {
        let a = cubic(vec![vec![0., 0.], vec![2., 0.], vec![4., 2.], vec![6., 6.]]);
        let b = cubic(vec![
            vec![0., 0.],
            vec![2., 0.],
            vec![6., 2.],
            vec![12., 12.],
        ]);
        for transformed in [false, true] {
            let mut wire = vec![
                b.clone(),
                Curve::from_polyline(vec![vec![12., 12.], vec![6., 6.]]).unwrap(),
                a.reverse().unwrap(),
            ];
            if transformed {
                for c in &mut wire {
                    for p in &mut c.control_points {
                        let (x, y) = (p[0], p[1]);
                        p[0] = 17. + x - y;
                        p[1] = -9. + x + y;
                    }
                }
            }
            let before = wire
                .iter()
                .map(|c| c.control_points.clone())
                .collect::<Vec<_>>();
            let expected = if transformed { 25.2 } else { 12.6 };
            assert!((validate_single_wire(&wire, 1e-7).unwrap() - expected).abs() < 1e-8);
            let normalized = normalize(&[wire.clone()], 1e-7).unwrap();
            assert_eq!(normalized.len(), 1);
            assert!((signed_area(&normalized[0], 1e-7).unwrap() - expected).abs() < 1e-8);
            assert!(normalized[0].iter().filter(|c| c.degree == 3).count() >= 2);
            assert_eq!(
                wire.iter()
                    .map(|c| c.control_points.clone())
                    .collect::<Vec<_>>(),
                before
            );
        }
    }
    #[test]
    fn curved_third_order_difference_is_a_valid_cusp_component() {
        let region = |y: [f64; 4]| {
            let c = cubic((0..4).map(|i| vec![10. * i as f64, y[i]]).collect());
            vec![
                Curve::from_polyline(vec![vec![0., -2.], vec![30., -2.]]).unwrap(),
                Curve::from_polyline(vec![vec![30., -2.], vec![30., y[3]]]).unwrap(),
                c.reverse().unwrap(),
                Curve::from_polyline(vec![vec![0., y[0]], vec![0., -2.]]).unwrap(),
            ]
        };
        let a = region([0., 0., 1., 3.]);
        let b = region([0., 0., 1., 6.]);
        let r = boolean(&[b], &[a], "difference", 1e-7).unwrap();
        assert_eq!(r.len(), 1);
        assert!((signed_area(&r[0], 1e-7).unwrap() - 22.5).abs() < 1e-7);
    }
}

#[cfg(test)]
mod rational_representation_tests {
    use super::*;
    fn retained_cubic()->Curve {
        // Exact binary64 source: H(t)=3(1+t)*(10t,5t²,1).
        Curve{degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            control_points:vec![vec![0.,0.],vec![2.5,0.],vec![6.,1.],vec![10.,5.]],
            weights:vec![3.,4.,5.,6.],periodic:false}
    }
    fn region(c:Curve)->Vec<Curve> {
        vec![Curve::from_polyline(vec![vec![0.,0.],vec![10.,0.]]).unwrap(),
            Curve::from_polyline(vec![vec![10.,0.],vec![10.,5.]]).unwrap(),c.reverse().unwrap()]
    }
    #[test]
    fn rational_cubic_common_factor_keeps_source_and_extrudes() {
        let c=retained_cubic();let wire=region(c.clone());let before=wire.clone();
        assert!((validate_single_wire(&wire,1e-7).unwrap()-50./3.).abs()<1e-6);
        let certificate=crate::profile_certificate::certify(&[wire.clone()],1e-7).unwrap();
        crate::profile_certificate::verify(&[wire.clone()],1e-7,&certificate).unwrap();
        let solid=crate::prism::extrude(&[wire.clone()],0.,3.).unwrap();
        solid.validate().unwrap();
        assert!(solid.edges.iter().any(|e|e.curve.degree==3&&e.curve.weights.iter().any(|w|*w!=e.curve.weights[0])));
        assert_eq!(wire,before);
        let result=boolean(&[wire.clone()],&[wire.clone()],"intersection",1e-7).unwrap();
        assert!((validate_profile(&result,1e-7).unwrap()-50./3.).abs()<1e-6);
        assert!(boolean(&[wire.clone()],&[wire],"difference",1e-7).unwrap().is_empty());
    }
    #[test]
    fn polynomial_quadratic_profile_has_exact_green_integral() {
        let c=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.,0.],vec![5.,0.],vec![10.,5.]],
            weights:vec![1.;3],periodic:false};
        let wire=region(c.clone());
        assert!(requires_general(&c,1e-7));
        assert!((validate_single_wire(&wire,1e-7).unwrap()-50./3.).abs()<1e-12);
        crate::prism::extrude(&[wire.clone()],0.,3.).unwrap().validate().unwrap();
        // Different degrees and a nonconstant common homogeneous factor must
        // still describe the same retained boundary in Boolean operations.
        let rational=region(retained_cubic());
        assert!(boolean(&[wire.clone()],&[rational.clone()],"difference",1e-7).unwrap().is_empty());
        let union=boolean(&[wire],&[rational],"union",1e-7).unwrap();
        assert!((validate_profile(&union,1e-7).unwrap()-50./3.).abs()<1e-6);
    }
    #[test]
    fn rational_green_bounds_enclose_an_independent_polynomial_integral() {
        let c=retained_cubic();
        let bounds=area_ops::integral(&c,[0.,0.],1e-8).unwrap();
        // Open parabola contribution: 1/2 integral (10t*10t-5t²*10)dt=25/3.
        assert!(bounds.lo<=25./3.&&25./3.<=bounds.hi);
        assert!(bounds.hi-bounds.lo<2.1e-8);
    }
}

#[cfg(test)]
mod parameter_domain_tests {
    use super::*;
    #[test]
    fn normalization_keeps_cusp_geometry_on_small_and_large_parameter_domains() {
        for end in [1e-15, 1., 1e8] {
            let c = Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., end, end, end, end],
                control_points: vec![vec![3., -1.], vec![-1., 1.], vec![-1., -1.], vec![3., 1.]],
                weights: vec![1.; 4],
                periodic: false,
            };
            let wire = vec![
                c,
                Curve::from_polyline(vec![vec![3., 1.], vec![3., -1.]]).unwrap(),
            ];
            let result = normalize(&[wire], 1e-7).unwrap();
            assert_eq!(result.len(), 1);
            assert!((signed_area(&result[0], 1e-7).unwrap() - 2.4).abs() < 1e-8);
            assert_eq!(result[0].iter().filter(|c| c.degree == 3).count(), 2);
        }
    }
}

#[cfg(test)]
mod nodal_parent_tests {
    use super::*;
    #[test]
    fn nodal_parent_survives_stationary_splitting_and_affine_transform() {
        for transformed in [false, true] {
            let c = Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
                control_points: vec![
                    vec![9., -18.],
                    vec![-7., 26.],
                    vec![-7., -26.],
                    vec![9., 18.],
                ],
                weights: vec![1.; 4],
                periodic: false,
            };
            let mut wire = vec![
                c,
                Curve::from_polyline(vec![vec![9., 18.], vec![9., -18.]]).unwrap(),
            ];
            if transformed {
                for c in &mut wire {
                    for p in &mut c.control_points {
                        let (x, y) = (p[0], p[1]);
                        p[0] = 17. + x - y;
                        p[1] = -9. + x + y;
                    }
                }
            }
            let result = normalize(&[wire], 1e-7).unwrap();
            assert_eq!(result.len(), 2);
            let area = result
                .iter()
                .map(|w| signed_area(w, 1e-7).unwrap())
                .sum::<f64>();
            assert!((area - if transformed { 288. } else { 144. }).abs() < 1e-6);
            assert!(result.iter().flatten().any(|c| c.degree == 3));
        }
    }
}
