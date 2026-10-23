//! Explicit bounded correction to an exactly planar rational section.
//! Positive unchanged weights bound the complete curve displacement by the
//! largest control displacement. Candidate rounding is never a plane proof.
use crate::{curve::Curve,distance_bounds::Interval,Error,Result};
use cad_predicates::{AuthoredScalar,Limits,Outcome,PredicateContext,Sign,SourceArena,ToleranceContext};
pub struct Report {
    pub curves:Option<Vec<Curve>>,
    pub displacement_upper:Option<f64>,
    pub exact_planar:bool,
    pub work:u64,
    pub reason:&'static str,
}
pub fn project(curves:&[Curve],axis:usize,coefficients:[f64;2],offset:f64,
    quantum:f64,tolerance:f64,max_work:u64)->Result<Report> {
    if curves.is_empty()||curves.len()>64||axis>2||!quantum.is_finite()||quantum<=0.
        ||!tolerance.is_finite()||tolerance<0.||!offset.is_finite()
        ||coefficients.iter().any(|x|!x.is_finite())||max_work>cad_predicates::MAX_WORK {
        return Err(Error::new("NURBS_INVALID_INPUT","Invalid bounded section projection"));
    }
    let out=Report{curves:None,displacement_upper:None,exact_planar:false,work:0,reason:"work-limit"};
    if max_work==0 {return Ok(out);}
    let count=curves.iter().map(|c|c.control_points.len()).sum::<usize>();
    if count>4096 {return Ok(out);}
    for c in curves {c.validate()?;if c.control_points[0].len()!=3 {
        return Err(Error::new("NURBS_INVALID_INPUT","Section projection needs 3D curves"));
    }}
    let axes=(0..3).filter(|k|*k!=axis).collect::<Vec<_>>();
    let mut corrected=curves.to_vec();
    for target in &mut corrected {
        for q in &mut target.control_points {
            for &k in &axes {q[k]=(q[k]/quantum).round()*quantum;}
            q[axis]=offset+coefficients[0]*q[axes[0]]+coefficients[1]*q[axes[1]];
        }
    }
    let plane=|u:f64,v:f64|{let mut p=[0.;3];p[axes[0]]=u;p[axes[1]]=v;p[axis]=offset+coefficients[0]*u+coefficients[1]*v;p};
    certify_candidate(curves,corrected,[plane(0.,0.),plane(1.,0.),plane(0.,1.)],tolerance,max_work)
}

fn certify_candidate(curves:&[Curve],corrected:Vec<Curve>,anchors:[[f64;3];3],
    tolerance:f64,max_work:u64)->Result<Report> {
    let mut out=Report{curves:None,displacement_upper:None,exact_planar:false,work:0,reason:"work-limit"};
    if max_work==0{return Ok(out);}
    let mut upper=0_f64;
    for (source,target) in curves.iter().zip(&corrected) {
        for (p,q) in source.control_points.iter().zip(&target.control_points) {
            if q.iter().any(|x|!x.is_finite()) {out.reason="numeric-range";return Ok(out);}
            let mut square=Interval::point(0.);
            for k in 0..3 {
                let delta=Interval::point(q[k]).sub(Interval::point(p[k]))?;
                let maximum=delta.lo.abs().max(delta.hi.abs());
                square=square.add(Interval::point(maximum).mul(Interval::point(maximum))?)?;
            }
            upper=upper.max(square.hi.max(0.).sqrt().next_up());
        }
    }
    out.displacement_upper=Some(upper);
    if upper>tolerance {out.reason="displacement-budget";return Ok(out);}
    let points=corrected.iter().flat_map(|c|c.control_points.iter()).collect::<Vec<_>>();
    let values=anchors.iter().flat_map(|p|p.iter()).chain(points.iter().flat_map(|p|p.iter()))
        .map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect();
    let source=SourceArena::authored("section-projection",1,values)
        .map_err(|_|Error::new("NURBS_INVALID_INPUT","Invalid section projection source"))?;
    let tolerance_context=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&source,&tolerance_context,Limits{max_work,..Limits::default()},None);
    let refs=|i:usize|std::array::from_fn(|k|source.leaf(3*i+k).unwrap());
    for i in 0..points.len() {
        let decision=cad_predicates::orient3d(&mut ctx,refs(0),refs(1),refs(2),refs(i+3))
            .map_err(|_|Error::new("NURBS_INVALID_INPUT","Invalid section plane predicate"))?;
        out.work=ctx.work_used();
        if decision.outcome!=Outcome::Sign(Sign::Zero) {out.reason="exact-plane-unproved";return Ok(out);}
    }
    out.exact_planar=true;out.curves=Some(corrected);out.reason="bounded-exact-plane";
    Ok(out)
}

/// Candidate plane aligned to an authored axis at normalized traversal.
/// The origin is inferred from a retained pole, then snapped to the requested
/// dyadic grid. Orientation is a candidate only: project recomputes exact
/// planarity and a whole-curve displacement bound before returning geometry.
pub fn project_authored_axis(curves: &[Curve], authored_axis: &Curve, traversal: f64,
    quantum: f64, tolerance: f64, max_work: u64) -> Result<Report> {
    authored_axis.validate()?;
    if !traversal.is_finite() || !(0. ..=1.).contains(&traversal)
        || authored_axis.control_points[0].len()!=3 || curves.is_empty()
        || !quantum.is_finite() || quantum<=0. {
        return Err(Error::new("NURBS_INVALID_INPUT", "Invalid authored section plane"));
    }
    for c in curves { c.validate()?; }
    if curves[0].control_points[0].len()!=3 {
        return Err(Error::new("NURBS_INVALID_INPUT", "Section projection needs 3D curves"));
    }
    let [a,b] = authored_axis.domain();
    let parameter = if traversal==1. {b} else {a+(b-a)*traversal};
    let normal = authored_axis.evaluate(parameter)?.point;
    let axis = (0..3).max_by(|i,j| normal[*i].abs().total_cmp(&normal[*j].abs())).unwrap();
    if !normal.iter().all(|x| x.is_finite()) || normal[axis]==0. {
        return Err(Error::new("NURBS_INVALID_INPUT", "Authored section axis must be finite and nonzero"));
    }
    let free = (0..3).filter(|k| *k!=axis).collect::<Vec<_>>();
    let coefficients = [-normal[free[0]]/normal[axis], -normal[free[1]]/normal[axis]];
    let origin = &curves[0].control_points[0];
    let offset = origin[axis]-coefficients[0]*origin[free[0]]-coefficients[1]*origin[free[1]];
    let offset = (offset/quantum).round()*quantum;
    let mut first=project(curves,axis,coefficients,offset,quantum,tolerance,max_work)?;
    // Preserve every accepted legacy candidate. A rounded quotient can make
    // otherwise regular sections fail exact planarity. Small exact integer
    // directions permit a lattice candidate without dividing its coordinates.
    // This is still only a proposal: the same full-pole exact predicate and
    // whole-curve displacement bound must pass in the remaining shared work.
    if first.reason!="exact-plane-unproved" || first.work==max_work{return Ok(first);}
    let bits=quantum.to_bits();
    if bits&((1_u64<<52)-1)!=0 || (bits>>52)&2047==0{return Ok(first);}
    let (candidate,selection_work)=integer_normal_candidate([normal[0],normal[1],normal[2]],axis,max_work-first.work);
    first.work+=selection_work;
    let Some(n)=candidate else{return Ok(first);};
    let denominator=n[axis].unsigned_abs() as i128;
    let sign=n[axis].signum() as i128;
    let grid_index=|x:f64|->Option<i128>{
        (x.is_finite()&&x.fract()==0.&&x.abs()<=2_f64.powi(52)).then(||x as i128)
    };
    let Some(intercept)=grid_index(offset/quantum)else{return Ok(first);};
    let scaled=|index:i128|->Option<f64>{
        if index.unsigned_abs()>(1_u128<<52){return None;}
        let value=index as f64*quantum;value.is_finite().then_some(value)
    };
    // A repaired circle owns shared center/generator coefficients. Rounding
    // its poles independently to the coarser plane lattice destroys those
    // relations and can introduce genuine profile tangent-plane jumps.
    let (shared,shared_work)=shared_free_generators(curves,&free,quantum,denominator,max_work-first.work);
    first.work+=shared_work;
    let mut corrected=shared.unwrap_or_else(||curves.to_vec());
    for curve in &mut corrected {for q in &mut curve.control_points {
        let Some(u)=grid_index((q[free[0]]/(quantum*denominator as f64)).round())else{return Ok(first);};
        let Some(v)=grid_index((q[free[1]]/(quantum*denominator as f64)).round())else{return Ok(first);};
        let indices=[u*denominator,v*denominator,
            intercept-sign*(n[free[0]] as i128*u+n[free[1]] as i128*v)];
        let Some(values)=indices.into_iter().map(scaled).collect::<Option<Vec<_>>>()else{return Ok(first);};
        q[free[0]]=values[0];q[free[1]]=values[1];q[axis]=values[2];
    }}
    let mut anchors=[[0.;3];3];
    for p in &mut anchors{p[axis]=offset;}
    for (i,&k) in free.iter().enumerate(){
        anchors[i+1][k]=denominator as f64;
        anchors[i+1][axis]=offset-sign as f64*n[k] as f64;
    }
    let mut fallback=certify_candidate(curves,corrected,anchors,tolerance,max_work-first.work)?;
    fallback.work+=first.work;
    Ok(fallback)
}

fn shared_free_generators(curves:&[Curve],free:&[usize],quantum:f64,multiple:i128,
    max_work:u64)->(Option<Vec<Curve>>,u64){
    if !curves.iter().all(|c|c.degree==2&&c.control_points.len()==9
        &&c.control_points[0]==c.control_points[8]
        &&c.weights.iter().enumerate().all(|(i,w)|*w==c.weights[i%2])){return (None,0);}
    let signs=[[1,0],[1,1],[0,1],[-1,1],[-1,0],[-1,-1],[0,-1],[1,-1],[1,0]];
    let mut corrected=curves.to_vec();let mut work=0;
    for (source,target) in curves.iter().zip(&mut corrected){for &axis in free {
        let center=source.control_points[0][axis]*0.5+source.control_points[4][axis]*0.5;
        let values=[center,source.control_points[0][axis]-center,source.control_points[2][axis]-center];
        let mut generators=[0_i128;3];
        for (i,value) in values.into_iter().enumerate(){
            if work==max_work{return (None,work);}work+=1;
            let index=(value/(quantum*multiple as f64)).round()*multiple as f64;
            if !index.is_finite()||index.fract()!=0.||index.abs()>=2_f64.powi(50){return (None,work);}
            generators[i]=index as i128;
        }
        for (i,[a,b]) in signs.into_iter().enumerate(){
            if work==max_work{return (None,work);}work+=1;
            let index=generators[0]+a*generators[1]+b*generators[2];
            let value=index as f64*quantum;
            if !value.is_finite(){return (None,work);}
            target.control_points[i][axis]=value;
        }
    }}
    (Some(corrected),work)
}

fn integer_normal_candidate(normal:[f64;3],axis:usize,max_work:u64)->(Option<[i64;3]>,u64){
    if let Some(exact)=small_integer_normal(normal){return (Some(exact),0);}
    // An evaluated tangent can be a few rounded operations away from a small
    // integer direction. This bounded heuristic proposes a nearby plane only;
    // neither its score nor any epsilon participates in the certificate.
    let ratios=normal.map(|x|x/normal[axis]);
    let mut best=None;let mut score=f64::INFINITY;let mut work=0;
    for denominator in 1..=64 {
        if work==max_work{return (None,work);}work+=1;
        let candidate=ratios.map(|x|(x*denominator as f64).round() as i64);
        let error=(0..3).map(|k|(candidate[k] as f64/denominator as f64-ratios[k]).abs()).fold(0.,f64::max);
        if error<score{score=error;best=Some(candidate);}
    }
    (best,work)
}

/// Decode the binary64 direction exactly; no approximate rational recovery.
/// Large integer ratios retain the existing refusal rather than spending an
/// unbounded search or constructing an excessively coarse correction grid.
fn small_integer_normal(normal:[f64;3])->Option<[i64;3]>{
    let mut parts=[(0_i128,0_i32);3];
    for (i,x) in normal.into_iter().enumerate(){
        if !x.is_finite(){return None;}if x==0.{continue;}
        let bits=x.to_bits();let exponent=((bits>>52)&2047) as i32;
        let mut significand=bits&((1_u64<<52)-1);
        let mut power=if exponent==0{-1074}else{significand|=1_u64<<52;exponent-1075};
        let zeros=significand.trailing_zeros();significand>>=zeros;power+=zeros as i32;
        parts[i]=(if x<0.{-(significand as i128)}else{significand as i128},power);
    }
    let minimum=parts.iter().filter(|(m,_)|*m!=0).map(|(_,e)|*e).min()?;
    let mut integers=[0_i128;3];
    for (i,(m,e)) in parts.into_iter().enumerate(){
        if m==0{continue;}
        let shift=(e-minimum) as u32;
        // Bound before shifting: checked_shl alone does not detect lost bits.
        if shift>=74 || m.unsigned_abs()>(1_u128<<52){return None;}
        integers[i]=m.checked_mul(1_i128.checked_shl(shift)?)?;
    }
    let gcd=|mut a:u128,mut b:u128|{while b!=0{(a,b)=(b,a%b);}a};
    let divisor=integers.iter().fold(0_u128,|a,n|gcd(a,n.unsigned_abs())) as i128;
    let reduced=integers.map(|n|n/divisor);
    if reduced.iter().any(|n|n.unsigned_abs()>(1_u128<<20)){return None;}
    Some(reduced.map(|n|n as i64))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrected_cubic_c2_endpoint_caps_keep_exact_plane_and_shared_budget(){
        use crate::progressive_sweep::{Sweep,Options,Orientation,Spacing,constant_vector_law};
        let path=Curve{degree:3,knots:vec![0.,0.,0.,0.,0.5,1.,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![0.16666666666666666,0.,0.],
                vec![0.5,0.16666666666666666,0.],vec![0.8333333333333334,0.6666666666666666,0.5],vec![1.,1.,1.125]],
            weights:vec![1.;5],periodic:false};
        let profiles=[crate::primitives::circle([0.;3],[1.,0.,0.],0.1).unwrap(),
            crate::primitives::circle([0.;3],[-1.,0.,0.],0.05).unwrap()];
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}]{
            let options=Options{orientation:Orientation::CorrectedFrenet,normal:[0.,0.,1.],spacing,
                initial_sections:3,max_sections:129,max_deviation:0.05};
            let sections=profiles.iter().map(|profile|Sweep::new(profile,&path,&scale,&twist,options).unwrap().sections_at(17).unwrap()).collect::<Vec<_>>();
            for (station,t) in [(0,0.),(16,1.)]{
                let original=sections.iter().map(|s|s[station].clone()).collect::<Vec<_>>();
                let mut repaired=crate::circle_sweep_repair::repair(&[original.clone()],2_f64.powi(-40),1e-9,Some(1000000.)).unwrap().sections.unwrap();
                for curves in [original,repaired.remove(0)] {
                let saved=curves.clone();let normal=path.evaluate(t).unwrap().d1.unwrap();
                let axis=constant_vector_law([normal[0],normal[1],normal[2]]).unwrap();
                if station==16 {
                    let coefficients=[-normal[0]/normal[2],-normal[1]/normal[2]];
                    let p=&curves[0].control_points[0];
                    let offset=((p[2]-coefficients[0]*p[0]-coefficients[1]*p[1])/2_f64.powi(-40)).round()*2_f64.powi(-40);
                    assert!(!project(&curves,2,coefficients,offset,2_f64.powi(-40),1e-9,1000000).unwrap().exact_planar,
                        "Regression must exercise the old rounded-quotient refusal");
                }
                let r=project_authored_axis(&curves,&axis,0.,2_f64.powi(-40),1e-9,1000000).unwrap();
                assert!(r.exact_planar,"{spacing:?}/{station}: {}",r.reason);
                assert!(r.displacement_upper.unwrap()<=1e-9);assert!(r.work>0&&r.work<=1000000);
                let short=project_authored_axis(&curves,&axis,0.,2_f64.powi(-40),1e-9,r.work-1).unwrap();
                assert!(!short.exact_planar&&short.curves.is_none());
                assert!(short.work<r.work);
                assert!(!project_authored_axis(&curves,&axis,0.,2_f64.powi(-40),1e-9,0).unwrap().exact_planar);
                let corrected=r.curves.unwrap();
                for (source,retained) in curves.iter().zip(&corrected){
                    assert_eq!(source.degree,retained.degree);assert_eq!(source.knots,retained.knots);
                    assert_eq!(source.weights,retained.weights);assert_eq!(source.periodic,retained.periodic);
                    for j in [0,2,4,6]{for k in 0..3 {
                        assert_eq!(retained.control_points[(j+7)%8][k]+retained.control_points[(j+1)%8][k],
                            2.*retained.control_points[j][k],"Plane correction must retain shared circle jets");
                    }}
                }
                assert_eq!(curves,saved);
                }
            }
        }
        assert_eq!(small_integer_normal([1.,2.,3.75]),Some([4,8,15]));
        assert_eq!(small_integer_normal([0.;3]),None);
        assert_eq!(small_integer_normal([f64::MAX,f64::MIN_POSITIVE,1.]),None);
    }
    #[test]
    fn authored_plane_preserves_tilted_periodic_section_and_bounds_correction() {
        let z = 10. + 0.25*1.25_f64.sqrt();
        let xy = [[1.,0.],[0.,1.],[-1.,0.],[0.,-1.],[1.,0.],[0.,1.]];
        let c = Curve { degree:2, knots:(0..9).map(|i| i as f64).collect(),
            control_points:xy.iter().map(|p| vec![p[0],p[1],z-0.5*p[1]]).collect(),
            weights:vec![1.;6],periodic:true };
        let axis = Curve {degree:1,knots:vec![2.,2.,5.,5.],
            control_points:vec![vec![0.,0.,1.],vec![0.,0.5,1.]],weights:vec![1.;2],periodic:false};
        let before = c.clone();
        let r = project_authored_axis(&[c.clone()],&axis,1.,2_f64.powi(-40),1e-9,1000000).unwrap();
        assert!(r.exact_planar, "{}", r.reason);
        assert!(r.displacement_upper.unwrap()<1e-9);
        let retained = &r.curves.unwrap()[0];
        assert!(retained.periodic);
        assert_eq!(retained.knots,c.knots);
        assert_eq!(retained.weights,c.weights);
        assert_eq!(retained.control_points[0],retained.control_points[4]);
        assert_eq!(retained.control_points[1],retained.control_points[5]);
        let flat = project(&[c.clone()],2,[0.,0.],10.,2_f64.powi(-40),1e-9,1000000).unwrap();
        assert!(flat.curves.is_none() && flat.reason=="displacement-budget");
        assert!(project_authored_axis(&[c.clone()],&axis,1.,2_f64.powi(-40),1e-9,0).unwrap().curves.is_none());
        let mut zero = axis.clone();zero.control_points=vec![vec![0.;3];2];
        assert!(project_authored_axis(&[c.clone()],&zero,1.,2_f64.powi(-40),1e-9,1000000).is_err());
        assert_eq!(c,before);
        #[cfg(feature="transport")]
        {
            let request=value_codec::json!({"op":"curve_project_section_authored_axis",
                "curves":[c],"frameAxis":axis,"traversal":1.,"quantum":2_f64.powi(-40),
                "tolerance":1e-9,"maxWork":1000000});
            let report=crate::transport::dispatch(request).unwrap();
            assert_eq!(report["exactPlanar"],true);
            assert!(report["displacementUpper"].as_f64().unwrap()<1e-9);
            assert_eq!(report["curves"][0]["periodic"],true);
        }

    }

    #[test]
    fn nonplanar_binary64_section_is_corrected_with_a_continuous_bound() {
        let c=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![10.223606797749978,10.2,14.6],vec![9.776393202250022,10.3,14.4],vec![9.552786404500042,10.1,14.8]],
            weights:vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.],periodic:false};
        let original=c.clone();
        let r=project(&[c.clone()],1,[0.,-0.5],17.5,2_f64.powi(-40),1e-9,1000000).unwrap();
        assert!(r.exact_planar);assert!(r.displacement_upper.unwrap()<1e-9);
        let corrected=&r.curves.unwrap()[0];
        assert_eq!(corrected.knots,c.knots);assert_eq!(corrected.weights,c.weights);
        for i in 0..=100 {let u=i as f64/100.;let a=c.evaluate(u).unwrap();let b=corrected.evaluate(u).unwrap();
            let distance=a.point.iter().zip(b.point).map(|(x,y)|(x-y).powi(2)).sum::<f64>().sqrt();
            assert!(distance<=r.displacement_upper.unwrap()+1e-14);
        }
        assert!(project(&[c.clone()],1,[0.,-0.5],17.5,2_f64.powi(-40),0.,1000000).unwrap().curves.is_none());
        assert!(project(&[c.clone()],1,[0.,-0.5],17.5,2_f64.powi(-40),1e-9,0).unwrap().curves.is_none());
        assert_eq!(c,original);
    }
}
