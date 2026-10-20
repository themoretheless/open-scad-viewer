//! Outward restriction of an original complete Bernstein span.
use crate::{Result, distance_bounds::Interval as I};
pub(super) fn is_complete_span(degree: usize, knots: &[f64], span: usize) -> bool {
    span >= degree && knots.get(span).zip(knots.get(span+1)).is_some_and(|(&a,&b)|
        a < b && knots[span-degree..=span].iter().all(|&x|x==a)
        && knots.get(span+1..span+degree+2).is_some_and(|k|k.iter().all(|&x|x==b)))
}
fn split(controls: &[[I;4]], alpha: I, weights: [f64;2]) -> Result<(Vec<[I;4]>,Vec<[I;4]>)> {
    let beta = I::point(1.).sub(alpha)?.intersect(0.,1.)?;
    let len = controls.len();
    let mut d = [[I::point(0.); 4]; 33];
    d[..len].copy_from_slice(controls);
    let mut left = Vec::with_capacity(len);
    let mut right = Vec::with_capacity(len);
    left.push(d[0]);
    right.push(d[len - 1]);
    for remaining in (1..len).rev() {
        for j in 0..remaining {
            let a=d[j];let b=d[j+1];
            for k in 0..4 { d[j][k]=super::mul_blend_factor(a[k],beta)?.add(super::mul_blend_factor(b[k],alpha)?)?
                .intersect(a[k].lo.min(b[k].lo),a[k].hi.max(b[k].hi))?; }
            d[j][3]=d[j][3].intersect(weights[0],weights[1])?;
        }
        left.push(d[0]);right.push(d[remaining-1]);
    }
    right.reverse();Ok((left,right))
}
pub(super) fn restrict(controls: &[[I;4]], span: [f64;2], domain: [f64;2], weights: [f64;2])
    -> Result<Vec<[I;4]>> {
    // Identity and clamped endpoints are original homogeneous coefficients;
    // no new arithmetic or rounded parameter is needed to enclose them.
    if domain==span { return Ok(controls.to_vec()); }
    if domain[0]==domain[1] && domain[0]==span[1] {
        return Ok(vec![*controls.last().unwrap()]);
    }
    if domain[1]==span[0] { return Ok(vec![controls[0]]); }
    let hi=I::point(domain[1]).sub(I::point(span[0]))?;
    let width=I::point(span[1]).sub(I::point(span[0]))?;
    // With the original upper endpoint, the first left subdivision is the
    // identity. Restrict the original coefficients directly on [lo, span.hi].
    // No rounded parameter or extra interval arithmetic is needed at alpha=1.
    if domain[1]==span[1] {
        let lo=I::point(domain[0]).sub(I::point(span[0]))?;
        return Ok(split(controls,lo.div(width)?.intersect(0.,1.)?,weights)?.1);
    }
    let (left,_)=split(controls,hi.div(width)?.intersect(0.,1.)?,weights)?;
    if domain[0]==domain[1] { return Ok(vec![*left.last().unwrap()]); }
    if domain[0]==span[0] { return Ok(left); }
    let lo=I::point(domain[0]).sub(I::point(span[0]))?;
    Ok(split(&left,lo.div(hi)?.intersect(0.,1.)?,weights)?.1)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_degree_thirteen_and_all_lower_monomials_are_enclosed() {
        for degree in 0..=13 {
            let c=(0..=degree).map(|k|[I::point(if k==degree {1.} else {0.}),
                I::point(0.),I::point(0.),I::point(if k==degree {2.} else {1.})]).collect::<Vec<_>>();
            for [lo,hi] in [[0.,0.],[0.,1.],[1.,1.],[0.25,0.75],[0.5,0.5],[0.,0.5],[0.5,1.]] {
                let out=restrict(&c,[0.,1.],[lo,hi],[1.,2.]).unwrap();
                assert_eq!(out.len(),if lo==hi {1}else{degree+1});
                for (k,v) in out.iter().enumerate() {
                    let exact=if lo==hi {lo.powi(degree as i32)} else {
                        lo.powi((degree-k) as i32)*hi.powi(k as i32)};
                    assert!(v[0].lo<=exact && exact<=v[0].hi);
                    assert!(v[3].lo<=1.+exact && 1.+exact<=v[3].hi);
                    assert!(v[3].lo>=1. && v[3].hi<=2.);
                }
            }
        }
    }
    #[test]
    fn full_span_and_endpoints_preserve_original_homogeneous_bits() {
        let c=(0..14).map(|i|[I::new(i as f64, i as f64+0.5).unwrap(),
            I::point(0.),I::point(0.),I::point(1.)]).collect::<Vec<_>>();
        let bits=|v: Vec<[I;4]>|v.into_iter().flat_map(|p|p.into_iter()
            .flat_map(|x|[x.lo.to_bits(),x.hi.to_bits()])).collect::<Vec<_>>();
        assert_eq!(bits(restrict(&c,[0.,1.],[0.,1.],[1.,1.]).unwrap()),bits(c.clone()));
        assert_eq!(bits(restrict(&c,[0.,1.],[0.,0.],[1.,1.]).unwrap()),bits(vec![c[0]]));
        assert_eq!(bits(restrict(&c,[0.,1.],[1.,1.],[1.,1.]).unwrap()),bits(vec![c[13]]));
    }
    #[test]
    fn non_bernstein_original_spans_keep_the_general_fallback() {
        assert!(is_complete_span(2,&[0.,0.,0.,1.,1.,1.],2));
        assert!(!is_complete_span(2,&[0.,0.,0.,0.5,1.,1.,1.],2));
        assert!(!is_complete_span(2,&[0.,0.,0.,0.5,1.,1.,1.],3));
    }
    #[test]
    fn upper_endpoint_restriction_encloses_exact_dyadic_bernstein_coefficients() {
        fn choose(n: usize,k: usize)->i128 {
            if k>n { return 0; }
            (0..k).fold(1,|v,j|v*(n-j) as i128/(j+1) as i128)
        }
        // B_i^n restricted to [m/8,1] has coefficient k equal to
        // C(n-k,i-k) (m/8)^(i-k) (1-m/8)^(n-i), or zero for i<k.
        // Through degree 13 these integer dyadic numerators are exact in f64.
        for n in 0..=13 {
            for i in 0..=n {
                let c=(0..=n).map(|j|[I::point(if j==i {1.}else{0.}),
                    I::point(0.),I::point(0.),I::point(1.)]).collect::<Vec<_>>();
                for m in 1_i128..8 {
                    let out=restrict(&c,[-3.,5.],[-3.+m as f64,5.],[1.,1.]).unwrap();
                    assert_eq!(out.len(),n+1);
                    for (k,h) in out.iter().enumerate() {
                        let exact=if i<k {0.}else{
                            let numerator=choose(n-k,i-k)*m.pow((i-k) as u32)
                                *(8-m).pow((n-i) as u32);
                            numerator as f64 / 8_i128.pow((n-k) as u32) as f64
                        };
                        assert!(h[0].lo<=exact && exact<=h[0].hi,
                            "n={n} i={i} k={k} m={m}: {:?}, {exact}",h[0]);
                        assert!(h[3].lo<=1. && h[3].hi>=1.);
                    }
                }
            }
        }
    }
    #[test]
    fn upper_endpoint_restriction_is_no_wider_than_redundant_identity_split() {
        for degree in [1,2,3,8,13] {
            let controls=(0..=degree).map(|i|[
                I::new(i as f64-7.25,i as f64-7.).unwrap(),
                I::point(i as f64/16.),I::point(0.),I::new(0.5,1.).unwrap()
            ]).collect::<Vec<_>>();
            let span=[-3.,5.];let width=I::point(5.).sub(I::point(-3.)).unwrap();
            let (old_left,_)=split(&controls,width.div(width).unwrap().intersect(0.,1.).unwrap(),[0.5,1.]).unwrap();
            for endpoint in [-2.,0.,1.,4.] {
                let alpha=I::point(endpoint).sub(I::point(-3.)).unwrap().div(width).unwrap().intersect(0.,1.).unwrap();
                let old=split(&old_left,alpha,[0.5,1.]).unwrap().1;
                let new=restrict(&controls,span,[endpoint,5.],[0.5,1.]).unwrap();
                for (a,b) in new.iter().zip(old) {for axis in 0..4 {
                    assert!(a[axis].lo>=b[axis].lo && a[axis].hi<=b[axis].hi);
                }}
            }
        }
    }

}
