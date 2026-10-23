use super::*;
/// Crossings below this unit-tangent sine stay explicitly unresolved:
/// near-parallel contacts cannot be certified transverse numerically.
pub(crate) const TRANSVERSE_SINE: f64 = 1e-6;

pub(crate) fn homogeneous4(curve: &Curve) -> Vec<[f64; 4]> {
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w, *w])
        .collect()
}
pub(crate) fn split_homogeneous(h: &[[f64; 4]]) -> (Vec<[f64; 4]>, Vec<[f64; 4]>) {
    let mut row = h.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|p| std::array::from_fn(|i| (p[0][i] + p[1][i]) * 0.5))
            .collect();
        left.push(row[0]);
        right.push(*row.last().unwrap());
    }
    right.reverse();
    (left, right)
}
/// Outward-rounded Cartesian control ranges of a positive-weight homogeneous
/// Bezier piece. Convex-hull containment makes axis gaps strict exclusions.
pub(crate) fn hull_ranges(h: &[[f64; 4]]) -> [[f64; 2]; 3] {
    std::array::from_fn(|axis| {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for p in h {
            let x = p[axis] / p[3];
            lo = lo.min(x.next_down());
            hi = hi.max(x.next_up());
        }
        [lo, hi]
    })
}
pub(crate) fn hulls_excluded(a: &[[f64; 4]], b: &[[f64; 4]]) -> bool {
    let ra = hull_ranges(a);
    let rb = hull_ranges(b);
    (0..3).any(|axis| ra[axis][1] < rb[axis][0] || rb[axis][1] < ra[axis][0])
}
#[inline(always)]
pub(crate) fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(a, b);
    d[0].hypot(d[1]).hypot(d[2])
}
/// One-sided first derivatives at a parameter where the two-sided jet may not
/// exist (a C0 knot): the left jet from the Bézier span ending at t and the
/// right jet from the span starting at t. Entries outside the active domain
/// stay None.
fn one_sided_curve_d1(curve: &Curve, t: f64) -> Result<[Option<Vec<f64>>; 2]> {
    let domain = curve.domain();
    let mut jets = [None, None];
    if t > domain[0]
        && let Some(a) = curve
            .knots
            .iter()
            .copied()
            .filter(|&k| k < t)
            .max_by(f64::total_cmp)
    {
        jets[0] = curve.trim(a, t)?.evaluate(t)?.d1;
    }
    if t < domain[1]
        && let Some(b) = curve
            .knots
            .iter()
            .copied()
            .filter(|&k| k > t)
            .min_by(f64::total_cmp)
    {
        jets[1] = curve.trim(t, b)?.evaluate(t)?.d1;
    }
    Ok(jets)
}
/// Tangent vectors at t: the two-sided derivative when it exists, otherwise
/// every available one-sided span derivative at a C0 knot, in [left, right]
/// order. An empty result means no usable jet exists at all.
pub(crate) fn curve_tangents(curve: &Curve, t: f64) -> Result<Vec<[f64; 3]>> {
    let jet = curve.evaluate(t)?;
    if let Some(d1) = &jet.d1 {
        return Ok(vec![point3(d1)]);
    }
    Ok(one_sided_curve_d1(curve, t)?
        .into_iter()
        .flatten()
        .map(|d| point3(&d))
        .collect())
}
/// Surface tangent vectors at (u,v): the two-sided jets when they exist,
/// otherwise the available one-sided span jets across C0 knots in U and V.
pub(crate) type SurfaceTangentPair = (Vec<[f64; 3]>, Vec<[f64; 3]>);

pub(crate) fn surface_tangents(surface: &Surface, u: f64, v: f64) -> Result<SurfaceTangentPair> {
    let jet = surface.evaluate(u, v)?;
    if let Some((du, dv)) = jet.first_derivatives() {
        return Ok((vec![du], vec![dv]));
    }
    let d = surface_domain(surface);
    let mut us = Vec::new();
    let mut vs = Vec::new();
    if u > d[0]
        && let Some(a) = surface
            .knots_u
            .iter()
            .copied()
            .filter(|&k| k < u)
            .max_by(f64::total_cmp)
        && let Some((du, dv)) = surface
            .trim([a, u, d[2], d[3]])?
            .evaluate(u, v)?
            .first_derivatives()
    {
        us.push(du);
        vs.push(dv);
    }
    if u < d[1]
        && let Some(b) = surface
            .knots_u
            .iter()
            .copied()
            .filter(|&k| k > u)
            .min_by(f64::total_cmp)
        && let Some((du, dv)) = surface
            .trim([u, b, d[2], d[3]])?
            .evaluate(u, v)?
            .first_derivatives()
    {
        us.push(du);
        vs.push(dv);
    }
    if vs.is_empty()
        && v > d[2]
        && let Some(a) = surface
            .knots_v
            .iter()
            .copied()
            .filter(|&k| k < v)
            .max_by(f64::total_cmp)
        && let Some((du, dv)) = surface
            .trim([d[0], d[1], a, v])?
            .evaluate(u, v)?
            .first_derivatives()
    {
        us.push(du);
        vs.push(dv);
    }
    if vs.is_empty()
        && v < d[3]
        && let Some(b) = surface
            .knots_v
            .iter()
            .copied()
            .filter(|&k| k > v)
            .min_by(f64::total_cmp)
        && let Some((du, dv)) = surface
            .trim([d[0], d[1], v, b])?
            .evaluate(u, v)?
            .first_derivatives()
    {
        us.push(du);
        vs.push(dv);
    }
    Ok((us, vs))
}
/// Unit-tangent sine at a parameter pair; zero/invalid tangents stay 0. At a
/// C0 knot every one-sided side pair is screened and the largest sine counts:
/// a crossing transverse from either side is resolvable.
pub(crate) fn tangent_sine(first: &Curve, second: &Curve, t: f64, u: f64) -> Result<f64> {
    let mut best: f64 = 0.;
    for va in curve_tangents(first, t)? {
        for vb in curve_tangents(second, u)? {
            let la = distance(va, [0.; 3]);
            let lb = distance(vb, [0.; 3]);
            if !la.is_finite() || la <= 0. || !lb.is_finite() || lb <= 0. {
                continue;
            }
            let c = cross(va.map(|x| x / la), vb.map(|x| x / lb));
            best = best.max(distance(c, [0.; 3]));
        }
    }
    Ok(best)
}
