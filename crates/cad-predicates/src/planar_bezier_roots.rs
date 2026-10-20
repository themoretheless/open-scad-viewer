//! Exact-expansion scalar reductions of planar rational Bézier intersections.
//! Fraction-free Sturm chains isolate every distinct scalar root, including
//! multiple roots. Root positions are dyadic enclosures, not rounded sources.
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Expansion, InputError, LeafRef, PredicateContext,
    Reason, Sign, exact_inputs,
};
#[path = "planar_bezier_implicit.rs"]
mod implicit;
pub use implicit::*;
type Poly = Vec<Expansion>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanarReduction {
    CommonCoordinate(usize),
    CommonProjection,
    LineCarrier { first: bool },
}
#[derive(Clone, Debug)]
pub struct PlanarScalarRoot {
    pub interval: [f64; 2],
    pub multiplicity: usize,
}
#[derive(Clone, Debug)]
pub enum PlanarScalarOutcome {
    Roots {
        reduction: PlanarReduction,
        roots: Vec<PlanarScalarRoot>,
    },
    Coincident {
        reduction: PlanarReduction,
    },
    NotReducible,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct PlanarScalarDecision {
    pub outcome: PlanarScalarOutcome,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// Exact positive content reduction keeps repeated-root PRS coefficients
/// small. The bounded integer fast path is optional: wider expansions retain
/// their exact coefficients and the normal work/precision guards still apply.
fn primitive_common(
    values: &mut [Expansion],
    ctx: &mut PredicateContext<'_>,
) -> Result<(), Reason> {
    if values.len() > 24 {
        return Ok(());
    }
    Expansion::reduce_projective(values, ctx)
}
fn trim(p: &mut Poly) {
    while p.len() > 1 && p.last().unwrap().sign() == Sign::Zero {
        p.pop();
    }
}
fn zero(p: &Poly) -> bool {
    p.iter().all(|c| c.sign() == Sign::Zero)
}
fn normalize(p: &mut Poly, ctx: &mut PredicateContext<'_>) -> Result<(), Reason> {
    trim(p);
    primitive_common(p, ctx)?;
    Expansion::normalize_common(p, ctx)
}
#[inline(always)]
fn choose(n: usize, k: usize) -> u64 {
    crate::binomial_u64(n, k)
}
pub(crate) fn power(b: &[Expansion], ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    let n = b.len() - 1;
    let mut out = vec![Expansion::scalar(0.); n + 1];
    for (i, value) in b.iter().enumerate() {
        let c_n_i = choose(n, i);
        for k in i..=n {
            let factor = Expansion::integer(c_n_i * choose(n - i, k - i), ctx)?;
            let term = value.mul(&factor, ctx)?;
            out[k] = if (k - i) % 2 == 0 {
                out[k].add(&term, ctx)?
            } else {
                out[k].sub(&term, ctx)?
            };
        }
    }
    trim(&mut out);
    Ok(out)
}
fn homogeneous(p: &[[Expansion; 3]], ctx: &mut PredicateContext<'_>) -> Result<[Poly; 3], Reason> {
    let mut constant_weight = p[0][2].sign() != Sign::Zero;
    for row in &p[1..] {
        if row[2].sub(&p[0][2], ctx)?.sign() != Sign::Zero {
            constant_weight = false;
            break;
        }
    }
    if constant_weight {
        // Cancel only an exactly common nonzero authored weight. The rational
        // map is polynomial; retaining that scalar through every resultant
        // product needlessly enlarges exact coefficients.
        let x = p.iter().map(|v| v[0].clone()).collect::<Vec<_>>();
        let y = p.iter().map(|v| v[1].clone()).collect::<Vec<_>>();
        return Ok([
            power(&x, ctx)?,
            power(&y, ctx)?,
            vec![Expansion::scalar(1.)],
        ]);
    }
    let x = p
        .iter()
        .map(|p| p[0].mul(&p[2], ctx))
        .collect::<Result<Vec<_>, _>>()?;
    let y = p
        .iter()
        .map(|p| p[1].mul(&p[2], ctx))
        .collect::<Result<Vec<_>, _>>()?;
    let w = p.iter().map(|p| p[2].clone()).collect::<Vec<_>>();
    Ok([power(&x, ctx)?, power(&y, ctx)?, power(&w, ctx)?])
}

fn mul(a: &Poly, b: &Poly, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    let mut out = vec![Expansion::scalar(0.); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] = out[i + j].add(&x.mul(y, ctx)?, ctx)?;
        }
    }
    trim(&mut out);
    Ok(out)
}
fn sub(a: &Poly, b: &Poly, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    let mut out = vec![Expansion::scalar(0.); a.len().max(b.len())];
    for (i, c) in out.iter_mut().enumerate() {
        if let Some(v) = a.get(i) {
            *c = c.add(v, ctx)?;
        }
        if let Some(v) = b.get(i) {
            *c = c.sub(v, ctx)?;
        }
    }
    trim(&mut out);
    Ok(out)
}
fn scaled(a: &Poly, b: &Expansion, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    a.iter().map(|a| a.mul(b, ctx)).collect()
}
fn derivative(p: &Poly, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    if p.len() == 1 {
        return Ok(vec![Expansion::scalar(0.)]);
    }
    (1..p.len())
        .map(|i| p[i].mul(&Expansion::integer(i as u64, ctx)?, ctx))
        .collect()
}
fn at(p: &Poly, t: f64, ctx: &mut PredicateContext<'_>) -> Result<Sign, Reason> {
    let t = Expansion::scalar(t);
    let mut result = Expansion::scalar(0.);
    for c in p.iter().rev() {
        result = result.mul(&t, ctx)?.add(c, ctx)?;
    }
    Ok(result.sign())
}
/// Q and R satisfy M*A=Q*B+R for a nonzero scalar M. Every
/// renormalization applies one common positive power of two to both Q and R.
fn pseudo_divide(
    a: &Poly,
    b: &Poly,
    ctx: &mut PredicateContext<'_>,
) -> Result<(Poly, Poly, bool), Reason> {
    if zero(b) {
        return Err(Reason::MissingProof);
    }
    let mut r = a.clone();
    trim(&mut r);
    let mut q = vec![Expansion::scalar(0.); a.len().saturating_sub(b.len()) + 1];
    let leading = b.last().unwrap();
    let mut negative = false;
    while !zero(&r) && r.len() >= b.len() {
        ctx.charge(1)?;
        let shift = r.len() - b.len();
        let coefficient = r.last().unwrap().clone();
        q = scaled(&q, leading, ctx)?;
        q[shift] = q[shift].add(&coefficient, ctx)?;
        r = scaled(&r, leading, ctx)?;
        for (j, value) in b.iter().enumerate() {
            r[shift + j] = r[shift + j].sub(&coefficient.mul(value, ctx)?, ctx)?;
        }
        trim(&mut r);
        if leading.sign() == Sign::Negative {
            negative = !negative;
        }
        let split = q.len();
        let mut both = q;
        both.extend(r);
        primitive_common(&mut both, ctx)?;
        Expansion::normalize_common(&mut both, ctx)?;
        r = both.split_off(split);
        q = both;
    }
    trim(&mut q);
    Ok((q, r, negative))
}
fn sturm(p: &Poly, ctx: &mut PredicateContext<'_>) -> Result<Vec<Poly>, Reason> {
    let mut p = p.clone();
    normalize(&mut p, ctx)?;
    let p_len = p.len();
    let mut d = derivative(&p, ctx)?;
    normalize(&mut d, ctx)?;
    let mut chain = vec![p];
    if zero(&d) {
        return Ok(chain);
    }
    chain.push(d);
    while chain.last().unwrap().len() > 1 {
        let n = chain.len();
        let (_, mut next, negative) = pseudo_divide(&chain[n - 2], &chain[n - 1], ctx)?;
        if zero(&next) {
            break;
        }
        if !negative {
            next = next.iter().map(Expansion::negated).collect();
        }
        normalize(&mut next, ctx)?;
        chain.push(next);
        if chain.len() > p_len + 1 {
            return Err(Reason::MissingProof);
        }
    }
    Ok(chain)
}
fn square_free_chain(
    p: &Poly,
    ctx: &mut PredicateContext<'_>,
) -> Result<(Vec<Poly>, Poly), Reason> {
    let original = sturm(p, ctx)?;
    let gcd = original.last().unwrap().clone();
    if gcd.len() == 1 {
        return Ok((original, gcd));
    }
    let (quotient, remainder, _) = pseudo_divide(p, &gcd, ctx)?;
    if !zero(&remainder) {
        return Err(Reason::MissingProof);
    }
    Ok((sturm(&quotient, ctx)?, gcd))
}
fn variations(chain: &[Poly], t: f64, ctx: &mut PredicateContext<'_>) -> Result<usize, Reason> {
    let mut previous = Sign::Zero;
    let mut count = 0;
    for p in chain {
        let sign = at(p, t, ctx)?;
        if sign == Sign::Zero {
            continue;
        }
        if previous != Sign::Zero && previous != sign {
            count += 1;
        }
        previous = sign;
    }
    Ok(count)
}
fn count(
    chain: &[Poly],
    lo: f64,
    hi: f64,
    ctx: &mut PredicateContext<'_>,
) -> Result<usize, Reason> {
    let a = variations(chain, lo, ctx)?;
    let b = variations(chain, hi, ctx)?;
    a.checked_sub(b).ok_or(Reason::MissingProof)
}
pub(crate) fn isolate(
    p: &Poly,
    tolerance: f64,
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<PlanarScalarRoot>, Reason> {
    if let Some(roots) = bernstein_endpoint_roots(p, ctx)? {
        return Ok(roots);
    }
    let (chain, mut gcd) = square_free_chain(p, ctx)?;
    let mut gcd_chains = Vec::new();
    while gcd.len() > 1 {
        let (sf, next) = square_free_chain(&gcd, ctx)?;
        gcd_chains.push(sf);
        if next.len() >= gcd.len() {
            return Err(Reason::MissingProof);
        }
        gcd = next;
    }
    let mut boxes = Vec::new();
    if at(&chain[0], 0., ctx)? == Sign::Zero {
        boxes.push([0., 0.]);
    }
    let initial = count(&chain, 0., 1., ctx)?;
    let mut pending = vec![(0., 1., initial, 0usize)];
    while let Some((lo, hi, number, depth)) = pending.pop() {
        ctx.charge(1)?;
        if number == 0 {
            continue;
        }
        if number == 1 && at(&chain[0], hi, ctx)? == Sign::Zero {
            boxes.push([hi, hi]);
            continue;
        }
        if number == 1 && hi - lo <= tolerance && at(&chain[0], lo, ctx)? != Sign::Zero {
            boxes.push([lo, hi]);
            continue;
        }
        if depth >= 1074 {
            return Err(Reason::PrecisionExhausted);
        }
        let mid = lo * 0.5 + hi * 0.5;
        if mid <= lo || mid >= hi {
            return Err(Reason::PrecisionExhausted);
        }
        let left = count(&chain, lo, mid, ctx)?;
        if left > number {
            return Err(Reason::MissingProof);
        }
        pending.push((mid, hi, number - left, depth + 1));
        pending.push((lo, mid, left, depth + 1));
    }
    boxes.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut roots = Vec::new();
    for interval in boxes {
        let mut multiplicity = 1;
        for sf in &gcd_chains {
            let owns = if interval[0] == interval[1] {
                at(&sf[0], interval[0], ctx)? == Sign::Zero
            } else {
                count(sf, interval[0], interval[1], ctx)? == 1
            };
            if !owns {
                break;
            }
            multiplicity += 1;
        }
        roots.push(PlanarScalarRoot {
            interval,
            multiplicity,
        });
    }
    Ok(roots)
}
// C(n,k)*b_k = sum_{i<=k} a_i*C(n-i,k-i). All factors are
// exact integers: no division or rounded Bernstein proxy is introduced.
// One-sided nonzero coefficients exclude every root in the open unit interval.
fn bernstein_endpoint_roots(
    p: &Poly,
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<Vec<PlanarScalarRoot>>, Reason> {
    let n = p.len() - 1;
    if n == 0 || n > 24 {
        return Ok(None);
    }
    let mut sign = Sign::Zero;
    for k in 0..=n {
        let mut value = Expansion::scalar(0.);
        for i in 0..=k {
            value = value.add(
                &p[i].mul(&Expansion::integer(choose(n - i, k - i), ctx)?, ctx)?,
                ctx,
            )?;
        }
        let next = value.sign();
        if next == Sign::Zero {
            continue;
        }
        if sign != Sign::Zero && sign != next {
            return Ok(None);
        }
        sign = next;
    }
    if sign == Sign::Zero {
        return Ok(None);
    }
    let mut roots = Vec::new();
    let first = p
        .iter()
        .position(|x| x.sign() != Sign::Zero)
        .ok_or(Reason::MissingProof)?;
    if first > 0 {
        roots.push(PlanarScalarRoot {
            interval: [0., 0.],
            multiplicity: first,
        });
    }
    for k in 0..=n {
        let mut value = Expansion::scalar(0.);
        for i in k..=n {
            value = value.add(
                &p[i].mul(&Expansion::integer(choose(i, k), ctx)?, ctx)?,
                ctx,
            )?;
        }
        if value.sign() != Sign::Zero {
            if k > 0 {
                roots.push(PlanarScalarRoot {
                    interval: [1., 1.],
                    multiplicity: k,
                });
            }
            return Ok(Some(roots));
        }
    }
    Err(Reason::MissingProof)
}

fn monotone(
    points: &[[Expansion; 3]],
    axis: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    let sign = points.last().unwrap()[axis]
        .sub(&points[0][axis], ctx)?
        .sign();
    if sign == Sign::Zero {
        return Ok(false);
    }
    let mut ordered = true;
    for p in points.windows(2) {
        let next = p[1][axis].sub(&p[0][axis], ctx)?.sign();
        if next != Sign::Zero && next != sign {
            ordered = false;
            break;
        }
    }
    if ordered {
        return Ok(true);
    }
    let h = homogeneous(points, ctx)?;
    monotone_quotient(&h[axis], &h[2], ctx)
}
fn monotone_quotient(
    numerator: &Poly,
    denominator: &Poly,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    let equation = sub(
        &mul(&derivative(numerator, ctx)?, denominator, ctx)?,
        &mul(numerator, &derivative(denominator, ctx)?, ctx)?,
        ctx,
    )?;
    if zero(&equation) {
        return Ok(false);
    }
    Ok(isolate(&equation, 1e-12, ctx)?.iter().all(|r| {
        r.multiplicity % 2 == 0
            || (r.interval[0] == r.interval[1] && (r.interval[0] == 0. || r.interval[0] == 1.))
    }))
}
/// A shared injective linear functional establishes t=s at every contact.
/// Its direction is constructed in exact arithmetic from original difference
/// coefficients, never from rounded rotated controls or a sampled axis.
fn projected_equation(
    a: &[Poly; 3],
    b: &[Poly; 3],
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<Poly>, Reason> {
    let dx = sub(&mul(&a[0], &b[2], ctx)?, &mul(&b[0], &a[2], ctx)?, ctx)?;
    let dy = sub(&mul(&a[1], &b[2], ctx)?, &mul(&b[1], &a[2], ctx)?, ctx)?;
    let n = dx.len().max(dy.len());
    let z = Expansion::scalar(0.);
    let Some(k) = (0..n).find(|&i| {
        dx.get(i).unwrap_or(&z).sign() != Sign::Zero || dy.get(i).unwrap_or(&z).sign() != Sign::Zero
    }) else {
        return Ok(None);
    };
    let x = dx.get(k).unwrap_or(&z);
    let y = dy.get(k).unwrap_or(&z);
    for i in 0..n {
        if dx
            .get(i)
            .unwrap_or(&z)
            .mul(y, ctx)?
            .sub(&dy.get(i).unwrap_or(&z).mul(x, ctx)?, ctx)?
            .sign()
            != Sign::Zero
        {
            return Ok(None);
        }
    }
    let projected = sub(&scaled(&a[0], y, ctx)?, &scaled(&a[1], x, ctx)?, ctx)?;
    if !monotone_quotient(&projected, &a[2], ctx)? {
        return Ok(None);
    }
    Ok(Some(if !zero(&dx) { dx } else { dy }))
}
fn straight(points: &[[Expansion; 3]], ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    let origin = &points[0];
    let last = points.last().unwrap();
    let dx = last[0].sub(&origin[0], ctx)?;
    let dy = last[1].sub(&origin[1], ctx)?;
    if dx.sign() == Sign::Zero && dy.sign() == Sign::Zero {
        return Ok(false);
    }
    for p in points {
        let x = p[0].sub(&origin[0], ctx)?;
        let y = p[1].sub(&origin[1], ctx)?;
        if x.mul(&dy, ctx)?.sub(&y.mul(&dx, ctx)?, ctx)?.sign() != Sign::Zero {
            return Ok(false);
        }
    }
    Ok(monotone(points, 0, ctx)? || monotone(points, 1, ctx)?)
}
/// Source rows are Euclidean XY plus positive weight, degree 1..8. Domains
/// are normalized only by the caller's parameter mapping; no trimmed/fitted
/// control net is introduced into the predicate's source arena.
pub fn planar_bezier_scalar_roots(
    ctx: &mut PredicateContext<'_>,
    a: &[[LeafRef; 3]],
    b: &[[LeafRef; 3]],
    parameter_tolerance: f64,
) -> Result<PlanarScalarDecision, InputError> {
    if !(2..=9).contains(&a.len())
        || !(2..=9).contains(&b.len())
        || !parameter_tolerance.is_finite()
        || parameter_tolerance <= 0.
        || parameter_tolerance >= 1.
    {
        return Err(InputError::InvalidInput("Invalid planar scalar root input"));
    }
    let source = a
        .iter()
        .chain(b)
        .flatten()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<PlanarScalarOutcome, Reason> {
        let exact = exact_inputs(&source, ctx)?;
        let points: Vec<[Expansion; 3]> = exact
            .chunks_exact(3)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect();
        if points.iter().any(|p| p[2].sign() != Sign::Positive) {
            return Err(Reason::MissingProof);
        }
        let (aa, bb) = points.split_at(a.len());
        let ha = homogeneous(aa, ctx)?;
        let hb = homogeneous(bb, ctx)?;
        for axis in 0..2 {
            let common = sub(
                &mul(&ha[axis], &hb[2], ctx)?,
                &mul(&hb[axis], &ha[2], ctx)?,
                ctx,
            )?;
            if !zero(&common) {
                continue;
            }
            if !monotone(aa, axis, ctx)? || !monotone(bb, axis, ctx)? {
                continue;
            }
            let free = 1 - axis;
            let equation = sub(
                &mul(&ha[free], &hb[2], ctx)?,
                &mul(&hb[free], &ha[2], ctx)?,
                ctx,
            )?;
            let reduction = PlanarReduction::CommonCoordinate(axis);
            if zero(&equation) {
                return Ok(PlanarScalarOutcome::Coincident { reduction });
            }
            return Ok(PlanarScalarOutcome::Roots {
                reduction,
                roots: isolate(&equation, parameter_tolerance, ctx)?,
            });
        }
        if let Some(equation) = projected_equation(&ha, &hb, ctx)? {
            return Ok(PlanarScalarOutcome::Roots {
                reduction: PlanarReduction::CommonProjection,
                roots: isolate(&equation, parameter_tolerance, ctx)?,
            });
        }
        for first in [true, false] {
            let (line, curve) = if first { (aa, &hb) } else { (bb, &ha) };
            if !straight(line, ctx)? {
                continue;
            }
            let origin = &line[0];
            let last = line.last().unwrap();
            let dx = last[0].sub(&origin[0], ctx)?;
            let dy = last[1].sub(&origin[1], ctx)?;
            let yy = sub(&curve[1], &scaled(&curve[2], &origin[1], ctx)?, ctx)?;
            let xx = sub(&curve[0], &scaled(&curve[2], &origin[0], ctx)?, ctx)?;
            let equation = sub(&scaled(&yy, &dx, ctx)?, &scaled(&xx, &dy, ctx)?, ctx)?;
            let reduction = PlanarReduction::LineCarrier { first };
            if zero(&equation) {
                return Ok(PlanarScalarOutcome::Coincident { reduction });
            }
            return Ok(PlanarScalarOutcome::Roots {
                reduction,
                roots: isolate(&equation, parameter_tolerance, ctx)?,
            });
        }
        Ok(PlanarScalarOutcome::NotReducible)
    })();
    let computed = computed.and_then(|v| ctx.charge(0).map(|_| v));
    Ok(PlanarScalarDecision {
        outcome: computed.unwrap_or_else(PlanarScalarOutcome::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn arbitrary_pair(a: &[[f64; 3]], b: &[[f64; 3]], limits: Limits) -> PlanarScalarOutcome {
        let source = SourceArena::authored(
            "planar-projected-root-test",
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
                .map(|i| std::array::from_fn(|k| source.leaf(i * 3 + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        planar_bezier_scalar_roots(&mut ctx, &refs(0, a.len()), &refs(a.len(), b.len()), 1e-12)
            .unwrap()
            .outcome
    }
    #[test]
    fn projected_coordinate_retains_multiple_contacts_and_nearby_bifurcations() {
        let ya = [0., 0., 1., 3.];
        let yb = [1., -1., 1., 7.];
        let frame = |i: usize, y: f64| [17. + 10. * i as f64 - y, -9. + 10. * i as f64 + y, 1.];
        let a = std::array::from_fn::<_, 4, _>(|i| frame(i, ya[i]));
        for (delta, count, multiplicity) in [
            (0., 1, 2),
            (2_f64.powi(-30), 0, 1),
            (-2_f64.powi(-30), 2, 1),
        ] {
            let b = std::array::from_fn::<_, 4, _>(|i| frame(i, yb[i] + delta));
            let PlanarScalarOutcome::Roots { reduction, roots } =
                arbitrary_pair(&a, &b, Limits::default())
            else {
                panic!("Missing projected reduction")
            };
            assert_eq!(reduction, PlanarReduction::CommonProjection);
            assert_eq!(roots.len(), count);
            assert!(roots.iter().all(|r| r.multiplicity == multiplicity));
            if delta == 0. {
                assert!(roots[0].interval[0] <= 1. / 3. && roots[0].interval[1] >= 1. / 3.)
            }
            if count == 2 {
                assert!(roots[0].interval[1] < roots[1].interval[0])
            }
        }
        let b = std::array::from_fn::<_, 4, _>(|i| frame(i, yb[i]));
        let mut limits = Limits::default();
        limits.max_work = 0;
        assert!(matches!(
            arbitrary_pair(&a, &b, limits),
            PlanarScalarOutcome::Indeterminate(Reason::ResourceLimit)
        ));
    }
    #[test]
    fn common_coordinate_uses_actual_monotonicity_and_original_weights() {
        let x = [0., 1., 0.9, 1.9];
        let ya = [0., 0., 1., 3.];
        let yb = [1., -1., 1., 7.];
        let mut a = std::array::from_fn::<_, 4, _>(|i| [x[i], ya[i], 1.]);
        let mut b = std::array::from_fn::<_, 4, _>(|i| [x[i], yb[i], 1.]);
        let PlanarScalarOutcome::Roots { reduction, roots } =
            arbitrary_pair(&a, &b, Limits::default())
        else {
            panic!("Missing actual monotonicity proof")
        };
        assert_eq!(reduction, PlanarReduction::CommonCoordinate(0));
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].multiplicity, 2);
        a[1][2] = 100.;
        a[2][2] = 100.;
        b[1][2] = 100.;
        b[2][2] = 100.;
        assert!(matches!(
            arbitrary_pair(&a, &b, Limits::default()),
            PlanarScalarOutcome::NotReducible | PlanarScalarOutcome::Indeterminate(_)
        ));
    }
    fn pair(y: &[f64; 4], limits: Limits) -> PlanarScalarOutcome {
        let a = (0..4).map(|i| [i as f64, 0., 1.]).collect::<Vec<_>>();
        let b = (0..4).map(|i| [i as f64, y[i], 1.]).collect::<Vec<_>>();
        let source = SourceArena::authored(
            "planar-scalar-test",
            1,
            a.iter()
                .chain(&b)
                .flatten()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let refs = |start: usize| {
            (start..start + 4)
                .map(|i| std::array::from_fn(|k| source.leaf(i * 3 + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        planar_bezier_scalar_roots(&mut ctx, &refs(0), &refs(4), 1e-12)
            .unwrap()
            .outcome
    }
    #[test]
    fn isolates_non_dyadic_multiple_root_and_distinguishes_both_nearby_perturbations() {
        let PlanarScalarOutcome::Roots { roots, .. } = pair(&[1., -1., 0., 4.], Limits::default())
        else {
            panic!("Missing exact reduction")
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].multiplicity, 2);
        assert!(roots[0].interval[0] <= 1. / 3. && roots[0].interval[1] >= 1. / 3.);
        let delta = 2_f64.powi(-40);
        let PlanarScalarOutcome::Roots { roots, .. } = pair(
            &[1. + delta, -1. + delta, delta, 4. + delta],
            Limits::default(),
        ) else {
            panic!("Missing exclusion")
        };
        assert!(roots.is_empty());
        let PlanarScalarOutcome::Roots { roots, .. } = pair(
            &[1. - delta, -1. - delta, -delta, 4. - delta],
            Limits::default(),
        ) else {
            panic!("Missing separated roots")
        };
        assert_eq!(roots.len(), 2);
        assert!(roots.iter().all(|r| r.multiplicity == 1));
        assert!(roots[0].interval[1] < roots[1].interval[0]);
    }
    #[test]
    fn fraction_free_sturm_preserves_multiplicity_and_endpoint_ownership() {
        let source = SourceArena::authored(
            "sturm-test",
            1,
            vec![AuthoredScalar::Binary64Bits(0_f64.to_bits())],
        )
        .unwrap();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        for (coefficients, expected) in [
            (vec![-0.1875, 1., -1.75, 1.], vec![(0.5, 2), (0.75, 1)]),
            (vec![0., 0., 0., -1., 3., -3., 1.], vec![(0., 3), (1., 3)]),
            (vec![1., -12., 60., -160., 240., -192., 64.], vec![(0.5, 6)]),
        ] {
            let polynomial = coefficients.into_iter().map(Expansion::scalar).collect();
            let roots = isolate(&polynomial, 1e-12, &mut ctx).unwrap();
            assert_eq!(roots.len(), expected.len());
            for (root, (t, m)) in roots.iter().zip(expected) {
                assert_eq!(root.interval, [t, t]);
                assert_eq!(root.multiplicity, m);
            }
        }
    }
    #[test]
    fn exhausted_predicate_work_never_becomes_a_disjoint_or_coincident_report() {
        assert!(matches!(
            pair(
                &[0.; 4],
                Limits {
                    max_work: 0,
                    ..Limits::default()
                }
            ),
            PlanarScalarOutcome::Indeterminate(Reason::ResourceLimit)
        ));
    }
}

#[derive(Clone, Debug)]
pub enum PlanarStationaryOutcome {
    Roots(Vec<PlanarScalarRoot>),
    Constant,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct PlanarStationaryDecision {
    pub outcome: PlanarStationaryOutcome,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// Exact roots of d(X/W)/dt on one original rational Bézier definition.
pub fn planar_bezier_stationary_roots(
    ctx: &mut PredicateContext<'_>,
    source: &[[LeafRef; 3]],
    axis: usize,
    tolerance: f64,
) -> Result<PlanarStationaryDecision, InputError> {
    if !(2..=9).contains(&source.len())
        || axis >= 2
        || !tolerance.is_finite()
        || tolerance <= 0.
        || tolerance >= 1.
    {
        return Err(InputError::InvalidInput(
            "Invalid planar stationary root input",
        ));
    }
    let values = source
        .iter()
        .flatten()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let outcome = (|| -> Result<PlanarStationaryOutcome, Reason> {
        let exact = exact_inputs(&values, ctx)?;
        let points: Vec<[Expansion; 3]> = exact
            .chunks_exact(3)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect();
        if points.iter().any(|p| p[2].sign() != Sign::Positive) {
            return Err(Reason::MissingProof);
        }
        let h = homogeneous(&points, ctx)?;
        let equation = sub(
            &mul(&derivative(&h[axis], ctx)?, &h[2], ctx)?,
            &mul(&h[axis], &derivative(&h[2], ctx)?, ctx)?,
            ctx,
        )?;
        if zero(&equation) {
            Ok(PlanarStationaryOutcome::Constant)
        } else {
            Ok(PlanarStationaryOutcome::Roots(isolate(
                &equation, tolerance, ctx,
            )?))
        }
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(PlanarStationaryDecision {
        outcome: outcome.unwrap_or_else(PlanarStationaryOutcome::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod stationary_tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn cusp(axis: usize, limits: Limits) -> PlanarStationaryOutcome {
        let points = [[1., -1., 1.], [-1., 2., 1.], [0., -4., 1.], [4., 8., 1.]];
        let source = SourceArena::authored(
            "stationary-cusp-test",
            1,
            points
                .iter()
                .flatten()
                .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let refs = (0..4)
            .map(|i| std::array::from_fn(|k| source.leaf(i * 3 + k).unwrap()))
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        planar_bezier_stationary_roots(&mut ctx, &refs, axis, 1e-14)
            .unwrap()
            .outcome
    }
    #[test]
    fn stationary_cusp_preserves_distinct_derivative_multiplicities_and_uncertainty() {
        for (axis, multiplicity) in [(0, 1), (1, 2)] {
            let PlanarStationaryOutcome::Roots(roots) = cusp(axis, Limits::default()) else {
                panic!("stationary proof absent")
            };
            assert_eq!(roots.len(), 1);
            assert_eq!(roots[0].multiplicity, multiplicity);
            assert!(roots[0].interval[0] <= 1. / 3. && roots[0].interval[1] >= 1. / 3.);
        }
        let mut limits = Limits::default();
        limits.max_work = 0;
        assert!(matches!(
            cusp(0, limits),
            PlanarStationaryOutcome::Indeterminate(Reason::ResourceLimit)
        ));
    }
}
