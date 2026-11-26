use super::*;
pub(super) fn measure(
    v: &Value,
    op: &str,
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    work: u64,
    prior: u64,
) -> Result<Value> {
    if op == "bound_boundary_section" {
        use section::BoundedPlanarBoundaryIntersection::*;
        let r = section::bound_planar_face_boundary_intersection(
            left,
            right,
            field(v, "parameter_width")?,
            work,
        )?;
        let mut out = match r.outcome {
            Transverse {
                line: l,
                proven,
                possible,
            } => {
                json!({"status":"transverse","line":line(l),"proven":ranges(proven),"possible":ranges(possible)})
            }
            Parallel => json!({"status":"parallel"}),
            Coincident => json!({"status":"coincident"}),
            Refused(e) => json!({"status":"refused","reason":format!("{e:?}")}),
        };
        out["workUsed"] = json!(prior + r.work_used);
        return Ok(out);
    }
    let r = match op {
        "shared_boundary_spans" => section::intersect_planar_face_boundaries(left, right, work)?,
        "shared_boundary_contacts" => {
            section::intersect_planar_face_boundary_contacts(left, right, work)?
        }
        _ => section::refine_planar_face_boundary_extents(
            left,
            right,
            field(v, "parameter_width")?,
            work,
        )?,
    };
    use section::PlanarBoundaryIntersection::*;
    let mut out = match r.outcome {
        Transverse {
            line: l,
            shared_boundary,
        } => json!({"status":"transverse","line":line(l),"proven":ranges(shared_boundary)}),
        Parallel => json!({"status":"parallel"}),
        Coincident => json!({"status":"coincident"}),
        Refused(e) => json!({"status":"refused","reason":format!("{e:?}")}),
    };
    out["workUsed"] = json!(prior + r.work_used);
    Ok(out)
}
