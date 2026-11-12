//! Gale-Nikaido (1965), Theorem 4: a differentiable map of a CLOSED
//! rectangle whose Jacobian is a P-matrix everywhere is globally one-to-one.
//! https://doi.org/10.1007/BF01360282
//! This kernel MUST only be called after the source owner proves C1 on the
//! whole box. It does not apply to arbitrary C0 joins or periodic quotients.
//! All SEVEN principal minors are proved outward on a complete source cover,
//! under ONE constant output transform and ONE positive coordinate scale.
use super::*;
fn principal_positive(columns:[V3;3],metric:[[f64;3];3])->Result<bool> {
    let a=projected(columns,metric)?;
    for i in 0..3 {if a[i][i].lo<=0. {return Ok(false);}}
    for i in 0..3 {for j in i+1..3 {
        let minor=a[i][i].mul(a[j][j])?.sub(a[j][i].mul(a[i][j])?)?;
        if minor.lo<=0. {return Ok(false);}
    }}
    Ok(iv::dot_tight(a[0],iv::cross(a[1],a[2])?)?.lo>0.)
}
/// PRIVATE source-owner entry: the caller owns differentiability premises.
/// No sample or separate local P-matrix proof supplies global credit.
pub(crate) fn certify(
    mut enclose:impl FnMut(Box3,&mut IdealVolumeReport,usize)->Result<Enclosed>,
    projection:[[f64;3];3],out:&mut IdealVolumeReport,max:usize,
)->Result<bool> {
    let mut cache=BTreeMap::new();
    let mut query=|b,out:&mut IdealVolumeReport|->Result<Enclosed>{
        if let Some(e)=cache.get(&key(b)) {return Ok(*e);}
        let e=enclose(b,out,max)?;cache.insert(key(b),e);Ok(e)
    };
    if out.cells==max {return Ok(false);}out.cells+=1;
    let Enclosed::Value(center)=query([[0.5;2];3],out)? else{return Ok(false);};
    let reference=projected(center.columns,projection)?;
    let Some(scale)=coordinate_scale(reference) else{return Ok(false);};
    // The caller's output coordinates may already define a P-map even when
    // a midpoint inverse destroys its principal-minor signs. Both are only
    // proposals; each must independently cover the whole original box.
    let identity=[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
    let mut metrics=vec![identity];
    if let Some(metric)=inverse_proposal(scaled(reference,scale)?) {metrics.push(metric);}
    for metric in metrics {
        if cover(projection,scale,metric,&mut query,out,max)? {return Ok(true);}
        if out.cells==max {return Ok(false);}
    }
    Ok(false)
}
fn cover(projection:[[f64;3];3],scale:[f64;3],metric:[[f64;3];3],
    query:&mut impl FnMut(Box3,&mut IdealVolumeReport)->Result<Enclosed>,
    out:&mut IdealVolumeReport,max:usize)->Result<bool> {
    let root=[[0.,1.];3];
    let chart=refinement::MetricChart{projection,scale,metric,predicate:principal_positive};
    if !refinement::promising(&[root],chart,query,out,max)? {return Ok(false);}
    let mut pending=vec![root];let mut visits=0;
    while let Some(b)=pending.pop() {
        if visits==1024 || out.cells==max {return Ok(false);}
        visits+=1;out.cells+=1;
        let e=query(b,out)?;
        if let Enclosed::Value(e)=e {
            if principal_positive(scaled(projected(e.columns,projection)?,scale)?,metric)? {continue;}
        }
        let choice=match e {
            Enclosed::Unavailable(Some(axis)) if split_axis(b,axis).is_some()=>
                refinement::Choice::Split{axis,complete:[false;2]},
            _=>refinement::axis(b,chart,query,out,max)?,
        };
        let (axis,complete)=match choice {
            refinement::Choice::Covered=>continue,
            refinement::Choice::Unavailable=>return Ok(false),
            refinement::Choice::Split{axis,complete}=>(axis,complete),
        };
        let Some(children)=split_axis(b,axis) else{return Ok(false);};
        for (child,proved) in children.into_iter().zip(complete) {if !proved {pending.push(child);}}
    }
    // The same transformed Jacobian is a P-matrix on EVERY source point.
    // Its positive full minor also proves the fixed transforms nonsingular.
    // Gale-Nikaido gives global injectivity. Compactness gives the domain
    // homeomorphism; the nonzero Jacobian gives regularity. Trims stay separate.
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_complete_c1_cover_cannot_drop_unproved_folded_siblings() {
        let projection=[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
        let run=|max,folded| {
            let mut out=IdealVolumeReport{embedding_certified:false,regularity_certified:false,
                parameter_topology_preserved:false,cells:0,exact_work:0,boxes:0,
                monotonicity_lower:None,projection,reason:None};
            let source=|b:Box3,out:&mut IdealVolumeReport,max:usize|->Result<Enclosed>{
                if out.cells==max {return Ok(Enclosed::Unavailable(None));}out.cells+=1;
                let [u,v,t]=b.map(|d|I::new(d[0],d[1]).unwrap());
                let mut columns=projection.map(|c|c.map(I::point));let mut x=u;
                if folded {
                    // A C-infinity polynomial with f'(0)=f'(1/2)=f'(1)=1,
                    // but f'(1/4)=-1/2. Every initial rejection probe passes;
                    // the full cover must still reject the interior fold.
                    let u2=u.mul(u)?;let u3=u2.mul(u)?;let u4=u3.mul(u)?;let u5=u4.mul(u)?;
                    x=u.add(I::point(128.).mul(u5)?.div(I::point(5.))?)?
                        .sub(I::point(64.).mul(u4)?)?
                        .add(I::point(160.).mul(u3)?.div(I::point(3.))?)?
                        .sub(I::point(16.).mul(u2)?)?;
                    let q=u.sub(I::point(0.5))?;
                    columns[0][0]=I::point(1.).sub(I::point(128.).mul(u)?
                        .mul(I::point(1.).sub(u)?)?.mul(q.mul(q)?)?)?;
                }
                Ok(Enclosed::Value(cell::Enclosure{value:[x,v,t],columns}))
            };
            let result=certify(source,projection,&mut out,max).unwrap();(result,out.cells)
        };
        let (ok,work)=run(10000,false);assert!(ok);
        assert!(run(work,false).0);let (ok,spent)=run(work-1,false);assert!(!ok && spent<work);
        let (ok,spent)=run(10000,true);assert!(!ok && spent<=10000);
    }
    #[test]
    fn p_matrix_requires_all_principal_minors_and_is_not_an_spd_alias() {
        let p=[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
        let shear=[[1.,0.,0.],[10.,1.,0.],[0.,0.,1.]].map(|c|c.map(I::point));
        assert!(principal_positive(shear,p).unwrap());
        assert!(!symmetric_positive(shear,p).unwrap());
        // Positive diagonal, leading 2x2 minor and full determinant; the
        // OTHER principal 2x2 minor is -3, so this is not a P-matrix.
        let bad=[[1.,2.,2.],[0.,1.,2.],[2.,0.,1.]].map(|c|c.map(I::point));
        assert!(iv::dot_tight(bad[0],iv::cross(bad[1],bad[2]).unwrap()).unwrap().lo>0.);
        assert!(!principal_positive(bad,p).unwrap());
    }
}
