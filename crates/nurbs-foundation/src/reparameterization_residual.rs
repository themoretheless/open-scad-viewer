//! Whole-cell original rational composition residual; no fitted/exact promotion.
use super::preimages::Map;
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
fn choose(n: usize, k: usize) -> Result<I> {
    (0..k.min(n - k)).try_fold(I::point(1.), |v, i| {
        v.mul(I::point((n - i) as f64))?
            .div(I::point((i + 1) as f64))
    })
}
fn product(a: &[I], b: &[I]) -> Result<Vec<I>> {
    let (n, m) = (a.len() - 1, b.len() - 1);
    let mut result = vec![I::point(0.); n + m + 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            let factor = choose(n, i)?
                .mul(choose(m, j)?)?
                .div(choose(n + m, i + j)?)?;
            result[i + j] = result[i + j].add(x.mul(*y)?.mul(factor)?)?;
        }
    }
    Ok(result)
}
fn restrict(
    net: &[Vec<I>],
    knots: &[f64],
    degree: usize,
    span: usize,
    domain: [f64; 2],
) -> Result<Vec<Vec<I>>> {
    let mut result = Vec::new();
    for k in 0..=degree {
        let mut q = net[span - degree..=span].to_vec();
        for r in 1..=degree {
            let parameter = if r - 1 < degree - k {
                domain[0]
            } else {
                domain[1]
            };
            for j in (r..=degree).rev() {
                let i = span - degree + j;
                let alpha = I::point(parameter)
                    .sub(I::point(knots[i]))?
                    .div(I::point(knots[i + degree - r + 1]).sub(I::point(knots[i]))?)?
                    .intersect(0., 1.)?;
                let beta = I::point(1.).sub(alpha)?.intersect(0., 1.)?;
                for axis in 0..q[j].len() {
                    let a = q[j - 1][axis];
                    let b = q[j][axis];
                    q[j][axis] = a
                        .mul(beta)?
                        .add(b.mul(alpha)?)?
                        .intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
                }
            }
        }
        result.push(q[degree].clone());
    }
    Ok(result)
}

fn span(curve: &Curve, range: I) -> Option<usize> {
    (curve.degree..curve.control_points.len()).find(|&i| {
        curve.knots[i] < curve.knots[i + 1]
            && curve.knots[i] <= range.lo
            && range.hi <= curve.knots[i + 1]
    })
}
fn curve_net(curve: &Curve, owner: usize, domain: [f64; 2], origin: &[f64]) -> Result<Vec<Vec<I>>> {
    let scale = curve.weights.iter().copied().fold(0., f64::max);
    let rows = curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(point, &weight)| {
            let w = I::point(weight).div(I::point(scale))?;
            let mut row = point
                .iter()
                .zip(origin)
                .map(|(&x, &o)| I::point(x).sub(I::point(o))?.mul(w))
                .collect::<Result<Vec<_>>>()?;
            row.push(w);
            Ok(row)
        })
        .collect::<Result<Vec<_>>>()?;
    restrict(&rows, &curve.knots, curve.degree, owner, domain)
}
fn compose(rows: &[Vec<I>], p: &[I], q: &[I], domain: [f64; 2]) -> Result<Vec<Vec<I>>> {
    let n = rows.len() - 1;
    check(
        n * (p.len() - 1) <= 25,
        "Composed residual basis exceeds degree25",
    )?;
    let extent = I::point(domain[1]).sub(I::point(domain[0]))?;
    let a = p
        .iter()
        .zip(q)
        .map(|(p, q)| p.sub(q.mul(I::point(domain[0]))?)?.div(extent))
        .collect::<Result<Vec<_>>>()?;
    let b = q
        .iter()
        .zip(&a)
        .map(|(q, a)| q.sub(*a))
        .collect::<Result<Vec<_>>>()?;
    let powers = |x: &[I]| -> Result<Vec<Vec<I>>> {
        let mut powers = vec![vec![I::point(1.)]];
        for i in 0..n {
            powers.push(product(&powers[i], x)?);
        }
        Ok(powers)
    };
    let ap = powers(&a)?;
    let bp = powers(&b)?;
    let mut out = vec![vec![I::point(0.); rows[0].len()]; n * (p.len() - 1) + 1];
    for i in 0..=n {
        let term = product(&ap[i], &bp[n - i])?;
        let c = choose(n, i)?;
        for (destination, t) in out.iter_mut().zip(term) {
            for (value, &coefficient) in destination.iter_mut().zip(&rows[i]) {
                *value = value.add(t.mul(c)?.mul(coefficient)?)?;
            }
        }
    }
    Ok(out)
}
fn apply(
    map: &Map,
    p: &mut Vec<I>,
    q: &mut Vec<I>,
    range: &mut I,
    work: &mut usize,
    limit: usize,
) -> Result<bool> {
    match map {
        Map::Composition(parts) => {
            for part in parts {
                if !apply(part, p, q, range, work, limit)? {
                    return Ok(false);
                }
            }
        }
        Map::Pieces(pieces) => {
            let Some(piece) = pieces
                .iter()
                .find(|piece| piece.domain[0] <= range.lo && range.hi <= piece.domain[1])
            else {
                return Ok(false);
            };
            let next = map.enclosure(*range, work, limit)?;
            let scale = piece.weights.iter().copied().fold(0., f64::max);
            let rows = piece
                .values
                .iter()
                .zip(&piece.weights)
                .map(|(&x, &weight)| {
                    let w = I::point(weight).div(I::point(scale))?;
                    Ok(vec![I::point(x).mul(w)?, w])
                })
                .collect::<Result<Vec<_>>>()?;
            let composed = compose(&rows, p, q, piece.domain)?;
            *p = composed.iter().map(|row| row[0]).collect();
            *q = composed.iter().map(|row| row[1]).collect();
            *range = next;
        }
    }
    Ok(true)
}
/// None means the cell spans a map/source/result knot or its positive
/// denominator enclosure is unresolved; the caller must use another proof.
pub(super) fn bound(
    source: &Curve,
    map: &Map,
    result: &Curve,
    domain: [f64; 2],
    work: &mut usize,
    limit: usize,
    target_rows: Option<&[Vec<I>]>,
) -> Result<Option<f64>> {
    let Some(target_span) = span(result, I::new(domain[0], domain[1])?) else {
        return Ok(None);
    };
    let mut p = vec![I::point(domain[0]), I::point(domain[1])];
    let mut q = vec![I::point(1.); 2];
    let mut range = I::new(domain[0], domain[1])?;
    if !apply(map, &mut p, &mut q, &mut range, work, limit)? {
        return Ok(None);
    }
    let Some(source_span) = span(source, range) else {
        return Ok(None);
    };
    let source_domain = [source.knots[source_span], source.knots[source_span + 1]];
    let origin = &source.control_points[0];
    let original = curve_net(source, source_span, source_domain, origin)?;
    let original = compose(&original, &p, &q, source_domain)?;
    let target = if let Some(rows) = target_rows {
        restrict(rows, &result.knots, result.degree, target_span, domain)?
    } else {
        curve_net(result, target_span, domain, origin)?
    };
    let dimension = origin.len();
    let column = |rows: &[Vec<I>], k: usize| rows.iter().map(|r| r[k]).collect::<Vec<_>>();
    let sw = column(&original, dimension);
    let tw = column(&target, dimension);
    let denominator = product(&sw, &tw)?;
    let minimum = denominator
        .iter()
        .map(|v| v.lo)
        .fold(f64::INFINITY, f64::min);
    if minimum <= 0. {
        return Ok(None);
    }
    let mut squared = I::point(0.);
    for k in 0..dimension {
        let a = product(&column(&original, k), &tw)?;
        let b = product(&column(&target, k), &sw)?;
        let mut maximum: f64 = 0.;
        for (a, b) in a.iter().zip(b) {
            let v = a.sub(b)?;
            maximum = maximum.max(v.lo.abs().max(v.hi.abs()));
        }
        let bound = I::point(maximum).div(I::point(minimum))?;
        squared = squared.add(bound.mul(bound)?)?;
    }
    Ok(Some(squared.hi.sqrt().next_up()))
}

fn literal_point(curve: &Curve, t: f64, origin: &[f64]) -> Result<Vec<I>> {
    let [a, b] = curve.domain();
    let p = curve.degree;
    let owner = if t == b {
        (p..curve.control_points.len())
            .rev()
            .find(|&i| curve.knots[i] < curve.knots[i + 1])
    } else {
        (p..curve.control_points.len()).find(|&i| curve.knots[i] <= t && t < curve.knots[i + 1])
    }
    .ok_or_else(|| crate::input("Endpoint is outside the curve cover"))?;
    let control = if t == a && curve.knots[..=p].iter().all(|&k| k == a) {
        Some(0)
    } else if t == b
        && curve.knots[curve.control_points.len()..]
            .iter()
            .all(|&k| k == b)
    {
        Some(curve.control_points.len() - 1)
    } else if curve.knots.iter().filter(|&&k| k == t).count() == p + 1 {
        Some(owner - p)
    } else {
        None
    };
    if let Some(control) = control {
        return curve.control_points[control]
            .iter()
            .zip(origin)
            .map(|(&x, &o)| I::point(x).sub(I::point(o)))
            .collect();
    }
    let rows = curve_net(curve, owner, [t, t], origin)?;
    let dimension = origin.len();
    let w = I::new(
        rows.iter()
            .map(|r| r[dimension].lo)
            .fold(f64::INFINITY, f64::min),
        rows.iter()
            .map(|r| r[dimension].hi)
            .fold(f64::NEG_INFINITY, f64::max),
    )?;
    (0..dimension)
        .map(|k| {
            I::new(
                rows.iter().map(|r| r[k].lo).fold(f64::INFINITY, f64::min),
                rows.iter()
                    .map(|r| r[k].hi)
                    .fold(f64::NEG_INFINITY, f64::max),
            )?
            .div(w)
        })
        .collect()
}
/// Declared monotone-map endpoint values are exact authored scalar endpoints.
/// Use the curve's right-sided knot convention, including partial-map endpoints.
pub(super) fn endpoints(
    source: &Curve,
    range: [f64; 2],
    result: &Curve,
    domain: [f64; 2],
    tolerance: f64,
) -> Result<(f64, Option<f64>)> {
    let mut maximum: f64 = 0.;
    let mut witness = None;
    for i in 0..2 {
        let source_point = literal_point(source, range[i], &source.control_points[0])?;
        let result_point = literal_point(result, domain[i], &source.control_points[0])?;
        let (lower, upper) = crate::distance_bounds::box_distance(&source_point, &result_point)?;
        maximum = maximum.max(upper);
        if lower > tolerance {
            witness = Some(domain[i]);
        }
    }
    Ok((maximum, witness))
}

/// Original tensor coefficients at fixed V, kept homogeneous and outward.
/// No rounded Euclidean isocurve is used as the geometry authority.
pub(super) fn surface_section_rows(
    surface: &crate::surface::Surface,
    v: f64,
    origin: &[f64],
) -> Result<Vec<Vec<I>>> {
    surface.validate()?;
    check(
        !surface.periodic_u && !surface.periodic_v && v.is_finite(),
        "Open tensor section required",
    )?;
    let p = surface.degree_v;
    let n = surface.control_points[0].len();
    let knots = &surface.knots_v;
    check(
        v >= knots[p] && v <= knots[n],
        "Section station outside tensor domain",
    )?;
    let owner = if v == knots[n] {
        (p..n).rev().find(|&j| knots[j] < knots[j + 1])
    } else {
        (p..n).find(|&j| knots[j] <= v && v < knots[j + 1])
    }
    .unwrap();
    let scale = surface.weights.iter().flatten().copied().fold(0., f64::max);
    surface
        .control_points
        .iter()
        .zip(&surface.weights)
        .map(|(points, weights)| {
            let mut rows = (owner - p..=owner)
                .map(|j| {
                    let w = I::point(weights[j]).div(I::point(scale))?;
                    let mut row = points[j]
                        .iter()
                        .zip(origin)
                        .map(|(&x, &o)| I::point(x).sub(I::point(o))?.mul(w))
                        .collect::<Result<Vec<_>>>()?;
                    row.push(w);
                    Ok(row)
                })
                .collect::<Result<Vec<_>>>()?;
            for r in 1..=p {
                for j in (r..=p).rev() {
                    let i = owner - p + j;
                    let a = I::point(v)
                        .sub(I::point(knots[i]))?
                        .div(I::point(knots[i + p - r + 1]).sub(I::point(knots[i]))?)?
                        .intersect(0., 1.)?;
                    let b = I::point(1.).sub(a)?.intersect(0., 1.)?;
                    for k in 0..rows[j].len() {
                        let x = rows[j - 1][k];
                        let y = rows[j][k];
                        rows[j][k] = x
                            .mul(b)?
                            .add(y.mul(a)?)?
                            .intersect(x.lo.min(y.lo), x.hi.max(y.hi))?;
                    }
                }
            }
            Ok(rows[p].clone())
        })
        .collect()
}
pub(super) fn original_curve_rows(curve: &Curve, origin: &[f64]) -> Result<Vec<Vec<I>>> {
    let scale = curve.weights.iter().copied().fold(0., f64::max);
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(point, &weight)| {
            let w = I::point(weight).div(I::point(scale))?;
            let mut row = point
                .iter()
                .zip(origin)
                .map(|(&x, &o)| I::point(x).sub(I::point(o))?.mul(w))
                .collect::<Result<Vec<_>>>()?;
            row.push(w);
            Ok(row)
        })
        .collect()
}
/// Conservative common-origin boxes, including all original nonempty spans.
pub(super) fn field_bounds(meta: &Curve, rows: &[Vec<I>], range: I) -> Result<Vec<I>> {
    let dimension = rows[0].len() - 1;
    let mut bounds = vec![[f64::INFINITY, f64::NEG_INFINITY]; dimension];
    for owner in meta.degree..meta.control_points.len() {
        if meta.knots[owner] >= meta.knots[owner + 1] {
            continue;
        }
        let a = range.lo.max(meta.knots[owner]);
        let b = range.hi.min(meta.knots[owner + 1]);
        if a > b {
            continue;
        }
        let net = restrict(rows, &meta.knots, meta.degree, owner, [a, b])?;
        let w = I::new(
            net.iter()
                .map(|r| r[dimension].lo)
                .fold(f64::INFINITY, f64::min),
            net.iter()
                .map(|r| r[dimension].hi)
                .fold(f64::NEG_INFINITY, f64::max),
        )?;
        for k in 0..dimension {
            let x = I::new(
                net.iter().map(|r| r[k].lo).fold(f64::INFINITY, f64::min),
                net.iter()
                    .map(|r| r[k].hi)
                    .fold(f64::NEG_INFINITY, f64::max),
            )?
            .div(w)?;
            bounds[k][0] = bounds[k][0].min(x.lo);
            bounds[k][1] = bounds[k][1].max(x.hi);
        }
    }
    bounds.into_iter().map(|[a, b]| I::new(a, b)).collect()
}
pub(super) fn surface_endpoints(
    source: &Curve,
    range: [f64; 2],
    meta: &Curve,
    rows: &[Vec<I>],
    domain: [f64; 2],
    tolerance: f64,
) -> Result<(f64, Option<f64>)> {
    let mut upper: f64 = 0.;
    let mut witness = None;
    for i in 0..2 {
        let a = literal_point(source, range[i], &source.control_points[0])?;
        let b = field_bounds(meta, rows, I::point(domain[i]))?;
        let (lower, error) = crate::distance_bounds::box_distance(&a, &b)?;
        upper = upper.max(error);
        if lower > tolerance {
            witness = Some(domain[i]);
        }
    }
    Ok((upper, witness))
}
