use super::*;
use crate::analysis::surface_chord_bound as chord;

pub(super) fn inspect(v: &Value, op: &str) -> Result<Value> {
    let surface = field(v, "surface")?;
    let budget = field(v, "max_cells")?;
    match op {
        "surface_cell_interpolation_error" => {
            let upper = chord::cell_interpolation_error(&surface, field(v, "uv_domain")?, budget)?;
            Ok(json!({"interpolationUpperMm":upper}))
        }
        "surface_triangle_interpolation_error" => {
            let r = chord::triangle_interpolation_error(
                &surface,
                field(v, "triangle_uv")?,
                field(v, "vertices")?,
                budget,
            )?;
            Ok(
                json!({"interpolationUpperMm":r.interpolation,"vertexDisplacementUpperMm":r.vertex_displacement,"totalUpperMm":r.total}),
            )
        }
        "surface_grid_interpolation_error" => {
            let u: Vec<f64> = field(v, "u_lines")?;
            let w: Vec<f64> = field(v, "v_lines")?;
            let vertices: Vec<[f64; 3]> = field(v, "vertices")?;
            let r = chord::grid_interpolation_error(&surface, &u, &w, &vertices, budget)?;
            Ok(
                json!({"totalUpperMm":r.upper,"worstCell":r.worst_cell,"knotRectangles":r.knot_rectangles}),
            )
        }
        _ => Err(input("Unknown surface interpolation error query")),
    }
}
