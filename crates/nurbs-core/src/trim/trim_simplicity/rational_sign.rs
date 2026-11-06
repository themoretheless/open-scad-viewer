//! Strict monotonicity of ORIGINAL rational coordinates, not a fitted clock.
//! W>0 and a nonzero, one-signed N'W-NW' on every exact Bernstein leaf imply
//! strict monotonicity even when derivatives vanish at isolated leaf joins.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
struct Work<'a> { used: &'a mut usize, max: usize }
impl Work<'_> {
    fn op(&mut self, f: impl FnOnce()->Q)->Option<Q> {
        if *self.used>=self.max {return None;}
        *self.used+=1;
        let q=f();
        (q.numer().bits()<=8192 && q.denom().bits()<=8192).then_some(q)
    }
    fn charge(&mut self)->Option<()> {
        if *self.used>=self.max {None}else{*self.used+=1;Some(())}
    }
}
fn choose(n:usize,k:usize)->u128 {
    (0..k.min(n-k)).fold(1,|v,i|v*(n-i)as u128/(i+1)as u128)
}
fn sign_cover(p:Vec<Q>,positive:bool,w:&mut Work)->Option<bool> {
    let mut pending=vec![(p,0)];
    while let Some((mut p,depth))=pending.pop() {
        let mut ordered=true;let mut nonzero=false;
        for c in &p {
            w.charge()?;
            ordered &= if positive {!c.is_negative()}else{!c.is_positive()};
            nonzero |= !c.is_zero();
        }
        if ordered && nonzero {continue;}
        if !nonzero || depth==16 {return Some(false);}
        let n=p.len();let mut left=vec![p[0].clone()];let mut right=vec![p[n-1].clone()];
        for level in 1..n {
            for i in 0..n-level {
                let sum=w.op(||&p[i]+&p[i+1])?;
                p[i]=w.op(||sum/Q::from_integer(2.into()))?;
            }
            left.push(p[0].clone());right.push(p[n-level-1].clone());
        }
        right.reverse();pending.push((right,depth+1));pending.push((left,depth+1));
    }
    Some(true)
}
fn coordinate(c:&Curve,axis:usize,weights:&[Q],w:&mut Work)->Option<bool> {
    let poles=c.control_points.iter().zip(weights).map(|(p,q)| {
        let x=w.op(||Q::from_float(p[axis]).expect("validated finite pole"))?;
        w.op(||x*q)
    }).collect::<Option<Vec<_>>>()?;
    let n=c.degree;
    let mut numerator=vec![Q::zero();2*n-1];
    // Antisymmetric pair identity cancels the leading derivative degree
    // BEFORE arithmetic: H has degree <=2n-2, not the naive 2n-1.
    // B'_j B_i-B_j B'_i=(j-i) B_j B_i/(t(1-t)).
    // Every i<j pair is owned exactly once, with no rounded cancellation.
    for i in 0..=n {for j in i+1..=n {
        let a=w.op(||&poles[j]*&weights[i])?;
        let b=w.op(||&poles[i]*&weights[j])?;
        let delta=w.op(||a-b)?;
        let ratio=w.op(||Q::new(((j-i)as u128*choose(n,i)*choose(n,j)).into(),choose(2*n-2,i+j-1).into()))?;
        let term=w.op(||delta*ratio)?;
        numerator[i+j-1]=w.op(||&numerator[i+j-1]+term)?;
    }}
    sign_cover(numerator,c.control_points[0][axis]<c.control_points.last()?[axis],w)
}
/// Cheap exact covariance-sign case, charged comparison by comparison.
/// Used ahead of the Mobius attempt ONLY for symmetric nonconstant weights,
/// which cannot be a positive geometric sequence. Existing Mobius work stays.
pub(super) fn ordered(c:&Curve,used:&mut usize,max:usize)->Result<bool> {
    c.validate()?;
    if !bezier(c) || !(1..=32).contains(&c.degree) || c.control_points.iter().any(|p|p.len()<2)
        || c.weights.iter().any(|v|!v.is_finite() || *v<=0.) {return Ok(false);}
    let mut work=Work{used,max};
    for axis in 0..2 {
        let a=c.control_points[0][axis];let b=c.control_points.last().unwrap()[axis];
        if a==b {continue;}
        let mut ordered=true;
        for p in c.control_points.windows(2) {
            if work.charge().is_none() {return Ok(false);}
            if if a<b {p[0][axis]>p[1][axis]}else{p[0][axis]<p[1][axis]} {ordered=false;break;}
        }
        if ordered {return Ok(true);}
    }
    Ok(false)
}
pub(super) fn prove(c:&Curve,used:&mut usize,max:usize)->Result<bool> {
    c.validate()?;
    if !bezier(c) || !(1..=32).contains(&c.degree)
        || c.control_points.iter().any(|p|p.len()<2)
        || c.weights.iter().any(|w|!w.is_finite() || *w<=0.) {return Ok(false);}
    let mut w=Work{used,max};
    let Some(weights)=c.weights.iter().map(|&x|w.op(||Q::from_float(x).expect("positive finite weight"))).collect::<Option<Vec<_>>>()else{return Ok(false);};
    for axis in 0..2 {
        if c.control_points[0][axis]==c.control_points.last().unwrap()[axis] {continue;}
        match coordinate(c,axis,&weights,&mut w) {
            Some(true)=>return Ok(true),None=>return Ok(false),Some(false)=>{}
        }
    }
    Ok(false)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn curve()->Curve {
        let weights=vec![1.,2.,2.,2.,2.,1.];
        Curve {degree:5,knots:[vec![0.;6],vec![1.;6]].concat(),
            control_points:weights.iter().enumerate().map(|(i,&w)|vec![if i%2==0 {-10./w}else{10./w},0.]).collect(),
            weights,periodic:false}
    }
    #[test]
    fn non_mobius_rational_derivative_owns_exact_sign_and_shared_work() {
        let c=curve();let before=c.clone();let mut used=0;
        assert!(!super::super::mobius::prove(&c,&mut 0,1000).unwrap());
        assert!(prove(&c,&mut used,1000).unwrap(),"work={used}");
        let mut replay=0;assert!(prove(&c,&mut replay,used).unwrap());assert_eq!(used,replay);
        assert!(!prove(&c,&mut 0,used-1).unwrap());assert!(!prove(&c,&mut 0,0).unwrap());
        let mut folded=c.clone();folded.control_points[2][0]=100.;
        assert!(!prove(&folded,&mut 0,1000).unwrap());
        let mut invalid=c.clone();invalid.weights[2]=0.;assert!(prove(&invalid,&mut 0,1000).is_err());
        assert_eq!(c,before);
    }
}
