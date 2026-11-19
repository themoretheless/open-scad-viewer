use super::*;
use crate::editing::exact_periodic_surface::*;
use num_rational::BigRational;

pub(super) const OPS: &[&str] = &[
    "evaluate_exact_wrapped_surface_point",
    "evaluate_exact_wrapped_surface_points",
    "lift_exact_periodic_surface_path",
    "exact_periodic_surface_path_winding",
    "exact_periodic_surface_path_signed_area",
    "exact_periodic_surface_path_centroid",
    "exact_periodic_surface_path_inertia",
];

fn ratio(x: BigRational) -> Value {
    json!({"numerator":x.numer().to_string(),"denominator":x.denom().to_string()})
}

fn centroid(p: PeriodicPathCentroidReport) -> Value {
    json!({"signedArea":p.signed_area.map(ratio),"centroid":p.centroid.map(|v|v.map(ratio)),
        "workUsed":p.work_used,"refusedCoordinate":p.refused_coordinate,"refusal":p.refusal.map(|r|format!("{r:?}"))})
}

pub(super) fn dispatch(request: &Value, op: &str) -> Result<Value> {
    let surface = field(request, "surface")?;
    let work = field(request, "max_work")?;
    if op == "evaluate_exact_wrapped_surface_point" {
        let p = evaluate_exact_wrapped_surface_point(&surface, field(request, "uv")?, work)?;
        return Ok(
            json!({"nativeUv":p.native_uv,"point":p.point,"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}"))}),
        );
    }
    let path: Vec<[f64; 2]> = field(request, "parameters")?;
    Ok(match op {
        "evaluate_exact_wrapped_surface_points" => {
            let p = evaluate_exact_wrapped_surface_points(&surface, &path, work)?;
            json!({"samples":p.samples,"workUsed":p.work_used,"refusedSample":p.refused_sample,"refusal":p.refusal.map(|r|format!("{r:?}"))})
        }
        "lift_exact_periodic_surface_path" => {
            let p = lift_exact_periodic_surface_path(&surface, &path, work)?;
            json!({"parameters":p.parameters,"workUsed":p.work_used,"refusedCoordinate":p.refused_coordinate,"refusal":p.refusal.map(|r|format!("{r:?}"))})
        }
        "exact_periodic_surface_path_winding" => {
            let p = exact_periodic_surface_path_winding(&surface, &path, work)?;
            json!({"winding":p.winding,"workUsed":p.work_used,"refusedCoordinate":p.refused_coordinate,"refusal":p.refusal.map(|r|format!("{r:?}"))})
        }
        "exact_periodic_surface_path_signed_area" => {
            let p = exact_periodic_surface_path_signed_area(&surface, &path, work)?;
            json!({"signedArea":p.signed_area.map(ratio),"workUsed":p.work_used,"refusedCoordinate":p.refused_coordinate,"refusal":p.refusal.map(|r|format!("{r:?}"))})
        }
        "exact_periodic_surface_path_centroid" => {
            centroid(exact_periodic_surface_path_centroid(&surface, &path, work)?)
        }
        "exact_periodic_surface_path_inertia" => {
            let p = exact_periodic_surface_path_inertia(&surface, &path, work)?;
            let mut report = centroid(p.geometry);
            report["centralMoments"] = json!(p.central_moments.map(|v| v.map(ratio)));
            report
        }
        _ => unreachable!(),
    })
}
