//! General source-equivalent repeated-factor removal. Independent exact
//! divisibility in BOTH directions proves zero-set equality; a guessed factor,
//! numerical multiplicity or sampled residual cannot authorize deflation.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactSsDeflationProof {
    original: ExactSsPolynomial,
    deflated: ExactSsPolynomial,
    factor: ExactSsPolynomial,
    power: usize,
}
impl ExactSsDeflationProof {
    pub fn original(&self) -> &ExactSsPolynomial {
        &self.original
    }
    pub fn deflated(&self) -> &ExactSsPolynomial {
        &self.deflated
    }
    pub fn removed_factor(&self) -> &ExactSsPolynomial {
        &self.factor
    }
    pub fn power_witness(&self) -> usize {
        self.power
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedSsDeflatedSystem {
    original: QualifiedSsGroebnerBasis,
    equations: Vec<ExactSsPolynomial>,
    proofs: Vec<ExactSsDeflationProof>,
    limits: ExactSsAlgebraLimits,
    work: usize,
}
impl QualifiedSsDeflatedSystem {
    pub fn original(&self) -> &QualifiedSsGroebnerBasis {
        &self.original
    }
    pub fn equations(&self) -> &[ExactSsPolynomial] {
        &self.equations
    }
    pub fn proofs(&self) -> &[ExactSsDeflationProof] {
        &self.proofs
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn reverify(&self) -> Result<()> {
        self.original.reverify()?;
        let fresh = qualify_ss_deflated_system(&self.original, self.limits)?;
        check(
            self.equations == fresh.equations
                && self.proofs == fresh.proofs
                && self.work == fresh.work,
            "Original SS factor removal or zero-set equivalence changed",
        )
    }
}
fn modulo_power(
    base: &ExactSsPolynomial,
    mut exponent: usize,
    modulus: &ExactSsPolynomial,
    w: &mut Work,
) -> Result<ExactSsPolynomial> {
    let mut result = division::constant(Q::one());
    let mut base = division::divide(base, modulus, w)?.1;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = division::divide(&result.product(&base, w)?, modulus, w)?.1;
        }
        exponent >>= 1;
        if exponent > 0 {
            base = division::divide(&base.product(&base, w)?, modulus, w)?.1;
        }
    }
    Ok(result)
}
/// Each equation H has exactly the same zeros as its original generator F:
/// F=H*G and F divides H^N. Both identities are recomputed from exact source
/// coefficients. The product's repeated factors may be removed without losing
/// any contact branch, including nonseparable and multivariate multiplicities.
/// Intrinsic singularities of the reduced zero set still need stratification.
pub fn qualify_ss_deflated_system(
    original: &QualifiedSsGroebnerBasis,
    limits: ExactSsAlgebraLimits,
) -> Result<QualifiedSsDeflatedSystem> {
    validate_limits(limits)?;
    let mut work = Work { used: 0, limits };
    let mut equations = Vec::new();
    let mut proofs = Vec::new();
    for f in original.polynomials() {
        let variables = gcd::variables(f);
        let mut factor = f.clone();
        for &axis in &variables {
            factor = gcd::compute(&factor, &f.derivative(axis)?, &variables, &mut work)?;
            if division::is_one(&factor) {
                break;
            }
        }
        factor = division::monic(&factor, &mut work)?;
        let deflated = division::exact(f, &factor, &mut work)?;
        let power = f
            .terms
            .keys()
            .map(|e| e.iter().sum())
            .max()
            .unwrap_or(0)
            .max(1);
        check(
            deflated.product(&factor, &mut work)? == *f,
            "Exact source SS factor identity failed",
        )?;
        if deflated != *f {
            check(
                modulo_power(&deflated, power, f, &mut work)?.is_zero(),
                "Exact SS deflation loses original roots",
            )?;
        }
        proofs.push(ExactSsDeflationProof {
            original: f.clone(),
            deflated: deflated.clone(),
            factor,
            power,
        });
        equations.push(deflated);
    }
    Ok(QualifiedSsDeflatedSystem {
        original: original.clone(),
        equations,
        proofs,
        limits,
        work: work.used,
    })
}
