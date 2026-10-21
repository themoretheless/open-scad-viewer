//! Continuous UV-to-world admission on an explicitly identified surface chart.
use super::{Admission, AdmissionStatus, Report, verify_partition};
use crate::{
    Result, check, curve::Curve, distance_bounds::Interval as I, surface::Surface,
    surface_parameter_bounds,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceSeamProof {
    /// Identical boundary parameter functions, including exact weight gauges.
    ClampedBoundaryIdentity,
    PeriodicStorageIdentity,
}
#[derive(Clone, Copy, Debug)]
pub struct SurfaceCorrespondenceLimits {
    pub tolerance_uv: f64,
    pub tolerance_world: f64,
    pub max_rectangles: usize,
    /// Shared original/candidate deviation cells across all curves.
    pub max_deviation_cells: usize,
}
#[derive(Clone, Debug)]
pub struct SurfaceCorrespondence {
    pub within_world_tolerance: bool,
    pub seams: [Option<SurfaceSeamProof>; 2],
    pub derivative_bounds: Option<surface_parameter_bounds::Bounds>,
    pub admissions: Vec<Admission>,
    pub deviation_cells: usize,
    /// Finite only after complete UV and supporting-surface admission.
    pub error_upper: f64,
    pub reason: Option<&'static str>,
}
/// Validate the actual source surface before proving a sufficient exact C0
/// identity of both authored-parameter boundary functions. None means this
/// proof is unavailable, not a sampled or approximate seam verdict.
pub fn verify_surface_seam_identity(
    surface: &Surface,
    axis: usize,
) -> Result<Option<SurfaceSeamProof>> {
    surface.validate()?;
    check(axis < 2, "Surface seam axis must be zero or one")?;
    Ok(seam(surface, axis))
}
pub(crate) fn seam(surface: &Surface, axis: usize) -> Option<SurfaceSeamProof> {
    let (degree, knots, n, periodic) = if axis == 0 {
        (
            surface.degree_u,
            &surface.knots_u,
            surface.control_points.len(),
            surface.periodic_u,
        )
    } else {
        (
            surface.degree_v,
            &surface.knots_v,
            surface.control_points[0].len(),
            surface.periodic_v,
        )
    };
    let [a, b] = [knots[degree], knots[n]];
    if periodic {
        let cycle = n - degree;
        let continuous = [a, b]
            .iter()
            .all(|x| knots.iter().filter(|k| *k == x).count() <= degree);
        let exact_knots = (0..knots.len() - cycle).all(|i| {
            crate::exact_products::sum_sign(&[
                (knots[i + cycle], 1.),
                (-knots[i], 1.),
                (-b, 1.),
                (a, 1.),
            ]) == std::cmp::Ordering::Equal
        });
        // Surface::validate already proves exact repeated controls and weights.
        return (continuous && exact_knots).then_some(SurfaceSeamProof::PeriodicStorageIdentity);
    }
    if !knots[..=degree].iter().all(|k| *k == a) || !knots[n..].iter().all(|k| *k == b) {
        return None;
    }
    let same = if axis == 0 {
        surface.control_points[0] == surface.control_points[n - 1]
            && surface.weights[0]
                .iter()
                .zip(&surface.weights[n - 1])
                .all(|(&x, &y)| {
                    crate::exact_products::sum_sign(&[
                        (x, surface.weights[n - 1][0]),
                        (-y, surface.weights[0][0]),
                    ]) == std::cmp::Ordering::Equal
                })
    } else {
        surface
            .control_points
            .iter()
            .zip(&surface.weights)
            .all(|(p, w)| {
                p[0] == p[n - 1]
                    && crate::exact_products::sum_sign(&[
                        (w[0], surface.weights[0][n - 1]),
                        (-w[n - 1], surface.weights[0][0]),
                    ]) == std::cmp::Ordering::Equal
            })
    };
    same.then_some(SurfaceSeamProof::ClampedBoundaryIdentity)
}
/// Bound world-space deviation of every source/candidate path continuously.
/// Chart must be the exact active surface domain. Each wrapped axis needs a
/// sufficient exact C0 seam proof; a periodic flag or sampled endpoint check
/// is insufficient. Nonwrapped source hulls stay in the surface chart.
/// Uses global authored-parameter derivative norms and continuous UV admission.
/// No surface injectivity, regularity or face topology is asserted.
pub fn verify_surface_correspondence(
    surface: &Surface,
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    limits: SurfaceCorrespondenceLimits,
) -> Result<SurfaceCorrespondence> {
    surface.validate()?;
    check(
        !sources.is_empty()
            && sources.len() <= 64
            && sources.len() == partitions.len()
            && partitions.iter().map(|p| p.pieces.len()).sum::<usize>() <= 256
            && limits.tolerance_uv.is_finite()
            && limits.tolerance_uv > 0.
            && limits.tolerance_world.is_finite()
            && limits.tolerance_world > 0.
            && (1..=10000).contains(&limits.max_rectangles)
            && (1..=100000).contains(&limits.max_deviation_cells)
            && chart
                .iter()
                .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1]),
        "Invalid supporting-surface curves, tolerances or budgets",
    )?;
    let mut out = SurfaceCorrespondence {
        within_world_tolerance: false,
        seams: [None; 2],
        derivative_bounds: None,
        admissions: Vec::new(),
        deviation_cells: 0,
        error_upper: f64::INFINITY,
        reason: None,
    };
    let domain = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    if chart != domain {
        out.reason = Some("supporting-chart-mismatch");
        return Ok(out);
    }
    for axis in 0..2 {
        if wrapped[axis] {
            out.seams[axis] = seam(surface, axis);
            if out.seams[axis].is_none() {
                out.reason = Some("supporting-seam-unproven");
                return Ok(out);
            }
        }
    }
    let bounds = surface_parameter_bounds::inspect(surface, limits.max_rectangles)?;
    let [lu, lv] = bounds.derivative_norm_upper_bounds;
    let scale = I::point(lu).add(I::point(lv))?.hi;
    // Subdivision target only; final outward world bound decides admission.
    let target = if scale > 0. {
        limits
            .tolerance_uv
            .min(limits.tolerance_world / scale * 0.99)
    } else {
        limits.tolerance_uv
    };
    let target = if target.is_finite() && target > 0. {
        target
    } else {
        limits.tolerance_uv
    };
    let mut uv_error = 0_f64;
    for (source, partition) in sources.iter().zip(partitions) {
        source.validate()?;
        check(
            !source.periodic && source.control_points[0].len() == 2,
            "Supporting-surface paths need lifted nonperiodic UV curves",
        )?;
        if source
            .control_points
            .iter()
            .any(|p| (0..2).any(|a| !wrapped[a] && (p[a] < chart[a][0] || chart[a][1] < p[a])))
        {
            out.reason = Some("nonwrapped-source-chart-unproven");
            return Ok(out);
        }
        for piece in &partition.pieces {
            piece.curve.validate()?;
            check(
                piece.curve.control_points[0].len() == 2
                    && piece.translation.iter().all(|x| x.is_finite()),
                "Supporting-surface candidate must be UV with finite translation",
            )?;
            if piece
                .curve
                .control_points
                .iter()
                .any(|p| (0..2).any(|a| p[a] < chart[a][0] || chart[a][1] < p[a]))
            {
                out.reason = Some("supporting-candidate-chart-unproven");
                return Ok(out);
            }
            if (0..2).any(|a| {
                if wrapped[a] {
                    crate::exact_products::sum_sign(&[
                        (piece.translation[a], 1.),
                        (-f64::from(piece.tile[a]), chart[a][1]),
                        (f64::from(piece.tile[a]), chart[a][0]),
                    ]) != std::cmp::Ordering::Equal
                } else {
                    piece.tile[a] != 0 || piece.translation[a] != 0.
                }
            }) {
                out.reason = Some("supporting-translation-unproven");
                return Ok(out);
            }
        }
        if out.deviation_cells == limits.max_deviation_cells {
            out.reason = Some("supporting-deviation-budget");
            return Ok(out);
        }
        let admission = verify_partition(
            source,
            partition,
            target,
            limits.max_deviation_cells - out.deviation_cells,
        )?;
        out.deviation_cells += admission.work;
        let admitted = admission.status == AdmissionStatus::WithinTolerance;
        uv_error = uv_error.max(admission.error_upper);
        out.admissions.push(admission);
        if !admitted {
            out.reason = Some(if target < limits.tolerance_uv {
                "world-tolerance-unproven"
            } else {
                "supporting-uv-deviation-unproven"
            });
            return Ok(out);
        }
    }
    // Each coordinate error is bounded by the Euclidean UV error. This sum
    // bound remains valid across finitely many exact C0 periodic seams.
    out.error_upper = I::point(lu).add(I::point(lv))?.mul(I::point(uv_error))?.hi;
    out.derivative_bounds = Some(bounds);
    out.within_world_tolerance = out.error_upper <= limits.tolerance_world;
    if !out.within_world_tolerance {
        out.reason = Some("world-tolerance-unproven");
    }
    Ok(out)
}
