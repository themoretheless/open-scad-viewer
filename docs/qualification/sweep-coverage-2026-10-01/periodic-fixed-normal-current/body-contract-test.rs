// Prepared for isolated Q after its current full bridge process is terminal.
// Original periodic source, circles, tolerances and full global budgets retained.
#[test]
fn periodic_fixed_normal_arc_station_bodies_keep_original_bounds_and_fresh_global_solid() {
    let law=|x|nurbs_core::progressive_sweep::constant_vector_law(x).unwrap();
    for joint in [false,true] {for periodic in [true] {for hollow in [false,true] {
        let path=if periodic {
            let mut points=vec![vec![4.,0.,0.],vec![3.,3.,1.],vec![0.,4.,0.],vec![-3.,3.,-1.],vec![-4.,0.,0.],vec![-3.,-3.,1.],vec![0.,-4.,0.],vec![3.,-3.,-1.]];points.extend_from_within(..6);
            Curve{degree:6,knots:(0..=20).map(|i|(i as f64-6.)/8.).collect(),control_points:points,weights:vec![1.;14],periodic:true}
        } else {nurbs_core::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap()};
        let (center,tangent)=if periodic {([-1.336111111111111,3.1694444444444443,-0.3388888888888889],[-20.133333333333333,-7.866666666666666,-4.266666666666667])}
            else {([5.,0.,0.],[0.,1.,0.])};
        let mut loops=vec![vec![nurbs_core::primitives::circle(center,tangent,0.01).unwrap()]];
        if hollow {loops.push(vec![nurbs_core::primitives::circle(center,tangent,0.005).unwrap().reverse().unwrap()]);}
        let budget=if periodic {0.5}else{2.};
        let mut request=json!({"op":"brep_nurbs_progressive_profile_body_reconstruct_stations","loops":loops,
            "path":path,"scale":law([1.,0.,0.]),"twist":law([0.;3]),"normal":[0.,0.,1.],"orientation":"fixed_normal","spacing":"arc_length",
            "length_tolerance":0.0001,"length_max_cells":100000,"initial_sections":33,"max_sections":129,"max_deviation":budget,
            "circle_correction_quantum":2_f64.powi(-40),"circle_correction_tolerance":1e-9,"circle_correction_max_work":1000000,
            "station_quantum":2_f64.powi(-40),"station_tolerance":0.01,"station_max_work":1000000,"smoothness_max_work":2000000});
        if joint {
            request["axis_scale"]=json!(law([1.5,0.75,1.]));
            request["center_law"]=json!(law([0.005,-0.0025,0.]));
        }
        let original=request.clone();let result=dispatch(request.clone()).unwrap();assert_eq!(request,original);
        let r=&result["stationReconstruction"];
        assert_eq!(r["boundaryCertificate"]["continuousBound"],true,"periodic={periodic}, hollow={hollow}: {r}");
        assert_eq!(r["boundaryCertificate"]["withinBudget"],true,"periodic={periodic}, hollow={hollow}: {}",r["boundaryCertificate"]);
        assert_eq!(r["boundaryCertificate"]["budget"],budget);
        assert_eq!(r["profileSmoothness"]["profile"]["exactG1G2Certified"],true,"{}",r["profileSmoothness"]);
        assert_eq!(r["profileSmoothness"]["station"]["stationG2Certified"],true,"{}",r["profileSmoothness"]);
        let model:brep_core::Model=field(&r["reconstruction"],"model").unwrap();
        assert_eq!(model.shells.len(),if hollow {2}else{1});
        let agreement=brep_core::boundary_agreement::verify_exact(&model,1000000).unwrap();
        assert!(agreement.all_equal&&agreement.all_joins_exact,"work={}, unresolved={:?}",agreement.work,agreement.uses.iter().filter(|u|!u.decision.as_ref().is_some_and(|d|d.outcome==cad_predicates::BezierIdentity::Equal)).map(|u|(u.face,u.coedge)).collect::<Vec<_>>());
        let global=dispatch(json!({"op":"brep_sweep_volume_charts_audit","model":model,"capFaces":[],
            "toleranceUv":1e-8,"maxExactWork":1000000,"maxTrimPairs":10000,"maxTrimCells":100000,"maxTrimDomainCells":1000000,
            "maxSpans":1024,"maxLinearCells":100000,"maxPairs":20000,"maxCells":200000,"maxDomainCells":1000000,
            "cellsPerPair":1000,"domainCellsPerPair":10000,"nestingPairs":1000,"nestingCells":100000,"nestingDomainCells":1000000,
            "orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100,
            "capBudgets":{"maxWalls":1024,"maxExactWork":1000000,"maxChartCells":1000,"maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":1000000}})).unwrap();
        assert_eq!(global["solidGeometryCertified"],true,"periodic={periodic}, hollow={hollow}: {global}");
        assert_eq!(global["allFacesInjective"],true);assert_eq!(global["allPairsClassified"],true);
        assert_eq!(global["retainedWallCharts"]["allChartsCertified"],true);
        assert_eq!(global["retainedWallCharts"]["charts"].as_array().unwrap().len(),model.faces.len());
        eprintln!("periodic FixedNormal retained body: joint={joint}, periodic={periodic}, hollow={hollow}, faces={}, bound={}",model.faces.len(),r["boundaryCertificate"]["errorUpper"]);
    }}}
}
