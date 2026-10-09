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
#[path = "tests/frame.rs"]
mod tests;
