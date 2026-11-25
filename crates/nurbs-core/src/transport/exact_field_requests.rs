use super::*;
use crate::editing::exact_derivative;

pub(super) fn curve_field(v: &Value, op: &str) -> Result<Value> {
    let source = field(v, "source")?;
    let budget = field(v,"max_work")?;
    let r = match op {
        "curve_derivative_field" => exact_derivative::curve_order(&source,field(v,"order")?,budget)?,
        "curve_speed_squared_field" => exact_derivative::squared_speed(&source,budget)?,
        "curve_binormal_field" => exact_derivative::binormal(&source,budget)?,
        "curve_curvature_squared_field" => exact_derivative::squared_curvature(&source,budget)?,
        _ => return Err(input("Unknown exact curve field"))
    };
    let pieces=r.pieces.ok_or_else(||input(format!("Exact field construction refused: {:?}",r.refusal)))?;
    let index:usize=field(v,"piece")?;
    let curve=pieces.get(index).ok_or_else(||input("Unknown exact field span"))?;
    let [a,b]=curve.domain();
    let parameter:f64=field(v,"parameter")?;
    check((a..=b).contains(&parameter),"Evaluation parameter is outside the selected field span")?;
    Ok(json!({"curve":curve,"spanCount":pieces.len(),"domain":[a,b],"parameter":parameter,
        "value":curve.evaluate(parameter)?.point,"workUsed":r.work_used}))
}
