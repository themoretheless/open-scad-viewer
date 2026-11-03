//! Shared bounded exact operations and one-sided original homogeneous jets.
use crate::core::bounded_rational::{Error,Work};
use num_rational::BigRational as Q;
use num_traits::{Signed,Zero};
pub(super) type R<T> = std::result::Result<T,Error>;
pub(super) fn add(w:&mut Work,a:&Q,b:&Q)->R<Q>{w.op(a,b,|a,b|a+b)}
pub(super) fn sub(w:&mut Work,a:&Q,b:&Q)->R<Q>{w.op(a,b,|a,b|a-b)}
pub(super) fn mul(w:&mut Work,a:&Q,b:&Q)->R<Q>{w.op(a,b,|a,b|a*b)}
pub(super) fn div(w:&mut Work,a:&Q,b:&Q)->R<Q>{
    if b.is_zero(){return Err(Error::NotRepresentable);}w.op(a,b,|a,b|a/b)
}
pub(super) fn same(w:&mut Work,a:&Q,b:&Q)->R<bool>{Ok(sub(w,a,b)?.is_zero())}
pub(super) fn unit(w:&mut Work,x:&Q)->R<bool>{
    let one=w.lift(1.)?;Ok(!x.is_negative() && !sub(w,&one,x)?.is_negative())
}

/// One actual nonempty original span; derivative limits retain its own side.
pub(super) fn span_jet(p:usize,k:&[Q],rows:&[Vec<Q>],t:&Q,span:usize,w:&mut Work,derivative:bool)->R<(Vec<Q>,Vec<Q>)> {
    let mut d=rows[span-p..=span].to_vec();
    let mut dd=if derivative {vec![vec![Q::zero();rows[0].len()];p+1]} else {Vec::new()};
    for r in 1..=p {for j in (r..=p).rev(){
        let i=span-p+j;let n=sub(w,t,&k[i])?;let gap=sub(w,&k[i+p-r+1],&k[i])?;
        if !gap.is_positive(){return Err(Error::NotRepresentable);}
        let a=div(w,&n,&gap)?;let one=w.lift(1.)?;let b=sub(w,&one,&a)?;
        let mut h=Vec::new();let mut first=Vec::new();
        for axis in 0..d[j].len(){
            let left=mul(w,&d[j-1][axis],&b)?;let right=mul(w,&d[j][axis],&a)?;
            h.push(add(w,&left,&right)?);
            if derivative {
                let left=mul(w,&dd[j-1][axis],&b)?;let right=mul(w,&dd[j][axis],&a)?;
                let average=add(w,&left,&right)?;let delta=sub(w,&d[j][axis],&d[j-1][axis])?;
                let slope=div(w,&delta,&gap)?;first.push(add(w,&average,&slope)?);
            }
        }
        d[j]=h;if derivative {dd[j]=first;}
    }}
    Ok((d.pop().unwrap(),if derivative {dd.pop().unwrap()} else {Vec::new()}))
}
