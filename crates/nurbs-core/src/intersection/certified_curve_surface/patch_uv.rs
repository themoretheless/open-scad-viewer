//! Candidate-guided 2D Krawczyk inversion of original rational bilinear clocks.
use super::*;
type Net = [[[I; 2]; 2]; 2];
fn eval(n: &Net, u: I, v: I) -> Result<[I; 2]> {
    let cu = I::point(1.).sub(u)?.intersect(0., 1.)?;
    let cv = I::point(1.).sub(v)?.intersect(0., 1.)?;
    let mut f = [I::point(0.); 2];
    for k in 0..2 {
        let a = n[0][0][k].mul(cu)?.add(n[1][0][k].mul(u)?)?;
        let b = n[0][1][k].mul(cu)?.add(n[1][1][k].mul(u)?)?;
        f[k] = a.mul(cv)?.add(b.mul(v)?)?;
    }
    Ok(f)
}
fn restrict(n: &Net, d: [[f64; 2]; 2]) -> Result<Net> {
    let mut out = [[[I::point(0.); 2]; 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            out[i][j] = eval(n, I::point(d[0][i]), I::point(d[1][j]))?;
        }
    }
    Ok(out)
}
fn include(n: &Net) -> Result<Option<[I; 2]>> {
    let hull = |a: I, b: I| I {
        lo: a.lo.min(b.lo),
        hi: a.hi.max(b.hi),
    };
    let mut jac = [[I::point(0.); 2]; 2];
    for k in 0..2 {
        jac[k][0] = hull(n[1][0][k].sub(n[0][0][k])?, n[1][1][k].sub(n[0][1][k])?);
        jac[k][1] = hull(n[0][1][k].sub(n[0][0][k])?, n[1][1][k].sub(n[1][0][k])?);
    }
    let m = jac.map(|r| r.map(|x| x.lo * 0.5 + x.hi * 0.5));
    let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    if !det.is_finite() || det == 0. {
        return Ok(None);
    }
    let r = [
        [m[1][1] / det, -m[0][1] / det],
        [-m[1][0] / det, m[0][0] / det],
    ];
    if !r.iter().flatten().all(|x| x.is_finite()) {
        return Ok(None);
    }
    let rd = I::point(r[0][0])
        .mul(I::point(r[1][1]))?
        .sub(I::point(r[0][1]).mul(I::point(r[1][0]))?)?;
    if rd.lo <= 0. && rd.hi >= 0. {
        return Ok(None);
    }
    let center = eval(n, I::point(0.5), I::point(0.5))?;
    let mut image = [I::point(0.5); 2];
    let mut norm = 0f64;
    for i in 0..2 {
        for j in 0..2 {
            image[i] = image[i].sub(I::point(r[i][j]).mul(center[j])?)?;
        }
        let mut row = I::point(0.);
        for k in 0..2 {
            let mut a = I::point(if i == k { 1. } else { 0. });
            for j in 0..2 {
                a = a.sub(I::point(r[i][j]).mul(jac[j][k])?)?;
            }
            row = row.add(I::point(a.lo.abs().max(a.hi.abs())))?;
            image[i] = image[i].add(a.mul(I::new(-0.5, 0.5)?)?)?;
        }
        norm = norm.max(row.hi);
    }
    Ok((norm < 0.5 && image.iter().all(|x| x.lo > 0. && x.hi < 1.)).then_some(image))
}
// Rounded Newton values only propose search boxes; they authorize no contact.
fn proposal(n: &Net) -> Result<[f64; 2]> {
    let midpoint = |x: I| x.lo * 0.5 + x.hi * 0.5;
    let mut p = [0.5; 2];
    for _ in 0..16 {
        let f = eval(n, I::point(p[0]), I::point(p[1]))?.map(midpoint);
        let mut j = [[0.; 2]; 2];
        for k in 0..2 {
            j[k][0] = midpoint(
                n[1][0][k]
                    .sub(n[0][0][k])?
                    .mul(I::point(1. - p[1]))?
                    .add(n[1][1][k].sub(n[0][1][k])?.mul(I::point(p[1]))?)?,
            );
            j[k][1] = midpoint(
                n[0][1][k]
                    .sub(n[0][0][k])?
                    .mul(I::point(1. - p[0]))?
                    .add(n[1][1][k].sub(n[1][0][k])?.mul(I::point(p[0]))?)?,
            );
        }
        let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
        if !det.is_finite() || det == 0. {
            break;
        }
        let next = [
            p[0] - (j[1][1] * f[0] - j[0][1] * f[1]) / det,
            p[1] - (-j[1][0] * f[0] + j[0][0] * f[1]) / det,
        ];
        if !next.iter().all(|x| x.is_finite()) {
            break;
        }
        let next = next.map(|x| x.clamp(1e-12, 1. - 1e-12));
        if next == p {
            break;
        }
        p = next;
    }
    Ok(p)
}
fn mapped(d: [[f64; 2]; 2], image: [I; 2]) -> Result<[[f64; 2]; 2]> {
    let mut out = d;
    for i in 0..2 {
        let b = I::point(d[i][0])
            .add(I::point(d[i][1]).sub(I::point(d[i][0]))?.mul(image[i])?)?
            .intersect(d[i][0], d[i][1])?;
        out[i] = [b.lo, b.hi];
    }
    Ok(out)
}
/// Existence and local uniqueness for every actual affine point enclosed by a,b.
/// Caller separately proves global target injectivity. At most 24 candidate
/// boxes and 8 original-coefficient refinements are tried; failure is explicit.
pub(super) fn inverse(w: &[Vec<f64>], a: I, b: I) -> Result<Option<[[f64; 2]; 2]>> {
    let mut n = [[[I::point(0.); 2]; 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            n[i][j] = [
                I::point(i as f64).sub(a)?.mul(I::point(w[i][j]))?,
                I::point(j as f64).sub(b)?.mul(I::point(w[i][j]))?,
            ];
        }
    }
    let p = proposal(&n)?;
    for attempt in 0..24 {
        let d = if attempt == 0 {
            [[0., 1.]; 2]
        } else {
            let r = 0.5f64.powi(attempt);
            p.map(|x| [(x - r).max(0.), (x + r).min(1.)])
        };
        let Some(image) = include(&restrict(&n, d)?)? else {
            continue;
        };
        let mut root = mapped(d, image)?;
        for _ in 0..8 {
            if root.iter().any(|x| x[0] >= x[1]) {
                break;
            }
            let Some(image) = include(&restrict(&n, root)?)? else {
                break;
            };
            let next = mapped(root, image)?;
            if next == root {
                break;
            }
            root = next;
        }
        return Ok(Some(root));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_weight_inverse_encloses_the_analytic_coupled_root() {
        let r = inverse(&[vec![1., 1.], vec![1., 2.]], I::point(0.5), I::point(0.5))
            .unwrap()
            .unwrap();
        let t = 2f64.sqrt() - 1.;
        assert!(r.iter().all(|b| b[0] <= t && t <= b[1]));
    }
    #[test]
    fn a_newton_proposal_does_not_discard_original_point_uncertainty() {
        let r = inverse(
            &[vec![1., 1.], vec![1., 1.]],
            I::new(-1., 2.).unwrap(),
            I::new(-1., 2.).unwrap(),
        )
        .unwrap();
        assert!(r.is_none());
    }
}
