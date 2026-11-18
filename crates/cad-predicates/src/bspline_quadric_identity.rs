//! Original clamped tensor B-spline quadric membership, without rounded poles.
use crate::{Algebra,AuthoredScalar,BezierIdentity,BezierIdentityDecision,Expansion,InputError,LeafRef,PredicateContext,Reason,Sign};
use crate::bezier_quadric_identity::{power,identity};

// Batch homogeneous blossom restriction. Every active blossom and every
// other-axis channel retains ONE common gauge throughout each iteration.
// Divisions are cleared using the product of all de Boor denominators.
fn extract(base:Vec<Vec<Expansion>>,knots:&[Expansion],p:usize,span:usize,ctx:&mut PredicateContext<'_>)->Result<Vec<Vec<Expansion>>,Reason> {
    let channels=base[0].len();let mut rows=vec![base;p+1];
    for r in 1..=p {
        let m=p-r+1;let mut coefficients=Vec::new();
        for j in r..=p {
            let i=span-p+j;let upper=&knots[i+p-r+1];let lower=&knots[i];
            coefficients.extend([upper.sub(lower,ctx)?,upper.sub(&knots[span],ctx)?,knots[span].sub(lower,ctx)?,upper.sub(&knots[span+1],ctx)?,knots[span+1].sub(lower,ctx)?]);
        }
        Expansion::normalize_common(&mut coefficients,ctx)?;
        let mut prefix=vec![Expansion::scalar(1.)];
        for j in 0..m {
            if coefficients[5*j].sign()!=Sign::Positive {return Err(Reason::MissingProof);}
            prefix.push(prefix[j].mul(&coefficients[5*j],ctx)?);
        }
        let mut suffix=vec![Expansion::scalar(1.);m+1];
        for j in (0..m).rev() {suffix[j]=suffix[j+1].mul(&coefficients[5*j],ctx)?;}
        for (blossom,row) in rows.iter_mut().enumerate() {
            let end=usize::from(r>p-blossom);
            for j in (r..=p).rev() {
                let c=&coefficients[5*(j-r)..5*(j-r)+5];let omit=prefix[j-r].mul(&suffix[j-r+1],ctx)?;
                for axis in 0..channels {
                    row[j][axis]=row[j-1][axis].mul(&c[1+2*end],ctx)?.add(&row[j][axis].mul(&c[2+2*end],ctx)?,ctx)?.mul(&omit,ctx)?;
                }
            }
        }
        let mut active=rows.iter().flat_map(|row|row[r..].iter().flatten().cloned()).collect::<Vec<_>>();
        Expansion::normalize_common(&mut active,ctx)?;
        let mut active=active.into_iter();
        for row in &mut rows {for pole in &mut row[r..] {for value in pole {*value=active.next().unwrap();}}}
    }
    Ok(rows.into_iter().map(|row|row[p].clone()).collect())
}
fn valid_basis(k:&[f64],p:usize,n:usize)->bool {
    if k.iter().any(|x|!x.is_finite())||k.windows(2).any(|w|w[0]>w[1])
        ||k[p]>=k[n]||!k[..=p].iter().all(|x|*x==k[p])||!k[n..].iter().all(|x|*x==k[n]) {return false;}
    let mut run=1;
    for w in k.windows(2) {run=if w[0]==w[1] {run+1}else{1};if run>p+1 {return false;}}
    true
}
/// Exact original positive-weight, nonperiodic, clamped tensor membership in
/// H^T Q H + sum(a*b)*W²=0. Degree 1..8, 256 controls/axis as in the native
/// surface owner. Literal binary64 leaves only. ALL nonempty spans share the
/// caller's existing context; neither extraction nor each span resets work.
/// No parameter regularity, correspondence, topology or solid credit follows.
pub fn binary_bspline_quadric_with_constant_products_identity(
    ctx:&mut PredicateContext<'_>,surface:&[Vec<[LeafRef;4]>],degrees:[usize;2],knots:[&[LeafRef];2],q:[[LeafRef;4];4],products:&[[LeafRef;2]],
)->Result<BezierIdentityDecision,InputError> {
    let [p,v]=degrees;let nu=surface.len();let nv=surface.first().map_or(0,Vec::len);
    if !(1..=8).contains(&p)||!(1..=8).contains(&v)||!(p+1..=256).contains(&nu)||!(v+1..=256).contains(&nv)
        ||surface.iter().any(|row|row.len()!=nv)||knots[0].len()!=nu+p+1||knots[1].len()!=nv+v+1||products.len()>16 {
        return Err(InputError::InvalidInput("Invalid original tensor B-spline quadric census"));
    }
    let refs=surface.iter().flatten().flatten().chain(q.iter().flatten()).chain(products.iter().flatten()).chain(knots[0]).chain(knots[1]);
    let raw=refs.map(|&r|match ctx.resolve(r)? {AuthoredScalar::Binary64Bits(b)=>Ok(f64::from_bits(*b)),_=>Err(InputError::InvalidInput("Quadric membership requires literal binary64 source"))}).collect::<Result<Vec<_>,_>>()?;
    let len=nu*nv*4;let ku_start=len+16+2*products.len();let kv_start=ku_start+knots[0].len();
    if !valid_basis(&raw[ku_start..kv_start],p,nu)||!valid_basis(&raw[kv_start..],v,nv) {
        return Err(InputError::InvalidInput("Quadric membership requires a valid clamped source basis"));
    }
    let result=(||->Result<BezierIdentity,Reason>{
        ctx.charge(raw.len() as u64)?;
        if raw[..len].chunks_exact(4).any(|a|a[3]<=0.) {return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));}
        let mut coefficients=raw[len..len+16].iter().map(|&x|Expansion::scalar(x)).collect::<Vec<_>>();
        for pair in raw[len+16..ku_start].chunks_exact(2) {
            coefficients[15]=coefficients[15].add(&Expansion::scalar(pair[0]).mul(&Expansion::scalar(pair[1]),ctx)?,ctx)?;
        }
        let ku=raw[ku_start..kv_start].iter().map(|&x|Expansion::scalar(x)).collect::<Vec<_>>();
        let kv=raw[kv_start..].iter().map(|&x|Expansion::scalar(x)).collect::<Vec<_>>();
        for iu in p..nu {
            ctx.charge(1)?;if raw[ku_start+iu]>=raw[ku_start+iu+1] {continue;}
            // Restrict the independent u basis ONCE for all original v
            // channels, retaining one common homogeneous gauge across them.
            let mut base=Vec::new();
            for i in iu-p..=iu {
                let mut row=Vec::new();
                for j in 0..nv {let a=&raw[(i*nv+j)*4..(i*nv+j)*4+4];let w=Expansion::scalar(a[3]);for &x in &a[..3]{row.push(Expansion::scalar(x).mul(&w,ctx)?);}row.push(w);}
                base.push(row);
            }
            let u=extract(base,&ku,p,iu,ctx)?;
            for iv in v..nv {
                ctx.charge(1)?;if raw[kv_start+iv]>=raw[kv_start+iv+1]{continue;}
                let base=(iv-v..=iv).map(|j|(0..=p).flat_map(|i|u[i][4*j..4*j+4].iter().cloned()).collect()).collect();
                let uv=extract(base,&kv,v,iv,ctx)?;
                let mut h=Vec::new();
                for axis in 0..4 {
                    let controls=(0..=p).map(|i|(0..=v).map(|j|uv[j][4*i+axis].clone()).collect()).collect();
                    h.push(power(&controls,ctx)?);
                }
                let answer=identity(&h,&coefficients,ctx)?;
                if answer!=BezierIdentity::Equal {return Ok(answer);}
            }
        }
        Ok(BezierIdentity::Equal)
    })();
    Ok(BezierIdentityDecision{outcome:result.unwrap_or_else(BezierIdentity::Indeterminate),work_used:ctx.work_used(),context:ctx.identity()})
}

#[cfg(test)]
#[path="bspline_quadric_identity_tests.rs"]
mod tests;
