//! Exact world-plane curves of coplanar trimmed tensor faces.
use super::exact_affine_hull::{
    PlanarLiftRefusal, PlanarSurfaceProjection, SharedPlanarProjectionOutcome, SurfaceAffineHull,
    lift_plane_point, project_coplanar_surfaces, validate_projection_axes,
};
use crate::{
    Result, check,
    core::bounded_rational::Work,
    curve::Curve,
    exact_geometry::exact_projective::{PcurvePullbackRefusal, lift_pcurve_to_surface},
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
                            crate::exact_geometry::exact_derivative::Refusal::UnprovedIdentity,
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

#[derive(Clone, Debug)]
pub enum PlanarContourLiftOutcome {
    Lifted(Vec<Vec<Curve>>),
    OpenLoop {
        contour: usize,
    },
    Refused {
        contour: usize,
        curve: usize,
        reason: PlanarLiftRefusal,
    },
}

#[derive(Clone, Debug)]
pub struct PlanarContourLiftReport {
    pub outcome: PlanarContourLiftOutcome,
    pub work_used: u64,
}

/// Lift exact closed 2D contours in the projection's world-plane coordinates.
/// The affine inverse acts on the Cartesian poles and retains every knot,
/// weight and parameter. Output curves are spatial boundary traces. Every
/// lifted pole must fit binary64 exactly.
pub fn lift_planar_trim_loops(
    projection: &PlanarSurfaceProjection,
    loops: &[Vec<Curve>],
    max_work: u64,
) -> Result<PlanarContourLiftReport> {
    validate_projection_axes(projection)?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Planar contour lift needs a valid arithmetic budget",
    )?;
    check(
        (1..=256).contains(&loops.len())
            && loops.iter().all(|curves| (1..=256).contains(&curves.len())),
        "Planar contour lift needs nonempty bounded contours",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = lift_loops(projection, loops, &mut work)?;
    Ok(PlanarContourLiftReport {
        outcome,
        work_used: work.used,
    })
}

fn lift_loops(
    projection: &PlanarSurfaceProjection,
    loops: &[Vec<Curve>],
    work: &mut Work,
) -> Result<PlanarContourLiftOutcome> {
    let mut lifted = Vec::with_capacity(loops.len());
    for (contour, source) in loops.iter().enumerate() {
        if exact_loop_joins(source)? != Some(true) {
            return Ok(PlanarContourLiftOutcome::OpenLoop { contour });
        }
        let mut output = Vec::with_capacity(source.len());
        for (curve, input) in source.iter().enumerate() {
            match lift_contour_curve(projection, input, work) {
                Ok(curve) => output.push(curve),
                Err(reason) => {
                    return Ok(PlanarContourLiftOutcome::Refused {
                        contour,
                        curve,
                        reason,
                    });
                }
            }
        }
        lifted.push(output);
    }
    Ok(PlanarContourLiftOutcome::Lifted(lifted))
}

fn lift_contour_curve(
    projection: &PlanarSurfaceProjection,
    input: &Curve,
    work: &mut Work,
) -> std::result::Result<Curve, PlanarLiftRefusal> {
    let mut curve = input.clone();
    for p in &mut curve.control_points {
        *p = lift_plane_point(projection, [p[0], p[1]], work)?.to_vec();
    }
    curve
        .validate()
        .map_err(|_| PlanarLiftRefusal::InvalidOutput)?;
    Ok(curve)
}
