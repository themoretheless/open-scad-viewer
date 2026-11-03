//! Exact localization using the elimination theorem; no numerical factors.
use super::polynomial::SparseSsPolynomial;
use super::*;
pub(super) fn saturate(
    equations: &[ExactSsPolynomial],
    multiplier: &ExactSsPolynomial,
    limits: ExactSsGroebnerLimits,
    w: &mut Work,
) -> Result<(Vec<ExactSsPolynomial>, usize)> {
    let mut input: Vec<SparseSsPolynomial<5>> = equations
        .iter()
        .map(|p| SparseSsPolynomial {
            terms: p
                .terms
                .iter()
                .map(|(e, c)| ([e[0], e[1], e[2], e[3], 0], c.clone()))
                .collect(),
        })
        .collect();
    let mut inverse = SparseSsPolynomial::<5>::default();
    inverse.insert([0; 5], Q::one(), w)?;
    for (e, c) in &multiplier.terms {
        inverse.insert([e[0], e[1], e[2], e[3], 1], -c, w)?;
    }
    input.push(inverse);
    let o = limits.variable_order;
    let (basis, pairs) = groebner::basis_for(
        &input,
        [4, o[0], o[1], o[2], o[3]],
        limits.max_pairs,
        limits.max_basis,
        w,
    )?;
    let equations = basis
        .into_iter()
        .filter(|p| p.terms.keys().all(|e| e[4] == 0))
        .map(|p| ExactSsPolynomial {
            terms: p
                .terms
                .into_iter()
                .map(|(e, c)| ([e[0], e[1], e[2], e[3]], c))
                .collect(),
        })
        .collect();
    Ok((equations, pairs))
}
pub(super) fn substitute(
    p: &ExactSsPolynomial,
    map: &[ExactSsPolynomial; 4],
    w: &mut Work,
) -> Result<ExactSsPolynomial> {
    let mut out = ExactSsPolynomial::default();
    for (e, c) in &p.terms {
        let mut term = division::constant(c.clone());
        for k in 0..4 {
            let mut exponent = e[k];
            let mut base = map[k].clone();
            while exponent > 0 {
                if exponent % 2 == 1 {
                    term = term.product(&base, w)?;
                }
                exponent /= 2;
                if exponent > 0 {
                    base = base.product(&base, w)?;
                }
            }
        }
        for (e, c) in term.terms {
            out.insert(e, c, w)?;
        }
    }
    Ok(out)
}
pub(super) fn evaluate(p: &ExactSsPolynomial, point: &[Q; 4], w: &mut Work) -> Result<Q> {
    let mut sum = Q::zero();
    for (e, c) in &p.terms {
        let mut term = c.clone();
        for k in 0..4 {
            for _ in 0..e[k] {
                w.charge()?;
                term *= &point[k];
                w.coefficient(&term)?;
            }
        }
        w.charge()?;
        sum += term;
        w.coefficient(&sum)?;
    }
    Ok(sum)
}
