//! Exact restriction of an ORIGINAL boundary curve and a separated graph.
use super::*;
pub struct CurveGraphProjectionDecision {
    pub proven: bool,
    pub work_used: u64,
    pub context: ContextIdentity,
    pub reason: Option<Reason>,
}
fn decode(
    raw: &[AuthoredScalar],
    shape: [usize; 2],
    axis: usize,
    reverse: bool,
    axes: [usize; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<[Net; 3], Reason> {
    let [n, m] = shape;
    let (p, q) = if axis == 0 { (n, m) } else { (m, n) };
    let mut h = std::array::from_fn(|_| vec![vec![Expansion::scalar(0.); q]; p]);
    for i in 0..p {
        for j in 0..q {
            let v = if reverse { q - 1 - j } else { j };
            let (a, b) = if axis == 0 { (i, v) } else { (v, i) };
            let at = 4 * (a * m + b);
            let x = |k| match raw[at + k] {
                AuthoredScalar::Binary64Bits(b) => Expansion::scalar(f64::from_bits(b)),
                _ => unreachable!(),
            };
            let w = x(3);
            if w.sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            h[0][i][j] = x(axes[0]).mul(&w, ctx)?;
            h[1][i][j] = x(axes[1]).mul(&w, ctx)?;
            h[2][i][j] = w;
        }
    }
    Ok(h)
}
fn restrict(
    a: &Net,
    span: &[Expansion; 2],
    ends: &[Expansion; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<Net, Reason> {
    let p = a[0].len() - 1;
    let mut row = Vec::new();
    for k in 0..=p {
        let mut b = a[0].clone();
        for level in 0..p {
            let t = &ends[usize::from(level >= p - k)];
            let left = span[1].sub(t, ctx)?;
            let right = t.sub(&span[0], ctx)?;
            for j in 0..p - level {
                b[j] = left
                    .mul(&b[j], ctx)?
                    .add(&right.mul(&b[j + 1], ctx)?, ctx)?;
            }
        }
        row.push(b[0].clone());
    }
    Ok(vec![row])
}
/// One source boundary, with strictly monotone ordering coordinate, is restricted
/// by literal ORIGINAL raw-parameter endpoint leaves. Homogeneous blossom
/// coefficients retain the positive (span width)^degree gauge, never rounded
/// inserted knots or Cartesian poles. The graph's ordering coordinate equals
/// this restricted curve's and is independent of its transverse parameter.
/// Its height differs strictly from the curve except on its declared side.
/// Equal projected points must therefore lie on that graph side AND this curve
/// restriction. Whole-curve injectivity excludes the unused source subarcs.
/// Caller must prove the other surface's contact image is this boundary curve.
pub fn binary_curve_graph_projection(
    ctx: &mut PredicateContext<'_>,
    sources: [&[Vec<[LeafRef; 4]>]; 2],
    transverse: [usize; 2],
    upper: [bool; 2],
    reverse_graph: bool,
    axes: [usize; 2],
    curve_span: [LeafRef; 2],
    restriction: [LeafRef; 2],
) -> Result<CurveGraphProjectionDecision, InputError> {
    if axes.iter().any(|a| *a > 2)
        || axes[0] == axes[1]
        || transverse.iter().any(|a| *a > 1)
        || sources.iter().any(|s| {
            !(2..=9).contains(&s.len())
                || s.first().is_none_or(|r| !(2..=9).contains(&r.len()))
                || s.iter().any(|r| r.len() != s[0].len())
        })
    {
        return Err(InputError::InvalidInput(
            "Invalid original curve/graph source layout",
        ));
    }
    let raw = sources
        .iter()
        .map(|s| {
            s.iter()
                .flatten()
                .flatten()
                .map(|&r| ctx.resolve(r).cloned())
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let parameters = curve_span
        .iter()
        .chain(&restriction)
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    if raw
        .iter()
        .flatten()
        .chain(&parameters)
        .any(|x| !matches!(x, AuthoredScalar::Binary64Bits(_)))
    {
        return Err(InputError::InvalidInput(
            "Curve graph requires literal original leaves",
        ));
    }
    let result = (|| -> Result<(), Reason> {
        ctx.charge(raw.iter().map(Vec::len).sum::<usize>() as u64 + 4)?;
        let e = parameters
            .iter()
            .map(|x| match x {
                AuthoredScalar::Binary64Bits(b) => Expansion::scalar(f64::from_bits(*b)),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        let span = [e[0].clone(), e[1].clone()];
        let ends = [e[2].clone(), e[3].clone()];
        if span[1].sub(&span[0], ctx)?.sign() != Sign::Positive
            || ends[1].sub(&ends[0], ctx)?.sign() == Sign::Zero
        {
            return Err(Reason::MissingProof);
        }
        for t in &ends {
            if t.sub(&span[0], ctx)?.sign() == Sign::Negative
                || span[1].sub(t, ctx)?.sign() == Sign::Negative
            {
                return Err(Reason::MissingProof);
            }
        }
        let h0 = decode(
            &raw[0],
            [sources[0].len(), sources[0][0].len()],
            transverse[0],
            false,
            axes,
            ctx,
        )?;
        let curve = h0.map(|p| boundary(&p, upper[0]));
        let dy = derivative(&curve[0], ctx)?;
        let dw = derivative(&curve[2], ctx)?;
        if strict(&cross(&dy, &curve[2], &curve[0], &dw, ctx)?).is_none() {
            return Err(Reason::MissingProof);
        }
        let trace = curve
            .iter()
            .map(|a| restrict(a, &span, &ends, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let graph = decode(
            &raw[1],
            [sources[1].len(), sources[1][0].len()],
            transverse[1],
            reverse_graph,
            axes,
            ctx,
        )?;
        if !zero(&cross(&graph[0], &trace[2], &graph[2], &trace[0], ctx)?) {
            return Err(Reason::MissingProof);
        }
        if support(
            &cross(&graph[1], &trace[2], &graph[2], &trace[1], ctx)?,
            upper[1],
        )
        .is_none()
        {
            return Err(Reason::MissingProof);
        }
        Ok(())
    })();
    Ok(CurveGraphProjectionDecision {
        proven: result.is_ok(),
        reason: result.err(),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests;
