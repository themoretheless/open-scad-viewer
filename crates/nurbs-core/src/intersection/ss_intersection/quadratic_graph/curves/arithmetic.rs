use super::super::*;
pub(super) type P = Vec<I>;
pub(super) struct Arithmetic {
    pub used: usize,
    pub max: usize,
}
impl Arithmetic {
    pub fn tick(&mut self, n: usize) -> Result<()> {
        if n > self.max - self.used {
            return Err(crate::resource(
                "Original SS curve construction work exhausted",
            ));
        }
        self.used += n;
        Ok(())
    }
    pub fn add(&mut self, a: I, b: I) -> Result<I> {
        self.tick(1)?;
        a.add(b)
    }
    pub fn sub(&mut self, a: I, b: I) -> Result<I> {
        self.tick(1)?;
        a.sub(b)
    }
    pub fn mul(&mut self, a: I, b: I) -> Result<I> {
        self.tick(1)?;
        a.mul(b)
    }
    pub fn div(&mut self, a: I, b: I) -> Result<I> {
        self.tick(1)?;
        a.div(b)
    }
    pub fn sqrt(&mut self, a: I) -> Result<I> {
        self.tick(1)?;
        I::new(
            a.lo.max(0.).sqrt().next_down().max(0.),
            a.hi.sqrt().next_up(),
        )
    }
    pub fn midpoint(&mut self, a: I) -> Result<f64> {
        self.tick(3)?;
        Ok(a.lo * 0.5 + a.hi * 0.5)
    }
    pub fn scale(&mut self, a: &P, b: I) -> Result<P> {
        a.iter().map(|&a| self.mul(a, b)).collect()
    }
    pub fn sum(&mut self, a: &P, b: &P) -> Result<P> {
        check(a.len() == b.len(), "SS polynomial degree mismatch")?;
        a.iter().zip(b).map(|(&a, &b)| self.add(a, b)).collect()
    }
    pub fn product(&mut self, a: &P, b: &P) -> Result<P> {
        let (m, n) = (a.len() - 1, b.len() - 1);
        check(m + n <= 16, "SS curve polynomial degree exceeded")?;
        let choose = |n: usize, k: usize| -> u64 {
            let mut v = 1;
            for i in 0..k {
                v = v * (n - i) as u64 / (i + 1) as u64;
            }
            v
        };
        let mut out = vec![I::point(0.); m + n + 1];
        for i in 0..=m {
            for j in 0..=n {
                let numerator = choose(m, i) * choose(n, j);
                let denominator = choose(m + n, i + j);
                let ratio = self.div(I::point(numerator as f64), I::point(denominator as f64))?;
                let term = self.mul(a[i], b[j])?;
                let term = self.mul(term, ratio)?;
                out[i + j] = self.add(out[i + j], term)?;
            }
        }
        Ok(out)
    }
    pub fn power(&mut self, a: &P, n: usize) -> Result<P> {
        let mut out = vec![I::point(1.)];
        for _ in 0..n {
            out = self.product(&out, a)?;
        }
        Ok(out)
    }
    pub fn norm_upper(&mut self, components: &[I]) -> Result<f64> {
        let mut square = I::point(0.);
        for c in components {
            let a = c.lo.abs().max(c.hi.abs());
            let term = self.mul(I::point(a), I::point(a))?;
            square = self.add(square, term)?;
        }
        Ok(self.sqrt(square)?.hi)
    }
}
#[derive(Clone)]
pub(super) struct Rational {
    pub numerator: Vec<P>,
    pub denominator: P,
}
impl Rational {
    pub fn degree(&self) -> usize {
        self.denominator.len() - 1
    }
}
