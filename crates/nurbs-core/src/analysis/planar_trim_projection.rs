//! Exact world-plane curves of coplanar trimmed tensor faces.
use super::exact_affine_hull::{
    PlanarSurfaceProjection, SharedPlanarProjectionOutcome, SurfaceAffineHull,
    project_coplanar_surfaces,
};
use crate::{
    Result, check,
    curve::Curve,
    editing::exact_projective::{PcurvePullbackRefusal, lift_pcurve_to_surface},
    surface::Surface,
    trim::trim_domain::exact_loop_joins,
};

#[derive(Clone, Debug)]
pub struct ProjectedTrimmedFace {
    pub projection: PlanarSurfaceProjection,
    /// Closed 2D world-plane curves in the original trim traversal order.
    pub loops: Vec<Vec<Curve>>,
}

#[derive(Clone, Debug)]
pub enum PlanarTrimOutcome {
    Projected(Vec<ProjectedTrimmedFace>),
    NotPlane(SurfaceAffineHull),
    OpenLoop {
        face: usize,
        contour: usize,
    },
    TraceRefused {
        face: usize,
        contour: usize,
        curve: usize,
        reason: PcurvePullbackRefusal,
    },
    ResourceLimit,
}

#[derive(Clone, Debug)]
pub struct PlanarTrimReport {
    pub outcome: PlanarTrimOutcome,
    pub work_used: u64,
}

/// Build 2D trim curves in one exact world-plane chart shared by all inputs.
/// Each output span keeps the original trim parameter. The operation checks
/// exact closure before and after composition. It certifies the boundary
/// images; planar region operations still require a simplicity audit.
pub fn project_coplanar_trim_loops(
    surfaces: &[Surface],
    trims: &[Vec<Vec<Curve>>],
    max_work: u64,
) -> Result<PlanarTrimReport> {
    check(
        surfaces.len() == trims.len()
            && trims.iter().all(|loops| {
                !loops.is_empty()
                    && loops.len() <= 256
                    && loops.iter().all(|curves| (1..=256).contains(&curves.len()))
            }),
        "Planar trim projection needs matching faces and nonempty bounded contours",
    )?;
    let report = project_coplanar_surfaces(surfaces, max_work)?;
    let mut used = report.work_used;
    let outcome = match report.outcome {
        SharedPlanarProjectionOutcome::Projected(charts) => {
            build(charts, trims, max_work, &mut used)?
        }
        SharedPlanarProjectionOutcome::NotPlane(hull) => PlanarTrimOutcome::NotPlane(hull),
        SharedPlanarProjectionOutcome::ResourceLimit => PlanarTrimOutcome::ResourceLimit,
    };
    Ok(PlanarTrimReport {
        outcome,
        work_used: used,
    })
}

fn build(
    charts: Vec<PlanarSurfaceProjection>,
    trims: &[Vec<Vec<Curve>>],
    max_work: u64,
    used: &mut u64,
) -> Result<PlanarTrimOutcome> {
    let mut faces = Vec::with_capacity(charts.len());
    for (face, (projection, contours)) in charts.into_iter().zip(trims).enumerate() {
        let mut loops = Vec::with_capacity(contours.len());
        for (contour, source) in contours.iter().enumerate() {
            if exact_loop_joins(source)? != Some(true) {
                return Ok(PlanarTrimOutcome::OpenLoop { face, contour });
            }
            let mut output = Vec::new();
            for (curve, uv) in source.iter().enumerate() {
                if *used == max_work {
                    return Ok(PlanarTrimOutcome::ResourceLimit);
                }
                let report = lift_pcurve_to_surface(&projection.surface, uv, max_work - *used)?;
                *used += report.work_used;
                let Some(mut pieces) = report.pieces else {
                    return Ok(PlanarTrimOutcome::TraceRefused {
                        face,
                        contour,
                        curve,
                        reason: report.refusal.unwrap_or(PcurvePullbackRefusal::Geometry(
                            crate::editing::exact_derivative::Refusal::UnprovedIdentity,
                        )),
                    });
                };
                if output.len() + pieces.len() > 256 {
                    return Ok(PlanarTrimOutcome::ResourceLimit);
                }
                for pole in pieces
                    .iter_mut()
                    .flat_map(|piece| &mut piece.control_points)
                {
                    pole.truncate(2);
                }
                output.extend(pieces);
            }
            if exact_loop_joins(&output)? != Some(true) {
                return Ok(PlanarTrimOutcome::OpenLoop { face, contour });
            }
            loops.push(output);
        }
        faces.push(ProjectedTrimmedFace { projection, loops });
    }
    Ok(PlanarTrimOutcome::Projected(faces))
}
