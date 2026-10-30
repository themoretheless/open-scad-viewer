use super::*;
pub(super) type Poly = Vec<I>;
pub(super) type Homogeneous = [Poly; 4];
pub(super) fn diff(a: &[I]) -> Result<Poly> {
    if a.len() == 1 {
        return Ok(vec![I::point(0.)]);
    }
    a.windows(2)
        .map(|w| w[1].sub(w[0])?.mul(I::point((a.len() - 1) as f64)))
        .collect()
}
pub(super) fn eval(a: &[I], t: I) -> Result<I> {
    let lo = a.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min);
    let hi = a.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max);
    let mut c = a.to_vec();
    let complement = I::point(1.).sub(t)?.intersect(0., 1.)?;
    for n in (1..c.len()).rev() {
        for i in 0..n {
            c[i] = c[i].mul(complement)?.add(c[i + 1].mul(t)?)?;
        }
    }
    c[0].intersect(lo, hi)
}
pub(super) fn mul(a: &[I], b: &[I]) -> Result<Poly> {
    crate::curve_surface_composition::mul(a, b)
}
fn add(a: &[I], b: &[I], sign: f64) -> Result<Poly> {
    check(a.len() == b.len(), "Profile polynomial degrees differ")?;
    a.iter()
        .zip(b)
        .map(|(&a, &b)| a.add(b.mul(I::point(sign))?))
        .collect()
}
pub(super) fn elevate(mut h: Homogeneous, degree: usize) -> Result<Homogeneous> {
    while h[0].len() < degree + 1 {
        for c in &mut h {
            let old = c.clone();
            let n = old.len();
            let mut out = vec![I::point(0.); n + 1];
            out[0] = old[0];
            out[n] = old[n - 1];
            for i in 1..n {
                let a = I::point(i as f64).div(I::point(n as f64))?;
                out[i] = old[i - 1].mul(a)?.add(old[i].mul(I::point(1.).sub(a)?)?)?;
            }
            *c = out;
        }
    }
    Ok(h)
}
pub(super) fn restrict(h: &Homogeneous, ends: [I; 2]) -> Result<Homogeneous> {
    let n = h[0].len() - 1;
    let mut out: Homogeneous = std::array::from_fn(|_| Vec::new());
    for k in 0..4 {
        for i in 0..=n {
            let mut c = h[k].clone();
            for r in 0..n {
                let t = ends[usize::from(r >= n - i)];
                let a = I::point(1.).sub(t)?.intersect(0., 1.)?;
                for j in 0..n - r {
                    c[j] = c[j].mul(a)?.add(c[j + 1].mul(t)?)?;
                }
            }
            out[k].push(c[0]);
        }
    }
    Ok(out)
}
pub(super) struct Pair {
    a: Homogeneous,
    b: Homogeneous,
    e: [Poly; 3],
    d: Poly,
}
impl Pair {
    pub fn new(a: Homogeneous, b: Homogeneous) -> Result<Self> {
        let mut e: [Poly; 3] = std::array::from_fn(|_| Vec::new());
        for k in 0..3 {
            e[k] = add(&mul(&b[k], &a[3])?, &mul(&a[k], &b[3])?, -1.)?;
        }
        let d = mul(&a[3], &b[3])?;
        Ok(Self { a, b, e, d })
    }
    pub fn bounds(&self, t: [f64; 2], axis: usize) -> Result<Option<[[f64; 2]; 2]>> {
        let t = I::new(t[0], t[1])?;
        let jets = |h: &Homogeneous| -> Result<jets::Jets> {
            let net = (0..h[0].len())
                .map(|i| vec![std::array::from_fn(|k| h[k][i]); 2])
                .collect::<Vec<_>>();
            let mut j = jets::evaluate_homogeneous_jets(&net, [t, I::point(0.)], 2)?;
            j[0][1] = [I::point(0.); 3];
            j[0][1][axis] = I::point(1.);
            j[1][1] = [I::point(0.); 3];
            j[0][2] = [I::point(0.); 3];
            Ok(j)
        };
        let a = jets(&self.a)?;
        let b = jets(&self.b)?;
        let d = eval(&self.d, t)?;
        if d.lo <= 0. {
            return Ok(None);
        }
        let d1 = eval(&diff(&self.d)?, t)?;
        let d2 = eval(&diff(&diff(&self.d)?)?, t)?;
        let d_sq = d.mul(d)?;
        let d_cb = d_sq.mul(d)?;
        let mut delta = [[I::point(0.); 3]; 3];
        for k in 0..3 {
            if k == axis {
                continue;
            }
            let e = eval(&self.e[k], t)?;
            let e1 = eval(&diff(&self.e[k])?, t)?;
            let e2 = eval(&diff(&diff(&self.e[k])?)?, t)?;
            delta[0][k] = e1.div(d)?.sub(e.mul(d1)?.div(d_sq)?)?;
            delta[1][k] = e2
                .div(d)?
                .sub(e.mul(d2)?.div(d_sq)?)?
                .sub(I::point(2.).mul(e1)?.mul(d1)?.div(d_sq)?)?
                .add(I::point(2.).mul(e)?.mul(d1)?.mul(d1)?.div(d_cb)?)?;
        }
        super::super::paired::bounds(&a, &b, delta)
    }
}
