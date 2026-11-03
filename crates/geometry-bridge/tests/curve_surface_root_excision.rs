use geometry_bridge::dispatch;
use value_codec::{json, Value};
fn query() -> Value {
    json!({"op":"nurbs_surface_iso_target_window","source":{"degreeU":2,"degreeV":1,"knotsU":[0,0,0,1,1,1],"knotsV":[0,0,1,1],"controlPoints":[[[0,0,0.35546875],[0,1,0.35546875]],[[0.5,0,-0.26953125],[0.5,1,-0.26953125]],[[1,0,0.10546875],[1,1,0.10546875]]],"weights":[[1,1],[1,1],[1,1]],"periodicU":false,"periodicV":false},"target":{"degreeU":1,"degreeV":1,"knotsU":[0,0,1,1],"knotsV":[0,0,1,1],"controlPoints":[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],"weights":[[1,1],[1,1]],"periodicU":false,"periodicV":false},"axis":0,"parameter":0.5,"window":[[0,1],[0,1],[0,1]],"maxCells":10000})
}
#[test]
fn artificial_seam_roots_have_complete_original_iso_coverage_without_topology_credit() {
    let r = dispatch(query()).unwrap();
    assert_eq!(r["windowCoverageComplete"], json!(true));
    assert_eq!(r["roots"].as_array().unwrap().len(), 2);
    assert_eq!(r["unresolved"], json!([]));
    assert_eq!(r["allSurfaceBranchesEnumerated"], json!(false));
    assert_eq!(r["trimAdmission"], json!(false));
    assert_eq!(r["bodyAdmission"], json!(false));
    for t in [7. / 16., 13. / 16.] {
        assert_eq!(
            r["roots"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|a| a["parameterBox"][0][0].as_f64().unwrap() <= t
                    && t <= a["parameterBox"][0][1].as_f64().unwrap())
                .count(),
            1
        );
    }
}
#[test]
fn caller_window_and_budget_limits_preserve_unresolved_coverage() {
    let mut q = query();
    q["maxCells"] = json!(1);
    let r = dispatch(q).unwrap();
    assert_eq!(r["cells"], json!(1));
    assert_eq!(r["windowCoverageComplete"], json!(false));
    assert!(!r["unresolved"].as_array().unwrap().is_empty());
    let mut q = query();
    q["window"][0][0] = json!(7. / 16.);
    let r = dispatch(q).unwrap();
    assert_eq!(r["windowCoverageComplete"], json!(false));
    assert!(!r["unresolved"].as_array().unwrap().is_empty());
}
#[test]
fn topology_flags_remain_forbidden_request_fields() {
    let mut q = query();
    q["trimAdmission"] = json!(true);
    assert!(dispatch(q).is_err());
}
