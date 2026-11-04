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
