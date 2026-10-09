use super::*;

fn cantilever() -> Value {
    json!({"op":"frame_solve",
        "nodesMm":[[0,0,0],[1000,0,0]],
        "members":[{"nodes":[0,1],"youngMpa":200000,"poisson":0.3,
            "areaMm2":100,"iyyMm4":1e6,"izzMm4":1e6,"jMm4":2e6}],
        "restrained":[[true,true,true,true,true,true],
                      [false,false,false,false,false,false]],
        "forcesN":[[0,0,0],[0,0,-1000]],
        "momentsNmm":[[0,0,0],[0,0,0]],
        "loads":[]})
}

#[test]
fn cantilever_tip_load_matches_analytical() {
    let r = crate::dispatch(cantilever()).unwrap();
    close(r["displacementsMm"][1][2].as_f64().unwrap(), -5. / 3.);
    close(r["reactionsN"][0][2].as_f64().unwrap(), 1000.);
    close(r["reactionMomentsNmm"][0][1].as_f64().unwrap(), -1e6);
    close(
        r["members"][0]["stations"][0]["momentYNmm"]
            .as_f64()
            .unwrap(),
        1e6,
    );
    close(
        r["members"][0]["stations"][10]["shearZN"].as_f64().unwrap(),
        -1000.,
    );
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-9 * b.abs().max(1.), "{a} != {b}");
}

#[test]
fn uniform_member_load_reaches_closed_form_reactions() {
    let mut v = cantilever();
    v["restrained"] = json!([
        [true, true, true, true, false, true],
        [false, true, true, false, false, true]
    ]);
    v["forcesN"] = json!([[0, 0, 0], [0, 0, 0]]);
    v["loads"] = json!([{"type":"uniform","member":0,
        "forceNPerMm":[0,0,-10],"localAxes":false}]);
    let r = crate::dispatch(v).unwrap();
    close(r["reactionsN"][0][2].as_f64().unwrap(), 5000.);
    close(r["reactionsN"][1][2].as_f64().unwrap(), 5000.);
    close(
        r["members"][0]["stations"][10]["momentYNmm"]
            .as_f64()
            .unwrap(),
        -1.25e6,
    );
}

#[test]
fn releases_and_unknown_fields_are_rejected() {
    let mut v = cantilever();
    v["members"][0]["releaseB"] = json!([false, true, false]);
    // Hinge at the free end orphans node B's ry DOF: singular.
    assert_eq!(crate::dispatch(v).unwrap_err().code, "FRAME_SINGULAR");
    let mut v = cantilever();
    v["members"][0]["releaseB"] = json!([true, true, false]);
    v["members"][0]["releaseA"] = json!([true, true, true]);
    assert_eq!(crate::dispatch(v).unwrap_err().code, "FRAME_INVALID_INPUT");
    let mut v = cantilever();
    v["members"][0]["E"] = json!(1);
    assert_eq!(
        crate::dispatch(v).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
    let mut v = cantilever();
    v["loads"] = json!([{"type":"spring","member":0}]);
    assert_eq!(
        crate::dispatch(v).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
    let mut v = cantilever();
    v["sections"] = json!([]);
    assert_eq!(
        crate::dispatch(v).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
}

#[test]
fn singular_model_preserves_core_error_code() {
    let mut v = cantilever();
    v["restrained"][0] = json!([false, false, false, false, false, false]);
    // Bending released at both ends, no restraints at A: rigid mechanisms.
    v["members"][0]["releaseA"] = json!([false, true, true]);
    v["members"][0]["releaseB"] = json!([false, true, true]);
    assert_eq!(crate::dispatch(v).unwrap_err().code, "FRAME_SINGULAR");
    // Torsion released at both ends: singular released block.
    let mut v = cantilever();
    v["members"][0]["releaseA"] = json!([true, false, false]);
    v["members"][0]["releaseB"] = json!([true, false, false]);
    assert_eq!(crate::dispatch(v).unwrap_err().code, "FRAME_INVALID_INPUT");
}

fn two_case_envelope() -> Value {
    json!({"op":"frame_envelope",
        "nodesMm":[[0,0,0],[1000,0,0]],
        "members":[{"nodes":[0,1],"youngMpa":200000,"poisson":0.3,
            "areaMm2":100,"iyyMm4":1e6,"izzMm4":1e6,"jMm4":2e6}],
        "restrained":[[true,true,true,true,true,true],
                      [false,false,false,false,false,false]],
        "cases":[
            {"forcesN":[[0,0,0],[0,0,-1000]],
             "momentsNmm":[[0,0,0],[0,0,0]],"loads":[]},
            {"forcesN":[[0,0,0],[0,0,600]],
             "momentsNmm":[[0,0,0],[0,0,0]],"loads":[]}],
        "combinations":[
            {"name":"G","factors":[1,0]},
            {"name":"Q","factors":[0,1]},
            {"name":"G-Q","factors":[1,-1]}]})
}

#[test]
fn envelope_matches_closed_form_extremes_and_governing_combos() {
    let r = crate::dispatch(two_case_envelope()).unwrap();
    // Tip uz: G −5/3, Q +1, G−Q −8/3.
    let tip_z = &r["displacementsMm"][1][2];
    close(tip_z["min"].as_f64().unwrap(), -8. / 3.);
    close(tip_z["max"].as_f64().unwrap(), 1.);
    assert_eq!(tip_z["minCombination"].as_u64().unwrap(), 2);
    assert_eq!(tip_z["maxCombination"].as_u64().unwrap(), 1);
    // Root moment_y envelope and station grid.
    let root = &r["members"][0]["stations"][0]["momentYNmm"];
    close(root["min"].as_f64().unwrap(), -6e5);
    close(root["max"].as_f64().unwrap(), 1.6e6);
    close(
        r["members"][0]["stations"][10]["xMm"].as_f64().unwrap(),
        500.,
    );
    close(r["maxDeflectionMm"]["max"].as_f64().unwrap(), 8. / 3.);
    assert_eq!(r["loadCases"].as_u64().unwrap(), 2);
    assert_eq!(r["combinations"][2].as_str().unwrap(), "G-Q");
    assert!(r["maxRelativeResidual"].as_f64().unwrap() < 1e-12);
}

#[test]
fn envelope_rejects_bad_combinations_and_unauthorized_fields() {
    let mut v = two_case_envelope();
    v["combinations"][0]["factors"] = json!([1.]);
    assert_eq!(crate::dispatch(v).unwrap_err().code, "COMBO_INVALID_INPUT");
    let mut v = two_case_envelope();
    v["combinations"][0]["name"] = json!("");
    assert_eq!(crate::dispatch(v).unwrap_err().code, "COMBO_INVALID_INPUT");
    let mut v = two_case_envelope();
    v["cases"][0]["selfWeight"] = json!(true);
    assert_eq!(
        crate::dispatch(v).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
    let mut v = two_case_envelope();
    v["cases"] = json!([]);
    assert_eq!(crate::dispatch(v).unwrap_err().code, "COMBO_INVALID_INPUT");
}

#[test]
fn spring_support_matches_compatibility_closed_form() {
    let mut v = cantilever();
    v["supports"] = json!([{"type":"spring","node":1,"dof":2,"stiffness":0.6}]);
    let r = crate::dispatch(v).unwrap();
    let k = 0.6f64;
    let l = 1000f64;
    let rb = 1000. * k * l.powi(3) / (3. * 2e5 * 1e6 + k * l.powi(3));
    close(r["reactionsN"][1][2].as_f64().unwrap(), rb);
    close(r["displacementsMm"][1][2].as_f64().unwrap(), -rb / k);
    close(r["reactionsN"][0][2].as_f64().unwrap(), 1000. - rb);
}

#[test]
fn lower_bound_engages_and_lifts_off_through_dispatch() {
    let mut v = cantilever();
    v["forcesN"] = json!([[0, 0, 0], [0, 0, 0]]);
    v["loads"] = json!([{"type":"uniform","member":0,
        "forceNPerMm":[0,0,-10],"localAxes":false}]);
    v["supports"] = json!([{"type":"lowerBound","node":1,"dof":2}]);
    let r = crate::dispatch(v.clone()).unwrap();
    close(r["reactionsN"][1][2].as_f64().unwrap(), 3750.);
    close(r["displacementsMm"][1][2].as_f64().unwrap(), 0.);
    // Flip the load upward: the prop cannot pull, contact opens.
    v["loads"] = json!([{"type":"uniform","member":0,
        "forceNPerMm":[0,0,10],"localAxes":false}]);
    let r = crate::dispatch(v).unwrap();
    close(r["reactionsN"][1][2].as_f64().unwrap(), 0.);
    close(r["displacementsMm"][1][2].as_f64().unwrap(), 6.25);
}

#[test]
fn supports_are_validated_and_kept_optional() {
    // No supports key at all: unchanged contract.
    let r = crate::dispatch(cantilever()).unwrap();
    close(r["reactionsN"][0][2].as_f64().unwrap(), 1000.);
    // Support on a rigidly restrained DOF.
    let mut v = cantilever();
    v["supports"] = json!([{"type":"lowerBound","node":0,"dof":2}]);
    assert_eq!(crate::dispatch(v).unwrap_err().code, "FRAME_INVALID_INPUT");
    // Unknown support type and unauthorized field.
    let mut v = cantilever();
    v["supports"] = json!([{"type":"magnet","node":1,"dof":2}]);
    assert_eq!(
        crate::dispatch(v).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
    let mut v = cantilever();
    v["supports"] = json!([{"type":"spring","node":1,"dof":2,"stiffness":1,"gap":0.1}]);
    assert_eq!(
        crate::dispatch(v).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
    // Envelope accepts supports too (unilateral → direct combo solves).
    let mut v = two_case_envelope();
    v["supports"] = json!([{"type":"lowerBound","node":1,"dof":2}]);
    let r = crate::dispatch(v).unwrap();
    // All cases press down or lift at the tip; the prop envelopes stay
    // consistent (no superposition artifacts).
    assert!(r["maxRelativeResidual"].as_f64().unwrap() < 1e-12);
}

#[test]
fn diagnose_reports_singular_dofs_and_stability_margin() {
    let mut v = cantilever();
    v["op"] = json!("frame_diagnose");
    for key in ["forcesN", "momentsNmm", "loads"] {
        v.as_object_mut().unwrap().remove(key);
    }
    // Stable cantilever: no issues, finite margin.
    let report = crate::dispatch(v.clone()).unwrap();
    assert_eq!(report["stable"], json!(true));
    assert_eq!(report["issues"], json!([]));
    assert!(report["minNormalizedPivot"].as_f64().unwrap() > 1e-12);
    // Free the fixed end: rigid-body mechanism.
    v["restrained"] = json!([
        [false, false, false, false, false, false],
        [false, false, false, false, false, false]
    ]);
    let report = crate::dispatch(v.clone()).unwrap();
    assert_eq!(report["stable"], json!(false));
    assert_eq!(report["issues"][0]["issue"], json!("mechanism"));
    assert!(report["minNormalizedPivot"].as_f64().unwrap() <= 1e-12);
    // Six springs standing in for the fixed end stabilize the beam.
    v["supports"] = json!(
        (0..6)
            .map(|dof| json!({"type":"spring","node":0,"dof":dof,"stiffness":1e9}))
            .collect::<Vec<_>>()
    );
    let report = crate::dispatch(v.clone()).unwrap();
    assert_eq!(report["stable"], json!(true));
    // Load fields are unauthorized on the diagnose op.
    v["forcesN"] = json!([[0, 0, 0], [0, 0, 0]]);
    assert_eq!(
        crate::dispatch(v).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
}

#[test]
fn buckling_column_matches_euler_and_validates_input() {
    let members: Vec<Value> = (0..4)
        .map(|i| {
            json!({"nodes":[i, i+1],"youngMpa":200000,"poisson":0.3,
            "areaMm2":100,"iyyMm4":1e6,"izzMm4":1e6,"jMm4":2e6})
        })
        .collect();
    let mut restrained: Vec<Value> = (0..5)
        .map(|_| json!([false, false, false, false, false, false]))
        .collect();
    restrained[0] = json!([true, true, true, true, true, true]);
    let mut forces: Vec<Value> = (0..5).map(|_| json!([0, 0, 0])).collect();
    forces[4] = json!([-1000, 0, 0]);
    let v = json!({"op":"frame_buckling",
        "nodesMm":(0..=4).map(|i| json!([i as f64 * 250., 0, 0])).collect::<Vec<_>>(),
        "members": members,
        "restrained": restrained,
        "reference": {"forcesN": forces,
            "momentsNmm": (0..5).map(|_| json!([0, 0, 0])).collect::<Vec<_>>(),
            "loads": []},
        "modes": 2});
    let r = crate::dispatch(v.clone()).unwrap();
    // P_cr = π²EI/(4L²) with L = 1000; degenerate pair (Iyy = Izz).
    let p_cr = std::f64::consts::PI.powi(2) * 2e5 * 1e6 / (4. * 1000. * 1000.);
    let lambda = p_cr / 1000.;
    for i in 0..2 {
        let factor = r["modes"][i]["loadFactor"].as_f64().unwrap();
        assert!(
            (factor - lambda).abs() < 0.005 * lambda,
            "{factor} vs {lambda}"
        );
        assert!(r["modes"][i]["relativeResidual"].as_f64().unwrap() < 1e-8);
    }
    assert_eq!(r["axialForcesN"].as_array().unwrap().len(), 4);
    assert_eq!(r["freeDofs"], json!(24));
    let mut bad = v.clone();
    bad["supports"] = json!([{"type":"lowerBound","node":4,"dof":1}]);
    assert_eq!(
        crate::dispatch(bad).unwrap_err().code,
        "FRAME_INVALID_INPUT"
    );
    let mut bad = v.clone();
    bad["modes"] = json!(0);
    assert_eq!(
        crate::dispatch(bad).unwrap_err().code,
        "FRAME_INVALID_INPUT"
    );
    let mut bad = v;
    bad["reference"]["selfWeight"] = json!(true);
    assert_eq!(
        crate::dispatch(bad).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
}

#[test]
fn modal_cantilever_matches_beam_theory_through_dispatch() {
    let members: Vec<Value> = (0..4)
        .map(|i| {
            json!({"nodes":[i, i+1],"youngMpa":200000,"poisson":0.3,
            "areaMm2":100,"iyyMm4":1e6,"izzMm4":1e6,"jMm4":2e6})
        })
        .collect();
    let mut restrained: Vec<Value> = (0..5)
        .map(|_| json!([false, false, false, false, false, false]))
        .collect();
    restrained[0] = json!([true, true, true, true, true, true]);
    let v = json!({"op":"frame_modal",
        "nodesMm":(0..=4).map(|i| json!([i as f64 * 250., 0, 0])).collect::<Vec<_>>(),
        "members": members,
        "restrained": restrained,
        "densitiesTMm3":[8e-9,8e-9,8e-9,8e-9],
        "massModel":"consistent",
        "modes":2});
    let r = crate::dispatch(v.clone()).unwrap();
    // f1 = β₁²/(2π)·√(EI/(ρA))/L² ≈ 279.77 Hz; degenerate lateral pair.
    let f1 = 1.8751f64.powi(2) / (2. * std::f64::consts::PI)
        * (2e5 * 1e6 / (8e-9 * 100.) as f64).sqrt()
        / (1000. * 1000.);
    for i in 0..2 {
        let f = r["modes"][i]["frequencyHz"].as_f64().unwrap();
        assert!((f - f1).abs() < 0.005 * f1, "{f} vs {f1}");
    }
    // Massless members leave no finite-frequency modes.
    let mut idle = v;
    idle["densitiesTMm3"] = json!([0, 0, 0, 0]);
    assert_eq!(crate::dispatch(idle).unwrap()["modes"], json!([]));
}

#[test]
fn collapse_propped_cantilever_through_dispatch() {
    let v = json!({"op":"frame_collapse",
        "nodesMm":[[0, 0, 0], [500, 0, 0], [1000, 0, 0]],
        "members":[
            {"nodes":[0, 1],"youngMpa":200000,"poisson":0.3,
             "areaMm2":100,"iyyMm4":1e6,"izzMm4":1e6,"jMm4":2e6},
            {"nodes":[1, 2],"youngMpa":200000,"poisson":0.3,
             "areaMm2":100,"iyyMm4":1e6,"izzMm4":1e6,"jMm4":2e6}],
        "restrained":[[true, true, true, true, true, true],
            [false, false, true, true, true, false],
            [false, true, true, true, true, false]],
        "reference": {"forcesN": [[0, 0, 0], [0, -1, 0], [0, 0, 0]],
            "momentsNmm": [[0, 0, 0], [0, 0, 0], [0, 0, 0]],
            "loads": []},
        "plasticMomentsNMm": [1e6, 1e6],
        "maxHinges": 16});
    let r = crate::dispatch(v.clone()).unwrap();
    // Classical: first hinge at the fixed end at 16Mp/(3L), collapse at
    // 6Mp/L once both mid-span ends yield.
    assert_eq!(r["status"], json!("mechanism"));
    assert_eq!(r["hinges"].as_array().unwrap().len(), 3);
    assert_eq!(r["hinges"][0]["member"], json!(0));
    assert_eq!(r["hinges"][0]["atNodeA"], json!(true));
    close(
        r["hinges"][0]["loadFactor"].as_f64().unwrap(),
        16. * 1e6 / (3. * 1000.),
    );
    close(r["collapseLoadFactor"].as_f64().unwrap(), 6. * 1e6 / 1000.);
    // Elastic member exempt from yielding: fixed-fixed variant goes
    // elastic-unlimited after member 0's two ends hinge.
    let mut elastic = v.clone();
    elastic["restrained"][2] = json!([true, true, true, true, true, true]);
    elastic["plasticMomentsNMm"] = json!([1e6, Value::Null]);
    let r = crate::dispatch(elastic).unwrap();
    assert_eq!(r["status"], json!("elasticUnlimited"));
    assert_eq!(r["collapseLoadFactor"], json!(null));
    assert_eq!(r["hinges"].as_array().unwrap().len(), 2);
    // Validation: bad plastic moment vector, bad budget, extra field.
    let mut bad = v.clone();
    bad["plasticMomentsNMm"] = json!([1e6]);
    assert_eq!(
        crate::dispatch(bad).unwrap_err().code,
        "FRAME_INVALID_INPUT"
    );
    let mut bad = v.clone();
    bad["plasticMomentsNMm"] = json!(["a lot", Value::Null]);
    assert_eq!(
        crate::dispatch(bad).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
    let mut bad = v.clone();
    bad["maxHinges"] = json!(0);
    assert_eq!(
        crate::dispatch(bad).unwrap_err().code,
        "FRAME_INVALID_INPUT"
    );
    let mut bad = v;
    bad["safetyFactor"] = json!(2);
    assert_eq!(
        crate::dispatch(bad).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
}

#[test]
fn influence_lines_through_dispatch() {
    // Simply supported beam, L = 1000, two elements, planar restraints.
    let v = json!({"op":"frame_influence",
        "nodesMm":[[0, 0, 0], [500, 0, 0], [1000, 0, 0]],
        "members":[
            {"nodes":[0, 1],"youngMpa":200000,"poisson":0.3,
             "areaMm2":100,"iyyMm4":1e6,"izzMm4":1e6,"jMm4":2e6},
            {"nodes":[1, 2],"youngMpa":200000,"poisson":0.3,
             "areaMm2":100,"iyyMm4":1e6,"izzMm4":1e6,"jMm4":2e6}],
        "restrained":[[true, true, true, true, true, false],
            [false, false, true, true, true, false],
            [false, true, true, true, true, false]],
        "forceN": [0, -1, 0],
        "positions": [
            {"member":0,"atMm":0}, {"member":0,"atMm":250},
            {"member":0,"atMm":500}, {"member":1,"atMm":250},
            {"member":1,"atMm":500}],
        "target": {"type":"reaction","node":0,"dof":1}});
    let r = crate::dispatch(v.clone()).unwrap();
    // Reaction at A: 1 − x/L.
    for (value, x) in r["values"]
        .as_array()
        .unwrap()
        .iter()
        .zip([0., 250., 500., 750., 1000.])
    {
        close(value.as_f64().unwrap(), 1. - x / 1000.);
    }
    // Mid-span moment line: triangle peaking at a·b/L = 250.
    let mut moment = v.clone();
    moment["target"] =
        json!({"type":"memberResultant","member":0,"atMm":500,"resultant":"momentZ"});
    let r = crate::dispatch(moment).unwrap();
    for (value, expected) in r["values"]
        .as_array()
        .unwrap()
        .iter()
        .zip([0., 125., 250., 125., 0.])
    {
        close(value.as_f64().unwrap().abs(), expected);
    }
    // Validation: position off the member, bad target type, extra field.
    let mut bad = v.clone();
    bad["positions"] = json!([{"member":0,"atMm":9000}]);
    assert_eq!(
        crate::dispatch(bad).unwrap_err().code,
        "FRAME_INVALID_INPUT"
    );
    let mut bad = v.clone();
    bad["target"] = json!({"type":"stress","node":0});
    assert_eq!(
        crate::dispatch(bad).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
    let mut bad = v;
    bad["vehicle"] = json!("truck");
    assert_eq!(
        crate::dispatch(bad).unwrap_err().code,
        "GEOMETRY_INVALID_INPUT"
    );
}
