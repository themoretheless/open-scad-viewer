//! Partial curved contact ownership in original convex polygonal UV faces.
//! Exact halfspace certificates and complete rational coordinate clocks.
use super::Contact;
use crate::{
    Model, TopoId,
    region_replacement::{EdgeRadius, plan_radius_network},
};
use num_rational::BigRational as Q;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use nurbs_core::{
    Result, curve::Curve, surface::Surface, surface_g2_pcurves, surface_linear_monotonicity,
};
use std::collections::BTreeSet;
type Point = [Q; 2];
mod source_clock;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Unresolved,
    BoundPartial,
    BoundFull,
}
pub struct Ownership {
    pub face: TopoId,
    /// Outward endpoint brackets, ordered in the original 3D edge direction.
    /// For nonlinear source clocks these are NOT exact scalar endpoints.
    pub source_clock_interval: [[f64; 2]; 2],
    /// Contact traversal runs opposite to the original source-edge parameter.
    pub contact_reversed: bool,
    pub clock: nurbs_core::curve_coordinate_clock::Report,
    /// Fresh strict source clock for nonaffine straight trims.
    pub source_clock: Option<nurbs_core::curve_coordinate_clock::Report>,
    pub injectivity: surface_linear_monotonicity::Report,
}
pub struct Report {
    pub decision: Decision,
    pub edge: TopoId,
    pub contacts: Vec<Ownership>,
    /// Outward brackets of max(start) and min(end) from both source uses.
    /// Only [brackets[0][1], brackets[1][0]] is a guaranteed common interior.
    /// Midpoints and the outer hull must not authorize a trim endpoint.
    pub common_source_clock: Option<[[f64; 2]; 2]>,
    pub positions: Vec<surface_g2_pcurves::Positions>,
    pub cells: usize,
    pub exact_work: u64,
    pub uv_work: usize,
    pub reason: &'static str,
}
fn point(p: &[f64]) -> Point {
    [Q::from_f64(p[0]).unwrap(), Q::from_f64(p[1]).unwrap()]
}
fn cross(a: &Point, b: &Point, p: &Point) -> Q {
    (&b[0] - &a[0]) * (&p[1] - &a[1]) - (&b[1] - &a[1]) * (&p[0] - &a[0])
}
fn length2(a: &Point, b: &Point) -> Q {
    let x = &b[0] - &a[0];
    let y = &b[1] - &a[1];
    &x * &x + &y * &y
}
// Exact B-spline linear reproduction, including degree-elevated/multispan
// charts. Only this fast path may skip inverse brackets: collinearity alone
// does not establish affine parameter speed.
fn segment(c: &Curve) -> Option<[Point; 2]> {
    let n = c.control_points.len();
    let p = c.degree;
    if c.validate().is_err()
        || !(1..=32).contains(&p)
        || c.periodic
        || n > 4096
        || c.control_points.iter().any(|p| p.len() != 2)
        || c.weights[0] <= 0.
        || c.weights.iter().any(|w| *w != c.weights[0])
        || c.knots[..=p].iter().any(|k| *k != c.knots[p])
        || c.knots[n..].iter().any(|k| *k != c.knots[n])
    {
        return None;
    }
    let a = point(&c.control_points[0]);
    let b = point(&c.control_points[n - 1]);
    let origin = Q::from_f64(c.knots[p])?;
    let width = Q::from_f64(c.knots[n])? - &origin;
    let degree = Q::from_integer(p.into());
    for (i, control) in c.control_points.iter().enumerate() {
        let greville: Q = c.knots[i + 1..=i + p].iter()
            .map(|k| Q::from_f64(*k).unwrap()).sum::<Q>() / &degree;
        let t = (greville - &origin) / &width;
        let actual = point(control);
        if (0..2).any(|k| actual[k] != &a[k] + (&b[k] - &a[k]) * &t) {
            return None;
        }
    }
    Some([a, b])
}
fn enclosure(q: &Q) -> Result<[f64; 2]> {
    let x = q.to_f64().filter(|v| v.is_finite()).ok_or_else(|| {
        nurbs_core::Error::new(
            "BREP_CONTACT_INTERVAL",
            "Source parameter cannot be enclosed",
        )
    })?;
    let mut lo = x;
    let mut hi = x;
    for _ in 0..8 {
        if Q::from_f64(lo).unwrap() > *q {
            lo = lo.next_down();
        }
        if Q::from_f64(hi).unwrap() < *q {
            hi = hi.next_up();
        }
        if lo.is_finite()
            && hi.is_finite()
            && Q::from_f64(lo).unwrap() <= *q
            && *q <= Q::from_f64(hi).unwrap()
        {
            return Ok([lo, hi]);
        }
    }
    Err(nurbs_core::Error::new(
        "BREP_CONTACT_INTERVAL",
        "Exact parameter enclosure remained unresolved",
    ))
}
/// A contact must be strictly nearer to the selected boundary carrier than
/// every other carrier throughout its positive rational control hull. This
/// conservative UV Voronoi gate prevents a far/opposite branch inheriting the
/// selected edge. It is not a rolling-ball continuation uniqueness proof.
/// Nonlinear-speed straight source trims use exact inverse brackets after a
/// whole-domain strict clock proof. Curved UV boundaries and nonconvex faces
/// remain unresolved. Partial ownership is explicit and
/// must never be promoted to complete-edge construction or radius admission.
pub fn inspect(
    source: &Model,
    shell: usize,
    request: EdgeRadius,
    skin: &Surface,
    contacts: [&Contact; 2],
    max_visits: usize,
    max_cells: usize,
    max_exact: u64,
    max_uv_work: usize,
) -> Result<Report> {
    let plan = plan_radius_network(source, shell, &[request], max_visits)?;
    skin.validate()?;
    let e = &plan.edges[0];
    let mut out = Report {
        decision: Decision::Unresolved,
        edge: e.identity,
        contacts: vec![],
        common_source_clock: None,
        positions: vec![],
        cells: 0,
        exact_work: 0,
        uv_work: 0,
        reason: "Original source lifts unresolved",
    };
    if max_uv_work > 1000000 {
        return Err(nurbs_core::Error::new(
            "BREP_CONTACT_UV_BUDGET",
            "Convex branch work must be bounded",
        ));
    }
    let original = super::topology::inspect_source_edge(
        source, shell, request, max_visits, max_cells, max_exact,
    )?;
    let proven = original.proven;
    out.cells += original.cells;
    out.exact_work += original.exact_work;
    out.positions.push(original);
    if !proven {
        return Ok(out);
    }
    let mut intervals = Vec::new();
    for (side, &fi) in e.supports.iter().enumerate() {
        out.reason = "Original UV trim is not one convex straight polygon";
        let face = &source.faces[fi];
        if !face.holes.is_empty() {
            return Ok(out);
        }
        let wire = &source.loops[face.outer].coedges;
        if !(3..=256).contains(&wire.len()) {
            return Ok(out);
        }
        let mut segments = Vec::new();
        let mut source_clocks = Vec::new();
        let mut selected = None;
        for (i, c) in wire.iter().enumerate() {
            let cost = c.pcurve.control_points.len().saturating_mul(
                c.pcurve.degree.saturating_add(8));
            if cost > max_uv_work.saturating_sub(out.uv_work) {
                out.reason = "Original straight trim work exhausted";
                return Ok(out);
            }
            out.uv_work += cost;
            // This path uses complete authored clocks; partial restrictions
            // require their own endpoint/clock transaction.
            if c.trim().is_some_and(|t| t.curve_parameter != source.edges[c.edge].curve.domain()
                || t.pcurve_parameter != c.pcurve.domain() || t.periodic_lift != [[0; 2]; 2]) {
                out.reason = "Restricted original trim clock unresolved";
                return Ok(out);
            }
            let affine = segment(&c.pcurve);
            let Some(s) = affine.clone().or_else(|| source_clock::straight(&c.pcurve)) else {
                return Ok(out);
            };
            let clock = if affine.is_none() {
                out.reason = "Original nonlinear trim clock unresolved";
                let axis = usize::from(s[0][0] == s[1][0]);
                let r = nurbs_core::curve_coordinate_clock::inspect(
                    &c.pcurve, axis, max_cells.saturating_sub(out.cells),
                )?;
                out.cells += r.cells;
                if !r.proven { return Ok(out); }
                Some(r)
            } else { None };
            source_clocks.push(clock);
            if c.edge == request.edge {
                selected = Some((i, c));
            }
            segments.push(s);
        }
        let Some((selected, co)) = selected else {
            return Ok(out);
        };
        let unique: BTreeSet<_> = segments.iter().map(|s| s[0].clone()).collect();
        if unique.len() != segments.len()
            || segments
                .iter()
                .enumerate()
                .any(|(i, s)| s[1] != segments[(i + 1) % segments.len()][0])
        {
            return Ok(out);
        }
        if max_uv_work.saturating_sub(out.uv_work) < 12 * segments.len() {
            out.reason = "UV halfspace work exhausted";
            return Ok(out);
        }
        out.uv_work += 12 * segments.len();
        let area: Q = segments
            .iter()
            .map(|[a, b]| &a[0] * &b[1] - &a[1] * &b[0])
            .sum();
        if area <= Q::zero() {
            return Ok(out);
        }
        for s in &segments {
            if s[0] == s[1] {
                return Ok(out);
            }
            for v in &unique {
                if max_uv_work.saturating_sub(out.uv_work) < 12 {
                    out.reason = "UV halfspace work exhausted";
                    return Ok(out);
                }
                out.uv_work += 12;
                if cross(&s[0], &s[1], v) < Q::zero() {
                    return Ok(out);
                }
            }
        }
        let [a, b] = &segments[selected];
        let free = if a[0] != b[0] { 0 } else { 1 };
        let (first, last) = if co.reversed { (b, a) } else { (a, b) };
        let trace = &contacts[side].support_pcurve;
        trace.validate()?;
        if trace.periodic
            || trace.degree == 0
            || trace.degree > 32
            || trace.control_points.len() > 4096
            || trace.control_points.iter().any(|p| p.len() != 2)
            || trace.weights.iter().any(|w| *w <= 0.)
        {
            return Ok(out);
        }
        let points: Vec<_> = trace.control_points.iter().map(|p| point(p)).collect();
        let mut t0 = (&points[0][free] - &first[free]) / (&last[free] - &first[free]);
        let mut t1 = (&points.last().unwrap()[free] - &first[free]) / (&last[free] - &first[free]);
        let contact_reversed = t0 > t1;
        if contact_reversed { std::mem::swap(&mut t0, &mut t1); }
        if t0 < Q::zero() || t1 > Q::from_integer(1.into()) || t0 >= t1 {
            out.reason = "Contact endpoint clock leaves selected edge";
            return Ok(out);
        }
        if max_uv_work.saturating_sub(out.uv_work) < 12 * points.len() {
            out.reason = "UV halfspace work exhausted";
            return Ok(out);
        }
        out.uv_work += 12 * points.len();
        let selected_distances: Vec<_> = points.iter().map(|p| cross(a, b, p)).collect();
        let lo = selected_distances.iter().min().unwrap();
        let hi = selected_distances.iter().max().unwrap();
        if lo <= &Q::zero() {
            out.reason = "Contact crosses original selected carrier";
            return Ok(out);
        }
        let selected_length = length2(a, b);
        for (j, [u, v]) in segments.iter().enumerate() {
            let mut low = None;
            for p in &points {
                if max_uv_work.saturating_sub(out.uv_work) < 12 {
                    out.reason = "UV halfspace work exhausted";
                    return Ok(out);
                }
                out.uv_work += 12;
                let distance = cross(u, v, p);
                if distance < Q::zero() {
                    out.reason = "Contact control hull leaves original convex trim";
                    return Ok(out);
                }
                if low.as_ref().is_none_or(|old| &distance < old) {
                    low = Some(distance);
                }
            }
            if j != selected {
                let lower = low.unwrap();
                if hi * hi * length2(u, v) >= &lower * &lower * &selected_length {
                    out.reason = "Selected carrier dominance is not separated";
                    return Ok(out);
                }
            }
        }
        out.reason = "Contact source clock unresolved";
        let clock = nurbs_core::curve_coordinate_clock::inspect(
            trace,
            free,
            max_cells.saturating_sub(out.cells),
        )?;
        out.cells += clock.cells;
        if !clock.proven {
            return Ok(out);
        }
        out.reason = "Original support injectivity unresolved";
        let injectivity = surface_linear_monotonicity::inspect_candidate(
            &face.surface,
            max_cells.saturating_sub(out.cells),
        )?;
        out.cells += injectivity.cells;
        if !injectivity.certified {
            return Ok(out);
        }
        out.reason = "Actual contact lifts unresolved";
        let positions = surface_g2_pcurves::verify_positions(
            &contacts[side].curve,
            [trace, &contacts[side].fillet_pcurve],
            [&face.surface, skin],
            [false; 2],
            source.tolerance_mm,
            max_cells.saturating_sub(out.cells),
            max_exact.saturating_sub(out.exact_work),
        )?;
        let proven = positions.proven;
        out.cells += positions.cells;
        out.exact_work += positions.exact_work;
        out.positions.push(positions);
        if !proven {
            return Ok(out);
        }
        out.reason = "Original nonlinear inverse clock work exhausted";
        let parameters = if source_clocks[selected].is_none() {
            [[t0.clone(), t0], [t1.clone(), t1]]
        } else {
            let ends = if contact_reversed { [points.last().unwrap(), &points[0]] }
                else { [&points[0], points.last().unwrap()] };
            let Some(start) = source_clock::inverse(&co.pcurve, free, &ends[0][free],
                co.reversed, &mut out.uv_work, max_uv_work) else { return Ok(out); };
            let Some(end) = source_clock::inverse(&co.pcurve, free, &ends[1][free],
                co.reversed, &mut out.uv_work, max_uv_work) else { return Ok(out); };
            [start, end]
        };
        if parameters[0][1] >= parameters[1][0] {
            out.reason = "Original inverse clock intervals not separated";
            return Ok(out);
        }
        let domain = source.edges[request.edge].curve.domain();
        let origin = Q::from_f64(domain[0]).unwrap();
        let width = Q::from_f64(domain[1]).unwrap() - &origin;
        out.contacts.push(Ownership {
            face: source.1.faces[fi],
            contact_reversed,
            source_clock_interval: [
                [enclosure(&(&origin + &width * &parameters[0][0]))?[0],
                 enclosure(&(&origin + &width * &parameters[0][1]))?[1]],
                [enclosure(&(&origin + &width * &parameters[1][0]))?[0],
                 enclosure(&(&origin + &width * &parameters[1][1]))?[1]],
            ],
            clock,
            source_clock: source_clocks[selected].take(),
            injectivity,
        });
        intervals.push(parameters);
    }
    let start: [Q; 2] = std::array::from_fn(|k|
        std::cmp::max(intervals[0][0][k].clone(), intervals[1][0][k].clone()));
    let end: [Q; 2] = std::array::from_fn(|k|
        std::cmp::min(intervals[0][1][k].clone(), intervals[1][1][k].clone()));
    if start[1] >= end[0] {
        out.reason = "Contacts have no common original-edge interval";
        return Ok(out);
    }
    let domain = source.edges[request.edge].curve.domain();
    let origin = Q::from_f64(domain[0]).unwrap();
    let width = Q::from_f64(domain[1]).unwrap() - &origin;
    let common = [
        [enclosure(&(&origin + &width * &start[0]))?[0],
         enclosure(&(&origin + &width * &start[1]))?[1]],
        [enclosure(&(&origin + &width * &end[0]))?[0],
         enclosure(&(&origin + &width * &end[1]))?[1]],
    ];
    if common[0][1] >= common[1][0] {
        out.reason = "Source parameter resolution leaves no guaranteed common interval";
        return Ok(out);
    }
    out.common_source_clock = Some(common);
    out.decision = if intervals
        .iter()
        .all(|t| t[0].iter().all(Zero::is_zero)
            && t[1].iter().all(|v| *v == Q::from_integer(1.into())))
    {
        Decision::BoundFull
    } else {
        Decision::BoundPartial
    };
    out.reason = "Both contact ribbons owned inside original convex trims";
    Ok(out)
}

