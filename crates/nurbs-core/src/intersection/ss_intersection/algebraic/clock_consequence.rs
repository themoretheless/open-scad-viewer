//! Original SS equations plus an owned scalar clock imply another clock.
use super::*;
fn univariate(coefficients:&[Q],axis:usize,scale:&Q,shift:&Q,w:&mut Work)->Result<ExactSsPolynomial> {
    check(axis<4 && !coefficients.is_empty() && coefficients.len()<=65,"Invalid SS clock constraint degree or axis")?;
    let mut map=ExactSsPolynomial::default();
    map.insert([0;4],shift.clone(),w)?;
    let mut e=[0;4]; e[axis]=1;
    map.insert(e,scale.clone(),w)?;
    let mut p=ExactSsPolynomial::default();
    for c in coefficients.iter().rev() {
        w.coefficient(c)?;
        p=p.product(&map,w)?;
        p.insert([0;4],c.clone(),w)?;
    }
    Ok(p)
}
pub(crate) fn prove(ideal:&QualifiedOriginalSsIdeal,first:(&[Q],usize),second:(&[Q],usize,Q,Q),limits:ExactSsGroebnerLimits)->Result<(bool,usize)> {
    validate_limits(limits.algebra)?;
    let mut order=limits.variable_order; order.sort();
    check(order==[0,1,2,3] && (1..=65536).contains(&limits.max_pairs)
        && (1..=256).contains(&limits.max_basis),"Invalid SS clock consequence elimination policy")?;
    let mut w=Work {used:0,limits:limits.algebra};
    let constraint=univariate(first.0,first.1,&Q::one(),&Q::zero(),&mut w)?;
    let consequence=univariate(second.0,second.1,&second.2,&second.3,&mut w)?;
    let mut equations=ideal.equations().to_vec(); equations.push(constraint);
    let (basis,_)=groebner::basis_for(&equations,limits.variable_order,limits.max_pairs,limits.max_basis,&mut w)?;
    let remainder=groebner::normal_form(&consequence,&basis,limits.variable_order,&mut w)?;
    Ok((remainder.is_zero(),w.used))
}

/// Eliminate every coordinate except the requested target, then express its
/// exact consequence polynomial in the target graph's native clock.
pub(crate) fn project(ideal:&QualifiedOriginalSsIdeal,first:(&[Q],usize),target:(usize,Q,Q),limits:ExactSsGroebnerLimits)->Result<(Vec<Q>,usize)> {
    validate_limits(limits.algebra)?;
    check(target.0<4 && !target.1.is_zero() && (1..=65536).contains(&limits.max_pairs)
        && (1..=256).contains(&limits.max_basis),"Invalid SS clock projection policy")?;
    let mut w=Work {used:0,limits:limits.algebra};
    let constraint=univariate(first.0,first.1,&Q::one(),&Q::zero(),&mut w)?;
    let mut equations=ideal.equations().to_vec(); equations.push(constraint);
    let mut axes:Vec<usize>=(0..4).filter(|k|*k!=target.0).collect(); axes.push(target.0);
    let order:[usize;4]=axes.try_into().unwrap();
    let (basis,_)=groebner::basis_for(&equations,order,limits.max_pairs,limits.max_basis,&mut w)?;
    let candidate=basis.iter().filter(|p|!p.is_zero()
        && p.terms.keys().all(|e|(0..4).all(|k|k==target.0 || e[k]==0)))
        .min_by_key(|p|p.terms.keys().map(|e|e[target.0]).max().unwrap_or(0))
        .ok_or_else(||crate::input("Original SS clock has no univariate target consequence"))?;
    let degree=candidate.terms.keys().map(|e|e[target.0]).max().unwrap_or(0);
    check((1..=64).contains(&degree),"Projected SS clock degree is unsupported")?;
    let mut dense=vec![Q::zero();degree+1];
    for (e,c) in &candidate.terms {dense[e[target.0]]=c.clone();}
    let mapped=univariate(&dense,target.0,&target.1,&target.2,&mut w)?;
    let mut coefficients=vec![Q::zero();degree+1];
    for (e,c) in mapped.terms {coefficients[e[target.0]]=c;}
    Ok((coefficients,w.used))
}
