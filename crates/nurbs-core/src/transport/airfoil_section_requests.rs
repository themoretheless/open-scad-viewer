use super::*;
use crate::construct::curves::airfoil;

pub(super) fn build(v:&Value,op:&str)->Result<Value>{
    let chord=field(v,"chord_mm")?;
    if op=="naca_five_digit_profile" {
        let code:u32=field(v,"profile_code")?;
        let curve=airfoil::naca5_airfoil(code,chord,field(v,"closed")?,field(v,"samples")?,field(v,"degree")?,field(v,"control_count")?)?;
        return Ok(json!({"curve":curve,"profileCode":code,"chordMm":chord,"fitted":true}));
    }
    let profile=airfoil::CstProfile{n1:field(v,"n1")?,n2:field(v,"n2")?,upper:field(v,"upper_coefficients")?,lower:field(v,"lower_coefficients")?};
    if op=="cst_spline_profile" {
        let curve=airfoil::cst_airfoil(&profile,chord,field(v,"samples")?,field(v,"degree")?,field(v,"control_count")?)?;
        return Ok(json!({"curve":curve,"chordMm":chord,"degree":curve.degree,"controlCount":curve.control_points.len(),"fitted":true}));
    }
    let budget:u64=field(v,"max_work")?;
    let model=airfoil::cst_algebraic(&profile,chord,budget)?;
    let properties=airfoil::cst_section_properties(&model,budget-model.work_used())?;
    let used=model.work_used()+properties.work_used;
    let moments=airfoil::cst_section_moments(&model,budget-used)?;
    Ok(json!({"curve":model.curve(),"chordMm":chord,"constructionErrorUpperMm":model.max_error(),"work":used+moments.work_used,
        "areaMm2":properties.area,"centroidMm":properties.centroid,"momentsAtOriginMm4":moments.about_origin,
        "momentsAtCentroidMm4":moments.about_centroid}))
}
