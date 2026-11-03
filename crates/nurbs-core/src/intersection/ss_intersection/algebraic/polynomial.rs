//! Exact sparse rational polynomials in the four ORIGINAL local span clocks.
use super::*;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SparseSsPolynomial<const N: usize> {
    pub(super) terms: BTreeMap<[usize; N], Q>,
}
impl<const N: usize> SparseSsPolynomial<N> {
    pub fn terms(&self) -> &BTreeMap<[usize; N], Q> {
        &self.terms
    }
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }
    pub fn evaluate(&self, point: &[Q; N]) -> Q {
        let mut sum = Q::zero();
        for (e, c) in &self.terms {
            let mut term = c.clone();
            for k in 0..N {
                for _ in 0..e[k] {
                    term *= &point[k];
                }
            }
            sum += term;
        }
        sum
    }
    pub(super) fn insert(&mut self, e: [usize; N], c: Q, w: &mut Work) -> Result<()> {
        w.charge()?;
        let next = self.terms.get(&e).cloned().unwrap_or_default() + c;
        w.coefficient(&next)?;
        if next.is_zero() {
            self.terms.remove(&e);
        } else {
            self.terms.insert(e, next);
        }
        if self.terms.len() > w.limits.max_terms {
            return Err(crate::resource(
                "Exact SS polynomial term allowance exhausted",
            ));
        }
        Ok(())
    }
    pub(super) fn scale_monomial(&self, e: [usize; N], c: &Q, w: &mut Work) -> Result<Self> {
        let mut out = Self::default();
        for (a, b) in &self.terms {
            let mut exponent = [0; N];
            for k in 0..N {
                exponent[k] = a[k]
                    .checked_add(e[k])
                    .ok_or_else(|| crate::resource("Exact SS monomial overflow"))?;
            }
            w.charge()?;
            let coefficient = b * c;
            w.coefficient(&coefficient)?;
            out.insert(exponent, coefficient, w)?;
        }
        Ok(out)
    }
    pub(super) fn subtract(&self, other: &Self, w: &mut Work) -> Result<Self> {
        let mut out = self.clone();
        for (e, c) in &other.terms {
            out.insert(*e, -c, w)?;
        }
        Ok(out)
    }
    pub(super) fn product(&self, other: &Self, w: &mut Work) -> Result<Self> {
        let mut out = Self::default();
        for (a, b) in &self.terms {
            let partial = other.scale_monomial(*a, b, w)?;
            for (e, c) in partial.terms {
                out.insert(e, c, w)?;
            }
        }
        Ok(out)
    }
    pub fn derivative(&self, axis: usize) -> Result<Self> {
        check(axis < N, "Invalid exact SS derivative axis")?;
        let mut terms = BTreeMap::new();
        for (e, c) in &self.terms {
            if e[axis] > 0 {
                let mut d = *e;
                d[axis] -= 1;
                terms.insert(d, c * Q::from_integer(e[axis].into()));
            }
        }
        Ok(Self { terms })
    }
}

pub type ExactSsPolynomial = SparseSsPolynomial<4>;
