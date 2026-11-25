use super::*;
use crate::{curve::Curve, trim_domain, trim_simplicity};

pub(super) fn inspect(v: &Value, op: &str) -> Result<Value> {
    let loops: Vec<Vec<Curve>> = field(v, "loops")?;
    if op == "uv_trim_location" {
        let domain = trim_domain::TrimDomain::new(&loops, field(v, "tolerance_uv")?)?;
        let report = domain.classify_point([field(v, "u")?, field(v, "v")?], field(v, "max_cells")?)?;
        return Ok(json!({"location":format!("{:?}",report.location),"reason":format!("{:?}",report.reason),
            "winding":report.winding,"cells":report.cells,"uncertain":report.uncertain.map(|(contour,curve,range)|json!({"contour":contour,"curve":curve,"parameter":range}))}));
    }
    let [contour] = loops.as_slice() else { return Err(input("Expected exactly one UV contour")); };
    if op == "uv_loop_closure" {
        return Ok(json!({"exactJoins":trim_domain::exact_loop_joins(contour)?}));
    }
    let r = trim_simplicity::inspect(contour, field(v,"tolerance_uv")?, field(v,"max_pairs")?, field(v,"max_cells")?)?;
    Ok(json!({"outcome":format!("{:?}",r.outcome()),"provenSimple":r.proven_simple,"exactJoins":r.exact_joins,
        "injective":r.injective,"testedPairs":r.pairs.len(),"totalPairs":r.total_pairs,"cells":r.cells}))
}
