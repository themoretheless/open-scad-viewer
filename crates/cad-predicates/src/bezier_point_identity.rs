//! Exact equality of two original rational Bezier points at authored parameters.
use crate::{
    exact_inputs, Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion,
    InputError, LeafRef, PredicateContext, Reason, Sign,
};
fn point(
    ctx: &mut PredicateContext<'_>,
    poles: &[Expansion],
    parameter: &[Expansion],
) -> Result<Option<Vec<Expansion>>, Reason> {
    let low = parameter[0].sub(&parameter[1], ctx)?;
    let high = parameter[2].sub(&parameter[0], ctx)?;
    if parameter[2].sub(&parameter[1], ctx)?.sign() != Sign::Positive
        || low.sign() == Sign::Negative
        || high.sign() == Sign::Negative
        || poles.chunks_exact(3).any(|p| p[2].sign() != Sign::Positive)
    {
        return Ok(None);
    }
    let mut values = poles
        .chunks_exact(3)
        .map(|p| {
            Ok(vec![
                p[0].mul(&p[2], ctx)?,
                p[1].mul(&p[2], ctx)?,
                p[2].clone(),
            ])
        })
        .collect::<Result<Vec<_>, Reason>>()?;
    // Homogeneous de Casteljau with a common un-divided parameter denominator.
    for count in (1..values.len()).rev() {
        for i in 0..count {
            for k in 0..3 {
                values[i][k] = values[i][k]
                    .mul(&high, ctx)?
                    .add(&values[i + 1][k].mul(&low, ctx)?, ctx)?;
            }
        }
    }
    Ok(Some(values.remove(0)))
}
pub fn rational_bezier_uv_point_identity(
    ctx: &mut PredicateContext<'_>,
    curves: [&[[LeafRef; 3]]; 2],
    parameters: [[LeafRef; 3]; 2],
) -> Result<BezierIdentityDecision, InputError> {
    if curves.iter().any(|p| !(2..=33).contains(&p.len())) {
        return Err(InputError::InvalidInput(
            "UV point identity supports degree 1..32",
        ));
    }
    let refs: Vec<_> = curves
        .iter()
        .flat_map(|p| p.iter().flatten())
        .chain(parameters.iter().flatten())
        .copied()
        .collect();
    let values = refs
        .iter()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let v = exact_inputs(&values, ctx)?;
        let n = curves[0].len() * 3;
        let m = curves[1].len() * 3;
        let Some(a) = point(ctx, &v[..n], &v[n + m..n + m + 3])? else {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        };
        let Some(b) = point(ctx, &v[n..n + m], &v[n + m + 3..])? else {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        };
        for k in 0..2 {
            if a[k]
                .mul(&b[2], ctx)?
                .sub(&b[k].mul(&a[2], ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(BezierIdentity::Different);
            }
        }
        Ok(BezierIdentity::Equal)
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

/// Original parameter rows are [numerator, positive denominator, domain low, domain high].
/// Rational candidates are evaluated homogeneously without division.
pub fn rational_bezier_uv_point_fraction_identity(
    ctx: &mut PredicateContext<'_>,
    curves: [&[[LeafRef; 3]]; 2],
    parameters: [[LeafRef; 4]; 2],
) -> Result<BezierIdentityDecision, InputError> {
    if curves.iter().any(|p| !(2..=33).contains(&p.len())) {
        return Err(InputError::InvalidInput(
            "UV point identity supports degree 1..32",
        ));
    }
    let refs: Vec<_> = curves
        .iter()
        .flat_map(|p| p.iter().flatten())
        .chain(parameters.iter().flatten())
        .copied()
        .collect();
    let mut values = refs
        .iter()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    // exact_inputs normalizes every input by one common positive factor.
    // Include an exact unit so numerator and domain*denominator expressions
    // have the same homogeneous degree after that normalization.
    values.push(AuthoredScalar::Binary64Bits(1_f64.to_bits()));
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let v = exact_inputs(&values, ctx)?;
        let n = curves[0].len() * 3;
        let m = curves[1].len() * 3;
        let mut parameters=Vec::new();
        for i in 0..2 {
            let row=&v[n+m+4*i..n+m+4*i+4];
            if row[1].sign()!=Sign::Positive {return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));}
            parameters.push(vec![row[0].mul(v.last().unwrap(),ctx)?,row[2].mul(&row[1],ctx)?,row[3].mul(&row[1],ctx)?]);
        }
        let Some(a)=point(ctx,&v[..n],&parameters[0])? else {return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));};
        let Some(b)=point(ctx,&v[n..n+m],&parameters[1])? else {return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));};
        for k in 0..2 {
            if a[k]
                .mul(&b[2], ctx)?
                .sub(&b[k].mul(&a[2], ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(BezierIdentity::Different);
            }
        }
        Ok(BezierIdentity::Equal)
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

fn equal_uv(a:&[Expansion],b:&[Expansion],ctx:&mut PredicateContext<'_>)->Result<bool,Reason> {
    for axis in 0..2 {
        if a[axis].mul(&b[2],ctx)?.sub(&b[axis].mul(&a[2],ctx)?,ctx)?.sign()!=Sign::Zero {return Ok(false);}
    }
    Ok(true)
}
fn spline_point(ctx:&mut PredicateContext<'_>,poles:&[Expansion],knots:&[Expansion],degree:usize,t:&Expansion)
    ->Result<Option<Vec<Expansion>>,Reason> {
    let count=poles.len()/3;
    if poles.chunks_exact(3).any(|p|p[2].sign()!=Sign::Positive) {return Ok(None);}
    for pair in knots.windows(2) {if pair[1].sub(&pair[0],ctx)?.sign()==Sign::Negative {return Ok(None);}}
    if knots[count].sub(&knots[degree],ctx)?.sign()!=Sign::Positive
        || t.sub(&knots[degree],ctx)?.sign()==Sign::Negative
        || knots[count].sub(t,ctx)?.sign()==Sign::Negative {return Ok(None);}
    let mut answer:Option<Vec<Expansion>>=None;
    for span in degree..count {
        ctx.charge(1)?;
        if knots[span+1].sub(&knots[span],ctx)?.sign()!=Sign::Positive
            || t.sub(&knots[span],ctx)?.sign()==Sign::Negative
            || knots[span+1].sub(t,ctx)?.sign()==Sign::Negative {continue;}
        let mut rows=poles[3*(span-degree)..3*(span+1)].chunks_exact(3).map(|p|
            Ok((vec![p[0].mul(&p[2],ctx)?,p[1].mul(&p[2],ctx)?,p[2].clone()],Expansion::scalar(1.)))
        ).collect::<Result<Vec<_>,Reason>>()?;
        for r in 1..=degree {for j in (r..=degree).rev() {
            let i=span-degree+j;
            let gap=knots[i+degree-r+1].sub(&knots[i],ctx)?;
            if gap.sign()!=Sign::Positive {return Ok(None);}
            let low=t.sub(&knots[i],ctx)?;let high=knots[i+degree-r+1].sub(t,ctx)?;
            let (a,da)=&rows[j-1];let (b,db)=&rows[j];let mut value=Vec::new();
            for axis in 0..3 {value.push(a[axis].mul(&high,ctx)?.mul(db,ctx)?
                .add(&b[axis].mul(&low,ctx)?.mul(da,ctx)?,ctx)?);}
            let scale=da.mul(db,ctx)?.mul(&gap,ctx)?;
            rows[j]=(value,scale);
        }}
        let value=rows.pop().unwrap().0;
        // A candidate at a repeated knot must agree from both adjacent
        // positive-width spans. Discontinuous or ambiguous points refuse.
        if let Some(old)=&answer {if !equal_uv(old,&value,ctx)? {return Ok(None);}}
        answer=Some(value);
    }
    Ok(answer)
}
/// Exact authored point equality on two rational B-splines. No evaluated
/// Cartesian point or caller-supplied span index supplies proof authority.
pub fn rational_bspline_uv_point_identity(
    ctx:&mut PredicateContext<'_>,curves:[&[[LeafRef;3]];2],knots:[&[LeafRef];2],
    degrees:[usize;2],parameters:[LeafRef;2],
)->Result<BezierIdentityDecision,InputError> {
    if (0..2).any(|i|!(1..=32).contains(&degrees[i]) || curves[i].len()<=degrees[i]
        || curves[i].len()>4096 || knots[i].len()!=curves[i].len()+degrees[i]+1) {
        return Err(InputError::InvalidInput("Invalid UV spline point layout"));
    }
    let refs:Vec<_>=curves.iter().flat_map(|c|c.iter().flatten())
        .chain(knots.iter().flat_map(|k|k.iter())).chain(parameters.iter()).copied().collect();
    let raw=refs.iter().map(|&r|ctx.resolve(r).cloned()).collect::<Result<Vec<AuthoredScalar>,_>>()?;
    let result=(||->Result<BezierIdentity,Reason>{
        ctx.charge(raw.len() as u64)?;let v=exact_inputs(&raw,ctx)?;
        let n=3*curves[0].len();let m=3*curves[1].len();let first=n+m;
        let second=first+knots[0].len();let end=second+knots[1].len();
        let Some(a)=spline_point(ctx,&v[..n],&v[first..second],degrees[0],&v[end])? else {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));};
        let Some(b)=spline_point(ctx,&v[n..n+m],&v[second..end],degrees[1],&v[end+1])? else {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));};
        Ok(if equal_uv(&a,&b,ctx)? {BezierIdentity::Equal}else{BezierIdentity::Different})
    })().and_then(|value|ctx.charge(0).map(|_|value));
    Ok(BezierIdentityDecision{outcome:result.unwrap_or_else(BezierIdentity::Indeterminate),work_used:ctx.work_used(),context:ctx.identity()})
}

#[cfg(test)]
mod fraction_tests {
    use super::*;
    use crate::{Limits,SourceArena,ToleranceContext};
    #[test]
    fn nonlinear_fraction_points_do_not_round_one_third() {
        let values:[f64;23]=[0.,0.,1.,0.5,0.,1.,1.,1.,1.,0.,0.,1.,3.,1.,1.,1.,3.,0.,1.,1.,9.,0.,1.];
        let arena=SourceArena::authored("fraction-points",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).unwrap();
        let a:Vec<_>=(0..3).map(|i|std::array::from_fn(|k|arena.leaf(3*i+k).unwrap())).collect();
        let b:Vec<_>=(0..2).map(|i|std::array::from_fn(|k|arena.leaf(9+3*i+k).unwrap())).collect();
        let parameters=std::array::from_fn(|i|std::array::from_fn(|k|arena.leaf(15+4*i+k).unwrap()));
        let tolerance=ToleranceContext::default_valid();
        let mut ctx=PredicateContext::new(&arena,&tolerance,Limits::default(),None);
        let result=rational_bezier_uv_point_fraction_identity(&mut ctx,[&a,&b],parameters).unwrap();
        assert_eq!(result.outcome,BezierIdentity::Equal);
    }
}
