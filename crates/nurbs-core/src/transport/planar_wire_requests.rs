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

pub(super) fn area(v: &Value) -> Result<Value> {
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
