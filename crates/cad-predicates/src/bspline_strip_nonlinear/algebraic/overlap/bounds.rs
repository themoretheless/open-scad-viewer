//! Source-bound exact Bernstein subdivision caches. Subdivision never changes
//! authored geometry; all coefficients descend from the original polynomials.
use super::*;
#[derive(Default)]
pub(super) struct Bounds {
    deflated: BTreeMap<(usize, usize, usize, usize, usize), Option<P>>,
    signs: BTreeMap<(usize, usize, usize), bool>,
    slopes: BTreeMap<(usize, usize, usize, usize, usize), P>,
    boxes: BTreeMap<(usize, usize, usize, usize, usize, usize), Vec<P>>,
}
fn half(p: &P, right: bool, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let mut work = p.clone();
    let mut left = vec![work[0].clone()];
    let mut rhs = vec![work.last().unwrap().clone()];
    for level in 1..p.len() {
        for i in 0..p.len() - level {
            work[i] = if work[i].n.sign() == Sign::Zero && work[i + 1].n.sign() == Sign::Zero {
                Q::zero()
            } else {
                work[i]
                    .add(&work[i + 1], ctx)?
                    .div_positive(&integer(2), ctx)?
            };
        }
        left.push(work[0].clone());
        rhs.push(work[p.len() - 1 - level].clone());
    }
    if right {
        rhs.reverse();
        Ok(rhs)
    } else {
        Ok(left)
    }
}
fn coefficients(p: &P, lo: &Q, hi: &Q, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let mut p = p.clone();
    if let Some(pivot) = p.iter().find(|q| q.n.sign() != Sign::Zero).cloned() {
        let pivot = if pivot.n.sign() == Sign::Negative {
            Q::zero().sub(&pivot, ctx)?
        } else {
            pivot
        };
        for q in &mut p {
            *q = q.div_positive(&pivot, ctx)?;
        }
    }
    let local = compose(
        &p,
        &vec![lo.clone(), hi.sub(lo, ctx)?],
        &vec![one()],
        p.len() - 1,
        ctx,
    )?;
    bernstein(&local, ctx)
}
// A nonzero polynomial with same-signed Bernstein coefficients is strictly
// signed in the OPEN domain. Zeros on its boundary are allowed.
fn one_sign(
    p: impl Iterator<Item = Sign>,
    sign: Option<Sign>,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    let mut sign = sign;
    let mut nonzero = false;
    for s in p {
        ctx.charge(1)?;
        if s == Sign::Zero {
            continue;
        }
        if sign.is_some_and(|x| x != s) {
            return Ok(false);
        }
        sign = Some(s);
        nonzero = true;
    }
    Ok(nonzero)
}
fn tensor(p: &B, a: [&Q; 2], b: [&Q; 2], ctx: &mut PredicateContext<'_>) -> Result<Vec<P>, Reason> {
    if p.is_empty() {
        return Ok(Vec::new());
    }
    let mut rows = to_t(p);
    let degree_s = rows.iter().map(Vec::len).max().unwrap() - 1;
    let degree_t = rows.len() - 1;
    for row in &mut rows {
        *row = compose(
            row,
            &vec![a[0].clone(), a[1].sub(a[0], ctx)?],
            &vec![one()],
            row.len() - 1,
            ctx,
        )?;
        row.resize(degree_s + 1, Q::zero());
        *row = bernstein(row, ctx)?;
    }
    let mut out = Vec::new();
    for i in 0..=degree_s {
        let p = rows.iter().map(|row| row[i].clone()).collect::<P>();
        let mut p = compose(
            &p,
            &vec![b[0].clone(), b[1].sub(b[0], ctx)?],
            &vec![one()],
            p.len() - 1,
            ctx,
        )?;
        p.resize(degree_t + 1, Q::zero());
        out.push(bernstein(&p, ctx)?);
    }
    Ok(out)
}
/// Address of one refinement box: the span pair, the cofactor axis, the subdivision level and the
/// box indices along each parameter at that level.
#[derive(Clone, Copy, Debug)]
pub(super) struct BoxKey {
    pub a: usize,
    pub b: usize,
    pub axis: usize,
    pub level: usize,
    pub pa: usize,
    pub pb: usize,
}
/// Non-empty knot spans and knot vectors of both strip parameters.
#[derive(Clone, Copy)]
pub(super) struct StripKnots<'a> {
    pub spans: &'a [Vec<(usize, Vec<H>)>],
    pub u: &'a [Vec<Q>],
}
/// One polynomial piece: strip `side`, knot span `span`, coordinate `axis`,
/// and the `part`-th of the `2^level` equal subintervals of that span.
#[derive(Clone, Copy, Debug)]
pub(super) struct PieceKey {
    pub side: usize,
    pub span: usize,
    pub axis: usize,
    pub level: usize,
    pub part: usize,
}
impl PieceKey {
    fn key(self) -> (usize, usize, usize, usize, usize) {
        (self.side, self.span, self.axis, self.level, self.part)
    }
    /// The piece one subdivision level up that contains this one.
    fn parent(self) -> Self {
        Self {
            level: self.level - 1,
            part: self.part / 2,
            ..self
        }
    }
}
/// Numerator of the derivative of coordinate `axis` over the span's homogeneous
/// weight, with the span's knot index.
fn slope_numerator(
    spans: &[(usize, Vec<H>)],
    span: usize,
    axis: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<(usize, P), Reason> {
    let (i, h) = &spans[span];
    let n = &h[0][axis];
    let d = &h[0][3];
    let p = sub(
        &mul(&derivative(n, ctx)?, d, ctx)?,
        &mul(n, &derivative(d, ctx)?, ctx)?,
        ctx,
    )?;
    Ok((*i, p))
}
impl Bounds {
    fn slope_coefficients(
        &mut self,
        piece: PieceKey,
        spans: &[(usize, Vec<H>)],
        u: &[Q],
        ctx: &mut PredicateContext<'_>,
    ) -> Result<P, Reason> {
        let key = piece.key();
        if let Some(p) = self.slopes.get(&key) {
            return Ok(p.clone());
        }
        let p = if piece.level == 0 {
            let (i, p) = slope_numerator(spans, piece.span, piece.axis, ctx)?;
            coefficients(&p, &u[i], &u[i + 1], ctx)?
        } else {
            let parent = self.slope_coefficients(piece.parent(), spans, u, ctx)?;
            half(&parent, piece.part % 2 == 1, ctx)?
        };
        self.slopes.insert(key, p.clone());
        Ok(p)
    }
    fn deflated_coefficients(
        &mut self,
        piece: PieceKey,
        spans: &[(usize, Vec<H>)],
        u: &[Q],
        ctx: &mut PredicateContext<'_>,
    ) -> Result<Option<P>, Reason> {
        let key = piece.key();
        if let Some(p) = self.deflated.get(&key) {
            return Ok(p.clone());
        }
        let p = if piece.level == 0 {
            let (i, p) = slope_numerator(spans, piece.span, piece.axis, ctx)?;
            match polynomial::even_deflation(&p, ctx)? {
                Some(p) => Some(coefficients(&p, &u[i], &u[i + 1], ctx)?),
                None => None,
            }
        } else {
            match self.deflated_coefficients(piece.parent(), spans, u, ctx)? {
                Some(p) => Some(half(&p, piece.part % 2 == 1, ctx)?),
                None => None,
            }
        };
        self.deflated.insert(key, p.clone());
        Ok(p)
    }
    pub(super) fn slope(
        &mut self,
        piece: PieceKey,
        spans: &[(usize, Vec<H>)],
        u: &[Q],
        sign: Sign,
        ctx: &mut PredicateContext<'_>,
    ) -> Result<bool, Reason> {
        let p = self.slope_coefficients(piece, spans, u, ctx)?;
        if one_sign(p.iter().map(|q| q.n.sign()), Some(sign), ctx)? {
            return Ok(true);
        }
        let Some(p) = self.deflated_coefficients(piece, spans, u, ctx)?
        else {
            return Ok(false);
        };
        // The removed factor is a nonzero polynomial square: its zeros are
        // finite, so the original coordinate remains strictly monotone.
        one_sign(p.iter().map(|q| q.n.sign()), Some(sign), ctx)
    }
    fn box_coefficients(
        &mut self,
        at: BoxKey,
        p: &B,
        knots: StripKnots<'_>,
        ctx: &mut PredicateContext<'_>,
    ) -> Result<Vec<P>, Reason> {
        let BoxKey { a, b, axis, level, pa, pb } = at;
        let StripKnots { spans, u } = knots;
        let key = (a, b, axis, level, pa, pb);
        if let Some(p) = self.boxes.get(&key) {
            return Ok(p.clone());
        }
        let out = if level == 0 {
            let ai = spans[0][a].0;
            let bi = spans[1][b].0;
            tensor(
                p,
                [&u[0][ai], &u[0][ai + 1]],
                [&u[1][bi], &u[1][bi + 1]],
                ctx,
            )?
        } else {
            let parent =
                self.box_coefficients(
                    BoxKey { level: level - 1, pa: pa / 2, pb: pb / 2, ..at },
                    p,
                    knots,
                    ctx,
                )?;
            if parent.is_empty() {
                return Ok(parent);
            }
            let mut rows = Vec::new();
            for row in parent {
                rows.push(half(&row, pb % 2 == 1, ctx)?);
            }
            let mut out = vec![vec![Q::zero(); rows[0].len()]; rows.len()];
            for j in 0..rows[0].len() {
                let col = rows.iter().map(|row| row[j].clone()).collect();
                let col = half(&col, pa % 2 == 1, ctx)?;
                for (i, q) in col.into_iter().enumerate() {
                    out[i][j] = q;
                }
            }
            out
        };
        self.boxes.insert(key, out.clone());
        Ok(out)
    }
    // For sign-definite g, any interior zero has grad(g)=0. On the
    // coordinate graph F=f*g=0 this also forces grad(F)=0, hence BOTH
    // selected coordinate derivatives vanish. Monotonicity above allows
    // only finitely many such critical parameter pairs. Off that finite
    // set g!=0 implies f=0; continuity proves f=0 at the remaining nodes.
    // Thus a critical projection cannot choose another real branch. Actual
    // source regularity remains mandatory at the public admission boundary.
    pub(super) fn cofactor(
        &mut self,
        at: BoxKey,
        p: &B,
        knots: StripKnots<'_>,
        ctx: &mut PredicateContext<'_>,
    ) -> Result<bool, Reason> {
        let BoxKey { a, b, axis, .. } = at;
        if p.is_empty() {
            return Ok(false);
        }
        let coeff = self.box_coefficients(at, p, knots, ctx)?;
        if one_sign(coeff.iter().flatten().map(|q| q.n.sign()), None, ctx)? {
            return Ok(true);
        }
        let key = (a, b, axis);
        if let Some(sign) = self.signs.get(&key) {
            return Ok(*sign);
        }
        let sign = semidefinite::certificate(p, ctx)?
            || circuits::certificate(p, ctx)?
            || gram::certificate(p, ctx)?
            || pullback::certificate(p, ctx)?;
        self.signs.insert(key, sign);
        Ok(sign)
    }
}
