use super::*;
type P = Vec<I>;
fn add(a: &[I], b: &[I], sign: f64) -> Result<P> {
    check(a.len() == b.len(), "Join polynomial degrees differ")?;
    a.iter()
        .zip(b)
        .map(|(&x, &y)| x.add(y.mul(I::point(sign))?))
        .collect()
}
fn scale(a: &[I], s: I) -> Result<P> {
    a.iter().map(|&x| x.mul(s)).collect()
}
fn mul(a: &[I], b: &[I]) -> Result<P> {
    crate::curve_surface_composition::mul(a, b)
}
fn diff(a: &[I]) -> Result<P> {
    a.windows(2)
        .map(|w| w[1].sub(w[0])?.mul(I::point((a.len() - 1) as f64)))
        .collect()
}
fn evaluate(a: &[I], t: I) -> Result<I> {
    let lo = a.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min);
    let hi = a.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max);
    let mut c = a.to_vec();
    let complement = I::point(1.).sub(t)?.intersect(0., 1.)?;
    for n in (1..c.len()).rev() {
        for j in 0..n {
            c[j] = c[j].mul(complement)?.add(c[j + 1].mul(t)?)?;
        }
    }
    c[0].intersect(lo, hi)
}
pub(super) struct Prepared {
    weight: P,
    du: [P; 3],
    duu: [P; 3],
    duv: [P; 3],
    pub left: usize,
    pub right: usize,
    pub v: [f64; 2],
}
impl Prepared {
    pub fn new(s: &Surface, knot: f64) -> Result<Self> {
        let (p, q, n, m) = (
            s.degree_u,
            s.degree_v,
            s.control_points.len(),
            s.control_points[0].len(),
        );
        check(
            (2..=16).contains(&p) && (1..=16).contains(&q) && !s.periodic_u && !s.periodic_v,
            "Join requires nonperiodic original degrees 2..16 by 1..16",
        )?;
        check(
            m == q + 1
                && s.knots_v[..=q].iter().all(|&v| v == s.knots_v[q])
                && s.knots_v[q + 1..].iter().all(|&v| v == s.knots_v[q + 1]),
            "Join needs one original V Bernstein span",
        )?;
        let spans: Vec<_> = (p..n)
            .filter(|&i| s.knots_u[i] < s.knots_u[i + 1])
            .collect();
        check(
            s.knots_u[..=p].iter().all(|&u| u == s.knots_u[p])
                && s.knots_u[n..].iter().all(|&u| u == s.knots_u[n])
                && spans
                    .iter()
                    .skip(1)
                    .all(|&i| s.knots_u.iter().filter(|&&u| u == s.knots_u[i]).count() == p),
            "Join needs original C0 Bernstein U spans",
        )?;
        let right = *spans
            .iter()
            .find(|&&i| s.knots_u[i] == knot && knot > s.knots_u[p])
            .ok_or_else(|| crate::input("Join knot has no original right span"))?;
        let left = *spans
            .iter()
            .find(|&&i| s.knots_u[i + 1] == knot)
            .ok_or_else(|| crate::input("Join knot has no original left span"))?;
        check(
            left == right - p,
            "Join has no literal common original column",
        )?;
        let origin = &s.control_points[left][0];
        let row = |index: usize| -> Result<[P; 4]> {
            let mut h: [P; 4] = std::array::from_fn(|_| Vec::new());
            for j in 0..=q {
                let w = I::point(s.weights[index][j]);
                for k in 0..3 {
                    h[k].push(
                        I::point(s.control_points[index][j][k])
                            .sub(I::point(origin[k]))?
                            .mul(w)?,
                    );
                }
                h[3].push(w);
            }
            Ok(h)
        };
        let h0 = row(left)?;
        let lm = row(left - 1)?;
        let lmm = row(left - 2)?;
        let rp = row(left + 1)?;
        let rpp = row(left + 2)?;
        let dl = I::point(s.knots_u[left + 1]).sub(I::point(s.knots_u[left]))?;
        let dr = I::point(s.knots_u[right + 1]).sub(I::point(s.knots_u[right]))?;
        let mut ul: [P; 4] = std::array::from_fn(|_| Vec::new());
        let mut ur = ul.clone();
        let mut uul = ul.clone();
        let mut uur = ul.clone();
        for k in 0..4 {
            ul[k] = scale(&add(&h0[k], &lm[k], -1.)?, I::point(p as f64).div(dl)?)?;
            ur[k] = scale(&add(&rp[k], &h0[k], -1.)?, I::point(p as f64).div(dr)?)?;
            let a = add(
                &add(&h0[k], &scale(&lm[k], I::point(2.))?, -1.)?,
                &lmm[k],
                1.,
            )?;
            let b = add(
                &add(&rpp[k], &scale(&rp[k], I::point(2.))?, -1.)?,
                &h0[k],
                1.,
            )?;
            uul[k] = scale(&a, I::point((p * (p - 1)) as f64).div(dl.mul(dl)?)?)?;
            uur[k] = scale(&b, I::point((p * (p - 1)) as f64).div(dr.mul(dr)?)?)?;
        }
        let w = &h0[3];
        let w2 = mul(w, w)?;
        let wu = add(&ur[3], &ul[3], -1.)?;
        let wuu = add(&uur[3], &uul[3], -1.)?;
        let wudu = add(&mul(&ur[3], &ur[3])?, &mul(&ul[3], &ul[3])?, -1.)?;
        let wd = diff(w)?;
        let mut du: [P; 3] = std::array::from_fn(|_| Vec::new());
        let mut duu = du.clone();
        let mut duv = du.clone();
        for k in 0..3 {
            du[k] = add(
                &mul(&add(&ur[k], &ul[k], -1.)?, w)?,
                &mul(&h0[k], &wu)?,
                -1.,
            )?;
            duv[k] = add(
                &mul(&diff(&du[k])?, w)?,
                &scale(&mul(&du[k], &wd)?, I::point(2.))?,
                -1.,
            )?;
            let product = add(&mul(&ur[k], &ur[3])?, &mul(&ul[k], &ul[3])?, -1.)?;
            let a = add(
                &mul(&add(&uur[k], &uul[k], -1.)?, &w2)?,
                &mul(&mul(&h0[k], &wuu)?, w)?,
                -1.,
            )?;
            let b = add(
                &scale(&mul(&product, w)?, I::point(-2.))?,
                &scale(&mul(&h0[k], &wudu)?, I::point(2.))?,
                1.,
            )?;
            duu[k] = add(&a, &b, 1.)?;
        }
        Ok(Self {
            weight: w.clone(),
            du,
            duu,
            duv,
            left,
            right,
            v: [s.knots_v[q], s.knots_v[q + 1]],
        })
    }
    pub fn jumps(&self, query: [f64; 2]) -> Result<[[I; 3]; 3]> {
        let dv = I::point(self.v[1]).sub(I::point(self.v[0]))?;
        let t = I::new(query[0], query[1])?
            .sub(I::point(self.v[0]))?
            .div(dv)?
            .intersect(0., 1.)?;
        let w = evaluate(&self.weight, t)?;
        let w2 = w.mul(w)?;
        let w3 = w2.mul(w)?;
        let mut out = [[I::point(0.); 3]; 3];
        for k in 0..3 {
            out[0][k] = evaluate(&self.du[k], t)?.div(w2)?;
            out[1][k] = evaluate(&self.duu[k], t)?.div(w3)?;
            out[2][k] = evaluate(&self.duv[k], t)?.div(w3)?.div(dv)?;
        }
        Ok(out)
    }
}
