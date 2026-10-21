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
fn spans(c: &Curve) -> Vec<(usize, [f64; 2])> {
    (c.degree..c.control_points.len())
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .map(|i| (i, [c.knots[i], c.knots[i + 1]]))
        .collect()
}
fn hull(c: &Curve, span: usize, d: [f64; 2]) -> Result<Vec<I>> {
    curve_distance::enclosure(c, span, I::new(d[0], d[1])?)
}
fn jacobian(c: &Curve, span: usize, d: [f64; 2], axes: [usize; 2]) -> Result<[I; 2]> {
    let jet = curve_jets::enclose(c, span, d)?;
    let width = I::point(d[1]).sub(I::point(d[0]))?;
    Ok([jet[1][axes[0]].div(width)?, jet[1][axes[1]].div(width)?])
}
fn krawczyk(
    a: &Curve,
    b: &Curve,
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
    if a == b {
        out.identical_parameter_overlap = true;
        out.complete = true;
        return Ok(out);
    }
    if let Some((crossings, complete)) = exact_line_pair(a, b) {
        out.crossings = crossings;
        out.complete = complete;
        return Ok(out);
    }
    let plane = (0..3).find(|&k| {
        a.control_points
            .iter()
            .all(|p| p[k] == a.control_points[0][k])
            && b.control_points
                .iter()
                .all(|p| p[k] == a.control_points[0][k])
    });
    let Some(plane) = plane else {
        out.unresolved.push(whole);
        return Ok(out);
    };
    let free: Vec<_> = (0..3).filter(|&k| k != plane).collect();
    let axes = [free[0], free[1]];
    let aa = spans(a);
    let bb = spans(b);
    check(
        aa.len().saturating_mul(bb.len()) <= 4096,
        "Planar intersection initial span-pair limit exceeded",
    )?;
    let mut stack = Vec::new();
    for &(sa, da) in &aa {
        for &(sb, db) in &bb {
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
