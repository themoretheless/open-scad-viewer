//! Strict transport adapter; structural validation belongs to mechanics-core.
use crate::{Result, Value, field, input, json, require_exact_fields};
use mechanics_core::truss::{MAX_MEMBERS, MAX_NODES, Member, Model};

pub(crate) fn solve(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &["op", "nodesMm", "members", "restrained", "forcesN"],
        "truss request",
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
    let mut members = Vec::new();
    for member in v["members"].as_array().unwrap() {
        require_exact_fields(member, &["nodes", "youngMpa", "areaMm2"], "truss member")?;
        members.push(Member {
            nodes: field(member, "nodes")?,
            young_mpa: field(member, "youngMpa")?,
            area_mm2: field(member, "areaMm2")?,
        });
    }
    let result = mechanics_core::truss::solve(&Model {
        nodes_mm: field(&v, "nodesMm")?,
        members,
        restrained: field(&v, "restrained")?,
        forces_n: field(&v, "forcesN")?,
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
