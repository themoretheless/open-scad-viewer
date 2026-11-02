use geometry_bridge::dispatch;
use value_codec::{Value, json};
fn request() -> Value {
    let source = json!({"degreeU":3,"degreeV":1,"knotsU":[0,0,0,0,1,1,1,1],"knotsV":[0,0,1,1],"controlPoints":[[[0.5625,-0.28125,0],[0.5625,-0.28125,2]],[[-0.4375,0.40625,0],[-0.4375,0.40625,2]],[[-0.4375,-0.40625,0],[-0.4375,-0.40625,2]],[[0.5625,0.28125,0.25],[0.5625,0.28125,2.25]]],"weights":[[1,2],[1,2],[1,2],[1,2]],"periodicU":false,"periodicV":false});
    let target = json!({"degreeU":1,"degreeV":1,"knotsU":[-2,-2,2,2],"knotsV":[10,10,14,14],"controlPoints":[[[0,-1,0],[0,-1,2]],[[0,1,0],[0,1,2]]],"weights":[[1,1],[1,1]],"periodicU":false,"periodicV":false});
    json!({"op":"nurbs_surface_iso_target_window","source":source,"target":target,"axis":0,"parameter":0.5,"window":[[0,1],[-2,2],[10,14]],"maxCells":10000})
}
#[test]
fn bridge_enumerates_both_original_roots_with_explicit_window_scope() {
    let v = request();
    let before = v.clone();
    let r = dispatch(v.clone()).unwrap();
    assert_eq!(v, before);
    assert_eq!(r["schema"], "nurbs-surface-iso-target-window/1");
    assert_eq!(r["status"], "present");
    assert_eq!(r["windowCoverageComplete"], true);
    assert_eq!(r["allSurfaceBranchesEnumerated"], false);
    assert_eq!(r["trimAdmission"], false);
    assert_eq!(r["bodyAdmission"], false);
    assert_eq!(r["unresolved"], json!([]));
    let roots = r["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 2);
    for t in [0.25, 0.75] {
        let root = roots
            .iter()
            .find(|r| {
                r["parameterBox"][0][0].as_f64().unwrap() <= t
                    && t <= r["parameterBox"][0][1].as_f64().unwrap()
            })
            .unwrap();
        assert_eq!(root["method"], "original-curve-surface-krawczyk");
        let z = 4. / 3. + t * t * t / 4.;
        let box_v = &root["parameterBox"][2];
        assert!(
            box_v[0].as_f64().unwrap() <= 10. + 2. * z
                && 10. + 2. * z <= box_v[1].as_f64().unwrap()
        );
    }
}
#[test]
fn budget_exhaustion_preserves_unresolved_window() {
    let mut v = request();
    v["maxCells"] = json!(1);
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "unresolved");
    assert_eq!(r["windowCoverageComplete"], false);
    assert_eq!(r["windowAbsenceProven"], false);
    assert!(!r["unresolved"].as_array().unwrap().is_empty());
}
#[test]
fn rejects_invalid_axis_window_and_forged_evidence() {
    for (key, value) in [
        ("axis", json!(2)),
        ("window", json!([[0, 2], [-2, 2], [10, 14]])),
        ("presenceProven", json!(true)),
    ] {
        let mut v = request();
        v[key] = value;
        assert!(dispatch(v).is_err());
    }
}
#[test]
fn separated_target_proves_absence_only_in_window() {
    let mut v = request();
    for row in v["target"]["controlPoints"].as_array_mut().unwrap() {
        for p in row.as_array_mut().unwrap() {
            p[0] = json!(10);
        }
    }
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "absent-in-window");
    assert_eq!(r["windowAbsenceProven"], true);
    assert_eq!(r["allSurfaceBranchesEnumerated"], false);
}
