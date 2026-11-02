//! Bounded binary-rational evaluation of unchanged original spline coefficients.
use crate::{core::bounded_rational::{Error,Work},curve::Curve};
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
pub(super) struct Spline { pub p:usize,pub k:Vec<Q>,pub h:Vec<Vec<Q>> }
impl Spline {
    pub fn load(c:&Curve,w:&mut Work)->R<Self>{
        let mut h=Vec::new();
        for (point,weight) in c.control_points.iter().zip(&c.weights){
            let weight=w.lift(*weight)?;
            if !weight.is_positive(){return Err(Error::NotRepresentable);}
            let mut row=Vec::new();
            for x in point {let x=w.lift(*x)?;row.push(mul(w,&x,&weight)?);}
            row.push(weight);h.push(row);
        }
        let k=c.knots.iter().map(|t|w.lift(*t)).collect::<R<_>>()?;
        Ok(Self{p:c.degree,k,h})
    }
    pub fn normalized(&self,t:&Q,w:&mut Work)->R<Vec<Q>>{
        let width=sub(w,&self.k[self.h.len()],&self.k[self.p])?;
        let offset=mul(w,&width,t)?;let raw=add(w,&self.k[self.p],&offset)?;
        self.point(&raw,w)
    }
    pub fn fraction(&self,raw:f64,w:&mut Work)->R<Q>{
        let raw=w.lift(raw)?;let n=sub(w,&raw,&self.k[self.p])?;
        let d=sub(w,&self.k[self.h.len()],&self.k[self.p])?;div(w,&n,&d)
    }
    pub fn point(&self,t:&Q,w:&mut Work)->R<Vec<Q>>{
        let mut result:Option<Vec<Q>>=None;
        for i in self.p..self.h.len(){
            w.charge(1)?;
            if !sub(w,&self.k[i+1],&self.k[i])?.is_positive()
                || sub(w,t,&self.k[i])?.is_negative() || sub(w,&self.k[i+1],t)?.is_negative(){continue;}
            let mut d=self.h[i-self.p..=i].to_vec();
            for r in 1..=self.p {for j in (r..=self.p).rev(){
                let k=i-self.p+j;let n=sub(w,t,&self.k[k])?;
                let gap=sub(w,&self.k[k+self.p-r+1],&self.k[k])?;
                if !gap.is_positive(){return Err(Error::NotRepresentable);}
                let a=div(w,&n,&gap)?;let one=w.lift(1.)?;let b=sub(w,&one,&a)?;
                let mut row=Vec::new();
                for axis in 0..d[j].len(){
                    let left=mul(w,&d[j-1][axis],&b)?;let right=mul(w,&d[j][axis],&a)?;
                    row.push(add(w,&left,&right)?);
                }
                d[j]=row;
            }}
            let value=d.pop().unwrap();let weight=value.last().unwrap();
            if !weight.is_positive(){return Err(Error::NotRepresentable);}
            let xyz=value[..value.len()-1].iter().map(|x|div(w,x,weight)).collect::<R<Vec<_>>>()?;
            if let Some(old)=&result {for (a,b) in old.iter().zip(&xyz){
                if !same(w,a,b)?{return Err(Error::NotRepresentable);}
            }}
            result=Some(xyz);
        }
        result.ok_or(Error::NotRepresentable)
    }
}
