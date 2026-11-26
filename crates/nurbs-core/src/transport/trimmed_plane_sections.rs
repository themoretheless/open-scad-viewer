use super::*;
use crate::{
    analysis::{
        exact_plane_intersection as section,
        planar_coplanar_ray::CoplanarRayPartitionOptions,
        planar_trim_projection::{
            PlanarTrimOutcome, ProjectedTrimmedFace, project_coplanar_trim_loops,
        },
    },
    curve::Curve,
    surface::Surface,
};
use num_rational::BigRational;

#[path = "boundary_section_requests.rs"]
mod boundary;

#[path = "stored_section_requests.rs"]
mod storage;

fn ranges(intervals: Vec<[BigRational; 2]>) -> Vec<[String; 2]> {
    intervals
        .into_iter()
        .map(|a| a.map(|q| q.to_string()))
        .collect()
}
fn line(l: section::ExactSupportLine) -> Value {
    json!({"origin":l.origin.map(|q|q.to_string()),"direction":l.direction.map(|q|q.to_string()),"clockAxis":l.clock_axis})
}
pub(super) fn apply(v: &Value, op: &str) -> Result<Value> {
    let budget: u64 = field(v, "max_work")?;
    let mut left: Vec<Vec<Curve>> = field(v, "loops")?;
    let split = field(v, "left_loop_count")?;
    crate::check(
        split > 0 && split < left.len(),
        "Each face needs at least one contour",
    )?;
    let right = left.split_off(split);
    let mut remaining = budget;
    let mut project = |key: &str, rings: Vec<Vec<Curve>>| -> Result<ProjectedTrimmedFace> {
        if remaining == 0 {
            return Err(input("Trim projection work limit reached"));
        }
        let surface: Surface = field(v, key)?;
        let r = project_coplanar_trim_loops(&[surface], &[rings], remaining)?;
        remaining -= r.work_used;
        let PlanarTrimOutcome::Projected(mut faces) = r.outcome else {
            return Err(input(format!("Trim projection refused: {:?}", r.outcome)));
        };
        Ok(faces.remove(0))
    };
    let first = project("source", left)?;
    let second = project("right", right)?;
    if remaining == 0 {
        return Ok(json!({"status":"refused","reason":"ResourceLimit","workUsed":budget}));
    }
    if matches!(
        op,
        "shared_boundary_spans"
            | "shared_boundary_contacts"
            | "refine_boundary_section"
            | "bound_boundary_section"
    ) {
        return boundary::measure(v, op, &first, &second, remaining, budget - remaining);
    }
    let options = CoplanarRayPartitionOptions {
        max_curve_parameter_width: field(v, "parameter_width")?,
        winding_tolerance: field(v, "tolerance_uv")?,
        max_winding_cells: field(v, "max_winding_cells")?,
        max_work: remaining,
    };
    if matches!(
        op,
        "store_section_edge" | "optimize_section_edge" | "store_section_edges"
    ) {
        return storage::store(v, op, &first, &second, options, budget - remaining);
    }
    let (mut out, used, winding) = if op == "clip_plane_section" {
        use section::TrimmedPlanarIntersection::*;
        let r = section::intersect_trimmed_planar_faces(&first, &second, options)?;
        let out = match r.outcome {
            Transverse {
                line: l,
                shared_interior,
                possible,
            } => {
                json!({"status":"transverse","line":line(l),"sharedInterior":ranges(shared_interior),"possible":ranges(possible)})
            }
            Coincident => json!({"status":"coincident"}),
            Parallel => json!({"status":"parallel"}),
            Refused(reason) => json!({"status":"refused","reason":format!("{reason:?}")}),
        };
        (out, r.work_used, r.winding_cells)
    } else {
        use section::PlanarIntersectionSegments::*;
        let r = section::build_planar_intersection_segments(&first, &second, options)?;
        let out = match r.outcome {
            Transverse {
                line: l,
                segments,
                possible,
            } => json!({"status":"transverse","line":line(l),"possible":ranges(possible),
                "segments":segments.into_iter().map(|s|json!({"parameter":s.parameter.map(|q|q.to_string()),
                    "xyz":s.xyz.map(|p|p.map(|q|q.to_string())),
                    "pcurves":s.pcurves.map(|c|c.map(|p|p.map(|q|q.to_string())))})).collect::<Vec<_>>()}),
            Coincident => json!({"status":"coincident"}),
            Parallel => json!({"status":"parallel"}),
            Refused(reason) => json!({"status":"refused","reason":format!("{reason:?}")}),
        };
        (out, r.work_used, r.winding_cells)
    };
    out["workUsed"] = json!(budget - remaining + used);
    out["windingCells"] = json!(winding);
    Ok(out)
}
