//! Strict transport adapter; structural validation belongs to mechanics-core.
use crate::{Result, Value, field, input, json, optional_field, require_exact_fields};
use mechanics_core::combos::{Combination, MAX_COMBINATIONS, MAX_LOAD_CASES, MinMax};
use mechanics_core::frame::{
    LoadSet, MAX_MEMBER_LOADS, MAX_MEMBERS, MAX_NODES, MAX_SUPPORTS, Member, MemberLoad, Model,
    Support,
};

const MEMBER_REQUIRED: &[&str] = &[
    "nodes", "youngMpa", "poisson", "areaMm2", "iyyMm4", "izzMm4", "jMm4",
];
const MEMBER_OPTIONAL: &[&str] = &[
    "shearAreaYMm2",
    "shearAreaZMm2",
    "localZHint",
    "releaseA",
    "releaseB",
];

fn member(v: &Value) -> Result<Member> {
    let object = v
        .as_object()
        .ok_or_else(|| input("frame member must be an object"))?;
    for key in MEMBER_REQUIRED {
        if !object.contains_key(*key) {
            return Err(input(format!("frame member misses {key}")));
        }
    }
    for key in object.keys() {
        if !MEMBER_REQUIRED.contains(&key.as_str()) && !MEMBER_OPTIONAL.contains(&key.as_str()) {
            return Err(input(format!("frame member has unauthorized field {key}")));
        }
    }
    Ok(Member {
        nodes: field(v, "nodes")?,
        young_mpa: field(v, "youngMpa")?,
        poisson: field(v, "poisson")?,
        area_mm2: field(v, "areaMm2")?,
        iyy_mm4: field(v, "iyyMm4")?,
        izz_mm4: field(v, "izzMm4")?,
        j_mm4: field(v, "jMm4")?,
        shear_area_y_mm2: optional_field(v, "shearAreaYMm2")?,
        shear_area_z_mm2: optional_field(v, "shearAreaZMm2")?,
        local_z_hint: optional_field(v, "localZHint")?,
        release_a: optional_field(v, "releaseA")?.unwrap_or([false; 3]),
        release_b: optional_field(v, "releaseB")?.unwrap_or([false; 3]),
    })
}

fn member_load(v: &Value) -> Result<MemberLoad> {
    let ty = v["type"].as_str().unwrap_or("");
    match ty {
        "pointForce" => {
            require_exact_fields(
                v,
                &["type", "member", "atMm", "forceN", "localAxes"],
                "point force load",
            )?;
            Ok(MemberLoad::PointForce {
                member: field(v, "member")?,
                at_mm: field(v, "atMm")?,
                force_n: field(v, "forceN")?,
                local_axes: field(v, "localAxes")?,
            })
        }
        "pointMoment" => {
            require_exact_fields(
                v,
                &["type", "member", "atMm", "momentNmm", "localAxes"],
                "point moment load",
            )?;
            Ok(MemberLoad::PointMoment {
                member: field(v, "member")?,
                at_mm: field(v, "atMm")?,
                moment_nmm: field(v, "momentNmm")?,
                local_axes: field(v, "localAxes")?,
            })
        }
        "uniform" => {
            require_exact_fields(
                v,
                &["type", "member", "forceNPerMm", "localAxes"],
                "uniform load",
            )?;
            Ok(MemberLoad::Uniform {
                member: field(v, "member")?,
                force_n_per_mm: field(v, "forceNPerMm")?,
                local_axes: field(v, "localAxes")?,
            })
        }
        "trapezoidal" => {
            require_exact_fields(
                v,
                &["type", "member", "fromNPerMm", "toNPerMm", "localAxes"],
                "trapezoidal load",
            )?;
            Ok(MemberLoad::Trapezoidal {
                member: field(v, "member")?,
                from_n_per_mm: field(v, "fromNPerMm")?,
                to_n_per_mm: field(v, "toNPerMm")?,
                local_axes: field(v, "localAxes")?,
            })
        }
        _ => Err(input(
            "frame load type must be pointForce, pointMoment, uniform, or trapezoidal",
        )),
    }
}

fn support(v: &Value) -> Result<Support> {
    let ty = v["type"].as_str().unwrap_or("");
    match ty {
        "spring" | "lowerSpring" | "upperSpring" => {
            require_exact_fields(v, &["type", "node", "dof", "stiffness"], "spring support")?;
            let (node, dof, stiffness) =
                (field(v, "node")?, field(v, "dof")?, field(v, "stiffness")?);
            Ok(match ty {
                "spring" => Support::Spring {
                    node,
                    dof,
                    stiffness,
                },
                "lowerSpring" => Support::LowerSpring {
                    node,
                    dof,
                    stiffness,
                },
                _ => Support::UpperSpring {
                    node,
                    dof,
                    stiffness,
                },
            })
        }
        "lowerBound" | "upperBound" => {
            require_exact_fields(v, &["type", "node", "dof"], "bound support")?;
            let (node, dof) = (field(v, "node")?, field(v, "dof")?);
            Ok(if ty == "lowerBound" {
                Support::LowerBound { node, dof }
            } else {
                Support::UpperBound { node, dof }
            })
        }
        _ => Err(input(
            "support type must be spring, lowerSpring, upperSpring, lowerBound, or upperBound",
        )),
    }
}

/// Detach the optional `supports` array so the strict field check passes.
fn take_supports(v: &mut Value) -> Result<Vec<Support>> {
    let Some(raw) = v
        .as_object_mut()
        .ok_or_else(|| input("frame request must be an object"))?
        .remove("supports")
    else {
        return Ok(Vec::new());
    };
    let array = raw
        .as_array()
        .ok_or_else(|| input("supports must be an array"))?;
    if array.len() > MAX_SUPPORTS {
        return Err(input(format!("supports exceed frame limit {MAX_SUPPORTS}")));
    }
    array.iter().map(support).collect()
}

pub(crate) fn solve(mut v: Value) -> Result<Value> {
    let supports = take_supports(&mut v)?;
    require_exact_fields(
        &v,
        &[
            "op",
            "nodesMm",
            "members",
            "restrained",
            "forcesN",
            "momentsNmm",
            "loads",
        ],
        "frame request",
    )?;
    // Check collection budgets before deserializing numerical arrays.
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
        ("forcesN", MAX_NODES),
        ("momentsNmm", MAX_NODES),
        ("loads", MAX_MEMBER_LOADS),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds frame limit {max}")));
        }
    }
    let members = v["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(member)
        .collect::<Result<Vec<_>>>()?;
    let loads = v["loads"]
        .as_array()
        .unwrap()
        .iter()
        .map(member_load)
        .collect::<Result<Vec<_>>>()?;
    let result = mechanics_core::frame::solve(&Model {
        nodes_mm: field(&v, "nodesMm")?,
        members,
        restrained: field(&v, "restrained")?,
        supports,
        forces_n: field(&v, "forcesN")?,
        moments_nmm: field(&v, "momentsNmm")?,
        loads,
    })?;
    let members: Vec<Value> = result
        .members
        .iter()
        .map(|m| {
            json!({"stations": m.stations.iter().map(|s| json!({
                "xMm": s.x_mm,
                "axialN": s.axial_n,
                "shearYN": s.shear_y_n,
                "shearZN": s.shear_z_n,
                "torsionNmm": s.torsion_nmm,
                "momentYNmm": s.moment_y_nmm,
                "momentZNmm": s.moment_z_nmm
            })).collect::<Vec<_>>()})
        })
        .collect();
    Ok(json!({
        "displacementsMm": result.displacements_mm,
        "rotationsRad": result.rotations_rad,
        "reactionsN": result.reactions_n,
        "reactionMomentsNmm": result.reaction_moments_nmm,
        "members": members,
        "maxDeflectionMm": result.max_deflection_mm,
        "maxRelativeResidual": result.max_relative_residual,
        "freeDofs": result.free_dofs
    }))
}

pub(crate) fn diagnose(mut v: Value) -> Result<Value> {
    let supports = take_supports(&mut v)?;
    require_exact_fields(
        &v,
        &["op", "nodesMm", "members", "restrained"],
        "frame diagnose request",
    )?;
    // Check collection budgets before deserializing numerical arrays.
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds frame limit {max}")));
        }
    }
    let members = v["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(member)
        .collect::<Result<Vec<_>>>()?;
    let diagnosis = mechanics_core::frame::diagnose(
        &field::<Vec<[f64; 3]>>(&v, "nodesMm")?,
        &members,
        &field::<Vec<[bool; 6]>>(&v, "restrained")?,
        &supports,
    )?;
    Ok(crate::diagnosis_json(&diagnosis))
}

pub(crate) fn buckling(mut v: Value) -> Result<Value> {
    let supports = take_supports(&mut v)?;
    require_exact_fields(
        &v,
        &[
            "op",
            "nodesMm",
            "members",
            "restrained",
            "reference",
            "modes",
        ],
        "frame buckling request",
    )?;
    // Check collection budgets before deserializing numerical arrays.
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds frame limit {max}")));
        }
    }
    let members = v["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(member)
        .collect::<Result<Vec<_>>>()?;
    let reference = load_case(&v["reference"])?;
    let modes: usize = field(&v, "modes")?;
    let result = mechanics_core::frame::buckling(
        &field::<Vec<[f64; 3]>>(&v, "nodesMm")?,
        &members,
        &field::<Vec<[bool; 6]>>(&v, "restrained")?,
        &supports,
        &reference,
        modes,
    )?;
    Ok(json!({
        "modes": result.modes.iter().map(|m| json!({
            "loadFactor": m.load_factor,
            "displacementsMm": m.displacements,
            "rotationsRad": m.rotations,
            "relativeResidual": m.relative_residual,
        })).collect::<Vec<_>>(),
        "axialForcesN": result.axial_forces_n,
        "freeDofs": result.free_dofs,
    }))
}

pub(crate) fn modal(mut v: Value) -> Result<Value> {
    let supports = take_supports(&mut v)?;
    require_exact_fields(
        &v,
        &[
            "op",
            "nodesMm",
            "members",
            "restrained",
            "densitiesTMm3",
            "massModel",
            "modes",
        ],
        "frame modal request",
    )?;
    // Check collection budgets before deserializing numerical arrays.
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
            return Err(input(format!("{key} exceeds frame limit {max}")));
        }
    }
    let members = v["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(member)
        .collect::<Result<Vec<_>>>()?;
    let result = mechanics_core::frame::modal(
        &field::<Vec<[f64; 3]>>(&v, "nodesMm")?,
        &members,
        &field::<Vec<[bool; 6]>>(&v, "restrained")?,
        &supports,
        &field::<Vec<f64>>(&v, "densitiesTMm3")?,
        crate::mass_model(&v["massModel"])?,
        field(&v, "modes")?,
    )?;
    Ok(json!({
        "modes": result.modes.iter().map(|m| json!({
            "frequencyHz": m.frequency_hz,
            "omegaRadS": m.omega_rad_s,
            "displacementsMm": m.displacements,
            "rotationsRad": m.rotations,
            "relativeResidual": m.relative_residual,
        })).collect::<Vec<_>>(),
        "totalMassT": result.total_mass_t,
        "freeDofs": result.free_dofs,
    }))
}

pub(crate) fn collapse(mut v: Value) -> Result<Value> {
    let supports = take_supports(&mut v)?;
    require_exact_fields(
        &v,
        &[
            "op",
            "nodesMm",
            "members",
            "restrained",
            "reference",
            "plasticMomentsNMm",
            "maxHinges",
        ],
        "frame collapse request",
    )?;
    // Check collection budgets before deserializing numerical arrays.
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
        ("plasticMomentsNMm", MAX_MEMBERS),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds frame limit {max}")));
        }
    }
    let members = v["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(member)
        .collect::<Result<Vec<_>>>()?;
    let reference = load_case(&v["reference"])?;
    let plastic_moments: Vec<Option<f64>> = v["plasticMomentsNMm"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            if e.is_null() {
                Ok(None)
            } else {
                e.as_f64()
                    .map(Some)
                    .ok_or_else(|| input("plasticMomentsNMm entries must be numbers or null"))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let result = mechanics_core::frame::collapse(
        &field::<Vec<[f64; 3]>>(&v, "nodesMm")?,
        &members,
        &field::<Vec<[bool; 6]>>(&v, "restrained")?,
        &supports,
        &reference,
        &plastic_moments,
        field(&v, "maxHinges")?,
    )?;
    let status = match result.status {
        mechanics_core::frame::CollapseStatus::Mechanism => "mechanism",
        mechanics_core::frame::CollapseStatus::HingeLimit => "hingeLimit",
        mechanics_core::frame::CollapseStatus::ElasticUnlimited => "elasticUnlimited",
    };
    Ok(json!({
        "hinges": result.hinges.iter().map(|h| json!({
            "member": h.member,
            "atNodeA": h.at_node_a,
            "loadFactor": h.load_factor,
        })).collect::<Vec<_>>(),
        "status": status,
        "collapseLoadFactor": result.collapse_load_factor,
    }))
}

pub(crate) fn influence(mut v: Value) -> Result<Value> {
    let supports = take_supports(&mut v)?;
    require_exact_fields(
        &v,
        &[
            "op",
            "nodesMm",
            "members",
            "restrained",
            "forceN",
            "positions",
            "target",
        ],
        "frame influence request",
    )?;
    // Check collection budgets before deserializing numerical arrays.
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
        ("positions", mechanics_core::frame::MAX_INFLUENCE_POSITIONS),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds frame limit {max}")));
        }
    }
    let members = v["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(member)
        .collect::<Result<Vec<_>>>()?;
    let positions: Vec<(usize, f64)> = v["positions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            require_exact_fields(p, &["member", "atMm"], "influence position")?;
            Ok((field(p, "member")?, field(p, "atMm")?))
        })
        .collect::<Result<Vec<_>>>()?;
    let target = influence_target(&v["target"])?;
    let result = mechanics_core::frame::influence(
        &field::<Vec<[f64; 3]>>(&v, "nodesMm")?,
        &members,
        &field::<Vec<[bool; 6]>>(&v, "restrained")?,
        &supports,
        &positions,
        field(&v, "forceN")?,
        &target,
    )?;
    Ok(json!({"values": result.values}))
}

fn influence_target(v: &Value) -> Result<mechanics_core::frame::InfluenceTarget> {
    use mechanics_core::frame::{InfluenceTarget, Resultant};
    let ty = v["type"].as_str().unwrap_or("");
    match ty {
        "displacement" => {
            require_exact_fields(v, &["type", "node", "dof"], "influence target")?;
            Ok(InfluenceTarget::Displacement {
                node: field(v, "node")?,
                dof: field(v, "dof")?,
            })
        }
        "reaction" => {
            require_exact_fields(v, &["type", "node", "dof"], "influence target")?;
            Ok(InfluenceTarget::Reaction {
                node: field(v, "node")?,
                dof: field(v, "dof")?,
            })
        }
        "memberResultant" => {
            require_exact_fields(
                v,
                &["type", "member", "atMm", "resultant"],
                "influence target",
            )?;
            let resultant = match v["resultant"].as_str().unwrap_or("") {
                "axial" => Resultant::Axial,
                "shearY" => Resultant::ShearY,
                "shearZ" => Resultant::ShearZ,
                "torsion" => Resultant::Torsion,
                "momentY" => Resultant::MomentY,
                "momentZ" => Resultant::MomentZ,
                _ => {
                    return Err(input(
                        "resultant must be axial, shearY, shearZ, torsion, momentY, or momentZ",
                    ));
                }
            };
            Ok(InfluenceTarget::MemberResultant {
                member: field(v, "member")?,
                at_mm: field(v, "atMm")?,
                resultant,
            })
        }
        _ => Err(input(
            "influence target type must be displacement, reaction, or memberResultant",
        )),
    }
}

fn load_case(v: &Value) -> Result<LoadSet> {
    require_exact_fields(v, &["forcesN", "momentsNmm", "loads"], "frame load case")?;
    let loads = v["loads"]
        .as_array()
        .ok_or_else(|| input("load case loads must be an array"))?;
    if loads.len() > MAX_MEMBER_LOADS {
        return Err(input(format!(
            "load case loads exceed frame limit {MAX_MEMBER_LOADS}"
        )));
    }
    for key in ["forcesN", "momentsNmm"] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > MAX_NODES {
            return Err(input(format!("{key} exceeds frame limit {MAX_NODES}")));
        }
    }
    Ok(LoadSet {
        forces_n: field(v, "forcesN")?,
        moments_nmm: field(v, "momentsNmm")?,
        loads: loads.iter().map(member_load).collect::<Result<Vec<_>>>()?,
    })
}

fn combination(v: &Value) -> Result<Combination> {
    require_exact_fields(v, &["name", "factors"], "load combination")?;
    if !v["name"].is_string() {
        return Err(input("combination name must be a string"));
    }
    Ok(Combination {
        name: field(v, "name")?,
        factors: field(v, "factors")?,
    })
}

fn minmax(m: &MinMax) -> Value {
    json!({
        "min": m.min,
        "max": m.max,
        "minCombination": m.min_combination,
        "maxCombination": m.max_combination
    })
}

fn minmax3(v: &[MinMax; 3]) -> Value {
    json!(v.iter().map(minmax).collect::<Vec<_>>())
}

pub(crate) fn envelope(mut v: Value) -> Result<Value> {
    let supports = take_supports(&mut v)?;
    require_exact_fields(
        &v,
        &[
            "op",
            "nodesMm",
            "members",
            "restrained",
            "cases",
            "combinations",
        ],
        "frame envelope request",
    )?;
    for (key, max) in [
        ("nodesMm", MAX_NODES),
        ("members", MAX_MEMBERS),
        ("restrained", MAX_NODES),
        ("cases", MAX_LOAD_CASES),
        ("combinations", MAX_COMBINATIONS),
    ] {
        let array = v[key]
            .as_array()
            .ok_or_else(|| input(format!("{key} must be an array")))?;
        if array.len() > max {
            return Err(input(format!("{key} exceeds frame limit {max}")));
        }
    }
    let members: Vec<Member> = v["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(member)
        .collect::<Result<Vec<_>>>()?;
    let cases: Vec<LoadSet> = v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(load_case)
        .collect::<Result<Vec<_>>>()?;
    let combinations: Vec<Combination> = v["combinations"]
        .as_array()
        .unwrap()
        .iter()
        .map(combination)
        .collect::<Result<Vec<_>>>()?;
    let result = mechanics_core::frame::solve_envelopes(
        &field::<Vec<[f64; 3]>>(&v, "nodesMm")?,
        &members,
        &field::<Vec<[bool; 6]>>(&v, "restrained")?,
        &supports,
        &cases,
        &combinations,
    )?;
    let members: Vec<Value> = result
        .members
        .iter()
        .map(|m| {
            json!({"stations": m.iter().map(|s| json!({
                "xMm": s.x_mm,
                "axialN": minmax(&s.axial_n),
                "shearYN": minmax(&s.shear_y_n),
                "shearZN": minmax(&s.shear_z_n),
                "torsionNmm": minmax(&s.torsion_nmm),
                "momentYNmm": minmax(&s.moment_y_nmm),
                "momentZNmm": minmax(&s.moment_z_nmm)
            })).collect::<Vec<_>>()})
        })
        .collect();
    Ok(json!({
        "loadCases": result.load_cases,
        "combinations": result.combinations,
        "displacementsMm": result.displacements_mm.iter().map(minmax3).collect::<Vec<_>>(),
        "rotationsRad": result.rotations_rad.iter().map(minmax3).collect::<Vec<_>>(),
        "reactionsN": result.reactions_n.iter().map(minmax3).collect::<Vec<_>>(),
        "reactionMomentsNmm": result.reaction_moments_nmm.iter().map(minmax3).collect::<Vec<_>>(),
        "members": members,
        "maxDeflectionMm": minmax(&result.max_deflection_mm),
        "maxRelativeResidual": result.max_relative_residual,
        "freeDofs": result.free_dofs
    }))
}

#[cfg(test)]
mod tests {
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
}
