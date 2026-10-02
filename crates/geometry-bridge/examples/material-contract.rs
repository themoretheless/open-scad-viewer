use value_codec::json;
fn main() {
    let path = std::env::args().nth(1).expect("output JSON path");
    let cube = brep_core::cuboid([0.;3],[10.;3]).unwrap();
    let cavity = brep_core::operations::boolean(&cube,&brep_core::cuboid([4.;3],[6.;3]).unwrap(),"difference").unwrap();
    let annular = brep_core::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
    let limits = json!({"pointCells":100000,"pointDomainCells":1000000,"segmentCells":100000,"segmentDomainCells":1000000,
        "validity":{"exactWork":1000000,"trimPairs":10000,"trimCells":100000,"trimDomainCells":1000000,"spans":4096,
            "facePairs":10000,"faceCells":150000,"faceDomainCells":1500000,"faceCellsPerPair":1024,"faceDomainCellsPerPair":100000,
            "nestingPairs":100,"nestingCells":100000,"nestingDomainCells":1000000,"orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100}});
    let mut cases = Vec::new();
    for (name,op,model,origin,direction,expected) in [
        ("cube-interior","cad_material_segment",&cube,[2.,5.,5.],[6.,0.,0.],true),
        ("cube-chord","cad_material_chord",&cube,[-2.,5.,5.],[14.,0.,0.],true),
        ("cavity-interior","cad_material_segment",&cavity,[2.,5.,5.],[6.,0.,0.],false),
        ("cavity-chord","cad_material_chord",&cavity,[-2.,5.,5.],[14.,0.,0.],false),
        ("annular-wall","cad_material_chord",&annular,[25.,2.,3.],[-24.,0.,0.],true),
        ("annular-hole","cad_material_chord",&annular,[25.,2.,3.],[-50.,0.,0.],false),
    ] {
        let input=json!({"op":op,"model":model,"origin":origin,"direction":direction,"toleranceUv":1e-7,"limits":limits});
        let result=geometry_bridge::dispatch(input.clone()).unwrap();
        assert_eq!(result["proven"],json!(expected));
        cases.push(json!({"name":name,"request":input,"result":result}));
    }
    let mut exhausted=cases[1]["request"].clone();
    exhausted["limits"]["segmentCells"]=json!(1);
    let result=geometry_bridge::dispatch(exhausted.clone()).unwrap();
    assert_eq!(result["proven"],json!(false));
    cases.push(json!({"name":"chord-work-limit","request":exhausted,"result":result}));
    if std::env::args().nth(2).as_deref()==Some("wall") {
        let face=|x|cube.faces.iter().position(|f|f.surface.control_points.iter().flatten().all(|p|p[0]==x)).unwrap();
        let a=vec![2,7,12,16,20,24];let b=vec![3,8,13,17,21,25];
        let mut wall_cases=Vec::new();
        for(name,index,origin,direction,groups,spans,cells,converged) in [
            ("cube-wall",1,[-2.,5.,5.],[14.,0.,0.],[vec![face(0.)],vec![face(10.)]],100,10000,true),
            ("cube-normal-budget",1,[-2.,5.,5.],[14.,0.,0.],[vec![face(0.)],vec![face(10.)]],1,10000,false),
            ("annular-wall",4,[15.,20.,3.],[-14.4,-19.2,0.],[a.clone(),b.clone()],100,10000,true),
            ("annular-wide",4,[15.,20.,3.],[-14.4,-19.2,0.],[vec![2,7,12,16,20,24,0,5,10],b.clone()],100,1,false),
            ("annular-hole",4,[25.,2.,3.],[-50.,0.,0.],[a,b],100,10000,false),
        ] {
            let mut input=cases[index]["request"].clone();input["op"]=json!("cad_material_wall");
            input["origin"]=json!(origin);input["direction"]=json!(direction);input["faceGroups"]=json!(groups);
            input["normalAudit"]=json!({"maxSineSquared":1e-6,"maxSpans":spans});input["toleranceMm"]=json!(1e-5);
            input["maxDistanceCells"]=json!(cells);input["maxDistanceDomainCells"]=json!(1000000);
            let result=geometry_bridge::dispatch(input.clone()).unwrap();assert_eq!(result["converged"],json!(converged));
            wall_cases.push(json!({"name":name,"request":input,"result":result}));
        }
        cases=wall_cases;
    }
    if std::env::args().nth(2).as_deref()==Some("normal") {
        let mut normal_cases=Vec::new();
        for (name,index,direction,spans,expected) in [
            ("cube-normal",1,[14.,0.,0.],2,Some(true)),
            ("cube-oblique",1,[14.,4.,0.],2,Some(false)),
            ("cube-normal-budget",1,[14.,0.,0.],1,None),
            ("annular-oblique",4,[-24.,0.,0.],100,Some(false)),
            ("annular-radial",4,[-14.4,-19.2,0.],100,Some(true)),
        ] {
            let mut input=cases[index]["request"].clone();
            input["direction"]=json!(direction);
            if name=="annular-radial" {input["origin"]=json!([15.,20.,3.]);}
            input["normalAudit"]=json!({"maxSineSquared":1e-6,"maxSpans":spans});
            let result=geometry_bridge::dispatch(input.clone()).unwrap();
            assert_eq!(result["normalEvidence"]["aligned"],json!(expected));
            normal_cases.push(json!({"name":name,"request":input,"result":result}));
        }
        cases=normal_cases;
    }
    std::fs::write(path,value_codec::to_string(&json!({"cases":cases})).unwrap()).unwrap();
}
