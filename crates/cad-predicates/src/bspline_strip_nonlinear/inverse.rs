//! Exact inverse features and rational-coordinate recovery for seam clocks.
use super::*;

// An exact projective linear inverse proposes a chart for general conics and
// full-rank spatial cubics. A/D can be nonlinear in every Cartesian coordinate.
// Free variables are fixed to zero; all final map identities remain mandatory.
pub(super) fn projective_coordinate(
    h: &H,
    power: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<[Q; 4]>, Reason> {
    let n = h.iter().map(Vec::len).max().unwrap().max(power + 1);
    let mut rows = (0..n)
        .map(|i| {
            let mut r = (0..4)
                .map(|k| h[k].get(i).cloned().unwrap_or_else(Q::zero))
                .collect::<Vec<_>>();
            r.push(if i == power { one() } else { Q::zero() });
            r
        })
        .collect::<Vec<_>>();
    let mut pivots = Vec::new();
    let mut r = 0;
    for col in 0..4 {
        let Some(at) = (r..n).find(|&j| rows[j][col].n.sign() != Sign::Zero) else {
            continue;
        };
        rows.swap(r, at);
        if rows[r][col].n.sign() == Sign::Negative {
            for k in col..=4 {
                rows[r][k] = Q::zero().sub(&rows[r][k], ctx)?;
            }
        }
        let pivot = rows[r][col].clone();
        for k in col..=4 {
            if rows[r][k].n.sign() != Sign::Zero {
                rows[r][k] = rows[r][k].div_positive(&pivot, ctx)?;
            }
        }
        for j in 0..n {
            if j == r {
                continue;
            }
            let factor = rows[j][col].clone();
            for k in col..=4 {
                rows[j][k] = rows[j][k].sub(&factor.mul(&rows[r][k], ctx)?, ctx)?;
            }
        }
        pivots.push(col);
        r += 1;
        if r == n {
            break;
        }
    }
    if rows
        .iter()
        .any(|r| r[..4].iter().all(|v| v.n.sign() == Sign::Zero) && r[4].n.sign() != Sign::Zero)
    {
        return Ok(None);
    }
    let mut answer = std::array::from_fn(|_| Q::zero());
    for (row, &col) in pivots.iter().enumerate() {
        answer[col] = rows[row][4].clone();
    }
    Ok(Some(answer))
}

// Solve L.H(s) - s M.H(s) == 0 jointly. Unlike two prescribed constant
// inverse coordinates, the denominator may be a nonconstant polynomial.
// Nullspace vectors are proposals only. Removable factors are cancelled exactly
// on the target before the complete positive map/jet/ordered-cover checks.
pub(super) fn inverse_features(
    h: &H,
    degree: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<P>, Reason> {
    if degree == 1 {
        return Ok(h.to_vec());
    }
    let mut basis = Vec::new();
    for i in 0..4 {
        for j in i..4 {
            if degree == 2 {
                basis.push(mul(&h[i], &h[j], ctx)?);
            } else {
                for k in j..4 {
                    basis.push(mul(&mul(&h[i], &h[j], ctx)?, &h[k], ctx)?);
                }
            }
        }
    }
    Ok(basis)
}

// Evaluate only the nonzero coefficients of the exact inverse witness.
// Both forms share each selected homogeneous monomial and each quadratic pair.
pub(super) fn inverse_forms(
    h: &H,
    l: &[Q],
    m: &[Q],
    degree: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<(P, P), Reason> {
    if degree == 1 {
        return Ok((feature_form(h, l, ctx)?, feature_form(h, m, ctx)?));
    }
    let (mut n, mut d) = (vec![Q::zero()], vec![Q::zero()]);
    let mut at = 0;
    for i in 0..4 {
        for j in i..4 {
            let mut pair: Option<P> = None;
            let count = if degree == 2 { 1 } else { 4 - j };
            for k in 0..count {
                if l[at].n.sign() != Sign::Zero || m[at].n.sign() != Sign::Zero {
                    if pair.is_none() {
                        pair = Some(mul(&h[i], &h[j], ctx)?);
                    }
                    let feature = if degree == 2 {
                        pair.as_ref().unwrap().clone()
                    } else {
                        mul(pair.as_ref().unwrap(), &h[j + k], ctx)?
                    };
                    if l[at].n.sign() != Sign::Zero {
                        n = add(&n, &scaled(&feature, &l[at], ctx)?, ctx)?;
                    }
                    if m[at].n.sign() != Sign::Zero {
                        d = add(&d, &scaled(&feature, &m[at], ctx)?, ctx)?;
                    }
                }
                at += 1;
            }
        }
    }
    Ok((n, d))
}

pub(super) fn feature_form(h: &[P], c: &[Q], ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let mut p = vec![Q::zero()];
    for (k, v) in c.iter().enumerate() {
        if v.n.sign() == Sign::Zero {
            continue;
        }
        p = add(&p, &scaled(&h[k], v, ctx)?, ctx)?;
    }
    Ok(p)
}

pub(super) fn normalize_inverse_coordinates(
    h: &H,
    scales: &[Q; 4],
    ctx: &mut PredicateContext<'_>,
) -> Result<H, Reason> {
    let mut out = h.clone();
    for k in 0..4 {
        if !scales[k].equal(&one(), ctx)? {
            out[k] = h[k]
                .iter()
                .map(|v| v.div_positive(&scales[k], ctx))
                .collect::<Result<Vec<_>, _>>()?;
        }
    }
    Ok(out)
}

pub(super) fn coupled_coordinate(
    h: &H,
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<(Vec<Q>, Vec<Q>, usize, [Q; 4])>, Reason> {
    // Exact positive diagonal coordinate normalization is an invertible proposal
    // change only. The very same scales are used on B; original jets remain
    // untouched and all final composition identities still use original A/B.
    let mut scales = std::array::from_fn(|_| one());
    for k in 0..4 {
        if let Some(c) = h[k].iter().find(|v| v.n.sign() != Sign::Zero) {
            scales[k] = if c.n.sign() == Sign::Negative {
                Q::zero().sub(c, ctx)?
            } else {
                c.clone()
            };
        }
    }
    let normalized = normalize_inverse_coordinates(h, &scales, ctx)?;
    for degree in [1, 2, 3] {
        let features = inverse_features(&normalized, degree, ctx)?;
        if let Some((l, m)) = solve_coupled(&features, ctx)? {
            return Ok(Some((l, m, degree, scales)));
        }
    }
    Ok(None)
}

pub(super) fn solve_coupled(
    h: &[P],
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<(Vec<Q>, Vec<Q>)>, Reason> {
    let original_width = h.len();
    let mut active = Vec::new();
    'features: for i in 0..h.len() {
        if zero(&h[i]) {
            continue;
        }
        for &j in &active {
            let other: &P = &h[j];
            if h[i].len() == other.len() {
                let mut equal = true;
                for (a, b) in h[i].iter().zip(other) {
                    if !a.equal(b, ctx)? {
                        equal = false;
                        break;
                    }
                }
                if equal {
                    continue 'features;
                }
            }
        }
        active.push(i);
    }
    let original = h;
    let compact = active.iter().map(|&i| h[i].clone()).collect::<Vec<_>>();
    let h = &compact;
    let width = h.len();
    if width == 0 {
        return Ok(None);
    }
    let columns = 2 * width;
    let n = h.iter().map(Vec::len).max().unwrap() + 1;
    let mut rows = (0..n)
        .map(|i| {
            let mut row = (0..width)
                .map(|k| h[k].get(i).cloned().unwrap_or_else(Q::zero))
                .collect::<Vec<_>>();
            row.extend(
                (0..width)
                    .map(|k| {
                        if i == 0 {
                            Ok(Q::zero())
                        } else {
                            Q::zero().sub(&h[k].get(i - 1).cloned().unwrap_or_else(Q::zero), ctx)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
            Ok(row)
        })
        .collect::<Result<Vec<_>, Reason>>()?;
    let mut pivots = Vec::new();
    let mut r = 0;
    for col in 0..columns {
        let Some(at) = (r..n).find(|&j| rows[j][col].n.sign() != Sign::Zero) else {
            continue;
        };
        rows.swap(r, at);
        if rows[r][col].n.sign() == Sign::Negative {
            for k in col..columns {
                rows[r][k] = Q::zero().sub(&rows[r][k], ctx)?;
            }
        }
        let pivot = rows[r][col].clone();
        for k in col..columns {
            if rows[r][k].n.sign() != Sign::Zero {
                rows[r][k] = rows[r][k].div_positive(&pivot, ctx)?;
            }
        }
        for j in r + 1..n {
            if j != r {
                let factor = rows[j][col].clone();
                if factor.n.sign() != Sign::Zero {
                    for k in col..columns {
                        if rows[r][k].n.sign() != Sign::Zero {
                            rows[j][k] = rows[j][k].sub(&factor.mul(&rows[r][k], ctx)?, ctx)?;
                        }
                    }
                }
            }
        }
        pivots.push(col);
        r += 1;
        if r == n {
            break;
        }
    }
    for free in 0..columns {
        if pivots.contains(&free) {
            continue;
        }
        let mut candidate = vec![Q::zero(); columns];
        candidate[free] = one();
        for (row, &col) in pivots.iter().enumerate().rev() {
            let mut value = Q::zero();
            for k in col + 1..columns {
                if rows[row][k].n.sign() != Sign::Zero && candidate[k].n.sign() != Sign::Zero {
                    value = value.sub(&rows[row][k].mul(&candidate[k], ctx)?, ctx)?;
                }
            }
            candidate[col] = value;
        }
        let l = candidate[..width].to_vec();
        let m = candidate[width..].to_vec();
        let denominator = feature_form(h, &m, ctx)?;
        if zero(&denominator) {
            continue;
        }
        let numerator = feature_form(h, &l, ctx)?;
        if zero(&sub(
            &numerator,
            &mul(&vec![Q::zero(), one()], &denominator, ctx)?,
            ctx,
        )?) {
            let mut full_l = vec![Q::zero(); original_width];
            let mut full_m = full_l.clone();
            for (k, &i) in active.iter().enumerate() {
                full_l[i] = l[k].clone();
                full_m[i] = m[k].clone();
            }
            // The compact basis is an exact deletion of zero/duplicate rows.
            // Recheck the lifted witness against the original feature net.
            let ln = feature_form(original, &full_l, ctx)?;
            let md = feature_form(original, &full_m, ctx)?;
            if zero(&sub(&ln, &mul(&vec![Q::zero(), one()], &md, ctx)?, ctx)?) {
                return Ok(Some((full_l, full_m)));
            }
        }
    }
    Ok(None)
}

// Exact Euclidean cancellation of a proposed rational map, including removable
// endpoint factors. This is not acceptance: denominator, derivative, cover and
// every original composed jet are still checked afterwards in the same ledger.
pub(super) fn reduce_ratio(n: P, d: P, ctx: &mut PredicateContext<'_>) -> Result<(P, P), Reason> {
    fn division(a: &P, b: &P, ctx: &mut PredicateContext<'_>) -> Result<(P, P), Reason> {
        if zero(b) {
            return Err(Reason::MissingProof);
        }
        let mut rem = trim(a.clone());
        let mut q = vec![Q::zero(); a.len().saturating_sub(b.len()) + 1];
        while !zero(&rem) && rem.len() >= b.len() {
            let i = rem.len() - b.len();
            let lead = b.last().unwrap();
            let sign = if lead.n.sign() == Sign::Negative {
                Q::zero().sub(&one(), ctx)?
            } else {
                one()
            };
            let coefficient = rem
                .last()
                .unwrap()
                .mul(&sign, ctx)?
                .div_positive(&lead.mul(&sign, ctx)?, ctx)?;
            q[i] = coefficient.clone();
            for j in 0..b.len() {
                rem[i + j] = rem[i + j].sub(&coefficient.mul(&b[j], ctx)?, ctx)?;
            }
            rem = trim(rem);
        }
        Ok((trim(q), rem))
    }
    if zero(&d) {
        return Err(Reason::MissingProof);
    }
    let mut a = trim(n.clone());
    let mut b = trim(d.clone());
    while !zero(&b) {
        let (_, r) = division(&a, &b, ctx)?;
        a = b;
        b = r;
    }
    let (nn, nr) = division(&n, &a, ctx)?;
    let (dd, dr) = division(&d, &a, ctx)?;
    if !zero(&nr) || !zero(&dr) {
        return Err(Reason::MissingProof);
    }
    // Euclidean gcds are defined up to a nonzero constant. Remove that common
    // gauge exactly before high-order projective jet products; no division of
    // coordinates or change of the proposed rational function is involved.
    let lead = dd
        .iter()
        .find(|v| v.n.sign() != Sign::Zero)
        .ok_or(Reason::MissingProof)?;
    let factor = if lead.n.sign() == Sign::Negative {
        Q::zero().sub(lead, ctx)?
    } else {
        lead.clone()
    };
    let nn = nn
        .iter()
        .map(|v| v.div_positive(&factor, ctx))
        .collect::<Result<P, _>>()?;
    let dd = dd
        .iter()
        .map(|v| v.div_positive(&factor, ctx))
        .collect::<Result<P, _>>()?;
    Ok((nn, dd))
}
