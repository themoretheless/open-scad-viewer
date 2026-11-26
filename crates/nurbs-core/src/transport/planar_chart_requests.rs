use super::*;
use crate::analysis::exact_affine_hull as chart;
use chart::{PlanarProjectionOutcome, SharedPlanarProjectionOutcome, SurfaceAffineHull};

fn hull(value: SurfaceAffineHull) -> Result<Value> {
    let (dimension, witnesses) = match value {
        SurfaceAffineHull::Point(p) => (0, json!([p])),
        SurfaceAffineHull::Line(p) => (1, json!(p)),
        SurfaceAffineHull::Plane(p) => (2, json!(p)),
        SurfaceAffineHull::Spatial(p) => (3, json!(p)),
        SurfaceAffineHull::ResourceLimit => return Err(input("Affine hull work limit reached")),
    };
    Ok(json!({"dimension":dimension,"witnesses":witnesses}))
}

pub(super) fn apply(v: &Value, op: &str) -> Result<Value> {
    let budget = field(v, "max_work")?;
    let source = field(v, "source")?;
    if op == "active_surface_hull" {
        let r = chart::classify_surface_affine_hull(&source, budget)?;
        return Ok(json!({"surface":source,"hull":hull(r.hull)?,"workUsed":r.work_used}));
    }
    let (mut projection, used) = if op == "shared_plane_chart" {
        let mut surfaces = vec![source];
        surfaces.extend(field::<Vec<crate::surface::Surface>>(v, "others")?);
        let index: usize = field(v, "patch_index")?;
        let r = chart::project_coplanar_surfaces(&surfaces, budget)?;
        match r.outcome {
            SharedPlanarProjectionOutcome::Projected(mut charts) if index < charts.len() => {
                (charts.swap_remove(index), r.work_used)
            }
            SharedPlanarProjectionOutcome::Projected(_) => {
                return Err(input("Patch index exceeds the planar group"));
            }
            SharedPlanarProjectionOutcome::NotPlane(_) => {
                return Err(input("Surface group does not span a common plane"));
            }
            SharedPlanarProjectionOutcome::ResourceLimit => {
                return Err(input("Shared projection work limit reached"));
            }
        }
    } else {
        let r = chart::project_planar_surface(&source, budget)?;
        match r.outcome {
            PlanarProjectionOutcome::Projected(p) => (p, r.work_used),
            PlanarProjectionOutcome::NotPlane(_) => {
                return Err(input("Active surface does not span a plane"));
            }
            PlanarProjectionOutcome::ResourceLimit => {
                return Err(input("Planar projection work limit reached"));
            }
        }
    };
    let mut work_used = used;
    if op == "lift_plane_chart" {
        projection.surface = field(v, "edited")?;
        if used >= budget {
            return Err(input("Planar lift work limit reached"));
        }
        let r = chart::lift_planar_surface(&projection, budget - used)?;
        projection.surface = r
            .surface
            .ok_or_else(|| input(format!("Planar lift refused: {:?}", r.refusal)))?;
        work_used += r.work_used;
    }
    Ok(
        json!({"surface":projection.surface,"retainedAxes":projection.retained_axes,
        "omittedAxis":projection.omitted_axis,"offset":projection.offset.to_string(),
        "slopes":projection.slopes.map(|s|s.to_string()),"firstPole":projection.surface.control_points[0][0],"workUsed":work_used}),
    )
}

pub(super) fn support_line(v: &Value) -> Result<Value> {
    use crate::analysis::exact_plane_intersection::{
        SupportPlaneIntersection as Intersection, intersect_planar_supports,
    };
    let budget = field::<u64>(v, "max_work")?;
    let mut remaining = budget;
    let mut project = |key: &str| -> Result<chart::PlanarSurfaceProjection> {
        if remaining == 0 {
            return Err(input("Support plane projection work limit reached"));
        }
        let report = chart::project_planar_surface(&field(v, key)?, remaining)?;
        remaining -= report.work_used;
        match report.outcome {
            PlanarProjectionOutcome::Projected(p) => Ok(p),
            PlanarProjectionOutcome::NotPlane(_) => {
                Err(input(format!("{key} does not span a plane")))
            }
            PlanarProjectionOutcome::ResourceLimit => {
                Err(input("Support plane projection work limit reached"))
            }
        }
    };
    let first = project("source")?;
    let second = project("right")?;
    if remaining == 0 {
        return Ok(json!({"status":"resourceLimit","workUsed":budget}));
    }
    let r = intersect_planar_supports(&first, &second, remaining)?;
    let mut out = match r.outcome {
        Intersection::Line(l) => {
            json!({"status":"line","clockAxis":l.clock_axis,"origin":l.origin.map(|x|x.to_string()),"direction":l.direction.map(|x|x.to_string())})
        }
        Intersection::Coincident => json!({"status":"coincident"}),
        Intersection::Parallel => json!({"status":"parallel"}),
        Intersection::ResourceLimit => json!({"status":"resourceLimit"}),
    };
    out["workUsed"] = json!(budget - remaining + r.work_used);
    Ok(out)
}
