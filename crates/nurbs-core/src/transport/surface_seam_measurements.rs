use super::*;
use crate::{surface_g1,surface_g2,surface_join};
use surface_join::Boundary;

fn boundary(v:&Value, key:&str)->Result<Boundary>{
    match field::<String>(v,key)?.as_str() {
        "u_min"=>Ok(Boundary::UMin),"u_max"=>Ok(Boundary::UMax),
        "v_min"=>Ok(Boundary::VMin),"v_max"=>Ok(Boundary::VMax),
        _=>Err(input("Unknown surface boundary")),
    }
}

pub(super) fn inspect(v:&Value,op:&str)->Result<Value>{
    let a=field(v,"surface")?;
    let b=field(v,"target")?;
    let ea=boundary(v,"source_edge")?;
    let eb=boundary(v,"target_edge")?;
    let reverse=field(v,"reverse")?;
    let position=field(v,"tolerance_mm")?;
    let budget=field(v,"max_cells")?;
    if op=="surface_seam_g0" {
        let r=surface_join::inspect_g0(&a,ea,&b,eb,reverse,position,budget)?;
        return Ok(json!({"decision":format!("{:?}",r.decision),"positionBoundsMm":r.bounds,"cells":r.cells}));
    }
    let seam=surface_g2::SurfaceSeam {a:&a,ea,b:&b,eb,reverse};
    let normal=field(v,"normal_tolerance")?;
    if op=="surface_seam_g1" {
        let r=surface_g1::inspect(seam,position,normal,budget)?;
        return Ok(json!({"decision":format!("{:?}",r.decision),"positionBoundsMm":r.position_bounds,
            "normalChordLower":r.normal_chord_lower,"normalChordUpper":r.normal_chord_upper,
            "regularityProven":r.regularity_proven,"cells":r.cells}));
    }
    let tolerance=surface_g2::G2Tolerance {position,normal,tensor:field(v,"curvature_tolerance_per_mm")?};
    let r=surface_g2::inspect_inferred(seam,field(v,"segments")?,tolerance,budget)?;
    let verification=r.verification.map(|v|json!({"decision":format!("{:?}",v.decision),
        "positionBoundsMm":v.position_bounds,"orientation":v.orientation.map(|o|format!("{o:?}")),
        "branches":v.branches.iter().map(|b|json!({"normalLower":b.normal_lower,"normalUpper":b.normal_upper,
            "tensorLowerPerMm":b.tensor_lower,"tensorUpperPerMm":b.tensor_upper})).collect::<Vec<_>>(),
        "regularityProven":v.regularity_proven,"cells":v.cells}));
    Ok(json!({"nodes":r.nodes,"candidateMonotone":r.candidate_monotone,"verification":verification}))
}
