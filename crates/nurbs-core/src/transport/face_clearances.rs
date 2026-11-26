use super::*;
use crate::analysis::curve_measure::{
    inspect_curve_surface_clearance, inspect_surface_surface_clearance, inspect_trimmed_surface_clearance,
    ClearanceDecision,
};
use crate::trimmed_surface_distance::{curve_distance, swept_ball_clearance, TrimmedDistance};

pub(super) const OPS: &[&str] = &["curve_patch_clearance", "patch_pair_clearance",
    "trimmed_face_clearance", "curve_trimmed_face_distance", "swept_ball_face_clearance"];

fn decision(value: ClearanceDecision) -> &'static str {
    match value {
        ClearanceDecision::MeetsMinimum => "meets_minimum",
        ClearanceDecision::BelowMinimum => "below_minimum",
        ClearanceDecision::Unresolved => "unresolved",
    }
}

fn trimmed_report(r: TrimmedDistance) -> Value {
    json!({"lower_bound_mm":r.lower_bound_mm,"upper_bound_mm":r.upper_bound_mm,
           "parameters":r.parameters,"points":r.points,"converged":r.converged,
           "cells":r.cells,"domain_cells":r.domain_cells})
}

pub(super) fn apply(v: &Value, op: &str) -> Result<Value> {
    let face = field(v,"surface")?;
    let tolerance = field(v,"tolerance_mm")?;
    let cells = field(v,"max_cells")?;
    check(cells <= 1_000_000, "Surface clearance cell budget exceeded")?;
    if op == "curve_patch_clearance" {
        let (d,r) = inspect_curve_surface_clearance(&field(v,"curve")?,&face,field(v,"minimum_mm")?,tolerance,cells)?;
        return Ok(json!({"decision":decision(d),"bounds_mm":r.distance_interval,
            "curve_parameter":r.curve_parameter,"surface_parameters":r.surface_parameters,
            "points":r.points,"converged":r.converged,"cells":r.cells}));
    }
    if op == "patch_pair_clearance" {
        let (d,r) = inspect_surface_surface_clearance(&face,&field(v,"other_surface")?,field(v,"minimum_mm")?,tolerance,cells)?;
        return Ok(json!({"decision":decision(d),"bounds_mm":r.distance_interval_mm,
            "parameters":r.parameters,"points":r.points,"converged":r.converged,"cells":r.cells}));
    }
    let domain_cells = field(v,"max_domain_cells")?;
    check(domain_cells <= 1_000_000, "Surface trim cell budget exceeded")?;
    let uv = field(v,"tolerance_uv")?;
    let loops: Vec<Vec<crate::curve::Curve>> = field(v,"loops")?;
    let domain = crate::trim_domain::TrimDomain::new(&loops,uv)?;
    if op == "trimmed_face_clearance" {
        let other_loops: Vec<Vec<crate::curve::Curve>> = field(v,"other_loops")?;
        let other_domain = crate::trim_domain::TrimDomain::new(&other_loops,uv)?;
        let (d,r) = inspect_trimmed_surface_clearance((&face,&domain),(&field(v,"other_surface")?,&other_domain),
            field(v,"minimum_mm")?,tolerance,cells,domain_cells)?;
        let mut value = trimmed_report(r);
        value["decision"] = json!(decision(d));
        return Ok(value);
    }
    let curve = field(v,"curve")?;
    if op == "swept_ball_face_clearance" {
        let r = swept_ball_clearance(&curve,field(v,"radius_mm")?,&face,&domain,tolerance,cells,domain_cells)?;
        let mut value = trimmed_report(r.centerline_distance);
        value["signed_margin_mm"] = json!(r.signed_margin_mm);
        return Ok(value);
    }
    Ok(trimmed_report(curve_distance(&curve,&face,&domain,tolerance,cells,domain_cells)?))
}
