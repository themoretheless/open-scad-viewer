use super::*;
use crate::editing::exact_projective as clock;

fn landmark([original, new]: [f64; 2]) -> clock::MobiusLandmark {
    clock::MobiusLandmark { original, new }
}

pub(super) fn apply(v: &Value, op: &str) -> Result<Value> {
    let budget = field(v, "max_work")?;
    if op.ends_with("_curve") {
        let source = field(v, "source")?;
        let r = match op {
            "mobius_bezier_curve" => {
                clock::mobius_bezier_curve(&source, field(v, "clock_factor")?, budget)?
            }
            "mobius_spline_curve" => {
                clock::mobius_spline_curve(&source, field(v, "clock_factor")?, budget)?
            }
            _ => {
                clock::fit_mobius_spline_curve(&source, landmark(field(v, "clock_pairs")?), budget)?
            }
        };
        let curve = r
            .geometry
            .ok_or_else(|| input(format!("Exact curve clock refused: {:?}", r.refusal)))?;
        return Ok(json!({"curve":curve,"knots":curve.knots,"workUsed":r.work_used}));
    }
    let source = field(v, "source")?;
    let r = match op {
        "mobius_bezier_surface" => {
            clock::mobius_bezier_surface(&source, field(v, "clock_factor")?, budget)?
        }
        "mobius_spline_surface" => {
            clock::mobius_spline_surface(&source, field(v, "clock_factor")?, budget)?
        }
        _ => clock::fit_mobius_spline_surface(
            &source,
            field::<[[f64; 2]; 2]>(v, "clock_pairs")?.map(landmark),
            budget,
        )?,
    };
    let surface = r
        .geometry
        .ok_or_else(|| input(format!("Exact surface clock refused: {:?}", r.refusal)))?;
    Ok(
        json!({"surface":surface,"uKnots":surface.knots_u,"vKnots":surface.knots_v,"workUsed":r.work_used}),
    )
}
