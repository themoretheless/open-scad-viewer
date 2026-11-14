//! Certified classification of tangential curve/plane contacts.
//!
//! The signed plane distance of a positive-weight rational span has the sign
//! of the homogeneous Bernstein polynomial `f(s) = sum w_i (n.P_i - d) B_i(s)`.
//! Its coefficients are formed exactly as floating-point expansions from the
//! trimmed span data, and every later step (dyadic halving, forward
//! differences, endpoint values) is exact, so all sign decisions below are
//! proofs about that polynomial, not samples.
//!
//! On a box where the m-th derivative has strictly one-signed Bernstein
//! coefficients, Rolle bounds the roots by m (with multiplicity). Exact
//! endpoint zeros carry their multiplicity (first non-vanishing derivative);
//! the remaining interior budget `r` together with the sign parity just inside
//! the endpoints settles the box when `r <= 1`. Otherwise the box is halved.
//! A tangency at a non-dyadic parameter can never be hit exactly and ends in
//! a typed refusal at the depth or box budget, never in a guessed answer.
//! Like the rest of curve/plane, the certificate covers the binary64 span data
//! after knot insertion, not the knot edit itself.
use super::*;

type Expansion = Vec<f64>;

fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}
/// Shewchuk grow-expansion with zero elimination: nonoverlapping output in
/// increasing magnitude, so the sign is the sign of the last component.
fn grow(e: &[f64], b: f64) -> Expansion {
    let mut q = b;
    let mut out = Vec::with_capacity(e.len() + 1);
    for &x in e {
        let (s, h) = two_sum(q, x);
        if h != 0. {
            out.push(h);
        }
        q = s;
    }
    if q != 0. {
        out.push(q);
    }
    out
}
fn add(a: &[f64], b: &[f64]) -> Expansion {
    b.iter().fold(a.to_vec(), |acc, &x| grow(&acc, x))
}
fn neg(a: &[f64]) -> Expansion {
    a.iter().map(|x| -x).collect()
}
fn scale(a: &[f64], b: f64) -> Expansion {
    let mut out = Vec::new();
    for &x in a {
        let p = x * b;
        let e = x.mul_add(b, -p);
        out = grow(&out, e);
        out = grow(&out, p);
    }
    out
}
fn sign(a: &[f64]) -> i8 {
    match a.last() {
        None => 0,
        Some(x) if *x > 0. => 1,
        Some(_) => -1,
    }
}
fn finite(a: &[f64]) -> bool {
    a.iter().all(|x| x.is_finite() && (*x == 0. || x.abs() >= f64::MIN_POSITIVE * 4.))
}

fn exact_coefficients(curve: &Curve, plane: Plane) -> Option<Vec<Expansion>> {
    let mut out = Vec::with_capacity(curve.control_points.len());
    for (point, &weight) in curve.control_points.iter().zip(&curve.weights) {
        let mut e: Expansion = grow(&[], -plane.offset);
        for (n, x) in plane.normal.into_iter().zip(point) {
            e = add(&e, &scale(&[*x], n));
        }
        let e = scale(&e, weight);
        if !finite(&e) {
            return None;
        }
        out.push(e);
    }
    Some(out)
}
fn halve(c: &[Expansion]) -> Option<(Vec<Expansion>, Vec<Expansion>)> {
    let n = c.len();
    let mut row = c.to_vec();
    let mut left = vec![row[0].clone()];
    let mut right = vec![row[n - 1].clone()];
    for len in (1..n).rev() {
        for i in 0..len {
            row[i] = scale(&add(&row[i], &row[i + 1]), 0.5);
            if !finite(&row[i]) {
                return None;
            }
        }
        left.push(row[0].clone());
        right.push(row[len - 1].clone());
    }
    right.reverse();
    Some((left, right))
}
/// Forward differences: signs of the successive derivative Bernstein
/// coefficients (positive degree/width factors dropped).
fn differences(c: &[Expansion]) -> Vec<Vec<Expansion>> {
    let mut all = vec![c.to_vec()];
    while all.last().unwrap().len() > 1 {
        let last = all.last().unwrap();
        all.push(last.windows(2).map(|w| add(&w[1], &neg(&w[0]))).collect());
    }
    all
}
/// Multiplicity of an endpoint zero and the sign of f just inside the box.
fn endpoint(diffs: &[Vec<Expansion>], upper: bool) -> Option<(u32, i8)> {
    for (j, d) in diffs.iter().enumerate() {
        let v = if upper { d.last() } else { d.first() }?;
        let s = sign(v);
        if s != 0 {
            let inside = if upper && j % 2 == 1 { -s } else { s };
            return Some((j as u32, inside));
        }
    }
    None
}
fn strict_order(diffs: &[Vec<Expansion>]) -> Option<usize> {
    diffs.iter().position(|d| {
        let s = sign(&d[0]);
        s != 0 && d.iter().all(|c| sign(c) == s)
    })
}

#[derive(Clone, Debug)]
pub(crate) struct CertifiedRoot {
    pub(crate) parameter: f64,
    pub(crate) parameter_interval: [f64; 2],
    pub(crate) multiplicity: u32,
}
pub(crate) enum SpanClass {
    Overlap,
    Roots(Vec<CertifiedRoot>),
    Refused([f64; 2]),
}

/// Classify all roots of one span. `boxes` is the shared work counter.
pub(crate) fn classify_span(
    span: &Curve,
    plane: Plane,
    domain: [f64; 2],
    options: Options,
    boxes: &mut usize,
) -> SpanClass {
    let Some(coefficients) = exact_coefficients(span, plane) else {
        return SpanClass::Refused(domain);
    };
    if coefficients.iter().all(|c| sign(c) == 0) {
        return SpanClass::Overlap;
    }
    let map = |s: f64| domain[0] + (domain[1] - domain[0]) * s;
    let mut roots: Vec<CertifiedRoot> = Vec::new();
    let push = |roots: &mut Vec<CertifiedRoot>, s: f64, iv: [f64; 2], m: u32| {
        if !roots.iter().any(|r| r.parameter == map(s)) {
            roots.push(CertifiedRoot {
                parameter: map(s),
                parameter_interval: [map(iv[0]), map(iv[1])],
                multiplicity: m,
            });
        }
    };
    let mut pending = vec![(0f64, 1f64, coefficients, 0usize)];
    while let Some((lo, hi, c, depth)) = pending.pop() {
        if *boxes >= options.max_boxes {
            return SpanClass::Refused([map(lo), map(hi)]);
        }
        *boxes += 1;
        let diffs = differences(&c);
        let Some(m) = strict_order(&diffs) else {
            // Identically zero on a sub-box means zero on the whole span.
            return SpanClass::Refused([map(lo), map(hi)]);
        };
        let (Some((k_lo, s_lo)), Some((k_hi, s_hi))) =
            (endpoint(&diffs, false), endpoint(&diffs, true))
        else {
            return SpanClass::Refused([map(lo), map(hi)]);
        };
        if k_lo > 0 {
            push(&mut roots, lo, [lo, lo], k_lo);
        }
        if k_hi > 0 {
            push(&mut roots, hi, [hi, hi], k_hi);
        }
        let Some(r) = (m as u32).checked_sub(k_lo + k_hi) else {
            return SpanClass::Refused([map(lo), map(hi)]);
        };
        let odd = s_lo != s_hi;
        if r == 0 || (r == 1 && !odd) {
            continue;
        }
        if r == 1 {
            // Exactly one simple interior root: bisect on exact signs.
            let (mut a, mut b, mut cc, mut sa) = (lo, hi, c, s_lo);
            loop {
                let mid = a + (b - a) * 0.5;
                if (map(b) - map(a)) <= options.parameter_tolerance || mid == a || mid == b {
                    push(&mut roots, mid, [a, b], 1);
                    break;
                }
                let Some((left, right)) = halve(&cc) else {
                    return SpanClass::Refused([map(a), map(b)]);
                };
                let sm = sign(left.last().unwrap());
                if sm == 0 {
                    push(&mut roots, mid, [mid, mid], 1);
                    break;
                }
                if sm != sa {
                    (b, cc) = (mid, left);
                } else {
                    (a, cc, sa) = (mid, right, sm);
                }
            }
            continue;
        }
        let mid = lo + (hi - lo) * 0.5;
        if depth >= options.max_depth || mid == lo || mid == hi {
            return SpanClass::Refused([map(lo), map(hi)]);
        }
        let Some((left, right)) = halve(&c) else {
            return SpanClass::Refused([map(lo), map(hi)]);
        };
        pending.push((mid, hi, right, depth + 1));
        pending.push((lo, mid, left, depth + 1));
    }
    roots.sort_by(|a, b| a.parameter.total_cmp(&b.parameter));
    SpanClass::Roots(roots)
}

/// Re-run a curve/plane query whose subdivision report refused a tangency or
/// near-coincidence. Returns `None` when any span cannot be certified, so the
/// caller keeps the original typed refusal.
pub(crate) fn certify_tangencies(
    curve: &Curve,
    plane: Plane,
    options: Options,
    boxes_visited: usize,
) -> Result<Option<Report<CurvePlaneComponent>>> {
    let mut boxes = boxes_visited;
    let mut components: Vec<CurvePlaneComponent> = Vec::new();
    // (parameter, multiplicity) of roots on interior knots seen from the left.
    let mut knot_roots: Vec<(f64, u32)> = Vec::new();
    let domain = curve.domain();
    for span_domain in spans(&curve.knots, curve.degree, curve.control_points.len()) {
        let span = curve.trim(span_domain[0], span_domain[1])?;
        match classify_span(&span, plane, span_domain, options, &mut boxes) {
            SpanClass::Refused(_) => return Ok(None),
            SpanClass::Overlap => {
                if let Some(CurvePlaneComponent::Overlap {
                    parameter_interval, ..
                }) = components.last_mut()
                    && parameter_interval[1] == span_domain[0]
                {
                    parameter_interval[1] = span_domain[1];
                } else {
                    components.push(CurvePlaneComponent::Overlap {
                        parameter_interval: span_domain,
                        control_residual: 0.,
                    });
                }
            }
            SpanClass::Roots(roots) => {
                for root in roots {
                    let on_knot = root.parameter == span_domain[0] || root.parameter == span_domain[1];
                    let at_end = root.parameter == domain[0] || root.parameter == domain[1];
                    if on_knot && !at_end {
                        if let Some(&(_, m)) =
                            knot_roots.iter().find(|(p, _)| *p == root.parameter)
                        {
                            // Second side of a knot root: one-sided orders
                            // must agree, otherwise the contact is a kink.
                            if m != root.multiplicity {
                                return Ok(None);
                            }
                            continue;
                        }
                        knot_roots.push((root.parameter, root.multiplicity));
                    }
                    if components.iter().any(|c| match c {
                        CurvePlaneComponent::Overlap {
                            parameter_interval, ..
                        } => {
                            parameter_interval[0] <= root.parameter
                                && root.parameter <= parameter_interval[1]
                        }
                        CurvePlaneComponent::Point(p) => p.parameter == root.parameter,
                    }) {
                        continue;
                    }
                    let point = point3(&curve.evaluate(root.parameter)?.point);
                    let contact = if at_end {
                        Contact::Boundary
                    } else if root.multiplicity >= 2 {
                        Contact::Tangent {
                            multiplicity: root.multiplicity,
                        }
                    } else {
                        Contact::Transverse
                    };
                    components.push(CurvePlaneComponent::Point(CurvePoint {
                        parameter: root.parameter,
                        parameter_interval: root.parameter_interval,
                        point,
                        plane_residual: plane.distance(point).abs(),
                        contact,
                    }));
                }
            }
        }
    }
    // A root on an overlap end is part of the overlap.
    let keep: Vec<bool> = components
        .iter()
        .map(|c| match c {
            CurvePlaneComponent::Point(p) => !components_overlap_contains(&components, p.parameter),
            _ => true,
        })
        .collect();
    let mut keep = keep.into_iter();
    components.retain(|_| keep.next().unwrap());
    let mut report = Report {
        components,
        boxes_visited: boxes,
        ..Report::default()
    };
    report.components.sort_by(|a, b| {
        let key = |c: &CurvePlaneComponent| match c {
            CurvePlaneComponent::Point(p) => p.parameter,
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => parameter_interval[0],
        };
        key(a).total_cmp(&key(b))
    });
    Ok(Some(report))
}
fn components_overlap_contains(components: &[CurvePlaneComponent], t: f64) -> bool {
    components.iter().any(|c| {
        matches!(c, CurvePlaneComponent::Overlap { parameter_interval, .. }
            if parameter_interval[0] <= t && t <= parameter_interval[1])
    })
}
