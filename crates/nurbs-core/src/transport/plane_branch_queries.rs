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

    let solve_axis = match v["direction"].as_str() {
        Some("u") => UvAxis::U,
        Some("v") => UvAxis::V,
        _ => return Err(input("Expected a U or V direction")),
    };
    if matches!(op, "stitch_plane_strip" | "trace_plane_strip") {
        let strip = plane::SurfacePlaneGraphStripOptions {
            solve_axis,
            solved_span: field(v, "solved_span")?,
            transverse_spans: field(v, "transverse_spans")?,
            max_normalized_width: field(v, "parameter_width")?,
            max_work,
        };
        let describe = |s: plane::SurfacePlaneGraphStrip| json!({"cells":s.cells.into_iter().map(|c|json!({"span":c.span,"domain":c.domain,"graph":graph(c.graph)})).collect::<Vec<_>>(),"junctions":s.junctions.into_iter().map(|j|json!({"transverse":j.transverse,"solved":j.solved.map(|q|q.to_string())})).collect::<Vec<_>>()});
        let (report, used) = if op == "stitch_plane_strip" {
            let r = plane::stitch_surface_plane_graph_strip(&surface, normal, offset, strip)?;
            (r.outcome.map(describe), r.work_used)
        } else {
            let r = plane::approximate_surface_plane_strip_xyz(
                &surface,
                normal,
                offset,
                plane::SurfacePlaneWorldStripOptions {
                    strip,
                    max_xyz_error: field(v, "tolerance_mm")?,
                    max_segments_per_cell: field(v, "max_segments_per_cell")?,
                },
            )?;
            (r.outcome.map(|w|json!({"strip":describe(w.strip),"uv":w.uv.into_iter().map(|p|p.map(|q|q.to_string())).collect::<Vec<_>>(),"xyz":w.xyz.into_iter().map(|p|p.map(|q|q.to_string())).collect::<Vec<_>>(),"cellVertices":w.cell_vertices,"cellErrorUpper":w.cell_error_upper.into_iter().map(|q|q.to_string()).collect::<Vec<_>>()})),r.work_used)
        };
        let mut out = match report {
            Ok(mut v) => {
                v["status"] = json!("complete");
                v
            }
            Err(reason) => json!({"status":"refused","reason":format!("{reason:?}")}),
        };
        out["workUsed"] = json!(used);
        return Ok(out);
    }
    let span = field(v, "knot_span")?;
    if matches!(
        op,
        "split_cell_by_plane" | "store_plane_branch" | "pair_plane_branch"
    ) {
        let options = plane::SurfacePlaneWorldPolylineOptions {
            span,
            solve_axis,
            max_xyz_error: field(v, "tolerance_mm")?,
            max_segments: field(v, "max_segments")?,
            max_work,
        };
        if op == "split_cell_by_plane" {
            let r = plane::split_surface_cell_by_plane(
                &surface,
                normal,
                offset,
                options,
                field(v, "gap_tolerance_mm")?,
            )?;
            let cut = r
                .outcome
                .map_err(|reason| input(format!("Cell split refused: {reason:?}")))?;
            let positive: bool = field(v, "positive_side")?;
            let selected = if positive {
                &cut.positive
            } else {
                &cut.negative
            };
            let pair = cut.section;
            return Ok(
                json!({"trimLoops":[selected],"positive":cut.positive,"negative":cut.negative,
                "positiveSide":positive,"sectionCurve":pair.spatial.curve,"sectionPcurve":pair.pcurve,
                "xyzErrorUpper":pair.spatial.xyz_error_upper.to_string(),"planeResidualUpper":pair.spatial.plane_residual_upper.to_string(),
                "gapTolerance":pair.correspondence_tolerance.to_string(),"workUsed":r.work_used}),
            );
        }
        let (spatial, paired, used) = if op == "store_plane_branch" {
            let r = plane::store_surface_plane_section_curve(&surface, normal, offset, options)?;
            (
                r.outcome
                    .map_err(|e| input(format!("Section storage refused: {e:?}")))?,
                None,
                r.work_used,
            )
        } else {
            let r = plane::store_surface_plane_section_pair(
                &surface,
                normal,
                offset,
                options,
                field(v, "gap_tolerance_mm")?,
            )?;
            let pair = r
                .outcome
                .map_err(|e| input(format!("Section pairing refused: {e:?}")))?;
            (
                pair.spatial,
                Some((pair.pcurve, pair.correspondence_tolerance)),
                r.work_used,
            )
        };
        let mut out = json!({"domain":spatial.domain,"xyzErrorUpper":spatial.xyz_error_upper.to_string(),"planeResidualUpper":spatial.plane_residual_upper.to_string(),"spatial":spatial.curve,"workUsed":used});
        if let Some((pcurve, tolerance)) = paired {
            out["pcurve"] = json!(pcurve);
            out["gapTolerance"] = json!(tolerance.to_string());
            out["curve"] = if field(v, "use_pcurve")? {
                json!(pcurve)
            } else {
                json!(spatial.curve)
            };
        } else {
            out["curve"] = json!(spatial.curve);
        }
        return Ok(out);
    }
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
