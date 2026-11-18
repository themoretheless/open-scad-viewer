//! Direct exact triangular input, without converting rational controls to F64.
use super::*;
pub type AtlasExactTriangle = Vec<Vec<[ExactRatio; 4]>>;
fn input(net: &AtlasExactTriangle, axis: u8, level: &ExactRatio) -> Result<(String, Vec<Vec<Q>>)> {
    if axis > 2
        || net.len() < 2
        || net.len() > 9
        || net
            .iter()
            .enumerate()
            .any(|(i, row)| row.len() != net.len() - i)
    {
        return Err(fail(
            "algebraic triangular pullback shape/degree budget exceeded",
        ));
    }
    let level = roots::decode(level)?;
    let h = net
        .iter()
        .map(|row| {
            row.iter()
                .map(|p| {
                    Ok([
                        roots::decode(&p[0])?,
                        roots::decode(&p[1])?,
                        roots::decode(&p[2])?,
                        roots::decode(&p[3])?,
                    ])
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Net>>()?;
    transport::check(&h)?;
    if h.iter().flatten().any(|p| p[3] <= Q::zero()) {
        return Err(fail(
            "implicit triangular pullback requires a strictly positive denominator",
        ));
    }
    let n = h.len() - 1;
    let mut equation = vec![vec![Q::zero(); n + 1]; n + 1];
    let mut work = Work::default();
    for (i, row) in h.iter().enumerate() {
        for (j, h) in row.iter().enumerate() {
            let coefficient = &h[axis as usize] - &level * &h[3];
            let k = n - i - j;
            let outer = choose(n, i) * choose(n - i, j);
            for a in 0..=k {
                for b in 0..=k - a {
                    work.charge(1)?;
                    let mut q = &coefficient * &outer * choose(k, a) * choose(k - a, b);
                    if (a + b) % 2 != 0 {
                        q = -q;
                    }
                    equation[j + b][i + a] += q;
                }
            }
        }
    }
    for row in &mut equation {
        if row
            .iter()
            .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
        {
            return Err(fail("triangular implicit coefficient budget exceeded"));
        }
        while row.last().is_some_and(Q::is_zero) {
            row.pop();
        }
    }
    while equation.last().is_some_and(Vec::is_empty) {
        equation.pop();
    }
    let mut hash = Sha256::new();
    hash.update(b"source-atlas-triangular-pullback-v1");
    hash.update([axis]);
    hash.update((n as u64).to_le_bytes());
    for p in h.iter().flatten() {
        for q in p {
            for s in roots::ratio(q) {
                hash.update((s.len() as u64).to_le_bytes());
                hash.update(s.as_bytes());
            }
        }
    }
    for s in roots::ratio(&level) {
        hash.update((s.len() as u64).to_le_bytes());
        hash.update(s.as_bytes());
    }
    let source = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok((source, equation))
}
pub fn certify_algebraic_triangle_pullback(
    net: &AtlasExactTriangle,
    axis: u8,
    level: &ExactRatio,
) -> Result<AtlasAlgebraicPullback> {
    certify_input(input(net, axis, level)?)
}
pub fn verify_algebraic_triangle_pullback(
    net: &AtlasExactTriangle,
    axis: u8,
    level: &ExactRatio,
    cert: &AtlasAlgebraicPullback,
) -> Result<()> {
    verify_input(input(net, axis, level)?, cert)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_curved_algebraic_preimage_binds_the_exact_triangular_map() {
        let half = Q::new(1.into(), 2.into());
        let h = (0..=2)
            .map(|i| {
                (0..=2 - i)
                    .map(|j| {
                        [
                            Q::new(i.into(), 2.into()),
                            Q::new(j.into(), 2.into()),
                            (if i == 2 { Q::one() } else { Q::zero() })
                                + (if j == 2 { Q::one() } else { Q::zero() })
                                - &half,
                            Q::one(),
                        ]
                        .map(|q| roots::ratio(&q))
                    })
                    .collect()
            })
            .collect::<AtlasExactTriangle>();
        let level = roots::ratio(&Q::zero());
        let cert = certify_algebraic_triangle_pullback(&h, 2, &level).unwrap();
        verify_algebraic_triangle_pullback(&h, 2, &level, &cert).unwrap();
        assert!(cert
            .branches
            .cuts
            .iter()
            .any(|c| c.parameter[0] != c.parameter[1]));
        assert!(cert
            .branches
            .cuts
            .iter()
            .any(|c| c.excluded_nodes.iter().any(|x| *x)));
        let mut changed = h.clone();
        changed[1][0][2] = roots::ratio(&Q::zero());
        assert!(verify_algebraic_triangle_pullback(&changed, 2, &level, &cert).is_err());
        let mut bad = cert.clone();
        bad.branches.cuts.remove(1);
        assert!(verify_algebraic_triangle_pullback(&h, 2, &level, &bad).is_err());
    }
}
