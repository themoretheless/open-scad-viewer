//! Outward restriction of an original complete Bernstein span.
use crate::{Result, distance_bounds::Interval as I};
pub(super) fn is_complete_span(degree: usize, knots: &[f64], span: usize) -> bool {
    span >= degree && knots.get(span).zip(knots.get(span+1)).is_some_and(|(&a,&b)|
        a < b && knots[span-degree..=span].iter().all(|&x|x==a)
        && knots.get(span+1..span+degree+2).is_some_and(|k|k.iter().all(|&x|x==b)))
}
fn split(controls: &[[I;4]], alpha: I, weights: [f64;2]) -> Result<(Vec<[I;4]>,Vec<[I;4]>)> {
    let beta = I::point(1.).sub(alpha)?.intersect(0.,1.)?;
    // Equal coordinate intervals remain exactly equal after the original
    // convex blend and hull clipping. Bound magnitudes to retain overflow
    // refusal behavior; zero endpoints keep the original signed-zero path.
    let constant=std::array::from_fn::<_,4,_>(|k| {
        let x=controls[0][k];
        x.lo!=0. && x.hi!=0. && x.lo.abs().max(x.hi.abs())<=f64::MAX/4.
            && controls.iter().all(|p|p[k].lo.to_bits()==x.lo.to_bits()
                && p[k].hi.to_bits()==x.hi.to_bits())
    });
    let mut d=controls.to_vec();
    let mut left=vec![d[0]];let mut right=vec![*d.last().unwrap()];
    for remaining in (1..d.len()).rev() {
        for j in 0..remaining {
            let a=d[j];let b=d[j+1];
            for k in 0..4 { if constant[k] { continue; } d[j][k]=super::mul_blend_factor(a[k],beta)?.add(super::mul_blend_factor(b[k],alpha)?)?
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
    fn constant_coordinate_shortcut_keeps_frozen_interval_bits_and_refusals() {
        fn bits(result: Result<(Vec<[I;4]>,Vec<[I;4]>)>) -> std::result::Result<Vec<u64>, (String,String)> {
            result.map(|(a,b)|a.into_iter().chain(b).flat_map(|p|p.into_iter()
                .flat_map(|v|[v.lo.to_bits(),v.hi.to_bits()])).collect())
                .map_err(|e|(format!("{:?}",e.code),e.message.to_string()))
        }
        let sub=f64::from_bits(1);
        let values=[-f64::MAX,-1e308,-1e100,-1.,-sub,-0.,0.,sub,1.,1e100,1e308,f64::MAX];
        for degree in 0..=13 {for &x in &values {for alpha in
            [I::point(0.),I::point(1.),I::point(0.5),I::new(0.25,0.75).unwrap(),I::new(0.5_f64.next_down(),0.5_f64.next_up()).unwrap()] {
            let controls=(0..=degree).map(|j|[I::point(x),I::point(j as f64),
                I::new(-sub,sub).unwrap(),I::point(1.)]).collect::<Vec<_>>();
            assert_eq!(bits(split(&controls,alpha,[0.5,2.])),bits(frozen_split(&controls,alpha,[0.5,2.])));
        }}}
    }
fn frozen_split(controls: &[[I;4]], alpha: I, weights: [f64;2]) -> Result<(Vec<[I;4]>,Vec<[I;4]>)> {
    let beta = I::point(1.).sub(alpha)?.intersect(0.,1.)?;
    let mut d=controls.to_vec();
    let mut left=vec![d[0]];let mut right=vec![*d.last().unwrap()];
    for remaining in (1..d.len()).rev() {
        for j in 0..remaining {
            let a=d[j];let b=d[j+1];
            for k in 0..4 { d[j][k]=super::super::mul_blend_factor(a[k],beta)?.add(super::super::mul_blend_factor(b[k],alpha)?)?
                .intersect(a[k].lo.min(b[k].lo),a[k].hi.max(b[k].hi))?; }
            d[j][3]=d[j][3].intersect(weights[0],weights[1])?;
        }
        left.push(d[0]);right.push(d[remaining-1]);
    }
    right.reverse();Ok((left,right))
}
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
}
