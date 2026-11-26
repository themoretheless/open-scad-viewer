use super::*;
use crate::editing::{exact_projective::UvAxis, exact_surface_derivative as plane};
use num_rational::BigRational;
fn graph(g: plane::SurfacePlaneGraph) -> Value {
    json!({"solveAxis":format!("{:?}",g.solve_axis),"position":format!("{:?}",g.position),"derivative":g.derivative.map(|x|x.to_string())})
}
pub(super) fn inspect(v: &Value, op: &str) -> Result<Value> {
    let surface = field(v, "surface")?;
    let normal = field(v, "plane_normal")?;
    let offset = field(v, "plane_offset_mm")?;
    let max_work = field(v, "max_work")?;
    let span = field(v, "knot_span")?;
    let solve_axis = match v["direction"].as_str() {
        Some("u") => UvAxis::U,
        Some("v") => UvAxis::V,
        _ => return Err(input("Expected a U or V direction")),
    };
    let (items, used) = match op {
        "sample_plane_branch" => {
            let ratios: Vec<[u64; 2]> = field(v, "transverse_ratios")?;
            crate::check(
                (1..=1024).contains(&ratios.len()) && ratios.iter().all(|&[n, d]| d > 0 && n <= d),
                "Expected 1..1024 ratios inside the normalized transverse span",
            )?;
            let transverse = ratios
                .into_iter()
                .map(|[n, d]| BigRational::new(n.into(), d.into()))
                .collect::<Vec<_>>();
            let r = plane::sample_surface_plane_graph(
                &surface,
                normal,
                offset,
                &transverse,
                plane::SurfacePlaneGraphSampleOptions {
                    span,
                    solve_axis,
                    max_normalized_width: field(v, "parameter_width")?,
                    max_work,
                },
            )?;
            (r.outcome.map(|x|json!({"domain":x.domain,"graph":graph(x.graph),"samples":x.samples.into_iter().map(|s|json!({"transverse":s.transverse.to_string(),"root":s.root.map(|q|q.to_string())})).collect::<Vec<_>>()})),r.work_used)
        }
        "plane_branch_uv_polyline" => {
            let r = plane::approximate_surface_plane_graph(
                &surface,
                normal,
                offset,
                plane::SurfacePlanePolylineOptions {
                    span,
                    solve_axis,
                    max_normalized_error: field(v, "tolerance_uv")?,
                    max_segments: field(v, "max_segments")?,
                    max_work,
                },
            )?;
            (r.outcome.map(|x|json!({"domain":x.domain,"graph":graph(x.graph),"uv":x.uv.into_iter().map(|p|p.map(|q|q.to_string())).collect::<Vec<_>>(),"normalizedErrorUpper":x.normalized_error_upper.to_string()})),r.work_used)
        }
        _ => {
            let r = plane::approximate_surface_plane_graph_xyz(
                &surface,
                normal,
                offset,
                plane::SurfacePlaneWorldPolylineOptions {
                    span,
                    solve_axis,
                    max_xyz_error: field(v, "tolerance_mm")?,
                    max_segments: field(v, "max_segments")?,
                    max_work,
                },
            )?;
            (r.outcome.map(|x|json!({"domain":x.domain,"graph":graph(x.graph),"uv":x.uv.into_iter().map(|p|p.map(|q|q.to_string())).collect::<Vec<_>>(),"xyz":x.xyz.into_iter().map(|p|p.map(|q|q.to_string())).collect::<Vec<_>>(),"xyzErrorUpper":x.xyz_error_upper.to_string()})),r.work_used)
        }
    };
    let mut out = match items {
        Ok(mut data) => {
            data["status"] = json!("complete");
            data
        }
        Err(reason) => json!({"status":"refused","reason":format!("{reason:?}")}),
    };
    out["workUsed"] = json!(used);
    Ok(out)
}
