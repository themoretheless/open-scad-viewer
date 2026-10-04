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
