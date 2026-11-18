//! Exact rational functions in the lower-level polynomial ring.
use super::*;
#[derive(Clone, Debug)]
pub(super) struct Fraction {
    pub numerator: ExactSsPolynomial,
    pub denominator: ExactSsPolynomial,
}
impl Fraction {
    pub fn polynomial(p: ExactSsPolynomial) -> Self {
        Self {
            numerator: p,
            denominator: constant(Q::one()),
        }
    }
    fn normalized(
        numerator: ExactSsPolynomial,
        denominator: ExactSsPolynomial,
        variables: &[usize],
        w: &mut Work,
    ) -> Result<Self> {
        check(
            !denominator.is_zero(),
            "Exact SS fraction denominator vanished",
        )?;
        if numerator.is_zero() {
            return Ok(Self::polynomial(numerator));
        }
        if is_one(&denominator) {
            return Ok(Self::polynomial(numerator));
        }
        let gcd = compute(&numerator, &denominator, variables, w)?;
        let numerator = exact(&numerator, &gcd, w)?;
        let denominator = exact(&denominator, &gcd, w)?;
        let (_, leading) = super::super::division::lead(&denominator).unwrap();
        w.charge()?;
        let scale = Q::one() / leading;
        w.coefficient(&scale)?;
        Ok(Self {
            numerator: numerator.scale_monomial([0; 4], &scale, w)?,
            denominator: denominator.scale_monomial([0; 4], &scale, w)?,
        })
    }
    pub fn product(&self, other: &Self, variables: &[usize], w: &mut Work) -> Result<Self> {
        Self::normalized(
            self.numerator.product(&other.numerator, w)?,
            self.denominator.product(&other.denominator, w)?,
            variables,
            w,
        )
    }
    pub fn subtract(&self, other: &Self, variables: &[usize], w: &mut Work) -> Result<Self> {
        let a = self.numerator.product(&other.denominator, w)?;
        let b = other.numerator.product(&self.denominator, w)?;
        Self::normalized(
            a.subtract(&b, w)?,
            self.denominator.product(&other.denominator, w)?,
            variables,
            w,
        )
    }
    pub fn divide(&self, other: &Self, variables: &[usize], w: &mut Work) -> Result<Self> {
        check(
            !other.numerator.is_zero(),
            "Exact SS field division by zero",
        )?;
        Self::normalized(
            self.numerator.product(&other.denominator, w)?,
            self.denominator.product(&other.numerator, w)?,
            variables,
            w,
        )
    }
}
