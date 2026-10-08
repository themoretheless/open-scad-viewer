//! Native bridge acceptance. Does not qualify a packaged WASM artifact.
use value_codec::{json,Value};
fn main() {
    let args:Vec<_>=std::env::args().collect();
    let original:Value=value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let before=value_codec::to_string(&original["definition"]).unwrap();
    let mut cases=Vec::new();let mut previously_proven=Vec::new();
    for (cells,adaptive,factored,pole,expanded) in [(0,false,false,false,false),(1,false,false,false,false),(50000,false,false,false,false),(50000,true,false,false,false),(50000,true,true,false,false),(50000,true,true,true,false),(50000,true,true,true,true)] {
        let mut request=original.clone();
        request["displaySegments"]=json!(0);
        request["wallCoverage"]=json!({"minimumMm":5.99,"limits":{
            "pairs":378,"planeControls":10000,"normalSpans":20000,
            "gapCells":1000,"gapSpans":2000,"maxSineSquared":1e-6}});
        if cells>0 {request["wallCoverage"]["normalProducts"]=json!({"cells":cells,"cellsPerPair":128});}
        if adaptive {request["wallCoverage"]["adaptiveSelf"]=json!({"planeControls":10000,
            "cells":10000,"spans":100000,"cellsPerFace":512,"spansPerFace":4096});}
        if factored {request["wallCoverage"]["factoredNormals"]=json!({"exactWork":100000000,
            "queries":10000,"cells":10000,"cellsPerPair":128});}
        if pole {request["wallCoverage"]["adaptiveSelf"]["pole"]=json!({"exactWork":100000000,
            "cells":100000,"spans":100000,"cellsPerFace":4096,"spansPerFace":100000});}
        if expanded {
            request["wallCoverage"]["adaptiveSelf"]["cells"]=json!(50000);
            request["wallCoverage"]["adaptiveSelf"]["cellsPerFace"]=json!(8192);
            request["wallCoverage"]["adaptiveSelf"]["spansPerFace"]=json!(10000);
        }
        let result=geometry_bridge::dispatch(request.clone()).unwrap();
        assert_eq!(result["admitted"],json!(true));
        let coverage=&result["wallCoverage"];
        assert_eq!(coverage["request"],request["wallCoverage"]);
        assert_eq!(coverage["enumerationComplete"],json!(true));
        assert_eq!(coverage["wholeWallQualified"],json!(false));
        assert!(coverage["lowerMm"].is_null());
        assert!(coverage["normalProductCells"].as_u64().unwrap()<=cells);
        let pairs=coverage["pairs"].as_array().unwrap();assert_eq!(pairs.len(),378);
        let unresolved=pairs.iter().filter(|p|p["proven"]==json!(false)).count();
        if cells<=1 {assert_eq!(unresolved,257);}else{assert!(unresolved<=177);}
        if factored {
            assert!(coverage["factoredExactWork"].as_u64().unwrap()<=100000000);
            assert!(coverage["factoredQueries"].as_u64().unwrap()<=10000);
            assert!(coverage["factoredProductCells"].as_u64().unwrap()<=10000);
            assert!(unresolved<173,"factored directions must improve the confirmed baseline");
        }else{
            for key in ["factoredExactWork","factoredQueries","factoredProductCells"] {
                assert_eq!(coverage[key],json!(0));
            }
        }
        if pole {
            let a=&coverage["adaptiveSelf"];
            assert!(a["poleExactWork"].as_u64().unwrap()<=100000000);
            assert!(a["poleCells"].as_u64().unwrap()<=100000);
            assert!(a["poleSpans"].as_u64().unwrap()<=100000);
            assert!(unresolved<169,"pole facts must retain and extend earlier certificates");
        }
        if expanded {assert!(unresolved<167,"regular self-face refinement must extend the baseline");}
        for faces in &previously_proven {
            assert!(pairs.iter().any(|p|&p["faces"]==faces&&p["proven"]==json!(true)));
        }
        previously_proven=pairs.iter().filter(|p|p["proven"]==json!(true))
            .map(|p|p["faces"].clone()).collect();
        assert_eq!(before,value_codec::to_string(&request["definition"]).unwrap());
        eprintln!("cells={cells} adaptive={adaptive} unresolved={unresolved}");
        cases.push(json!({"cells":cells,"adaptive":adaptive,"factored":factored,"pole":pole,"expanded":expanded,"coverage":coverage}));
    }
    std::fs::write(&args[2],value_codec::to_string_pretty(&json!({
        "scope":"Native bridge only; no whole-wall or self-intersection certificate",
        "originalDefinitionUnchanged":true,"cases":cases})).unwrap()).unwrap();
}
