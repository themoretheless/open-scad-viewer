//! Rounded display only; exact source-owned geometry remains native.
use crate::{field, input, Result, Value};
use nurbs_core::{curve::Curve, surface::Surface, original_graph_transition::{self as graph, Budget}};
use value_codec::json;
pub(super) fn create(v:Value)->Result<Value>{
    let object=v.as_object().ok_or_else(||input("Expected source graph preview request"))?;
    let keys=["op","sources","width","radiusMm","radiusToleranceMm","maxWork","maxCells"];
    if object.keys().any(|k|!keys.contains(&k.as_str())) {return Err(input("Unexpected graph preview field"));}
    let sources:[Surface;2]=field(&v,"sources")?;
    let width:Curve=field(&v,"width")?;
    let max_work:usize=field(&v,"maxWork")?;
    let max_cells:usize=field(&v,"maxCells")?;
    if max_cells==0||max_cells>100_000{return Err(input("Invalid graph preview cell limit"));}
    let mut b=Budget::new(max_work)?;
    let patch=graph::build_convex_g2([&sources[0],&sources[1]],&width,&mut b)?;
    let (display,error)=patch.preview(&mut b)?;
    let curvature=patch.inspect_curvature(10.,max_cells,&mut b)?;
    // No narrow-band qualification: the preview receipt covers the full patch.
    let radius=patch.inspect_normal_radius(field(&v,"radiusMm")?,field(&v,"radiusToleranceMm")?,[[0.,1.];2],max_cells,&mut b)?;
    let curvature_decision=match curvature.decision {graph::CurvatureDecision::Within=>"within",graph::CurvatureDecision::Exceeds=>"exceeds",graph::CurvatureDecision::Unresolved=>"unresolved"};
    let radius_decision=match radius.decision {graph::NormalRadiusDecision::Within=>"within",graph::NormalRadiusDecision::Exceeds=>"exceeds",graph::NormalRadiusDecision::Unresolved=>"unresolved"};
    Ok(json!({"patch":display,"report":{
        "method":"native-source-graph-convex-G2-preview/1","representation":"rounded-display",
        "sourceContactJets":patch.contact_jets(),"previewStorageErrorMm":error,
        "curvatureLimitPerMm":10.,"curvatureDecision":curvature_decision,
        "absoluteCurvatureUpper":curvature.absolute_curvature_upper,
        "radiusDomain":[[0.,1.],[0.,1.]],"radiusDecision":radius_decision,
        "radiusMm":field::<f64>(&v,"radiusMm")?,"radiusToleranceMm":field::<f64>(&v,"radiusToleranceMm")?,
        "radiusWitnessUv":radius.witness_uv,"radiusWitnessMm":radius.witness_radius_mm,
        "coefficientWork":b.used(),"maxWork":max_work,
        "solidGeometryCertified":false,"interiorEmbeddingCertified":false,
        "applicationAdmitted":false}}))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn request()->Value {
        let sources:[Surface;2]=std::array::from_fn(|side|Surface{
            degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:(0..2).map(|i|vec![vec![i as f64,0.,0.],vec![i as f64,if side==0{-1.}else{1.},-1./1024.]]).collect(),
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false});
        let width=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![1./1024.,0.,0.];2],weights:vec![1.;2],periodic:false};
        json!({"op":"brep_source_graph_g2_preview","sources":sources,"width":width,"radiusMm":1.,"radiusToleranceMm":0.001,"maxWork":1000000,"maxCells":1000})
    }
    #[test]
    fn preview_reports_complete_radius_failure_without_solid_admission(){
        let v=request();let before=v.clone();let r=crate::dispatch(v.clone()).unwrap();
        assert_eq!(r["report"]["curvatureDecision"],"within");
        assert_eq!(r["report"]["radiusDecision"],"exceeds");
        assert_eq!(r["report"]["sourceContactJets"],json!([true,true]));
        for key in ["solidGeometryCertified","interiorEmbeddingCertified","applicationAdmitted"]{assert_eq!(r["report"][key],false);}
        assert_eq!(v,before);
    }
    #[test]
    fn exhausted_or_overridden_preview_never_yields_admission(){
        let mut v=request();v["maxWork"]=json!(0);assert!(crate::dispatch(v).is_err());
        let mut v=request();v["applicationAdmitted"]=json!(true);assert!(crate::dispatch(v).is_err());
    }
}
