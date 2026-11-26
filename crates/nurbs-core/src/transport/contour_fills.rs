use super::*;
use super::rotating_tools::xy_curve;
use crate::analysis::curve_measure::*;
use crate::chord_winding::FillRule;
pub(super) const OPS:&[&str]=&["orientation_winding","curve_winding_at_point","contour_fill_at_point","curve_winding_over_box",
    "signed_contour_distance","contour_fill_over_box","signed_contour_box_distance"];

fn fill(r:PlanarCurveFillReport)->Value{
    json!({"winding":r.winding,"filled":r.filled,"workUsed":r.work_cells})
}
fn distance(r:PlanarCurveSignedDistanceReport)->Value{
    json!({"signedDistance":r.signed_bounds,"fill":fill(r.fill),"witnessContour":r.witness_contour,
        "witnessParameter":r.witness_parameter,"distanceCells":r.distance_cells,"stopReason":format!("{:?}",r.distance_stop_reason)})
}
pub(super) fn dispatch(v:&Value,op:&str)->Result<Value>{
    let tol=field(v,"parameter_tolerance")?;
    let budget=field(v,"max_cells")?;
    if matches!(op,"orientation_winding"|"curve_winding_at_point"|"curve_winding_over_box"){
        let c=xy_curve(field(v,"curve")?)?;
        let r=match op{
            "orientation_winding"=>measure_planar_orientation_winding(&c,tol,budget)?,
            "curve_winding_at_point"=>measure_planar_curve_winding_about_point(&c,field(v,"point")?,tol,budget)?,
            _=>measure_planar_curve_winding_over_box(&c,field(v,"bounds")?,tol,budget)?,
        };
        return Ok(json!({"winding":r.winding,"net":r.travel.net_bounds,"total":r.travel.total_bounds,"workUsed":r.travel.work_cells}));
    }
    let raw:Vec<crate::curve::Curve>=field(v,"curves")?;
    check((1..=32).contains(&raw.len()),"Expected one to 32 planar contours")?;
    let curves=raw.into_iter().map(xy_curve).collect::<Result<Vec<_>>>()?;
    let contours=curves.iter().collect::<Vec<_>>();
    let rule=if field(v,"even_odd")?{FillRule::EvenOdd}else{FillRule::NonZero};
    Ok(match op {
        "contour_fill_at_point"=>fill(classify_planar_curve_fill(&contours,field(v,"point")?,rule,tol,budget)?),
        "contour_fill_over_box"=>fill(classify_planar_curve_fill_over_box(&contours,field(v,"bounds")?,rule,tol,budget)?),
        "signed_contour_distance"=>distance(measure_planar_curve_fill_signed_distance(&contours,field(v,"point")?,rule,tol,field(v,"distance_tolerance_mm")?,budget,field(v,"max_distance_cells")?)?),
        "signed_contour_box_distance"=>{
            let r=measure_planar_curve_fill_signed_distance_over_box(&contours,field(v,"bounds")?,rule,tol,field(v,"distance_tolerance_mm")?,budget,field(v,"max_distance_cells")?)?;
            json!({"signedDistance":r.signed_bounds,"center":distance(r.center)})
        }
        _=>return Err(input("Unknown contour fill operation")),
    })
}
