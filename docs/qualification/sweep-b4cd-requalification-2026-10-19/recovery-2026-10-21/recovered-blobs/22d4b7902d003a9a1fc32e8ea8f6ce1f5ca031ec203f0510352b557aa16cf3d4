use geometry_bridge::dispatch;
use nurbs_core::{curve::Curve,primitives::line};
use value_codec::json;
use nurbs_core::progressive_sweep::constant_vector_law;
fn rings(z:f64)->Vec<Vec<Curve>>{
    let p=[[0.,0.,z],[2.,0.,z],[2.,2.,z],[0.,2.,z]];
    vec![(0..4).map(|i|line(p[i],p[(i+1)%4]).unwrap()).collect()]
}
#[test]
fn retained_caps_transport_binds_actual_geometry_without_promoting_body_guarantees(){
    let endpoints=[rings(0.),rings(10.)];
    let model=brep_core::rational_section_loft(&endpoints).unwrap();
    let request=json!({"op":"brep_sweep_retained_caps_audit","model":model,"endpoints":endpoints,
        "maxWalls":1024,"maxExactWork":1000000,"maxChartCells":1000,"maxTrimPairs":100000,
        "maxTrimCells":100000,"maxTrimDomainCells":1000000,"maxEdges":1024});
    let proof=dispatch(request.clone()).unwrap();
    assert_eq!(proof["exact"],true);assert_eq!(proof["capErrorUpper"],0.);
    for key in ["continuousBound","globalEmbeddingCertified","solidCertified"] {assert_eq!(proof[key],false);}
    let mut partial=request.clone();partial["maxExactWork"]=json!(1);
    assert_eq!(dispatch(partial).unwrap()["exact"],false);
    let mut malformed=request;malformed["endpoints"]=json!([rings(0.)]);
    assert!(dispatch(malformed).is_err());
}

#[test]
fn progressive_body_reports_cap_ownership_from_its_own_source_sections(){
    let path=line([0.;3],[0.,0.,10.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let mut base=json!({"op":"brep_nurbs_progressive_profile_body","loops":rings(0.),"path":path,
        "scale":scale,"twist":twist,"orientation":"rmf","normal":[1.,0.,0.],"spacing":"parameter",
        "initial_sections":3,"max_sections":3,"max_deviation":0.01});
    for mode in ["plain","affine","authored","guide"] {
        let mut request=base.clone();
        if mode!="plain" {
            request["axis_scale"]=json!(constant_vector_law([2.,3.,1.]).unwrap());
            request["center_law"]=json!(constant_vector_law([0.;3]).unwrap());
        }
        if mode=="authored" {
            request["orientation"]=json!("authored");
            request["frame_axis"]=json!(constant_vector_law([0.,0.,1.]).unwrap());
            request["frame_normal"]=json!(constant_vector_law([1.,0.,0.]).unwrap());
        }
        if mode=="guide" {request["orientation_guide"]=json!(line([1.,0.,0.],[1.,0.,10.]).unwrap());}
        let result=dispatch(request).unwrap();
        assert_eq!(result["retainedCaps"]["exact"],true,"{mode}: {result:?}");
        assert_eq!(result["retainedCaps"]["scope"],"constructor-owned-retained-endpoint-regions");
        assert_eq!(result["retainedCaps"]["continuousBound"],false);
        assert_eq!(result["globalEmbeddingCertified"],false);
        assert_eq!(result["boundaryContinuousBound"],true);
        assert_eq!(result["boundaryErrorWithinBudget"],true);
        assert_eq!(result["retainedWalls"]["certified"],true);
        assert_eq!(result["boundaryErrorScope"],"constructor-owned-retained-wall-and-cap-union");
        assert!(result["boundaryErrorUpper"].as_f64().is_some_and(|upper|upper>=0.&&upper<0.01));
        assert_eq!(result["capProjection"]["idealCapDomainsCertified"],true);
        let cap_bounds=result["filledCapErrorUpper"].as_array().unwrap();
        assert_eq!(cap_bounds.len(),2);
        assert!(cap_bounds.iter().all(|value|value.as_f64().is_some_and(|upper|upper>=0.&&upper<0.01)));
        assert!(result["approximation"]["report"]["endpointContourErrorUpper"].as_array().is_some());
    }
    let mut hollow=base.clone();
    let mut extracted=base.clone();
    let mut extracted_loops=rings(0.);
    extracted_loops[0][0]=nurbs_core::curve::Curve {degree:2,knots:vec![0.,0.,0.,0.5,1.,1.,1.],
        control_points:vec![vec![0.,0.,0.],vec![0.5,-0.25,0.],vec![1.5,-0.25,0.],vec![2.,0.,0.]],
        weights:vec![1.,0.75,1.25,1.],periodic:false};
    extracted["loops"]=json!(extracted_loops);
    let extracted_body=dispatch(extracted).unwrap();
    assert_eq!(extracted_body["bodyDecompositionProducts"],54);
    assert!(extracted_body["bodyDecompositionErrorUpper"].as_f64().unwrap()>0.);
    assert_eq!(extracted_body["retainedWalls"]["certified"],true);
    assert_eq!(extracted_body["retainedCaps"]["exact"],true);
    assert_eq!(extracted_body["boundaryContinuousBound"],true);
    let mut loops=rings(0.);
    let hole=[[0.5,0.5,0.],[0.5,1.5,0.],[1.5,1.5,0.],[1.5,0.5,0.]];
    loops.push((0..4).map(|i|line(hole[i],hole[(i+1)%4]).unwrap()).collect());
    hollow["loops"]=json!(loops);
    let hollow_body=dispatch(hollow).unwrap();
    assert_eq!(hollow_body["retainedCaps"]["exact"],true);
    assert_eq!(hollow_body["retainedCaps"]["inspectedEdges"],16);
    assert_eq!(hollow_body["retainedWalls"]["certified"],true);
    assert_eq!(hollow_body["boundaryContinuousBound"],true);
    assert_eq!(hollow_body["boundaryErrorWithinBudget"],true);
    let mut curved=base.clone();
    curved["path"]=json!(Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.,0.,5.],vec![1.,0.,10.]],weights:vec![1.;3],periodic:false});
    curved["max_deviation"]=json!(1.);
    let unresolved=dispatch(curved).unwrap();
    assert_eq!(unresolved["filledCapErrorUpper"],value_codec::Value::Null);
    assert_eq!(unresolved["boundaryContinuousBound"],false);
    assert_eq!(unresolved["boundaryErrorWithinBudget"],value_codec::Value::Null);
    base["orientation"]=json!("authored");
    base["orientation_guide"]=json!(line([1.,0.,0.],[1.,0.,10.]).unwrap());
    assert!(dispatch(base).is_err());
}
