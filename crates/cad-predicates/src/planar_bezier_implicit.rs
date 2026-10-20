//! Moving-line implicitization of a proper planar quadratic/cubic carrier.
//! All polynomial identities use the original homogeneous source expansions.
//! A linear rational inverse filters the complete eliminant census to [0,1].
use super::*;
#[derive(Clone, Debug)]
pub struct PlanarPairRoot {
    pub first: [f64; 2],
    pub second: [f64; 2],
    /// Multiplicity in the implicit eliminant (may combine singular branches).
    pub multiplicity: usize,
}
#[derive(Clone, Debug)]
pub enum PlanarPairOutcome {
    Roots(Vec<PlanarPairRoot>),
    /// Equality of implicit carriers alone does not establish finite overlap.
    CommonCarrier,
    NotReducible,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct PlanarPairDecision {
    pub outcome: PlanarPairOutcome,
    pub work_used: u64,
    pub context: ContextIdentity,
    /// Last entered computation phase, including on budget/precision refusal.
    pub phase: &'static str,
}
pub struct PlanarEndpointSeparation {
    /// Sufficient whole-carrier separation except exact shared curve endpoints.
    /// False is unproven, never a demonstrated finite-arc intersection.
    pub proven: bool,
    pub reason: Option<Reason>,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// Direct Bernstein conic separation. A proper original quadratic is the
/// carrier; the original rational query has degree 1..8. No power eliminant,
/// cropped curve, inverse fit or sampled sign grants admission.
pub fn planar_bezier_endpoint_separation(
    ctx: &mut PredicateContext<'_>,
    a: &[[LeafRef; 3]],
    b: &[[LeafRef; 3]],
) -> Result<PlanarEndpointSeparation, InputError> {
    if a.len() != 3 || !(2..=9).contains(&b.len()) {
        return Err(InputError::InvalidInput(
            "Quadratic carrier and degree 1..8 query required",
        ));
    }
    let input = a
        .iter()
        .chain(b)
        .flatten()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let computed = (|| -> Result<bool, Reason> {
        let exact = exact_inputs(&input, ctx)?;
        let points = exact
            .chunks_exact(3)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect::<Vec<[Expansion; 3]>>();
        if points.iter().any(|p| p[2].sign() != Sign::Positive) {
            return Err(Reason::MissingProof);
        }
        let (aa, bb) = points.split_at(3);
        let homogeneous = |p: &[[Expansion; 3]],
                           ctx: &mut PredicateContext<'_>|
         -> Result<Vec<[Expansion; 3]>, Reason> {
            let mut common = true;
            for row in &p[1..] {
                if row[2].sub(&p[0][2], ctx)?.sign() != Sign::Zero {
                    common = false;
                    break;
                }
            }
            p.iter()
                .map(|row| {
                    Ok(if common {
                        [row[0].clone(), row[1].clone(), Expansion::scalar(1.)]
                    } else {
                        [
                            row[0].mul(&row[2], ctx)?,
                            row[1].mul(&row[2], ctx)?,
                            row[2].clone(),
                        ]
                    })
                })
                .collect()
        };
        let h = homogeneous(aa, ctx)?;
        let query = homogeneous(bb, ctx)?;
        let mut dual = Vec::new();
        for k in 0..3 {
            let (u, v) = ((k + 1) % 3, (k + 2) % 3);
            dual.push(
                (0..3)
                    .map(|j| {
                        let (x, y) = ((j + 1) % 3, (j + 2) % 3);
                        h[u][x]
                            .mul(&h[v][y], ctx)?
                            .sub(&h[u][y].mul(&h[v][x], ctx)?, ctx)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        let scalar = |row: &[Expansion],
                      point: &[Expansion; 3],
                      ctx: &mut PredicateContext<'_>|
         -> Result<Expansion, Reason> {
            let mut value = Expansion::scalar(0.);
            for k in 0..3 {
                value = value.add(&row[k].mul(&point[k], ctx)?, ctx)?;
            }
            Ok(value)
        };
        if scalar(&dual[0], &h[0], ctx)?.sign() == Sign::Zero {
            return Ok(false);
        }
        let traces = dual
            .iter()
            .map(|row| {
                query
                    .iter()
                    .map(|point| scalar(row, point, ctx))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let n = b.len() - 1;
        let degree = 2 * n;
        let gcd = |mut x: u64, mut y: u64| {
            while y != 0 {
                let z = x % y;
                x = y;
                y = z;
            }
            x
        };
        let mut scale = 1u64;
        for k in 0..=degree {
            let c = choose(degree, k);
            scale = (scale / gcd(scale, c))
                .checked_mul(c)
                .ok_or(Reason::ResourceLimit)?;
        }
        let mut coefficients = Vec::new();
        for k in 0..=degree {
            let mut value = Expansion::scalar(0.);
            for i in k.saturating_sub(n)..=k.min(n) {
                let j = k - i;
                let factor = choose(n, i) * choose(n, j) * (scale / choose(degree, k));
                let term = traces[1][i].mul(&traces[1][j], ctx)?.sub(
                    &traces[0][i]
                        .mul(&traces[2][j], ctx)?
                        .mul(&Expansion::scalar(4.), ctx)?,
                    ctx,
                )?;
                value = value.add(&term.mul(&Expansion::integer(factor, ctx)?, ctx)?, ctx)?;
            }
            coefficients.push(value);
        }
        for end in [0, 1] {
            let q = if end == 0 { 0 } else { n };
            let k = if end == 0 { 0 } else { degree };
            if coefficients[k].sign() == Sign::Zero {
                let mut shared = false;
                for s in [0, 2] {
                    if bb[q][0].sub(&aa[s][0], ctx)?.sign() == Sign::Zero
                        && bb[q][1].sub(&aa[s][1], ctx)?.sign() == Sign::Zero
                    {
                        shared = true;
                    }
                }
                if !shared {
                    return Ok(false);
                }
            }
        }
        conic_bernstein_no_interior_roots(&coefficients, ctx)
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(PlanarEndpointSeparation {
        proven: matches!(computed, Ok(true)),
        reason: match computed {
            Ok(true) => None,
            Ok(false) => Some(Reason::MissingProof),
            Err(e) => Some(e),
        },
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
fn conic_bernstein_no_interior_roots(
    coefficients: &[Expansion],
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    use crate::arithmetic::Interval as I;
    let n = coefficients.len() - 1;
    let controls = coefficients
        .iter()
        .map(|v| {
            if v.sign() == Sign::Zero {
                return Ok(I::point(0.));
            }
            let mut b = v.enclosure(ctx)?;
            if v.sign() == Sign::Positive {
                b.lo = b.lo.max(0.);
            } else {
                b.hi = b.hi.min(0.);
            }
            Ok(b)
        })
        .collect::<Result<Vec<_>, Reason>>()?;
    let positive = |v: &[I]| v.iter().all(|x| x.lo >= 0.) && v.iter().any(|x| x.lo > 0.);
    let negative = |v: &[I]| v.iter().all(|x| x.hi <= 0.) && v.iter().any(|x| x.hi < 0.);
    let mut pending = vec![controls];
    while let Some(b) = pending.pop() {
        ctx.charge(1)?;
        if positive(&b) || negative(&b) {
            continue;
        }
        let d = b
            .windows(2)
            .map(|v| v[1].sub(&v[0], ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let increasing=positive(&d);let decreasing=negative(&d);
        if (increasing && (b[0].lo >= 0. || b[n].hi <= 0.))
            || (decreasing && (b[0].hi <= 0. || b[n].lo >= 0.))
        {
            continue;
        }
        if (increasing||decreasing)&&((b[0].hi<0.&&b[n].lo>0.)||(b[0].lo>0.&&b[n].hi<0.)) {
            return Ok(false);
        }
        let mut levels = vec![b];
        for k in 1..=n {
            levels.push(
                levels[k - 1]
                    .windows(2)
                    .map(|v| {
                        if v.iter().all(|x| x.lo == 0. && x.hi == 0.) {
                            Ok(I::point(0.))
                        } else {
                            v[0].add(&v[1], ctx)?.mul(&I::point(0.5), ctx)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        // Every new split point is itself covered. Ambiguous or true interior
        // contact refuses; it is never hidden as a child-cell endpoint.
        if levels[n][0].sign().is_none() {
            return Ok(false);
        }
        pending.push(levels.iter().map(|row| row[0]).collect());
        pending.push(
            levels
                .iter()
                .rev()
                .map(|row| *row.last().unwrap())
                .collect(),
        );
    }
    Ok(true)
}

fn gcd(a: &Poly, b: &Poly, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    let (mut a, mut b) = (a.clone(), b.clone());
    normalize(&mut a, ctx)?;
    normalize(&mut b, ctx)?;
    while !zero(&b) {
        let (_, mut r, _) = pseudo_divide(&a, &b, ctx)?;
        normalize(&mut r, ctx)?;
        a = b;
        b = r;
    }
    Ok(a)
}
/// Cancel a common homogeneous polynomial factor without division/rounded
/// reconstruction. Pseudo-quotients can have different scalar multipliers;
/// align them by exact coefficient products and check all source identities.
/// The original positive-weight denominator has no zero on [0,1], so its
/// exact common factor and the reduced denominator have none either.
fn primitive_carrier(a: [Poly; 3], ctx: &mut PredicateContext<'_>) -> Result<[Poly; 3], Reason> {
    // A nonzero constant homogeneous denominator already proves the common
    // polynomial factor is constant. Computing gcd(X,Y) first can exhaust the
    // work budget on approximate high-degree position coefficients even though
    // W immediately rules out every nonconstant common factor.
    if a[2].len() == 1 && !zero(&a[2]) {
        return Ok(a);
    }
    let g = gcd(&gcd(&a[0], &a[1], ctx)?, &a[2], ctx)?;
    if g.len() <= 1 {
        return Ok(a);
    }
    let mut quotients = Vec::new();
    let mut source_coefficients = Vec::new();
    let mut product_coefficients = Vec::new();
    for p in &a {
        if zero(p) {
            quotients.push(vec![Expansion::scalar(0.)]);
            source_coefficients.push(Expansion::scalar(1.));
            product_coefficients.push(Expansion::scalar(1.));
            continue;
        }
        let (q, r, _) = pseudo_divide(p, &g, ctx)?;
        if !zero(&r) {
            return Err(Reason::MissingProof);
        }
        let product = mul(&q, &g, ctx)?;
        let k = p
            .iter()
            .position(|v| v.sign() != Sign::Zero)
            .ok_or(Reason::MissingProof)?;
        let source = p[k].clone();
        let scale = product.get(k).cloned().ok_or(Reason::MissingProof)?;
        if scale.sign() == Sign::Zero {
            return Err(Reason::MissingProof);
        }
        for j in 0..p.len().max(product.len()) {
            let z = Expansion::scalar(0.);
            if p.get(j)
                .unwrap_or(&z)
                .mul(&scale, ctx)?
                .sub(&product.get(j).unwrap_or(&z).mul(&source, ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Err(Reason::MissingProof);
            }
        }
        quotients.push(q);
        source_coefficients.push(source);
        product_coefficients.push(scale);
    }
    let mut reduced = Vec::new();
    for i in 0..3 {
        let mut scale = source_coefficients[i].clone();
        for j in 0..3 {
            if i != j {
                scale = scale.mul(&product_coefficients[j], ctx)?;
            }
        }
        reduced.push(scaled(&quotients[i], &scale, ctx)?);
    }
    for i in 0..2 {
        if !zero(&sub(
            &mul(&reduced[i], &a[2], ctx)?,
            &mul(&a[i], &reduced[2], ctx)?,
            ctx,
        )?) {
            return Err(Reason::MissingProof);
        }
    }
    if reduced[2][0].sign() == Sign::Zero {
        return Err(Reason::MissingProof);
    }
    if reduced[2][0].sign() == Sign::Negative {
        for p in &mut reduced {
            for c in p {
                *c = c.negated();
            }
        }
    }
    let lengths: Vec<_> = reduced.iter().map(Vec::len).collect();
    let mut all = reduced.into_iter().flatten().collect::<Vec<_>>();
    primitive_common(&mut all, ctx)?;
    Expansion::normalize_common(&mut all, ctx)?;
    let mut cursor = 0;
    Ok(std::array::from_fn(|i| {
        let p = all[cursor..cursor + lengths[i]].to_vec();
        cursor += lengths[i];
        p
    }))
}
/// Fraction-free echelon kernel; every candidate is checked against the
/// original matrix before it can define a moving line.
fn primitive_row(row: &mut Vec<Expansion>, ctx: &mut PredicateContext<'_>) -> Result<(), Reason> {
    let mut values: [Expansion; 6] = row.clone().try_into().map_err(|_| Reason::MissingProof)?;
    Expansion::reduce_projective(&mut values, ctx)?;
    *row = values.to_vec();
    Ok(())
}
fn kernel(
    matrix: &[Vec<Expansion>],
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<Vec<Expansion>>, Reason> {
    let mut rows = matrix.to_vec();
    let mut pivots = Vec::new();
    for col in 0..6 {
        let k = pivots.len();
        let Some(row) = (k..rows.len()).find(|&i| rows[i][col].sign() != Sign::Zero) else {
            continue;
        };
        rows.swap(k, row);
        for i in k + 1..rows.len() {
            if rows[i][col].sign() == Sign::Zero {
                continue;
            }
            let coefficient = rows[i][col].clone();
            let pivot = rows[k][col].clone();
            for j in col..6 {
                rows[i][j] = rows[i][j]
                    .mul(&pivot, ctx)?
                    .sub(&rows[k][j].mul(&coefficient, ctx)?, ctx)?;
            }
            Expansion::normalize_common(&mut rows[i], ctx)?;
            primitive_row(&mut rows[i], ctx)?;
        }
        pivots.push(col);
        if pivots.len() == rows.len() {
            break;
        }
    }
    let mut result = Vec::new();
    for free in (0..6).filter(|j| !pivots.contains(j)) {
        let mut v = vec![Expansion::scalar(0.); 6];
        v[free] = Expansion::scalar(1.);
        for (i, &col) in pivots.iter().enumerate().rev() {
            let mut sum = Expansion::scalar(0.);
            for j in col + 1..6 {
                sum = sum.add(&rows[i][j].mul(&v[j], ctx)?, ctx)?
            }
            let pivot = rows[i][col].clone();
            for value in &mut v {
                *value = value.mul(&pivot, ctx)?
            }
            v[col] = sum.negated();
            Expansion::normalize_common(&mut v, ctx)?;
            primitive_row(&mut v, ctx)?;
        }
        for row in matrix {
            let mut sum = Expansion::scalar(0.);
            for j in 0..6 {
                sum = sum.add(&row[j].mul(&v[j], ctx)?, ctx)?
            }
            if sum.sign() != Sign::Zero {
                return Err(Reason::MissingProof);
            }
        }
        result.push(v);
    }
    Ok(result)
}
fn dot(l: &[Expansion], h: &[Poly; 3], ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    let mut out = vec![Expansion::scalar(0.)];
    for k in 0..3 {
        let term = scaled(&h[k], &l[k], ctx)?;
        out = sub(&out, &term.iter().map(Expansion::negated).collect(), ctx)?;
    }
    Ok(out)
}
fn product_power(a: &Poly, n: usize, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    let mut out = vec![Expansion::scalar(1.)];
    for _ in 0..n {
        out = mul(&out, a, ctx)?
    }
    Ok(out)
}
fn form(
    coefficients: &Poly,
    degree: usize,
    x: &Poly,
    y: &Poly,
    ctx: &mut PredicateContext<'_>,
) -> Result<Poly, Reason> {
    let mut out = vec![Expansion::scalar(0.)];
    for (j, c) in coefficients.iter().enumerate() {
        if c.sign() == Sign::Zero {
            continue;
        }
        if j > degree {
            return Err(Reason::MissingProof);
        }
        let term = scaled(
            &mul(
                &product_power(x, j, ctx)?,
                &product_power(y, degree - j, ctx)?,
                ctx,
            )?,
            c,
            ctx,
        )?;
        out = sub(&out, &term.iter().map(Expansion::negated).collect(), ctx)?;
    }
    Ok(out)
}
fn refine(
    f: &[Poly],
    interval: &mut [f64; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<(), Reason> {
    let mid = interval[0] * 0.5 + interval[1] * 0.5;
    if mid <= interval[0] || mid >= interval[1] {
        return Err(Reason::PrecisionExhausted);
    }
    if at(&f[0], mid, ctx)? == Sign::Zero {
        *interval = [mid, mid]
    } else if count(f, interval[0], mid, ctx)? == 1 {
        interval[1] = mid
    } else {
        interval[0] = mid
    }
    Ok(())
}
/// Sign of G at the single root of F in this box. GCD proves exact zeros;
/// otherwise disjoint Sturm root boxes prove the sign. No epsilon-zero test.
fn root_sign(
    f: &Poly,
    chain: &[Poly],
    g: &Poly,
    interval: &mut [f64; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<Sign, Reason> {
    if zero(g) {
        return Ok(Sign::Zero);
    }
    if interval[0] == interval[1] {
        return at(g, interval[0], ctx);
    }
    let gc = square_free_chain(g, ctx)?.0;
    let lo = at(g, interval[0], ctx)?;
    if lo != Sign::Zero && count(&gc, interval[0], interval[1], ctx)? == 0 {
        return Ok(lo);
    }
    let common = gcd(f, g, ctx)?;
    if common.len() > 1
        && count(
            &square_free_chain(&common, ctx)?.0,
            interval[0],
            interval[1],
            ctx,
        )? == 1
    {
        return Ok(Sign::Zero);
    }
    for _ in 0..1074 {
        let lo = at(g, interval[0], ctx)?;
        if lo != Sign::Zero && count(&gc, interval[0], interval[1], ctx)? == 0 {
            return Ok(lo);
        }
        refine(chain, interval, ctx)?;
        if interval[0] == interval[1] {
            return at(g, interval[0], ctx);
        }
    }
    Err(Reason::PrecisionExhausted)
}
fn inverse_model(
    a: &[Poly; 3],
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<(Vec<Expansion>, usize, Poly, bool)>, Reason> {
    let degree = a.iter().map(|p| p.len() - 1).max().unwrap();
    if !(2..=3).contains(&degree) {
        return Ok(None);
    }
    if degree == 2 {
        if let Some(model) = quadratic_inverse(a, ctx)? {
            return Ok(Some(model));
        }
    }
    let z = Expansion::scalar(0.);
    let matrix = (0..degree + 2)
        .map(|i| {
            (0..6)
                .map(|j| {
                    let shift = j / 3;
                    if i < shift {
                        z.clone()
                    } else {
                        a[j % 3].get(i - shift).unwrap_or(&z).clone()
                    }
                })
                .collect()
        })
        .collect::<Vec<_>>();
    for moving in kernel(&matrix, ctx)? {
        let cross = (0..3)
            .map(|k| {
                let i = (k + 1) % 3;
                let j = (k + 2) % 3;
                moving[i]
                    .mul(&moving[3 + j], ctx)?
                    .sub(&moving[j].mul(&moving[3 + i], ctx)?, ctx)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let Some(axis) = cross.iter().position(|v| v.sign() != Sign::Zero) else {
            continue;
        };
        let g = dot(&moving[3..], a, ctx)?;
        let source_z = &a[axis];
        if g.len() > degree || zero(&g) || gcd(&g, source_z, ctx)?.len() != 1 {
            continue;
        }
        return Ok(Some((moving, axis, g, false)));
    }
    Ok(None)
}
// H(t)=C0+C1*t+C2*t². Adjugate rows L0=C1×C2 and L1=C2×C0
// satisfy L0(H)=det and L1(H)=det*t. A nonzero determinant proves
// the global inverse t=L1/L0 on the represented finite source interval.
// Every dual identity is checked with the original exact expansions.
fn quadratic_inverse(
    a: &[Poly; 3],
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<(Vec<Expansion>, usize, Poly, bool)>, Reason> {
    let columns = (0..3)
        .map(|i| {
            std::array::from_fn::<_, 3, _>(|k| {
                a[k].get(i)
                    .cloned()
                    .unwrap_or_else(|| Expansion::scalar(0.))
            })
        })
        .collect::<Vec<_>>();
    let cross = |x: &[Expansion; 3],
                 y: &[Expansion; 3],
                 ctx: &mut PredicateContext<'_>|
     -> Result<Vec<Expansion>, Reason> {
        (0..3)
            .map(|k| {
                let (i, j) = ((k + 1) % 3, (k + 2) % 3);
                x[i].mul(&y[j], ctx)?.sub(&x[j].mul(&y[i], ctx)?, ctx)
            })
            .collect()
    };
    let l0 = cross(&columns[1], &columns[2], ctx)?;
    let l1 = cross(&columns[2], &columns[0], ctx)?;
    let scalar = |row: &[Expansion],
                  column: &[Expansion; 3],
                  ctx: &mut PredicateContext<'_>|
     -> Result<Expansion, Reason> {
        let mut result = Expansion::scalar(0.);
        for k in 0..3 {
            result = result.add(&row[k].mul(&column[k], ctx)?, ctx)?;
        }
        Ok(result)
    };
    let determinant = scalar(&l0, &columns[0], ctx)?;
    if determinant.sign() == Sign::Zero {
        return Ok(None);
    }
    for (r, row) in [&l0, &l1].into_iter().enumerate() {
        for i in 0..3 {
            let expected = if r == i {
                determinant.clone()
            } else {
                Expansion::scalar(0.)
            };
            if scalar(row, &columns[i], ctx)?.sub(&expected, ctx)?.sign() != Sign::Zero {
                return Err(Reason::MissingProof);
            }
        }
    }
    let axis = columns[2]
        .iter()
        .position(|x| x.sign() != Sign::Zero)
        .ok_or(Reason::MissingProof)?;
    let moving = l1.iter().map(Expansion::negated).chain(l0).collect();
    Ok(Some((moving, axis, vec![determinant], true)))
}

fn solve(
    a: &[Poly; 3],
    b: &[Poly; 3],
    tolerance: f64,
    ctx: &mut PredicateContext<'_>,
    phase: &mut &'static str,
) -> Result<PlanarPairOutcome, Reason> {
    let degree = a.iter().map(|p| p.len() - 1).max().unwrap();
    *phase = "carrier-inverse";
    if let Some((moving, axis, g, quadratic)) = inverse_model(a, ctx)? {
        *phase = "eliminant-composition";
        let source_z = &a[axis];
        let x = dot(&moving[..3], b, ctx)?;
        let y = dot(&moving[3..], b, ctx)?;
        let nx = x.iter().map(Expansion::negated).collect();
        let mut f = if quadratic {
            // The proper quadratic's adjugate coordinates satisfy L0*L2=L1².
            // This removes the known nonzero source-axis scalar from the
            // generic eliminant before expensive polynomial multiplication.
            let c0: [Expansion; 3] = std::array::from_fn(|k| {
                a[k].first()
                    .cloned()
                    .unwrap_or_else(|| Expansion::scalar(0.))
            });
            let c1: [Expansion; 3] = std::array::from_fn(|k| {
                a[k].get(1)
                    .cloned()
                    .unwrap_or_else(|| Expansion::scalar(0.))
            });
            let l2 = (0..3)
                .map(|k| {
                    let (i, j) = ((k + 1) % 3, (k + 2) % 3);
                    c0[i].mul(&c1[j], ctx)?.sub(&c0[j].mul(&c1[i], ctx)?, ctx)
                })
                .collect::<Result<Vec<_>, _>>()?;
            sub(&mul(&y, &dot(&l2, b, ctx)?, ctx)?, &mul(&x, &x, ctx)?, ctx)?
        } else {
            sub(
                &mul(&b[axis], &form(&g, degree - 1, &nx, &y, ctx)?, ctx)?,
                &form(source_z, degree, &nx, &y, ctx)?,
                ctx,
            )?
        };
        normalize(&mut f, ctx)?;
        if zero(&f) {
            return Ok(PlanarPairOutcome::CommonCarrier);
        }
        *phase = "eliminant-root-census";
        let roots = isolate(&f, tolerance, ctx)?;
        *phase = "root-correspondence";
        let chain = if roots.iter().all(|r| r.interval[0] == r.interval[1]) {
            // Exact point roots use direct evaluation in root_sign; a Sturm
            // chain is never consulted for these roots.
            vec![f.clone()]
        } else {
            square_free_chain(&f, ctx)?.0
        };
        let mut out = Vec::new();
        for root in roots {
            let mut interval = root.interval;
            let sy = root_sign(&chain[0], &chain, &y, &mut interval, ctx)?;
            let sx = root_sign(&chain[0], &chain, &x, &mut interval, ctx)?;
            if sy == Sign::Zero {
                if sx == Sign::Zero {
                    for singular in isolate(&g, tolerance, ctx)? {
                        out.push(PlanarPairRoot {
                            first: singular.interval,
                            second: interval,
                            multiplicity: root.multiplicity,
                        });
                    }
                }
                continue;
            }
            let xy = sub(&x, &y.iter().map(Expansion::negated).collect(), ctx)?;
            let se = root_sign(&chain[0], &chain, &xy, &mut interval, ctx)?;
            // t=-X/Y: negative t or t>1 belongs only to the infinite carrier.
            if (sx != Sign::Zero && sx == sy) || (se != Sign::Zero && se != sy) {
                continue;
            }
            let mut first = if sx == Sign::Zero {
                [0., 0.]
            } else if se == Sign::Zero {
                [1., 1.]
            } else {
                [0., 1.]
            };
            for _ in 0..1074 {
                if first[1] - first[0] <= tolerance {
                    break;
                }
                let mid = first[0] * 0.5 + first[1] * 0.5;
                if mid <= first[0] || mid >= first[1] {
                    return Err(Reason::PrecisionExhausted);
                }
                let test = sub(&x, &scaled(&y, &Expansion::scalar(-mid), ctx)?, ctx)?;
                let sign = root_sign(&chain[0], &chain, &test, &mut interval, ctx)?;
                if sign == Sign::Zero {
                    first = [mid, mid];
                    break;
                }
                if sign == sy {
                    first[1] = mid
                } else {
                    first[0] = mid
                }
            }
            if first[1] - first[0] > tolerance {
                return Err(Reason::PrecisionExhausted);
            }
            out.push(PlanarPairRoot {
                first,
                second: interval,
                multiplicity: root.multiplicity,
            });
        }
        *phase = "complete";
        return Ok(PlanarPairOutcome::Roots(out));
    }
    Ok(PlanarPairOutcome::NotReducible)
}

/// Complete paired root census for a proper rational quadratic/cubic source
/// against an original rational Bezier query of degree 1..8. Only the first
/// curve supplies the moving-line inverse; no low-degree query is assumed.
/// Resource/conditioning failures are explicit; they never mean no contact.
pub fn planar_bezier_pair_roots(
    ctx: &mut PredicateContext<'_>,
    a: &[[LeafRef; 3]],
    b: &[[LeafRef; 3]],
    parameter_tolerance: f64,
) -> Result<PlanarPairDecision, InputError> {
    if !(2..=4).contains(&a.len())
        || !(2..=9).contains(&b.len())
        || !parameter_tolerance.is_finite()
        || parameter_tolerance <= 0.
        || parameter_tolerance >= 1.
    {
        return Err(InputError::InvalidInput("Invalid planar paired root input"));
    }
    let source = a
        .iter()
        .chain(b)
        .flatten()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let mut phase = "source-admission";
    let computed = (|| {
        let exact = exact_inputs(&source, ctx)?;
        let points = exact
            .chunks_exact(3)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect::<Vec<[Expansion; 3]>>();
        if points.iter().any(|p| p[2].sign() != Sign::Positive) {
            return Err(Reason::MissingProof);
        }
        let (aa, bb) = points.split_at(a.len());
        phase = "source-carrier-reduction";
        let first = primitive_carrier(homogeneous(aa, ctx)?, ctx)?;
        phase = "query-carrier-reduction";
        let second = primitive_carrier(homogeneous(bb, ctx)?, ctx)?;
        solve(
            &first,
            &second,
            parameter_tolerance,
            ctx,
            &mut phase,
        )
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(PlanarPairDecision {
        outcome: computed.unwrap_or_else(PlanarPairOutcome::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
        phase,
    })
}

/// Every distinct preimage pair on a proper planar cubic must belong to the
/// exceptional fiber X=Y=0 of its verified linear inverse. That fiber has at
/// most two parameters. A single (possibly double) preimage is a cusp, not a
/// second branch, and stationary splitting handles it separately.
pub fn planar_bezier_self_roots(
    ctx: &mut PredicateContext<'_>,
    a: &[[LeafRef; 3]],
    parameter_tolerance: f64,
) -> Result<PlanarPairDecision, InputError> {
    if !(2..=4).contains(&a.len())
        || !parameter_tolerance.is_finite()
        || parameter_tolerance <= 0.
        || parameter_tolerance >= 1.
    {
        return Err(InputError::InvalidInput("Invalid planar self root input"));
    }
    let source = a
        .iter()
        .flatten()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let mut phase = "source-admission";
    let computed = (|| {
        let exact = exact_inputs(&source, ctx)?;
        let points = exact
            .chunks_exact(3)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect::<Vec<[Expansion; 3]>>();
        if points.iter().any(|p| p[2].sign() != Sign::Positive) {
            return Err(Reason::MissingProof);
        }
        phase = "source-carrier-reduction";
        let h = primitive_carrier(homogeneous(&points, ctx)?, ctx)?;
        phase = "carrier-inverse";
        let Some((_, _, g, _)) = inverse_model(&h, ctx)? else {
            return Ok(PlanarPairOutcome::NotReducible);
        };
        phase = "self-root-census";
        let roots = isolate(&g, parameter_tolerance, ctx)?;
        phase = "root-correspondence";
        let mut pairs = Vec::new();
        for i in 0..roots.len() {
            for j in i + 1..roots.len() {
                pairs.push(PlanarPairRoot {
                    first: roots[i].interval,
                    second: roots[j].interval,
                    multiplicity: 2,
                });
            }
        }
        phase = "complete";
        Ok(PlanarPairOutcome::Roots(pairs))
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(PlanarPairDecision {
        outcome: computed.unwrap_or_else(PlanarPairOutcome::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
        phase,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn pair(a: &[[f64; 3]], b: &[[f64; 3]], limits: Limits) -> PlanarPairOutcome {
        let source = SourceArena::authored(
            "implicit-pair-regression",
            1,
            a.iter()
                .chain(b)
                .flatten()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let refs = |start: usize, n: usize| {
            (start..start + n)
                .map(|i| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        planar_bezier_pair_roots(&mut ctx, &refs(0, a.len()), &refs(a.len(), b.len()), 1e-12)
            .unwrap()
            .outcome
    }
    #[test]
    fn tiny_finite_cap_crossing_is_not_admitted_and_bounded_inward_proposal_is_separated() {
        // Exact binary64 regression of the original source cap. Independent
        // rational arithmetic gives opposite conic signs at 1-2^-16 and
        // 1-2^-20, with the entire inverse range strictly inside (0,1).
        let a: [[f64;3];3]=[[0.26853768389484367,0.16625579515959196,0.8762538394705197],
            [0.18573459718751711,0.,0.7958957478207549],[0.,0.,1.]];
        let original: [[f64;2];6]=[[0.371469194535166,0.],[0.3030302000309807,0.],[0.2345912056748032,0.],
            [0.19004607787900915,0.052433411889112505],[0.23802662093647095,0.10499429840894568],
            [0.26853768389484367,0.16625579515959196]];
        for repaired in [false,true] {
            let mut points=original;
            if repaired {
                points[1]=[0.30303020013988224,1.5625e-10];
                points[2]=[0.23459120579439832,1.5625e-10];
                points[3]=[0.19004607800556444,0.05243341203716979];
                points[4]=[0.2380266210555293,0.10499429854879032];
            }
            let b=points.map(|p|[p[0],p[1],0.9999999999999999]);
            let source=SourceArena::authored("finite-cap-crossing-regression",1,a.iter().chain(&b).flatten().map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect()).unwrap();
            let refs=|start:usize,n:usize|(start..start+n).map(|i|std::array::from_fn(|k|source.leaf(i*3+k).unwrap())).collect::<Vec<_>>();
            let tolerance=ToleranceContext::default_valid();
            let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work:1000000,..Limits::default()},None);
            let proof=planar_bezier_endpoint_separation(&mut ctx,&refs(0,3),&refs(3,6)).unwrap();
            assert_eq!(proof.proven,repaired,"{:?}",proof.reason);
            assert!(proof.work_used<=1000000);
        }
    }
    #[test]
    fn direct_quadratic_bernstein_separation_keeps_exact_ends_and_refuses_crossings() {
        let a=[[0.,0.,1.],[2.5,0.,1.],[5.,25.,1.]];
        let b=[[0.,0.,1.],[1.,0.,1.],[2.,0.,1.],[3.,0.,1.],[4.,0.,1.],[5.,25.,1.]];
        let prove=|a:&[[f64;3]],b:&[[f64;3]],work:u64| {
            let source=SourceArena::authored("direct-conic-separation-test",1,a.iter().chain(b).flatten().map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect()).unwrap();
            let refs=|start:usize,n:usize|(start..start+n).map(|i|std::array::from_fn(|k|source.leaf(i*3+k).unwrap())).collect::<Vec<_>>();
            let t=ToleranceContext::default_valid();let mut ctx=PredicateContext::new(&source,&t,Limits{max_work:work,..Limits::default()},None);
            planar_bezier_endpoint_separation(&mut ctx,&refs(0,a.len()),&refs(a.len(),b.len())).unwrap()
        };
        for transform in [false,true] {
            let frame=|p:[f64;3]|if transform {[17.+p[0]-p[1],-9.+p[0]+p[1],p[2]]}else{p};
            let aa=a.map(frame);let bb=b.map(frame);
            let good=prove(&aa,&bb,1000000);assert!(good.proven);assert!(good.work_used<=1000000);
            let mut crossing=b;crossing[1][1]=2.;
            assert!(!prove(&aa,&crossing.map(frame),1000000).proven);
        }
        assert!(!prove(&a,&a,1000000).proven);
        let empty=prove(&a,&b,1);assert!(!empty.proven&&empty.work_used<=1);
        assert_eq!(empty.reason,Some(Reason::ResourceLimit));
    }
    #[test]
    fn quadratic_carrier_accepts_quintic_query_without_losing_interior_events() {
        let a = [[0.,0.,1.], [2.5,0.,1.], [5.,25.,1.]];
        let b = [[0.,0.,1.],[1.,0.,1.],[2.,0.,1.],[3.,0.,1.],[4.,0.,1.],[5.,25.,1.]];
        let PlanarPairOutcome::Roots(roots) = pair(&a,&b,Limits::default()) else { panic!("Complete quintic census required"); };
        assert_eq!(roots.len(),2);
        assert!(roots.iter().any(|r|r.first==[0.,0.]&&r.second==[0.,0.]&&r.multiplicity==2));
        assert!(roots.iter().any(|r|r.first==[1.,1.]&&r.second==[1.,1.]));
        let mut crossing=b; crossing[1][1]=2.;
        let PlanarPairOutcome::Roots(roots) = pair(&a,&crossing,Limits::default()) else {panic!("Interior quintic census required");};
        assert!(roots.iter().any(|r|r.first[0]>0.&&r.first[1]<1.&&r.second[0]>0.&&r.second[1]<1.));
        assert!(matches!(pair(&a,&b,Limits{max_work:1,..Limits::default()}),PlanarPairOutcome::Indeterminate(_)));
    }
    #[test]
    fn nonlinear_parameter_maps_retain_third_order_contact_and_affine_covariance() {
        // A(t)=(3t,3t²), B(s)=(3s+3s²,3s²+3s³).
        // The implicit substitution is a nonzero multiple of s³(1+s).
        let a = [[0., 0., 1.], [1.5, 0., 1.], [3., 3., 1.]];
        let b = [[0., 0., 1.], [1., 0., 1.], [3., 1., 1.], [6., 6., 1.]];
        for transformed in [false, true] {
            let frame = |p: [f64; 3]| {
                if transformed {
                    [17. + p[0] - p[1], -9. + p[0] + p[1], p[2]]
                } else {
                    p
                }
            };
            let PlanarPairOutcome::Roots(roots) =
                pair(&a.map(frame), &b.map(frame), Limits::default())
            else {
                panic!("missing implicit census")
            };
            assert_eq!(roots.len(), 1);
            assert_eq!(roots[0].first, [0., 0.]);
            assert_eq!(roots[0].second, [0., 0.]);
            assert_eq!(roots[0].multiplicity, 3);
        }
    }
    #[test]
    fn inverse_filters_contacts_outside_the_finite_carrier_and_preserves_pair_parameters() {
        let a = [[0., 0., 1.], [1.5, 0., 1.], [3., 3., 1.]];
        // B(s)=(3s,3/4): A meets B at t=s=1/2.
        let b = [[0., 0.75, 1.], [3., 0.75, 1.]];
        let PlanarPairOutcome::Roots(roots) = pair(&a, &b, Limits::default()) else {
            panic!("missing roots")
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].first, [0.5, 0.5]);
        assert_eq!(roots[0].second, [0.5, 0.5]);
        // Same infinite parabola, but contact x=6 is outside A.
        let b = [[6., 3., 1.], [6., 15., 1.]];
        assert!(
            matches!(pair(&a,&b,Limits::default()),PlanarPairOutcome::Roots(r) if r.is_empty())
        );
    }
    #[test]
    fn singular_carrier_returns_both_node_preimages_and_only_the_true_cusp_preimage() {
        // A(t)=(3(2t-1)²,3(2t-1)³): a cusp at t=1/2.
        let a = [[3., -3., 1.], [-1., 3., 1.], [-1., -3., 1.], [3., 3., 1.]];
        let b = [[-1., 0., 1.], [1., 0., 1.]];
        let PlanarPairOutcome::Roots(r) = pair(&a, &b, Limits::default()) else {
            panic!("missing cusp")
        };
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].first, [0.5, 0.5]);
        assert_eq!(r[0].second, [0.5, 0.5]);
        // Proper nodal cubic: x=3u²-3, y=3u³-3u, u=4t-2.
        let a = [
            [9., -18., 1.],
            [-7., 26., 1.],
            [-7., -26., 1.],
            [9., 18., 1.],
        ];
        let b = [[-1., 0., 1.], [1., 0., 1.]];
        let outcome = pair(&a, &b, Limits::default());
        let PlanarPairOutcome::Roots(r) = outcome else {
            panic!("missing node: {outcome:?}")
        };
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].first, [0.25, 0.25]);
        assert_eq!(r[1].first, [0.75, 0.75]);
        assert!(r.iter().all(|r| r.second == [0.5, 0.5]));
    }
    #[test]
    fn common_carrier_and_resource_exhaustion_do_not_claim_empty_intersections() {
        let a = [[0., 0., 1.], [1.5, 0., 1.], [3., 3., 1.]];
        assert!(matches!(
            pair(&a, &a, Limits::default()),
            PlanarPairOutcome::CommonCarrier
        ));
        let mut limits = Limits::default();
        limits.max_work = 0;
        assert!(matches!(
            pair(&a, &a, limits),
            PlanarPairOutcome::Indeterminate(Reason::ResourceLimit)
        ));
    }
    #[test]
    fn quartic_eliminant_keeps_distinct_pair_parameters_and_a_double_contact() {
        // A(u)=(192u,384u²), B(s)=(96s+96s²,Y(s)).
        // Y-X²/96=-96(s-1/4)²(s-3/4)(s+1).
        let a = [[0., 0., 1.], [96., 0., 1.], [192., 384., 1.]];
        let b = [
            [0., 4.5, 1.],
            [32., -8., 1.],
            [96., 37.5, 1.],
            [192., 357., 1.],
        ];
        let outcome = pair(&a, &b, Limits::default());
        let PlanarPairOutcome::Roots(roots) = outcome else {
            panic!("quartic census: {outcome:?}")
        };
        assert_eq!(roots.len(), 2);
        for (r, (u, s, m)) in roots.iter().zip([(0.15625, 0.25, 2), (0.65625, 0.75, 1)]) {
            assert_eq!(r.first, [u, u]);
            assert_eq!(r.second, [s, s]);
            assert_eq!(r.multiplicity, m);
        }
    }
    #[test]
    fn rational_conic_keeps_original_weights_and_irrational_observation_box() {
        let a = [[3., 0., 1.], [3., 3., 0.5], [0., 3., 1.]];
        let b = [[1., 1., 1.], [4., 4., 1.]];
        let outcome = pair(&a, &b, Limits::default());
        let PlanarPairOutcome::Roots(roots) = outcome else {
            panic!("rational census: {outcome:?}")
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].first, [0.5, 0.5]);
        assert!(roots[0].second[0] <= 1. / 3. && roots[0].second[1] >= 1. / 3.);
        assert!(roots[0].second[1] - roots[0].second[0] <= 1e-12);
    }
    #[test]
    fn exceptional_inverse_fiber_censuses_self_nodes_but_does_not_double_count_a_cusp() {
        for (a, expected) in [
            (
                [
                    [9., -18., 1.],
                    [-7., 26., 1.],
                    [-7., -26., 1.],
                    [9., 18., 1.],
                ],
                1,
            ),
            (
                [[3., -3., 1.], [-1., 3., 1.], [-1., -3., 1.], [3., 3., 1.]],
                0,
            ),
        ] {
            let source = SourceArena::authored(
                "self-pair-regression",
                1,
                a.iter()
                    .flatten()
                    .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                    .collect(),
            )
            .unwrap();
            let refs = (0..a.len())
                .map(|i| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap()))
                .collect::<Vec<_>>();
            let tolerance = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
            let outcome = planar_bezier_self_roots(&mut ctx, &refs, 1e-12)
                .unwrap()
                .outcome;
            let PlanarPairOutcome::Roots(roots) = outcome else {
                panic!("self census: {outcome:?}")
            };
            assert_eq!(roots.len(), expected);
            if expected > 0 {
                assert_eq!(roots[0].first, [0.25, 0.25]);
                assert_eq!(roots[0].second, [0.75, 0.75]);
            }
        }
    }
}
