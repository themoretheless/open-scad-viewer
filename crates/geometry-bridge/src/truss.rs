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
    let mut members = Vec::new();
    for member in v["members"].as_array().unwrap() {
        require_exact_fields(member, &["nodes", "youngMpa", "areaMm2"], "truss member")?;
        members.push(Member {
            nodes: field(member, "nodes")?,
            young_mpa: field(member, "youngMpa")?,
            area_mm2: field(member, "areaMm2")?,
        });
    }
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
            crate::dispatch(loaded_bar()).unwrap(),
            crate::dispatch(bar()).unwrap()
        );
        let mut v = loaded_bar();
        let mut subtract = v["loads"][0].clone();
        subtract["forceN"] = json!([-25, 0, 0]);
        v["loads"].as_array_mut().unwrap().push(subtract);
        let result = crate::dispatch(v).unwrap();
        assert_eq!(result["axialForcesN"][0].as_f64(), Some(75.));
    }
    #[test]
    fn refuses_unrealizable_wrenches_and_ambiguous_load_schemas() {
        let mut v = loaded_bar();
        v["loads"][0]["momentNmm"] = json!([0, 0, 100]);
        assert_eq!(
            crate::dispatch(v).unwrap_err().code,
            "TRUSS_LOAD_UNREALIZABLE"
        );
        let mut v = loaded_bar();
        v["forcesN"] = json!([[0, 0, 0], [100, 0, 0]]);
        assert_eq!(
            crate::dispatch(v).unwrap_err().code,
            "GEOMETRY_INVALID_INPUT"
        );
        let mut v = loaded_bar();
        v["loads"][0]["momentNm"] = json!([0, 0, 0]);
        assert_eq!(
            crate::dispatch(v).unwrap_err().code,
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
                crate::dispatch(v).unwrap_err().code,
                "GEOMETRY_INVALID_INPUT"
            );
        }
        for count in [0, 33] {
            let mut v = loaded_bar();
            let load = v["loads"][0].clone();
            v["loads"] = json!(vec![load; count]);
            assert_eq!(
                crate::dispatch(v).unwrap_err().code,
                "GEOMETRY_INVALID_INPUT"
            );
        }
        let mut v = loaded_bar();
        let load = v["loads"][0].clone();
        v["loads"] = json!(vec![load; 32]);
        let result = crate::dispatch(v).unwrap();
        assert!((result["axialForcesN"][0].as_f64().unwrap() - 3200.).abs() < 1e-9);
        let mut v = loaded_bar();
        let mut load = v["loads"][0].clone();
        load["forceN"] = json!([f64::MAX / 4., 0., 0.]);
        v["loads"] = json!(vec![load; 5]);
        assert_eq!(
            crate::dispatch(v).unwrap_err().code,
            "TRUSS_LOAD_NUMERIC_RANGE"
        );
    }
    #[test]
    fn dispatches_signed_response_and_rejects_unknown_fields() {
        let response = crate::dispatch(bar()).unwrap();
        assert_eq!(response["displacementsMm"][1][0].as_f64(), Some(0.25));
        assert_eq!(response["reactionsN"][0][0].as_f64(), Some(-100.));
        let mut v = bar();
        v["momentsNm"] = json!([[0, 0, 0], [0, 0, 1]]);
        assert_eq!(
            crate::dispatch(v).unwrap_err().code,
            "GEOMETRY_INVALID_INPUT"
        );
        let mut v = bar();
        v["members"][0]["spring"] = json!(1);
        assert!(crate::dispatch(v).is_err());
    }
    #[test]
    fn preserves_singular_error_and_rejects_transport_shape() {
        let mut v = bar();
        v["restrained"][1] = json!([false, false, false]);
        assert_eq!(crate::dispatch(v).unwrap_err().code, "TRUSS_SINGULAR");
        for invalid in [json!([0, 1]), json!([0, 1, 2, 3]), json!([0, "1", 2])] {
            let mut v = bar();
            v["nodesMm"][0] = invalid;
            assert!(crate::dispatch(v).is_err());
        }
        let mut v = bar();
        v["nodesMm"] = json!(vec![[0.; 3]; MAX_NODES + 1]);
        assert!(crate::dispatch(v).is_err());
    }
}
