//! Conditional boundary-set error composition. Premise provenance, topology,
//! orientation and global embedding must be established independently.
use crate::numerics::error_upper;
#[derive(Clone)]
pub struct Premises {
    pub ideal_domains_certified: bool,
    pub retained_regions_exact: bool,
    pub projection_normal_dots: Option<[[f64;2];2]>,
    pub endpoint_error: Option<[f64;2]>,
    pub correction: Option<f64>,
    pub decomposition: Option<[f64;2]>,
    pub parallel_planes: [bool;2],
}
/// Bounds for both filled caps; no partial result on a missing/invalid premise.
pub fn filled_caps(p:&Premises)->Option<[f64;2]>{
    if !p.ideal_domains_certified||!p.retained_regions_exact{return None;}
    let dots=p.projection_normal_dots?;
    if dots.iter().any(|d|!d[0].is_finite()||!d[1].is_finite()||d[0]>d[1]||!(d[0]>0.||d[1]<0.)){return None;}
    let endpoints=p.endpoint_error?;let correction=p.correction?;let decomposition=p.decomposition?;
    let mut bounds=[0.;2];
    for i in 0..2{
        let epsilon=error_upper::add(error_upper::add(endpoints[i],correction)?,decomposition[i])?;
        bounds[i]=if p.parallel_planes[i]{epsilon}else{error_upper::sqrt_two(epsilon)?};
    }
    Some(bounds)
}
/// Maximum over complete wall and cap sets. A closed sweep has no cap obligation.
pub fn boundary(wall:Option<f64>,caps:Option<[f64;2]>,closed:bool)->Option<f64>{
    let valid=|x:f64|x.is_finite()&&x>=0.;
    let wall=wall?;if !valid(wall){return None;}
    if closed{return Some(wall);}
    let caps=caps?;if !caps.into_iter().all(valid){return None;}
    Some(wall.max(caps[0]).max(caps[1]))
}
#[cfg(test)]
mod tests{
    use super::*;
    fn fixture()->Premises{Premises{ideal_domains_certified:true,retained_regions_exact:true,
        projection_normal_dots:Some([[1.,1.],[-1.,-1.]]),endpoint_error:Some([0.,0.]),
        correction:Some(0.),decomposition:Some([0.,0.]),parallel_planes:[true,true]}}
    #[test]
    fn cap_composition_requires_all_premises_and_both_endpoints(){
        let p=fixture();assert_eq!(filled_caps(&p),Some([0.,0.]));
        let mut bad=p.clone();bad.ideal_domains_certified=false;assert_eq!(filled_caps(&bad),None);
        let mut bad=p.clone();bad.retained_regions_exact=false;assert_eq!(filled_caps(&bad),None);
        let mut bad=p.clone();bad.projection_normal_dots=Some([[1.,1.],[-1.,1.]]);assert_eq!(filled_caps(&bad),None);
        let mut bad=p.clone();bad.endpoint_error=None;assert_eq!(filled_caps(&bad),None);
        let mut bad=p.clone();bad.correction=None;assert_eq!(filled_caps(&bad),None);
        let mut bad=p.clone();bad.decomposition=None;assert_eq!(filled_caps(&bad),None);
        let mut bad=p.clone();bad.decomposition=Some([0.,f64::INFINITY]);assert_eq!(filled_caps(&bad),None);
        let mut shifted=p.clone();shifted.correction=Some(0.125);shifted.parallel_planes=[true,false];
        let bounds=filled_caps(&shifted).unwrap();assert_eq!(bounds[0],0.125);assert!(bounds[1]>bounds[0]);
    }
    #[cfg(feature="transport")]
    #[test]
    fn cap_error_transport_keeps_conditional_bounds_distinct_from_continuous_admission(){
        use value_codec::{json,Value};
        let request=json!({"op":"sweep_filled_cap_error_upper","idealCapDomainsCertified":true,
            "retainedCapRegionsExact":true,"projectionNormalDots":[[1.,1.],[-1.,-1.]],
            "endpointContourErrorUpper":[0.,0.],"correctionDisplacementUpper":0.125,
            "decompositionErrorUpper":[0.,0.],"parallelPlanesCertified":[true,false]});
        let report=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(report["continuousBound"],false);assert_eq!(report["capErrorUpper"][0],0.125);
        assert!(report["capErrorUpper"][1].as_f64().unwrap()>0.125);
        let mut missing=request.clone();missing["decompositionErrorUpper"]=Value::Null;
        assert_eq!(crate::transport::dispatch(missing).unwrap()["capErrorUpper"],Value::Null);
        let mut invalid=request;invalid["endpointContourErrorUpper"]=json!([0.]);
        assert!(crate::transport::dispatch(invalid).is_err());
        let boundary=json!({"op":"sweep_boundary_error_upper","wall":1.,"caps":[2.,3.],"closed":false});
        assert_eq!(crate::transport::dispatch(boundary).unwrap()["boundaryErrorUpper"],3.);
    }
    #[test]
    fn boundary_union_refuses_missing_open_caps_and_invalid_bounds(){
        assert_eq!(boundary(Some(1.),Some([2.,3.]),false),Some(3.));
        assert_eq!(boundary(Some(1.),None,false),None);
        assert_eq!(boundary(Some(1.),None,true),Some(1.));
        assert_eq!(boundary(None,Some([0.,0.]),false),None);
        assert_eq!(boundary(Some(-1.),Some([0.,0.]),false),None);
        assert_eq!(boundary(Some(0.),Some([0.,f64::NAN]),false),None);
    }
}

pub struct BoundaryCertificate {
    pub continuous_bound:bool,pub within_budget:Option<bool>,pub error_upper:Option<f64>,
    pub wall_error_upper:Option<f64>,pub filled_cap_error_upper:Option<[f64;2]>,pub reason:Option<&'static str>,
}
/// Conditional composition only: source ownership belongs to the constructor.
/// A complete bound can be proved while failing the caller's tolerance.
pub fn compose_boundary(wall:Option<f64>,caps:Option<[f64;2]>,closed:bool,budget:Option<f64>)->BoundaryCertificate {
    let valid=|x:f64|x.is_finite()&&x>=0.;
    let wall=wall.filter(|&x|valid(x));
    let valid_caps=caps.filter(|c|c.iter().all(|&x|valid(x)));
    let budget=budget.filter(|x|x.is_finite()&&*x>0.);
    let error_upper=budget.and_then(|_|boundary(wall,valid_caps,closed));
    let within_budget=error_upper.zip(budget).map(|(upper,budget)|upper<=budget);
    let reason=if budget.is_none() {Some("invalid-budget")}
        else if wall.is_none() {Some("wall-bound-unproved")}
        else if error_upper.is_none() {Some("filled-cap-bound-unproved")}
        else if within_budget==Some(false) {Some("boundary-budget-exceeded")}else{None};
    BoundaryCertificate {continuous_bound:error_upper.is_some(),within_budget,error_upper,
        wall_error_upper:wall,filled_cap_error_upper:if closed {None}else{valid_caps},reason}
}
#[cfg(test)]
mod boundary_certificate_tests {
    use super::*;
    #[test]
    fn union_certificate_distinguishes_completeness_tolerance_and_missing_premises() {
        #[cfg(feature="transport")]
        {
            let request=value_codec::json!({"op":"sweep_boundary_certificate","wall":0.1,
                "caps":[0.2,0.3],"closed":false,"budget":0.4});
            let r=crate::transport::dispatch(request.clone()).unwrap();
            assert_eq!(r["continuousBound"],true);assert_eq!(r["withinBudget"],true);
            assert_eq!(r["scope"],"boundary-set-hausdorff");
            let mut invalid=request;invalid["caps"]=value_codec::json!([Option::<f64>::None,Some(0.3)]);
            let r=crate::transport::dispatch(invalid).unwrap();
            assert_eq!(r["continuousBound"],false);
            assert_eq!(r["reason"],"filled-cap-bound-unproved");
        }
        let r=compose_boundary(Some(0.1),Some([0.2,0.3]),false,Some(0.4));
        assert!(r.continuous_bound && r.within_budget==Some(true) && r.error_upper==Some(0.3));
        let r=compose_boundary(Some(0.1),Some([0.2,0.3]),false,Some(0.25));
        assert!(r.continuous_bound && r.within_budget==Some(false));
        assert_eq!(r.reason,Some("boundary-budget-exceeded"));
        let r=compose_boundary(Some(0.1),None,false,Some(0.4));
        assert!(!r.continuous_bound && r.within_budget.is_none() && r.reason==Some("filled-cap-bound-unproved"));
        let r=compose_boundary(Some(0.1),None,true,Some(0.4));
        assert!(r.continuous_bound && r.filled_cap_error_upper.is_none());
        for wall in [None,Some(-1.),Some(f64::NAN),Some(f64::INFINITY)] {
            let r=compose_boundary(wall,Some([0.,0.]),false,Some(0.4));
            assert!(!r.continuous_bound && r.reason==Some("wall-bound-unproved"));
        }
        for budget in [None,Some(0.),Some(-1.),Some(f64::NAN),Some(f64::INFINITY)] {
            let r=compose_boundary(Some(0.),Some([0.,0.]),false,budget);
            assert!(!r.continuous_bound && r.reason==Some("invalid-budget"));
        }
    }
}
