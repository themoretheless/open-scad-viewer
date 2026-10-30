use super::*;
fn config(cells: usize) -> Value {
    json!({"shell":{"exactWork":100_000_000,"driverCells":10000,"regions":{
        "region":{"pairs":10000,"regionCells":10000,"domainCells":10000,"agreementCells":10000},
        "searchCells":10000,"pointChecks":10000,"mappingCells":10000,"driverCells":10000,
        "membershipCells":10000,"controls":10000,"windingCells":10000,"steps":64}},
        "embedding":{"toleranceUv":1e-8,"corners":10000,"spans":20000,"linearCells":10000,
            "exactWork":10000000,"driverCells":10000,"pairs":{"pairs":10000,"cells":10000,"domainCells":100000,"cellsPerPair":32,"domainCellsPerPair":512}},
        "volume":{"axis":2,"origin":0.,"absoluteError":0.25,"toleranceUv":1e-8,"cells":cells,"spans":100000,"domainCells":100000}})
}
#[test]
fn projective_json_dispatch_preserves_preview_wall_coverage_and_step() {
    let model=brep_core::cuboid([0.,0.,0.],[10.,20.,30.]).unwrap();
    let shell=brep_core::source_support_shell::prepare_projective(&model,brep_core::source_support_shell::Limits{
        tolerance_uv:1e-8,faces:4096,uses:100000,exact_work:100000000,
        regions:brep_core::trimmed_face_recipe::Limits{pairs:10000,region_cells:10000,domain_cells:10000,agreement_cells:10000}}).unwrap().shell.unwrap();
    let native_limits=limits(&config(50000)).unwrap();
    let geometry=brep_core::source_shell_geometry::qualify_projective(shell,native_limits.embedding).unwrap().geometry.unwrap();
    let body=brep_core::source_volume::qualify(geometry,native_limits.volume).unwrap().body.unwrap();
    let original=body.definition().unwrap();
    let request=json!({"op":"cad_source_body_restore","definition":original.clone(),"limits":config(50000),"endpointSpans":10000,
        "displaySegments":4,"faceDisplay":{"divisions":2,"toleranceUv":1e-8,"domainCellsPerFace":10000},
        "wallScan":{"minimumMm":11.,"toleranceUv":1e-8,"grid":1,"maxAttempts":6,
            "limits":{"cells":10000,"domainCells":10000,"normalSpans":10000,"maxSineSquared":1e-12}},
        "wallCoverage":{"minimumMm":9.99,"limits":{"pairs":21,"planeControls":10000,"normalSpans":10000,"gapCells":10000,"gapSpans":20000,"maxSineSquared":1e-6}},
        "stepExchange":{"toleranceMm":1e-7,"trimWork":1000000,"limits":{"rootChecks":10000,"mappingCells":100000,
            "replayMappingPerUse":10000,"exactWork":100000000,"driverCells":10000,"spans":100000,"endpoints":10000}}});
    let response:Value=value_codec::from_str(&super::super::execute(&value_codec::to_string(&request).unwrap())).unwrap();
    assert_eq!(response["ok"],json!(true),"{response:?}");let response=&response["value"];
    assert_eq!(response["admitted"],json!(true));assert_eq!(response["sourceBody"],original);
    assert_eq!(response["faceCount"],json!(6));assert_eq!(response["edges"].as_array().unwrap().len(),12);
    assert_eq!(response["displayFaces"].as_array().unwrap().len(),6);
    assert_eq!(response["wallScan"]["thinFound"],json!(true));assert_eq!(response["wallScan"]["wholeWallQualified"],json!(false));
    assert_eq!(response["wallCoverage"]["wholeWallQualified"],json!(true));assert_eq!(response["wallCoverage"]["totalPairs"],json!(21));
    assert_eq!(response["stepExchange"]["prepared"],json!(true));assert!(response["stepExchange"]["text"].as_str().unwrap().contains("ISO-10303-21"));
    let mut ids=std::collections::BTreeMap::new();
    for edge in response["edges"].as_array().unwrap() {
        let segments=edge["displaySegments"].as_array().unwrap();assert_eq!(segments.len(),4);
        for end in 0..2 {
            let id=edge["vertices"][end].as_u64().unwrap();
            let segment=&segments[if end==0 {0}else{3}];let point=segment[1][end].clone();
            if let Some(previous)=ids.insert(id,point.clone()) {assert_eq!(previous,point);}
        }
    }
    assert_eq!(ids.len(),8);
    // Export only native-authored inputs and the checked response for the
    // independent WASM/worker replay; saved admission never grants authority.
    if let Ok(directory)=std::env::var("CAD_PROJECTIVE_WORKER_FIXTURE_DIR") {
        let directory=std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(directory.join("request.json"),value_codec::to_string(&request).unwrap()).unwrap();
        std::fs::write(directory.join("native-response.json"),value_codec::to_string(response).unwrap()).unwrap();
    }
    let mut exhausted=request.clone();exhausted["limits"]["embedding"]["pairs"]["pairs"]=json!(1);
    let denied=super::super::dispatch(exhausted).unwrap();assert_eq!(denied["admitted"],json!(false));assert!(denied["sourceBody"].is_null());
    let mut forged=request.clone();forged["definition"]["shell"]["pairs"][0]["edge"]["uses"][0]["weights"]=json!([1.,2.]);
    assert_eq!(super::super::dispatch(forged).unwrap()["admitted"],json!(false));
    let mut invalid=request;invalid["endpointSpans"]=json!(0);assert!(super::super::dispatch(invalid).is_err());
    assert_eq!(body.definition().unwrap(),original);
}
#[test]
fn json_dispatch_restores_body_and_preserves_exact_edge_addresses() {
    let spans = brep_core::linear_canal::construct(
        [[0., 0., 0.], [0., 0., 2.]],
        [0.5, 0.5],
        [1., 0., 0.],
        std::f64::consts::TAU,
    )
    .unwrap();
    let shell = brep_core::linear_canal::to_capped_source_shell(
        &spans,
        1e-7,
        1e-8,
        brep_core::trimmed_face_recipe::Limits {
            pairs: 10000,
            region_cells: 10000,
            domain_cells: 10000,
            agreement_cells: 10000,
        },
        100_000_000,
    )
    .unwrap()
    .shell
    .unwrap();
    let network = brep_core::source_boundary_network::inspect(&shell, 10000, 10000).unwrap();
    assert!(!network.vertices.is_empty());
    assert_eq!(
        network.vertices.iter().map(|v| v.ends.len()).sum::<usize>(),
        2 * shell.edges().len()
    );
    assert_eq!(
        network
            .vertices
            .iter()
            .map(|v| v.poles.len())
            .sum::<usize>(),
        shell.poles().len()
    );
    assert!(brep_core::source_boundary_network::inspect(&shell, 1, 10000).is_err());
    assert!(brep_core::source_boundary_network::inspect(&shell, 10000, 1).is_err());
    let recovered = brep_core::source_shell_restore::restore(
        shell.definition().unwrap(),
        &limits(&config(50000)).unwrap().shell,
    )
    .unwrap()
    .shell
    .unwrap();
    assert_eq!(
        network,
        brep_core::source_boundary_network::inspect(&recovered, 10000, 10000).unwrap()
    );
    for region in shell.regions().unwrap() {
        assert!(region.whole_chart_material());
        let preview =
            brep_core::source_region_display::prepare(region, 4, 1e-8, 10000).unwrap();
        assert_eq!(preview.tiles.len(), 16);
        assert_eq!(preview.domain_cells, 0);
        assert!(preview.unresolved.is_empty());
        for tile in preview.tiles {
            assert!(tile.corners.iter().flatten().all(|x| x.is_finite()));
        }
    }
    let definition = json!({"version":1,"shell":shell.definition().unwrap(),"success":true,"volume":[1.,1.]});
    // Browser binary transport uses integer tags for integral numbers.
    fn wire_numbers(v: &mut Value) {
        match v {
            Value::Number(value_codec::Number::Float(n))
                if n.fract() == 0.
                    && n.abs() <= 9007199254740991.
                    && !(*n == 0. && n.is_sign_negative()) =>
            {
                *v = if *n >= 0. {
                    json!(*n as u64)
                } else {
                    json!(*n as i64)
                };
            }
            Value::Array(a) => a.iter_mut().for_each(wire_numbers),
            Value::Object(o) => o.values_mut().for_each(wire_numbers),
            _ => {}
        }
    }
    let mut wire_definition = definition.clone();
    wire_numbers(&mut wire_definition);

    let run = |cells| {
        let request = json!({"op":"cad_source_body_restore","definition":wire_definition.clone(),"limits":config(cells),"endpointSpans":10000,"displaySegments":8,"faceDisplay":{"divisions":4,"toleranceUv":1e-8,"domainCellsPerFace":10000}});
        let text = value_codec::to_string(&request).unwrap();
        let result = super::super::execute(&text);
        {
            let response = value_codec::from_str::<Value>(&result).unwrap();
            assert_eq!(response["ok"], json!(true), "{response:?}");
            response["value"].clone()
        }
    };
    let invalid = super::super::dispatch(json!({"op":"cad_source_body_restore",
        "definition":definition.clone(),"limits":config(1),"endpointSpans":0}));
    assert!(invalid.is_err());
    let denied = run(1);
    assert_eq!(denied["admitted"], json!(false), "{denied:?}");
    assert!(denied["sourceBody"].is_null());
    assert_eq!(
        denied["diagnostics"]["reason"],
        json!("source-volume-initial-work-limit")
    );
    let restored = run(50000);
    let seam_option = json!({"edge":0,"endpointLimitsExactWork":100,"limits":{"maxSineSquared":1e-6,"cells":100,"curveSpans":100,"normalSpans":100}});
    let checked = super::super::dispatch(json!({"op":"cad_source_body_restore",
        "definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,
        "seamQualification":seam_option.clone()}))
    .unwrap();
    assert_eq!(checked["admitted"], json!(true));
    let seam = &checked["seamQualification"];
    assert_eq!(seam["request"], seam_option);
    assert!(seam["qualified"].as_bool().is_some());
    assert!(seam["reason"].as_str().unwrap().starts_with("source-seam-"));
    assert!(seam["cells"].as_u64().unwrap() <= 100);
    assert!(seam["normalSpans"].as_u64().unwrap() <= 100);
    if seam["qualified"] == json!(true) {
        assert!(seam["uncertainCanonical"].is_null());
        assert!(seam["sineSquaredBounds"].as_array().is_some());
    } else {
        assert!(seam["sineSquaredBounds"].is_null());
    }
    for bad in [json!({"edge":999999,"limits":seam_option["limits"]}),
        json!({"edge":0,"endpointLimitsExactWork":0,"limits":seam_option["limits"]}),
        json!({"edge":0,"endpointLimitsExactWork":100000001,"limits":seam_option["limits"]}),
        json!({"edge":0,"limits":{"maxSineSquared":1e-6,"cells":0,"curveSpans":100,"normalSpans":100}})] {
        assert!(super::super::dispatch(json!({"op":"cad_source_body_restore",
            "definition":wire_definition.clone(),"limits":config(1),"endpointSpans":10000,
            "seamQualification":bad}))
            .is_err()
        );
    }

    let wall_option = json!({"groups":[[0],[1]],"minimumMm":0.1,"toleranceMm":0.01,"toleranceUv":1e-8,
        "grid":1,"maxAttempts":1,"limits":{"gapCells":1,"gapSpans":100,"cells":100,
            "domainCells":100,"normalSpans":100,"maxSineSquared":1e-6}});
    let checked_wall = super::super::dispatch(json!({"op":"cad_source_body_restore",
        "definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,
        "wallQualification":wall_option.clone()}))
    .unwrap();
    let wall = &checked_wall["wallQualification"];
    assert_eq!(wall["request"], wall_option);
    assert_eq!(wall["qualified"], json!(false));
    assert_eq!(wall["converged"], json!(false));
    assert!(wall["intervalMm"].is_null());
    assert!(wall["witness"].is_null());
    assert!(wall["clearance"]["cells"].as_u64().unwrap() <= 1);
    assert!(wall["search"]["attempts"].as_u64().unwrap() <= 1);
    for bad_groups in [json!([[0], [0]]), json!([[999999], [1]]), json!([[], [1]])] {
        let mut bad = wall_option.clone();
        bad["groups"] = bad_groups;
        assert!(
            super::super::dispatch(json!({"op":"cad_source_body_restore",
            "definition":wire_definition.clone(),"limits":config(1),"endpointSpans":10000,
            "wallQualification":bad}))
            .is_err()
        );
    }
    let coverage_option = json!({"minimumMm":0.1,"limits":{"pairs":1,"planeControls":1000,
        "normalSpans":100,"gapCells":100,"gapSpans":100,"maxSineSquared":1e-6}});
    let covered = super::super::dispatch(json!({"op":"cad_source_body_restore",
        "definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,
        "wallCoverage":coverage_option.clone()})).unwrap();
    let coverage=&covered["wallCoverage"];
    let mut retained_option=coverage_option.clone();
    retained_option["retainedGap"]=json!({"cells":100,"spans":200,"domainCells":1000,"cellsPerPair":10,"spansPerPair":20,"domainCellsPerPair":100,"domainCellsPerQuery":10,"toleranceUv":1e-8});
    let retained=super::super::dispatch(json!({"op":"cad_source_body_restore","definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,"wallCoverage":retained_option.clone()})).unwrap();
    assert_eq!(retained["wallCoverage"]["request"],retained_option);
    assert_eq!(retained["wallCoverage"]["retainedGap"]["cells"],json!(0));assert_eq!(retained["wallCoverage"]["retainedGap"]["pairs"].as_array().unwrap().len(),0);
    for (key,value) in [("cells",json!(0)),("spans",json!(100001)),("domainCells",json!(1000001)),("cellsPerPair",json!(0)),("spansPerPair",json!(0)),("domainCellsPerPair",json!(0)),("domainCellsPerQuery",json!(0)),("toleranceUv",json!(0))] {
        let mut invalid=retained_option.clone();invalid["retainedGap"][key]=value;
        assert!(super::super::dispatch(json!({"op":"cad_source_body_restore","definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,"wallCoverage":invalid})).is_err());
    }
    let mut mixed_option=coverage_option.clone();
    mixed_option["mixedProducts"]=retained_option["retainedGap"].clone();
    mixed_option["mixedProducts"]["normalSpans"]=json!(100);
    mixed_option["mixedProducts"]["normalSpansPerPair"]=json!(20);
    let mixed=super::super::dispatch(json!({"op":"cad_source_body_restore","definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,"wallCoverage":mixed_option.clone()})).unwrap();
    assert_eq!(mixed["wallCoverage"]["request"],mixed_option);
    assert_eq!(mixed["wallCoverage"]["mixedProducts"]["cells"],json!(0));
    assert_eq!(mixed["wallCoverage"]["mixedProducts"]["normalSpans"],json!(0));
    assert_eq!(mixed["wallCoverage"]["mixedProducts"]["pairs"].as_array().unwrap().len(),0);
    for key in ["cells","spans","domainCells","cellsPerPair","spansPerPair","domainCellsPerPair","domainCellsPerQuery","normalSpans","normalSpansPerPair","toleranceUv"] {
        let mut invalid=mixed_option.clone();invalid["mixedProducts"][key]=json!(0);
        assert!(super::super::dispatch(json!({"op":"cad_source_body_restore","definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,"wallCoverage":invalid})).is_err());
    }
    assert_eq!(coverage["request"],coverage_option);
    assert_eq!(coverage["wholeWallQualified"],json!(false));
    assert_eq!(coverage["enumerationComplete"],json!(false));
    assert!(coverage["lowerMm"].is_null());
    assert_eq!(coverage["pairs"].as_array().unwrap().len(), 1);
    for (key, value) in [
        ("pairs", json!(0)),
        ("planeControls", json!(1000001)),
        ("normalSpans", json!(0)),
        ("gapCells", json!(0)),
        ("gapSpans", json!(100001)),
        ("maxSineSquared", json!(1.)),
    ] {
        let mut bad = coverage_option.clone();
        bad["limits"][key] = value;
        assert!(
            super::super::dispatch(json!({"op":"cad_source_body_restore",
            "definition":wire_definition.clone(),"limits":config(1),"endpointSpans":10000,
            "wallCoverage":bad}))
            .is_err()
        );
    }
    let mut adaptive_option = coverage_option.clone();
    adaptive_option["adaptiveSelf"] =
        json!({"planeControls":1000,"cells":1,"spans":1,"cellsPerFace":1,"spansPerFace":1});
    let adaptive = super::super::dispatch(json!({"op":"cad_source_body_restore",
        "definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,
        "wallCoverage":adaptive_option.clone()})).unwrap();
    let a=&adaptive["wallCoverage"];
    assert_eq!(a["request"],adaptive_option);
    assert_eq!(a["wholeWallQualified"],json!(false));
    assert!(a["adaptiveSelf"]["faces"].as_array().unwrap().len()>0);
    assert!(a["adaptiveSelf"]["cells"].as_u64().unwrap()<=1);
    assert!(a["adaptiveSelf"]["spans"].as_u64().unwrap()<=1);
    for face in a["adaptiveSelf"]["faces"].as_array().unwrap() {
        let pairs=face["unresolvedPairs"].as_array().unwrap();
        assert_eq!(pairs.len() as u64,face["pending"].as_u64().unwrap());
        assert_eq!(face["endpointExchangeSymmetric"],json!(true));
        if let Some(last)=pairs.last() {assert_eq!(last,&face["uncertain"]);}
        else {assert_eq!(face["uncertain"],Value::Null);}
    }

    let mut pole_option=adaptive_option.clone();
    pole_option["adaptiveSelf"]["pole"]=json!({"exactWork":1,"cells":1,"spans":1,"cellsPerFace":1,"spansPerFace":1});
    let pole=super::super::dispatch(json!({"op":"cad_source_body_restore",
        "definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,
        "wallCoverage":pole_option.clone()})).unwrap();
    assert_eq!(pole["wallCoverage"]["request"],pole_option);
    for counter in ["poleExactWork","poleCells","poleSpans"] {
        assert!(pole["wallCoverage"]["adaptiveSelf"][counter].as_u64().unwrap()<=1);
    }
    for key in ["exactWork","cells","spans","cellsPerFace","spansPerFace"] {
        for value in [json!(0),json!(100000001)] {
            let mut bad=pole_option.clone();bad["adaptiveSelf"]["pole"][key]=value;
            assert!(super::super::dispatch(json!({"op":"cad_source_body_restore",
                "definition":wire_definition.clone(),"limits":config(1),"wallCoverage":bad})).is_err());
        }
    }

    let mut refined_option=adaptive_option.clone();
    refined_option["adaptiveSelf"]["refinement"]=json!({"cells":1,"spans":1,"cellsPerFace":1,"spansPerFace":1});
    let refined=super::super::dispatch(json!({"op":"cad_source_body_restore",
        "definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,
        "wallCoverage":refined_option.clone()})).unwrap();
    assert_eq!(refined["wallCoverage"]["request"],refined_option);
    for counter in ["refinementCells","refinementSpans"] {
        assert!(refined["wallCoverage"]["adaptiveSelf"][counter].as_u64().unwrap()<=1);
    }
    for key in ["cells","spans","cellsPerFace","spansPerFace"] {
        for value in [json!(0),json!(100001)] {
            let mut bad=refined_option.clone();bad["adaptiveSelf"]["refinement"][key]=value;
            assert!(super::super::dispatch(json!({"op":"cad_source_body_restore",
                "definition":wire_definition.clone(),"limits":config(1),"wallCoverage":bad})).is_err());
        }
    }

    for key in ["planeControls","cells","spans","cellsPerFace","spansPerFace"] {
        let mut bad=adaptive_option.clone();bad["adaptiveSelf"][key]=json!(0);
        assert!(super::super::dispatch(json!({"op":"cad_source_body_restore",
            "definition":wire_definition.clone(),"limits":config(1),"wallCoverage":bad})).is_err());
    }
    let scan_option = json!({"minimumMm":7.,"toleranceUv":1e-7,"grid":1,"maxAttempts":1,
        "limits":{"cells":100,"domainCells":100,"normalSpans":100,"maxSineSquared":1e-6}});
    let scanned = super::super::dispatch(
        json!({"op":"cad_source_body_restore","definition":wire_definition.clone(),
        "limits":config(50000),"endpointSpans":10000,"wallScan":scan_option.clone()}),
    )
    .unwrap();
    let scan = &scanned["wallScan"];
    assert_eq!(scan["request"], scan_option);
    assert_eq!(scan["wholeWallQualified"], json!(false));
    assert_eq!(scan["proposalsExhausted"], json!(false));
    assert_eq!(scan["attempts"], json!(1));
    assert_eq!(scan["facesVisited"], json!(1));
    let bad_scan = json!({"minimumMm":7.,"toleranceUv":1e-7,"grid":3,"maxAttempts":0,
        "limits":{"cells":100,"domainCells":100,"normalSpans":100,"maxSineSquared":1e-6}});
    assert!(
        super::super::dispatch(
            json!({"op":"cad_source_body_restore","definition":wire_definition.clone(),
        "limits":config(1),"endpointSpans":10000,"wallScan":bad_scan})
        )
        .is_err()
    );
    let mut bad = wall_option.clone();
    bad["limits"]["cells"] = json!(0);
    assert!(
        super::super::dispatch(json!({"op":"cad_source_body_restore",
        "definition":wire_definition.clone(),"limits":config(1),"endpointSpans":10000,
        "wallQualification":bad}))
        .is_err()
    );
    if let Some(path) = std::env::var_os("CAD_SOURCE_BODY_FIXTURE_OUTPUT") {
        let request = json!({"op":"cad_source_body_restore","definition":definition.clone(),"limits":config(50000),"endpointSpans":10000});
        std::fs::write(path, value_codec::to_string(&request).unwrap()).unwrap();
    }
    assert_eq!(restored["admitted"], json!(true), "{restored:?}");
    assert_eq!(restored["sourceBody"]["shell"], definition["shell"]);
    assert_eq!(
        restored["edges"].as_array().unwrap().len(),
        shell.edges().len()
    );
    for (index, edge) in restored["edges"].as_array().unwrap().iter().enumerate() {
        assert_eq!(edge["definition"], shell.edges()[index].definition());
        assert_eq!(edge["index"], json!(index));
        let display = edge["displaySegments"].as_array().unwrap();
        assert_eq!(display.len(), 8);
        for segment in display {
            assert_eq!(segment.as_array().unwrap().len(), 3);
        }
        assert_eq!(
            edge["uses"],
            json!(shell.uses()[index].map(|a| [a.face, a.wire, a.edge]))
        );
    }
    let mut anchors = std::collections::BTreeMap::new();
    for edge in restored["edges"].as_array().unwrap() {
        let segments = edge["displaySegments"].as_array().unwrap();
        for end in 0..2 {
            let id = edge["vertices"][end].as_u64().unwrap();
            let point = segments[if end == 0 { 0 } else { segments.len() - 1 }][1][end].clone();
            if let Some(previous) = anchors.insert(id, point.clone()) {
                assert_eq!(previous, point);
            }
        }
    }
    let faces = restored["displayFaces"].as_array().unwrap();
    assert_eq!(faces.len(), shell.faces().len());
    for face in faces {
        assert_eq!(face["tiles"].as_array().unwrap().len(), 16);
        assert!(face["unresolved"].as_array().unwrap().is_empty());
    }
    let fresh = brep_core::source_body_restore::restore(
        restored["sourceBody"].clone(),
        limits(&config(50000)).unwrap(),
    )
    .unwrap();
    let source_model =
        brep_core::source_body_topology::convert(fresh.body().unwrap(), 1e-7, 10000, 10000)
            .unwrap();
    assert_eq!(source_model.definition(), &restored["sourceBody"]);
    let topology = source_model.topology();
    assert_eq!(
        topology.edges.len(),
        shell.edges().len() + shell.poles().len()
    );
    assert_eq!(
        topology.edges.iter().filter(|e| e.degenerate).count(),
        shell.poles().len()
    );
    for edge in &topology.edges {
        match &edge.curve {
            brep_core::source_body_topology::Carrier::Edge(_) => assert!(!edge.degenerate),
            brep_core::source_body_topology::Carrier::Pole(pole) => {
                assert!(edge.degenerate);
                assert_eq!(edge.vertices[0], edge.vertices[1]);
                let bounds = topology.vertices[edge.vertices[0]].point.bounds;
                assert_eq!(bounds, pole.point().map(|p| [p, p]));
            }
        }
    }
    let oracle = 2. * std::f64::consts::PI / 3.;
    assert!(
        restored["volume"][0].as_f64().unwrap() <= oracle
            && oracle <= restored["volume"][1].as_f64().unwrap()
    );
}
