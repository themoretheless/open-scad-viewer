//! Univariate sixth-order interval Taylor coefficients, f^(k)/k!.
use super::expression::Scalar;
use super::*;
#[derive(Clone, Copy)]
struct T([I; 7]);
impl T {
    fn variable(x: I, varying: bool) -> Self {
        let mut a = [I::point(0.); 7];
        a[0] = x;
        if varying {
            a[1] = I::point(1.);
        }
        Self(a)
    }
}
impl Scalar for T {
    fn c(x: I) -> Self {
        Self::variable(x, false)
    }
    fn value(self) -> I {
        self.0[0]
    }
    fn add(self, b: Self) -> Result<Self> {
        let mut a = self.0;
        for (x, y) in a.iter_mut().zip(b.0) {
            *x = x.add(y)?;
        }
        Ok(Self(a))
    }
    fn sub(self, b: Self) -> Result<Self> {
        let mut a = self.0;
        for (x, y) in a.iter_mut().zip(b.0) {
            *x = x.sub(y)?;
        }
        Ok(Self(a))
    }
    fn mul(self, b: Self) -> Result<Self> {
        let mut a = [I::point(0.); 7];
        for n in 0..7 {
            for k in 0..=n {
                a[n] = a[n].add(self.0[k].mul(b.0[n - k])?)?;
            }
        }
        Ok(Self(a))
    }
    fn div(self, b: Self) -> Result<Self> {
        let mut a = [I::point(0.); 7];
        a[0] = self.0[0].div_signed(b.0[0])?;
        for n in 1..7 {
            let mut residual = self.0[n];
            for k in 1..=n {
                residual = residual.sub(b.0[k].mul(a[n - k])?)?;
            }
            a[n] = residual.div_signed(b.0[0])?;
        }
        Ok(Self(a))
    }
    fn sin_cos(self) -> Result<(Self, Self)> {
        let mut s = [I::point(0.); 7];
        let mut c = s;
        (s[0], c[0]) = crate::involute::certified_trig(self.0[0])?;
        for n in 1..7 {
            let mut sin = I::point(0.);
            let mut cos = I::point(0.);
            for k in 1..=n {
                let d = I::point(k as f64).mul(self.0[k])?;
                sin = sin.add(d.mul(c[n - k])?)?;
                cos = cos.sub(d.mul(s[n - k])?)?;
            }
            s[n] = sin.div(I::point(n as f64))?;
            c[n] = cos.div(I::point(n as f64))?;
        }
        Ok((Self(s), Self(c)))
    }
    fn sqrt(self) -> Result<Self> {
        let mut out = [I::point(0.); 7];
        out[0] = super::expression::sqrt_positive(self.0[0])?;
        let divisor = I::point(2.).mul(out[0])?;
        for n in 1..=6 {
            let mut residual = self.0[n];
            for k in 1..n {
                residual = residual.sub(out[k].mul(out[n - k])?)?;
            }
            out[n] = residual.div(divisor)?;
        }
        Ok(Self(out))
    }
    fn asin(self) -> Result<Self> {
        let denominator = T::c(I::point(1.)).sub(self.mul(self)?)?.sqrt()?.0;
        let mut out = [I::point(0.); 7];
        out[0] = super::expression::asin_interval(self.0[0])?;
        let mut derivative = [I::point(0.); 6];
        for n in 0..6 {
            let mut residual = I::point((n + 1) as f64).mul(self.0[n + 1])?;
            for k in 1..=n {
                residual = residual.sub(denominator[k].mul(derivative[n - k])?)?;
            }
            derivative[n] = residual.div(denominator[0])?;
            out[n + 1] = derivative[n].div(I::point((n + 1) as f64))?;
        }
        Ok(Self(out))
    }
    fn restrict_value(mut self, x: I) -> Result<Self> {
        self.0[0] = self.0[0].intersect(x.lo, x.hi)?;
        Ok(self)
    }
}
pub(super) fn sixth(spec: Spec, flank: Flank, domain: ChartDomain, axis: usize) -> Result<[I; 3]> {
    let u = T::variable(iv(domain.u)?, axis == 0);
    let b = T::variable(iv(domain.beta)?, axis == 1);
    Ok(super::expression::evaluate(spec, flank, u, b)?
        .point
        .map(|t| t.0[6]))
}
// Solve the coefficient-n equation F_U * U_n + remainder_n = 0.
// `base` encloses the already proved original root throughout beta's interval;
// `derivative` encloses F_U there and must exclude zero.
fn implicit_coefficients(
    base: I,
    derivative: I,
    mut evaluate: impl FnMut(T) -> Result<T>,
) -> Result<T> {
    check(
        derivative.lo > 0. || derivative.hi < 0.,
        "Stock implicit derivative is not separated from zero",
    )?;
    let mut root = T::c(base);
    for n in 1..=6 {
        let remainder = evaluate(root)?.0[n];
        root.0[n] = neg(remainder)?.div_signed(derivative)?;
    }
    Ok(root)
}

pub(super) fn stock_chart(
    spec: Spec,
    flank: Flank,
    domain: ChartDomain,
    root: [f64; 2],
    t: [f64; 2],
    axis: usize,
) -> Result<[[I; 3]; 3]> {
    let beta = T::variable(iv(domain.beta)?, axis == 1);
    let source = super::expression::prepare(spec, flank)?;
    let root = if axis == 0 {
        T::c(iv(root)?)
    } else {
        let derivative =
            super::chart::radial_gradient(spec, flank, ChartDomain { u: root, ..domain })?[0];
        implicit_coefficients(iv(root)?, derivative, |u| {
            super::expression::radial_prepared(&source, u, beta)
        })?
    };
    let crest = T::c(I::point(if flank == Flank::Descending { 0. } else { 1. }));
    let local = T::variable(iv(t)?, axis == 0);
    let mut u = crest.add(root.sub(crest)?.mul(local)?)?;
    // The actual affine interpolation between crest and stock root is known
    // to stay within the original U domain, including endpoint roundoff.
    u.0[0] = I::new(u.0[0].lo.max(0.), u.0[0].hi.min(1.))?;
    let p = super::expression::evaluate(spec, flank, u, beta)?.point;
    Ok([p.map(|v| v.0[0]), p.map(|v| v.0[1]), p.map(|v| v.0[6])])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn implicit_square_root_coefficients_follow_original_equation_through_six() {
        let beta = T::variable(I::point(0.), true);
        let root = implicit_coefficients(I::point(1.), I::point(2.), |u| {
            u.mul(u)?.sub(T::c(I::point(1.)))?.sub(beta.mul(beta)?)
        })
        .unwrap();
        for (actual, expected) in root.0.iter().zip([1., 0., 0.5, 0., -0.125, 0., 0.0625]) {
            assert!(actual.lo <= expected && expected <= actual.hi);
        }
        assert!(implicit_coefficients(I::point(1.), I::new(-1., 1.).unwrap(), |u| Ok(u)).is_err());
    }
    #[test]
    fn polynomial_reciprocal_and_trig_coefficients_match_independent_series() {
        let x = T::variable(I::point(0.), true);
        let one = T::c(I::point(1.));
        let reciprocal = one.div(one.sub(x).unwrap()).unwrap();
        for c in reciprocal.0 {
            assert!(c.lo <= 1. && 1. <= c.hi);
        }
        let (s, c) = x.sin_cos().unwrap();
        for (a, v) in s.0.iter().zip([0., 1., 0., -1. / 6., 0., 1. / 120., 0.]) {
            assert!(a.lo <= v && v <= a.hi);
        }
        for (a, v) in c.0.iter().zip([1., 0., -0.5, 0., 1. / 24., 0., -1. / 720.]) {
            assert!(a.lo <= v && v <= a.hi);
        }
    }
}

pub(super) fn root_sixth(
    spec: Spec,
    part: RootPart,
    domain: ChartDomain,
    axis: usize,
) -> Result<[I; 3]> {
    Ok(super::expression::root(
        spec,
        part,
        T::variable(iv(domain.u)?, axis == 0),
        T::variable(iv(domain.beta)?, axis == 1),
    )?
    .point
    .map(|t| t.0[6]))
}

pub(super) fn axial_root_sixth(
    spec: Spec,
    part: RootPart,
    domain: AxialDomain,
    axis: usize,
) -> Result<[I; 3]> {
    Ok(super::expression::axial_root(
        spec,
        part,
        T::variable(iv(domain.t)?, axis == 0),
        T::variable(iv(domain.y)?, axis == 1),
    )?
    .point
    .map(|t| t.0[6]))
}

#[cfg(test)]
mod inverse_series_tests {
    use super::*;
    #[test]
    fn sqrt_and_asin_coefficients_match_independent_binomial_series_through_six() {
        let x = T::variable(I::point(0.), true);
        let square_root = T::c(I::point(1.)).add(x).unwrap().sqrt().unwrap();
        for (actual, expected) in
            square_root
                .0
                .iter()
                .zip([1., 0.5, -0.125, 0.0625, -5. / 128., 7. / 256., -21. / 1024.])
        {
            assert!(actual.lo <= expected && expected <= actual.hi);
        }
        let inverse = x.asin().unwrap();
        for (actual, expected) in inverse
            .0
            .iter()
            .zip([0., 1., 0., 1. / 6., 0., 3. / 40., 0.])
        {
            assert!(actual.lo <= expected && expected <= actual.hi);
        }
        let x = T::variable(I::point(0.2), true);
        let recovered = x.asin().unwrap().sin_cos().unwrap().0;
        for (actual, expected) in recovered.0.iter().zip([0.2, 1., 0., 0., 0., 0., 0.]) {
            assert!(actual.lo <= expected && expected <= actual.hi);
        }
        assert!(T::c(I::point(0.)).sqrt().is_err());
        assert!(T::c(I::point(1.)).asin().is_err());
    }
}

pub(super) fn axial_stock_sixth(
    spec: Spec,
    flank: Flank,
    d: AxialDomain,
    root: [f64; 2],
    derivative: I,
    axis: usize,
) -> Result<[I; 3]> {
    let source = super::expression::prepare(spec, flank)?;
    let y = T::variable(iv(d.y)?, axis == 1);
    let root = if axis == 0 {
        T::c(iv(root)?)
    } else {
        implicit_coefficients(iv(root)?, derivative, |u| {
            super::expression::axial_radial(&source, u, y)
        })?
    };
    let t = T::variable(iv(d.t)?, axis == 0);
    let crest = T::c(I::point(if flank == Flank::Descending { 0. } else { 1. }));
    let mut u = crest.add(root.sub(crest)?.mul(t)?)?;
    u.0[0] = u.0[0].intersect(0., 1.)?;
    Ok(super::expression::axial_flank(spec, flank, u, y)?
        .point
        .map(|x| x.0[6]))
}

pub(super) fn axial_tip_sixth(
    spec: Spec,
    radius: f64,
    d: AxialDomain,
    roots: [[f64; 2]; 2],
    derivatives: [I; 2],
    point_bounds: [[[f64; 2]; 3]; 2],
    axis: usize,
) -> Result<[I; 3]> {
    let y = T::variable(iv(d.y)?, axis == 1);
    let t = T::variable(iv(d.t)?, axis == 0);
    let mut points = [[T::c(I::point(0.)); 3]; 2];
    for (i, flank) in [Flank::Descending, Flank::Ascending]
        .into_iter()
        .enumerate()
    {
        let source = super::expression::prepare(spec, flank)?;
        let root = if axis == 0 {
            T::c(iv(roots[i])?)
        } else {
            implicit_coefficients(iv(roots[i])?, derivatives[i], |u| {
                super::expression::axial_radial(&source, u, y)
            })?
        };
        points[i] = if axis == 0 {
            let mut bound = [I::point(0.); 3];
            for k in 0..3 {
                bound[k] = iv(point_bounds[i][k])?;
            }
            bound.map(T::c)
        } else {
            let mut p = super::expression::axial_flank(spec, flank, root, y)?.point;
            // Whole-source mean-value boxes refine only coefficient zero;
            // all original implicit derivative coefficients remain enclosed.
            for k in 0..3 {
                p[k].0[0] = p[k].0[0].intersect(point_bounds[i][k][0], point_bounds[i][k][1])?;
            }
            p
        };
    }
    Ok(super::expression::tip_blend(radius, t, y, points[0], points[1])?.map(|p| p.0[6]))
}

#[cfg(test)]
mod constrained_circle_series_tests {
    use super::*;
    #[test]
    fn original_circle_constraint_preserves_independent_sixth_coefficients() {
        // A=(1,0), B=(1/2,-sqrt(3)/2) are EXACT original unit-circle ends.
        // |(1-t)A+tB|^2=1-t+t^2; its inverse square root has the
        // independent Legendre coefficients P_n(1/2).
        let sin60 = super::super::expression::sqrt_positive(I::point(0.75)).unwrap();
        let a = [T::c(I::point(1.)), T::c(I::point(0.)), T::c(I::point(0.))];
        let b = [
            T::c(I::point(0.5)),
            T::c(I::point(0.)),
            T::c(neg(sin60).unwrap()),
        ];
        let p = super::super::expression::tip_blend(
            1.,
            T::variable(I::point(0.), true),
            T::c(I::point(0.)),
            a,
            b,
        )
        .unwrap();
        assert!(p[0].0[6].lo <= 285. / 1024. && 285. / 1024. <= p[0].0[6].hi);
        let z = neg(sin60).unwrap().mul(I::point(23. / 256.)).unwrap();
        assert!(p[2].0[6].lo <= z.hi && z.lo <= p[2].0[6].hi);
    }
}
