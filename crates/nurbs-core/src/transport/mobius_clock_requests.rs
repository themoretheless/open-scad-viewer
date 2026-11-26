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
            "affine_clock_curve" => clock::affine_curve_clock(&source, field(v, "clock_pairs")?, budget)?,
            "piecewise_clock_curve" => clock::mobius_piecewise_bezier_curve(
                &source,
                &field::<Vec<f64>>(v, "span_factors")?,
                budget,
            )?,
            "relayout_clock_curve" => clock::relayout_bezier_curve(
                &source,
                clock::BezierSpanLayout {
                    breaks: &field::<Vec<f64>>(v, "span_breaks")?,
                    factors: &field::<Vec<f64>>(v, "span_factors")?,
                },
                budget,
            )?,
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
        "affine_clock_surface" => clock::affine_surface_clock(&source, field(v, "clock_pairs")?, budget)?,
        "piecewise_clock_surface" => clock::mobius_piecewise_bezier_surface(
            &source,
            &field::<Vec<f64>>(v, "u_factors")?,
            &field::<Vec<f64>>(v, "v_factors")?,
            budget,
        )?,
        "relayout_clock_surface" => clock::relayout_bezier_surface(
            &source,
            [
                clock::BezierSpanLayout {
                    breaks: &field::<Vec<f64>>(v, "u_breaks")?,
                    factors: &field::<Vec<f64>>(v, "u_factors")?,
                },
                clock::BezierSpanLayout {
                    breaks: &field::<Vec<f64>>(v, "v_breaks")?,
                    factors: &field::<Vec<f64>>(v, "v_factors")?,
                },
            ],
            budget,
        )?,
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

struct AxisLayout {
    breaks: Vec<f64>,
    factors: Vec<f64>,
}
impl AxisLayout {
    fn view(&self) -> clock::BezierSpanLayout<'_> {
        clock::BezierSpanLayout {
            breaks: &self.breaks,
            factors: &self.factors,
        }
    }
}
fn layouts(v: &Value) -> Result<[AxisLayout; 2]> {
    Ok([
        AxisLayout {
            breaks: field(v, "u_breaks")?,
            factors: field(v, "u_factors")?,
        },
        AxisLayout {
            breaks: field(v, "v_breaks")?,
            factors: field(v, "v_factors")?,
        },
    ])
}
pub(super) fn pullback(v: &Value, op: &str) -> Result<Value> {
    let source = field(v, "source")?;
    let budget = field(v, "max_work")?;
    let r = if op == "transfer_uv_grid_piece" {
        let [u, v_axis] = layouts(v)?;
        clock::pullback_relayout_pcurve(
            &source,
            [
                &field::<Vec<f64>>(v, "old_u_breaks")?,
                &field::<Vec<f64>>(v, "old_v_breaks")?,
            ],
            [u.view(), v_axis.view()],
            budget,
        )?
    } else {
        clock::pullback_mobius_pcurve(
            &source,
            field(v, "uv_domain")?,
            field(v, "clock_factor")?,
            budget,
        )?
    };
    let mut pieces = r
        .pieces
        .ok_or_else(|| input(format!("UV contour transfer refused: {:?}", r.refusal)))?;
    let index: usize = field(v, "piece")?;
    if index >= pieces.len() {
        return Err(input(
            "Requested UV piece is outside the transferred contour",
        ));
    }
    let count = pieces.len();
    let curve = pieces.swap_remove(index);
    Ok(
        json!({"curve":curve,"pieceCount":count,"selectedPiece":index,"degree":curve.degree,
        "domain":curve.domain(),"workUsed":r.work_used}),
    )
}

pub(super) fn trim(v: &Value, op: &str) -> Result<Value> {
    let budget = field(v, "max_work")?;
    if op == "certify_uv_chart" {
        let curve = field(v, "source")?;
        let r = clock::certify_pcurve_chart_domain(&curve, field(v, "uv_domain")?, budget)?;
        let inside = r
            .inside
            .ok_or_else(|| input(format!("UV containment proof refused: {:?}", r.refusal)))?;
        return Ok(json!({"curve":curve,"inside":inside,"workUsed":r.work_used}));
    }
    let loops: Vec<Vec<crate::curve::Curve>> = field(v, "loops")?;
    let surface = field(v, "source")?;
    let r = if matches!(op, "affine_trim_surface" | "affine_trim_piece") {
        clock::affine_trimmed_surface_clock(&surface, &loops, field(v, "clock_pairs")?, budget)?
    } else if matches!(op, "transpose_trim_surface" | "transpose_trim_piece") {
        clock::transpose_trimmed_surface(&surface, &loops, budget)?
    } else if matches!(op, "remap_trim_surface" | "remap_trim_piece") {
        let order = match field::<usize>(v, "axis_order")? {
            0 => clock::TrimmedAxisOrder::Uv,
            1 => clock::TrimmedAxisOrder::Vu,
            _ => return Err(input("Axis order must be 0 (UV) or 1 (VU)")),
        };
        clock::remap_trimmed_surface_axes(&surface, &loops, order, field(v, "clock_pairs")?, budget)?
    } else if matches!(op, "relayout_trim_surface" | "relayout_trim_piece") {
        let [u, v_axis] = layouts(v)?;
        clock::relayout_trimmed_bezier_surface(&surface, &loops, [u.view(), v_axis.view()], budget)?
    } else {
        clock::mobius_trimmed_surface(&surface, &loops, field(v, "clock_factor")?, budget)?
    };
    let result = r
        .geometry
        .ok_or_else(|| input(format!("Trim transfer refused: {:?}", r.refusal)))?;
    let counts: Vec<_> = result.loops.iter().map(Vec::len).collect();
    if matches!(op, "affine_trim_surface" | "transpose_trim_surface" | "remap_trim_surface") {
        return Ok(json!({"surface":result.surface,"trimLoops":result.loops,"loopPieces":counts,"workUsed":r.work_used}));
    }
    if matches!(op, "relayout_trim_surface" | "reclock_trim_surface") {
        return Ok(json!({"surface":result.surface,"loopPieces":counts,"workUsed":r.work_used}));
    }
    let ring: usize = field(v, "loop_index")?;
    let piece: usize = field(v, "piece")?;
    let curve = result
        .loops
        .get(ring)
        .and_then(|edges| edges.get(piece))
        .ok_or_else(|| input("Selected trim piece is outside the transferred loops"))?;
    Ok(
        json!({"curve":curve,"loopPieces":counts,"selectedLoop":ring,"selectedPiece":piece,"domain":curve.domain(),"workUsed":r.work_used}),
    )
}

pub(super) fn lift(v: &Value) -> Result<Value> {
    let r = clock::lift_pcurve_to_surface(
        &field(v, "surface")?,
        &field(v, "source")?,
        field(v, "max_work")?,
    )?;
    let mut pieces = r
        .pieces
        .ok_or_else(|| input(format!("Space edge lift refused: {:?}", r.refusal)))?;
    let count = pieces.len();
    let index = field(v, "piece")?;
    if index >= count {
        return Err(input(
            "Selected space edge piece is outside the lifted trace",
        ));
    }
    let curve = pieces.swap_remove(index);
    Ok(
        json!({"curve":curve,"domain":curve.domain(),"degree":curve.degree,"pieceCount":count,"workUsed":r.work_used}),
    )
}
