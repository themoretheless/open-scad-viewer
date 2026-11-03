//! Exact bounded Buchberger elimination over Q. Ideal equivalence preserves
//! multiple/tangent roots; it does NOT assert that the ideal is radical or that
//! its complex dimension equals the dimension of the real/world zero set.
use super::polynomial::SparseSsPolynomial;
use super::*;
use std::collections::VecDeque;
#[derive(Clone, Copy, Debug)]
pub struct ExactSsGroebnerLimits {
    pub algebra: ExactSsAlgebraLimits,
    pub max_pairs: usize,
    pub max_basis: usize,
    /// Lexicographic priority of the ORIGINAL local span coordinates.
    pub variable_order: [usize; 4],
}
impl Default for ExactSsGroebnerLimits {
    fn default() -> Self {
        Self {
            algebra: Default::default(),
            max_pairs: 4096,
            max_basis: 128,
            variable_order: [0, 1, 2, 3],
        }
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedSsGroebnerBasis {
    original: QualifiedOriginalSsIdeal,
    limits: ExactSsGroebnerLimits,
    basis: Vec<ExactSsPolynomial>,
    work: usize,
    pairs: usize,
    dimension: Option<usize>,
}
impl QualifiedSsGroebnerBasis {
    pub fn original(&self) -> &QualifiedOriginalSsIdeal {
        &self.original
    }
    pub fn polynomials(&self) -> &[ExactSsPolynomial] {
        &self.basis
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn pairs(&self) -> usize {
        self.pairs
    }
    pub fn proves_disjoint_on_span(&self) -> bool {
        self.dimension.is_none()
    }
    pub fn complex_dimension(&self) -> Option<usize> {
        self.dimension
    }
    pub fn reverify(&self) -> Result<()> {
        self.original.reverify()?;
        let fresh = qualify_ss_groebner_basis(&self.original, self.limits)?;
        check(
            self.basis == fresh.basis
                && self.work == fresh.work
                && self.pairs == fresh.pairs
                && self.dimension == fresh.dimension,
            "Original SS elimination ideal or its dimension changed",
        )
    }
    pub fn remainder(
        &self,
        polynomial: &ExactSsPolynomial,
        limits: ExactSsAlgebraLimits,
    ) -> Result<ExactSsPolynomial> {
        validate_limits(limits)?;
        normal_form(
            polynomial,
            &self.basis,
            self.limits.variable_order,
            &mut Work { used: 0, limits },
        )
    }
}
fn compare<const N: usize>(
    a: &[usize; N],
    b: &[usize; N],
    order: [usize; N],
) -> std::cmp::Ordering {
    for k in order {
        let cmp = a[k].cmp(&b[k]);
        if !cmp.is_eq() {
            return cmp;
        }
    }
    std::cmp::Ordering::Equal
}
fn lead<const N: usize>(p: &SparseSsPolynomial<N>, order: [usize; N]) -> Option<([usize; N], Q)> {
    p.terms
        .iter()
        .max_by(|(a, _), (b, _)| compare(a, b, order))
        .map(|(e, c)| (*e, c.clone()))
}
fn divides<const N: usize>(a: [usize; N], b: [usize; N]) -> bool {
    (0..N).all(|k| a[k] <= b[k])
}
fn monic<const N: usize>(
    p: SparseSsPolynomial<N>,
    order: [usize; N],
    w: &mut Work,
) -> Result<SparseSsPolynomial<N>> {
    let Some((_, coefficient)) = lead(&p, order) else {
        return Ok(p);
    };
    w.charge()?;
    let scale = Q::one() / coefficient;
    w.coefficient(&scale)?;
    p.scale_monomial([0; N], &scale, w)
}
fn normal_form<const N: usize>(
    input: &SparseSsPolynomial<N>,
    basis: &[SparseSsPolynomial<N>],
    order: [usize; N],
    w: &mut Work,
) -> Result<SparseSsPolynomial<N>> {
    let mut p = input.clone();
    let mut remainder = SparseSsPolynomial::<N>::default();
    while let Some((exponent, coefficient)) = lead(&p, order) {
        w.charge()?;
        let mut reduced = false;
        for g in basis {
            let Some((ge, gc)) = lead(g, order) else {
                continue;
            };
            if !divides(ge, exponent) {
                continue;
            }
            let shift = std::array::from_fn(|k| exponent[k] - ge[k]);
            w.charge()?;
            let scale = &coefficient / gc;
            w.coefficient(&scale)?;
            let term = g.scale_monomial(shift, &scale, w)?;
            p = p.subtract(&term, w)?;
            reduced = true;
            break;
        }
        if !reduced {
            p.terms.remove(&exponent);
            remainder.insert(exponent, coefficient, w)?;
        }
    }
    Ok(remainder)
}
fn dimension(basis: &[ExactSsPolynomial], order: [usize; 4]) -> Option<usize> {
    let leading = basis
        .iter()
        .filter_map(|p| lead(p, order).map(|v| v.0))
        .collect::<Vec<_>>();
    if leading.contains(&[0; 4]) {
        return None;
    }
    Some(
        (0_u32..16)
            .filter(|mask| {
                leading
                    .iter()
                    .all(|e| (0..4).any(|k| e[k] > 0 && mask & (1 << k) == 0))
            })
            .map(|mask| mask.count_ones() as usize)
            .max()
            .unwrap(),
    )
}
/// Native source polynomialization and elimination have separate explicit work
/// allowances. Exhaustion never returns a purported complete elimination basis.
pub fn qualify_ss_groebner_basis(
    original: &QualifiedOriginalSsIdeal,
    limits: ExactSsGroebnerLimits,
) -> Result<QualifiedSsGroebnerBasis> {
    validate_limits(limits.algebra)?;
    let mut order = limits.variable_order;
    order.sort();
    check(
        order == [0, 1, 2, 3]
            && (1..=65536).contains(&limits.max_pairs)
            && (1..=256).contains(&limits.max_basis),
        "Invalid original SS elimination order or work",
    )?;
    let mut work = Work {
        used: 0,
        limits: limits.algebra,
    };
    let order = limits.variable_order;
    let (basis, pairs) = basis_for(
        &original.equations,
        order,
        limits.max_pairs,
        limits.max_basis,
        &mut work,
    )?;
    let dimension = dimension(&basis, order);
    Ok(QualifiedSsGroebnerBasis {
        original: original.clone(),
        limits,
        basis,
        work: work.used,
        pairs,
        dimension,
    })
}

pub(super) fn basis_for<const N: usize>(
    input: &[SparseSsPolynomial<N>],
    order: [usize; N],
    max_pairs: usize,
    max_basis: usize,
    work: &mut Work,
) -> Result<(Vec<SparseSsPolynomial<N>>, usize)> {
    let mut basis = Vec::new();
    for f in input {
        let p = normal_form(f, &basis, order, work)?;
        if !p.is_zero() {
            if basis.len() == max_basis {
                return Err(crate::resource(
                    "Exact original SS basis allowance exhausted",
                ));
            }
            basis.push(monic(p, order, work)?);
        }
    }
    let mut pending = VecDeque::new();
    for i in 0..basis.len() {
        for j in 0..i {
            pending.push_back((j, i));
        }
    }
    let mut pairs = 0;
    while let Some((a, b)) = pending.pop_front() {
        if pairs == max_pairs {
            return Err(crate::resource(
                "Exact original SS critical-pair work exhausted",
            ));
        }
        pairs += 1;
        work.charge()?;
        let (ae, ac) = lead(&basis[a], order).unwrap();
        let (be, bc) = lead(&basis[b], order).unwrap();
        if (0..N).all(|k| ae[k].min(be[k]) == 0) {
            continue;
        }
        let lcm: [usize; N] = std::array::from_fn(|k| ae[k].max(be[k]));
        let left = basis[a].scale_monomial(
            std::array::from_fn(|k| lcm[k] - ae[k]),
            &(Q::one() / ac),
            work,
        )?;
        let right = basis[b].scale_monomial(
            std::array::from_fn(|k| lcm[k] - be[k]),
            &(Q::one() / bc),
            work,
        )?;
        let s = left.subtract(&right, work)?;
        let remainder = normal_form(&s, &basis, order, work)?;
        if !remainder.is_zero() {
            if basis.len() == max_basis {
                return Err(crate::resource(
                    "Exact original SS basis allowance exhausted",
                ));
            }
            let p = monic(remainder, order, work)?;
            if lead(&p, order).unwrap().0 == [0; N] {
                basis = vec![p];
                break;
            }
            let n = basis.len();
            for i in 0..n {
                pending.push_back((i, n));
            }
            basis.push(p);
        }
    }
    // Keep one polynomial per minimal leading monomial. Removing duplicates
    // sequentially avoids deleting both copies of an equivalent generator.
    basis.sort_by(|a, b| {
        compare(
            &lead(a, order).unwrap().0,
            &lead(b, order).unwrap().0,
            order,
        )
    });
    let mut minimal: Vec<SparseSsPolynomial<N>> = Vec::new();
    for p in basis {
        let e = lead(&p, order).unwrap().0;
        if !minimal
            .iter()
            .any(|g| divides(lead(g, order).unwrap().0, e))
        {
            minimal.push(p);
        }
    }
    let mut basis = Vec::new();
    for i in 0..minimal.len() {
        let other = minimal
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, p)| p.clone())
            .collect::<Vec<_>>();
        basis.push(monic(
            normal_form(&minimal[i], &other, order, work)?,
            order,
            work,
        )?);
    }
    basis.sort_by(|a, b| {
        compare(
            &lead(a, order).unwrap().0,
            &lead(b, order).unwrap().0,
            order,
        )
    });

    Ok((basis, pairs))
}
