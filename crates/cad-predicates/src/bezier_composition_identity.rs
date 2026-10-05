//! Exact polynomial identity C(t)=S(P(t)) for rational Bezier definitions.
//! The caller separately proves that P lies inside the admitted surface chart.
use crate::{Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion,
    InputError, LeafRef, PredicateContext, Reason, Sign, exact_inputs};
type Poly = Vec<Expansion>;
// Exact zero leading coefficients do not change a power polynomial. Keep a
// nonempty representation even for zero so convolution dimensions stay valid.
fn trim(p: &mut Poly) {
    while p.len() > 1 && p.last().is_some_and(|x| x.sign() == Sign::Zero) {
        p.pop();
    }
}
fn choose(n: usize,k: usize)->u64 {(0..k).fold(1,|v,i|v*(n-i) as u64/(i+1) as u64)}
fn add_scaled(out: &mut Poly, a: &Poly, scale: &Expansion, ctx: &mut PredicateContext<'_>)->Result<(),Reason>{
    out.resize_with(out.len().max(a.len()),||Expansion::scalar(0.));
    if scale.sign()==Sign::Zero {return ctx.charge(0);}
    for (i,x) in a.iter().enumerate(){
        if x.sign()!=Sign::Zero {out[i]=out[i].add(&x.mul(scale,ctx)?,ctx)?;}
    }
    Ok(())
}
fn mul(a:&Poly,b:&Poly,ctx:&mut PredicateContext<'_>)->Result<Poly,Reason>{
    let mut out=vec![Expansion::scalar(0.);a.len()+b.len()-1];
    for (i,x) in a.iter().enumerate(){for (j,y) in b.iter().enumerate(){
        if x.sign()==Sign::Zero || y.sign()==Sign::Zero {continue;}
        out[i+j]=out[i+j].add(&x.mul(y,ctx)?,ctx)?;
    }}
    trim(&mut out);
    Ok(out)
}
/// Bernstein coefficients to power coefficients. All factors are integers;
/// no rounded intermediate control points or rational division is introduced.
fn polynomial(values:&[Expansion],ctx:&mut PredicateContext<'_>)->Result<Poly,Reason>{
    let n=values.len()-1;let mut out=vec![Expansion::scalar(0.);n+1];
    for (i,value) in values.iter().enumerate(){for k in i..=n {
        let factor=Expansion::integer(choose(n,i)*choose(n-i,k-i),ctx)?;
        let term=value.mul(&factor,ctx)?;
        out[k]=if (k-i)%2==0 {out[k].add(&term,ctx)?}else{out[k].sub(&term,ctx)?};
    }}
    trim(&mut out);
    Ok(out)
}
fn homogeneous<const N:usize>(points:&[[Expansion;N]],ctx:&mut PredicateContext<'_>)->Result<Vec<Poly>,Reason>{
    (0..N).map(|axis|{
        let values=points.iter().map(|p|if axis==N-1 {Ok(p[axis].clone())}else{p[axis].mul(&p[N-1],ctx)}).collect::<Result<Vec<_>,_>>()?;
        polynomial(&values,ctx)
    }).collect()
}
/// Euclidean XYZ+weight for C and S, UV+weight for P. Surface domain bounds
/// are authored leaves. Degrees: C 1..32; P and each surface axis 1..8.
/// Reversed traversal is expressed by reversing C's control sequence.
pub fn rational_bezier_composition_identity(
    ctx:&mut PredicateContext<'_>,curve:&[[LeafRef;4]],pcurve:&[[LeafRef;3]],
    surface:&[Vec<[LeafRef;4]>],domain:[[LeafRef;2];2],
)->Result<BezierIdentityDecision,InputError>{
    if !(2..=33).contains(&curve.len()) || !(2..=9).contains(&pcurve.len())
        || !(2..=9).contains(&surface.len()) || surface.first().is_none_or(|r|!(2..=9).contains(&r.len()))
        || surface.iter().any(|r|r.len()!=surface[0].len()) {
        return Err(InputError::InvalidInput("Bezier composition dimensions exceed the supported degree bounds"));
    }
    let refs:Vec<_>=curve.iter().flatten().chain(pcurve.iter().flatten())
        .chain(surface.iter().flatten().flatten()).chain(domain.iter().flatten()).copied().collect();
    let values=refs.iter().map(|&r|ctx.resolve(r).cloned()).collect::<Result<Vec<AuthoredScalar>,_>>()?;
    let result=(||->Result<BezierIdentity,Reason>{
        ctx.charge(values.len() as u64)?;
        let values=exact_inputs(&values,ctx)?;
        let (nc,np,nu,nv)=(curve.len(),pcurve.len(),surface.len(),surface[0].len());
        let c:Vec<[Expansion;4]>=values[..4*nc].chunks_exact(4).map(|x|std::array::from_fn(|i|x[i].clone())).collect();
        let p:Vec<[Expansion;3]>=values[4*nc..4*nc+3*np].chunks_exact(3).map(|x|std::array::from_fn(|i|x[i].clone())).collect();
        let start=4*nc+3*np;
        let s:Vec<[Expansion;4]>=values[start..start+4*nu*nv].chunks_exact(4).map(|x|std::array::from_fn(|i|x[i].clone())).collect();
        let d=&values[start+4*nu*nv..];
        if c.iter().chain(&s).any(|x|x[3].sign()!=Sign::Positive) || p.iter().any(|x|x[2].sign()!=Sign::Positive)
            || d[1].sub(&d[0],ctx)?.sign()!=Sign::Positive || d[3].sub(&d[2],ctx)?.sign()!=Sign::Positive {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        // An exact affine bilinear chart maps each rational pole with the
        // same parameter weights. Prove the chart and pole identities using
        // cross-multiplied domain widths; no rounded inverse is introduced.
        if nu==2 && nv==2 && nc==np {
            let mut eligible=true;
            for pole in &s {
                eligible &= pole[3].sub(&s[0][3],ctx)?.sign()==Sign::Zero;
            }
            for i in 0..nc {
                eligible &= c[i][3].sub(&p[i][2],ctx)?.sign()==Sign::Zero;
            }
            for axis in 0..3 {
                let mixed=s[3][axis].sub(&s[2][axis],ctx)?.sub(&s[1][axis],ctx)?.add(&s[0][axis],ctx)?;
                eligible &= mixed.sign()==Sign::Zero;
            }
            if eligible {
                let wu=d[1].sub(&d[0],ctx)?;
                let wv=d[3].sub(&d[2],ctx)?;
                let area=wu.mul(&wv,ctx)?;
                for axis in 0..3 {
                    let u=s[2][axis].sub(&s[0][axis],ctx)?.mul(&wv,ctx)?;
                    let v=s[1][axis].sub(&s[0][axis],ctx)?.mul(&wu,ctx)?;
                    for i in 0..nc {
                        let left=c[i][axis].sub(&s[0][axis],ctx)?.mul(&area,ctx)?;
                        let right=p[i][0].sub(&d[0],ctx)?.mul(&u,ctx)?
                            .add(&p[i][1].sub(&d[2],ctx)?.mul(&v,ctx)?,ctx)?;
                        if left.sub(&right,ctx)?.sign()!=Sign::Zero {
                            return Ok(BezierIdentity::Different);
                        }
                    }
                }
                return Ok(BezierIdentity::Equal);
            }
        }
        // Positive rational bases preserve a coordinate that is exactly the
        // same constant at every curve and surface pole. Compare exact source
        // expansions, never a tolerance; disagreement keeps the full predicate.
        let mut constant_axes=[false;3];
        for axis in 0..3 {
            let first=&c[0][axis];
            let mut same=true;
            for pole in c.iter().chain(&s) {
                if pole[axis].sub(first,ctx)?.sign()!=Sign::Zero {same=false;break;}
            }
            constant_axes[axis]=same;
        }
        let c=homogeneous(&c,ctx)?;let p=homogeneous(&p,ctx)?;
        let mut basis=Vec::new();
        for axis in 0..2 {
            let mut low=p[axis].clone();add_scaled(&mut low,&p[2],&Expansion::scalar(0.).sub(&d[2*axis],ctx)?,ctx)?;
            let mut high=Vec::new();add_scaled(&mut high,&p[2],&d[2*axis+1],ctx)?;add_scaled(&mut high,&p[axis],&Expansion::scalar(-1.),ctx)?;
            let degree=if axis==0 {nu-1}else{nv-1};
            // Reuse exact powers across Bernstein basis terms. Recomputing
            // each prefix spends the shared proof budget on identical work.
            let mut lows=vec![vec![Expansion::scalar(1.)]];
            let mut highs=vec![vec![Expansion::scalar(1.)]];
            for exponent in 1..=degree {
                lows.push(mul(&lows[exponent-1],&low,ctx)?);
                highs.push(mul(&highs[exponent-1],&high,ctx)?);
            }
            let mut parts=Vec::new();
            for i in 0..=degree {
                let product=mul(&lows[i],&highs[degree-i],ctx)?;let factor=Expansion::integer(choose(degree,i),ctx)?;
                let mut part=Vec::new();add_scaled(&mut part,&product,&factor,ctx)?;parts.push(part);
            }
            basis.push(parts);
        }
        let mut composed:Vec<Poly>=(0..4).map(|_|Vec::new()).collect();
        for i in 0..nu {for j in 0..nv {
            let basis=mul(&basis[0][i],&basis[1][j],ctx)?;let control=&s[i*nv+j];
            for axis in 0..4 {
                if axis<3 && constant_axes[axis] {continue;}
                let factor=if axis==3 {control[3].clone()}else{control[axis].mul(&control[3],ctx)?};
                add_scaled(&mut composed[axis],&basis,&factor,ctx)?;
            }
        }}
        for axis in 0..3 {
            if constant_axes[axis] {continue;}
            let a=mul(&c[axis],&composed[3],ctx)?;let b=mul(&composed[axis],&c[3],ctx)?;
            let zero=Expansion::scalar(0.);
            for i in 0..a.len().max(b.len()) {
                if a.get(i).unwrap_or(&zero).sub(b.get(i).unwrap_or(&zero),ctx)?.sign()!=Sign::Zero {return Ok(BezierIdentity::Different);}
            }
        }
        Ok(BezierIdentity::Equal)
    })().and_then(|v|ctx.charge(0).map(|_|v));
    Ok(BezierIdentityDecision{outcome:result.unwrap_or_else(BezierIdentity::Indeterminate),work_used:ctx.work_used(),context:ctx.identity()})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits,SourceArena,ToleranceContext};
    fn run(c:&[[f64;4]],p:&[[f64;3]],domain:[[f64;2];2],limits:Limits)->BezierIdentity {
        run_weighted(c,p,domain,limits,1.)
    }
    fn run_weighted(c:&[[f64;4]],p:&[[f64;3]],domain:[[f64;2];2],limits:Limits,w:f64)->BezierIdentity {
        // Separable rational bilinear graph, over the supplied chart domain.
        let s=[[[0.,0.,0.,1.],[0.,1.,0.,w]],[[1.,0.,0.,w],[1.,1.,1.,w*w]]];
        let source=SourceArena::authored("composition-test",1,c.iter().flatten().chain(p.iter().flatten())
            .chain(s.iter().flatten().flatten()).chain(domain.iter().flatten())
            .map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect()).unwrap();
        let mut next=0;
        let cc:Vec<[LeafRef;4]>=(0..c.len()).map(|_|std::array::from_fn(|_|{let r=source.leaf(next).unwrap();next+=1;r})).collect();
        let pp:Vec<[LeafRef;3]>=(0..p.len()).map(|_|std::array::from_fn(|_|{let r=source.leaf(next).unwrap();next+=1;r})).collect();
        let ss:Vec<Vec<[LeafRef;4]>>=(0..2).map(|_|(0..2).map(|_|std::array::from_fn(|_|{let r=source.leaf(next).unwrap();next+=1;r})).collect()).collect();
        let dd=std::array::from_fn(|_|std::array::from_fn(|_|{let r=source.leaf(next).unwrap();next+=1;r}));
        let tolerance=ToleranceContext::default_valid();let mut ctx=PredicateContext::new(&source,&tolerance,limits,None);
        rational_bezier_composition_identity(&mut ctx,&cc,&pp,&ss,dd).unwrap().outcome
    }
    #[test]
    fn diagonal_graph_composition_is_exact_for_polynomial_and_rational_pcurves() {
        for weight in [1.,2.] {
            let c=[[0.,0.,0.,1.],[0.5,0.5,0.,weight],[1.,1.,1.,weight*weight]];
            for d in [[[0.,1.],[0.,1.]],[[2.,4.],[3.,7.]]] {
                let p=[[d[0][0],d[1][0],1.],[d[0][1],d[1][1],weight]];
                assert_eq!(run(&c,&p,d,Limits::default()),BezierIdentity::Equal);
                let mut wrong=c;wrong[1][2]=f64::EPSILON;
                assert_eq!(run(&wrong,&p,d,Limits::default()),BezierIdentity::Different);
                let reversed_c=[c[2],c[1],c[0]];let reversed_p=[p[1],p[0]];
                assert_eq!(run(&reversed_c,&reversed_p,d,Limits::default()),BezierIdentity::Equal);
                assert_eq!(run(&reversed_c,&p,d,Limits::default()),BezierIdentity::Different);
            }
        }
    }
    #[test]
    fn reduced_power_degrees_compare_missing_coefficients_as_zero() {
        let p=[[0.,0.,1.],[1.,0.,1.]];
        let d=[[0.,1.],[0.,1.]];
        let c=[[0.,0.,0.,1.],[0.5,0.,0.,1.],[1.,0.,0.,1.]];
        assert_eq!(run(&c,&p,d,Limits::default()),BezierIdentity::Equal);
        let mut bent=c;bent[1][0]=0.75;
        assert_eq!(run(&bent,&p,d,Limits::default()),BezierIdentity::Different);
        let short=[[0.,0.,0.,1.],[1.,1.,1.,1.]];
        let diagonal=[[0.,0.,1.],[1.,1.,1.]];
        assert_eq!(run(&short,&diagonal,d,Limits::default()),BezierIdentity::Different);
    }
    #[test]
    fn rational_surface_weights_participate_in_the_exact_composition() {
        let c=[[0.,0.,0.,1.],[0.5,0.5,0.,2.],[1.,1.,1.,4.]];
        let p=[[0.,0.,1.],[1.,1.,1.]];let d=[[0.,1.],[0.,1.]];
        assert_eq!(run_weighted(&c,&p,d,Limits::default(),2.),BezierIdentity::Equal);
        assert_eq!(run_weighted(&c,&p,d,Limits::default(),2_f64.next_up()),BezierIdentity::Different);
    }
    #[test]
    fn missing_weights_and_work_do_not_establish_identity() {
        let c=[[0.,0.,0.,1.],[0.5,0.5,0.,1.],[1.,1.,1.,1.]];
        let p=[[0.,0.,1.],[1.,1.,1.]];let d=[[0.,1.],[0.,1.]];
        assert!(matches!(run(&c,&p,d,Limits{max_work:0,..Limits::default()}),BezierIdentity::Indeterminate(_)));
        let mut bad=p;bad[1][2]=0.;
        assert_eq!(run(&c,&bad,d,Limits::default()),BezierIdentity::Indeterminate(Reason::MissingProof));
        assert_eq!(run(&c,&p,[[1.,0.],[0.,1.]],Limits::default()),BezierIdentity::Indeterminate(Reason::MissingProof));
    }
}

/// Exact formal identity that S(P(t)) lies in the plane through three source
/// anchors. Chart membership and denominator positivity are separate proofs.
/// The plane normal and every composition coefficient are exact expansions.
pub fn rational_bezier_composition_plane_identity(
    ctx: &mut PredicateContext<'_>, pcurve: &[[LeafRef;3]],
    surface: &[Vec<[LeafRef;4]>], domain: [[LeafRef;2];2], plane: [[LeafRef;3];3],
) -> Result<BezierIdentityDecision,InputError> {
    if !(2..=9).contains(&pcurve.len()) || !(2..=9).contains(&surface.len())
        || surface.first().is_none_or(|r| !(2..=9).contains(&r.len()))
        || surface.iter().any(|r| r.len()!=surface[0].len()) {
        return Err(InputError::InvalidInput("Plane composition dimensions exceed supported degree bounds"));
    }
    let refs:Vec<_>=pcurve.iter().flatten().chain(surface.iter().flatten().flatten())
        .chain(domain.iter().flatten()).chain(plane.iter().flatten()).copied().collect();
    let values=refs.iter().map(|&r|ctx.resolve(r).cloned()).collect::<Result<Vec<AuthoredScalar>,_>>()?;
    let result=(|| -> Result<BezierIdentity,Reason> {
        ctx.charge(values.len() as u64)?;
        let values=exact_inputs(&values,ctx)?;
        let (np,nu,nv)=(pcurve.len(),surface.len(),surface[0].len());
        let p:Vec<[Expansion;3]>=values[..3*np].chunks_exact(3).map(|v|std::array::from_fn(|k|v[k].clone())).collect();
        let s:Vec<[Expansion;4]>=values[3*np..3*np+4*nu*nv].chunks_exact(4).map(|v|std::array::from_fn(|k|v[k].clone())).collect();
        let d=&values[3*np+4*nu*nv..3*np+4*nu*nv+4];
        let anchors=&values[3*np+4*nu*nv+4..];
        if p.iter().any(|v|v[2].sign()!=Sign::Positive) || s.iter().any(|v|v[3].sign()!=Sign::Positive)
            || d[1].sub(&d[0],ctx)?.sign()!=Sign::Positive || d[3].sub(&d[2],ctx)?.sign()!=Sign::Positive {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        let a=(0..3).map(|k|anchors[3+k].sub(&anchors[k],ctx)).collect::<Result<Vec<_>,_>>()?;
        let b=(0..3).map(|k|anchors[6+k].sub(&anchors[k],ctx)).collect::<Result<Vec<_>,_>>()?;
        let normal=(0..3).map(|k|a[(k+1)%3].mul(&b[(k+2)%3],ctx)?.sub(&a[(k+2)%3].mul(&b[(k+1)%3],ctx)?,ctx)).collect::<Result<Vec<_>,_>>()?;
        if normal.iter().all(|v|v.sign()==Sign::Zero) {return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));}
        let mut factors=Vec::new();
        for pole in &s {
            let mut signed=Expansion::scalar(0.);
            for k in 0..3 { signed=signed.add(&pole[k].sub(&anchors[k],ctx)?.mul(&normal[k],ctx)?,ctx)?; }
            factors.push(signed.mul(&pole[3],ctx)?);
        }
        if factors.iter().all(|v|v.sign()==Sign::Zero) { return Ok(BezierIdentity::Equal); }
        let p=homogeneous(&p,ctx)?;
        let mut basis=Vec::new();
        for axis in 0..2 {
            let mut low=p[axis].clone();
            add_scaled(&mut low,&p[2],&Expansion::scalar(0.).sub(&d[2*axis],ctx)?,ctx)?;
            let mut high=Vec::new();
            add_scaled(&mut high,&p[2],&d[2*axis+1],ctx)?;
            add_scaled(&mut high,&p[axis],&Expansion::scalar(-1.),ctx)?;
            let degree=if axis==0 {nu-1} else {nv-1};
            let mut lows=vec![vec![Expansion::scalar(1.)]];
            let mut highs=vec![vec![Expansion::scalar(1.)]];
            for i in 1..=degree {lows.push(mul(&lows[i-1],&low,ctx)?);highs.push(mul(&highs[i-1],&high,ctx)?);}
            let mut parts=Vec::new();
            for i in 0..=degree {
                let product=mul(&lows[i],&highs[degree-i],ctx)?;
                let mut part=Vec::new();
                add_scaled(&mut part,&product,&Expansion::integer(choose(degree,i),ctx)?,ctx)?;
                parts.push(part);
            }
            basis.push(parts);
        }
        let mut signed=Vec::new();
        for i in 0..nu {for j in 0..nv {
            if factors[i*nv+j].sign()==Sign::Zero {continue;}
            add_scaled(&mut signed,&mul(&basis[0][i],&basis[1][j],ctx)?,&factors[i*nv+j],ctx)?;
        }}
        Ok(if signed.iter().all(|v|v.sign()==Sign::Zero) {BezierIdentity::Equal} else {BezierIdentity::Different})
    })().and_then(|v|ctx.charge(0).map(|_|v));
    Ok(BezierIdentityDecision {outcome:result.unwrap_or_else(BezierIdentity::Indeterminate),work_used:ctx.work_used(),context:ctx.identity()})
}

/// Exact plane membership of C(q), where q is an optionally reflected fraction
/// (t-lo)/(hi-lo) from another original source domain. Evaluation stays entirely
/// homogeneous; neither q nor a Cartesian point is rounded.
pub fn rational_bezier_plane_point_identity(
    ctx:&mut PredicateContext<'_>,curve:&[[LeafRef;4]],plane:[[LeafRef;3];3],
    parameter:[LeafRef;3],reversed:bool,
) -> Result<BezierIdentityDecision,InputError> {
    if !(2..=33).contains(&curve.len()) {
        return Err(InputError::InvalidInput("Plane point curve degree exceeds supported bounds"));
    }
    let refs:Vec<_>=curve.iter().flatten().chain(plane.iter().flatten()).chain(parameter.iter()).copied().collect();
    let values=refs.iter().map(|&r|ctx.resolve(r).cloned()).collect::<Result<Vec<AuthoredScalar>,_>>()?;
    let result=(|| -> Result<BezierIdentity,Reason> {
        ctx.charge(values.len() as u64)?;
        let v=exact_inputs(&values,ctx)?;
        let points=&v[..4*curve.len()];let anchors=&v[4*curve.len()..4*curve.len()+9];
        let parameter=&v[4*curve.len()+9..];
        if points.chunks_exact(4).any(|p|p[3].sign()!=Sign::Positive) {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        let low=parameter[0].sub(&parameter[1],ctx)?;
        let high=parameter[2].sub(&parameter[0],ctx)?;
        if parameter[2].sub(&parameter[1],ctx)?.sign()!=Sign::Positive
            || low.sign()==Sign::Negative || high.sign()==Sign::Negative {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        let a=(0..3).map(|k|anchors[3+k].sub(&anchors[k],ctx)).collect::<Result<Vec<_>,_>>()?;
        let b=(0..3).map(|k|anchors[6+k].sub(&anchors[k],ctx)).collect::<Result<Vec<_>,_>>()?;
        let normal=(0..3).map(|k|a[(k+1)%3].mul(&b[(k+2)%3],ctx)?.sub(&a[(k+2)%3].mul(&b[(k+1)%3],ctx)?,ctx)).collect::<Result<Vec<_>,_>>()?;
        if normal.iter().all(|x|x.sign()==Sign::Zero) { return Ok(BezierIdentity::Indeterminate(Reason::MissingProof)); }
        let degree=curve.len()-1;
        let (numerator,complement)=if reversed {(high,low)} else {(low,high)};
        let mut powers=vec![Expansion::scalar(1.)];let mut complements=vec![Expansion::scalar(1.)];
        for i in 1..=degree {powers.push(powers[i-1].mul(&numerator,ctx)?);complements.push(complements[i-1].mul(&complement,ctx)?);}
        let mut total=Expansion::scalar(0.);
        for (i,p) in points.chunks_exact(4).enumerate() {
            let mut signed=Expansion::scalar(0.);
            for k in 0..3 {signed=signed.add(&p[k].sub(&anchors[k],ctx)?.mul(&normal[k],ctx)?,ctx)?;}
            let factor=signed.mul(&p[3],ctx)?.mul(&Expansion::integer(choose(degree,i),ctx)?,ctx)?;
            let term=factor.mul(&powers[i],ctx)?.mul(&complements[degree-i],ctx)?;
            total=total.add(&term,ctx)?;
        }
        Ok(if total.sign()==Sign::Zero {BezierIdentity::Equal} else {BezierIdentity::Different})
    })().and_then(|v|ctx.charge(0).map(|_|v));
    Ok(BezierIdentityDecision {outcome:result.unwrap_or_else(BezierIdentity::Indeterminate),work_used:ctx.work_used(),context:ctx.identity()})
}
