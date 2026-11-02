//! Correlated ORIGINAL rational carrier difference in a common Bernstein basis.
//! A finite positive gauge k is only a numerical proposal. The outward identity
//! D=C1-C0=[(N1-kN0)-C0(W1-kW0)]/W1 holds for ANY such k, so unequal rational
//! weights require no equality assumption or rounded replacement surface.
use super::*;
type H = [I; 5]; // numerator delta XYZ, weight delta, original end weight
pub(super) struct Difference<'s> {
    caps: [&'s Surface; 2],
    jets: [H; 3],
}
fn bezier(s: &Surface) -> bool {
    let n = s.control_points.len();
    let m = s.control_points[0].len();
    !s.periodic_u
        && !s.periodic_v
        && (1..=32).contains(&s.degree_u)
        && (1..=32).contains(&s.degree_v)
        && n == s.degree_u + 1
        && m == s.degree_v + 1
        && [(&s.knots_u, n), (&s.knots_v, m)]
            .into_iter()
            .all(|(k, n)| {
                k.len() == 2 * n
                    && k[0].is_finite()
                    && k[n].is_finite()
                    && k[0] < k[n]
                    && k[..n].iter().all(|&x| x == k[0])
                    && k[n..].iter().all(|&x| x == k[n])
            })
}
fn hull(values: impl Iterator<Item = H>) -> H {
    let mut out = [I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 5];
    for x in values {
        for k in 0..5 {
            out[k].lo = out[k].lo.min(x[k].lo);
            out[k].hi = out[k].hi.max(x[k].hi);
        }
    }
    out
}
fn derivative(a: H, b: H, degree: usize) -> Result<H> {
    let mut out = [I::point(0.); 5];
    for k in 0..5 {
        out[k] = b[k].sub(a[k])?.mul(I::point(degree as f64))?;
    }
    Ok(out)
}
impl<'s> Difference<'s> {
    pub(super) fn prepare(
        caps: [&'s Surface; 2],
        spent: &mut usize,
        max: usize,
    ) -> Result<Option<Self>> {
        let [a, b] = caps;
        if !bezier(a) || !bezier(b) || a.degree_u != b.degree_u || a.degree_v != b.degree_v {
            return Ok(None);
        }
        let n = a.control_points.len();
        let m = a.control_points[0].len();
        // Complete original control census plus both derivative control nets and
        // every original knot inspected for normalized-basis ownership.
        let charge = n * m
            + (n - 1) * m
            + n * (m - 1)
            + a.knots_u.len()
            + a.knots_v.len()
            + b.knots_u.len()
            + b.knots_v.len();
        if charge > max - *spent {
            return Ok(None);
        }
        *spent += charge;
        let k = b.weights[0][0] / a.weights[0][0];
        if !k.is_finite() || k <= 0. {
            return Ok(None);
        }
        let calculation = (|| -> Result<Self> {
            let mut net = vec![vec![[I::point(0.); 5]; m]; n];
            for i in 0..n {
                for j in 0..m {
                    let wa = I::point(a.weights[i][j]);
                    let wb = I::point(b.weights[i][j]);
                    let scaled = I::point(k).mul(wa)?;
                    for axis in 0..3 {
                        net[i][j][axis] = wb
                            .mul(I::point(b.control_points[i][j][axis]))?
                            .sub(scaled.mul(I::point(a.control_points[i][j][axis]))?)?;
                    }
                    net[i][j][3] = wb.sub(scaled)?;
                    net[i][j][4] = wb;
                }
            }
            let value = hull(net.iter().flatten().copied());
            let mut du = Vec::with_capacity((n - 1) * m);
            let mut dv = Vec::with_capacity(n * (m - 1));
            for i in 0..n {
                for j in 0..m {
                    if i + 1 < n {
                        du.push(derivative(net[i][j], net[i + 1][j], a.degree_u)?);
                    }
                    if j + 1 < m {
                        dv.push(derivative(net[i][j], net[i][j + 1], a.degree_v)?);
                    }
                }
            }
            Ok(Self {
                caps,
                jets: [value, hull(du.into_iter()), hull(dv.into_iter())],
            })
        })();
        match calculation {
            Ok(d) => Ok(Some(d)),
            Err(e) if e.code == "NURBS_NUMERIC_ERROR" => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub(super) fn enclose(
        &self,
        caps: [&Surface; 2],
        a: [V3; 3],
        spent: &mut usize,
        max: usize,
    ) -> Result<Option<[V3; 3]>> {
        if !(0..2).all(|i| std::ptr::eq(caps[i], self.caps[i])) || *spent == max {
            return Ok(None);
        }
        *spent += 1;
        let calculation = (|| -> Result<[V3; 3]> {
            let w = self.jets[0][4];
            let mut out = [[I::point(0.); 3]; 3];
            for k in 0..3 {
                out[0][k] = self.jets[0][k].sub(a[0][k].mul(self.jets[0][3])?)?.div(w)?;
                for axis in 1..3 {
                    let r = self.jets[axis][k]
                        .sub(a[axis][k].mul(self.jets[0][3])?)?
                        .sub(a[0][k].mul(self.jets[axis][3])?)?;
                    out[axis][k] = r.sub(out[0][k].mul(self.jets[axis][4])?)?.div(w)?;
                }
            }
            Ok(out)
        })();
        match calculation {
            Ok(d) => Ok(Some(d)),
            Err(e) if e.code == "NURBS_NUMERIC_ERROR" => Ok(None),
            Err(e) => Err(e),
        }
    }
}
#[cfg(test)]
mod tests;
