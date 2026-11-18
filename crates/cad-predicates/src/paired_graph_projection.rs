//! Exact original coefficient premises for two ordered graph projections.
//! Supplies no topology, region, ownership or whole-face injectivity credit.
use crate::projected_surface_jacobian::mul;
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Expansion, InputError, LeafRef, PredicateContext,
    Reason, Sign,
};
type Net = Vec<Vec<Expansion>>;
pub struct PairedGraphProjectionDecision {
    pub directions: Option<[Sign; 2]>,
    pub work_used: u64,
    pub context: ContextIdentity,
    pub reason: Option<Reason>,
}
fn difference(a: Net, b: Net, ctx: &mut PredicateContext<'_>) -> Result<Net, Reason> {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| a.into_iter().zip(b).map(|(a, b)| a.sub(&b, ctx)).collect())
        .collect()
}
fn cross(
    a: &Net,
    b: &Net,
    c: &Net,
    d: &Net,
    ctx: &mut PredicateContext<'_>,
) -> Result<Net, Reason> {
    difference(mul(a, b, [None; 2], ctx)?, mul(c, d, [None; 2], ctx)?, ctx)
}
fn boundary(a: &Net, upper: bool) -> Net {
    vec![a[if upper { a.len() - 1 } else { 0 }].clone()]
}
fn zero(a: &Net) -> bool {
    a.iter().flatten().all(|x| x.sign() == Sign::Zero)
}
fn derivative(a: &Net, ctx: &mut PredicateContext<'_>) -> Result<Net, Reason> {
    let degree = a[0].len() - 1;
    let p = Expansion::integer(degree as u64, ctx)?;
    Ok(vec![
        (0..degree)
            .map(|j| a[0][j + 1].sub(&a[0][j], ctx)?.mul(&p, ctx))
            .collect::<Result<Vec<_>, _>>()?,
    ])
}
fn strict(a: &Net) -> Option<Sign> {
    let sign = a.first()?.first()?.sign();
    (sign != Sign::Zero && a.iter().flatten().all(|x| x.sign() == sign)).then_some(sign)
}
fn support(a: &Net, upper: bool) -> Option<Sign> {
    let boundary = if upper { a.len() - 1 } else { 0 };
    let mut sign = None;
    for (i, row) in a.iter().enumerate() {
        for x in row {
            let s = x.sign();
            if i == boundary {
                if s != Sign::Zero {
                    return None;
                }
            } else if s == Sign::Zero || sign.is_some_and(|a| a != s) {
                return None;
            } else {
                sign = Some(s);
            }
        }
    }
    sign
}
/// XYZ+positive-weight leaves in clamped SINGLE Bezier source order. `axes`
/// select an ordering coordinate and a height. Source parameter proposals give
/// the transverse axis, its shared endpoint, and reversal of the second trace.
/// Raw knot/span validation and natural-boundary ownership remain caller duties.
///
/// Proves Y(u,v)=y(v), strict monotonic y, a common boundary height z(v), and
/// opposite strict signs of Z(u,v)-z(v) away from each declared natural side.
/// Equal projected points must therefore be on BOTH declared boundaries.
pub fn binary_paired_graph_projection(
    ctx: &mut PredicateContext<'_>,
    sources: [&[Vec<[LeafRef; 4]>]; 2],
    transverse: [usize; 2],
    upper: [bool; 2],
    reverse_second: bool,
    axes: [usize; 2],
) -> Result<PairedGraphProjectionDecision, InputError> {
    if axes[0] > 2
        || axes[1] > 2
        || axes[0] == axes[1]
        || transverse.iter().any(|a| *a > 1)
        || sources.iter().any(|s| {
            !(2..=9).contains(&s.len())
                || s.first().is_none_or(|r| !(2..=9).contains(&r.len()))
                || s.iter().any(|r| r.len() != s[0].len())
        })
    {
        return Err(InputError::InvalidInput(
            "Invalid paired graph coefficient layout",
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
    if raw
        .iter()
        .flatten()
        .any(|x| !matches!(x, AuthoredScalar::Binary64Bits(_)))
    {
        return Err(InputError::InvalidInput(
            "Paired graph requires original binary64 leaves",
        ));
    }
    let result = (|| -> Result<[Sign; 2], Reason> {
        ctx.charge(raw.iter().map(Vec::len).sum::<usize>() as u64)?;
        let mut h = Vec::new();
        for slot in 0..2 {
            let (n, m) = (sources[slot].len(), sources[slot][0].len());
            let (p, q) = if transverse[slot] == 0 {
                (n, m)
            } else {
                (m, n)
            };
            let mut nets = vec![vec![vec![Expansion::scalar(0.); q]; p]; 3];
            for i in 0..p {
                for j in 0..q {
                    let j0 = if slot == 1 && reverse_second {
                        q - 1 - j
                    } else {
                        j
                    };
                    let (a, b) = if transverse[slot] == 0 {
                        (i, j0)
                    } else {
                        (j0, i)
                    };
                    let at = 4 * (a * m + b);
                    let values = &raw[slot][at..at + 4];
                    let scalar = |k| match values[k] {
                        AuthoredScalar::Binary64Bits(b) => Expansion::scalar(f64::from_bits(b)),
                        _ => unreachable!(),
                    };
                    let w = scalar(3);
                    if w.sign() != Sign::Positive {
                        return Err(Reason::MissingProof);
                    }
                    nets[0][i][j] = scalar(axes[0]).mul(&w, ctx)?;
                    nets[1][i][j] = scalar(axes[1]).mul(&w, ctx)?;
                    nets[2][i][j] = w;
                }
            }
            h.push(nets);
        }
        let mut traces = Vec::new();
        let mut directions = Vec::new();
        for slot in 0..2 {
            let y = boundary(&h[slot][0], upper[slot]);
            let z = boundary(&h[slot][1], upper[slot]);
            let w = boundary(&h[slot][2], upper[slot]);
            if !zero(&cross(&h[slot][0], &w, &h[slot][2], &y, ctx)?) {
                return Err(Reason::MissingProof);
            }
            let height = cross(&h[slot][1], &w, &h[slot][2], &z, ctx)?;
            directions.push(support(&height, upper[slot]).ok_or(Reason::MissingProof)?);
            traces.push([y, z, w]);
        }
        if directions[0] == directions[1] {
            return Err(Reason::MissingProof);
        }
        for axis in 0..2 {
            if !zero(&cross(
                &traces[0][axis],
                &traces[1][2],
                &traces[0][2],
                &traces[1][axis],
                ctx,
            )?) {
                return Err(Reason::MissingProof);
            }
        }
        let dy = derivative(&traces[0][0], ctx)?;
        let dw = derivative(&traces[0][2], ctx)?;
        if strict(&cross(&dy, &traces[0][2], &traces[0][0], &dw, ctx)?).is_none() {
            return Err(Reason::MissingProof);
        }
        Ok([directions[0], directions[1]])
    })();
    Ok(PairedGraphProjectionDecision {
        directions: result.as_ref().ok().copied(),
        reason: result.err(),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests;

mod curve;
pub use curve::{CurveGraphProjectionDecision, ProjectionSource, binary_curve_graph_projection};
