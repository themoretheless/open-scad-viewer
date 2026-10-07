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
    let mut out=Report{curves:None,displacement_upper:None,exact_planar:false,work:0,reason:"work-limit"};
    if max_work==0 {return Ok(out);}
    let count=curves.iter().map(|c|c.control_points.len()).sum::<usize>();
    if count>4096 {return Ok(out);}
    for c in curves {c.validate()?;if c.control_points[0].len()!=3 {
        return Err(Error::new("NURBS_INVALID_INPUT","Section projection needs 3D curves"));
    }}
    let axes=(0..3).filter(|k|*k!=axis).collect::<Vec<_>>();
    let mut corrected=curves.to_vec();let mut upper=0_f64;
    for (source,target) in curves.iter().zip(&mut corrected) {
        for (p,q) in source.control_points.iter().zip(&mut target.control_points) {
            for &k in &axes {q[k]=(p[k]/quantum).round()*quantum;}
            q[axis]=offset+coefficients[0]*q[axes[0]]+coefficients[1]*q[axes[1]];
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
    let plane=|u:f64,v:f64|{let mut p=[0.;3];p[axes[0]]=u;p[axes[1]]=v;p[axis]=offset+coefficients[0]*u+coefficients[1]*v;p};
    let anchors=[plane(0.,0.),plane(1.,0.),plane(0.,1.)];
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
    project(curves,axis,coefficients,offset,quantum,tolerance,max_work)
}

#[cfg(test)]
mod tests {
    use super::*;
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
