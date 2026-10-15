//! Qualification-only full-domain correspondence bounds for reviewed rounding
//! migrations. This does not admit Solid or establish source-bound provenance.
use nurbs_core::{curve::Curve, interval_eval::Interval as I, numerics::error_upper, surface::Surface};
use sha2::{Digest, Sha256};
use value_codec::{Value, json};

type Coefficient = ([f64; 3], f64);
type Answer<T> = Result<T, String>;

fn norm_upper(vector: [I; 3]) -> Answer<f64> {
    let mut square = I::point(0.);
    for x in vector {
        if x.lo == 0. && x.hi == 0. { continue; }
        square = square.add(x.mul(x).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    }
    if square.hi == 0. {return Ok(0.);}
    let upper = square.hi.sqrt().next_up();
    if !upper.is_finite() {return Err("numeric-range".into());}
    Ok(upper)
}
fn coefficients(points: &[Vec<f64>], weights: &[f64]) -> Answer<Vec<Coefficient>> {
    if points.len()!=weights.len() {return Err("coefficient-shape".into());}
    points.iter().zip(weights).map(|(p,&w)| {
        if p.len()!=3 || p.iter().any(|x|!x.is_finite()) || !w.is_finite() || w<=0. {
            return Err("finite-positive-3d-coefficients-required".into());
        }
        Ok(([p[0],p[1],p[2]],w))
    }).collect()
}
fn delta(a: Coefficient, b: Coefficient, weighted: bool) -> Answer<[I; 3]> {
    let mut result=[I::point(0.);3];
    for (k,value) in result.iter_mut().enumerate() {
        if !weighted && a.0[k]==b.0[k] {continue;}
        let x=if weighted {I::point(a.0[k]).mul(I::point(a.1))}else{Ok(I::point(a.0[k]))}.map_err(|e|e.to_string())?;
        let y=if weighted {I::point(b.0[k]).mul(I::point(b.1))}else{Ok(I::point(b.0[k]))}.map_err(|e|e.to_string())?;
        *value=x.sub(y).map_err(|e|e.to_string())?;
    }
    Ok(result)
}
fn bound_coefficients(a: &[Coefficient], b: &[Coefficient], work: &mut usize, max_work: usize) -> Answer<f64> {
    if a.is_empty() || a.len()!=b.len() {return Err("coefficient-shape".into());}
    if a.len()>max_work.saturating_sub(*work) {return Err("work-limit".into());}
    *work+=a.len();
    if a==b {return Ok(0.);}
    // Equal positive rational weights yield the same nonnegative normalized
    // basis on the complete common natural domain. Its sum is exactly one.
    if a.iter().zip(b).all(|(x,y)|x.1==y.1) {
        let mut upper=0f64;
        for (&x,&y) in a.iter().zip(b) {upper=upper.max(norm_upper(delta(x,y,false)?)?);}
        return Ok(upper);
    }
    // For A/W and B/V with a common nonnegative partition of unity:
    // A/W-B/V=(A-B)/W+B*(V-W)/(W*V). Coefficient hulls enclose each
    // numerator; positive minimum weights bound both denominators below.
    let mut numerator=0f64;let mut second=0f64;let mut weight_delta=0f64;
    let mut min_a=f64::INFINITY;let mut min_b=f64::INFINITY;
    for (&x,&y) in a.iter().zip(b) {
        numerator=numerator.max(norm_upper(delta(x,y,true)?)?);
        second=second.max(norm_upper(delta(y,([0.;3],y.1),true)?)?);
        let dw=I::point(x.1).sub(I::point(y.1)).map_err(|e|e.to_string())?;
        weight_delta=weight_delta.max(dw.lo.abs().max(dw.hi.abs()));
        min_a=min_a.min(x.1);min_b=min_b.min(y.1);
    }
    let first=I::point(numerator).div(I::point(min_a)).map_err(|e|e.to_string())?.hi;
    let denominator=I::point(min_a).mul(I::point(min_b)).map_err(|e|e.to_string())?;
    let correction=I::point(second).mul(I::point(weight_delta)).and_then(|x|x.div(denominator)).map_err(|e|e.to_string())?.hi;
    error_upper::add(first,correction).ok_or_else(||"numeric-range".into())
}
fn subject(pair: &Value, work: &mut usize, max_work: usize) -> Answer<f64> {
    let a=pair["original"].clone();let b=pair["current"].clone();
    match pair["kind"].as_str() {
        Some("surface")=>{
            let a:Surface=value_codec::from_value(a).map_err(|e|e.to_string())?;
            let b:Surface=value_codec::from_value(b).map_err(|e|e.to_string())?;
            a.validate().map_err(|e|e.to_string())?;b.validate().map_err(|e|e.to_string())?;
            if a.degree_u!=b.degree_u || a.degree_v!=b.degree_v || a.knots_u!=b.knots_u || a.knots_v!=b.knots_v
                || a.periodic_u!=b.periodic_u || a.periodic_v!=b.periodic_v
                || a.control_points.len()!=b.control_points.len()
                || a.control_points.iter().zip(&b.control_points).any(|(x,y)|x.len()!=y.len()) {
                return Err("basis-changed".into());
            }
            let aa=coefficients(&a.control_points.into_iter().flatten().collect::<Vec<_>>(),&a.weights.into_iter().flatten().collect::<Vec<_>>())?;
            let bb=coefficients(&b.control_points.into_iter().flatten().collect::<Vec<_>>(),&b.weights.into_iter().flatten().collect::<Vec<_>>())?;
            bound_coefficients(&aa,&bb,work,max_work)
        }
        Some("curve")=>{
            let a:Curve=value_codec::from_value(a).map_err(|e|e.to_string())?;
            let b:Curve=value_codec::from_value(b).map_err(|e|e.to_string())?;
            a.validate().map_err(|e|e.to_string())?;b.validate().map_err(|e|e.to_string())?;
            if a.degree!=b.degree || a.knots!=b.knots || a.periodic!=b.periodic || a.control_points.len()!=b.control_points.len() {
                return Err("basis-changed".into());
            }
            bound_coefficients(&coefficients(&a.control_points,&a.weights)?,&coefficients(&b.control_points,&b.weights)?,work,max_work)
        }
        _=>Err("unknown-subject".into()),
    }
}
fn audit_case(case: &Value, max_work: usize) -> Value {
    let mut work=0usize;let mut upper=0f64;let mut reason=None;
    let mut subjects=Vec::new();
    if let Some(pairs)=case["subjects"].as_array() {
        if pairs.is_empty() {reason=Some("missing-subjects".into());}
        for pair in pairs {
            match subject(pair,&mut work,max_work) {
                Ok(bound)=>{upper=upper.max(bound);subjects.push(json!({"id":pair["id"],"upperMm":bound}));}
                Err(error)=>{reason=Some(error);break;}
            }
        }
    }else{reason=Some("missing-subjects".into());}
    let current=case["currentBoundaryUpperMm"].as_f64();let budget=case["originalBudgetMm"].as_f64();
    let combined=current.and_then(|value|error_upper::add(value,upper));
    let within=reason.is_none()&&combined.zip(budget).is_some_and(|(value,limit)|limit.is_finite()&&limit>0.&&value<=limit);
    json!({"file":case["file"],"certified":reason.is_none(),"upperMm":if reason.is_none(){Some(upper)}else{None},
        "composedUpperMm":if reason.is_none(){combined}else{None},"withinOriginalBudget":within,
        "work":work,"maxWork":max_work,"reason":reason,"subjects":subjects})
}
fn hash(bytes: &[u8]) -> String {Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect()}
fn main() {
    let args=std::env::args().skip(1).collect::<Vec<_>>();assert_eq!(args.len(),2,"Supply request JSON and output JSON");
    let bytes=std::fs::read(&args[0]).unwrap();let input:Value=value_codec::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
    let max_work=usize::try_from(input["maxWork"].as_u64().unwrap()).unwrap();assert!(max_work<=1000000);
    let cases=input["cases"].as_array().unwrap().iter().map(|case|audit_case(case,max_work)).collect::<Vec<_>>();
    let passed=cases.iter().all(|case|case["certified"]==json!(true)&&case["withinOriginalBudget"]==json!(true));
    let output=json!({"schema":"sweep-retained-evaluation-migration/1","passed":passed,"requestSha256":hash(&bytes),
        "producerSha256":hash(include_bytes!("certify_sweep_geometry_migration.rs")),"cases":cases,
        "globalEmbeddingCertified":false,
        "scope":"Full common natural-domain correspondence via positive rational basis coefficient hulls. Same trim/edge ownership and supplied source-bound provenance require independent verification. No Solid admission."});
    std::fs::write(&args[1],value_codec::to_string(&output).unwrap()).unwrap();
    if !passed {std::process::exit(1);}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translation_weights_and_exhausted_work_do_not_weaken_the_bound() {
        let a=[([0.,0.,0.],1.),([1.,0.,0.],1.)];
        let b=[([1.,2.,2.],1.),([2.,2.,2.],1.)];
        let bound=bound_coefficients(&a,&b,&mut 0,2).unwrap();assert!(bound>=3.&&bound<3.+1e-12);
        assert_eq!(bound_coefficients(&a,&a,&mut 0,2).unwrap(),0.);
        assert!(bound_coefficients(&a,&a,&mut 0,0).is_err());
        let c=[([0.,0.,0.],1.),([1.,0.,0.],2.)];
        assert!(bound_coefficients(&a,&c,&mut 0,2).unwrap()>=1./6.);
    }
    #[test]
    fn budget_composition_refuses_missing_or_insufficient_premises() {
        let empty=json!({"file":"test","subjects":[],"currentBoundaryUpperMm":0.5,"originalBudgetMm":0.25});
        assert_eq!(audit_case(&empty,10)["withinOriginalBudget"],json!(false));
        let missing=json!({"file":"test","subjects":[]});assert_eq!(audit_case(&missing,10)["withinOriginalBudget"],json!(false));
        let vacuous=json!({"file":"test","subjects":[],"currentBoundaryUpperMm":0.5,"originalBudgetMm":1.});
        assert_eq!(audit_case(&vacuous,10)["certified"],json!(false));
    }
}
