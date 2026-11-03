//! Exact normalized first partials of unchanged rational tensor splines.
use super::*;
use super::super::exact::{span_jet,same};
pub(super) struct Tensor { pu:usize,pv:usize,ku:Vec<Q>,kv:Vec<Q>,h:Vec<Vec<Vec<Q>>> }
fn spans(p:usize,k:&[Q],n:usize,t:&Q,w:&mut Work)->R<Vec<usize>> {
    let mut result=Vec::new();for i in p..n {
        w.charge(1)?;
        if sub(w,&k[i+1],&k[i])?.is_positive() && !sub(w,t,&k[i])?.is_negative() && !sub(w,&k[i+1],t)?.is_negative(){result.push(i);}
    }
    if result.is_empty(){return Err(Error::NotRepresentable);}Ok(result)
}
fn clock(p:usize,k:&[Q],n:usize,t:&Q,w:&mut Work)->R<(Q,Q)> {
    let width=sub(w,&k[n],&k[p])?;let delta=mul(w,&width,t)?;Ok((add(w,&k[p],&delta)?,width))
}
impl Tensor {
    pub fn load(s:&Surface,w:&mut Work)->R<Self> {
        let mut h=Vec::new();
        for (row,weights) in s.control_points.iter().zip(&s.weights) {
            let mut hrow=Vec::new();for (p,weight) in row.iter().zip(weights) {
                let weight=w.lift(*weight)?;if !weight.is_positive(){return Err(Error::NotRepresentable);}
                let mut point=Vec::new();for x in p {let x=w.lift(*x)?;point.push(mul(w,&x,&weight)?);}
                point.push(weight);hrow.push(point);
            }h.push(hrow);
        }
        let ku=s.knots_u.iter().map(|x|w.lift(*x)).collect::<R<_>>()?;
        let kv=s.knots_v.iter().map(|x|w.lift(*x)).collect::<R<_>>()?;
        Ok(Self{pu:s.degree_u,pv:s.degree_v,ku,kv,h})
    }
    pub fn first(&self,uv:&[Q;2],w:&mut Work)->R<[[Q;3];2]> {
        let (u,su)=clock(self.pu,&self.ku,self.h.len(),&uv[0],w)?;
        let (v,sv)=clock(self.pv,&self.kv,self.h[0].len(),&uv[1],w)?;
        let us=spans(self.pu,&self.ku,self.h.len(),&u,w)?;
        let vs=spans(self.pv,&self.kv,self.h[0].len(),&v,w)?;
        let mut answer:Option<[[Q;3];3]>=None;
        for j in vs {
            let mut rows=Vec::new();let mut vrows=Vec::new();
            for row in &self.h {let (h,v)=span_jet(self.pv,&self.kv,row,&v,j,w,true)?;rows.push(h);vrows.push(v);}
            for &i in &us {
                let (h,hu)=span_jet(self.pu,&self.ku,&rows,&u,i,w,true)?;
                let (hv,_)=span_jet(self.pu,&self.ku,&vrows,&u,i,w,false)?;
                if !h[3].is_positive(){return Err(Error::NotRepresentable);}
                let ww=mul(w,&h[3],&h[3])?;let mut jet=Vec::new();
                jet.push((0..3).map(|k|div(w,&h[k],&h[3])).collect::<R<Vec<_>>>()?.try_into().unwrap());
                for (d,scale) in [(&hu,&su),(&hv,&sv)] {
                    let mut partial=Vec::new();for k in 0..3 {
                        let a=mul(w,&d[k],&h[3])?;let b=mul(w,&h[k],&d[3])?;let n=sub(w,&a,&b)?;
                        let q=div(w,&n,&ww)?;partial.push(mul(w,&q,scale)?);
                    }jet.push(partial.try_into().unwrap());
                }
                let jet:[[Q;3];3]=jet.try_into().unwrap();
                if let Some(old)=&answer {for (a,b) in old.iter().flatten().zip(jet.iter().flatten()) {
                    if !same(w,a,b)?{return Err(Error::NotRepresentable);}
                }}answer=Some(jet);
            }
        }
        let [_,du,dv]=answer.ok_or(Error::NotRepresentable)?;Ok([du,dv])
    }
}
