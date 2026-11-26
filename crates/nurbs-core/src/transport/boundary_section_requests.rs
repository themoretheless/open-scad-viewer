use super::*;
pub(super) fn measure(
    v: &Value,
    op: &str,
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    work: u64,
    prior: u64,
) -> Result<Value> {
    if op == "match_quadratic_contacts" {
        use section::AlgebraicPlanarContacts as Contacts;
        let r = section::match_quadratic_planar_boundary_contacts(left, right, work)?;
        let mut out = match r.outcome {
            Contacts::Transverse { line: l, points } => {
                let clocks = points
                    .into_iter()
                    .map(super::events::clock)
                    .collect::<Vec<_>>();
                json!({"status":"transverse","line":line(l),"points":clocks})
            }
            Contacts::Parallel => json!({"status":"parallel"}),
            Contacts::Coincident => json!({"status":"coincident"}),
            Contacts::Refused(e) => json!({"status":"refused","reason":format!("{e:?}")}),
        };
        out["workUsed"] = json!(prior + r.work_used);
        return Ok(out);
    }
    if matches!(op, "bound_boundary_section" | "resolve_boundary_section") {
        use section::BoundedPlanarBoundaryIntersection::*;
        let (bounds, used, adaptive) = if op == "resolve_boundary_section" {
            let r = section::resolve_planar_face_boundary_intersection(
                left,
                right,
                section::AdaptiveBoundaryOptions {
                    xyz_tolerance: field(v, "xyz_tolerance")?,
                    max_passes: field(v, "max_passes")?,
                    max_work: work,
                },
            )?;
            (
                r.bounds,
                r.work_used,
                Some(
                    json!({"resolution":format!("{:?}",r.resolution),"passes":r.passes,
                "maxUnresolvedLengthSquared":r.max_unresolved_length_squared.map(|q|q.to_string())}),
                ),
            )
        } else {
            let r = section::bound_planar_face_boundary_intersection(
                left,
                right,
                field(v, "parameter_width")?,
                work,
            )?;
            (r.outcome, r.work_used, None)
        };
        let mut out = match bounds {
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
        out["workUsed"] = json!(prior + used);
        if let Some(report) = adaptive {
            out["refinement"] = report;
        }
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
