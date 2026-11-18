//! Exact regular/singular decomposition of a source-equivalent equation system.
//! All maximal Jacobian minors are included; no sampled rank selects branches.
//! Singularity here means failure of a rank-three parameter-root chart. It is
//! not a claim about intrinsic real dimension or physical world valence.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct SsRankStrataLimits {
    pub elimination: ExactSsGroebnerLimits,
    pub max_minors: usize,
}
impl Default for SsRankStrataLimits {
    fn default() -> Self {
        Self {
            elimination: Default::default(),
            max_minors: 4096,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactSsRankLocation {
    OffSource,
    Regular,
    Singular,
}
#[derive(Clone, Debug)]
pub struct QualifiedSsRankStrata {
    source: QualifiedSsDeflatedSystem,
    minors: Vec<ExactSsPolynomial>,
    singular: Vec<ExactSsPolynomial>,
    limits: SsRankStrataLimits,
    work: usize,
    pairs: usize,
    considered: usize,
}
impl QualifiedSsRankStrata {
    pub fn source(&self) -> &QualifiedSsDeflatedSystem {
        &self.source
    }
    pub fn nonzero_minors(&self) -> &[ExactSsPolynomial] {
        &self.minors
    }
    pub fn singular_equations(&self) -> &[ExactSsPolynomial] {
        &self.singular
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn considered_minors(&self) -> usize {
        self.considered
    }
    pub fn proves_no_singular_roots(&self) -> bool {
        self.singular
            .iter()
            .any(|p| p.terms.len() == 1 && p.terms.contains_key(&[0; 4]))
    }
    pub fn classify_exact(
        &self,
        point: [Q; 4],
        limits: ExactSsAlgebraLimits,
    ) -> Result<ExactSsRankLocation> {
        validate_limits(limits)?;
        let mut w = Work { used: 0, limits };
        for v in &point {
            w.coefficient(v)?;
        }
        check(
            point.iter().all(|v| Q::zero() <= *v && *v <= Q::one()),
            "Rank query leaves original source spans",
        )?;
        for f in self.source.equations() {
            if !localization::evaluate(f, &point, &mut w)?.is_zero() {
                return Ok(ExactSsRankLocation::OffSource);
            }
        }
        for f in &self.minors {
            if !localization::evaluate(f, &point, &mut w)?.is_zero() {
                return Ok(ExactSsRankLocation::Regular);
            }
        }
        Ok(ExactSsRankLocation::Singular)
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let f = qualify_ss_rank_strata(&self.source, self.limits)?;
        check(
            self.minors == f.minors
                && self.singular == f.singular
                && self.work == f.work
                && self.pairs == f.pairs
                && self.considered == f.considered,
            "Original SS maximal minor stratification changed",
        )
    }
}
fn derivative(p: &ExactSsPolynomial, axis: usize, w: &mut Work) -> Result<ExactSsPolynomial> {
    let mut out = ExactSsPolynomial::default();
    for (e, c) in &p.terms {
        if e[axis] > 0 {
            w.charge()?;
            let v = c * Q::from_integer(e[axis].into());
            w.coefficient(&v)?;
            let mut d = *e;
            d[axis] -= 1;
            out.insert(d, v, w)?;
        }
    }
    Ok(out)
}
fn determinant(
    rows: &[[ExactSsPolynomial; 4]],
    ids: [usize; 3],
    axes: [usize; 3],
    w: &mut Work,
) -> Result<ExactSsPolynomial> {
    let mut out = ExactSsPolynomial::default();
    for (p, sign) in [
        ([0, 1, 2], 1),
        ([1, 2, 0], 1),
        ([2, 0, 1], 1),
        ([2, 1, 0], -1),
        ([1, 0, 2], -1),
        ([0, 2, 1], -1),
    ] {
        let term = rows[ids[0]][axes[p[0]]]
            .product(&rows[ids[1]][axes[p[1]]], w)?
            .product(&rows[ids[2]][axes[p[2]]], w)?;
        for (e, c) in term.terms {
            out.insert(e, c * Q::from_integer(sign.into()), w)?;
        }
    }
    Ok(out)
}
/// V(I) is the disjoint union of V(I, all rank-three minors) and its regular
/// complement. At every regular point at least one original coordinate chart
/// exists by the implicit function theorem. All minors participate in this
/// identity, even when multiple generator triples are necessary.
pub fn qualify_ss_rank_strata(
    source: &QualifiedSsDeflatedSystem,
    limits: SsRankStrataLimits,
) -> Result<QualifiedSsRankStrata> {
    validate_limits(limits.elimination.algebra)?;
    let mut order = limits.elimination.variable_order;
    order.sort();
    check(
        (1..=65536).contains(&limits.max_minors)
            && order == [0, 1, 2, 3]
            && (1..=65536).contains(&limits.elimination.max_pairs)
            && (1..=256).contains(&limits.elimination.max_basis),
        "Invalid original rank-stratum work",
    )?;
    let mut w = Work {
        used: 0,
        limits: limits.elimination.algebra,
    };
    let mut jacobian = Vec::new();
    for f in source.equations() {
        let mut row = std::array::from_fn(|_| ExactSsPolynomial::default());
        for k in 0..4 {
            row[k] = derivative(f, k, &mut w)?;
        }
        jacobian.push(row);
    }
    let mut minors = Vec::new();
    let mut considered = 0;
    for a in 0..jacobian.len() {
        for b in 0..a {
            for c in 0..b {
                for omitted in 0..4 {
                    check(
                        considered < limits.max_minors,
                        "Original SS maximal minor allowance exhausted",
                    )?;
                    considered += 1;
                    let axes = (0..4)
                        .filter(|&k| k != omitted)
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap();
                    let d = determinant(&jacobian, [c, b, a], axes, &mut w)?;
                    if !d.is_zero() && !minors.contains(&d) {
                        minors.push(d);
                    }
                }
            }
        }
    }
    let mut input = source.equations().to_vec();
    input.extend(minors.iter().cloned());
    let (singular, pairs) = groebner::basis_for(
        &input,
        limits.elimination.variable_order,
        limits.elimination.max_pairs,
        limits.elimination.max_basis,
        &mut w,
    )?;
    Ok(QualifiedSsRankStrata {
        source: source.clone(),
        minors,
        singular,
        limits,
        work: w.used,
        pairs,
        considered,
    })
}
