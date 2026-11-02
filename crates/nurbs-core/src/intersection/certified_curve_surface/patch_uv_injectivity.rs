//! Exact global univalence of positive rational bilinear rectangle coordinates.
use super::affine_plane::q;
use super::*;
use num_rational::BigRational as Q;
use num_traits::Zero;
#[derive(Clone, Debug)]
pub struct BilinearUvInjectivity {
    pub proven: bool,
    /// Positive Bernstein coefficients of Q in det D(F,G)=Q/W^3.
    pub determinant_bernstein: [Q; 4],
    pub determinant_factorization_verified: bool,
}
type P = [[Q; 3]; 3];
fn zero() -> P {
    std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()))
}
fn bilinear(c: [Q; 4]) -> P {
    let mut p = zero();
    p[0][0] = c[0].clone();
    p[1][0] = &c[1] - &c[0];
    p[0][1] = &c[2] - &c[0];
    p[1][1] = &c[3] - &c[1] - &c[2] + &c[0];
    p
}
fn multiply(a: &P, b: &P) -> Result<P> {
    let mut p = zero();
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                for l in 0..3 {
                    if a[i][j].is_zero() || b[k][l].is_zero() {
                        continue;
                    }
                    check(
                        i + k < 3 && j + l < 3,
                        "Unexpected bilinear determinant degree",
                    )?;
                    p[i + k][j + l] += &a[i][j] * &b[k][l];
                }
            }
        }
    }
    Ok(p)
}
/// With A=w00,B=w10,C=w01,D=w11, F_u=LR/W^2>0 and
/// det D(F,G)=Q/W^3>0. Every fixed F=alpha has one u(v), from
/// F(0,v)=0,F(1,v)=1. Along that level, dG/dv=det/F_u>0;
/// G runs from 0 to 1. Thus every interior affine point has exactly
/// one interior UV preimage, including nonseparable weights. Boundary
/// edges have positive projective clocks and distinct corner images.
pub(super) fn certify(w: &[Vec<f64>]) -> Result<BilinearUvInjectivity> {
    check(
        w.len() == 2
            && w.iter()
                .all(|r| r.len() == 2 && r.iter().all(|x| x.is_finite() && *x > 0.)),
        "Bilinear UV proof needs positive finite 2x2 weights",
    )?;
    let (a, b, c, d) = (q(w[0][0]), q(w[1][0]), q(w[0][1]), q(w[1][1]));
    let l = bilinear([a.clone(), a.clone(), c.clone(), c.clone()]);
    let r = bilinear([b.clone(), b.clone(), d.clone(), d.clone()]);
    let m = bilinear([a.clone(), b.clone(), a.clone(), b.clone()]);
    let n = bilinear([c.clone(), d.clone(), c.clone(), d.clone()]);
    let denominator = bilinear([a.clone(), b.clone(), c.clone(), d.clone()]);
    let coefficients = [&a * &b * &c, &a * &b * &d, &a * &c * &d, &b * &c * &d];
    let expected = multiply(&denominator, &bilinear(coefficients.clone()))?;
    let mut actual = multiply(&multiply(&l, &r)?, &multiply(&m, &n)?)?;
    let delta = &a * &d - &b * &c;
    let square = &delta * &delta;
    actual[1][1] -= &square;
    actual[2][1] += &square;
    actual[1][2] += &square;
    actual[2][2] -= &square;
    check(
        actual == expected && coefficients.iter().all(|x| *x > Q::zero()),
        "Original bilinear Jacobian factorization failed",
    )?;
    Ok(BilinearUvInjectivity {
        proven: true,
        determinant_bernstein: coefficients,
        determinant_factorization_verified: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_factorization_covers_asymmetric_and_extreme_positive_weights() {
        for w in [
            vec![vec![1., 1.], vec![1., 2.]],
            vec![vec![0.1, 0.3], vec![1.1, 2.7]],
            vec![vec![f64::from_bits(1), 1.], vec![1., f64::MAX]],
        ] {
            let p = certify(&w).unwrap();
            assert!(p.proven && p.determinant_factorization_verified);
            assert!(p.determinant_bernstein.iter().all(|x| *x > Q::zero()));
        }
    }
    #[test]
    fn invalid_weights_never_form_an_injectivity_proof() {
        for x in [0., -1., f64::NAN] {
            assert!(certify(&[vec![1., 1.], vec![1., x]]).is_err());
        }
    }
}
