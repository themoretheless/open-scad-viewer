//! Exact full rectangular trim and world-boundary identity premise.
use super::{curve,surface,curve_surface_agreement,Result,input};
#[cfg(feature="codec")]
use value_codec::{Value,json,Deserialize};
#[cfg(feature="codec")]
fn field<T:for<'a> Deserialize<'a>>(v:&Value,key:&str)->Result<T>{
    value_codec::from_value(v[key].clone()).map_err(|e|input(format!("Invalid {key}: {e}")))
}
pub fn covers(s:&surface::Surface,holes:&[usize],uses:&[(&curve::Curve,&curve::Curve,bool)],limit:u64)->Result<(bool,u64)>{
    if limit>1000000{return Err(input("Retained wall exact budget exceeds 1000000"));}
    let mut work=0u64;
    let refuse=|work|(false,work);
    if !holes.is_empty()||uses.len()!=4{return Ok(refuse(work));}
    let mut curves=Vec::with_capacity(4);
    for &(_,c,_) in uses{
        if c.periodic||c.degree!=1||c.knots!=[0.,0.,1.,1.]||c.control_points.len()!=2
            ||c.control_points.iter().any(|p|p.len()!=2||p.iter().any(|x|!x.is_finite()))
            ||c.weights.len()!=2||c.weights.iter().any(|w|!w.is_finite()||*w<=0.) {return Ok(refuse(work));}
        curves.push(c);
    }
    let corners=[[0.,0.],[1.,0.],[1.,1.],[0.,1.]];
    let rectangle=(0..4).any(|start|[1isize,-1].into_iter().any(|direction|{
        curves.iter().enumerate().all(|(i,c)|{
            let a=(start as isize+direction*i as isize).rem_euclid(4) as usize;
            let b=(start as isize+direction*(i+1) as isize).rem_euclid(4) as usize;
            c.control_points[0].as_slice()==corners[a]&&c.control_points[1].as_slice()==corners[b]
        })
    }));
    if !rectangle{return Ok(refuse(work));}
    for &(world,uv,reversed) in uses{
        if work>=limit{return Ok(refuse(work));}
        let Some(decision)=curve_surface_agreement::verify_exact(world,uv,s,reversed,limit-work)? else{return Ok(refuse(work));};
        if decision.work_used>limit-work{return Ok(refuse(work));}
        work+=decision.work_used;
        if decision.outcome!=cad_predicates::BezierIdentity::Equal{return Ok(refuse(work));}
    }
    Ok((true,work))
}
#[cfg(feature="codec")]
pub fn inspect(v:&Value)->Result<Value>{
    let surface:surface::Surface=field(v,"surface")?;
    let holes:Vec<usize>=field(v,"holes")?;
    let uses:Vec<Value>=field(v,"coedges")?;
    let owned=uses.iter().map(|usage|Ok((field::<curve::Curve>(usage,"world")?,field::<curve::Curve>(usage,"uv")?,field::<bool>(usage,"reversed")?))).collect::<Result<Vec<_>>>()?;
    let coedges=owned.iter().map(|(world,uv,reversed)|crate::retained_wall_domain_certificate::Coedge {world,uv,reversed:*reversed}).collect::<Vec<_>>();
    let report=crate::retained_wall_domain_certificate::inspect(&surface,&coedges,!holes.is_empty(),field(v,"maxWork")?)?;
    Ok(json!({"domainCertified":report.domain_certified,"work":report.work,"globalEmbeddingCertified":false}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture()->Value{
        let corners=[[0.,0.],[1.,0.],[1.,1.],[0.,1.]];
        let uses:Vec<Value>=(0..4).map(|i|{
            let a=corners[i];let b=corners[(i+1)%4];
            json!({"uv":{"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[a,b],"weights":[1.,1.],"periodic":false},
                "world":{"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[[a[0],a[1],0.],[b[0],b[1],0.]],"weights":[1.,1.],"periodic":false},"reversed":false})
        }).collect();
        json!({"surface":{"degreeU":1,"degreeV":1,"knotsU":[0.,0.,1.,1.],"knotsV":[0.,0.,1.,1.],"controlPoints":[[[0.,0.,0.],[0.,1.,0.]],[[1.,0.,0.],[1.,1.,0.]]],"weights":[[1.,1.],[1.,1.]],"periodicU":false,"periodicV":false},"holes":[],"coedges":uses,"maxWork":1000000})
    }
    #[test]
    fn retained_wall_domain_requires_matching_rational_parameterization(){
        let mut v=fixture();
        let uses=v["coedges"].as_array().unwrap().iter().map(|usage|{
            let mut usage=usage.clone();usage["uv"]["weights"]=json!([2.,1.]);
            usage["world"]["weights"]=json!([2.,1.]);usage
        }).collect::<Vec<_>>();v["coedges"]=json!(uses);
        assert_eq!(inspect(&v).unwrap()["domainCertified"],true);
        v["coedges"][0]["world"]["weights"]=json!([1.,1.]);
        assert_eq!(inspect(&v).unwrap()["domainCertified"],false);
        v["coedges"][0]["uv"]["weights"]=json!([0.,1.]);
        assert_eq!(inspect(&v).unwrap()["domainCertified"],false);
    }
    #[test]
    fn retained_wall_domain_requires_complete_trim_and_exact_world_identity(){
        let v=fixture();let report=inspect(&v).unwrap();assert_eq!(report["domainCertified"],true);
        let work=report["work"].as_u64().unwrap();assert!(work>0);
        let mut cutoff=v.clone();cutoff["maxWork"]=json!(work-1);assert_eq!(inspect(&cutoff).unwrap()["domainCertified"],false);
        let mut wrong=v.clone();wrong["coedges"][0]["world"]["controlPoints"][0][2]=json!(0.125);assert_eq!(inspect(&wrong).unwrap()["domainCertified"],false);
        let mut hole=v.clone();hole["holes"]=json!([0]);assert_eq!(inspect(&hole).unwrap()["domainCertified"],false);
        let mut clipped=v.clone();clipped["coedges"][0]["uv"]["controlPoints"][1][0]=json!(0.5);assert_eq!(inspect(&clipped).unwrap()["domainCertified"],false);
        let mut direction=v.clone();direction["coedges"][0]["reversed"]=json!(true);assert_eq!(inspect(&direction).unwrap()["domainCertified"],false);
        let mut exact_budget=v.clone();exact_budget["maxWork"]=json!(work);assert_eq!(inspect(&exact_budget).unwrap()["domainCertified"],true);
        let mut rotated=v.clone();let mut uses=rotated["coedges"].as_array().unwrap().clone();uses.rotate_left(1);rotated["coedges"]=json!(uses);assert_eq!(inspect(&rotated).unwrap()["domainCertified"],true);
        let mut reversed=v.clone();let uses=reversed["coedges"].as_array().unwrap().iter().rev().map(|usage|{
            let mut usage=usage.clone();let mut points=usage["uv"]["controlPoints"].as_array().unwrap().clone();points.reverse();usage["uv"]["controlPoints"]=json!(points);usage["reversed"]=json!(true);usage
        }).collect::<Vec<_>>();reversed["coedges"]=json!(uses);assert_eq!(inspect(&reversed).unwrap()["domainCertified"],true);
        assert_eq!(v,fixture());
    }
}
