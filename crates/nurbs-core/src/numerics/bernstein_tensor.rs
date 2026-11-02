//! Shared outward tensor-Bernstein polynomial arithmetic. No sampled geometry
//! or public proof callback. Product degrees are bounded before integer choose.
use crate::{Result, check, distance_bounds::Interval as I};
pub(crate) type Net = Vec<Vec<I>>;
fn choose(n: usize, k: usize) -> u64 {
    let mut c = 1u64;
    for j in 0..k.min(n - k) {
        c = c * (n - j) as u64 / (j + 1) as u64;
    }
    c
}
pub(crate) fn derivative(a: &Net, axis: usize) -> Result<Net> {
    let p = if axis == 0 {
        a.len() - 1
    } else {
        a[0].len() - 1
    };
    if p == 0 {
        return Ok(vec![vec![I::point(0.); a[0].len()]; a.len()]);
    }
    let mut out = vec![
        vec![I::point(0.); a[0].len() - usize::from(axis == 1)];
        a.len() - usize::from(axis == 0)
    ];
    for u in 0..out.len() {
        for v in 0..out[0].len() {
            out[u][v] = a[u + usize::from(axis == 0)][v + usize::from(axis == 1)]
                .sub(a[u][v])?
                .mul(I::point(p as f64))?;
        }
    }
    Ok(out)
}
pub(crate) fn product(a: &Net, b: &Net) -> Result<Net> {
    let [m, n] = [a.len() - 1, a[0].len() - 1];
    let [p, q] = [b.len() - 1, b[0].len() - 1];
    check(
        m + p <= 15 && n + q <= 15,
        "Tensor Bernstein product degree exceeds finite kernel ceiling",
    )?;
    let mut out = vec![vec![I::point(0.); n + q + 1]; m + p + 1];
    for k in 0..=m + p {
        for l in 0..=n + q {
            for i in k.saturating_sub(p)..=k.min(m) {
                for j in l.saturating_sub(q)..=l.min(n) {
                    let u = I::point(choose(m, i) as f64)
                        .mul(I::point(choose(p, k - i) as f64))?
                        .div(I::point(choose(m + p, k) as f64))?;
                    let v = I::point(choose(n, j) as f64)
                        .mul(I::point(choose(q, l - j) as f64))?
                        .div(I::point(choose(n + q, l) as f64))?;
                    out[k][l] = out[k][l].add(a[i][j].mul(b[k - i][l - j])?.mul(u)?.mul(v)?)?;
                }
            }
        }
    }
    Ok(out)
}
pub(crate) fn minus(a: Net, b: Net) -> Result<Net> {
    check(
        (a.len(), a[0].len()) == (b.len(), b[0].len()),
        "Tensor Bernstein subtraction requires equal degrees",
    )?;
    a.into_iter()
        .zip(b)
        .map(|(a, b)| a.into_iter().zip(b).map(|(a, b)| a.sub(b)).collect())
        .collect()
}

// Restrict the ORIGINAL polynomial by outward subdivision, then use its
// Bernstein convex hull. The split ratio is a proposed exact real ratio
// enclosed outward; these coefficients never replace source control points.
fn split_line(mut a: Vec<I>, t: I) -> Result<(Vec<I>, Vec<I>)> {
    let n=a.len(); let mut left=vec![a[0]]; let mut right=vec![a[n-1]];
    for count in (1..n).rev() {
        let beta=I::point(1.).sub(t)?.intersect(0.,1.)?;
        for i in 0..count {
            let (x,y)=(a[i],a[i+1]);
            let direct=x.mul(beta)?.add(y.mul(t)?)?;
            let left=x.add(t.mul(y.sub(x)?)?)?;
            let right=y.add(beta.mul(x.sub(y)?)?)?;
            a[i]=direct.intersect(left.lo,left.hi)?.intersect(right.lo,right.hi)?;
        }
        left.push(a[0]);right.push(a[count-1]);
    }
    right.reverse();Ok((left,right))
}
fn restricted_hull(a: &[I], t: I) -> Result<I> {
    check(t.lo>=0. && t.hi<=1., "Bernstein restriction outside unit domain")?;
    if t.lo==1. {return Ok(a[a.len()-1]);}
    let right=if t.lo==0. {a.to_vec()} else {split_line(a.to_vec(),I::point(t.lo))?.1};
    let restricted=if t.hi==1. {right} else {
        let beta=I::point(t.hi).sub(I::point(t.lo))?
            .div(I::point(1.).sub(I::point(t.lo))?)?.intersect(0.,1.)?;
        split_line(right,beta)?.0
    };
    I::new(restricted.iter().map(|x|x.lo).fold(f64::INFINITY,f64::min),
        restricted.iter().map(|x|x.hi).fold(f64::NEG_INFINITY,f64::max))
}

pub(crate) fn evaluate_line(mut a: Vec<I>, t: I) -> Result<I> {
    let hull = I::new(
        a.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min),
        a.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max),
    )?;
    let restricted=restricted_hull(&a,t)?;
    let beta = I::point(1.).sub(t)?.intersect(0., 1.)?;
    for n in (1..a.len()).rev() {
        for i in 0..n {
            let x = a[i];
            let y = a[i + 1];
            let direct = x.mul(beta)?.add(y.mul(t)?)?;
            let left = x.add(y.sub(x)?.mul(t)?)?;
            let right = y.add(x.sub(y)?.mul(beta)?)?;
            a[i] = I::new(
                direct.lo.max(left.lo).max(right.lo),
                direct.hi.min(left.hi).min(right.hi),
            )?;
        }
    }
    a[0].intersect(hull.lo, hull.hi)?.intersect(restricted.lo,restricted.hi)
}
pub(crate) fn evaluate(a: &Net, t: [I; 2]) -> Result<I> {
    let rows = a
        .iter()
        .map(|r| evaluate_line(r.clone(), t[1]))
        .collect::<Result<Vec<_>>>()?;
    evaluate_line(rows, t[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_subinterval_restriction_retains_linear_range_and_original_derivative_units() {
        let net=vec![vec![I::point(5.)],vec![I::point(4.)],vec![I::point(3.)],vec![I::point(2.)],vec![I::point(1.)]];
        let t=[I::new(0.375,0.5625).unwrap(),I::new(0.,1.).unwrap()];
        let value=evaluate(&net,t).unwrap();
        assert!(value.lo<=2.75 && value.hi>=3.5);
        assert!(value.lo>2.75-1e-12 && value.hi<3.5+1e-12,"{value:?}");
        let derivative=evaluate(&derivative(&net,0).unwrap(),t).unwrap();
        assert!(derivative.lo<=-4. && derivative.hi>=-4.);
        assert!(derivative.lo> -4.-1e-12 && derivative.hi< -4.+1e-12);
        for x in [0.,0.375,0.5625,1.] {
            let value=evaluate_line(net.iter().map(|r|r[0]).collect(),I::point(x)).unwrap();
            assert!(value.lo<=5.-4.*x && value.hi>=5.-4.*x);
        }
    }
    #[test]
    fn tensor_restriction_encloses_full_product_range_not_only_corners() {
        // (u-1/2)^2+(v-1/2)^2 has an interior minimum that corners miss.
        let a=[0.25,-0.25,0.25];
        let net=a.map(|x|a.map(|y|I::point(x+y)).to_vec()).to_vec();
        let value=evaluate(&net,[I::new(0.25,0.75).unwrap();2]).unwrap();
        assert!(value.lo<=0. && value.hi>=0.125);
        assert!(value.lo>= -0.125-1e-12 && value.hi<=0.125+1e-12,"{value:?}");
    }
}
