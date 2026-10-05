use super::*;
use crate::surface::Surface;
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn offset_transport_preserves_numerical_and_interval_claim_boundaries() {
    let s = plane();
    let proposal = dispatch(
        json!({"op":"surface_offset_evaluate","surface":s,"parameters":[0.37,0.62],"distance":0.2}),
    )
    .unwrap();
    assert_eq!(proposal["point"], json!([0.37, 0.62, 0.2]));
    assert_eq!(proposal["certified"], json!(false));
    for op in ["surface_offset_bounds", "surface_offset_jacobian_bounds"] {
        let r = dispatch(
            json!({"op":op,"surface":s,"domain":[[0.,1.],[0.,1.]],"distance":0.2,"maxSpans":1}),
        )
        .unwrap();
        assert!(!r["image"].is_null());
        assert_eq!(r["offsetRegularityCertified"], json!(false));
        assert_eq!(r["topologyAuthority"], json!(false));
    }
}
#[test]
fn offset_section_transport_does_not_promote_a_contact_to_a_whole_fillet() {
    let a = plane();
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            let z = p[1];
            p[1] = 0.5;
            p[2] = z;
        }
    }
    let r=dispatch(json!({"op":"surface_offset_contact_section","a":a,"b":b,"distances":[0.2,0.2],
        "fixedAxis":0,"fixed":0.37,"firstOther":[0.25,0.35],"secondDomain":[[0.32,0.42],[0.15,0.25]],"maxSpans":2})).unwrap();
    assert_eq!(r["status"], json!("unique-contact"));
    assert_eq!(r["rootExistenceProven"], json!(true));
    for field in [
        "wholeCurveComplete",
        "trimMembershipProven",
        "topologyAuthority",
    ] {
        assert_eq!(r[field], json!(false));
    }
    assert!(!r["witness"].is_null());
}
#[test]
fn offset_candidate_transport_retains_incomplete_work_and_validates_second_operand() {
    let a = plane();
    let request = json!({"op":"surface_offset_candidates","a":a,"b":a,"distances":[0.2,0.2],
        "domains":[[[0.,1.],[0.,1.]],[[0.,1.],[0.,1.]]],"parameterTolerance":0.01,"maxBoxes":1,"maxSpans":1});
    let r = dispatch(request.clone()).unwrap();
    assert_eq!(r["reason"], json!("work-limit"));
    assert!(!r["pendingBoxes"].as_array().unwrap().is_empty());
    assert_eq!(r["rootExistenceProven"], json!(false));
    let mut invalid = request;
    invalid["b"]["weights"][0][0] = json!(0.);
    assert!(dispatch(invalid).is_err());
}

#[test]
fn offset_band_transport_exposes_uniform_coverage_without_global_admission() {
    let a = plane();
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            let z = p[1];
            p[1] = 0.5;
            p[2] = z;
        }
    }
    let request = json!({"op":"surface_offset_contact_band","a":a,"b":b,"distances":[0.2,0.2],
        "fixedAxis":0,"fixedInterval":[0.35,0.39],"firstOther":[0.25,0.35],
        "secondDomain":[[0.30,0.44],[0.15,0.25]],"maxSpans":2});
    let r = dispatch(request.clone()).unwrap();
    assert_eq!(r["status"], json!("continuous-branch"));
    for field in [
        "rootForEveryParameterProven",
        "uniqueWithinTube",
        "continuousBranchProven",
    ] {
        assert_eq!(r[field], json!(true));
    }
    for field in [
        "wholeCurveComplete",
        "trimMembershipProven",
        "topologyAuthority",
    ] {
        assert_eq!(r[field], json!(false));
    }
    let mut narrow = request;
    narrow["secondDomain"][0] = json!([0.36, 0.38]);
    let r = dispatch(narrow).unwrap();
    assert_eq!(r["status"], json!("unresolved"));
    assert_eq!(r["rootForEveryParameterProven"], json!(false));
    assert!(r["witness"].is_null());
}

#[test]
fn trimmed_offset_band_transport_requires_whole_domain_membership() {
    let a = plane();
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            let z = p[1];
            p[1] = 0.5;
            p[2] = z
        }
    }
    let rectangle = |lo: [f64; 2], hi: [f64; 2], reverse: bool| {
        let mut p = vec![lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]];
        if reverse {
            p.reverse()
        }
        (0..4).map(|i|json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[p[i],p[(i+1)%4]],"weights":[1.,1.],"periodic":false})).collect::<Vec<_>>()
    };
    let outer = rectangle([0., 0.], [1., 1.], false);
    let request = json!({"op":"surface_offset_trimmed_contact_band","a":a,"b":b,"distances":[0.2,0.2],"fixedAxis":0,"fixedInterval":[0.35,0.39],"firstOther":[0.25,0.35],"secondDomain":[[0.30,0.44],[0.15,0.25]],"maxSpans":2,"firstLoops":[outer],"secondLoops":[outer],"toleranceUv":1e-7,"maxPairs":10000,"maxCells":10000,"maxDomainCells":100000});
    let r = dispatch(request.clone()).unwrap();
    assert_eq!(r["trimMembershipProven"], true);
    assert_eq!(r["topologyAuthority"], false);
    let mut hole = request.clone();
    hole["firstLoops"] = json!([outer, rectangle([0.34, 0.29], [0.40, 0.31], true)]);
    let r = dispatch(hole).unwrap();
    assert_eq!(r["trimMembershipProven"], false);
    assert_eq!(r["reason"], "contact-outside-trim");
    let mut invalid = request;
    invalid["firstOther"] = json!([0.75, 0.85]);
    invalid["secondLoops"][0][0]["weights"] = json!([0., 1.]);
    assert!(dispatch(invalid).is_err());
}

#[test]
fn offset_source_boundary_transport_requires_all_original_spatial_coedges() {
    let a = plane();
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            let z = p[1];
            p[1] = 0.5;
            p[2] = z
        }
    }
    let p = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let uv=(0..4).map(|i|json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[p[i],p[(i+1)%4]],"weights":[1.,1.],"periodic":false})).collect::<Vec<_>>();
    let world = |side: usize| {
        uv.iter()
            .enumerate()
            .map(|(i, c)| {
                let mut c = c.clone();
                let lift = |uv: [f64; 2]| {
                    if side == 0 {
                        [uv[0], uv[1], 0.]
                    } else {
                        [uv[0], 0.5, uv[1]]
                    }
                };
                c["controlPoints"] = json!([lift(p[i]), lift(p[(i + 1) % 4])]);
                json!({"world":c,"reversed":false})
            })
            .collect::<Vec<_>>()
    };
    let request = json!({"op":"surface_offset_source_boundary","a":a,"b":b,"distances":[0.2,0.2],"fixedAxis":0,"fixedInterval":[0.35,0.39],"firstOther":[0.25,0.35],"secondDomain":[[0.30,0.44],[0.15,0.25]],"maxSpans":2,"firstLoops":[uv],"secondLoops":[uv],"firstCoedges":[world(0)],"secondCoedges":[world(1)],"toleranceUv":1e-7,"maxPairs":10000,"maxCells":10000,"maxDomainCells":100000,"toleranceMm":1e-5,"maxExactWork":1000000,"maxAgreementCells":10000});
    let r = dispatch(request.clone()).unwrap();
    assert_eq!(r["worldCoedgeIdentityProven"], true);
    assert_eq!(r["checkedCoedges"], 8);
    assert_eq!(r["topologyAuthority"], false);
    let mut changed = request.clone();
    changed["secondCoedges"][0][0]["world"]["controlPoints"][0][1] = json!(0.501);
    let r = dispatch(changed).unwrap();
    assert_eq!(r["worldBoundaryWithinToleranceProven"], false);
    assert_eq!(r["reason"], "spatial-boundary-mismatch");
    let mut missing = request.clone();
    missing["secondCoedges"] = json!([]);
    assert!(dispatch(missing).is_err());
    let mut hidden = request;
    hidden["firstOther"] = json!([0.75, 0.85]);
    hidden["secondCoedges"][0][0]["world"]["weights"] = json!([0., 1.]);
    assert!(dispatch(hidden).is_err());
}

#[test]
fn offset_contact_tangent_transport_does_not_admit_an_envelope() {
    let a = plane();
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            let z = p[1];
            p[1] = 0.5;
            p[2] = z
        }
    }
    let request = json!({"op":"surface_offset_contact_tangent","a":a,"b":b,"distances":[0.2,0.2],"fixedAxis":0,"fixedInterval":[0.35,0.39],"firstOther":[0.25,0.35],"secondDomain":[[0.30,0.44],[0.15,0.25]],"maxSpans":2});
    let r = dispatch(request).unwrap();
    assert_eq!(r["centerRegularityProven"], true);
    assert!(!r["parameterDerivativeIntervals"].is_null());
    for field in [
        "wholeCurveComplete",
        "envelopeRegularityProven",
        "trimMembershipProven",
        "topologyAuthority",
    ] {
        assert_eq!(r[field], false);
    }
}

#[test]
fn offset_envelope_transport_keeps_full_arc_and_topology_gates() {
    let a=plane();let mut b=a.clone();
    for row in &mut b.control_points {for p in row {let z=p[1];p[1]=0.5;p[2]=z;}}
    let mut input=json!({"op":"surface_offset_envelope","a":a,"b":b,"distances":[0.2,0.2],
        "fixedAxis":0,"fixedInterval":[0.35,0.39],"firstOther":[0.25,0.35],
        "secondDomain":[[0.30,0.44],[0.15,0.25]],"maxSpans":2,"maxCells":255});
    let r=dispatch(input.clone()).unwrap();
    assert_eq!(r["envelopeRegularityProven"],json!(true));
    assert_eq!(r["centerTangent"]["centerRegularityProven"],json!(true));
    for key in ["wholeCurveComplete","finiteNurbsPatchProven","trimMembershipProven","embeddingProven","topologyAuthority"] {
        assert_eq!(r[key],json!(false));
    }
    input["maxCells"]=json!(1);
    let r=dispatch(input).unwrap();
    assert_eq!(r["envelopeRegularityProven"],json!(false));
    assert_eq!(r["visitedCells"],json!(1));
    assert_eq!(r["cells"][0]["arcParameter"],json!([0.,1.]));
}

#[test]
fn finite_envelope_transport_binds_candidate_and_complete_partition() {
    let a=plane();let mut b=a.clone();for row in &mut b.control_points {for p in row {let z=p[1];p[1]=0.5;p[2]=z;}}
    let mut input=json!({"op":"surface_offset_envelope_fit","a":a,"b":b,"distances":[0.2,0.2],"fixedAxis":0,
        "fixedInterval":[0.35,0.39],"firstOther":[0.25,0.35],"secondDomain":[[0.30,0.44],[0.15,0.25]],
        "maxSpans":2,"maxCells":2047,"toleranceMm":1e-4});
    let r=dispatch(input.clone()).unwrap();
    assert_eq!(r["candidateSource"],json!("section-proposal"));
    assert_eq!(r["qualification"]["finiteNurbsPatchProven"],json!(true));
    assert!(!r["candidateSurface"].is_null());
    assert_eq!(r["qualification"]["partition"][0]["domain"],json!([[0.35,0.39],[0.,1.]]));
    for k in ["wholeCurveComplete","tangentToleranceProven","trimMembershipProven","embeddingProven","topologyAuthority"] {assert_eq!(r[k],json!(false));assert_eq!(r["qualification"][k],json!(false));}
    input["candidate"]=r["candidateSurface"].clone();input["maxCells"]=json!(1);
    let r=dispatch(input).unwrap();assert_eq!(r["candidateSource"],json!("authored"));
    assert_eq!(r["proposalSections"],json!(0));assert_eq!(r["qualification"]["finiteNurbsPatchProven"],json!(false));
    assert_eq!(r["qualification"]["partition"][0]["leaf"],json!(0));
}
