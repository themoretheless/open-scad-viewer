use super::*;
type U=P<1>;
pub(super) fn poly(c:&[Q],w:&mut Work)->Result<U> {
    let mut p=U::default();for (i,v) in c.iter().enumerate() {p.insert([i],v.clone(),w)?;}Ok(p)
}
fn reduce(p:U,m:&U,w:&mut Work)->Result<U> {groebner::normal_form(&p,&[m.clone()],[0],w)}
#[derive(Clone)]
pub(super) struct Rat {pub n:U,d:U}
impl Rat {
    fn constant(c:Q,w:&mut Work)->Result<Self> {Ok(Self {n:poly(&[c],w)?,d:poly(&[Q::one()],w)?})}
    pub fn variable(w:&mut Work)->Result<Self> {Ok(Self {n:poly(&[Q::zero(),Q::one()],w)?,d:poly(&[Q::one()],w)?})}
    fn mul(&self,b:&Self,m:&U,w:&mut Work)->Result<Self> {
        let n=reduce(self.n.product(&b.n,w)?,m,w)?;let d=reduce(self.d.product(&b.d,w)?,m,w)?;
        check(!d.is_zero(),"Third-point modular denominator vanished")?;Ok(Self {n,d})
    }
    fn add(&self,b:&Self,m:&U,w:&mut Work)->Result<Self> {
        let left=self.n.product(&b.d,w)?;let right=b.n.product(&self.d,w)?;
        let neg=right.scale_monomial([0],&-Q::one(),w)?;
        let n=reduce(left.subtract(&neg,w)?,m,w)?;
        let d=reduce(self.d.product(&b.d,w)?,m,w)?;
        check(!d.is_zero(),"Third-point modular denominator vanished")?;Ok(Self {n,d})
    }
    fn pow(&self,mut e:usize,m:&U,w:&mut Work)->Result<Self> {
        let mut out=Self::constant(Q::one(),w)?;let mut base=self.clone();
        while e>0 {if e%2==1 {out=out.mul(&base,m,w)?;}e/=2;if e>0 {base=base.mul(&base,m,w)?;}}
        Ok(out)
    }
    pub fn enclose(&self,t:&[Q;2],w:&mut Work)->Result<[Q;2]> {
        let n=interval(&self.n,t,w)?;let d=interval(&self.d,t,w)?;
        check(d[1]<Q::zero() || Q::zero()<d[0],"Third-point denominator is not separated from zero")?;
        w.charge()?;let a=Q::one()/&d[1];w.coefficient(&a)?;
        w.charge()?;let b=Q::one()/&d[0];w.coefficient(&b)?;
        times(&n,&[a,b],w)
    }
}
pub(super) fn extract(basis:&[P<7>],axis:usize,clock:usize,m:&U,w:&mut Work)->Result<Rat> {
    let candidate=basis.iter().filter(|p|p.terms.keys().all(|e|e[axis]<=1 && (0..7).all(|k|k==axis || k==clock || e[k]==0))
        && p.terms.keys().any(|e|e[axis]==1)).min_by_key(|p|p.terms.len())
        .ok_or_else(||crate::input("Third-point rational-univariate coordinate lift is unavailable"))?;
    let mut n=U::default();let mut d=U::default();
    for (e,c) in &candidate.terms {
        if e[axis]==1 {d.insert([e[clock]],c.clone(),w)?;} else {n.insert([e[clock]],-c,w)?;}
    }
    let n=reduce(n,m,w)?;let d=reduce(d,m,w)?;
    check(!d.is_zero(),"Third-point coordinate denominator is zero modulo the clock")?;Ok(Rat {n,d})
}
pub(super) fn evaluate(p:&P<7>,maps:&[Rat],m:&U,w:&mut Work)->Result<Rat> {
    let mut out=Rat::constant(Q::zero(),w)?;
    for (e,c) in &p.terms {
        check(e[6]==0,"Inverse-variable relation cannot substitute for original geometry")?;
        let mut term=Rat::constant(c.clone(),w)?;
        for k in 0..6 {if e[k]>0 {term=term.mul(&maps[k].pow(e[k],m,w)?,m,w)?;}}
        out=out.add(&term,m,w)?;
    }
    Ok(out)
}
fn times(a:&[Q;2],b:&[Q;2],w:&mut Work)->Result<[Q;2]> {
    let mut values=Vec::new();for x in a {for y in b {w.charge()?;let v=x*y;w.coefficient(&v)?;values.push(v);}}
    Ok([values.iter().min().unwrap().clone(),values.iter().max().unwrap().clone()])
}
fn power(t:&[Q;2],mut e:usize,w:&mut Work)->Result<[Q;2]> {
    let mut out=[Q::one(),Q::one()];let mut b=t.clone();
    while e>0 {if e%2==1 {out=times(&out,&b,w)?;}e/=2;if e>0 {b=times(&b,&b,w)?;}}Ok(out)
}
fn interval(p:&U,t:&[Q;2],w:&mut Work)->Result<[Q;2]> {
    let mut out=[Q::zero(),Q::zero()];
    for (e,c) in &p.terms {
        let term=times(&[c.clone(),c.clone()],&power(t,e[0],w)?,w)?;
        for k in 0..2 {w.charge()?;out[k]+=&term[k];w.coefficient(&out[k])?;}
    }
    Ok(out)
}

pub(super) fn has_relation(basis:&[P<7>],axis:usize,clock:usize)->bool {
    basis.iter().any(|p|p.terms.keys().all(|e|e[axis]<=1 && (0..7).all(|k|k==axis || k==clock || e[k]==0))
        && p.terms.keys().any(|e|e[axis]==1))
}
pub(super) fn scalar_image(c:&[Q],m:&U,t:&[Q;2],w:&mut Work)->Result<[Q;2]> {
    let p=reduce(poly(c,w)?,m,w)?;interval(&p,t,w)
}
