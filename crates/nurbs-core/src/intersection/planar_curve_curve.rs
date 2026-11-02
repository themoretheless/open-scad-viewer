//! Original-source planar crossing existence and uniqueness. Numerical
//! preconditioning proposes a matrix; every proof operation is outward.
use super::ContactClass;
use crate::{
    Result, check,
    curve::Curve,
    curve_distance, curve_jets,
    distance_bounds::{Interval as I, box_distance},
};
type Domain = [[f64; 2]; 2];
mod coplanar;
mod source;
use source::Bounds;
#[derive(Clone, Debug)]
pub struct CertifiedCurveCrossing {
    pub parameters: Domain,
    pub position_bounds: [[f64; 2]; 3],
    pub contact: ContactClass,
}
#[derive(Clone, Debug)]
pub struct CertifiedCurveIntersection {
    pub crossings: Vec<CertifiedCurveCrossing>,
    pub identical_parameter_overlap: bool,
    pub unresolved: Vec<Domain>,
    pub cells: usize,
    pub complete: bool,
}
fn spans(c: &impl Bounds) -> Vec<(usize, [f64; 2])> {
    c.spans()
}
fn hull(c: &impl Bounds, span: usize, d: [f64; 2]) -> Result<Vec<I>> {
    c.hull(span, d)
}
fn jacobian(c: &impl Bounds, span: usize, d: [f64; 2], axes: [usize; 2]) -> Result<[I; 2]> {
    let j = c.derivative(span, d)?;
    Ok([j[axes[0]], j[axes[1]]])
}
fn krawczyk(
    a: &impl Bounds,
    b: &impl Bounds,
    sa: usize,
    sb: usize,
    d: Domain,
    axes: [usize; 2],
) -> Result<Option<Domain>> {
    let da = jacobian(a, sa, d[0], axes)?;
    let db = jacobian(b, sb, d[1], axes)?;
    let j = [
        [da[0], I::new(-db[0].hi, -db[0].lo)?],
        [da[1], I::new(-db[1].hi, -db[1].lo)?],
    ];
    let m = d.map(|r| r[0] + (r[1] - r[0]) * 0.5);
    let center = |x: I| x.lo + (x.hi - x.lo) * 0.5;
    let q = j.map(|row| row.map(center));
    let det = q[0][0] * q[1][1] - q[0][1] * q[1][0];
    if !det.is_finite() || det == 0. {
        return Ok(None);
    }
    let y = [
        [q[1][1] / det, -q[0][1] / det],
        [-q[1][0] / det, q[0][0] / det],
    ];
    if !y.iter().flatten().all(|x| x.is_finite()) {
        return Ok(None);
    }
    // CRITICAL: the residual at the stored midpoint is an original outward
    // enclosure, never a plain floating-point evaluation relabeled exact.
    let pa = hull(a, sa, [m[0]; 2])?;
    let pb = hull(b, sb, [m[1]; 2])?;
    let f = [pa[axes[0]].sub(pb[axes[0]])?, pa[axes[1]].sub(pb[axes[1]])?];
    let mut out = [[0.; 2]; 2];
    for row in 0..2 {
        let mut base = I::point(m[row]);
        for k in 0..2 {
            base = base.sub(I::point(y[row][k]).mul(f[k])?)?;
        }
        let mut norm = I::point(0.);
        for column in 0..2 {
            let mut r = I::point(if row == column { 1. } else { 0. });
            for k in 0..2 {
                r = r.sub(I::point(y[row][k]).mul(j[k][column])?)?;
            }
            norm = norm.add(I::point(r.lo.abs().max(r.hi.abs())))?;
            base =
                base.add(r.mul(I::new(d[column][0], d[column][1])?.sub(I::point(m[column]))?)?)?;
        }
        // Contraction on the complete original cell proves uniqueness.
        if norm.hi >= 1. || base.lo <= d[row][0] || base.hi >= d[row][1] {
            return Ok(None);
        }
        out[row] = [base.lo, base.hi];
    }
    Ok(Some(out))
}
fn exact_line_pair(a: &Curve, b: &Curve) -> Option<(Vec<CertifiedCurveCrossing>, bool)> {
    use crate::exact_curve_segments::{exact_add as add, exact_mul as mul};
    let affine = |c: &Curve| {
        c.degree == 1
            && c.control_points.len() == 2
            && c.knots[0] == c.knots[1]
            && c.knots[2] == c.knots[3]
            && c.weights[0] == c.weights[1]
    };
    if !affine(a) || !affine(b) {
        return None;
    }
    let p = &a.control_points[0];
    let q = &b.control_points[0];
    let mut u = [0.; 3];
    let mut v = [0.; 3];
    let mut w = [0.; 3];
    for k in 0..3 {
        u[k] = add(a.control_points[1][k], -p[k])?;
        v[k] = add(b.control_points[1][k], -q[k])?;
        w[k] = add(q[k], -p[k])?;
    }
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let Some(det) = mul(u[i], -v[j]).and_then(|x| add(x, -mul(u[j], -v[i])?)) else {
            continue;
        };
        if det == 0. {
            continue;
        }
        let na = add(mul(w[i], -v[j])?, -mul(w[j], -v[i])?)?;
        let nb = add(mul(u[i], w[j])?, -mul(u[j], w[i])?)?;
        let x = na / det;
        let y = nb / det;
        if mul(x, det)? != na || mul(y, det)? != nb {
            return None;
        }
        if !(0. ..=1.).contains(&x) || !(0. ..=1.).contains(&y) {
            return Some((vec![], true));
        }
        let mut point = [0.; 3];
        for k in 0..3 {
            point[k] = add(p[k], mul(x, u[k])?)?;
            let other = add(q[k], mul(y, v[k])?)?;
            if point[k] != other {
                return Some((vec![], true));
            }
        }
        let map = |c: &Curve, t: f64| -> Option<[f64; 2]> {
            let [lo, hi] = c.domain();
            let v = I::point(lo)
                .add(I::point(hi).sub(I::point(lo)).ok()?.mul(I::point(t)).ok()?)
                .ok()?;
            Some([v.lo.max(lo), v.hi.min(hi)])
        };
        let contact = if x == 0. || x == 1. || y == 0. || y == 1. {
            ContactClass::Boundary
        } else {
            ContactClass::Transverse
        };
        return Some((
            vec![CertifiedCurveCrossing {
                parameters: [map(a, x)?, map(b, y)?],
                position_bounds: point.map(|p| [p, p]),
                contact,
            }],
            true,
        ));
    }
    None
}

/// Certified bounded intersection: exact checked affine 3D line pairs, or
/// original 3D curves in an exactly shared
/// axis-aligned plane. Positive rational original spans are used throughout.
/// A strict Krawczyk self-inclusion plus a contraction proves one transverse
/// crossing; complete original hull coverage excludes all other cells.
/// Literal identical definitions prove whole same-parameter overlap.
/// Noncoplanar sources, tangencies, other coincidence, roots at span/partition
/// boundaries, insufficient localization and exhausted work stay explicit.
/// No sampled residual, rounded subdivision control or numerical tolerance is
/// used as an existence proof. `tolerance` bounds each admitted spatial box's
/// diameter; it never changes the exact equation C(t)=D(u).
/// Limits: 1..100000 visited cells, at most 4096 initial active span pairs.
pub fn intersect_curve_curve_certified_report(
    a: &Curve,
    b: &Curve,
    tolerance: f64,
    max_cells: usize,
) -> Result<CertifiedCurveIntersection> {
    intersect_selected(a, b, tolerance, max_cells, false, false)
}

/// Certifies crossings between distinct original spans of one planar curve.
/// `complete` covers only those pairs: it does not audit self-loops wholly
/// inside one span. The diagonal and the identical-curve overlap shortcut are
/// deliberately excluded. Knots and controls are never extracted or rounded.
pub fn intersect_distinct_spans_certified_report(
    curve: &Curve,
    tolerance: f64,
    max_cells: usize,
) -> Result<CertifiedCurveIntersection> {
    intersect_selected(curve, curve, tolerance, max_cells, true, false)
}

/// Self-crossing audit including pairs within each original span. Diagonal
/// regions are excluded only by original derivative monotonicity, or split
/// into two smaller diagonal regions and one disjoint-interior pair. Cusps,
/// contacts at subdivision boundaries and exhausted work remain unresolved.
pub fn intersect_self_curve_certified_report(
    curve: &Curve,
    tolerance: f64,
    max_cells: usize,
) -> Result<CertifiedCurveIntersection> {
    intersect_selected(curve, curve, tolerance, max_cells, true, true)
}

fn intersect_selected(
    a: &Curve,
    b: &Curve,
    tolerance: f64,
    max_cells: usize,
    self_pairs: bool,
    diagonal: bool,
) -> Result<CertifiedCurveIntersection> {
    a.validate()?;
    b.validate()?;
    check(
        a.control_points[0].len() == 3
            && b.control_points[0].len() == 3
            && !a.periodic
            && !b.periodic,
        "Planar crossing needs nonperiodic original 3D curves",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0. && (1..=100000).contains(&max_cells),
        "Planar crossing needs positive tolerance and bounded work",
    )?;
    let whole = [a.domain(), b.domain()];
    let mut out = CertifiedCurveIntersection {
        crossings: vec![],
        identical_parameter_overlap: false,
        unresolved: vec![],
        cells: 0,
        complete: false,
    };
    if !self_pairs && a == b {
        out.identical_parameter_overlap = true;
        out.complete = true;
        return Ok(out);
    }
    if let Some((crossings, complete)) = (!self_pairs).then(|| exact_line_pair(a, b)).flatten() {
        out.crossings = crossings;
        out.complete = complete;
        return Ok(out);
    }
    let Some(axes) = coplanar::axes(a, b) else {
        out.unresolved.push(whole);
        return Ok(out);
    };
    solve(
        a, b, axes, tolerance, max_cells, self_pairs, diagonal, false,
    )
}

fn solve(
    a: &impl Bounds,
    b: &impl Bounds,
    axes: [usize; 2],
    tolerance: f64,
    max_cells: usize,
    self_pairs: bool,
    diagonal: bool,
    stop_after_first: bool,
) -> Result<CertifiedCurveIntersection> {
    let whole = [a.domain(), b.domain()];
    let mut out = CertifiedCurveIntersection {
        crossings: vec![],
        identical_parameter_overlap: false,
        unresolved: vec![],
        cells: 0,
        complete: false,
    };
    let aa = spans(a);
    let bb = spans(b);
    check(
        self_pairs || aa.len().saturating_mul(bb.len()) <= 4096,
        "Planar intersection initial span-pair limit exceeded",
    )?;
    let mut stack = Vec::new();
    if diagonal {
        for &(span, domain) in &aa {
            if stack.len() == 4096 {
                out.unresolved.push(whole);
                return Ok(out);
            }
            stack.push((span, span, [domain; 2], 0usize));
        }
    }
    for &(sa, da) in &aa {
        for &(sb, db) in &bb {
            if self_pairs && sa >= sb {
                continue;
            }
            if stack.len() == 4096 {
                out.unresolved.push(whole);
                return Ok(out);
            }
            stack.push((sa, sb, [da, db], 0usize));
        }
    }
    while let Some((sa, sb, d, depth)) = stack.pop() {
        if out.cells == max_cells {
            out.unresolved.push(d);
            out.unresolved
                .extend(stack.into_iter().map(|(_, _, d, _)| d));
            break;
        }
        out.cells += 1;
        if diagonal && sa == sb {
            let union = [d[0][0].min(d[1][0]), d[0][1].max(d[1][1])];
            if let Ok(jet) = a.derivative(sa, union) {
                if jet.iter().any(|x| x.lo > 0. || x.hi < 0.) {
                    continue;
                }
            }
        }
        if self_pairs && d[0][1] == d[1][0] {
            if let (Ok(left), Ok(right)) = (a.derivative(sa, d[0]), b.derivative(sb, d[1])) {
                if source::ordered_join(&left, &right).unwrap_or(false) {
                    continue;
                }
            }
        }
        if diagonal && sa == sb && d[0] == d[1] {
            let mid = d[0][0] + (d[0][1] - d[0][0]) * 0.4375;
            if depth == 64 || mid <= d[0][0] || mid >= d[0][1] {
                out.unresolved.push(d);
                continue;
            }
            let left = [d[0][0], mid];
            let right = [mid, d[0][1]];
            stack.push((sa, sa, [right, right], depth + 1));
            stack.push((sa, sa, [left, left], depth + 1));
            stack.push((sa, sa, [left, right], depth + 1));
            continue;
        }
        let pa = hull(a, sa, d[0])?;
        let pb = hull(b, sb, d[1])?;
        if box_distance(&pa, &pb)?.0 > 0. {
            continue;
        }
        if let Ok(Some(root)) = krawczyk(a, b, sa, sb, d, axes) {
            let x = hull(a, sa, root[0])?;
            let y = hull(b, sb, root[1])?;
            let bounds: [[f64; 2]; 3] =
                std::array::from_fn(|k| [x[k].lo.max(y[k].lo), x[k].hi.min(y[k].hi)]);
            let mut diameter = I::point(0.);
            for r in bounds {
                let w = I::point(r[1]).sub(I::point(r[0]))?;
                diameter = diameter.add(w.mul(w)?)?;
            }
            if bounds.iter().all(|r| r[0] <= r[1]) && diameter.hi.sqrt().next_up() <= tolerance {
                out.crossings.push(CertifiedCurveCrossing {
                    parameters: root,
                    position_bounds: bounds,
                    contact: ContactClass::Transverse,
                });
                if stop_after_first {
                    out.unresolved
                        .extend(stack.into_iter().map(|(_, _, d, _)| d));
                    out.complete = out.unresolved.is_empty();
                    return Ok(out);
                }
                continue;
            }
            // Every root of the parent lies in its Krawczyk image. Preserve
            // that proved root by contracting to the image instead of cutting
            // it onto an artificial bisection boundary during localization.
            if depth < 64 && root.iter().all(|r| r[0] < r[1]) {
                stack.push((sa, sb, root, depth + 1));
                continue;
            }
        }
        let axis = depth % 2;
        let mid = d[axis][0] + (d[axis][1] - d[axis][0]) * 0.4375;
        if mid <= d[axis][0] || mid >= d[axis][1] || depth == 64 {
            out.unresolved.push(d);
            continue;
        }
        let mut left = d;
        let mut right = d;
        left[axis][1] = mid;
        right[axis][0] = mid;
        stack.push((sa, sb, right, depth + 1));
        stack.push((sa, sb, left, depth + 1));
    }
    out.complete = out.unresolved.is_empty();
    Ok(out)
}

/// Searches for the first planar iso self-crossing at the exact active-domain
/// midpoint of the other axis (axis 0 = U, 1 = V). Recomputed exact homogeneous
/// coefficients own the plane identity; bounds use the original surface.
/// Unvisited cells remain unresolved. This does not prove surface completeness.
pub fn intersect_surface_iso_certified_report(
    surface: &crate::surface::Surface,
    axis: usize,
    tolerance: f64,
    max_cells: usize,
) -> Result<CertifiedCurveIntersection> {
    surface.validate()?;
    check(
        axis < 2
            && !surface.periodic_u
            && !surface.periodic_v
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&max_cells),
        "Invalid original iso audit",
    )?;
    let source = source::Iso::new(surface, axis)?;
    let Some(homogeneous) = crate::intersection::self_surface::linear_crossing::iso(surface, axis)
    else {
        return Ok(CertifiedCurveIntersection {
            crossings: vec![],
            identical_parameter_overlap: false,
            unresolved: vec![[source.domain(); 2]],
            cells: 0,
            complete: false,
        });
    };
    let points = homogeneous
        .iter()
        .map(|h| std::array::from_fn(|i| &h[i] / &h[3]))
        .collect();
    let Some(axes) = coplanar::axes_from_points(points) else {
        return Ok(CertifiedCurveIntersection {
            crossings: vec![],
            identical_parameter_overlap: false,
            unresolved: vec![[source.domain(); 2]],
            cells: 0,
            complete: false,
        });
    };
    solve(
        &source, &source, axes, tolerance, max_cells, true, true, true,
    )
}
