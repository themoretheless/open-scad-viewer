//! Complete distinct real roots on one closed exact interval.
use super::*;
use std::collections::VecDeque;
#[derive(Clone,Copy,Debug)]
pub struct SsAlgebraicRootCensusLimits {
    pub max_cells:usize,
    pub max_roots:usize,
    pub clock:SsAlgebraicClockLimits,
}
impl Default for SsAlgebraicRootCensusLimits {
    fn default()->Self {Self {max_cells:16384,max_roots:64,clock:SsAlgebraicClockLimits {max_operations:1000000,..Default::default()}}}
}
#[derive(Clone,Debug)]
pub struct QualifiedSsAlgebraicRootCensus {
    polynomial:Vec<Q>,domain:[Q;2],limits:SsAlgebraicRootCensusLimits,
    roots:Vec<QualifiedSsAlgebraicClock>,work:usize,cells:usize,
}
impl QualifiedSsAlgebraicRootCensus {
    pub fn polynomial(&self)->&[Q] {&self.polynomial}
    pub fn domain(&self)->&[Q;2] {&self.domain}
    pub fn roots(&self)->&[QualifiedSsAlgebraicClock] {&self.roots}
    pub fn work(&self)->usize {self.work}
    pub fn cells(&self)->usize {self.cells}
    pub fn isolate(mut polynomial:Vec<Q>,domain:[Q;2],limits:SsAlgebraicRootCensusLimits)->Result<Self> {
        let mut w=Work::new(limits.clock)?;
        check((1..=1000000).contains(&limits.max_cells) && (1..=64).contains(&limits.max_roots),"Invalid complete clock census policy")?;
        check(!polynomial.is_empty() && polynomial.len()<=limits.clock.max_degree+1,"Clock census degree allowance exceeded")?;
        for c in polynomial.iter().chain(domain.iter()) {w.input(c.clone())?;}
        trim(&mut polynomial);
        check(!polynomial.is_empty() && domain[0]<=domain[1],"Zero polynomial or reversed clock census domain")?;
        let mut roots=Vec::new();let mut cells=0;
        if polynomial.len()>1 {
            let chain=Arc::new(sturm(&polynomial,&mut w)?);
            let clock=|d:[Q;2],used:usize| QualifiedSsAlgebraicClock {
                polynomial:polynomial.clone(),interval:d,chain:chain.clone(),operations:used,limits:limits.clock,
            };
            for k in 0..2 {
                if k==1 && domain[0]==domain[1] {break;}
                if evaluate(&polynomial,&domain[k],&mut w)?.is_zero() {
                    roots.push(clock([domain[k].clone(),domain[k].clone()],w.used));
                }
            }
            let interior=count_open(&chain,&domain,&mut w)?;
            let total=interior.checked_add(roots.len()).ok_or_else(||crate::resource("Clock census root overflow"))?;
            check(total<=limits.max_roots,"Clock census root allowance exhausted")?;
            let mut pending=VecDeque::new();
            if interior>0 {pending.push_back((domain.clone(),interior));}
            while let Some((d,n))=pending.pop_front() {
                if cells==limits.max_cells {return Err(crate::resource("Complete clock census cell allowance exhausted"));}
                cells+=1;
                let left_root=evaluate(&polynomial,&d[0],&mut w)?.is_zero();
                let right_root=evaluate(&polynomial,&d[1],&mut w)?.is_zero();
                if n==1 && !left_root && !right_root {roots.push(clock(d,w.used));continue;}
                let sum=w.keep(&d[0]+&d[1])?;
                let mid=w.keep(sum/Q::from_integer(2.into()))?;
                let middle_root=evaluate(&polynomial,&mid,&mut w)?.is_zero();
                let left=[d[0].clone(),mid.clone()];let right=[mid.clone(),d[1].clone()];
                let a=count_open(&chain,&left,&mut w)?;let b=count_open(&chain,&right,&mut w)?;
                check(a+b+usize::from(middle_root)==n,"Complete clock census lost an exact root")?;
                if middle_root {roots.push(clock([mid.clone(),mid],w.used));}
                if a>0 {pending.push_back((left,a));}if b>0 {pending.push_back((right,b));}
            }
            check(roots.len()==total,"Complete clock census root count changed")?;
            roots.sort_by(|a,b|a.interval[0].cmp(&b.interval[0]));
        }
        Ok(Self {polynomial,domain,limits,roots,work:w.used,cells})
    }
    pub fn reverify(&self)->Result<()> {
        let fresh=Self::isolate(self.polynomial.clone(),self.domain.clone(),self.limits)?;
        check(self.cells==fresh.cells && self.roots.len()==fresh.roots.len()
            && self.roots.iter().zip(&fresh.roots).all(|(a,b)|a.interval==b.interval),"Complete clock census changed")
    }
}
