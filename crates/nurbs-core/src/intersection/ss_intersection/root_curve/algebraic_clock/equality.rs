//! Common-root comparisons under exact affine changes of scalar parameter.
use super::*;
pub(super) fn compare(a: (&[Q],&[Q;2]), b: (&[Q],&[Q;2]), w: &mut Work) -> Result<bool> {
        let d=[a.1[0].clone().max(b.1[0].clone()),
            a.1[1].clone().min(b.1[1].clone())];
        if d[1]<d[0] {return Ok(false);}
        if d[0]==d[1] {
            let same=evaluate(&a.0,&d[0],w)?.is_zero()
                && evaluate(&b.0,&d[0],w)?.is_zero();
            return Ok(same);
        }
        let common=gcd(&a.0,&b.0,w)?;
        if common.len()<2 {return Ok(false);}
        // Each nonpoint isolator has nonroot endpoints. Hence an intersection
        // endpoint cannot be a common root unless one isolator was a point.
        check(!evaluate(&common,&d[0],w)?.is_zero()
            && !evaluate(&common,&d[1],w)?.is_zero(),"Common-root interval endpoint invariant failed")?;
        let chain=sturm(&common,w)?;
        let roots=count(&chain,&d,w)?;
        check(roots<=1,"Common-root count contradicts unique selectors")?;
        Ok(roots==1)
}
pub(super) fn affine(a: &QualifiedSsAlgebraicClock,b: &QualifiedSsAlgebraicClock,scale: &Q,shift: &Q,limits: SsAlgebraicClockLimits) -> Result<(bool,usize)> {
    let mut w=Work::new(limits)?;
    w.input(scale.clone())?;
    w.input(shift.clone())?;
    check(!scale.is_zero(),"Affine SS clock map must be invertible")?;
    for clock in [a,b] {
        check(clock.polynomial.len()<=limits.max_degree+1,"Affine clock degree allowance exceeded")?;
        for c in clock.polynomial.iter().chain(clock.interval.iter()) {w.keep(c.clone())?;}
    }
    // Horner composition p(scale*x+shift) retains every exact coefficient.
    let mut p=Vec::<Q>::new();
    for coefficient in a.polynomial.iter().rev() {
        let mut next=vec![Q::zero();p.len()+1];
        for (i,c) in p.iter().enumerate() {
            let t=w.keep(c*shift)?;
            next[i]=w.keep(&next[i]+t)?;
            let t=w.keep(c*scale)?;
            next[i+1]=w.keep(&next[i+1]+t)?;
        }
        next[0]=w.keep(&next[0]+coefficient)?;
        trim(&mut next);
        p=next;
    }
    let mut d=[Q::zero(),Q::zero()];
    for k in 0..2 {
        let t=w.keep(&a.interval[k]-shift)?;
        d[k]=w.keep(t/scale)?;
    }
    if scale.is_negative() {d.swap(0,1);}
    let equal=compare((&p,&d),(&b.polynomial,&b.interval),&mut w)?;
    Ok((equal,w.used))
}
