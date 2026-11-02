use super::*;
pub(in super::super) fn extraction(
    knots: &[f64],
    p: usize,
    span: usize,
    w: &mut Work,
) -> Result<Vec<Vec<Q>>> {
    check(
        p <= 24 && span >= p && span + p + 1 < knots.len() && knots[span] < knots[span + 1],
        "Invalid exact spline span",
    )?;
    let k = knots
        .iter()
        .copied()
        .map(authored)
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::new();
    for r in 0..=p {
        let args: Vec<_> = (0..p)
            .map(|i| k[span + usize::from(i >= p - r)].clone())
            .collect();
        let mut d: Vec<Vec<Q>> = (0..=p)
            .map(|i| {
                (0..=p)
                    .map(|j| if i == j { Q::one() } else { Q::zero() })
                    .collect()
            })
            .collect();
        for level in 1..=p {
            for j in (level..=p).rev() {
                let left = span - p + j;
                let den = &k[span + 1 + j - level] - &k[left];
                check(!den.is_zero(), "Singular exact spline extraction")?;
                let alpha = (&args[level - 1] - &k[left]) / den;
                for i in 0..=p {
                    d[j][i] = mix(&d[j - 1][i], &d[j][i], &alpha, w)?;
                }
            }
        }
        out.push(d.remove(p));
    }
    Ok(out)
}


pub(super) fn mix(a: &Q, b: &Q, t: &Q, w: &mut Work) -> Result<Q> {
    let q = a + t * (b - a);
    w.charge(&q)?;
    Ok(q)
}
pub(super) fn blossom(mut row: Vec<Q>, arguments: &[Q], w: &mut Work) -> Result<Q> {
    check(
        row.len() == arguments.len() + 1,
        "Invalid exact blossom degree",
    )?;
    for t in arguments {
        row = row
            .windows(2)
            .map(|p| mix(&p[0], &p[1], t, w))
            .collect::<Result<Vec<_>>>()?;
    }
    Ok(row.remove(0))
}
