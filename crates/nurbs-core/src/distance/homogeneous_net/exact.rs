//! Exact scalar identities for homogeneous polynomial construction only.
//! Public Interval outward-rounding and frozen binary contracts stay unchanged.
use super::Interval as I;
use crate::Result;
pub(crate) trait ExactIdentities {
    fn exact_add(self, b: Self) -> Result<Self>
    where
        Self: Sized;
    fn exact_sub(self, b: Self) -> Result<Self>
    where
        Self: Sized;
    fn exact_mul(self, b: Self) -> Result<Self>
    where
        Self: Sized;
    fn exact_div(self, b: Self) -> Result<Self>
    where
        Self: Sized;
}
fn point(a: I, x: f64) -> bool {
    a.lo == x && a.hi == x
}
impl ExactIdentities for I {
    fn exact_add(self, b: Self) -> Result<Self> {
        if point(self, 0.) || point(b, 0.) {
            return I::new(self.lo + b.lo, self.hi + b.hi);
        }
        self.add(b)
    }
    fn exact_sub(self, b: Self) -> Result<Self> {
        if point(b, 0.) {
            return I::new(self.lo - b.lo, self.hi - b.hi);
        }
        if point(self, 0.) {
            return I::new(-b.hi, -b.lo);
        }
        if self.lo == self.hi && b.lo == b.hi && self.lo == b.lo {
            return I::new(self.lo - b.lo, self.hi - b.hi);
        }
        self.sub(b)
    }
    fn exact_mul(self, b: Self) -> Result<Self> {
        if (point(self, 0.) || point(b, 0.))
            && [self.lo, self.hi, b.lo, b.hi].iter().all(|x| x.is_finite())
        {
            return I::new(self.lo * b.lo, self.hi * b.hi);
        }
        if point(self, 1.) {
            return I::new(b.lo, b.hi);
        }
        if point(b, 1.) {
            return I::new(self.lo, self.hi);
        }
        self.mul(b)
    }
    fn exact_div(self, b: Self) -> Result<Self> {
        if b.lo > 0. && b.hi.is_finite() && b.lo <= b.hi {
            if point(self, 0.) || point(b, 1.) {
                return I::new(self.lo / b.lo, self.hi / b.lo);
            }
            if self.lo == self.hi && b.lo == b.hi && self.lo == b.lo {
                return Ok(I::point(1.));
            }
        }
        self.div(b)
    }
}
