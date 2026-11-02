//! Original coefficient numerators of d(H/W); no sampled derivative bounds.
use super::*;
use crate::numerics::bernstein_tensor::{Net, derivative as d, minus, product};
pub(super) struct Jets {nets:[Vec<Vec<H>>;2]}
pub(super) fn coefficient_cost(p:usize,q:usize)->usize {
    4*(2*p*(2*q+1)+(2*p+1)*2*q)
}
impl Jets {
    pub(super) fn prepare(source:&[Vec<H>])->Result<Self> {
        let w:Net=source.iter().map(|r|r.iter().map(|p|p[4]).collect()).collect();
        let mut nets=Vec::new();
        for axis in 0..2 {
            let dw=d(&w,axis)?;let mut channels=Vec::new();
            for k in 0..4 {
                let h:Net=source.iter().map(|r|r.iter().map(|p|p[k]).collect()).collect();
                channels.push(minus(product(&d(&h,axis)?,&w)?,product(&h,&dw)?)?);
            }
            let mut net=vec![vec![[I::point(0.);5];channels[0][0].len()];channels[0].len()];
            for i in 0..net.len() {for j in 0..net[0].len() {for k in 0..4 {
                net[i][j][k]=channels[k][i][j];
            }}}
            nets.push(net);
        }
        Ok(Self{nets:nets.try_into().unwrap()})
    }
    pub(super) fn enclose(&self,uv:[[f64;2];2],w:I)->Result<[[I;4];2]> {
        let denominator=w.mul(w)?;let mut out=[[I::point(0.);4];2];
        for axis in 0..2 {
            let bound=restricted_hull(&self.nets[axis],uv)?;
            for k in 0..4 {out[axis][k]=bound[k].div(denominator)?;}
        }
        Ok(out)
    }
}
