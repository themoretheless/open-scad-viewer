//! Strict transport adapter; structural validation belongs to mechanics-core.
use crate::{Result, Value, field, input, json, require_exact_fields};
use mechanics_core::truss::{MAX_MEMBERS, MAX_NODES, Member, Model};
use mechanics_core::truss_loads::{Wrench, distribute};
use std::collections::BTreeSet;

const MAX_WRENCHES: usize = 32;

fn assemble_loads(v: &Value, nodes: &[[f64; 3]]) -> Result<Vec<[f64; 3]>> {
    let loads = v["loads"]
        .as_array()
        .ok_or_else(|| input("loads must be an array"))?;
    if loads.is_empty() || loads.len() > MAX_WRENCHES {
        return Err(input("Provide between 1 and 32 explicit truss loads"));
    }
    let mut forces = vec![[0.; 3]; nodes.len()];
    for load in loads {
        require_exact_fields(
            load,
            &["nodes", "originMm", "forceN", "momentNmm"],
            "truss load",
        )?;
        let selected = load["nodes"]
            .as_array()
            .ok_or_else(|| input("load nodes must be an array"))?;
        if selected.is_empty() || selected.len() > MAX_NODES {
            return Err(input("Select between 1 and 125 load nodes"));
        }
        let indices: Vec<usize> = field(load, "nodes")?;
        let mut seen = BTreeSet::new();
        if indices.iter().any(|&i| i >= nodes.len() || !seen.insert(i)) {
            return Err(input("Load node indices must be valid and unique"));
        }
        let points: Vec<_> = indices.iter().map(|&i| nodes[i]).collect();
        let distributed = distribute(
            &points,
            &Wrench {
                origin_mm: field(load, "originMm")?,
                force_n: field(load, "forceN")?,
                moment_n_mm: field(load, "momentNmm")?,
            },
        )?;
        for (index, force) in indices.into_iter().zip(distributed) {
            for (sum, value) in forces[index].iter_mut().zip(force) {
                *sum += value;
                if !sum.is_finite() {
                    return Err(crate::Error::new(
                        "TRUSS_LOAD_NUMERIC_RANGE",
                        "Combined loads exceed finite numeric range",
                    ));
                }
            }
        }
    }
    Ok(forces)
}

/// Parse and field-check the `members` array (budget checked by the caller).
fn members(v: &Value) -> Result<Vec<Member>> {
    let mut members = Vec::new();
    for member in v["members"].as_array().unwrap() {
        require_exact_fields(member, &["nodes", "youngMpa", "areaMm2"], "truss member")?;
        members.push(Member {
            nodes: field(member, "nodes")?,
            young_mpa: field(member, "youngMpa")?,
            area_mm2: field(member, "areaMm2")?,
        });
    }
    Ok(members)
}

pub(crate) fn solve(v: Value) -> Result<Value> {
    let wrenches = v["op"].as_str() == Some("truss_solve_wrenches");
    let load_field = if wrenches { "loads" } else { "forcesN" };
    require_exact_fields(
        &v,
        &["op", "nodesMm", "members", "restrained", load_field],
        "truss request",
    )?;
    // Check collection budgets before cloning/deserializing numerical arrays.
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
        (load_field, if wrenches { MAX_WRENCHES } else { MAX_NODES }),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds truss limit {max}")));
        }
    }
    let members = members(&v)?;
    let nodes_mm = field::<Vec<[f64; 3]>>(&v, "nodesMm")?;
    let forces_n = if wrenches {
        assemble_loads(&v, &nodes_mm)?
    } else {
        field(&v, "forcesN")?
    };
    let result = mechanics_core::truss::solve(&Model {
        nodes_mm,
        members,
        restrained: field(&v, "restrained")?,
        forces_n,
    })?;
    Ok(json!({
        "displacementsMm": result.displacements_mm,
        "reactionsN": result.reactions_n,
        "axialForcesN": result.axial_forces_n,
        "axialStressesMpa": result.axial_stresses_mpa,
        "maxDeflectionMm": result.max_deflection_mm,
        "maxRelativeResidual": result.max_relative_residual,
        "freeDofs": result.free_dofs
    }))
}

pub(crate) fn diagnose(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &["op", "nodesMm", "members", "restrained"],
        "truss diagnose request",
    )?;
    // Check collection budgets before cloning/deserializing numerical arrays.
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds truss limit {max}")));
        }
    }
    let members = members(&v)?;
    let nodes_mm = field::<Vec<[f64; 3]>>(&v, "nodesMm")?;
    let diagnosis = mechanics_core::truss::diagnose(&Model {
        forces_n: vec![[0.; 3]; nodes_mm.len()],
        nodes_mm,
        members,
        restrained: field(&v, "restrained")?,
    })?;
    Ok(crate::diagnosis_json(&diagnosis))
}

pub(crate) fn buckling(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &["op", "nodesMm", "members", "restrained", "forcesN", "modes"],
        "truss buckling request",
    )?;
    // Check collection budgets before cloning/deserializing numerical arrays.
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
        ("forcesN", MAX_NODES),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds truss limit {max}")));
        }
    }
    let members = members(&v)?;
    let modes: usize = field(&v, "modes")?;
    let result = mechanics_core::truss::buckling(
        &Model {
            nodes_mm: field(&v, "nodesMm")?,
            members,
            restrained: field(&v, "restrained")?,
            forces_n: field(&v, "forcesN")?,
        },
        modes,
    )?;
    Ok(json!({
        "modes": result.modes.iter().map(|m| json!({
            "loadFactor": m.load_factor,
            "displacementsMm": m.displacements,
            "relativeResidual": m.relative_residual,
        })).collect::<Vec<_>>(),
        "axialForcesN": result.axial_forces_n,
        "freeDofs": result.free_dofs,
    }))
}

pub(crate) fn modal(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &["op", "nodesMm", "members", "restrained", "densitiesTMm3", "massModel", "modes"],
        "truss modal request",
    )?;
    // Check collection budgets before cloning/deserializing numerical arrays.
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
        ("densitiesTMm3", MAX_MEMBERS),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds truss limit {max}")));
        }
    }
    let members = members(&v)?;
    let result = mechanics_core::truss::modal(
        &Model {
            nodes_mm: field(&v, "nodesMm")?,
            members,
            restrained: field(&v, "restrained")?,
            forces_n: vec![[0.; 3]; v["nodesMm"].as_array().unwrap().len()],
        },
        &field::<Vec<f64>>(&v, "densitiesTMm3")?,
        crate::mass_model(&v["massModel"])?,
        field(&v, "modes")?,
    )?;
    Ok(json!({
        "modes": result.modes.iter().map(|m| json!({
            "frequencyHz": m.frequency_hz,
            "omegaRadS": m.omega_rad_s,
            "displacementsMm": m.displacements,
            "relativeResidual": m.relative_residual,
        })).collect::<Vec<_>>(),
        "totalMassT": result.total_mass_t,
        "freeDofs": result.free_dofs,
    }))
}

pub(crate) fn nonlinear(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &[
            "op",
            "nodesMm",
            "members",
            "restrained",
            "forcesN",
            "steps",
            "tolerance",
            "maxIterations",
        ],
        "truss nonlinear request",
    )?;
    // Check collection budgets before cloning/deserializing numerical arrays.
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
        ("forcesN", MAX_NODES),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds truss limit {max}")));
        }
    }
    let members = members(&v)?;
    let result = mechanics_core::truss::solve_nonlinear(
        &Model {
            nodes_mm: field(&v, "nodesMm")?,
            members,
            restrained: field(&v, "restrained")?,
            forces_n: field(&v, "forcesN")?,
        },
        &mechanics_core::truss::NonlinearOptions {
            steps: field(&v, "steps")?,
            tolerance: field(&v, "tolerance")?,
            max_iterations: field(&v, "maxIterations")?,
        },
    )?;
    Ok(json!({
        "displacementsMm": result.displacements_mm,
        "reactionsN": result.reactions_n,
        "axialForcesN": result.axial_forces_n,
        "axialStressesMpa": result.axial_stresses_mpa,
        "steps": result.steps.iter().map(|s| json!({
            "loadFactor": s.load_factor,
            "iterations": s.iterations,
            "relativeResidual": s.relative_residual,
        })).collect::<Vec<_>>(),
        "loadFactor": result.load_factor,
        "converged": result.converged,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bar() -> Value {
        json!({"op":"truss_solve", "nodesMm":[[0,0,0],[10,0,0]],
            "members":[{"nodes":[0,1],"youngMpa":2000,"areaMm2":2}],
            "restrained":[[true,true,true],[false,true,true]],
            "forcesN":[[0,0,0],[100,0,0]]})
    }
    fn loaded_bar() -> Value {
        let mut v = bar();
        v.as_object_mut().unwrap().remove("forcesN");
        v["op"] = json!("truss_solve_wrenches");
        v["loads"] =
            json!([{"nodes":[1],"originMm":[10,0,0],"forceN":[100,0,0],"momentNmm":[0,0,0]}]);
        v
    }
    #[test]
    fn explicit_wrench_matches_the_nodal_bar_and_superposes_signed_loads() {
        assert_eq!(
            crate::handle(loaded_bar()).unwrap(),
            crate::handle(bar()).unwrap()
        );
        let mut v = loaded_bar();
        let mut subtract = v["loads"][0].clone();
        subtract["forceN"] = json!([-25, 0, 0]);
        v["loads"].as_array_mut().unwrap().push(subtract);
        let result = crate::handle(v).unwrap();
        assert_eq!(result["axialForcesN"][0].as_f64(), Some(75.));
    }
    #[test]
    fn refuses_unrealizable_wrenches_and_ambiguous_load_schemas() {
        let mut v = loaded_bar();
        v["loads"][0]["momentNmm"] = json!([0, 0, 100]);
        assert_eq!(
            crate::handle(v).unwrap_err().code,
            "TRUSS_LOAD_UNREALIZABLE"
        );
        let mut v = loaded_bar();
        v["forcesN"] = json!([[0, 0, 0], [100, 0, 0]]);
        assert_eq!(
            crate::handle(v).unwrap_err().code,
            "GEOMETRY_INVALID_INPUT"
        );
        let mut v = loaded_bar();
        v["loads"][0]["momentNm"] = json!([0, 0, 0]);
        assert_eq!(
            crate::handle(v).unwrap_err().code,
            "GEOMETRY_INVALID_INPUT"
        );
    }
    #[test]
    fn bounds_load_count_and_requires_unique_explicit_selected_nodes() {
        for indices in [
            json!([]),
            json!([1, 1]),
            json!([2]),
            json!([-1]),
            json!([0.5]),
            json!(vec![0; 126]),
        ] {
            let mut v = loaded_bar();
            v["loads"][0]["nodes"] = indices;
            assert_eq!(
                crate::handle(v).unwrap_err().code,
                "GEOMETRY_INVALID_INPUT"
            );
        }
        for count in [0, 33] {
            let mut v = loaded_bar();
            let load = v["loads"][0].clone();
            v["loads"] = json!(vec![load; count]);
            assert_eq!(
                crate::handle(v).unwrap_err().code,
                "GEOMETRY_INVALID_INPUT"
            );
        }
        let mut v = loaded_bar();
        let load = v["loads"][0].clone();
        v["loads"] = json!(vec![load; 32]);
        let result = crate::handle(v).unwrap();
        assert!((result["axialForcesN"][0].as_f64().unwrap() - 3200.).abs() < 1e-9);
        let mut v = loaded_bar();
        let mut load = v["loads"][0].clone();
        load["forceN"] = json!([f64::MAX / 4., 0., 0.]);
        v["loads"] = json!(vec![load; 5]);
        assert_eq!(
            crate::handle(v).unwrap_err().code,
            "TRUSS_LOAD_NUMERIC_RANGE"
        );
    }
    #[test]
    fn dispatches_signed_response_and_rejects_unknown_fields() {
        let response = crate::handle(bar()).unwrap();
        assert_eq!(response["displacementsMm"][1][0].as_f64(), Some(0.25));
        assert_eq!(response["reactionsN"][0][0].as_f64(), Some(-100.));
        let mut v = bar();
        v["momentsNm"] = json!([[0, 0, 0], [0, 0, 1]]);
        assert_eq!(
            crate::handle(v).unwrap_err().code,
            "GEOMETRY_INVALID_INPUT"
        );
        let mut v = bar();
        v["members"][0]["spring"] = json!(1);
        assert!(crate::handle(v).is_err());
    }
    #[test]
    fn preserves_singular_error_and_rejects_transport_shape() {
        let mut v = bar();
        v["restrained"][1] = json!([false, false, false]);
        assert_eq!(crate::handle(v).unwrap_err().code, "TRUSS_SINGULAR");
        for invalid in [json!([0, 1]), json!([0, 1, 2, 3]), json!([0, "1", 2])] {
            let mut v = bar();
            v["nodesMm"][0] = invalid;
            assert!(crate::handle(v).is_err());
        }
        let mut v = bar();
        v["nodesMm"] = json!(vec![[0.; 3]; MAX_NODES + 1]);
        assert!(crate::handle(v).is_err());
    }
    #[test]
    fn diagnose_reports_unrestrained_dofs_and_rejects_extra_fields() {
        let mut v = bar();
        v["op"] = json!("truss_diagnose");
        v.as_object_mut().unwrap().remove("forcesN");
        v["restrained"][1] = json!([false, false, false]);
        let report = crate::dispatch(v).unwrap();
        assert_eq!(report["stable"], json!(false));
        assert_eq!(report["minNormalizedPivot"], json!(null));
        assert_eq!(report["issues"].as_array().unwrap().len(), 2);
        assert_eq!(report["issues"][0]["node"], json!(1));
        assert_eq!(report["issues"][0]["dofName"], json!("y"));
        assert_eq!(report["issues"][0]["issue"], json!("unrestrained"));
        assert_eq!(report["issues"][1]["dofName"], json!("z"));
        let mut v = bar();
        v["op"] = json!("truss_diagnose");
        let stable = crate::dispatch(v.clone()).unwrap_err();
        assert_eq!(stable.code, "GEOMETRY_INVALID_INPUT"); // forcesN not allowed here
        v.as_object_mut().unwrap().remove("forcesN");
        let report = crate::dispatch(v).unwrap();
        assert_eq!(report["stable"], json!(true));
        assert!(report["minNormalizedPivot"].as_f64().unwrap() > 1e-12);
        assert_eq!(report["issues"], json!([]));
    }
    #[test]
    fn buckling_toggle_matches_closed_form_and_validates_input() {
        // Von Mises toggle: apex at (10, 5), span 20, compressed by a downward
        // apex load. Closed form from the 2-DOF pencil: with sin²θ = 0.2,
        // cos²θ = 0.8, N = −F·L/(2h) the snap-through factor is
        // λ = (EA/L·sin²θ) / (|N|/L·cos²θ) = 8.9443, the lateral one 143.1.
        let v = json!({"op":"truss_buckling",
            "nodesMm":[[0,0,0],[10,5,0],[20,0,0]],
            "members":[{"nodes":[0,1],"youngMpa":2000,"areaMm2":2},
                       {"nodes":[1,2],"youngMpa":2000,"areaMm2":2}],
            "restrained":[[true,true,true],[false,false,true],[true,true,true]],
            "forcesN":[[0,0,0],[0,-100,0],[0,0,0]],
            "modes":2});
        let r = crate::dispatch(v.clone()).unwrap();
        let k_elastic = 2. * 4000. / 11.180_339_887_498_949 * 0.2;
        let k_geo = 2. * (100. * 11.180_339_887_498_949 / 10.) / 11.180_339_887_498_949 * 0.8;
        let lambda = k_elastic / k_geo;
        let first = r["modes"][0]["loadFactor"].as_f64().unwrap();
        assert!((first - lambda).abs() < 1e-6 * lambda, "{first} vs {lambda}");
        // The second (lateral) mode costs cos²/sin² = 4× more.
        let second = r["modes"][1]["loadFactor"].as_f64().unwrap();
        assert!((second - 16. * lambda).abs() < 1e-6 * 16. * lambda);
        // Snap-through mode shape: vertical apex motion dominates.
        let apex_y = r["modes"][0]["displacementsMm"][1][1].as_f64().unwrap().abs();
        assert!((apex_y - 1.).abs() < 1e-12, "{apex_y}");
        assert!(r["modes"][0]["relativeResidual"].as_f64().unwrap() < 1e-10);
        assert_eq!(r["freeDofs"], json!(2));
        for bad in [json!(0), json!(9)] {
            let mut v = v.clone();
            v["modes"] = bad;
            assert_eq!(crate::dispatch(v).unwrap_err().code, "TRUSS_INVALID_INPUT");
        }
        let mut v = v.clone();
        v["reference"] = json!({});
        assert_eq!(crate::dispatch(v).unwrap_err().code, "GEOMETRY_INVALID_INPUT");
    }
    #[test]
    fn modal_rod_matches_rod_theory_and_validates_input() {
        // Fixed-free axial rod: f₁ = √(E/ρ)/(4L) = 1250 Hz for these numbers.
        let v = json!({"op":"truss_modal",
            "nodesMm":[[0,0,0],[250,0,0],[500,0,0],[750,0,0],[1000,0,0]],
            "members":[[0,1],[1,2],[2,3],[3,4]].iter().map(|nodes|
                json!({"nodes":nodes,"youngMpa":200000,"areaMm2":100})).collect::<Vec<_>>(),
            "restrained":[[true,true,true],[false,true,true],[false,true,true],
                [false,true,true],[false,true,true]],
            "densitiesTMm3":[8e-9,8e-9,8e-9,8e-9],
            "massModel":"consistent",
            "modes":1});
        let r = crate::dispatch(v.clone()).unwrap();
        let f1 = r["modes"][0]["frequencyHz"].as_f64().unwrap();
        assert!((f1 - 1250.).abs() < 0.02 * 1250., "{f1}");
        let total_mass = r["totalMassT"].as_f64().unwrap();
        assert!((total_mass - 8e-4).abs() < 1e-12, "{total_mass}");
        assert_eq!(r["modes"][0]["omegaRadS"].as_f64().unwrap(),
            2. * std::f64::consts::PI * f1);
        let mut bad = v.clone();
        bad["massModel"] = json!("smeared");
        assert_eq!(crate::dispatch(bad).unwrap_err().code, "GEOMETRY_INVALID_INPUT");
        let mut bad = v.clone();
        bad["densitiesTMm3"] = json!([8e-9]);
        assert_eq!(crate::dispatch(bad).unwrap_err().code, "TRUSS_INVALID_INPUT");
    }

    #[test]
    fn nonlinear_toggle_matches_exact_path_and_detects_snap_through() {
        let toggle = |load: f64| {
            json!({"op":"truss_nonlinear",
                "nodesMm":[[-1000, 0, 0], [1000, 0, 0], [0, 30, 0]],
                "members":[{"nodes":[0, 2],"youngMpa":200000,"areaMm2":100},
                    {"nodes":[1, 2],"youngMpa":200000,"areaMm2":100}],
                "restrained":[[true, true, true], [true, true, true],
                    [false, false, true]],
                "forcesN":[[0, 0, 0], [0, 0, 0], [0, -load, 0]],
                "steps":10,"tolerance":1e-9,"maxIterations":50})
        };
        // Exact path point: P(20) lowers the apex from 30 to 20 (v = 10 mm).
        let l0 = (1000f64.powi(2) + 900.).sqrt();
        let l = (1000f64.powi(2) + 400.).sqrt();
        let p = 2. * 200_000. * 100. * (l0 - l) / l0 * (20. / l);
        let r = crate::dispatch(toggle(p)).unwrap();
        assert_eq!(r["converged"], json!(true));
        assert_eq!(r["loadFactor"], json!(1.));
        let v = r["displacementsMm"][2][1].as_f64().unwrap();
        assert!((v + 10.).abs() < 1e-6, "{v}");
        // Beyond the limit point (≈ 207.7 N) load control stalls.
        let r = crate::dispatch(toggle(400.)).unwrap();
        assert_eq!(r["converged"], json!(false));
        let lambda = r["loadFactor"].as_f64().unwrap();
        assert!(lambda > 0.35 && lambda < 0.65, "{lambda}");
        // Validation.
        let mut bad = toggle(100.);
        bad["steps"] = json!(0);
        assert_eq!(
            crate::dispatch(bad).unwrap_err().code,
            "TRUSS_INVALID_INPUT"
        );
        let mut bad = toggle(100.);
        bad["damping"] = json!(0.1);
        assert_eq!(
            crate::dispatch(bad).unwrap_err().code,
            "GEOMETRY_INVALID_INPUT"
        );
    }
}
