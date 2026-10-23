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
    curve.degree >= 3 || (curve.degree == 2 &&
        crate::planar_trim::span_green_area(curve, [0.,0.], tolerance).is_err())
}
fn refused(message: &str) -> Error {
    Error::new("BREP_UNSUPPORTED_BEZIER_PROFILE", message)
}
fn controls(curve: &Curve) -> Result<Vec<[f64; 2]>> {
    curve.validate()?;
    if !(1..=8).contains(&curve.degree)
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
            "Bézier admission requires clamped positive-weight planar spans of degree 1..8",
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
    if wire.len() < 2 || wire.len() > MAX_CURVES || !tolerance.is_finite() || tolerance <= 0. {
        return Err(refused(
            "Bézier extrusion requires 2–128 spans and a positive tolerance",
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
    if wire.len()==2 {
        let contact=crate::profile_certificate::certify_endpoint_contacts(&wire[0],&wire[1])?;
        if contact.common_endpoints!=2 {return Err(refused("Two-span boundary needs exactly its two endpoint contacts"));}
        return signed_area(wire,tolerance);
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
#[path = "bezier_profile/tests/tests_integration.rs"]
mod tests;

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
#[path = "bezier_profile/tests/implicit_joint_tests_integration.rs"]
mod implicit_joint_tests;

#[cfg(test)]
#[path = "bezier_profile/tests/rational_representation_tests_integration.rs"]
mod rational_representation_tests;

#[cfg(test)]
#[path = "bezier_profile/tests/parameter_domain_tests_integration.rs"]
mod parameter_domain_tests;

#[cfg(test)]
#[path = "bezier_profile/tests/nodal_parent_tests_integration.rs"]
mod nodal_parent_tests;
