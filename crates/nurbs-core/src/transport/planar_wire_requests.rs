use super::*;
use crate::check;
use crate::{
    analysis::{exact_affine_hull as chart, planar_trim_projection as wire},
    curve::Curve,
    surface::Surface,
};

pub(super) fn apply(v: &Value, op: &str) -> Result<Value> {
    let surface: Surface = field(v, "source")?;
    let loops: Vec<Vec<Curve>> = field(v, "loops")?;
    let budget = field(v, "max_work")?;
    let (wires, axes, used) = if op == "project_planar_wire" {
        let mut surfaces = vec![surface];
        surfaces.extend(field::<Vec<Surface>>(v, "others")?);
        let counts: Vec<usize> = field(v, "contour_counts")?;
        check(
            counts.len() == surfaces.len()
                && counts.iter().all(|n| *n > 0 && *n <= loops.len())
                && counts.iter().sum::<usize>() == loops.len(),
            "Provide the contour count for each surface",
        )?;
        let mut iter = loops.into_iter();
        let grouped = counts
            .into_iter()
            .map(|n| iter.by_ref().take(n).collect())
            .collect::<Vec<_>>();
        let report = wire::project_coplanar_trim_loops(&surfaces, &grouped, budget)?;
        let wire::PlanarTrimOutcome::Projected(mut faces) = report.outcome else {
            return Err(input(format!(
                "Planar contour projection refused: {:?}",
                report.outcome
            )));
        };
        let index: usize = field(v, "patch_index")?;
        check(
            index < faces.len(),
            "Selected face exceeds the projected group",
        )?;
        let face = faces.swap_remove(index);
        (face.loops, face.projection.retained_axes, report.work_used)
    } else {
        let chart = chart::project_planar_surface(&surface, budget)?;
        let chart::PlanarProjectionOutcome::Projected(p) = chart.outcome else {
            return Err(input("A planar contour lift requires a proved plane"));
        };
        check(
            chart.work_used < budget,
            "Planar contour lift work limit reached",
        )?;
        let report = wire::lift_planar_trim_loops(&p, &loops, budget - chart.work_used)?;
        let wire::PlanarContourLiftOutcome::Lifted(wires) = report.outcome else {
            return Err(input(format!(
                "Planar contour lift refused: {:?}",
                report.outcome
            )));
        };
        (wires, p.retained_axes, chart.work_used + report.work_used)
    };
    let ring: usize = field(v, "loop_index")?;
    let piece: usize = field(v, "piece")?;
    let curve = wires
        .get(ring)
        .and_then(|w| w.get(piece))
        .ok_or_else(|| input("Unknown contour or curve piece"))?;
    Ok(
        json!({"curve":curve,"retainedAxes":axes,"firstPole":curve.control_points[0],
        "domain":curve.domain(),"loopPieces":wires.iter().map(Vec::len).collect::<Vec<_>>(),"workUsed":used}),
    )
}

pub(super) fn measure_face(v: &Value, op: &str) -> Result<Value> {
    let surface: Surface = field(v, "source")?;
    let loops: Vec<Vec<Curve>> = field(v, "loops")?;
    let r = wire::project_coplanar_trim_loops(
        std::slice::from_ref(&surface),
        &[loops],
        field(v, "max_work")?,
    )?;
    let wire::PlanarTrimOutcome::Projected(faces) = r.outcome else {
        return Err(input(format!(
            "Planar area projection refused: {:?}",
            r.outcome
        )));
    };
    use crate::analysis::{planar_boundary_distance as distance, planar_face_perimeter};
    let boundary = |b: distance::PlanarBoundaryDistanceReport| {
        json!({
        "distanceMm":b.distance,"converged":b.converged,"stopReason":format!("{:?}",b.stop_reason),"cells":b.cells,
        "witness":{"contour":b.witness.contour,"curve":b.witness.curve,"parameter":b.witness.parameter,
        "pointBounds":b.witness.point_bounds,"distanceUpperMm":b.witness.distance_upper}})
    };
    match op {
        "cast_trimmed_face_ray" => {
            use crate::analysis::planar_face_ray::{self, PlanarFaceRayOutcome as Outcome};
            let remaining = field::<u64>(v, "max_work")? - r.work_used;
            if remaining == 0 {
                return Ok(json!({"status":"resourceLimit","workUsed":r.work_used}));
            }
            let ray = planar_face_ray::cast_projected_face_ray(
                &faces[0],
                field(v, "query_point")?,
                field(v, "ray_direction")?,
                field(v, "tolerance_uv")?,
                field(v, "max_winding_cells")?,
                remaining,
            )?;
            let intersection =
                |status: &str,
                 p: planar_face_ray::ExactPlaneRayPoint,
                 c: Option<crate::trim::trim_domain::Classification>| {
                    json!({
                "status":status,"parameter":p.parameter.to_string(),"point":p.point.map(|q|q.to_string()),
                "classification":c.map(|c|json!({"location":format!("{:?}",c.location),"reason":format!("{:?}",c.reason),"winding":c.winding,"cells":c.cells}))})
                };
            let mut report = match ray.outcome {
                Outcome::Hit {
                    intersection: p,
                    classification: c,
                } => intersection("hit", p, Some(c)),
                Outcome::Miss {
                    intersection: p,
                    classification: c,
                } => intersection("miss", p, c),
                Outcome::BoundaryUnresolved {
                    intersection: p,
                    classification: c,
                } => intersection("boundaryUnresolved", p, Some(c)),
                Outcome::Behind { parameter } => {
                    json!({"status":"behind","parameter":parameter.to_string()})
                }
                Outcome::Parallel => json!({"status":"parallel"}),
                Outcome::Coplanar => json!({"status":"coplanar"}),
                Outcome::ResourceLimit => json!({"status":"resourceLimit"}),
            };
            report["workUsed"] = json!(r.work_used + ray.work_used);
            return Ok(report);
        }
        "planar_face_perimeter" => {
            let b = planar_face_perimeter::bound_projected_face_perimeter(
                &faces[0],
                field(v, "tolerance_mm")?,
                field(v, "max_cells")?,
            )?;
            return Ok(
                json!({"perimeterMm":b.perimeter,"contoursMm":b.contours,"converged":b.within_tolerance,"cells":b.cells}),
            );
        }
        "planar_boundary_distance" => {
            return Ok(boundary(distance::distance_to_projected_face_boundary(
                &faces[0],
                field(v, "query_point")?,
                field(v, "tolerance_mm")?,
                field(v, "max_cells")?,
            )?));
        }
        "planar_material_distance" => {
            let b = distance::distance_to_projected_face(
                &faces[0],
                field(v, "query_point")?,
                distance::PlanarFaceDistanceOptions {
                    tolerance: field(v, "tolerance_mm")?,
                    winding_tolerance: field(v, "tolerance_uv")?,
                    max_winding_cells: field(v, "max_winding_cells")?,
                    max_boundary_cells: field(v, "max_cells")?,
                },
            )?;
            let c = b.projection_classification;
            return Ok(
                json!({"distanceMm":b.distance,"converged":b.converged,"projection":b.orthogonal_projection,
                "classification":{"location":format!("{:?}",c.location),"winding":c.winding,"reason":format!("{:?}",c.reason),"cells":c.cells},
                "boundary":b.boundary.map(boundary)}),
            );
        }
        _ => {}
    }
    let report = crate::analysis::planar_face_area::measure_projected_face_area(
        &faces[0],
        field(v, "tolerance_mm2")?,
        field(v, "max_cells")?,
    )?;
    Ok(
        json!({"signedAreaMm2":report.signed_area,"converged":report.converged,
        "cells":report.cells,"projectionWorkUsed":r.work_used,"retainedAxes":faces[0].projection.retained_axes}),
    )
}
