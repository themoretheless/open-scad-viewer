//! Exact edits of periodic tensor-product surface axes.
pub use crate::trim::periodic_patch::{PeriodicTrimComponent, PeriodicTrimPatch};

use super::{
    exact_curve_merge::{GaugeMergeRefusal, Refusal, rotate_periodic_curve_at_exact_knot},
    exact_tensor_edit::row,
    rational_edit::Work,
    rational_factor::Refusal as ExactRefusal,
};
use crate::{
    Result,
    curve::Curve,
    surface::{Axis, Surface},
};

use num_rational::BigRational;

#[derive(Clone, Debug)]
pub struct Report {
    pub surface: Option<Surface>,
    pub work_used: u64,
    pub refusal: Option<GaugeMergeRefusal>,
}

/// Move a periodic surface seam to an existing knot. The chosen axis keeps
/// its period and parameter units, while the other axis is unchanged.
/// The complete net is permuted literally, including all rational weights.
pub fn rotate_periodic_surface_at_exact_knot(
    source: &Surface,
    axis: Axis,
    parameter: f64,
    max_work: u64,
) -> Result<Report> {
    source.validate()?;
    let reference = row(source, axis, 0);
    let rotation = rotate_periodic_curve_at_exact_knot(&reference, parameter, max_work)?;
    let Some(rotated) = rotation.curve else {
        return Ok(Report {
            surface: None,
            work_used: rotation.work_used,
            refusal: rotation.refusal,
        });
    };
    let p = reference.degree;
    let count = reference.control_points.len();
    let cycle = count - p;
    let Some(shift) = reference.knots[p..count]
        .iter()
        .position(|&knot| knot == parameter)
    else {
        return Ok(Report {
            surface: None,
            work_used: rotation.work_used,
            refusal: Some(GaugeMergeRefusal::Join(Refusal::InvalidOutput)),
        });
    };
    let indices = (0..count).map(|i| (i + shift) % cycle);
    let mut surface = source.clone();
    match axis {
        Axis::U => {
            surface.knots_u = rotated.knots;
            for (i, j) in indices.enumerate() {
                surface.control_points[i].clone_from(&source.control_points[j]);
                surface.weights[i].clone_from(&source.weights[j]);
            }
        }
        Axis::V => {
            surface.knots_v = rotated.knots;
            for (i, j) in indices.enumerate() {
                for u in 0..surface.control_points.len() {
                    surface.control_points[u][i].clone_from(&source.control_points[u][j]);
                    surface.weights[u][i] = source.weights[u][j];
                }
            }
        }
    }
    if surface.validate().is_err() {
        return Ok(Report {
            surface: None,
            work_used: rotation.work_used,
            refusal: Some(GaugeMergeRefusal::Join(Refusal::InvalidOutput)),
        });
    }
    Ok(Report {
        surface: Some(surface),
        work_used: rotation.work_used,
        refusal: None,
    })
}

/// Move a periodic surface seam inside an active span by inserting its
/// cyclic knot in every row. A single exact weight gauge covers the tensor.
pub fn insert_and_rotate_periodic_surface_seam(
    source: &Surface,
    axis: Axis,
    parameter: f64,
    max_work: u64,
) -> Result<Report> {
    source.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact periodic surface work must be in 1..=1000000",
    )?;
    let reference = row(source, axis, 0);
    let [lo, hi] = reference.domain();
    if !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(&reference)?
        || !(lo <= parameter && parameter < hi)
        || reference.knots.contains(&parameter)
    {
        return rotate_periodic_surface_at_exact_knot(source, axis, parameter, max_work);
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let refined = match rebuild_surface_axis(
        source,
        axis,
        reference.degree,
        true,
        &mut work,
        |row, work| super::exact_curve_merge::refine_periodic_seam_net(row, parameter, work),
    ) {
        Ok(refined) => refined,
        Err(reason) => {
            return Ok(Report {
                surface: None,
                work_used: work.used,
                refusal: Some(GaugeMergeRefusal::Exact(reason)),
            });
        }
    };
    if work.used == max_work {
        return Ok(Report {
            surface: None,
            work_used: work.used,
            refusal: Some(GaugeMergeRefusal::Exact(ExactRefusal::ResourceLimit)),
        });
    }
    let mut report =
        rotate_periodic_surface_at_exact_knot(&refined, axis, parameter, max_work - work.used)?;
    report.work_used += work.used;
    Ok(report)
}

/// Open a full periodic axis at its current seam, leaving the transverse
/// axis and both parameter domains unchanged.
pub fn open_exact_periodic_surface_axis(
    source: &Surface,
    axis: Axis,
    max_work: u64,
) -> Result<Report> {
    source.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact periodic surface work must be in 1..=1000000",
    )?;
    let reference = row(source, axis, 0);
    if !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(&reference)? {
        return Ok(Report {
            surface: None,
            work_used: 0,
            refusal: Some(GaugeMergeRefusal::Join(Refusal::InvalidOutput)),
        });
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    Ok(clamp_surface_axis(
        source,
        axis,
        reference.domain(),
        &mut work,
    ))
}

/// Restrict a periodic surface to a clamped strip along one axis, using the
/// original parameter units. The start is in the source half-open domain;
/// the end may cross its seam by up to one full period from the start.
pub fn restrict_exact_periodic_surface_axis(
    source: &Surface,
    axis: Axis,
    interval: [f64; 2],
    max_work: u64,
) -> Result<Report> {
    restrict_periodic_surface_interval(source, axis, interval, max_work, false)
}

/// Extract a clamped strip spanning full and partial turns of the selected
/// periodic surface axis. The anchor must lie in the original half-open
/// domain. Keep the lifted absolute parameters and the transverse basis.
pub fn restrict_exact_periodic_surface_over_turns(
    source: &Surface,
    axis: Axis,
    interval: [f64; 2],
    max_work: u64,
) -> Result<Report> {
    restrict_periodic_surface_interval(source, axis, interval, max_work, true)
}

fn restrict_periodic_surface_interval(
    source: &Surface,
    axis: Axis,
    interval: [f64; 2],
    max_work: u64,
    allow_multiple_turns: bool,
) -> Result<Report> {
    let [lo, hi] = interval;
    crate::check(
        lo.is_finite() && hi.is_finite() && lo < hi,
        "Periodic surface restriction needs an increasing finite interval",
    )?;
    let rotated = insert_and_rotate_periodic_surface_seam(source, axis, lo, max_work)?;
    let Some(surface) = rotated.surface else {
        return Ok(rotated);
    };
    let reference = row(&surface, axis, 0);
    let mut work = Work {
        used: rotated.work_used,
        limit: max_work,
    };
    if hi <= reference.domain()[1] {
        return Ok(clamp_surface_axis(&surface, axis, interval, &mut work));
    }
    if !allow_multiple_turns {
        return Ok(Report {
            surface: None,
            work_used: work.used,
            refusal: Some(GaugeMergeRefusal::Join(Refusal::NonAdjacentDomain)),
        });
    }
    let extended = match extend_tensor_support(&surface, axis, reference, hi, &mut work) {
        Ok(extended) => extended,
        Err(reason) => {
            return Ok(Report {
                surface: None,
                work_used: work.used,
                refusal: Some(GaugeMergeRefusal::Exact(reason)),
            });
        }
    };
    Ok(clamp_surface_axis(&extended, axis, interval, &mut work))
}

fn clamp_surface_axis(source: &Surface, axis: Axis, interval: [f64; 2], work: &mut Work) -> Report {
    let opened = rebuild_surface_axis(
        source,
        axis,
        axis_degree(source, axis),
        false,
        work,
        |row, work| super::exact_curve_merge::clamp_curve_net(row, interval, work),
    );
    match opened {
        Ok(surface) => Report {
            surface: Some(surface),
            work_used: work.used,
            refusal: None,
        },
        Err(reason) => Report {
            surface: None,
            work_used: work.used,
            refusal: Some(GaugeMergeRefusal::Exact(reason)),
        },
    }
}

fn axis_degree(surface: &Surface, axis: Axis) -> usize {
    match axis {
        Axis::U => surface.degree_u,
        Axis::V => surface.degree_v,
    }
}

fn rebuild_surface_axis(
    source: &Surface,
    axis: Axis,
    degree: usize,
    periodic: bool,
    work: &mut Work,
    transform: impl Fn(
        &crate::curve::Curve,
        &mut Work,
    ) -> std::result::Result<(Vec<Vec<BigRational>>, Vec<f64>), ExactRefusal>,
) -> std::result::Result<Surface, ExactRefusal> {
    use super::rational_edit::store;

    let [nu, nv] = [source.control_points.len(), source.control_points[0].len()];
    let transverse = match axis {
        Axis::U => nv,
        Axis::V => nu,
    };
    let mut channels: [Vec<BigRational>; 4] = std::array::from_fn(|_| Vec::new());
    let mut knots = Vec::new();
    for index in 0..transverse {
        let (net, refined_knots) = transform(&row(source, axis, index), work)?;
        knots = refined_knots;
        for pole in net {
            for (channel, value) in channels.iter_mut().zip(pole) {
                channel.push(value);
            }
        }
    }
    let (points, weights) = store(&channels, work)?;
    let count = knots.len() - degree - 1;
    let cycle = if periodic { count - degree } else { count };
    let mut refined = source.clone();
    match axis {
        Axis::U => {
            refined.knots_u = knots;
            refined.degree_u = degree;
            refined.periodic_u = periodic;
            refined.control_points = (0..count)
                .map(|i| {
                    (0..nv)
                        .map(|j| points[j * cycle + i % cycle].clone())
                        .collect()
                })
                .collect();
            refined.weights = (0..count)
                .map(|i| (0..nv).map(|j| weights[j * cycle + i % cycle]).collect())
                .collect();
        }
        Axis::V => {
            refined.knots_v = knots;
            refined.degree_v = degree;
            refined.periodic_v = periodic;
            refined.control_points = (0..nu)
                .map(|i| {
                    (0..count)
                        .map(|j| points[i * cycle + j % cycle].clone())
                        .collect()
                })
                .collect();
            refined.weights = (0..nu)
                .map(|i| (0..count).map(|j| weights[i * cycle + j % cycle]).collect())
                .collect();
        }
    }
    refined
        .validate()
        .map_err(|_| ExactRefusal::InvalidOutput)?;
    Ok(refined)
}

/// Extract a clamped rectangle across both periodic seams in the original
/// parameter units. Each interval covers at most one turn on its axis.
/// Both axis edits must be exactly storable and share one work budget.
pub fn restrict_exact_doubly_periodic_surface(
    source: &Surface,
    u: [f64; 2],
    v: [f64; 2],
    max_work: u64,
) -> Result<Report> {
    restrict_periodic_rectangle(source, u, v, max_work, false)
}

/// Extract a clamped rectangle across full and partial turns on both axes.
/// Each start lies in its source half-open domain. Preserve the lifted
/// absolute U and V coordinates and share one work budget across both cuts.
pub fn restrict_exact_doubly_periodic_surface_over_turns(
    source: &Surface,
    u: [f64; 2],
    v: [f64; 2],
    max_work: u64,
) -> Result<Report> {
    restrict_periodic_rectangle(source, u, v, max_work, true)
}

fn restrict_periodic_rectangle(
    source: &Surface,
    u: [f64; 2],
    v: [f64; 2],
    max_work: u64,
    allow_multiple_turns: bool,
) -> Result<Report> {
    crate::check(
        [u, v]
            .iter()
            .all(|[lo, hi]| lo.is_finite() && hi.is_finite() && lo < hi),
        "Periodic surface rectangle needs increasing finite intervals",
    )?;
    let first =
        restrict_periodic_surface_interval(source, Axis::U, u, max_work, allow_multiple_turns)?;
    let Some(strip) = first.surface else {
        return Ok(first);
    };
    if first.work_used == max_work {
        return Ok(Report {
            surface: None,
            work_used: first.work_used,
            refusal: Some(GaugeMergeRefusal::Exact(ExactRefusal::ResourceLimit)),
        });
    }
    let mut second = restrict_periodic_surface_interval(
        &strip,
        Axis::V,
        v,
        max_work - first.work_used,
        allow_multiple_turns,
    )?;
    second.work_used += first.work_used;
    Ok(second)
}

#[derive(Clone, Debug)]
pub struct StripPartitionReport {
    pub strips: Option<Vec<Surface>>,
    pub work_used: u64,
    /// Zero-based strip index, absent if preparing the periodic support failed.
    pub refused_strip: Option<usize>,
    pub refusal: Option<GaugeMergeRefusal>,
}

/// Partition one complete period into clamped strips along the selected axis.
/// Cuts are strictly increasing on the lifted clock. The transverse basis
/// stays fixed. Each strip has one common weight gauge and all strips share
/// one work budget. A refusal discards the entire partition.
pub fn partition_exact_periodic_surface_axis(
    source: &Surface,
    axis: Axis,
    seam: f64,
    cuts: &[f64],
    max_work: u64,
) -> Result<StripPartitionReport> {
    partition_periodic_surface_interval(source, axis, seam, None, cuts, max_work)
}

/// Partition an absolute interval across full and partial periodic turns.
/// Cuts must increase strictly inside the interval. All strips share one
/// exact cyclic extension and one work budget.
pub fn partition_exact_periodic_surface_over_turns(
    source: &Surface,
    axis: Axis,
    interval: [f64; 2],
    cuts: &[f64],
    max_work: u64,
) -> Result<StripPartitionReport> {
    let [start, end] = interval;
    crate::check(
        start.is_finite() && end.is_finite() && start < end,
        "Periodic partition needs an increasing finite interval",
    )?;
    partition_periodic_surface_interval(source, axis, start, Some(end), cuts, max_work)
}

fn partition_periodic_surface_interval(
    source: &Surface,
    axis: Axis,
    seam: f64,
    end: Option<f64>,
    cuts: &[f64],
    max_work: u64,
) -> Result<StripPartitionReport> {
    crate::check(
        cuts.len() < 256,
        "Periodic partition supports at most 256 strips",
    )?;
    let rotated = insert_and_rotate_periodic_surface_seam(source, axis, seam, max_work)?;
    let Some(surface) = rotated.surface else {
        return Ok(StripPartitionReport {
            strips: None,
            work_used: rotated.work_used,
            refused_strip: None,
            refusal: rotated.refusal,
        });
    };
    let reference = row(&surface, axis, 0);
    let [start, one_turn_end] = reference.domain();
    let end = end.unwrap_or(one_turn_end);
    let mut left = start;
    for &cut in cuts {
        crate::check(
            left < cut && cut < end,
            "Periodic strip cuts must increase inside the lifted period",
        )?;
        left = cut;
    }

    let mut work = Work {
        used: rotated.work_used,
        limit: max_work,
    };
    let surface = if end > one_turn_end {
        match extend_tensor_support(&surface, axis, reference, end, &mut work) {
            Ok(surface) => surface,
            Err(reason) => {
                return Ok(StripPartitionReport {
                    strips: None,
                    work_used: work.used,
                    refused_strip: None,
                    refusal: Some(GaugeMergeRefusal::Exact(reason)),
                });
            }
        }
    } else {
        surface
    };
    let mut strips = Vec::with_capacity(cuts.len() + 1);
    let mut left = start;
    for (index, right) in cuts.iter().copied().chain([end]).enumerate() {
        let report = clamp_surface_axis(&surface, axis, [left, right], &mut work);
        let Some(strip) = report.surface else {
            return Ok(StripPartitionReport {
                strips: None,
                work_used: work.used,
                refused_strip: Some(index),
                refusal: report.refusal,
            });
        };
        strips.push(strip);
        left = right;
    }
    Ok(StripPartitionReport {
        strips: Some(strips),
        work_used: work.used,
        refused_strip: None,
        refusal: None,
    })
}

#[derive(Clone, Debug)]
pub struct PeriodicGridReport {
    /// Patches indexed first by the U interval and then by the V interval.
    pub patches: Option<Vec<Vec<Surface>>>,
    pub work_used: u64,
    /// Indices of the failed U strip and V patch, when available.
    pub refused_intervals: [Option<usize>; 2],
    pub refusal: Option<GaugeMergeRefusal>,
}

/// Partition one turn on each axis into a grid of exact clamped patches.
/// All patches use the lifted U and V clocks and share one work budget.
/// The grid is returned only if every patch succeeds.
pub fn partition_exact_doubly_periodic_surface(
    source: &Surface,
    seams: [f64; 2],
    u_cuts: &[f64],
    v_cuts: &[f64],
    max_work: u64,
) -> Result<PeriodicGridReport> {
    partition_periodic_grid(source, seams, [None, None], [u_cuts, v_cuts], max_work)
}

/// Partition a lifted rectangle spanning full or partial turns on both
/// axes. Cuts are absolute coordinates inside each requested interval.
pub fn partition_exact_doubly_periodic_surface_over_turns(
    source: &Surface,
    u: [f64; 2],
    v: [f64; 2],
    u_cuts: &[f64],
    v_cuts: &[f64],
    max_work: u64,
) -> Result<PeriodicGridReport> {
    crate::check(
        [u, v]
            .iter()
            .all(|[start, end]| start.is_finite() && end.is_finite() && start < end),
        "Periodic grid needs increasing finite intervals",
    )?;
    partition_periodic_grid(
        source,
        [u[0], v[0]],
        [Some(u[1]), Some(v[1])],
        [u_cuts, v_cuts],
        max_work,
    )
}

fn partition_periodic_grid(
    source: &Surface,
    seams: [f64; 2],
    ends: [Option<f64>; 2],
    [u_cuts, v_cuts]: [&[f64]; 2],
    max_work: u64,
) -> Result<PeriodicGridReport> {
    crate::check(
        u_cuts.len() < 256 && v_cuts.len() < 256 && (u_cuts.len() + 1) * (v_cuts.len() + 1) <= 256,
        "Periodic surface grid supports at most 256 patches",
    )?;
    let u =
        partition_periodic_surface_interval(source, Axis::U, seams[0], ends[0], u_cuts, max_work)?;
    let mut work_used = u.work_used;
    let Some(strips) = u.strips else {
        return Ok(PeriodicGridReport {
            patches: None,
            work_used,
            refused_intervals: [u.refused_strip, None],
            refusal: u.refusal,
        });
    };

    let mut patches = Vec::with_capacity(strips.len());
    for (index, strip) in strips.iter().enumerate() {
        if work_used == max_work {
            return Ok(PeriodicGridReport {
                patches: None,
                work_used,
                refused_intervals: [Some(index), None],
                refusal: Some(GaugeMergeRefusal::Exact(ExactRefusal::ResourceLimit)),
            });
        }
        let v = partition_periodic_surface_interval(
            strip,
            Axis::V,
            seams[1],
            ends[1],
            v_cuts,
            max_work - work_used,
        )?;
        work_used += v.work_used;
        let Some(row) = v.strips else {
            return Ok(PeriodicGridReport {
                patches: None,
                work_used,
                refused_intervals: [Some(index), v.refused_strip],
                refusal: v.refusal,
            });
        };
        patches.push(row);
    }
    Ok(PeriodicGridReport {
        patches: Some(patches),
        work_used,
        refused_intervals: [None, None],
        refusal: None,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicAssemblyRefusal {
    Join {
        strip: usize,
        reason: super::exact_surface_merge::Refusal,
    },
    Wrap {
        transverse_index: usize,
    },
    Exact(ExactRefusal),
}

#[derive(Clone, Debug)]
pub struct PeriodicAssemblyReport {
    pub surface: Option<Surface>,
    pub work_used: u64,
    pub refusal: Option<PeriodicAssemblyRefusal>,
}

/// Assemble compatible clamped strips into periodic storage along one axis.
/// Every join and the final wrap require literal agreement of poles and
/// weights. This proves C0 closure and preserves both parameter laws.
pub fn assemble_exact_periodic_surface_axis(
    strips: &[Surface],
    axis: Axis,
    max_work: u64,
) -> Result<PeriodicAssemblyReport> {
    crate::check(
        (2..=256).contains(&strips.len()),
        "Periodic assembly needs 2..=256 strips",
    )?;
    assemble_surface_axis(strips, axis, max_work, false)
}

/// Assemble periodic strips whose shared boundary weights differ by one
/// positive scale per strip. Exact global gauges preserve the full tensor
/// law, including the transverse direction.
pub fn assemble_periodic_surface_axis_with_exact_weight_scaling(
    strips: &[Surface],
    axis: Axis,
    max_work: u64,
) -> Result<PeriodicAssemblyReport> {
    crate::check(
        (2..=256).contains(&strips.len()),
        "Periodic assembly needs 2..=256 strips",
    )?;
    assemble_surface_axis(strips, axis, max_work, true)
}

fn assemble_surface_axis(
    strips: &[Surface],
    axis: Axis,
    max_work: u64,
    scale_weights: bool,
) -> Result<PeriodicAssemblyReport> {
    use PeriodicAssemblyRefusal as Failure;

    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Periodic assembly work must be in 1..=1000000",
    )?;
    for strip in strips {
        strip.validate()?;
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let reject = |reason, work_used| PeriodicAssemblyReport {
        surface: None,
        work_used,
        refusal: Some(reason),
    };
    let mut surface = strips[0].clone();
    for (strip, next) in strips.iter().enumerate().skip(1) {
        let controls = |s: &Surface| (s.control_points.len() * s.control_points[0].len()) as u64;
        if work.charge(controls(&surface) + controls(next)).is_err() {
            return Ok(reject(
                Failure::Exact(ExactRefusal::ResourceLimit),
                work.used,
            ));
        }
        let mut joined = super::exact_surface_merge::merge(&surface, next, axis)?;
        if scale_weights
            && matches!(
                joined.refusal,
                Some(super::exact_surface_merge::Refusal::SeamMismatch { .. })
            )
        {
            let (left, right) = match align_strip_weights(&surface, next, axis, &mut work) {
                Ok(pair) => pair,
                Err(reason) => return Ok(reject(Failure::Exact(reason), work.used)),
            };
            joined = super::exact_surface_merge::merge(&left, &right, axis)?;
        }
        let Some(merged) = joined.surface else {
            let reason = joined
                .refusal
                .unwrap_or(super::exact_surface_merge::Refusal::InvalidOutput);
            return Ok(reject(Failure::Join { strip, reason }, work.used));
        };
        surface = merged;
    }

    match close_surface_cycle(surface, axis, &mut work) {
        Ok(surface) => Ok(PeriodicAssemblyReport {
            surface: Some(surface),
            work_used: work.used,
            refusal: None,
        }),
        Err(reason) => Ok(reject(reason, work.used)),
    }
}

fn close_surface_cycle(
    mut surface: Surface,
    axis: Axis,
    work: &mut Work,
) -> std::result::Result<Surface, PeriodicAssemblyRefusal> {
    use PeriodicAssemblyRefusal as Failure;

    let reference = row(&surface, axis, 0);
    let p = reference.degree;
    let count = reference.control_points.len();
    if reference.periodic
        || !reference.knots[..=p]
            .iter()
            .all(|&knot| knot == reference.knots[p])
        || !reference.knots[count..]
            .iter()
            .all(|&knot| knot == reference.knots[count])
    {
        return Err(Failure::Exact(ExactRefusal::InvalidOutput));
    }
    let [nu, nv] = [
        surface.control_points.len(),
        surface.control_points[0].len(),
    ];
    let (depth, transverse) = match axis {
        Axis::U => (nu, nv),
        Axis::V => (nv, nu),
    };
    work.charge(transverse as u64)
        .map_err(|e| Failure::Exact(e.into()))?;
    for i in 0..transverse {
        let ((a, b), (c, d)) = match axis {
            Axis::U => ((0, i), (depth - 1, i)),
            Axis::V => ((i, 0), (i, depth - 1)),
        };
        if surface.control_points[a][b] != surface.control_points[c][d]
            || surface.weights[a][b] != surface.weights[c][d]
        {
            return Err(Failure::Wrap {
                transverse_index: i,
            });
        }
    }
    let cycle =
        super::exact_curve_merge::repeat_clamped_cycle(&reference, work).map_err(Failure::Exact)?;
    match axis {
        Axis::U => {
            surface.knots_u = cycle.knots;
            surface.periodic_u = true;
            surface
                .control_points
                .extend_from_within(1..surface.degree_u);
            surface.weights.extend_from_within(1..surface.degree_u);
        }
        Axis::V => {
            surface.knots_v = cycle.knots;
            surface.periodic_v = true;
            for points in &mut surface.control_points {
                points.extend_from_within(1..surface.degree_v);
            }
            for weights in &mut surface.weights {
                weights.extend_from_within(1..surface.degree_v);
            }
        }
    }
    surface
        .validate()
        .map_err(|_| Failure::Exact(ExactRefusal::InvalidOutput))?;
    Ok(surface)
}

#[derive(Clone, Debug)]
pub struct PeriodicGridAssemblyReport {
    pub surface: Option<Surface>,
    pub work_used: u64,
    /// U row whose V joins failed, absent for a failure in the final U pass.
    pub refused_row: Option<usize>,
    pub refusal: Option<PeriodicAssemblyRefusal>,
}

/// Assemble a rectangular grid indexed by U then V into a single doubly
/// periodic surface. Both directions use exact structural joins and C0
/// wraps, preserving all patch parameter laws. No partial surface is emitted.
pub fn assemble_exact_doubly_periodic_surface(
    patches: &[Vec<Surface>],
    max_work: u64,
) -> Result<PeriodicGridAssemblyReport> {
    assemble_periodic_grid(patches, max_work, false)
}

/// Assemble a periodic grid with independently scaled weights on each
/// patch. One positive scale per complete patch preserves its rational
/// parameter law. Every internal boundary and both wraps must agree.
pub fn assemble_doubly_periodic_surface_with_exact_weight_scaling(
    patches: &[Vec<Surface>],
    max_work: u64,
) -> Result<PeriodicGridAssemblyReport> {
    assemble_periodic_grid(patches, max_work, true)
}

fn assemble_periodic_grid(
    patches: &[Vec<Surface>],
    max_work: u64,
    scale_weights: bool,
) -> Result<PeriodicGridAssemblyReport> {
    let columns = patches.first().map_or(0, Vec::len);
    crate::check(
        (1..=256).contains(&patches.len())
            && (1..=256).contains(&columns)
            && patches.len() * columns <= 256
            && patches.iter().all(|row| row.len() == columns),
        "Periodic assembly needs a rectangular grid of 1..=256 patches",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Periodic assembly work must be in 1..=1000000",
    )?;
    let reject = |refused_row, work_used, refusal| PeriodicGridAssemblyReport {
        surface: None,
        work_used,
        refused_row,
        refusal,
    };
    let exhausted = Some(PeriodicAssemblyRefusal::Exact(ExactRefusal::ResourceLimit));
    let mut work_used = 0;
    let mut strips = Vec::with_capacity(patches.len());
    for (index, row) in patches.iter().enumerate() {
        if work_used == max_work {
            return Ok(reject(Some(index), work_used, exhausted));
        }
        let result = assemble_surface_axis(row, Axis::V, max_work - work_used, scale_weights)?;
        work_used += result.work_used;
        let Some(strip) = result.surface else {
            return Ok(reject(Some(index), work_used, result.refusal));
        };
        strips.push(strip);
    }
    if work_used == max_work {
        return Ok(reject(None, work_used, exhausted));
    }
    let result = assemble_surface_axis(&strips, Axis::U, max_work - work_used, scale_weights)?;
    Ok(PeriodicGridAssemblyReport {
        surface: result.surface,
        work_used: work_used + result.work_used,
        refused_row: None,
        refusal: result.refusal,
    })
}

fn align_strip_weights(
    left: &Surface,
    right: &Surface,
    axis: Axis,
    work: &mut Work,
) -> std::result::Result<(Surface, Surface), ExactRefusal> {
    let last_weight = match axis {
        Axis::U => left.weights[left.weights.len() - 1][0],
        Axis::V => left.weights[0][left.weights[0].len() - 1],
    };
    let a = work.lift(last_weight)?;
    let b = work.lift(right.weights[0][0])?;
    let scale = work.op(&a, &b, |a, b| a / b)?;
    let mut weights = Vec::new();
    for &weight in left.weights.iter().flatten() {
        weights.push(work.lift(weight)?);
    }
    for &weight in right.weights.iter().flatten() {
        let weight = work.lift(weight)?;
        weights.push(work.op(&weight, &scale, |a, b| a * b)?);
    }
    let (_, stored) = super::rational_edit::store(&[weights], work)?;
    let mut stored = stored.into_iter();
    let mut left = left.clone();
    let mut right = right.clone();
    for (weight, replacement) in left
        .weights
        .iter_mut()
        .chain(&mut right.weights)
        .flatten()
        .zip(&mut stored)
    {
        *weight = replacement;
    }
    Ok((left, right))
}

/// Refine compatible clamped transverse bases to their exact knot union,
/// then assemble the strips with proportional boundary weights.
pub fn assemble_periodic_surface_axis_with_exact_knot_union(
    strips: &[Surface],
    axis: Axis,
    max_work: u64,
) -> Result<PeriodicAssemblyReport> {
    use super::exact_surface_knot_insertion::{self as insertion, Refusal as InsertRefusal};
    use PeriodicAssemblyRefusal as Failure;

    crate::check(
        (2..=256).contains(&strips.len()),
        "Periodic assembly needs 2..=256 strips",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Periodic assembly work must be in 1..=1000000",
    )?;
    for strip in strips {
        strip.validate()?;
    }
    let transverse = match axis {
        Axis::U => Axis::V,
        Axis::V => Axis::U,
    };
    let reference = row(&strips[0], transverse, 0);
    let [a, b] = reference.domain();
    let mut bases = Vec::with_capacity(strips.len());
    let mut interior = Vec::new();
    let mut work_used = 0;
    let refused = |reason, work_used| PeriodicAssemblyReport {
        surface: None,
        work_used,
        refusal: Some(reason),
    };
    let exhausted = Failure::Exact(ExactRefusal::ResourceLimit);
    for (index, strip) in strips.iter().enumerate() {
        let basis = row(strip, transverse, 0);
        let p = basis.degree;
        if basis.periodic
            || p != reference.degree
            || basis.domain() != [a, b]
            || basis.knots[..=p].iter().any(|&k| k != a)
            || basis.knots[basis.control_points.len()..]
                .iter()
                .any(|&k| k != b)
        {
            return Ok(refused(
                Failure::Join {
                    strip: index,
                    reason: super::exact_surface_merge::Refusal::OtherBasisMismatch,
                },
                work_used,
            ));
        }
        let knots = basis.knots;
        let charge = knots.len() as u64;
        if charge > max_work - work_used {
            return Ok(refused(exhausted, work_used));
        }
        work_used += charge;
        interior.extend(knots.iter().copied().filter(|&k| a < k && k < b));
        bases.push(knots);
    }
    interior.sort_by(f64::total_cmp);
    interior.dedup_by(|a, b| *a == *b);
    let comparisons = (interior.len() * bases.len()) as u64;
    if interior.len() > 256 || comparisons > max_work - work_used {
        return Ok(refused(exhausted, work_used));
    }
    work_used += comparisons;
    let multiplicity = |knots: &[f64], knot| {
        knots.partition_point(|&k| k <= knot) - knots.partition_point(|&k| k < knot)
    };
    let union: Vec<_> = interior
        .into_iter()
        .map(|knot| {
            let count = bases
                .iter()
                .map(|knots| multiplicity(knots, knot))
                .max()
                .unwrap_or(0);
            (knot, count)
        })
        .collect();
    if reference.degree + 1 + union.iter().map(|&(_, count)| count).sum::<usize>() > 256 {
        return Ok(refused(exhausted, work_used));
    }
    let mut refined = Vec::with_capacity(strips.len());
    for (strip, knots) in strips.iter().zip(&bases) {
        let mut surface = strip.clone();
        for &(knot, count) in &union {
            let missing = count - multiplicity(knots, knot);
            if missing == 0 {
                continue;
            }
            if work_used == max_work {
                return Ok(refused(exhausted, work_used));
            }
            let result =
                insertion::insert(&surface, transverse, knot, missing, max_work - work_used)?;
            work_used += result.work_used;
            if let Some(reason) = result.refusal {
                let reason = match reason {
                    InsertRefusal::Arithmetic(reason) => reason,
                    InsertRefusal::Curve {
                        decision: cad_predicates::BezierIdentity::Indeterminate(reason),
                        ..
                    } => ExactRefusal::PredicateIndeterminate(reason),
                    InsertRefusal::InvalidOutput => ExactRefusal::InvalidOutput,
                    _ => ExactRefusal::DifferentDefinition,
                };
                return Ok(refused(Failure::Exact(reason), work_used));
            }
            surface = result.surface;
        }
        refined.push(surface);
    }
    if work_used == max_work {
        return Ok(refused(exhausted, work_used));
    }
    let mut report = assemble_surface_axis(&refined, axis, max_work - work_used, true)?;
    report.work_used += work_used;
    Ok(report)
}

/// Elevate clamped transverse axes to the highest degree in the input,
/// refine them to the exact knot union, and assemble a periodic surface.
pub fn assemble_periodic_surface_axis_with_exact_basis_matching(
    strips: &[Surface],
    axis: Axis,
    max_work: u64,
) -> Result<PeriodicAssemblyReport> {
    assemble_with_matching_degrees(strips, axis, max_work, false)
}

/// Match degrees along the periodic assembly axis as well as the clamped
/// transverse bases. Elevation preserves both original parameter laws.
pub fn assemble_periodic_surface_axis_with_exact_degree_matching(
    strips: &[Surface],
    axis: Axis,
    max_work: u64,
) -> Result<PeriodicAssemblyReport> {
    assemble_with_matching_degrees(strips, axis, max_work, true)
}

fn assemble_with_matching_degrees(
    strips: &[Surface],
    axis: Axis,
    max_work: u64,
    match_selected: bool,
) -> Result<PeriodicAssemblyReport> {
    use PeriodicAssemblyRefusal as Failure;

    crate::check(
        (2..=256).contains(&strips.len()),
        "Periodic assembly needs 2..=256 strips",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Periodic assembly work must be in 1..=1000000",
    )?;
    for strip in strips {
        strip.validate()?;
    }
    let transverse = match axis {
        Axis::U => Axis::V,
        Axis::V => Axis::U,
    };
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let refuse = |reason, work_used| PeriodicAssemblyReport {
        surface: None,
        work_used,
        refusal: Some(reason),
    };
    let prepared = if match_selected {
        match elevate_strip_axes(
            strips,
            axis,
            super::exact_surface_merge::Refusal::NonClamped,
            &mut work,
        ) {
            Ok(prepared) => prepared,
            Err(reason) => return Ok(refuse(reason, work.used)),
        }
    } else {
        strips.to_vec()
    };
    let prepared = match elevate_strip_axes(
        &prepared,
        transverse,
        super::exact_surface_merge::Refusal::OtherBasisMismatch,
        &mut work,
    ) {
        Ok(prepared) => prepared,
        Err(reason) => return Ok(refuse(reason, work.used)),
    };
    if work.used == max_work {
        return Ok(refuse(
            Failure::Exact(ExactRefusal::ResourceLimit),
            work.used,
        ));
    }
    let mut report = assemble_periodic_surface_axis_with_exact_knot_union(
        &prepared,
        axis,
        max_work - work.used,
    )?;
    report.work_used += work.used;
    Ok(report)
}

fn elevate_strip_axes(
    strips: &[Surface],
    axis: Axis,
    mismatch: super::exact_surface_merge::Refusal,
    work: &mut Work,
) -> std::result::Result<Vec<Surface>, PeriodicAssemblyRefusal> {
    use PeriodicAssemblyRefusal as Failure;
    let degree = strips
        .iter()
        .map(|s| axis_degree(s, axis))
        .max()
        .unwrap_or(0);
    let mut prepared = Vec::with_capacity(strips.len());
    for (index, strip) in strips.iter().enumerate() {
        let basis = row(strip, axis, 0);
        let [lo, hi] = basis.domain();
        let p = basis.degree;
        if basis.periodic
            || !basis.knots[..=p].iter().all(|&k| k == lo)
            || !basis.knots[basis.control_points.len()..]
                .iter()
                .all(|&k| k == hi)
        {
            return Err(Failure::Join {
                strip: index,
                reason: mismatch,
            });
        }
        if p == degree {
            prepared.push(strip.clone());
            continue;
        }
        let mut breaks = basis.knots;
        breaks.dedup();
        if (breaks.len() - 1) * degree + 1 > 256 {
            return Err(Failure::Exact(ExactRefusal::ResourceLimit));
        }
        match rebuild_surface_axis(strip, axis, degree, false, work, |row, work| {
            super::exact_knot_insertion::elevated_homogeneous(row, degree, &breaks, work)
        }) {
            Ok(surface) => prepared.push(surface),
            Err(reason) => return Err(Failure::Exact(reason)),
        }
    }
    Ok(prepared)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicKnotRemovalRefusal {
    Open(GaugeMergeRefusal),
    Remove(super::exact_surface_knot_removal::Refusal),
    Close(PeriodicAssemblyRefusal),
    NoControlReduction,
}

#[derive(Clone, Debug)]
pub struct PeriodicKnotRemovalReport {
    pub surface: Option<Surface>,
    pub work_used: u64,
    pub refusal: Option<PeriodicKnotRemovalRefusal>,
}

/// Remove a redundant interior knot from a periodic tensor axis. Open the
/// original seam, prove exact tensor removal, then restore cyclic storage
/// on the same parameter domain. Success reduces the stored control count.
pub fn remove_exact_periodic_surface_knot(
    source: &Surface,
    axis: Axis,
    knot: f64,
    max_work: u64,
) -> Result<PeriodicKnotRemovalReport> {
    use super::exact_surface_knot_removal as removal;
    use PeriodicKnotRemovalRefusal as Failure;

    let opened = open_exact_periodic_surface_axis(source, axis, max_work)?;
    let refuse = |work_used, refusal| PeriodicKnotRemovalReport {
        surface: None,
        work_used,
        refusal: Some(refusal),
    };
    let Some(surface) = opened.surface else {
        return Ok(refuse(
            opened.work_used,
            Failure::Open(
                opened
                    .refusal
                    .unwrap_or(GaugeMergeRefusal::Exact(ExactRefusal::InvalidOutput)),
            ),
        ));
    };
    if opened.work_used == max_work {
        return Ok(refuse(
            opened.work_used,
            Failure::Remove(removal::Refusal::ResourceLimit),
        ));
    }
    let removed = removal::remove(&surface, axis, knot, max_work - opened.work_used)?;
    let mut work = Work {
        used: opened.work_used + removed.work_used,
        limit: max_work,
    };
    if !removed.removed {
        return Ok(refuse(
            work.used,
            Failure::Remove(removed.refusal.unwrap_or(removal::Refusal::InvalidOutput)),
        ));
    }
    let closed = match close_surface_cycle(removed.surface, axis, &mut work) {
        Ok(surface) => surface,
        Err(reason) => return Ok(refuse(work.used, Failure::Close(reason))),
    };
    if axis_control_count(&closed, axis) >= axis_control_count(source, axis) {
        return Ok(refuse(work.used, Failure::NoControlReduction));
    }
    Ok(PeriodicKnotRemovalReport {
        surface: Some(closed),
        work_used: work.used,
        refusal: None,
    })
}

fn axis_control_count(surface: &Surface, axis: Axis) -> usize {
    match axis {
        Axis::U => surface.control_points.len(),
        Axis::V => surface.control_points[0].len(),
    }
}

#[derive(Clone, Debug)]
pub struct PeriodicAxisSimplificationReport {
    pub surface: Option<Surface>,
    /// Net number of control rows or columns eliminated after reclosure.
    pub removed: usize,
    pub work_used: u64,
    pub refusal: Option<PeriodicKnotRemovalRefusal>,
}

/// Attempt each original interior knot occurrence on one periodic axis.
/// Open the seam once and verify every tensor removal before restoring the
/// original period. Budget or reclosure failure yields no partial result.
/// If a completed pass yields no net reduction, return the original surface.
pub fn simplify_exact_periodic_surface_axis(
    source: &Surface,
    axis: Axis,
    max_work: u64,
) -> Result<PeriodicAxisSimplificationReport> {
    use super::exact_surface_knot_removal as removal;
    use PeriodicKnotRemovalRefusal as Failure;

    let opened = open_exact_periodic_surface_axis(source, axis, max_work)?;
    let refuse = |work_used, refusal| PeriodicAxisSimplificationReport {
        surface: None,
        removed: 0,
        work_used,
        refusal: Some(refusal),
    };
    let Some(surface) = opened.surface else {
        return Ok(refuse(
            opened.work_used,
            Failure::Open(
                opened
                    .refusal
                    .unwrap_or(GaugeMergeRefusal::Exact(ExactRefusal::InvalidOutput)),
            ),
        ));
    };
    if opened.work_used == max_work {
        return Ok(refuse(
            opened.work_used,
            Failure::Remove(removal::Refusal::ResourceLimit),
        ));
    }
    let simplified = removal::simplify(&surface, axis, max_work - opened.work_used)?;
    let mut work = Work {
        used: opened.work_used + simplified.work_used,
        limit: max_work,
    };
    if !simplified.completed_pass {
        return Ok(refuse(
            work.used,
            Failure::Remove(removal::Refusal::ResourceLimit),
        ));
    }
    // Reclosing a clamped basis adds exactly degree minus one controls.
    let proposed_count =
        axis_control_count(&simplified.surface, axis) + axis_degree(source, axis) - 1;
    let removed = axis_control_count(source, axis).saturating_sub(proposed_count);
    let result = if removed == 0 {
        source.clone()
    } else {
        match close_surface_cycle(simplified.surface, axis, &mut work) {
            Ok(surface) => surface,
            Err(reason) => return Ok(refuse(work.used, Failure::Close(reason))),
        }
    };
    Ok(PeriodicAxisSimplificationReport {
        surface: Some(result),
        removed,
        work_used: work.used,
        refusal: None,
    })
}

/// Remove one occurrence of the original seam knot by moving the seam to
/// an existing strictly interior anchor first. The returned parameter
/// window starts at the anchor and keeps the exact original period.
pub fn remove_exact_periodic_surface_seam_knot(
    source: &Surface,
    axis: Axis,
    anchor: f64,
    max_work: u64,
) -> Result<PeriodicKnotRemovalReport> {
    use PeriodicKnotRemovalRefusal as Failure;

    source.validate()?;
    let [start, end] = row(source, axis, 0).domain();
    crate::check(
        start < anchor && anchor < end,
        "The new seam must be strictly inside the original period",
    )?;
    let rotated = rotate_periodic_surface_at_exact_knot(source, axis, anchor, max_work)?;
    let Some(surface) = rotated.surface else {
        return Ok(PeriodicKnotRemovalReport {
            surface: None,
            work_used: rotated.work_used,
            refusal: Some(Failure::Open(
                rotated
                    .refusal
                    .unwrap_or(GaugeMergeRefusal::Exact(ExactRefusal::InvalidOutput)),
            )),
        });
    };
    if rotated.work_used == max_work {
        return Ok(PeriodicKnotRemovalReport {
            surface: None,
            work_used: rotated.work_used,
            refusal: Some(Failure::Remove(
                super::exact_surface_knot_removal::Refusal::ResourceLimit,
            )),
        });
    }
    // The original right endpoint is the next exact copy of the old seam.
    let mut report =
        remove_exact_periodic_surface_knot(&surface, axis, end, max_work - rotated.work_used)?;
    report.work_used += rotated.work_used;
    Ok(report)
}

#[derive(Clone)]
pub struct DoublyPeriodicSimplificationReport {
    pub surface: Option<Surface>,
    /// Net controls eliminated along U and V, respectively.
    pub removed: [usize; 2],
    pub work_used: u64,
    /// Axis and stage of a failed simplification.
    pub refusal: Option<(Axis, PeriodicKnotRemovalRefusal)>,
}

/// Simplify both periodic axes with one work budget. Each pass retains its
/// original period and parameter domain. A refusal withholds the complete
/// result, even if the first axis was already reduced.
pub fn simplify_exact_doubly_periodic_surface(
    source: &Surface,
    max_work: u64,
) -> Result<DoublyPeriodicSimplificationReport> {
    let refuse = |axis, work_used, reason| DoublyPeriodicSimplificationReport {
        surface: None,
        removed: [0, 0],
        work_used,
        refusal: Some((axis, reason)),
    };
    let fallback =
        PeriodicKnotRemovalRefusal::Open(GaugeMergeRefusal::Exact(ExactRefusal::InvalidOutput));
    let first = simplify_exact_periodic_surface_axis(source, Axis::U, max_work)?;
    let Some(surface) = first.surface else {
        return Ok(refuse(
            Axis::U,
            first.work_used,
            first.refusal.unwrap_or(fallback),
        ));
    };
    if first.work_used == max_work {
        return Ok(refuse(
            Axis::V,
            first.work_used,
            PeriodicKnotRemovalRefusal::Open(GaugeMergeRefusal::Exact(ExactRefusal::ResourceLimit)),
        ));
    }
    let second =
        simplify_exact_periodic_surface_axis(&surface, Axis::V, max_work - first.work_used)?;
    let work_used = first.work_used + second.work_used;
    let Some(surface) = second.surface else {
        return Ok(refuse(
            Axis::V,
            work_used,
            second.refusal.unwrap_or(fallback),
        ));
    };
    Ok(DoublyPeriodicSimplificationReport {
        surface: Some(surface),
        removed: [first.removed, second.removed],
        work_used,
        refusal: None,
    })
}

#[derive(Clone, Debug)]
pub struct PeriodReductionReport {
    pub surface: Option<Surface>,
    /// Number of identical turns represented by the returned single turn.
    pub turns: usize,
    pub work_used: u64,
    pub refusal: Option<GaugeMergeRefusal>,
}

/// Reduce repeated periodic storage to the shortest supported cyclic basis
/// on one axis. Every control, weight and knot translate must agree exactly.
/// The active domain starts at the original seam and spans one retained turn.
pub fn reduce_exact_periodic_surface_period(
    source: &Surface,
    axis: Axis,
    max_work: u64,
) -> Result<PeriodReductionReport> {
    source.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact period reduction work must be in 1..=1000000",
    )?;
    let reference = row(source, axis, 0);
    let refused = |work_used, reason| PeriodReductionReport {
        surface: None,
        turns: 0,
        work_used,
        refusal: Some(reason),
    };
    if !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(&reference)? {
        return Ok(refused(0, GaugeMergeRefusal::Join(Refusal::InvalidOutput)));
    }

    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let cycle = reference.control_points.len() - reference.degree;
    let shortest = match find_repeated_cycle(source, &reference, axis, &mut work) {
        Ok(shortest) => shortest,
        Err(reason) => return Ok(refused(work.used, GaugeMergeRefusal::Exact(reason))),
    };
    let mut result = source.clone();
    let count = shortest + reference.degree;
    let knot_count = count + reference.degree + 1;
    match axis {
        Axis::U => {
            result.knots_u.truncate(knot_count);
            result.control_points.truncate(count);
            result.weights.truncate(count);
        }
        Axis::V => {
            result.knots_v.truncate(knot_count);
            for (points, weights) in result.control_points.iter_mut().zip(&mut result.weights) {
                points.truncate(count);
                weights.truncate(count);
            }
        }
    }
    if result.validate().is_err() {
        return Ok(refused(
            work.used,
            GaugeMergeRefusal::Join(Refusal::InvalidOutput),
        ));
    }
    Ok(PeriodReductionReport {
        surface: Some(result),
        turns: cycle / shortest,
        work_used: work.used,
        refusal: None,
    })
}

fn find_repeated_cycle(
    source: &Surface,
    reference: &crate::curve::Curve,
    axis: Axis,
    work: &mut Work,
) -> std::result::Result<usize, ExactRefusal> {
    let count = reference.control_points.len();
    let cycle = count - reference.degree;
    let transverse = match axis {
        Axis::U => source.control_points[0].len(),
        Axis::V => source.control_points.len(),
    };
    for candidate in reference.degree + 1..=cycle / 2 {
        if cycle % candidate != 0 {
            continue;
        }
        let repeats = |i: usize, j: usize| {
            let ((a, b), (c, d)) = match axis {
                Axis::U => ((i, j), (i % candidate, j)),
                Axis::V => ((j, i), (j, i % candidate)),
            };
            source.weights[a][b] == source.weights[c][d]
                && source.control_points[a][b] == source.control_points[c][d]
        };
        work.charge(((count - candidate) * transverse) as u64)?;
        if !(candidate..count).all(|i| (0..transverse).all(|j| repeats(i, j))) {
            continue;
        }
        let first = work.lift(reference.knots[reference.degree])?;
        let last = work.lift(reference.knots[reference.degree + candidate])?;
        let period = work.op(&last, &first, |a, b| a - b)?;
        if period <= BigRational::from_integer(0.into()) {
            continue;
        }
        let mut exact = true;
        for pair in reference.knots.iter().zip(&reference.knots[candidate..]) {
            let a = work.lift(*pair.0)?;
            let b = work.lift(*pair.1)?;
            if work.op(&b, &a, |a, b| a - b)? != period {
                exact = false;
                break;
            }
        }
        if exact {
            return Ok(candidate);
        }
    }
    Ok(cycle)
}

/// Store exactly `turns` copies of the source's active period on one axis.
/// The original seam and parameter units are preserved. Both the translated
/// knots and the final periodic basis must be exactly representable.
pub fn repeat_exact_periodic_surface_period(
    source: &Surface,
    axis: Axis,
    turns: usize,
    max_work: u64,
) -> Result<Report> {
    source.validate()?;
    crate::check(turns > 0, "Periodic repetition needs at least one turn")?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact period repetition work must be in 1..=1000000",
    )?;
    let reference = row(source, axis, 0);
    let refused = |work_used, reason| Report {
        surface: None,
        work_used,
        refusal: Some(reason),
    };
    if !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(&reference)? {
        return Ok(refused(0, GaugeMergeRefusal::Join(Refusal::InvalidOutput)));
    }
    let degree = reference.degree;
    let cycle = reference.control_points.len() - degree;
    if turns > (256 - degree) / cycle {
        return Ok(refused(0, GaugeMergeRefusal::Join(Refusal::ControlLimit)));
    }
    let count = cycle * turns + degree;
    let transverse = axis_control_count(
        source,
        match axis {
            Axis::U => Axis::V,
            Axis::V => Axis::U,
        },
    );
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut knots = reference.knots;
    let mut extend = || -> std::result::Result<(), ExactRefusal> {
        work.charge((count * transverse) as u64)?;
        let lo = work.lift(knots[degree])?;
        let hi = work.lift(knots[cycle + degree])?;
        let period = work.op(&hi, &lo, |hi, lo| hi - lo)?;
        while knots.len() < count + degree + 1 {
            let prior = work.lift(knots[knots.len() - cycle])?;
            let shifted = work.op(&prior, &period, |a, b| a + b)?;
            knots.push(work.float(&shifted)?);
        }
        Ok(())
    };
    if let Err(reason) = extend() {
        return Ok(refused(work.used, GaugeMergeRefusal::Exact(reason)));
    }
    let surface = surface_with_repeated_axis(source, axis, knots, count, cycle, true);
    if surface.validate().is_err() {
        return Ok(refused(
            work.used,
            GaugeMergeRefusal::Join(Refusal::InvalidOutput),
        ));
    }
    Ok(Report {
        surface: Some(surface),
        work_used: work.used,
        refusal: None,
    })
}

fn surface_with_repeated_axis(
    source: &Surface,
    axis: Axis,
    knots: Vec<f64>,
    count: usize,
    cycle: usize,
    periodic: bool,
) -> Surface {
    let mut surface = source.clone();
    match axis {
        Axis::U => {
            surface.knots_u = knots;
            surface.periodic_u = periodic;
            surface.control_points = (0..count)
                .map(|i| source.control_points[i % cycle].clone())
                .collect();
            surface.weights = (0..count)
                .map(|i| source.weights[i % cycle].clone())
                .collect();
        }
        Axis::V => {
            surface.knots_v = knots;
            surface.periodic_v = periodic;
            for (points, original) in surface
                .control_points
                .iter_mut()
                .zip(&source.control_points)
            {
                *points = (0..count).map(|i| original[i % cycle].clone()).collect();
            }
            for (weights, original) in surface.weights.iter_mut().zip(&source.weights) {
                *weights = (0..count).map(|i| original[i % cycle]).collect();
            }
        }
    }
    surface
}

fn extend_tensor_support(
    source: &Surface,
    axis: Axis,
    reference: crate::curve::Curve,
    end: f64,
    work: &mut Work,
) -> std::result::Result<Surface, ExactRefusal> {
    let degree = reference.degree;
    let mut count = reference.control_points.len();
    let cycle = count - degree;
    let [lo, hi] = reference.domain();
    let lo = work.lift(lo)?;
    let hi = work.lift(hi)?;
    let period = work.op(&hi, &lo, |a, b| a - b)?;
    let mut knots = reference.knots;
    // Keep one complete de Boor support window beyond the last active knot.
    while end > knots[count] {
        if count == 256 {
            return Err(ExactRefusal::ResourceLimit);
        }
        let prior = work.lift(knots[knots.len() - cycle])?;
        let shifted = work.op(&prior, &period, |a, b| a + b)?;
        knots.push(work.float(&shifted)?);
        count += 1;
    }
    let other = match axis {
        Axis::U => Axis::V,
        Axis::V => Axis::U,
    };
    work.charge((count * axis_control_count(source, other)) as u64)?;
    // This finite window need not end at an integral turn; its selected
    // axis is clamped by the caller before any surface is exposed.
    Ok(surface_with_repeated_axis(
        source, axis, knots, count, cycle, false,
    ))
}

#[derive(Clone, Debug)]
pub struct PeriodicPointReport {
    /// The actual parameters passed to the surface evaluator.
    pub native_uv: Option<[f64; 2]>,
    pub point: Option<[f64; 3]>,
    pub work_used: u64,
    pub refusal: Option<ExactRefusal>,
}

/// Evaluate a point using exact modulo reduction on each periodic axis.
/// The upper seam maps to the lower endpoint. Nonperiodic coordinates
/// must lie inside their domain. Refuse an unstorable reduced coordinate
/// instead of rounding it across a knot or seam.
pub fn evaluate_exact_wrapped_surface_point(
    source: &Surface,
    uv: [f64; 2],
    max_work: u64,
) -> Result<PeriodicPointReport> {
    source.validate()?;
    crate::check(
        uv.iter().all(|t| t.is_finite()),
        "Surface parameters must be finite",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact parameter wrapping work must be in 1..=1000000",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut native_uv = uv;
    for (i, axis) in [Axis::U, Axis::V].into_iter().enumerate() {
        let reference = row(source, axis, 0);
        let [lo, hi] = reference.domain();
        if !reference.periodic {
            crate::check(
                lo <= uv[i] && uv[i] <= hi,
                "Parameter is outside the nonperiodic axis",
            )?;
            continue;
        }
        let wrapped = if crate::analysis::curve_partition_agreement::verify_periodic_source_closure(
            &reference,
        )? {
            wrap_parameter_exact(uv[i], [lo, hi], &mut work)
        } else {
            Err(ExactRefusal::InvalidOutput)
        };
        match wrapped {
            Ok(value) => native_uv[i] = value,
            Err(reason) => {
                return Ok(PeriodicPointReport {
                    native_uv: None,
                    point: None,
                    work_used: work.used,
                    refusal: Some(reason),
                });
            }
        }
    }
    let point = source.evaluate_validated(native_uv[0], native_uv[1])?.point;
    Ok(PeriodicPointReport {
        native_uv: Some(native_uv),
        point: Some(point),
        work_used: work.used,
        refusal: None,
    })
}

fn wrap_parameter_exact(
    parameter: f64,
    [lo, hi]: [f64; 2],
    work: &mut Work,
) -> std::result::Result<f64, ExactRefusal> {
    if lo <= parameter && parameter < hi {
        return Ok(parameter);
    }
    let value = work.lift(parameter)?;
    let start = work.lift(lo)?;
    let end = work.lift(hi)?;
    let period = work.op(&end, &start, |end, start| end - start)?;
    let offset = work.op(&value, &start, |value, start| value - start)?;
    let turns = work.op(&offset, &period, |offset, period| (offset / period).floor())?;
    let shift = work.op(&turns, &period, |turns, period| turns * period)?;
    let reduced = work.op(&value, &shift, |value, shift| value - shift)?;
    Ok(work.float(&reduced)?)
}

#[derive(Clone, Debug)]
pub struct PeriodicPointBatchReport {
    /// Native parameters and evaluated points in input order.
    pub samples: Option<Vec<([f64; 2], [f64; 3])>>,
    pub work_used: u64,
    /// Index of the first point whose exact reduction failed.
    pub refused_sample: Option<usize>,
    pub refusal: Option<ExactRefusal>,
}

/// Evaluate a batch after exact periodic parameter reduction. Validate
/// the surface and prove its periodic axes once, then reuse one evaluator
/// and one reduction budget. No prefix is returned after a refusal.
pub fn evaluate_exact_wrapped_surface_points(
    source: &Surface,
    parameters: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicPointBatchReport> {
    use crate::surface::SurfaceEvaluator;

    crate::check(
        parameters.len() <= 4096,
        "Periodic surface batch supports at most 4096 points",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact parameter wrapping work must be in 1..=1000000",
    )?;
    let mut evaluator = SurfaceEvaluator::new(source)?;
    let mut axes = [([0., 0.], false); 2];
    for (i, axis) in [Axis::U, Axis::V].into_iter().enumerate() {
        let reference = row(source, axis, 0);
        if reference.periodic
            && !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(
                &reference,
            )?
        {
            return Ok(PeriodicPointBatchReport {
                samples: None,
                work_used: 0,
                refused_sample: None,
                refusal: Some(ExactRefusal::InvalidOutput),
            });
        }
        axes[i] = (reference.domain(), reference.periodic);
    }
    for uv in parameters {
        for (&value, ([lo, hi], periodic)) in uv.iter().zip(axes) {
            crate::check(value.is_finite(), "Surface parameters must be finite")?;
            crate::check(
                periodic || (lo <= value && value <= hi),
                "Parameter is outside the nonperiodic axis",
            )?;
        }
    }

    let mut samples = Vec::with_capacity(parameters.len());
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    for (index, &uv) in parameters.iter().enumerate() {
        let mut reduce = || -> std::result::Result<[f64; 2], ExactRefusal> {
            work.charge(1)?;
            let mut native = uv;
            for (value, (domain, periodic)) in native.iter_mut().zip(axes) {
                if periodic {
                    *value = wrap_parameter_exact(*value, domain, &mut work)?;
                }
            }
            Ok(native)
        };
        let native = match reduce() {
            Ok(native) => native,
            Err(reason) => {
                return Ok(PeriodicPointBatchReport {
                    samples: None,
                    work_used: work.used,
                    refused_sample: Some(index),
                    refusal: Some(reason),
                });
            }
        };
        samples.push((native, evaluator.evaluate(native[0], native[1])?.point));
    }
    Ok(PeriodicPointBatchReport {
        samples: Some(samples),
        work_used: work.used,
        refused_sample: None,
        refusal: None,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicLiftRefusal {
    Arithmetic(ExactRefusal),
    AmbiguousHalfTurn,
}

#[derive(Clone, Debug)]
pub struct PeriodicLiftReport {
    pub parameters: Option<Vec<[f64; 2]>>,
    pub work_used: u64,
    /// Zero-based sample and axis indices, when a sample could not be lifted.
    pub refused_coordinate: Option<[usize; 2]>,
    pub refusal: Option<PeriodicLiftRefusal>,
}

/// Lift a sampled path from native UV by choosing the shortest signed
/// step independently on each periodic axis. This convention requires
/// sampling densely enough for the intended step to be shorter than half
/// the period. An exact half-turn or an unstorable lift is refused.
pub fn lift_exact_periodic_surface_path(
    source: &Surface,
    native: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicLiftReport> {
    source.validate()?;
    crate::check(
        native.len() <= 4096,
        "Periodic path supports at most 4096 samples",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact periodic path work must be in 1..=1000000",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let refused = |coordinate, reason, work_used| PeriodicLiftReport {
        parameters: None,
        work_used,
        refused_coordinate: coordinate,
        refusal: Some(reason),
    };
    let mut output = native.to_vec();
    for (axis_index, axis) in [Axis::U, Axis::V].into_iter().enumerate() {
        let reference = row(source, axis, 0);
        let [lo, hi] = reference.domain();
        for uv in native {
            let t = uv[axis_index];
            crate::check(
                t.is_finite() && lo <= t && (t < hi || !reference.periodic && t == hi),
                "Path parameters must lie inside the native domain (half-open on periodic axes)",
            )?;
        }
        if !reference.periodic {
            continue;
        }
        if !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(&reference)?
        {
            return Ok(refused(
                None,
                PeriodicLiftRefusal::Arithmetic(ExactRefusal::InvalidOutput),
                work.used,
            ));
        }
        if let Err((index, reason)) =
            lift_periodic_axis(native, &mut output, axis_index, [lo, hi], &mut work)
        {
            return Ok(refused(Some([index, axis_index]), reason, work.used));
        }
    }
    Ok(PeriodicLiftReport {
        parameters: Some(output),
        work_used: work.used,
        refused_coordinate: None,
        refusal: None,
    })
}

impl From<crate::core::bounded_rational::Error> for PeriodicLiftRefusal {
    fn from(error: crate::core::bounded_rational::Error) -> Self {
        Self::Arithmetic(error.into())
    }
}

fn lift_periodic_axis(
    native: &[[f64; 2]],
    out: &mut [[f64; 2]],
    axis: usize,
    [lo, hi]: [f64; 2],
    work: &mut Work,
) -> std::result::Result<(), (usize, PeriodicLiftRefusal)> {
    use num_traits::Zero;

    let mut index = 0;
    let mut lift = || -> std::result::Result<(), PeriodicLiftRefusal> {
        let Some(first) = native.first() else {
            return Ok(());
        };
        let lo = work.lift(lo)?;
        let hi = work.lift(hi)?;
        let period = work.op(&hi, &lo, |hi, lo| hi - lo)?;
        let half = work.op(&period, &BigRational::from_integer(2.into()), |a, b| a / b)?;
        let mut previous = work.lift(first[axis])?;
        let mut shift = BigRational::zero();
        for (i, uv) in native.iter().enumerate().skip(1) {
            index = i;
            let value = work.lift(uv[axis])?;
            let change = shortest_step_turn(&value, &previous, &half, work)?;
            if change != 0 {
                shift = work.op(
                    &shift,
                    &period,
                    |a, b| {
                        if change > 0 { a + b } else { a - b }
                    },
                )?;
            }
            let lifted = work.op(&value, &shift, |value, shift| value + shift)?;
            out[i][axis] = work.float(&lifted)?;
            previous = value;
        }
        Ok(())
    };
    lift().map_err(|reason| (index, reason))
}

fn shortest_step_turn(
    value: &BigRational,
    previous: &BigRational,
    half: &BigRational,
    work: &mut Work,
) -> std::result::Result<i32, PeriodicLiftRefusal> {
    use num_traits::Signed;
    use std::cmp::Ordering;
    let step = work.op(value, previous, |a, b| a - b)?;
    match step.abs().cmp(half) {
        Ordering::Equal => Err(PeriodicLiftRefusal::AmbiguousHalfTurn),
        Ordering::Greater => Ok(if step.is_positive() { -1 } else { 1 }),
        Ordering::Less => Ok(0),
    }
}

#[derive(Clone, Debug)]
pub struct PeriodicPathWindingReport {
    /// Signed winding along U and V. Nonperiodic axes always contribute zero.
    pub winding: Option<[i32; 2]>,
    pub work_used: u64,
    pub refused_coordinate: Option<[usize; 2]>,
    pub refusal: Option<PeriodicLiftRefusal>,
}

/// Count exact signed turns in a closed sequence of native UV samples.
/// Successive samples follow the shortest-step convention on each axis.
/// The first and last native samples must coincide; half-turns are ambiguous.
pub fn exact_periodic_surface_path_winding(
    source: &Surface,
    native: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicPathWindingReport> {
    source.validate()?;
    crate::check(
        (2..=4096).contains(&native.len()) && native.first() == native.last(),
        "Periodic winding needs a closed path of 2..=4096 samples",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact periodic winding work must be in 1..=1000000",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut winding = [0; 2];
    let refused = |coordinate, reason, work_used| PeriodicPathWindingReport {
        winding: None,
        work_used,
        refused_coordinate: coordinate,
        refusal: Some(reason),
    };
    for (i, axis) in [Axis::U, Axis::V].into_iter().enumerate() {
        let reference = row(source, axis, 0);
        let [lo, hi] = reference.domain();
        crate::check(
            native.iter().all(|uv| {
                let t = uv[i];
                lo <= t && (t < hi || !reference.periodic && t == hi)
            }),
            "Path parameters must lie inside the native domain (half-open on periodic axes)",
        )?;
        if !reference.periodic {
            continue;
        }
        if !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(&reference)?
        {
            return Ok(refused(
                None,
                PeriodicLiftRefusal::Arithmetic(ExactRefusal::InvalidOutput),
                work.used,
            ));
        }
        let mut index = 0;
        let mut count = || -> std::result::Result<i32, PeriodicLiftRefusal> {
            let lo = work.lift(lo)?;
            let hi = work.lift(hi)?;
            let half = work.op(&hi, &lo, |a, b| {
                (a - b) / BigRational::from_integer(2.into())
            })?;
            let mut previous = work.lift(native[0][i])?;
            let mut turns = 0;
            for (sample, uv) in native.iter().enumerate().skip(1) {
                index = sample;
                let value = work.lift(uv[i])?;
                turns += shortest_step_turn(&value, &previous, &half, &mut work)?;
                previous = value;
            }
            Ok(turns)
        };
        match count() {
            Ok(turns) => winding[i] = turns,
            Err(reason) => return Ok(refused(Some([index, i]), reason, work.used)),
        }
    }
    Ok(PeriodicPathWindingReport {
        winding: Some(winding),
        work_used: work.used,
        refused_coordinate: None,
        refusal: None,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicPathAreaRefusal {
    Lift(PeriodicLiftRefusal),
    NonzeroWinding,
    Arithmetic(ExactRefusal),
}

#[derive(Clone, Debug)]
pub struct PeriodicPathAreaReport {
    /// Algebraic area of the lifted UV polygon, in squared parameter units.
    pub signed_area: Option<BigRational>,
    pub work_used: u64,
    pub refused_coordinate: Option<[usize; 2]>,
    pub refusal: Option<PeriodicPathAreaRefusal>,
}

/// Compute exact algebraic area for a closed native UV sample sequence
/// whose shortest-step lift is representable and closes in the plane.
/// Self-intersecting polygons contribute signed area with multiplicity.
pub fn exact_periodic_surface_path_signed_area(
    source: &Surface,
    native: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicPathAreaReport> {
    let moments = periodic_path_moments(source, native, max_work, 0)?.0;
    Ok(PeriodicPathAreaReport {
        signed_area: moments.signed_area,
        work_used: moments.work_used,
        refused_coordinate: moments.refused_coordinate,
        refusal: moments.refusal,
    })
}

#[derive(Clone, Debug)]
pub struct PeriodicPathCentroidReport {
    pub signed_area: Option<BigRational>,
    /// Centroid in the lifted UV chart. Undefined when the signed area is zero.
    pub centroid: Option<[BigRational; 2]>,
    pub work_used: u64,
    pub refused_coordinate: Option<[usize; 2]>,
    pub refusal: Option<PeriodicPathAreaRefusal>,
}

/// Compute the algebraic area and exact UV centroid of a closed lifted
/// polygon. The centroid is relative to the lift anchored at the first
/// sample, and uses signed weighting for self-intersecting paths.
pub fn exact_periodic_surface_path_centroid(
    source: &Surface,
    native: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicPathCentroidReport> {
    Ok(periodic_path_moments(source, native, max_work, 1)?.0)
}

#[derive(Clone, Debug)]
pub struct PeriodicPathInertiaReport {
    pub geometry: PeriodicPathCentroidReport,
    /// Signed integrals of centered U², U·V and V², in that order.
    /// Undefined when the centroid is undefined.
    pub central_moments: Option<[BigRational; 3]>,
}

/// Compute exact signed second moments of a lifted UV polygon about its
/// centroid. Reversing the contour reverses the sign of every integral.
pub fn exact_periodic_surface_path_inertia(
    source: &Surface,
    native: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicPathInertiaReport> {
    let (geometry, central_moments) = periodic_path_moments(source, native, max_work, 2)?;
    Ok(PeriodicPathInertiaReport {
        geometry,
        central_moments,
    })
}

fn periodic_path_moments(
    source: &Surface,
    native: &[[f64; 2]],
    max_work: u64,
    order: usize,
) -> Result<(PeriodicPathCentroidReport, Option<[BigRational; 3]>)> {
    use PeriodicPathAreaRefusal as Failure;
    use num_traits::Zero;
    crate::check(
        native.len() >= 4 && native.first() == native.last(),
        "Periodic path area needs a closed polygon with at least three edges",
    )?;
    let lift = lift_exact_periodic_surface_path(source, native, max_work)?;
    let refused = |reason, work_used, refused_coordinate| {
        (
            PeriodicPathCentroidReport {
                signed_area: None,
                centroid: None,
                work_used,
                refused_coordinate,
                refusal: Some(reason),
            },
            None,
        )
    };
    let Some(points) = lift.parameters else {
        let reason = lift
            .refusal
            .unwrap_or(PeriodicLiftRefusal::Arithmetic(ExactRefusal::InvalidOutput));
        return Ok(refused(
            Failure::Lift(reason),
            lift.work_used,
            lift.refused_coordinate,
        ));
    };
    if points.first() != points.last() {
        return Ok(refused(Failure::NonzeroWinding, lift.work_used, None));
    }
    let mut work = Work {
        used: lift.work_used,
        limit: max_work,
    };
    let mut integrate = || -> std::result::Result<_, ExactRefusal> {
        let mut moment: [BigRational; 2] = std::array::from_fn(|_| BigRational::zero());
        let axes = [(0, 0), (0, 1), (1, 1)];
        let mut second = axes.map(|_| BigRational::zero());
        let mut twice_area = BigRational::from_integer(0.into());
        let [x, y] = points[0];
        let mut previous = [work.lift(x)?, work.lift(y)?];
        for &[x, y] in &points[1..] {
            let current = [work.lift(x)?, work.lift(y)?];
            let forward = work.op(&previous[0], &current[1], |a, b| a * b)?;
            let backward = work.op(&previous[1], &current[0], |a, b| a * b)?;
            let cross = work.op(&forward, &backward, |a, b| a - b)?;
            twice_area = work.op(&twice_area, &cross, |a, b| a + b)?;
            if order >= 1 {
                for axis in 0..2 {
                    let sum = work.op(&previous[axis], &current[axis], |a, b| a + b)?;
                    let term = work.op(&sum, &cross, |a, b| a * b)?;
                    moment[axis] = work.op(&moment[axis], &term, |a, b| a + b)?;
                }
            }
            if order >= 2 {
                for (sum, (i, j)) in second.iter_mut().zip(axes) {
                    let a = work.op(&previous[i], &current[i], |a, b| a + b)?;
                    let b = work.op(&previous[j], &current[j], |a, b| a + b)?;
                    let mixed = work.op(&a, &b, |a, b| a * b)?;
                    let start = work.op(&previous[i], &previous[j], |a, b| a * b)?;
                    let end = work.op(&current[i], &current[j], |a, b| a * b)?;
                    let same = work.op(&start, &end, |a, b| a + b)?;
                    let factor = work.op(&mixed, &same, |a, b| a + b)?;
                    let term = work.op(&cross, &factor, |a, b| a * b)?;
                    *sum = work.op(sum, &term, |a, b| a + b)?;
                }
            }
            previous = current;
        }
        let area = work.op(&twice_area, &BigRational::from_integer(2.into()), |a, b| {
            a / b
        })?;
        let centroid = if order >= 1 && !twice_area.is_zero() {
            let divisor = work.op(&twice_area, &BigRational::from_integer(3.into()), |a, b| {
                a * b
            })?;
            Some([
                work.op(&moment[0], &divisor, |a, b| a / b)?,
                work.op(&moment[1], &divisor, |a, b| a / b)?,
            ])
        } else {
            None
        };
        let central = if let (true, Some(center)) = (order >= 2, &centroid) {
            let divisor = BigRational::from_integer(24.into());
            for (sum, (i, j)) in second.iter_mut().zip(axes) {
                let raw = work.op(sum, &divisor, |a, b| a / b)?;
                let product = work.op(&center[i], &center[j], |a, b| a * b)?;
                let translation = work.op(&area, &product, |a, b| a * b)?;
                *sum = work.op(&raw, &translation, |a, b| a - b)?;
            }
            Some(second)
        } else {
            None
        };
        Ok((area, centroid, central))
    };
    Ok(match integrate() {
        Ok((area, centroid, central)) => (
            PeriodicPathCentroidReport {
                signed_area: Some(area),
                centroid,
                work_used: work.used,
                refused_coordinate: None,
                refusal: None,
            },
            central,
        ),
        Err(reason) => refused(Failure::Arithmetic(reason), work.used, None),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicPathLocation {
    Outside,
    Inside,
    Boundary,
}

#[derive(Clone, Debug)]
pub struct PeriodicPathLocationReport {
    pub location: Option<PeriodicPathLocation>,
    pub work_used: u64,
    pub refusal: Option<PeriodicPathAreaRefusal>,
}

/// Classify a native UV point against a closed shortest-step polygon lift.
/// Use even-odd fill in the lift and accept any periodic copy of the query.
/// Boundary takes precedence over interior when copies overlap.
pub fn classify_exact_periodic_surface_path_point(
    source: &Surface,
    native: &[[f64; 2]],
    query: [f64; 2],
    max_work: u64,
) -> Result<PeriodicPathLocationReport> {
    let batch = classify_exact_periodic_surface_path_points(source, native, &[query], max_work)?;
    Ok(PeriodicPathLocationReport {
        location: batch.locations.and_then(|mut locations| locations.pop()),
        work_used: batch.work_used,
        refusal: batch.refusal,
    })
}

#[derive(Clone, Debug)]
pub struct PeriodicPathLocationBatchReport {
    pub locations: Option<Vec<PeriodicPathLocation>>,
    pub work_used: u64,
    pub refused_query: Option<usize>,
    pub refusal: Option<PeriodicPathAreaRefusal>,
}

/// Classify up to 4096 native UV points against one exact lifted polygon.
/// A failure withholds the entire result and identifies the query when
/// the failure occurs after the contour has been prepared.
pub fn classify_exact_periodic_surface_path_points(
    source: &Surface,
    native: &[[f64; 2]],
    queries: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicPathLocationBatchReport> {
    use PeriodicPathAreaRefusal as Failure;

    crate::check(
        native.len() >= 4 && native.first() == native.last(),
        "Periodic point classification needs a closed polygon",
    )?;
    crate::check(
        queries.len() <= 4096,
        "Periodic classification supports at most 4096 queries",
    )?;
    let lift = lift_exact_periodic_surface_path(source, native, max_work)?;
    let refused = |reason, work_used, refused_query| PeriodicPathLocationBatchReport {
        locations: None,
        refused_query,
        work_used,
        refusal: Some(reason),
    };
    let Some(points) = lift.parameters else {
        let reason = lift
            .refusal
            .unwrap_or(PeriodicLiftRefusal::Arithmetic(ExactRefusal::InvalidOutput));
        return Ok(refused(Failure::Lift(reason), lift.work_used, None));
    };
    if points.first() != points.last() {
        return Ok(refused(Failure::NonzeroWinding, lift.work_used, None));
    }
    let mut work = Work {
        used: lift.work_used,
        limit: max_work,
    };
    let mut domains = [None; 2];
    for (i, axis) in [Axis::U, Axis::V].into_iter().enumerate() {
        let reference = row(source, axis, 0);
        let [lo, hi] = reference.domain();
        for query in queries {
            crate::check(
                lo <= query[i] && (query[i] < hi || !reference.periodic && query[i] == hi),
                "Query must lie inside the native domain (half-open on periodic axes)",
            )?;
        }
        if reference.periodic {
            domains[i] = Some([lo, hi]);
        }
    }
    let polygon = match prepare_lifted_polygon(&points, domains, &mut work) {
        Ok(polygon) => polygon,
        Err(reason) => return Ok(refused(Failure::Arithmetic(reason), work.used, None)),
    };
    let mut locations = Vec::with_capacity(queries.len());
    for (index, &query) in queries.iter().enumerate() {
        match classify_lifted_polygon(&polygon, query, &mut work) {
            Ok(location) => locations.push(location),
            Err(reason) => return Ok(refused(Failure::Arithmetic(reason), work.used, Some(index))),
        }
    }
    Ok(PeriodicPathLocationBatchReport {
        locations: Some(locations),
        work_used: work.used,
        refused_query: None,
        refusal: None,
    })
}

struct LiftedPolygon {
    points: Vec<[BigRational; 2]>,
    periods: [Option<BigRational>; 2],
    bounds: [[BigRational; 2]; 2],
}

fn prepare_lifted_polygon(
    points: &[[f64; 2]],
    domains: [Option<[f64; 2]>; 2],
    work: &mut Work,
) -> std::result::Result<LiftedPolygon, ExactRefusal> {
    let points = points
        .iter()
        .map(|&[x, y]| Ok([work.lift(x)?, work.lift(y)?]))
        .collect::<std::result::Result<Vec<_>, ExactRefusal>>()?;
    let mut periods = [None, None];
    let mut bounds = std::array::from_fn(|_| [BigRational::default(), BigRational::default()]);
    for (i, domain) in domains.into_iter().enumerate() {
        if let Some([lo, hi]) = domain {
            let lo = work.lift(lo)?;
            let hi = work.lift(hi)?;
            periods[i] = Some(work.op(&hi, &lo, |a, b| a - b)?);
        }
        let min = points
            .iter()
            .map(|p| &p[i])
            .min()
            .ok_or(ExactRefusal::InvalidOutput)?;
        let max = points
            .iter()
            .map(|p| &p[i])
            .max()
            .ok_or(ExactRefusal::InvalidOutput)?;
        bounds[i] = [min.clone(), max.clone()];
    }
    Ok(LiftedPolygon {
        points,
        periods,
        bounds,
    })
}

fn classify_lifted_polygon(
    polygon: &LiftedPolygon,
    query: [f64; 2],
    work: &mut Work,
) -> std::result::Result<PeriodicPathLocation, ExactRefusal> {
    Ok(classify_lifted_polygon_with_shift(polygon, query, work)?.0)
}

fn classify_lifted_polygon_with_shift(
    polygon: &LiftedPolygon,
    query: [f64; 2],
    work: &mut Work,
) -> std::result::Result<(PeriodicPathLocation, [i64; 2]), ExactRefusal> {
    use PeriodicPathLocation as Location;
    use num_traits::ToPrimitive;

    let query = [work.lift(query[0])?, work.lift(query[1])?];
    let mut ranges = [(0, 0); 2];
    for (i, [min, max]) in polygon.bounds.iter().enumerate() {
        if let Some(period) = &polygon.periods[i] {
            let lo = work.op(min, &query[i], |a, b| a - b)?;
            let hi = work.op(max, &query[i], |a, b| a - b)?;
            ranges[i] = (
                work.op(&lo, period, |a, b| (a / b).ceil())?
                    .to_integer()
                    .to_i64()
                    .ok_or(ExactRefusal::ResourceLimit)?,
                work.op(&hi, period, |a, b| (a / b).floor())?
                    .to_integer()
                    .to_i64()
                    .ok_or(ExactRefusal::ResourceLimit)?,
            );
        } else if &query[i] < min || &query[i] > max {
            return Ok((Location::Outside, [0; 2]));
        }
    }
    let mut location = (Location::Outside, [0; 2]);
    for u in ranges[0].0..=ranges[0].1 {
        for v in ranges[1].0..=ranges[1].1 {
            let shift = |i: usize, turns: i64, work: &mut Work| {
                let Some(period) = &polygon.periods[i] else {
                    return Ok(query[i].clone());
                };
                let delta = work.op(period, &BigRational::from_integer(turns.into()), |a, b| {
                    a * b
                })?;
                work.op(&query[i], &delta, |a, b| a + b)
            };
            let p = [shift(0, u, work)?, shift(1, v, work)?];
            match classify_polygon_copy(&polygon.points, &p, work)? {
                Location::Boundary => return Ok((Location::Boundary, [u, v])),
                Location::Inside => location = (Location::Inside, [u, v]),
                Location::Outside => {}
            }
        }
    }
    Ok(location)
}

fn classify_polygon_copy(
    points: &[[BigRational; 2]],
    p: &[BigRational; 2],
    work: &mut Work,
) -> std::result::Result<PeriodicPathLocation, ExactRefusal> {
    use PeriodicPathLocation as Location;
    use num_traits::{Signed, Zero};

    let mut inside = false;
    for edge in points.windows(2) {
        let a = &edge[0];
        let b = &edge[1];
        let dx = work.op(&b[0], &a[0], |b, a| b - a)?;
        let dy = work.op(&b[1], &a[1], |b, a| b - a)?;
        let x = work.op(&p[0], &a[0], |p, a| p - a)?;
        let y = work.op(&p[1], &a[1], |p, a| p - a)?;
        let left = work.op(&dx, &y, |a, b| a * b)?;
        let right = work.op(&dy, &x, |a, b| a * b)?;
        let cross = work.op(&left, &right, |a, b| a - b)?;
        if cross.is_zero()
            && (0..2).all(|i| {
                let lo = (&a[i]).min(&b[i]);
                let hi = (&a[i]).max(&b[i]);
                lo <= &p[i] && &p[i] <= hi
            })
        {
            return Ok(Location::Boundary);
        }
        if (a[1] > p[1]) != (b[1] > p[1]) && cross.is_positive() == dy.is_positive() {
            inside = !inside;
        }
    }
    Ok(if inside {
        Location::Inside
    } else {
        Location::Outside
    })
}

#[derive(Clone, Debug)]
pub struct PeriodicRegionLocationReport {
    pub locations: Option<Vec<PeriodicPathLocation>>,
    pub work_used: u64,
    pub refused_loop: Option<usize>,
    pub refused_query: Option<usize>,
    pub refusal: Option<PeriodicPathAreaRefusal>,
}

/// Classify native UV queries against a family of closed periodic polygons.
/// Toggle membership for each projected contour that contains the point,
/// so nested contours define holes regardless of orientation. A point on
/// any contour is on the boundary. One budget covers all loops and queries.
pub fn classify_exact_periodic_surface_region_points(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    queries: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicRegionLocationReport> {
    use PeriodicPathLocation as Location;

    crate::check(
        (1..=256).contains(&contours.len()),
        "Periodic classification needs 1..=256 contours",
    )?;
    crate::check(
        queries.len() <= 4096,
        "Periodic classification supports at most 4096 queries",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact periodic region work must be in 1..=1000000",
    )?;
    let mut locations = vec![Location::Outside; queries.len()];
    let mut work_used = 0;
    for (index, contour) in contours.iter().enumerate() {
        let remaining = max_work - work_used;
        if remaining == 0 {
            return Ok(PeriodicRegionLocationReport {
                locations: None,
                work_used,
                refused_loop: Some(index),
                refused_query: None,
                refusal: Some(PeriodicPathAreaRefusal::Arithmetic(
                    ExactRefusal::ResourceLimit,
                )),
            });
        }
        let report =
            classify_exact_periodic_surface_path_points(source, contour, queries, remaining)?;
        work_used += report.work_used;
        let Some(classified) = report.locations else {
            return Ok(PeriodicRegionLocationReport {
                locations: None,
                work_used,
                refused_loop: Some(index),
                refused_query: report.refused_query,
                refusal: report.refusal,
            });
        };
        for (location, next) in locations.iter_mut().zip(classified) {
            *location = match (*location, next) {
                (Location::Boundary, _) | (_, Location::Boundary) => Location::Boundary,
                (Location::Inside, Location::Inside) => Location::Outside,
                (Location::Inside, Location::Outside) | (Location::Outside, Location::Inside) => {
                    Location::Inside
                }
                (Location::Outside, Location::Outside) => Location::Outside,
            };
        }
    }
    Ok(PeriodicRegionLocationReport {
        locations: Some(locations),
        work_used,
        refused_loop: None,
        refused_query: None,
        refusal: None,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicPathContact {
    /// Edge indices in the original sample order.
    pub edges: [usize; 2],
    /// Translation of the second edge in periods of U and V.
    pub turns: [i64; 2],
}

#[derive(Clone, Debug)]
pub struct PeriodicPathSimplicityReport {
    /// True if all cyclic edge pairs pass the exact contact check.
    pub simple: Option<bool>,
    pub contact: Option<PeriodicPathContact>,
    pub work_used: u64,
    pub refusal: Option<PeriodicPathAreaRefusal>,
}

/// Check whether a closed shortest-step polygon is simple on the periodic
/// chart. Detect crossings, touches and overlaps with its translated copies.
/// Adjacent edges in the original lift may share their single common vertex.
pub fn certify_exact_periodic_surface_path_simplicity(
    source: &Surface,
    native: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicPathSimplicityReport> {
    use PeriodicPathAreaRefusal as Failure;

    crate::check(
        native.len() >= 4 && native.first() == native.last(),
        "Periodic simplicity needs a closed polygon",
    )?;
    let lift = lift_exact_periodic_surface_path(source, native, max_work)?;
    let refused = |reason, work_used| PeriodicPathSimplicityReport {
        simple: None,
        contact: None,
        work_used,
        refusal: Some(reason),
    };
    let Some(points) = lift.parameters else {
        let reason = lift
            .refusal
            .unwrap_or(PeriodicLiftRefusal::Arithmetic(ExactRefusal::InvalidOutput));
        return Ok(refused(Failure::Lift(reason), lift.work_used));
    };
    if points.first() != points.last() {
        return Ok(refused(Failure::NonzeroWinding, lift.work_used));
    }
    let domains = [Axis::U, Axis::V].map(|axis| {
        let axis = row(source, axis, 0);
        axis.periodic.then(|| axis.domain())
    });
    let mut work = Work {
        used: lift.work_used,
        limit: max_work,
    };
    let outcome = prepare_lifted_polygon(&points, domains, &mut work)
        .and_then(|polygon| find_cyclic_polygon_contact(&polygon, &mut work));
    Ok(match outcome {
        Ok(contact) => PeriodicPathSimplicityReport {
            simple: Some(contact.is_none()),
            contact,
            work_used: work.used,
            refusal: None,
        },
        Err(reason) => refused(Failure::Arithmetic(reason), work.used),
    })
}

fn find_cyclic_polygon_contact(
    polygon: &LiftedPolygon,
    work: &mut Work,
) -> std::result::Result<Option<PeriodicPathContact>, ExactRefusal> {
    use num_traits::ToPrimitive;
    let p = &polygon.points;
    let count = p.len() - 1;
    for (i, edge) in p.windows(2).enumerate() {
        work.charge(1)?;
        if edge[0] == edge[1] {
            return Ok(Some(PeriodicPathContact {
                edges: [i, i],
                turns: [0, 0],
            }));
        }
    }
    let mut limit = [0; 2];
    for (i, period) in polygon.periods.iter().enumerate() {
        if let Some(period) = period {
            let [lo, hi] = &polygon.bounds[i];
            let width = work.op(hi, lo, |a, b| a - b)?;
            limit[i] = work
                .op(&width, period, |a, b| (a / b).floor())?
                .to_integer()
                .to_i64()
                .ok_or(ExactRefusal::ResourceLimit)?;
        }
    }
    for u in 0..=limit[0] {
        let first_v = if u == 0 { 0 } else { -limit[1] };
        for v in first_v..=limit[1] {
            let turns = [u, v];
            let mut shift = [BigRational::default(), BigRational::default()];
            for axis in 0..2 {
                if let Some(period) = &polygon.periods[axis] {
                    shift[axis] = work.op(
                        period,
                        &BigRational::from_integer(turns[axis].into()),
                        |a, b| a * b,
                    )?;
                }
            }
            for i in 0..count {
                let first_j = if turns == [0, 0] { i + 1 } else { 0 };
                for j in first_j..count {
                    let a = &p[i];
                    let b = &p[i + 1];
                    let c = translate_polygon_vertex(&p[j], &shift, work)?;
                    let d = translate_polygon_vertex(&p[j + 1], &shift, work)?;
                    let hit = if turns == [0, 0] && j == i + 1 {
                        on_polygon_segment(a, &c, &d, work)? || on_polygon_segment(&d, a, b, work)?
                    } else if turns == [0, 0] && i == 0 && j + 1 == count {
                        on_polygon_segment(b, &c, &d, work)? || on_polygon_segment(&c, a, b, work)?
                    } else {
                        polygon_edges_contact(a, b, &c, &d, work)?
                    };
                    if hit {
                        return Ok(Some(PeriodicPathContact {
                            edges: [i, j],
                            turns,
                        }));
                    }
                }
            }
        }
    }
    Ok(None)
}

fn translate_polygon_vertex(
    p: &[BigRational; 2],
    shift: &[BigRational; 2],
    work: &mut Work,
) -> std::result::Result<[BigRational; 2], ExactRefusal> {
    Ok([
        work.op(&p[0], &shift[0], |a, b| a + b)?,
        work.op(&p[1], &shift[1], |a, b| a + b)?,
    ])
}

fn polygon_orientation(
    a: &[BigRational; 2],
    b: &[BigRational; 2],
    p: &[BigRational; 2],
    work: &mut Work,
) -> std::result::Result<std::cmp::Ordering, ExactRefusal> {
    let dx = work.op(&b[0], &a[0], |a, b| a - b)?;
    let dy = work.op(&b[1], &a[1], |a, b| a - b)?;
    let x = work.op(&p[0], &a[0], |a, b| a - b)?;
    let y = work.op(&p[1], &a[1], |a, b| a - b)?;
    let left = work.op(&dx, &y, |a, b| a * b)?;
    let right = work.op(&dy, &x, |a, b| a * b)?;
    Ok(left.cmp(&right))
}

fn in_polygon_edge_bounds(
    p: &[BigRational; 2],
    a: &[BigRational; 2],
    b: &[BigRational; 2],
) -> bool {
    (0..2).all(|i| (&a[i]).min(&b[i]) <= &p[i] && &p[i] <= (&a[i]).max(&b[i]))
}

fn on_polygon_segment(
    p: &[BigRational; 2],
    a: &[BigRational; 2],
    b: &[BigRational; 2],
    work: &mut Work,
) -> std::result::Result<bool, ExactRefusal> {
    Ok(in_polygon_edge_bounds(p, a, b) && polygon_orientation(a, b, p, work)?.is_eq())
}

fn polygon_edges_contact(
    a: &[BigRational; 2],
    b: &[BigRational; 2],
    c: &[BigRational; 2],
    d: &[BigRational; 2],
    work: &mut Work,
) -> std::result::Result<bool, ExactRefusal> {
    if (0..2)
        .any(|i| (&a[i]).max(&b[i]) < (&c[i]).min(&d[i]) || (&c[i]).max(&d[i]) < (&a[i]).min(&b[i]))
    {
        return Ok(false);
    }
    let abc = polygon_orientation(a, b, c, work)?;
    let abd = polygon_orientation(a, b, d, work)?;
    let cda = polygon_orientation(c, d, a, work)?;
    let cdb = polygon_orientation(c, d, b, work)?;
    Ok((abc.is_eq() && in_polygon_edge_bounds(c, a, b))
        || (abd.is_eq() && in_polygon_edge_bounds(d, a, b))
        || (cda.is_eq() && in_polygon_edge_bounds(a, c, d))
        || (cdb.is_eq() && in_polygon_edge_bounds(b, c, d))
        || ([abc, abd, cda, cdb].iter().all(|o| !o.is_eq()) && abc != abd && cda != cdb))
}

#[derive(Clone, Debug)]
pub struct PeriodicPathPairReport {
    /// Boundary disjointness, including across all relevant period translates.
    pub disjoint: Option<bool>,
    pub contact: Option<PeriodicPathContact>,
    pub work_used: u64,
    pub refusal: Option<PeriodicPathAreaRefusal>,
}

/// Check two closed shortest-step polygon lifts for boundary contact.
/// Disjoint boundaries can still enclose nested regions. A contact reports
/// one edge from each input and the period translation of the second.
pub fn check_exact_periodic_surface_path_contact(
    source: &Surface,
    first: &[[f64; 2]],
    second: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicPathPairReport> {
    use PeriodicPathAreaRefusal as Failure;

    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact periodic contact work must be in 1..=1000000",
    )?;
    let refused = |reason, work_used| PeriodicPathPairReport {
        disjoint: None,
        contact: None,
        work_used,
        refusal: Some(reason),
    };
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let [a, b] = match prepare_periodic_polygon_pair(source, first, second, &mut work)? {
        Ok(pair) => pair,
        Err(reason) => return Ok(refused(reason, work.used)),
    };
    Ok(match find_periodic_path_pair_contact(&a, &b, &mut work) {
        Ok(contact) => PeriodicPathPairReport {
            disjoint: Some(contact.is_none()),
            contact,
            work_used: work.used,
            refusal: None,
        },
        Err(reason) => refused(Failure::Arithmetic(reason), work.used),
    })
}

fn prepare_periodic_polygon_pair(
    source: &Surface,
    first: &[[f64; 2]],
    second: &[[f64; 2]],
    work: &mut Work,
) -> Result<std::result::Result<[LiftedPolygon; 2], PeriodicPathAreaRefusal>> {
    use PeriodicPathAreaRefusal as Failure;
    crate::check(
        [first, second]
            .iter()
            .all(|path| path.len() >= 4 && path.first() == path.last()),
        "Periodic contact needs two closed polygons",
    )?;

    let mut lift = |path: &[[f64; 2]]| -> Result<std::result::Result<Vec<[f64; 2]>, Failure>> {
        if work.used == work.limit {
            return Ok(Err(Failure::Arithmetic(ExactRefusal::ResourceLimit)));
        }
        let report = lift_exact_periodic_surface_path(source, path, work.limit - work.used)?;
        work.used += report.work_used;
        let Some(points) = report.parameters else {
            return Ok(Err(Failure::Lift(report.refusal.unwrap_or(
                PeriodicLiftRefusal::Arithmetic(ExactRefusal::InvalidOutput),
            ))));
        };
        if points.first() != points.last() {
            return Ok(Err(Failure::NonzeroWinding));
        }
        Ok(Ok(points))
    };
    let first = match lift(first)? {
        Ok(points) => points,
        Err(reason) => return Ok(Err(reason)),
    };
    let second = match lift(second)? {
        Ok(points) => points,
        Err(reason) => return Ok(Err(reason)),
    };
    let domains = [Axis::U, Axis::V].map(|axis| {
        let axis = row(source, axis, 0);
        axis.periodic.then(|| axis.domain())
    });
    let mut prepare = || -> std::result::Result<_, ExactRefusal> {
        let a = prepare_lifted_polygon(&first, domains, work)?;
        let mut b = prepare_lifted_polygon(&second, [None; 2], work)?;
        b.periods.clone_from(&a.periods);
        Ok([a, b])
    };
    Ok(prepare().map_err(Failure::Arithmetic))
}

fn find_periodic_path_pair_contact(
    a: &LiftedPolygon,
    b: &LiftedPolygon,
    work: &mut Work,
) -> std::result::Result<Option<PeriodicPathContact>, ExactRefusal> {
    use num_traits::ToPrimitive;

    let mut turns = [[0_i64; 2]; 2];
    for (axis, limits) in turns.iter_mut().enumerate() {
        let [a_lo, a_hi] = &a.bounds[axis];
        let [b_lo, b_hi] = &b.bounds[axis];
        if let Some(period) = &a.periods[axis] {
            let start = work.op(a_lo, b_hi, |a, b| a - b)?;
            let end = work.op(a_hi, b_lo, |a, b| a - b)?;
            limits[0] = work
                .op(&start, period, |a, b| (a / b).ceil())?
                .to_integer()
                .to_i64()
                .ok_or(ExactRefusal::ResourceLimit)?;
            limits[1] = work
                .op(&end, period, |a, b| (a / b).floor())?
                .to_integer()
                .to_i64()
                .ok_or(ExactRefusal::ResourceLimit)?;
            if limits[0] > limits[1] {
                return Ok(None);
            }
        } else if a_hi < b_lo || b_hi < a_lo {
            return Ok(None);
        }
    }
    for u in turns[0][0]..=turns[0][1] {
        for v in turns[1][0]..=turns[1][1] {
            let offset = [u, v];
            let mut shift = [BigRational::default(), BigRational::default()];
            for axis in 0..2 {
                if let Some(period) = &a.periods[axis] {
                    let turns = BigRational::from_integer(offset[axis].into());
                    shift[axis] = work.op(period, &turns, |a, b| a * b)?;
                }
            }
            let translated = b
                .points
                .iter()
                .map(|p| translate_polygon_vertex(p, &shift, work))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for (i, edge) in a.points.windows(2).enumerate() {
                for (j, other) in translated.windows(2).enumerate() {
                    work.charge(1)?;
                    if polygon_edges_contact(&edge[0], &edge[1], &other[0], &other[1], work)? {
                        return Ok(Some(PeriodicPathContact {
                            edges: [i, j],
                            turns: offset,
                        }));
                    }
                }
            }
        }
    }
    Ok(None)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicPathRelation {
    FirstContainsSecond,
    SecondContainsFirst,
    Separate,
    BoundaryContact(PeriodicPathContact),
    NonSimple {
        contour: usize,
        contact: PeriodicPathContact,
    },
}

#[derive(Clone, Debug)]
pub struct PeriodicPathRelationReport {
    pub relation: Option<PeriodicPathRelation>,
    pub work_used: u64,
    pub refusal: Option<PeriodicPathAreaRefusal>,
}

/// Compare the bounded regions of two simple closed lifts projected onto
/// the periodic parameter chart. Certify simplicity and boundary separation
/// before using one point on each contour to distinguish strict nesting.
pub fn relate_exact_periodic_surface_paths(
    source: &Surface,
    first: &[[f64; 2]],
    second: &[[f64; 2]],
    max_work: u64,
) -> Result<PeriodicPathRelationReport> {
    use PeriodicPathRelation as Relation;

    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact periodic relation work must be in 1..=1000000",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let refused = |reason, work_used| PeriodicPathRelationReport {
        relation: None,
        work_used,
        refusal: Some(reason),
    };
    let [a, b] = match prepare_periodic_polygon_pair(source, first, second, &mut work)? {
        Ok(pair) => pair,
        Err(reason) => return Ok(refused(reason, work.used)),
    };
    let mut compare = || -> std::result::Result<Relation, ExactRefusal> {
        for (contour, polygon) in [&a, &b].into_iter().enumerate() {
            if let Some(contact) = find_cyclic_polygon_contact(polygon, &mut work)? {
                return Ok(Relation::NonSimple { contour, contact });
            }
        }
        if let Some(contact) = find_periodic_path_pair_contact(&a, &b, &mut work)? {
            return Ok(Relation::BoundaryContact(contact));
        }
        if classify_lifted_polygon(&a, second[0], &mut work)? == PeriodicPathLocation::Inside {
            Ok(Relation::FirstContainsSecond)
        } else if classify_lifted_polygon(&b, first[0], &mut work)? == PeriodicPathLocation::Inside
        {
            Ok(Relation::SecondContainsFirst)
        } else {
            Ok(Relation::Separate)
        }
    };
    Ok(match compare() {
        Ok(relation) => PeriodicPathRelationReport {
            relation: Some(relation),
            work_used: work.used,
            refusal: None,
        },
        Err(reason) => refused(PeriodicPathAreaRefusal::Arithmetic(reason), work.used),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicContourNode {
    /// Index of the immediate containing contour, in input order.
    pub parent: Option<usize>,
    pub depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicContourForestRefusal {
    Path {
        contour: usize,
        reason: PeriodicPathAreaRefusal,
    },
    NonSimple {
        contour: usize,
        contact: PeriodicPathContact,
    },
    Contact {
        contours: [usize; 2],
        contact: PeriodicPathContact,
    },
    Arithmetic(ExactRefusal),
}

#[derive(Clone, Debug)]
pub struct PeriodicContourForestReport {
    pub nodes: Option<Vec<PeriodicContourNode>>,
    pub work_used: u64,
    pub refusal: Option<PeriodicContourForestRefusal>,
}

/// Build the containment forest of simple, mutually disjoint closed UV
/// contours on a periodic chart. Even depths bound filled regions; odd
/// depths bound holes under even-odd fill. The result uses input indices.
pub fn exact_periodic_surface_contour_forest(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
) -> Result<PeriodicContourForestReport> {
    Ok(prepare_periodic_contour_forest(source, contours, max_work)?.0)
}

fn prepare_periodic_contour_forest(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
) -> Result<(PeriodicContourForestReport, Vec<LiftedPolygon>)> {
    use PeriodicContourForestRefusal as Failure;

    crate::check(
        (1..=256).contains(&contours.len()),
        "Periodic contour forest needs 1..=256 contours",
    )?;
    crate::check(
        contours
            .iter()
            .all(|path| (4..=4096).contains(&path.len()) && path.first() == path.last()),
        "Periodic contour forest needs closed polygons of 4..=4096 samples",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact contour forest work must be in 1..=1000000",
    )?;
    source.validate()?;
    let domains = [Axis::U, Axis::V].map(|axis| {
        let reference = row(source, axis, 0);
        reference.periodic.then(|| reference.domain())
    });
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let refused = |reason, work_used| {
        (
            PeriodicContourForestReport {
                nodes: None,
                work_used,
                refusal: Some(reason),
            },
            Vec::new(),
        )
    };
    let mut polygons = Vec::with_capacity(contours.len());
    for (contour, path) in contours.iter().enumerate() {
        if work.used == max_work {
            return Ok(refused(
                Failure::Arithmetic(ExactRefusal::ResourceLimit),
                work.used,
            ));
        }
        let lift = lift_exact_periodic_surface_path(source, path, max_work - work.used)?;
        work.used += lift.work_used;
        let Some(points) = lift.parameters else {
            let reason = PeriodicPathAreaRefusal::Lift(
                lift.refusal
                    .unwrap_or(PeriodicLiftRefusal::Arithmetic(ExactRefusal::InvalidOutput)),
            );
            return Ok(refused(Failure::Path { contour, reason }, work.used));
        };
        if points.first() != points.last() {
            return Ok(refused(
                Failure::Path {
                    contour,
                    reason: PeriodicPathAreaRefusal::NonzeroWinding,
                },
                work.used,
            ));
        }
        let polygon = match prepare_lifted_polygon(&points, domains, &mut work) {
            Ok(polygon) => polygon,
            Err(reason) => return Ok(refused(Failure::Arithmetic(reason), work.used)),
        };
        match find_cyclic_polygon_contact(&polygon, &mut work) {
            Ok(Some(contact)) => {
                return Ok(refused(Failure::NonSimple { contour, contact }, work.used));
            }
            Ok(None) => polygons.push(polygon),
            Err(reason) => return Ok(refused(Failure::Arithmetic(reason), work.used)),
        }
    }
    match build_periodic_contour_forest(&polygons, contours, &mut work) {
        Ok(nodes) => Ok((
            PeriodicContourForestReport {
                nodes: Some(nodes),
                work_used: work.used,
                refusal: None,
            },
            polygons,
        )),
        Err(reason) => Ok(refused(reason, work.used)),
    }
}

fn build_periodic_contour_forest(
    polygons: &[LiftedPolygon],
    native: &[&[[f64; 2]]],
    work: &mut Work,
) -> std::result::Result<Vec<PeriodicContourNode>, PeriodicContourForestRefusal> {
    use PeriodicContourForestRefusal as Failure;
    let mut containers = vec![Vec::new(); polygons.len()];
    for (i, a) in polygons.iter().enumerate() {
        for (j, b) in polygons.iter().enumerate().skip(i + 1) {
            if let Some(contact) =
                find_periodic_path_pair_contact(a, b, work).map_err(Failure::Arithmetic)?
            {
                return Err(Failure::Contact {
                    contours: [i, j],
                    contact,
                });
            }
            if classify_lifted_polygon(a, native[j][0], work).map_err(Failure::Arithmetic)?
                == PeriodicPathLocation::Inside
            {
                containers[j].push(i);
            } else if classify_lifted_polygon(b, native[i][0], work).map_err(Failure::Arithmetic)?
                == PeriodicPathLocation::Inside
            {
                containers[i].push(j);
            }
        }
    }
    Ok(containers
        .iter()
        .map(|ancestors| PeriodicContourNode {
            depth: ancestors.len(),
            parent: ancestors
                .iter()
                .copied()
                .max_by_key(|&index| containers[index].len()),
        })
        .collect())
}

#[derive(Clone, Debug)]
pub struct PeriodicRegionAreaReport {
    /// Exact area in squared UV parameter units.
    pub area: Option<BigRational>,
    pub work_used: u64,
    pub refusal: Option<PeriodicContourForestRefusal>,
}

/// Measure the UV area of a periodic region bounded by simple, strictly
/// nested or separate contours. Use even-odd fill independently of input
/// orientation. All topology checks and integrations share one budget.
pub fn exact_periodic_surface_region_area(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
) -> Result<PeriodicRegionAreaReport> {
    use PeriodicContourForestRefusal as Failure;
    use num_traits::{Signed, Zero};

    let forest = exact_periodic_surface_contour_forest(source, contours, max_work)?;
    let mut work_used = forest.work_used;
    let refused = |reason, work_used| PeriodicRegionAreaReport {
        area: None,
        work_used,
        refusal: Some(reason),
    };
    let Some(nodes) = forest.nodes else {
        return Ok(refused(
            forest
                .refusal
                .unwrap_or(Failure::Arithmetic(ExactRefusal::InvalidOutput)),
            work_used,
        ));
    };
    let exhausted = Failure::Arithmetic(ExactRefusal::ResourceLimit);
    let mut area = BigRational::zero();
    for (contour, (path, node)) in contours.iter().zip(nodes).enumerate() {
        if work_used == max_work {
            return Ok(refused(exhausted, work_used));
        }
        let report = exact_periodic_surface_path_signed_area(source, path, max_work - work_used)?;
        work_used += report.work_used;
        let Some(signed) = report.signed_area else {
            let reason = report
                .refusal
                .unwrap_or(PeriodicPathAreaRefusal::Arithmetic(
                    ExactRefusal::InvalidOutput,
                ));
            return Ok(refused(Failure::Path { contour, reason }, work_used));
        };
        let mut work = Work {
            used: work_used,
            limit: max_work,
        };
        let magnitude = signed.abs();
        area = match work.op(&area, &magnitude, |a, b| {
            if node.depth % 2 == 0 { a + b } else { a - b }
        }) {
            Ok(sum) => sum,
            Err(reason) => return Ok(refused(Failure::Arithmetic(reason.into()), work.used)),
        };
        work_used = work.used;
    }
    Ok(PeriodicRegionAreaReport {
        area: Some(area),
        work_used,
        refusal: None,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicRegionComponentCentroid {
    pub outer_contour: usize,
    pub area: BigRational,
    /// Exact UV centroid in the lift anchored at the outer contour's start.
    pub centroid: [BigRational; 2],
}

#[derive(Clone, Debug)]
pub struct PeriodicRegionCentroidReport {
    /// One centroid per filled component, in outer-contour input order.
    pub components: Option<Vec<PeriodicRegionComponentCentroid>>,
    pub work_used: u64,
    pub refusal: Option<PeriodicContourForestRefusal>,
}

/// Compute exact UV centroids of the filled components of a simple
/// periodic contour forest. Move each direct hole into its outer contour's
/// lift before subtracting its area and moments. Islands get their own
/// centroids, since separate components have independent lifts.
pub fn exact_periodic_surface_region_component_centroids(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
) -> Result<PeriodicRegionCentroidReport> {
    let report = measure_periodic_region_components(source, contours, max_work, false)?;
    Ok(PeriodicRegionCentroidReport {
        components: report.components.map(|components| {
            components
                .into_iter()
                .map(|component| component.geometry)
                .collect()
        }),
        work_used: report.work_used,
        refusal: report.refusal,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicRegionComponentInertia {
    pub geometry: PeriodicRegionComponentCentroid,
    /// Integrals of centered U², U·V and V² in the outer contour's lift.
    pub central_moments: [BigRational; 3],
}

#[derive(Clone, Debug)]
pub struct PeriodicRegionInertiaReport {
    pub components: Option<Vec<PeriodicRegionComponentInertia>>,
    pub work_used: u64,
    pub refusal: Option<PeriodicContourForestRefusal>,
}

/// Compute exact UV second moments about each filled component's centroid.
/// Translate the holes to the same lift before subtracting their moments.
pub fn exact_periodic_surface_region_component_inertia(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
) -> Result<PeriodicRegionInertiaReport> {
    measure_periodic_region_components(source, contours, max_work, true)
}

struct ContourMoments {
    area: BigRational,
    center: [BigRational; 2],
    central: Option<[BigRational; 3]>,
}

fn measure_periodic_region_components(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
    inertia: bool,
) -> Result<PeriodicRegionInertiaReport> {
    use PeriodicContourForestRefusal as Failure;
    use num_traits::Signed;

    let forest = exact_periodic_surface_contour_forest(source, contours, max_work)?;
    let refused = |reason, work_used| PeriodicRegionInertiaReport {
        components: None,
        work_used,
        refusal: Some(reason),
    };
    let invalid = Failure::Arithmetic(ExactRefusal::InvalidOutput);
    let exhausted = Failure::Arithmetic(ExactRefusal::ResourceLimit);
    let Some(nodes) = forest.nodes else {
        return Ok(refused(forest.refusal.unwrap_or(invalid), forest.work_used));
    };
    let mut work = Work {
        used: forest.work_used,
        limit: max_work,
    };
    let mut moments = Vec::with_capacity(contours.len());
    for (contour, path) in contours.iter().enumerate() {
        if work.used == max_work {
            return Ok(refused(exhausted, work.used));
        }
        let (report, central) = periodic_path_moments(
            source,
            path,
            max_work - work.used,
            if inertia { 2 } else { 1 },
        )?;
        work.used += report.work_used;
        if let Some(reason) = report.refusal {
            return Ok(refused(Failure::Path { contour, reason }, work.used));
        }
        let (Some(area), Some(center)) = (report.signed_area, report.centroid) else {
            return Ok(refused(invalid, work.used));
        };
        if inertia && central.is_none() {
            return Ok(refused(invalid, work.used));
        }
        moments.push(ContourMoments {
            center,
            central: central.map(|values| {
                if area.is_negative() {
                    values.map(|value| -value)
                } else {
                    values
                }
            }),
            area: area.abs(),
        });
    }
    let domains = [Axis::U, Axis::V].map(|axis| {
        let reference = row(source, axis, 0);
        reference.periodic.then(|| reference.domain())
    });
    let mut components = Vec::new();
    for (outer_contour, _) in nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.depth % 2 == 0)
    {
        if work.used == max_work {
            return Ok(refused(exhausted, work.used));
        }
        let lift = lift_exact_periodic_surface_path(
            source,
            contours[outer_contour],
            max_work - work.used,
        )?;
        work.used += lift.work_used;
        let Some(points) = lift.parameters else {
            let reason = PeriodicPathAreaRefusal::Lift(
                lift.refusal
                    .unwrap_or(PeriodicLiftRefusal::Arithmetic(ExactRefusal::InvalidOutput)),
            );
            return Ok(refused(
                Failure::Path {
                    contour: outer_contour,
                    reason,
                },
                work.used,
            ));
        };
        let result = prepare_lifted_polygon(&points, domains, &mut work).and_then(|polygon| {
            measure_periodic_component(
                &polygon,
                outer_contour,
                &nodes,
                contours,
                &moments,
                &mut work,
            )
        });
        match result {
            Ok(component) => components.push(component),
            Err(reason) => return Ok(refused(Failure::Arithmetic(reason), work.used)),
        }
    }
    Ok(PeriodicRegionInertiaReport {
        components: Some(components),
        work_used: work.used,
        refusal: None,
    })
}

fn measure_periodic_component(
    polygon: &LiftedPolygon,
    outer: usize,
    nodes: &[PeriodicContourNode],
    contours: &[&[[f64; 2]]],
    moments: &[ContourMoments],
    work: &mut Work,
) -> std::result::Result<PeriodicRegionComponentInertia, ExactRefusal> {
    use num_traits::Signed;

    let ContourMoments {
        area,
        center,
        central,
    } = &moments[outer];
    let mut area = area.clone();
    let mut second = central.clone();
    if let Some(raw) = &mut second {
        shift_uv_second_moments(raw, &area, center, work)?;
    }
    let mut weighted = [
        work.op(&area, &center[0], |a, b| a * b)?,
        work.op(&area, &center[1], |a, b| a * b)?,
    ];
    for (hole, _) in nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.parent == Some(outer))
    {
        let (location, turns) =
            classify_lifted_polygon_with_shift(polygon, contours[hole][0], work)?;
        if location != PeriodicPathLocation::Inside {
            return Err(ExactRefusal::InvalidOutput);
        }
        let ContourMoments {
            area: hole_area,
            center,
            central,
        } = &moments[hole];
        let mut aligned = center.clone();
        area = work.op(&area, hole_area, |a, b| a - b)?;
        for i in 0..2 {
            let mut coordinate = center[i].clone();
            if let Some(period) = &polygon.periods[i] {
                let offset = work.op(
                    period,
                    &BigRational::from_integer(turns[i].into()),
                    |a, b| a * b,
                )?;
                coordinate = work.op(&coordinate, &offset, |a, b| a + b)?;
            }
            aligned[i] = coordinate;
            let term = work.op(hole_area, &aligned[i], |a, b| a * b)?;
            weighted[i] = work.op(&weighted[i], &term, |a, b| a - b)?;
        }
        if let Some(raw) = &mut second {
            let mut hole_raw = central.clone().ok_or(ExactRefusal::InvalidOutput)?;
            shift_uv_second_moments(&mut hole_raw, hole_area, &aligned, work)?;
            for (total, value) in raw.iter_mut().zip(hole_raw) {
                *total = work.op(total, &value, |a, b| a - b)?;
            }
        }
    }
    if !area.is_positive() {
        return Err(ExactRefusal::InvalidOutput);
    }
    let centroid = [
        work.op(&weighted[0], &area, |a, b| a / b)?,
        work.op(&weighted[1], &area, |a, b| a / b)?,
    ];
    let central_moments = if let Some(mut raw) = second {
        shift_uv_second_moments(&mut raw, &(-area.clone()), &centroid, work)?;
        raw
    } else {
        std::array::from_fn(|_| BigRational::default())
    };
    Ok(PeriodicRegionComponentInertia {
        geometry: PeriodicRegionComponentCentroid {
            outer_contour: outer,
            area,
            centroid,
        },
        central_moments,
    })
}

fn shift_uv_second_moments(
    moments: &mut [BigRational; 3],
    area: &BigRational,
    centroid: &[BigRational; 2],
    work: &mut Work,
) -> std::result::Result<(), ExactRefusal> {
    for (moment, (i, j)) in moments.iter_mut().zip([(0, 0), (0, 1), (1, 1)]) {
        let product = work.op(&centroid[i], &centroid[j], |a, b| a * b)?;
        let shift = work.op(area, &product, |a, b| a * b)?;
        *moment = work.op(moment, &shift, |a, b| a + b)?;
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiftedPeriodicContour {
    pub contour: usize,
    /// Integer periods added to the contour's own shortest-step lift.
    pub translation: [i64; 2],
    /// Closed rational polygon in the component's chart.
    pub vertices: Vec<[BigRational; 2]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiftedPeriodicComponent {
    pub outer: LiftedPeriodicContour,
    pub holes: Vec<LiftedPeriodicContour>,
}

#[derive(Clone, Debug)]
pub struct PeriodicRegionLiftReport {
    pub components: Option<Vec<LiftedPeriodicComponent>>,
    pub work_used: u64,
    pub refusal: Option<PeriodicContourForestRefusal>,
}

/// Export a rational UV chart for each filled component of a simple
/// periodic contour forest. Put each direct hole in the same lift as
/// its outer polygon. Input edge order and contour indices are preserved.
pub fn lift_exact_periodic_surface_region_components(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
) -> Result<PeriodicRegionLiftReport> {
    use PeriodicContourForestRefusal as Failure;

    let (forest, polygons) = prepare_periodic_contour_forest(source, contours, max_work)?;
    let Some(nodes) = forest.nodes else {
        return Ok(PeriodicRegionLiftReport {
            components: None,
            work_used: forest.work_used,
            refusal: forest.refusal,
        });
    };
    let mut work = Work {
        used: forest.work_used,
        limit: max_work,
    };
    let mut components = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if node.depth % 2 != 0 {
            continue;
        }
        let outer = &polygons[index];
        let mut component = LiftedPeriodicComponent {
            outer: LiftedPeriodicContour {
                contour: index,
                translation: [0; 2],
                vertices: outer.points.clone(),
            },
            holes: Vec::new(),
        };
        for (hole, _) in nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.parent == Some(index))
        {
            match lift_hole_into_component(
                outer,
                &polygons[hole],
                hole,
                contours[hole][0],
                &mut work,
            ) {
                Ok(hole) => component.holes.push(hole),
                Err(reason) => {
                    return Ok(PeriodicRegionLiftReport {
                        components: None,
                        work_used: work.used,
                        refusal: Some(Failure::Arithmetic(reason)),
                    });
                }
            }
        }
        components.push(component);
    }
    Ok(PeriodicRegionLiftReport {
        components: Some(components),
        work_used: work.used,
        refusal: None,
    })
}

fn lift_hole_into_component(
    outer: &LiftedPolygon,
    hole: &LiftedPolygon,
    contour: usize,
    witness: [f64; 2],
    work: &mut Work,
) -> std::result::Result<LiftedPeriodicContour, ExactRefusal> {
    let (location, translation) = classify_lifted_polygon_with_shift(outer, witness, work)?;
    if location != PeriodicPathLocation::Inside {
        return Err(ExactRefusal::InvalidOutput);
    }
    let mut shift = [BigRational::default(), BigRational::default()];
    for i in 0..2 {
        if let Some(period) = &outer.periods[i] {
            shift[i] = work.op(
                period,
                &BigRational::from_integer(translation[i].into()),
                |a, b| a * b,
            )?;
        }
    }
    let vertices = hole
        .points
        .iter()
        .map(|p| translate_polygon_vertex(p, &shift, work))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(LiftedPeriodicContour {
        contour,
        translation,
        vertices,
    })
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimComponentsReport {
    pub components: Option<Vec<PeriodicTrimComponent>>,
    pub work_used: u64,
    pub refusal: Option<PeriodicContourForestRefusal>,
}

/// Turn exact periodic component lifts into degree-one trim loops that
/// can be consumed by the planar UV classifier. Refuse unstorable UVs.
/// Each component retains its own lifted UV chart.
pub fn build_exact_periodic_surface_trim_components(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
) -> Result<PeriodicTrimComponentsReport> {
    build_periodic_trim_components(source, contours, max_work, &mut [])
}

fn build_periodic_trim_components(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
    reversed: &mut [bool],
) -> Result<PeriodicTrimComponentsReport> {
    let lifted = lift_exact_periodic_surface_region_components(source, contours, max_work)?;
    let Some(components) = lifted.components else {
        return Ok(PeriodicTrimComponentsReport {
            components: None,
            work_used: lifted.work_used,
            refusal: lifted.refusal,
        });
    };
    let mut work = Work {
        used: lifted.work_used,
        limit: max_work,
    };
    let result = components
        .iter()
        .map(|component| {
            let contours = std::iter::once(&component.outer).chain(&component.holes);
            let spans: usize = contours.clone().map(|path| path.vertices.len() - 1).sum();
            let curves: usize = contours
                .clone()
                .map(|path| (path.vertices.len() - 1).div_ceil(255))
                .sum();
            if spans > 4096 || curves > 256 {
                return Err(ExactRefusal::ResourceLimit);
            }
            let mut result = PeriodicTrimComponent {
                contours: Vec::new(),
                loops: Vec::new(),
            };
            for (i, contour) in contours.enumerate() {
                result.contours.push(contour.contour);
                let (curves, was_reversed) =
                    store_periodic_trim_loop(&contour.vertices, i == 0, &mut work)?;
                if let Some(flag) = reversed.get_mut(contour.contour) {
                    *flag = was_reversed;
                }
                result.loops.push(curves);
            }
            Ok(result)
        })
        .collect::<std::result::Result<Vec<_>, _>>();
    Ok(match result {
        Ok(components) => PeriodicTrimComponentsReport {
            components: Some(components),
            work_used: work.used,
            refusal: None,
        },
        Err(reason) => PeriodicTrimComponentsReport {
            components: None,
            work_used: work.used,
            refusal: Some(PeriodicContourForestRefusal::Arithmetic(reason)),
        },
    })
}

fn store_periodic_trim_loop(
    vertices: &[[BigRational; 2]],
    ccw: bool,
    work: &mut Work,
) -> std::result::Result<(Vec<crate::curve::Curve>, bool), ExactRefusal> {
    use crate::curve::Curve;
    use num_traits::{Signed, Zero};

    let mut area = BigRational::zero();
    for edge in vertices.windows(2) {
        let a = work.op(&edge[0][0], &edge[1][1], |a, b| a * b)?;
        let b = work.op(&edge[0][1], &edge[1][0], |a, b| a * b)?;
        let cross = work.op(&a, &b, |a, b| a - b)?;
        area = work.op(&area, &cross, |a, b| a + b)?;
    }
    if area.is_zero() {
        return Err(ExactRefusal::InvalidOutput);
    }
    let mut points = vertices
        .iter()
        .map(|[u, v]| Ok(vec![work.float(u)?, work.float(v)?]))
        .collect::<std::result::Result<Vec<_>, ExactRefusal>>()?;
    let reversed = area.is_positive() != ccw;
    if reversed {
        points.reverse();
    }
    let curves = (0..points.len() - 1)
        .step_by(255)
        .map(|start| {
            let end = (start + 256).min(points.len());
            Curve::from_polyline(points[start..end].to_vec())
                .map_err(|_| ExactRefusal::InvalidOutput)
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok((curves, reversed))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicTrimPatchRefusal {
    Contours(PeriodicContourForestRefusal),
    Support {
        component: usize,
        reason: GaugeMergeRefusal,
    },
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimPatchReport {
    pub patches: Option<Vec<PeriodicTrimPatch>>,
    pub work_used: u64,
    pub refusal: Option<PeriodicTrimPatchRefusal>,
}

/// Build a clamped support patch and its oriented trim loops for each
/// periodic UV component. The outer contour's bounding rectangle sets the
/// exact chart. Shift it by whole turns as needed to anchor the extraction
/// in the source domain. Both support and trims use the resulting UV clock.
pub fn build_exact_periodic_surface_trim_patches(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
) -> Result<PeriodicTrimPatchReport> {
    build_periodic_trim_patches(source, contours, max_work, &mut [])
}

fn build_periodic_trim_patches(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
    reversed: &mut [bool],
) -> Result<PeriodicTrimPatchReport> {
    use PeriodicTrimPatchRefusal as Failure;

    let trims = build_periodic_trim_components(source, contours, max_work, reversed)?;
    let refused = |reason, work_used| PeriodicTrimPatchReport {
        patches: None,
        work_used,
        refusal: Some(reason),
    };
    let Some(trims_list) = trims.components else {
        let reason = trims
            .refusal
            .unwrap_or(PeriodicContourForestRefusal::Arithmetic(
                ExactRefusal::InvalidOutput,
            ));
        return Ok(refused(Failure::Contours(reason), trims.work_used));
    };
    let mut work = Work {
        used: trims.work_used,
        limit: max_work,
    };
    let mut patches = Vec::with_capacity(trims_list.len());
    for mut trim in trims_list {
        let component = trim.contours[0];
        let (bounds, uv_translation) = match anchor_trim_patch(source, &mut trim, &mut work) {
            Ok(chart) => chart,
            Err(reason) => {
                return Ok(refused(
                    Failure::Support {
                        component,
                        reason: GaugeMergeRefusal::Exact(reason),
                    },
                    work.used,
                ));
            }
        };
        let mut support = source.clone();
        for (axis, interval) in [Axis::U, Axis::V].into_iter().zip(bounds) {
            if work.used == max_work {
                return Ok(refused(
                    Failure::Support {
                        component,
                        reason: GaugeMergeRefusal::Exact(ExactRefusal::ResourceLimit),
                    },
                    work.used,
                ));
            }
            let report = if row(&support, axis, 0).periodic {
                let mut report = restrict_exact_periodic_surface_over_turns(
                    &support,
                    axis,
                    interval,
                    max_work - work.used,
                )?;
                report.work_used += work.used;
                report
            } else {
                clamp_surface_axis(&support, axis, interval, &mut work)
            };
            work.used = report.work_used;
            let Some(patch) = report.surface else {
                let reason = report
                    .refusal
                    .unwrap_or(GaugeMergeRefusal::Exact(ExactRefusal::InvalidOutput));
                return Ok(refused(Failure::Support { component, reason }, work.used));
            };
            support = patch;
        }
        patches.push(PeriodicTrimPatch {
            surface: support,
            trim,
            uv_translation,
        });
    }
    Ok(PeriodicTrimPatchReport {
        patches: Some(patches),
        work_used: work.used,
        refusal: None,
    })
}

fn anchor_trim_patch(
    source: &Surface,
    trim: &mut PeriodicTrimComponent,
    work: &mut Work,
) -> std::result::Result<([[f64; 2]; 2], [BigRational; 2]), ExactRefusal> {
    use num_traits::Zero;

    let mut bounds = [[0.; 2]; 2];
    let mut translation = std::array::from_fn(|_| BigRational::zero());
    for (i, axis) in [Axis::U, Axis::V].into_iter().enumerate() {
        let coordinates = || {
            trim.loops[0]
                .iter()
                .flat_map(|curve| &curve.control_points)
                .map(|point| point[i])
        };
        let min = coordinates().fold(f64::INFINITY, f64::min);
        let max = coordinates().fold(f64::NEG_INFINITY, f64::max);
        let reference = row(source, axis, 0);
        let [lo, hi] = reference.domain();
        if reference.periodic && !(lo <= min && min < hi) {
            let start = work.lift(lo)?;
            let end = work.lift(hi)?;
            let period = work.op(&end, &start, |a, b| a - b)?;
            let bound = work.lift(min)?;
            let delta = work.op(&bound, &start, |a, b| a - b)?;
            let turns = work.op(&delta, &period, |a, b| (a / b).floor())?;
            translation[i] = work.op(&turns, &period, |a, b| -a * b)?;
        }
        if translation[i].is_zero() {
            bounds[i] = [min, max];
            continue;
        }
        let shift = |x, work: &mut Work| -> std::result::Result<_, ExactRefusal> {
            let x = work.lift(x)?;
            let shifted = work.op(&x, &translation[i], |a, b| a + b)?;
            Ok(work.float(&shifted)?)
        };
        bounds[i] = [shift(min, work)?, shift(max, work)?];
        for point in trim
            .loops
            .iter_mut()
            .flatten()
            .flat_map(|curve| &mut curve.control_points)
        {
            point[i] = shift(point[i], work)?;
        }
    }
    for curve in trim.loops.iter().flatten() {
        curve.validate().map_err(|_| ExactRefusal::InvalidOutput)?;
    }
    Ok((bounds, translation))
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimEdge {
    pub pcurve: crate::curve::Curve,
    pub curve: crate::curve::Curve,
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimPatchEdges {
    pub patch: PeriodicTrimPatch,
    /// Ordered UV/3D edge pairs for the outer loop followed by its holes.
    pub loops: Vec<Vec<PeriodicTrimEdge>>,
}

/// Location of one lifted edge on its source polygon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PeriodicTrimEdgeSource {
    /// Original input contour index.
    pub contour: usize,
    /// Segment index after orienting the polygon for its outer or hole role.
    pub segment: usize,
    /// Parameter interval within that oriented segment, from zero to one.
    pub interval: [f64; 2],
}

impl PeriodicTrimPatchEdges {
    /// Map each lifted edge to its source contour and oriented polygon interval.
    /// The outer/hole grouping and edge order match `self.loops`.
    pub fn edge_sources(&self) -> Result<Vec<Vec<PeriodicTrimEdgeSource>>> {
        let trim = &self.patch.trim;
        let error = || crate::input("Periodic trim edge domains do not partition the source");
        if self.loops.len() != trim.loops.len() || self.loops.len() != trim.contours.len() {
            return Err(error());
        }
        self.loops
            .iter()
            .zip(&trim.loops)
            .zip(&trim.contours)
            .map(|((edges, polylines), &contour)| {
                let mut edges = edges.iter();
                let mut segment_offset = 0;
                let mut sources = Vec::new();
                for polyline in polylines {
                    polyline.validate()?;
                    let segment_count = polyline.control_points.len() - 1;
                    let [start, end] = polyline.domain();
                    if polyline.degree != 1 || start != 0. || end != segment_count as f64 {
                        return Err(error());
                    }
                    let mut next = start;
                    while next < end {
                        let edge = edges.next().ok_or_else(error)?;
                        edge.pcurve.validate()?;
                        edge.curve.validate()?;
                        let [lo, hi] = edge.pcurve.domain();
                        let segment = next.floor();
                        if lo != next
                            || hi <= lo
                            || hi > segment + 1.
                            || edge.curve.domain() != [lo, hi]
                        {
                            return Err(error());
                        }
                        sources.push(PeriodicTrimEdgeSource {
                            contour,
                            segment: segment_offset + segment as usize,
                            interval: [lo - segment, hi - segment],
                        });
                        next = hi;
                    }
                    segment_offset += segment_count;
                }
                if edges.next().is_some() {
                    return Err(error());
                }
                Ok(sources)
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicTrimEdgeRefusal {
    Patch(PeriodicTrimPatchRefusal),
    Lift(super::exact_projective::PcurvePullbackRefusal),
    Restrict(ExactRefusal),
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimEdgesReport {
    pub patches: Option<Vec<PeriodicTrimPatchEdges>>,
    pub work_used: u64,
    /// Patch, loop and original pcurve indices, before cell subdivision.
    pub refused_curve: Option<[usize; 3]>,
    pub refusal: Option<PeriodicTrimEdgeRefusal>,
}

/// Construct exact 3D edges and matching UV restrictions of periodic trim
/// patches. Edges split at chart-cell crossings and retain their source
/// pcurve parameters. A refusal withholds all patches and edge pairs.
pub fn build_exact_periodic_surface_trim_edges(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
) -> Result<PeriodicTrimEdgesReport> {
    build_periodic_trim_edges(source, contours, max_work, &mut [])
}

fn build_periodic_trim_edges(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
    reversed: &mut [bool],
) -> Result<PeriodicTrimEdgesReport> {
    use PeriodicTrimEdgeRefusal as Failure;

    let report = build_periodic_trim_patches(source, contours, max_work, reversed)?;
    let mut work_used = report.work_used;
    let refused = |refusal, work_used, refused_curve| PeriodicTrimEdgesReport {
        patches: None,
        work_used,
        refused_curve,
        refusal: Some(refusal),
    };
    let Some(patches) = report.patches else {
        let reason = report.refusal.unwrap_or(PeriodicTrimPatchRefusal::Contours(
            PeriodicContourForestRefusal::Arithmetic(ExactRefusal::InvalidOutput),
        ));
        return Ok(refused(Failure::Patch(reason), work_used, None));
    };
    let exhausted = Failure::Restrict(ExactRefusal::ResourceLimit);
    let mut output = Vec::with_capacity(patches.len());
    for (i, patch) in patches.into_iter().enumerate() {
        let mut loops = Vec::with_capacity(patch.trim.loops.len());
        for (j, path) in patch.trim.loops.iter().enumerate() {
            let mut edges = Vec::new();
            for (k, pcurve) in path.iter().enumerate() {
                let index = Some([i, j, k]);
                if work_used == max_work {
                    return Ok(refused(exhausted, work_used, index));
                }
                let lift = super::exact_projective::lift_pcurve_to_surface(
                    &patch.surface,
                    pcurve,
                    max_work - work_used,
                )?;
                work_used += lift.work_used;
                let Some(pieces) = lift.pieces else {
                    let reason = lift
                        .refusal
                        .map(Failure::Lift)
                        .unwrap_or(Failure::Restrict(ExactRefusal::InvalidOutput));
                    return Ok(refused(reason, work_used, index));
                };
                for curve in pieces {
                    if work_used == max_work {
                        return Ok(refused(exhausted, work_used, index));
                    }
                    let restricted = super::exact_knot_insertion::restrict(
                        pcurve,
                        curve.domain(),
                        max_work - work_used,
                    )?;
                    work_used += restricted.work_used;
                    let Some(pcurve) = restricted.curve else {
                        return Ok(refused(
                            Failure::Restrict(
                                restricted.refusal.unwrap_or(ExactRefusal::InvalidOutput),
                            ),
                            work_used,
                            index,
                        ));
                    };
                    edges.push(PeriodicTrimEdge { pcurve, curve });
                }
            }
            loops.push(edges);
        }
        output.push(PeriodicTrimPatchEdges { patch, loops });
    }
    Ok(PeriodicTrimEdgesReport {
        patches: Some(output),
        work_used,
        refused_curve: None,
        refusal: None,
    })
}

/// Directed parameters on a segment in the original input sample order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicTrimInputEdgeSource {
    pub contour: usize,
    pub segment: usize,
    /// Parameters at the generated edge start and end. Descends for a reversed contour.
    pub parameters: [BigRational; 2],
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimInputEdges {
    pub geometry: PeriodicTrimEdgesReport,
    /// Same patch, loop and edge order as the geometry. Empty on refusal.
    pub sources: Vec<Vec<Vec<PeriodicTrimInputEdgeSource>>>,
}

/// Build periodic trim edges and retain their parameters in the caller's
/// contour order, including contours reoriented to bound holes or outer wires.
pub fn build_exact_periodic_surface_trim_input_edges(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    max_work: u64,
) -> Result<PeriodicTrimInputEdges> {
    use num_traits::One;

    let mut reversed = vec![false; contours.len().min(256)];
    let geometry = build_periodic_trim_edges(source, contours, max_work, &mut reversed)?;
    let mut sources = Vec::new();
    for patch in geometry.patches.iter().flatten() {
        let mut loops = Vec::new();
        for path in patch.edge_sources()? {
            let mut edges = Vec::with_capacity(path.len());
            for edge in path {
                let mut segment = edge.segment;
                let [lo, hi] = edge.interval.map(BigRational::from_float);
                let (Some(lo), Some(hi)) = (lo, hi) else {
                    return Err(crate::input("Nonfinite periodic trim source interval"));
                };
                let mut parameters = [lo, hi];
                if reversed[edge.contour] {
                    segment = contours[edge.contour].len() - 2 - segment;
                    parameters = parameters.map(|t| BigRational::one() - t);
                }
                edges.push(PeriodicTrimInputEdgeSource {
                    contour: edge.contour,
                    segment,
                    parameters,
                });
            }
            loops.push(edges);
        }
        sources.push(loops);
    }
    Ok(PeriodicTrimInputEdges { geometry, sources })
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimInputPoint {
    /// Exact world coordinates. Absent for an unmatched query or refusal.
    pub point: Option<[BigRational; 3]>,
    pub work_used: u64,
    pub refusal: Option<ExactRefusal>,
}

/// Exact Cartesian derivatives in increasing original segment parameter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PeriodicTrimInputTangents {
    pub before: Option<[BigRational; 3]>,
    pub after: Option<[BigRational; 3]>,
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimTangentReport {
    pub tangents: Option<PeriodicTrimInputTangents>,
    pub work_used: u64,
    pub refusal: Option<ExactRefusal>,
}

/// Exact first-order classification in the original contour parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicTrimTangentJoin {
    OneSided,
    Stationary,
    C1,
    G1,
    Reversal,
    Corner,
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimTangentJoinReport {
    pub join: Option<PeriodicTrimTangentJoin>,
    pub work_used: u64,
    pub refusal: Option<ExactRefusal>,
}

/// A join in original contour order. Parameter zero denotes an original vertex.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicTrimContourJoin {
    pub segment: usize,
    pub parameter: BigRational,
    pub join: PeriodicTrimTangentJoin,
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimContourJoinReport {
    pub joins: Option<Vec<PeriodicTrimContourJoin>>,
    pub work_used: u64,
    pub refusal: Option<ExactRefusal>,
}

#[derive(Default)]
struct PeriodicTrimInputSample {
    point: Option<[BigRational; 3]>,
    tangents: PeriodicTrimInputTangents,
}

impl PeriodicTrimInputEdges {
    /// Evaluate an original contour segment at a rational parameter, and
    /// require the exact 3D values of all incident pieces to agree.
    pub fn evaluate_input_point(
        &self,
        contour: usize,
        segment: usize,
        parameter: &BigRational,
        max_work: u64,
    ) -> Result<PeriodicTrimInputPoint> {
        crate::check(
            (1..=cad_predicates::MAX_WORK).contains(&max_work),
            "Exact periodic trim point work must be in 1..=1000000",
        )?;
        let mut work = Work {
            used: 0,
            limit: max_work,
        };
        let result = self.evaluate_input_sample(contour, segment, parameter, false, &mut work);
        let (point, refusal) = match result {
            Ok(sample) => (sample.point, None),
            Err(reason) => (None, Some(reason)),
        };
        Ok(PeriodicTrimInputPoint {
            point,
            work_used: work.used,
            refusal,
        })
    }

    /// Evaluate both one-sided tangents, retaining geometric creases at splits.
    /// At an original segment endpoint only the side within that segment is present.
    pub fn evaluate_input_tangents(
        &self,
        contour: usize,
        segment: usize,
        parameter: &BigRational,
        max_work: u64,
    ) -> Result<PeriodicTrimTangentReport> {
        crate::check(
            (1..=cad_predicates::MAX_WORK).contains(&max_work),
            "Exact periodic trim tangent work must be in 1..=1000000",
        )?;
        let mut work = Work {
            used: 0,
            limit: max_work,
        };
        let result = self.evaluate_input_sample(contour, segment, parameter, true, &mut work);
        let (tangents, refusal) = match result {
            Ok(sample) => (sample.point.map(|_| sample.tangents), None),
            Err(reason) => (None, Some(reason)),
        };
        Ok(PeriodicTrimTangentReport {
            tangents,
            work_used: work.used,
            refusal,
        })
    }

    /// Classify the exact first-order join at a source segment parameter.
    /// A missing sample or a refusal leaves the classification absent.
    pub fn classify_input_tangent_join(
        &self,
        contour: usize,
        segment: usize,
        parameter: &BigRational,
        max_work: u64,
    ) -> Result<PeriodicTrimTangentJoinReport> {
        crate::check(
            (1..=cad_predicates::MAX_WORK).contains(&max_work),
            "Exact periodic trim tangent join work must be in 1..=1000000",
        )?;
        let mut work = Work {
            used: 0,
            limit: max_work,
        };
        let result = self
            .evaluate_input_sample(contour, segment, parameter, true, &mut work)
            .and_then(|sample| {
                sample
                    .point
                    .map(|_| classify_trim_tangent_pair(&sample.tangents, &mut work))
                    .transpose()
            });
        let (join, refusal) = match result {
            Ok(join) => (join, None),
            Err(reason) => (None, Some(reason)),
        };
        Ok(PeriodicTrimTangentJoinReport {
            join,
            work_used: work.used,
            refusal,
        })
    }

    /// Classify the join at an original polygon vertex. Vertex zero joins
    /// the last segment to the first, as the input polygon is closed.
    pub fn classify_input_vertex_join(
        &self,
        contour: usize,
        vertex: usize,
        max_work: u64,
    ) -> Result<PeriodicTrimTangentJoinReport> {
        crate::check(
            (1..=cad_predicates::MAX_WORK).contains(&max_work),
            "Exact periodic trim vertex join work must be in 1..=1000000",
        )?;
        let mut work = Work {
            used: 0,
            limit: max_work,
        };
        let result = self.classify_input_vertex_with_work(contour, vertex, &mut work);
        let (join, refusal) = match result {
            Ok(join) => (join, None),
            Err(reason) => (None, Some(reason)),
        };
        Ok(PeriodicTrimTangentJoinReport {
            join,
            work_used: work.used,
            refusal,
        })
    }

    fn classify_input_vertex_with_work(
        &self,
        contour: usize,
        vertex: usize,
        work: &mut Work,
    ) -> std::result::Result<Option<PeriodicTrimTangentJoin>, ExactRefusal> {
        use num_traits::{One, Zero};

        let outgoing =
            self.evaluate_input_sample(contour, vertex, &BigRational::zero(), true, work)?;
        let Some(point) = outgoing.point else {
            return Ok(None);
        };
        let previous = if vertex > 0 {
            vertex - 1
        } else {
            let mut last = 0;
            for edge in self.sources.iter().flatten().flatten() {
                work.charge(1)?;
                if edge.contour == contour {
                    last = last.max(edge.segment);
                }
            }
            last
        };
        let incoming =
            self.evaluate_input_sample(contour, previous, &BigRational::one(), true, work)?;
        let (Some(before), Some(after)) = (incoming.tangents.before, outgoing.tangents.after)
        else {
            return Err(ExactRefusal::InvalidOutput);
        };
        if incoming.point.as_ref() != Some(&point) {
            return Err(ExactRefusal::InvalidOutput);
        }
        classify_trim_tangent_pair(
            &PeriodicTrimInputTangents {
                before: Some(before),
                after: Some(after),
            },
            work,
        )
        .map(Some)
    }

    /// Audit every original vertex and every generated split of one closed contour.
    /// The complete contour must partition its original segment parameters.
    pub fn audit_input_contour_joins(
        &self,
        contour: usize,
        max_work: u64,
    ) -> Result<PeriodicTrimContourJoinReport> {
        crate::check(
            (1..=cad_predicates::MAX_WORK).contains(&max_work),
            "Exact periodic trim contour join work must be in 1..=1000000",
        )?;
        let mut work = Work {
            used: 0,
            limit: max_work,
        };
        let result = self.audit_input_contour_with_work(contour, &mut work);
        let (joins, refusal) = match result {
            Ok(joins) => (joins, None),
            Err(reason) => (None, Some(reason)),
        };
        Ok(PeriodicTrimContourJoinReport {
            joins,
            work_used: work.used,
            refusal,
        })
    }

    fn audit_input_contour_with_work(
        &self,
        contour: usize,
        work: &mut Work,
    ) -> std::result::Result<Option<Vec<PeriodicTrimContourJoin>>, ExactRefusal> {
        use num_traits::{One, Zero};

        let Some(patches) = &self.geometry.patches else {
            return Ok(None);
        };
        if patches.len() != self.sources.len() {
            return Err(ExactRefusal::InvalidOutput);
        }
        let mut ordered = std::collections::BTreeMap::new();
        for (patch, sources) in patches.iter().zip(&self.sources) {
            if patch.loops.len() != sources.len() {
                return Err(ExactRefusal::InvalidOutput);
            }
            for (edges, sources) in patch.loops.iter().zip(sources) {
                if edges.len() != sources.len() {
                    return Err(ExactRefusal::InvalidOutput);
                }
                for (edge, source) in edges.iter().zip(sources) {
                    work.charge(1)?;
                    if source.contour != contour {
                        continue;
                    }
                    let [a, b] = &source.parameters;
                    if ordered
                        .insert((source.segment, a.min(b)), (edge, source))
                        .is_some()
                    {
                        return Err(ExactRefusal::InvalidOutput);
                    }
                }
            }
        }
        if ordered.is_empty() {
            return Ok(None);
        }
        let ordered: Vec<_> = ordered.into_values().collect();
        let mut joins = Vec::with_capacity(ordered.len());
        for (i, ((incoming, left), (outgoing, right))) in ordered
            .iter()
            .cycle()
            .skip(ordered.len() - 1)
            .zip(&ordered)
            .enumerate()
        {
            let [a, b] = &left.parameters;
            let [c, d] = &right.parameters;
            let end = a.max(b);
            let start = c.min(d);
            let valid = if i == 0 {
                right.segment == 0 && start.is_zero() && end.is_one()
            } else if left.segment == right.segment {
                end == start
            } else {
                left.segment.checked_add(1) == Some(right.segment)
                    && end.is_one()
                    && start.is_zero()
            };
            if !valid || a == b || c == d {
                return Err(ExactRefusal::InvalidOutput);
            }

            let left_width = work.op(b, a, |x, y| x - y)?;
            let right_width = work.op(d, c, |x, y| x - y)?;
            let end_fraction = if a < b {
                BigRational::one()
            } else {
                BigRational::zero()
            };
            let start_fraction = if c < d {
                BigRational::zero()
            } else {
                BigRational::one()
            };
            let (left_point, before) =
                evaluate_trim_bezier(&incoming.curve, &end_fraction, Some(&left_width), work)?;
            let (right_point, after) =
                evaluate_trim_bezier(&outgoing.curve, &start_fraction, Some(&right_width), work)?;
            if left_point != right_point || before.is_none() || after.is_none() {
                return Err(ExactRefusal::InvalidOutput);
            }
            joins.push(PeriodicTrimContourJoin {
                segment: right.segment,
                parameter: start.clone(),
                join: classify_trim_tangent_pair(
                    &PeriodicTrimInputTangents { before, after },
                    work,
                )?,
            });
        }
        Ok(Some(joins))
    }

    fn evaluate_input_sample(
        &self,
        contour: usize,
        segment: usize,
        parameter: &BigRational,
        with_tangent: bool,
        work: &mut Work,
    ) -> std::result::Result<PeriodicTrimInputSample, ExactRefusal> {
        let Some(patches) = &self.geometry.patches else {
            return Ok(PeriodicTrimInputSample::default());
        };
        if patches.len() != self.sources.len() {
            return Err(ExactRefusal::InvalidOutput);
        }
        let mut sample = PeriodicTrimInputSample::default();
        for (patch, sources) in patches.iter().zip(&self.sources) {
            if patch.loops.len() != sources.len() {
                return Err(ExactRefusal::InvalidOutput);
            }
            for (edges, sources) in patch.loops.iter().zip(sources) {
                if edges.len() != sources.len() {
                    return Err(ExactRefusal::InvalidOutput);
                }
                for (edge, source) in edges.iter().zip(sources) {
                    work.charge(1)?;
                    if source.contour != contour || source.segment != segment {
                        continue;
                    }
                    let [start, end] = &source.parameters;
                    if parameter < start.min(end) || parameter > start.max(end) {
                        continue;
                    }
                    if start == end {
                        return Err(ExactRefusal::InvalidOutput);
                    }
                    let offset = work.op(parameter, start, |a, b| a - b)?;
                    let width = work.op(end, start, |a, b| a - b)?;
                    let fraction = work.op(&offset, &width, |a, b| a / b)?;
                    let (candidate, tangent) = evaluate_trim_bezier(
                        &edge.curve,
                        &fraction,
                        with_tangent.then_some(&width),
                        work,
                    )?;
                    if sample
                        .point
                        .as_ref()
                        .is_some_and(|previous| previous != &candidate)
                    {
                        return Err(ExactRefusal::InvalidOutput);
                    }
                    sample.point = Some(candidate);
                    if let Some(tangent) = tangent {
                        for (include, side) in [
                            (parameter > start.min(end), &mut sample.tangents.before),
                            (parameter < start.max(end), &mut sample.tangents.after),
                        ] {
                            if include {
                                if side.as_ref().is_some_and(|previous| previous != &tangent) {
                                    return Err(ExactRefusal::InvalidOutput);
                                }
                                *side = Some(tangent.clone());
                            }
                        }
                    }
                }
            }
        }
        Ok(sample)
    }
}

fn classify_trim_tangent_pair(
    tangents: &PeriodicTrimInputTangents,
    work: &mut Work,
) -> std::result::Result<PeriodicTrimTangentJoin, ExactRefusal> {
    use PeriodicTrimTangentJoin::*;
    use num_traits::{Signed, Zero};

    let (Some(before), Some(after)) = (&tangents.before, &tangents.after) else {
        return Ok(OneSided);
    };
    let Some(axis) = before.iter().position(|component| !component.is_zero()) else {
        return Ok(Stationary);
    };
    if after.iter().all(Zero::is_zero) {
        return Ok(Stationary);
    }
    if before == after {
        return Ok(C1);
    }
    for (a, b) in before.iter().zip(after) {
        let left = work.op(a, &after[axis], |x, y| x * y)?;
        let right = work.op(b, &before[axis], |x, y| x * y)?;
        if left != right {
            return Ok(Corner);
        }
    }
    if before[axis].is_positive() == after[axis].is_positive() {
        Ok(G1)
    } else {
        Ok(Reversal)
    }
}

type TrimBezierSample = ([BigRational; 3], Option<[BigRational; 3]>);

fn evaluate_trim_bezier(
    curve: &Curve,
    fraction: &BigRational,
    input_width: Option<&BigRational>,
    work: &mut Work,
) -> std::result::Result<TrimBezierSample, ExactRefusal> {
    use num_traits::{One, Zero};

    curve.validate().map_err(|_| ExactRefusal::InvalidOutput)?;
    let count = curve.control_points.len();
    if curve.periodic || count != curve.degree + 1 || curve.control_points[0].len() != 3 {
        return Err(ExactRefusal::InvalidOutput);
    }
    let [start, end] = curve.domain();
    if curve.knots[..count].iter().any(|&t| t != start)
        || curve.knots[count..].iter().any(|&t| t != end)
    {
        return Err(ExactRefusal::InvalidOutput);
    }
    let complement = work.op(&BigRational::one(), fraction, |a, b| a - b)?;
    let mut net = Vec::with_capacity(count);
    for (point, &weight) in curve.control_points.iter().zip(&curve.weights) {
        let weight = work.lift(weight)?;
        let mut pole = [
            BigRational::zero(),
            BigRational::zero(),
            BigRational::zero(),
            BigRational::zero(),
        ];
        for (coordinate, &value) in pole.iter_mut().zip(point) {
            let value = work.lift(value)?;
            *coordinate = work.op(&value, &weight, |a, b| a * b)?;
        }
        pole[3] = weight;
        net.push(pole);
    }
    let mut derivative = None;
    for width in (1..count).rev() {
        if width == 1 && input_width.is_some() {
            let degree = BigRational::from_integer(curve.degree.into());
            let mut values = net[0].clone();
            for (axis, value) in values.iter_mut().enumerate() {
                let delta = work.op(&net[1][axis], &net[0][axis], |a, b| a - b)?;
                *value = work.op(&degree, &delta, |a, b| a * b)?;
            }
            derivative = Some(values);
        }
        for i in 0..width {
            for axis in 0..4 {
                let left = work.op(&net[i][axis], &complement, |a, b| a * b)?;
                let right = work.op(&net[i + 1][axis], fraction, |a, b| a * b)?;
                net[i][axis] = work.op(&left, &right, |a, b| a + b)?;
            }
        }
    }
    let [x, y, z, w] = net.swap_remove(0);
    if w.is_zero() {
        return Err(ExactRefusal::InvalidOutput);
    }
    let point = [
        work.op(&x, &w, |a, b| a / b)?,
        work.op(&y, &w, |a, b| a / b)?,
        work.op(&z, &w, |a, b| a / b)?,
    ];
    let tangent = if let (Some([dx, dy, dz, dw]), Some(width)) = (derivative, input_width) {
        let divisor = work.op(&w, width, |a, b| a * b)?;
        let mut tangent = [dx, dy, dz];
        for (value, point) in tangent.iter_mut().zip(&point) {
            let weight_term = work.op(point, &dw, |a, b| a * b)?;
            let numerator = work.op(value, &weight_term, |a, b| a - b)?;
            *value = work.op(&numerator, &divisor, |a, b| a / b)?;
        }
        Some(tangent)
    } else {
        None
    };
    Ok((point, tangent))
}

/// Certified length enclosure for a complete source contour. An absent
/// enclosure means the contour was unavailable or its minimum cell budget
/// could not be met.
#[derive(Clone, Debug)]
pub struct PeriodicTrimContourLength {
    pub length: Option<crate::analysis::curve_measure::LengthReport>,
    pub minimum_cells: usize,
}

impl PeriodicTrimInputEdges {
    /// Enclose the world-space arc length of every piece of a source contour.
    /// One cell budget covers the entire contour, including all seam pieces.
    pub fn measure_input_contour_length(
        &self,
        contour: usize,
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimContourLength> {
        let mut curves = Vec::new();
        for patch in self.geometry.patches.iter().flatten() {
            crate::check(
                patch.patch.trim.contours.len() == patch.loops.len(),
                "Periodic trim length contour and loop counts differ",
            )?;
            for (&source, path) in patch.patch.trim.contours.iter().zip(&patch.loops) {
                if source == contour {
                    curves.extend(path.iter().map(|edge| (&edge.curve, None)));
                }
            }
        }
        measure_periodic_trim_curves(curves, tolerance, max_cells)
    }

    /// Enclose a single original segment, summing every generated piece once.
    /// The source bindings must cover the entire segment without gaps or overlaps.
    pub fn measure_input_segment_length(
        &self,
        contour: usize,
        segment: usize,
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimContourLength> {
        measure_periodic_trim_curves(
            self.input_segment_pieces(contour, segment)?
                .into_iter()
                .map(|(curve, _)| (curve, None))
                .collect(),
            tolerance,
            max_cells,
        )
    }

    /// Enclose the length between two normalized input segment parameters.
    /// Either direction is accepted. A cut must map to an exact binary64 edge clock.
    pub fn measure_input_segment_range_length(
        &self,
        contour: usize,
        segment: usize,
        range: [f64; 2],
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimContourLength> {
        use num_traits::ToPrimitive;
        let [a, b] = range;
        crate::check(
            range.iter().all(|t| (0. ..=1.).contains(t)) && a != b,
            "Periodic trim length range needs distinct parameters in [0,1]",
        )?;
        let low = BigRational::from_float(a.min(b)).unwrap();
        let high = BigRational::from_float(a.max(b)).unwrap();
        let mut curves = Vec::new();
        for (curve, source) in self.input_segment_pieces(contour, segment)? {
            let [from, to] = &source.parameters;
            let lo = (&low).max(from.min(to));
            let hi = (&high).min(from.max(to));
            if lo >= hi {
                continue;
            }
            let clocks = map_trim_input_interval(curve, source, [lo, hi])?;
            let store = |clock: &BigRational| {
                clock
                    .to_f64()
                    .filter(|stored| BigRational::from_float(*stored).as_ref() == Some(clock))
                    .ok_or_else(|| {
                        crate::input("Periodic trim length cut is not exactly representable")
                    })
            };
            let x = store(&clocks[0])?;
            let y = store(&clocks[1])?;
            curves.push((curve, Some([x.min(y), x.max(y)])));
        }
        measure_periodic_trim_curves(curves, tolerance, max_cells)
    }

    /// Enclose the length of a rational input range using outward and inward
    /// edge intervals. The shared cell budget includes both integrations.
    pub fn measure_rational_input_segment_range_length(
        &self,
        contour: usize,
        segment: usize,
        range: [&BigRational; 2],
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimContourLength> {
        use num_traits::{One, Zero};

        let [a, b] = range;
        let (low, high) = (a.min(b), a.max(b));
        crate::check(
            low >= &BigRational::zero() && high <= &BigRational::one() && low < high,
            "Periodic trim length range needs distinct parameters in [0,1]",
        )?;
        crate::check(
            range
                .iter()
                .all(|t| t.numer().bits() <= 4096 && t.denom().bits() <= 4096),
            "Periodic trim length parameter exceeds the rational size limit",
        )?;
        let mut jobs = Vec::new();
        for (curve, source) in self.input_segment_pieces(contour, segment)? {
            let [from, to] = &source.parameters;
            let lo = low.max(from.min(to));
            let hi = high.min(from.max(to));
            if lo >= hi {
                continue;
            }
            let [x, y] = map_trim_input_interval(curve, source, [lo, hi])?;
            let [a, b] = trim_clock_bracket((&x).min(&y))?;
            let [c, d] = trim_clock_bracket((&x).max(&y))?;
            if a == b && c == d {
                jobs.push((curve, Some([a, d]), LengthBoundSide::Both));
            } else {
                jobs.push((curve, Some([a, d]), LengthBoundSide::Upper));
                if b < c {
                    jobs.push((curve, Some([b, c]), LengthBoundSide::Lower));
                }
            }
        }
        measure_periodic_trim_length_jobs(jobs, tolerance, max_cells)
    }

    fn input_segment_pieces(
        &self,
        contour: usize,
        segment: usize,
    ) -> Result<Vec<TrimInputPiece<'_>>> {
        let mut pieces = self.input_contour_pieces(contour)?;
        pieces.retain(|(_, source)| source.segment == segment);
        verify_trim_segment_partition(&mut pieces)?;
        Ok(pieces)
    }

    fn input_contour_pieces(&self, contour: usize) -> Result<Vec<TrimInputPiece<'_>>> {
        let mut pieces = Vec::new();
        if let Some(patches) = &self.geometry.patches {
            crate::check(
                patches.len() == self.sources.len(),
                "Periodic trim segment patch count differs",
            )?;
            for (patch, sources) in patches.iter().zip(&self.sources) {
                crate::check(
                    patch.loops.len() == sources.len(),
                    "Periodic trim segment loop count differs",
                )?;
                for (edges, sources) in patch.loops.iter().zip(sources) {
                    crate::check(
                        edges.len() == sources.len(),
                        "Periodic trim segment edge count differs",
                    )?;
                    pieces.extend(
                        edges
                            .iter()
                            .zip(sources)
                            .filter(|(_, source)| source.contour == contour)
                            .map(|(edge, source)| (&edge.curve, source)),
                    );
                }
            }
        }
        Ok(pieces)
    }
}

type TrimInputPiece<'a> = (&'a Curve, &'a PeriodicTrimInputEdgeSource);

fn verify_trim_segment_partition(pieces: &mut [TrimInputPiece<'_>]) -> Result<()> {
    use num_traits::{One, Zero};

    pieces.sort_by(|(_, a), (_, b)| {
        let [a0, a1] = &a.parameters;
        let [b0, b1] = &b.parameters;
        a0.min(a1).cmp(b0.min(b1))
    });
    let zero = BigRational::zero();
    let mut next = &zero;
    for (_, source) in pieces.iter() {
        let [a, b] = &source.parameters;
        let lo = a.min(b);
        let hi = a.max(b);
        crate::check(
            lo == next && lo < hi,
            "Periodic trim segment bindings do not partition the segment",
        )?;
        next = hi;
    }
    crate::check(
        pieces.is_empty() || next.is_one(),
        "Periodic trim segment bindings end before the segment",
    )
}

fn map_trim_input_interval(
    curve: &Curve,
    source: &PeriodicTrimInputEdgeSource,
    range: [&BigRational; 2],
) -> Result<[BigRational; 2]> {
    curve.validate()?;
    let [start, end] = curve.domain().map(BigRational::from_float);
    let (Some(start), Some(end)) = (start, end) else {
        return Err(crate::input("Invalid periodic trim edge clock"));
    };
    let [from, to] = &source.parameters;
    let scale = (end - &start) / (to - from);
    Ok(range.map(|parameter| &start + (parameter - from) * &scale))
}

fn trim_clock_bracket(clock: &BigRational) -> Result<[f64; 2]> {
    use num_traits::ToPrimitive;
    let invalid = || crate::input("Cannot enclose periodic trim edge clock");
    let rounded = clock.to_f64().ok_or_else(invalid)?;
    let stored = BigRational::from_float(rounded).ok_or_else(invalid)?;
    let bracket = match stored.cmp(clock) {
        std::cmp::Ordering::Less => [rounded, rounded.next_up()],
        std::cmp::Ordering::Equal => [rounded; 2],
        std::cmp::Ordering::Greater => [rounded.next_down(), rounded],
    };
    let [lo, hi] = bracket.map(BigRational::from_float);
    if lo.as_ref().is_some_and(|v| v <= clock) && hi.as_ref().is_some_and(|v| v >= clock) {
        Ok(bracket)
    } else {
        Err(invalid())
    }
}

#[derive(Clone, Copy)]
enum LengthBoundSide {
    Both,
    Lower,
    Upper,
}

fn measure_periodic_trim_curves(
    curves: Vec<(&Curve, Option<[f64; 2]>)>,
    tolerance: f64,
    max_cells: usize,
) -> Result<PeriodicTrimContourLength> {
    measure_periodic_trim_length_jobs(
        curves
            .into_iter()
            .map(|(curve, interval)| (curve, interval, LengthBoundSide::Both))
            .collect(),
        tolerance,
        max_cells,
    )
}

fn measure_periodic_trim_length_jobs(
    curves: Vec<(&Curve, Option<[f64; 2]>, LengthBoundSide)>,
    tolerance: f64,
    max_cells: usize,
) -> Result<PeriodicTrimContourLength> {
    use crate::{
        analysis::curve_measure::{LengthReport, StopReason, length_on_interval},
        distance_bounds::Interval,
    };

    crate::check(
        tolerance.is_finite() && tolerance > 0. && (1..=100_000).contains(&max_cells),
        "Periodic trim length needs positive finite tolerance and 1..100000 cells",
    )?;
    let mut spans = Vec::with_capacity(curves.len());
    for (curve, interval, _) in &curves {
        curve.validate()?;
        let [lo, hi] = interval.unwrap_or_else(|| curve.domain());
        spans.push(
            curve.knots[curve.degree..=curve.control_points.len()]
                .windows(2)
                .filter(|knots| lo.max(knots[0]) < hi.min(knots[1]))
                .count(),
        );
    }
    let minimum_cells = spans.iter().sum();
    if curves.is_empty() || minimum_cells > max_cells {
        return Ok(PeriodicTrimContourLength {
            length: None,
            minimum_cells,
        });
    }

    let piece_tolerance = (tolerance / curves.len() as f64).max(f64::from_bits(1));
    let mut remaining_minimum = minimum_cells;
    let mut cells = 0;
    let mut lo = 0.0_f64;
    let mut hi = 0.0_f64;
    let mut stop = StopReason::PrecisionLimit;
    for ((curve, interval, side), minimum) in curves.into_iter().zip(spans) {
        remaining_minimum -= minimum;
        let report = length_on_interval(
            curve,
            interval.unwrap_or_else(|| curve.domain()),
            piece_tolerance,
            max_cells - cells - remaining_minimum,
        )?;
        if report.stop_reason == StopReason::WorkLimit {
            stop = StopReason::WorkLimit;
        }
        cells += report.cells;
        if !matches!(side, LengthBoundSide::Upper) {
            lo = (lo + report.bounds[0]).next_down().max(0.);
        }
        if !matches!(side, LengthBoundSide::Lower) {
            hi = (hi + report.bounds[1]).next_up();
        }
    }
    Interval::new(lo, hi)?;
    let value = lo + (hi - lo) * 0.5;
    let error_upper = (value - lo).max(hi - value).next_up();
    let within_tolerance = error_upper <= tolerance;
    Ok(PeriodicTrimContourLength {
        length: Some(LengthReport {
            value,
            bounds: [lo, hi],
            error_upper,
            within_tolerance,
            cells,
            stop_reason: if within_tolerance {
                StopReason::Tolerance
            } else {
                stop
            },
        }),
        minimum_cells,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicTrimLengthLocationStatus {
    Tolerance,
    WorkLimit,
    PrecisionLimit,
    LengthUncertainty,
    Outside,
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimLengthLocation {
    /// Guaranteed to contain a parameter with the requested prefix length.
    pub parameter_bounds: Option<[f64; 2]>,
    pub cells: usize,
    pub status: PeriodicTrimLengthLocationStatus,
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimLengthLocations {
    pub stations: Vec<PeriodicTrimLengthLocation>,
    pub table_cells: usize,
    pub cells: usize,
}

impl PeriodicTrimInputEdges {
    /// Invert certified 3D prefix length in increasing original segment order.
    /// Ambiguous comparisons never discard a possible parameter.
    pub fn locate_input_segment_at_length(
        &self,
        contour: usize,
        segment: usize,
        distance: f64,
        length_tolerance: f64,
        parameter_tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimLengthLocation> {
        use PeriodicTrimLengthLocationStatus as Status;
        use num_traits::Zero;

        validate_trim_length_inversion(distance, parameter_tolerance)?;
        let result = |parameter_bounds, cells, status| PeriodicTrimLengthLocation {
            parameter_bounds,
            cells,
            status,
        };
        let total =
            self.measure_input_segment_length(contour, segment, length_tolerance, max_cells)?;
        let Some(length) = total.length else {
            let status = if total.minimum_cells == 0 {
                Status::Unavailable
            } else {
                Status::WorkLimit
            };
            return Ok(result(None, 0, status));
        };
        let cells = length.cells;
        if distance == 0. {
            return Ok(result(Some([0., 0.]), cells, Status::Tolerance));
        }
        if distance > length.bounds[1] {
            return Ok(result(None, cells, Status::Outside));
        }
        if distance > length.bounds[0] {
            return Ok(result(None, cells, Status::LengthUncertainty));
        }

        let zero = BigRational::zero();
        refine_trim_length_location(
            [0., 1.],
            [distance; 2],
            parameter_tolerance,
            cells,
            max_cells,
            |candidate, remaining| {
                let parameter = BigRational::from_float(candidate).unwrap();
                let prefix = self.measure_rational_input_segment_range_length(
                    contour,
                    segment,
                    [&zero, &parameter],
                    length_tolerance,
                    remaining,
                )?;
                Ok(prefix.length.map(|length| (length.bounds, length.cells)))
            },
        )
    }

    /// Invert certified length measured from vertex zero of a closed contour.
    /// Returned parameters use segment index plus local parameter.
    pub fn locate_input_contour_at_length(
        &self,
        contour: usize,
        distance: f64,
        length_tolerance: f64,
        parameter_tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimLengthLocation> {
        validate_trim_length_inversion(distance, parameter_tolerance)?;
        let table =
            self.measure_input_contour_prefix_lengths(contour, length_tolerance, max_cells)?;
        self.locate_contour_length_with_table(
            contour,
            distance,
            length_tolerance,
            parameter_tolerance,
            &table,
            table.cells,
            max_cells,
        )
    }

    /// Locate multiple contour stations in input order under one cell budget.
    /// A station's cell count excludes the shared table and earlier stations.
    pub fn locate_input_contour_at_lengths(
        &self,
        contour: usize,
        distances: &[f64],
        length_tolerance: f64,
        parameter_tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimLengthLocations> {
        crate::check(
            !distances.is_empty() && distances.len() <= 4096,
            "Periodic trim station batch needs 1..4096 distances",
        )?;
        for &distance in distances {
            validate_trim_length_inversion(distance, parameter_tolerance)?;
        }
        let table =
            self.measure_input_contour_prefix_lengths(contour, length_tolerance, max_cells)?;
        let mut cells = table.cells;
        let mut stations: Vec<PeriodicTrimLengthLocation> = Vec::with_capacity(distances.len());
        let mut earlier = std::collections::BTreeMap::<u64, usize>::new();
        for &distance in distances {
            let mut station: PeriodicTrimLengthLocation;
            if let Some(&index) = earlier.get(&distance.to_bits()) {
                station = PeriodicTrimLengthLocation {
                    cells: 0,
                    ..stations[index].clone()
                };
            } else {
                station = self.locate_contour_length_with_table(
                    contour,
                    distance,
                    length_tolerance,
                    parameter_tolerance,
                    &table,
                    cells,
                    max_cells,
                )?;
                let used = station.cells - cells;
                cells = station.cells;
                station.cells = used;
                earlier.insert(distance.to_bits(), stations.len());
            }
            stations.push(station);
        }
        Ok(PeriodicTrimLengthLocations {
            stations,
            table_cells: table.cells,
            cells,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn locate_contour_length_with_table(
        &self,
        contour: usize,
        distance: f64,
        length_tolerance: f64,
        parameter_tolerance: f64,
        table: &PeriodicTrimContourLengthTable,
        cells: usize,
        max_cells: usize,
    ) -> Result<PeriodicTrimLengthLocation> {
        use PeriodicTrimLengthLocationStatus as Status;

        let result = |parameter_bounds, status| PeriodicTrimLengthLocation {
            parameter_bounds,
            cells,
            status,
        };
        let Some(prefixes) = &table.vertex_prefix_bounds else {
            let status = if table.minimum_cells == 0 {
                Status::Unavailable
            } else {
                Status::WorkLimit
            };
            return Ok(result(None, status));
        };
        let [total_low, total_high] = prefixes[prefixes.len() - 1];
        if distance == 0. {
            return Ok(result(Some([0., 0.]), Status::Tolerance));
        }
        if distance > total_high {
            return Ok(result(None, Status::Outside));
        }
        if distance > total_low {
            return Ok(result(None, Status::LengthUncertainty));
        }
        self.refine_contour_length_target(
            contour,
            prefixes,
            [distance; 2],
            length_tolerance,
            parameter_tolerance,
            cells,
            max_cells,
        )
    }

    /// Divide a closed contour into equal fractions of its total 3D length.
    /// Includes the start and the closing endpoint. All stations share the cell budget.
    pub fn divide_input_contour_by_length(
        &self,
        contour: usize,
        divisions: usize,
        length_tolerance: f64,
        parameter_tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimLengthLocations> {
        use crate::distance_bounds::Interval;
        use PeriodicTrimLengthLocationStatus as Status;

        crate::check(
            (1..=4096).contains(&divisions),
            "Periodic trim length divisions must be in 1..4096",
        )?;
        validate_trim_length_inversion(0., parameter_tolerance)?;
        let table =
            self.measure_input_contour_prefix_lengths(contour, length_tolerance, max_cells)?;
        let Some(prefixes) = table.vertex_prefix_bounds else {
            let status = if table.minimum_cells == 0 {
                Status::Unavailable
            } else {
                Status::WorkLimit
            };
            return Ok(PeriodicTrimLengthLocations {
                stations: vec![
                    PeriodicTrimLengthLocation {
                        parameter_bounds: None,
                        cells: 0,
                        status,
                    };
                    divisions + 1
                ],
                table_cells: table.cells,
                cells: table.cells,
            });
        };
        let end = (prefixes.len() - 1) as f64;
        let [lo, hi] = prefixes[prefixes.len() - 1];
        let total = Interval::new(lo, hi)?;
        let endpoint = |t| PeriodicTrimLengthLocation {
            parameter_bounds: Some([t, t]),
            cells: 0,
            status: Status::Tolerance,
        };
        let mut cells = table.cells;
        let mut stations = Vec::with_capacity(divisions + 1);
        stations.push(endpoint(0.));
        for station in 1..divisions {
            let fraction = BigRational::new(station.into(), divisions.into());
            let [lo, hi] = trim_clock_bracket(&fraction)?;
            let target = total.mul(Interval::new(lo, hi)?)?;
            let mut result = self.refine_contour_length_target(
                contour,
                &prefixes,
                [target.lo.max(0.), target.hi],
                length_tolerance,
                parameter_tolerance,
                cells,
                max_cells,
            )?;
            let used = result.cells - cells;
            cells = result.cells;
            result.cells = used;
            stations.push(result);
        }
        stations.push(endpoint(end));
        Ok(PeriodicTrimLengthLocations {
            stations,
            table_cells: table.cells,
            cells,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn refine_contour_length_target(
        &self,
        contour: usize,
        prefixes: &[[f64; 2]],
        target: [f64; 2],
        length_tolerance: f64,
        parameter_tolerance: f64,
        cells: usize,
        max_cells: usize,
    ) -> Result<PeriodicTrimLengthLocation> {
        use crate::distance_bounds::Interval;
        use num_traits::Zero;

        let lo = prefixes
            .partition_point(|bounds| bounds[1] < target[0])
            .saturating_sub(1);
        let hi = prefixes
            .partition_point(|bounds| bounds[0] <= target[1])
            .min(prefixes.len() - 1);
        let zero = BigRational::zero();
        refine_trim_length_location(
            [lo as f64, hi as f64],
            target,
            parameter_tolerance,
            cells,
            max_cells,
            |candidate, remaining| {
                let segment = candidate.floor() as usize;
                let local = candidate - segment as f64;
                let [lo, hi] = prefixes[segment];
                if local == 0. {
                    return Ok(Some(([lo, hi], 0)));
                }
                let parameter = BigRational::from_float(local).unwrap();
                let prefix = self.measure_rational_input_segment_range_length(
                    contour,
                    segment,
                    [&zero, &parameter],
                    length_tolerance,
                    remaining,
                )?;
                let Some(length) = prefix.length else {
                    return Ok(None);
                };
                let bounds = Interval::new(lo, hi)?
                    .add(Interval::new(length.bounds[0], length.bounds[1])?)?;
                Ok(Some(([bounds.lo.max(lo), bounds.hi], length.cells)))
            },
        )
    }
}

fn validate_trim_length_inversion(distance: f64, parameter_tolerance: f64) -> Result<()> {
    crate::check(
        distance.is_finite()
            && distance >= 0.
            && parameter_tolerance.is_finite()
            && parameter_tolerance > 0.,
        "Periodic trim length inversion needs a nonnegative distance and positive parameter tolerance",
    )
}

fn refine_trim_length_location(
    mut bounds: [f64; 2],
    target: [f64; 2],
    parameter_tolerance: f64,
    mut cells: usize,
    max_cells: usize,
    mut prefix_at: impl FnMut(f64, usize) -> Result<Option<([f64; 2], usize)>>,
) -> Result<PeriodicTrimLengthLocation> {
    use PeriodicTrimLengthLocationStatus as Status;

    let status = loop {
        let [lo, hi] = bounds;
        if (hi - lo).next_up() <= parameter_tolerance {
            break Status::Tolerance;
        }
        let third = (hi - lo) / 3.;
        let candidates = [lo + third, hi - third];
        if !(lo < candidates[0] && candidates[0] < candidates[1] && candidates[1] < hi) {
            break Status::PrecisionLimit;
        }
        for candidate in candidates {
            if candidate <= bounds[0] || candidate >= bounds[1] {
                continue;
            }
            let remaining = max_cells - cells;
            let prefix = if remaining == 0 {
                None
            } else {
                prefix_at(candidate, remaining)?
            };
            let Some((prefix, used)) = prefix else {
                return Ok(PeriodicTrimLengthLocation {
                    parameter_bounds: Some(bounds),
                    cells,
                    status: Status::WorkLimit,
                });
            };
            cells += used;
            if prefix[1] < target[0] {
                bounds[0] = candidate;
            } else if prefix[0] > target[1] {
                bounds[1] = candidate;
            }
        }
        if bounds == [lo, hi] {
            break Status::LengthUncertainty;
        }
    };
    Ok(PeriodicTrimLengthLocation {
        parameter_bounds: Some(bounds),
        cells,
        status,
    })
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimContourLengthTable {
    /// Prefix bounds at original vertices, starting at zero and ending at closure.
    pub vertex_prefix_bounds: Option<Vec<[f64; 2]>>,
    pub minimum_cells: usize,
    pub cells: usize,
}

impl PeriodicTrimInputEdges {
    /// Measure cumulative world length in original contour order under one cell budget.
    pub fn measure_input_contour_prefix_lengths(
        &self,
        contour: usize,
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimContourLengthTable> {
        use crate::distance_bounds::Interval;
        crate::check(
            tolerance.is_finite() && tolerance > 0. && (1..=100_000).contains(&max_cells),
            "Periodic trim length needs positive finite tolerance and 1..100000 cells",
        )?;
        let mut segments = std::collections::BTreeMap::<_, Vec<_>>::new();
        for piece in self.input_contour_pieces(contour)? {
            segments.entry(piece.1.segment).or_default().push(piece);
        }
        let mut minimum_cells = 0;
        let mut jobs = Vec::with_capacity(segments.len());
        for (expected, (segment, mut pieces)) in segments.into_iter().enumerate() {
            crate::check(
                segment == expected,
                "Periodic trim contour segment is missing",
            )?;
            verify_trim_segment_partition(&mut pieces)?;
            let mut minimum = 0;
            let mut curves = Vec::with_capacity(pieces.len());
            for (curve, _) in pieces {
                curve.validate()?;
                minimum += curve.knots[curve.degree..=curve.control_points.len()]
                    .windows(2)
                    .filter(|pair| pair[0] < pair[1])
                    .count();
                curves.push((curve, None));
            }
            minimum_cells += minimum;
            jobs.push((curves, minimum));
        }
        if jobs.is_empty() || minimum_cells > max_cells {
            return Ok(PeriodicTrimContourLengthTable {
                vertex_prefix_bounds: None,
                minimum_cells,
                cells: 0,
            });
        }
        let piece_tolerance = (tolerance / jobs.len() as f64).max(f64::from_bits(1));
        let mut remaining_minimum = minimum_cells;
        let mut cells = 0;
        let mut prefix = Interval::point(0.);
        let mut prefixes = vec![[0., 0.]];
        for (curves, minimum) in jobs {
            remaining_minimum -= minimum;
            let report = measure_periodic_trim_curves(
                curves,
                piece_tolerance,
                max_cells - cells - remaining_minimum,
            )?;
            let length = report
                .length
                .ok_or_else(|| crate::input("Missing periodic trim segment length"))?;
            cells += length.cells;
            let mut next = prefix.add(Interval::new(length.bounds[0], length.bounds[1])?)?;
            next.lo = next.lo.max(prefix.lo);
            next.hi = next.hi.max(prefix.hi);
            prefixes.push([next.lo, next.hi]);
            prefix = next;
        }
        Ok(PeriodicTrimContourLengthTable {
            vertex_prefix_bounds: Some(prefixes),
            minimum_cells,
            cells,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimStationBounds {
    pub location: PeriodicTrimLengthLocation,
    /// One outward-rounded interval per world coordinate.
    pub point_bounds: Option<[[f64; 2]; 3]>,
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimDivisionBounds {
    pub stations: Vec<PeriodicTrimStationBounds>,
    pub table_cells: usize,
    pub cells: usize,
}

impl PeriodicTrimInputEdges {
    /// Enclose the world-space point at a certified contour station.
    /// The box covers every curve piece intersecting the proved parameter range.
    pub fn enclose_input_contour_station(
        &self,
        contour: usize,
        distance: f64,
        length_tolerance: f64,
        parameter_tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimStationBounds> {
        let location = self.locate_input_contour_at_length(
            contour,
            distance,
            length_tolerance,
            parameter_tolerance,
            max_cells,
        )?;
        let pieces = if location.parameter_bounds.is_some() {
            self.sorted_station_pieces(contour)?
        } else {
            Vec::new()
        };
        Self::enclose_contour_location(&pieces, location)
    }

    /// Enclose the world coordinates of every equal-length division station.
    /// Stations retain their individual search status and any proved bracket.
    pub fn enclose_input_contour_divisions(
        &self,
        contour: usize,
        divisions: usize,
        length_tolerance: f64,
        parameter_tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimDivisionBounds> {
        let division = self.divide_input_contour_by_length(
            contour,
            divisions,
            length_tolerance,
            parameter_tolerance,
            max_cells,
        )?;
        let pieces = if division
            .stations
            .iter()
            .any(|station| station.parameter_bounds.is_some())
        {
            self.sorted_station_pieces(contour)?
        } else {
            Vec::new()
        };
        let stations = division
            .stations
            .into_iter()
            .map(|location| Self::enclose_contour_location(&pieces, location))
            .collect::<Result<Vec<_>>>()?;
        Ok(PeriodicTrimDivisionBounds {
            stations,
            table_cells: division.table_cells,
            cells: division.cells,
        })
    }

    fn sorted_station_pieces(&self, contour: usize) -> Result<Vec<TrimInputPiece<'_>>> {
        let mut pieces = self.input_contour_pieces(contour)?;
        pieces.sort_by_key(|(_, source)| source.segment);
        Ok(pieces)
    }

    fn enclose_contour_location(
        pieces: &[TrimInputPiece<'_>],
        location: PeriodicTrimLengthLocation,
    ) -> Result<PeriodicTrimStationBounds> {
        let point_bounds = location
            .parameter_bounds
            .map(|parameter| Self::enclose_contour_parameter_bounds(pieces, parameter))
            .transpose()?;
        Ok(PeriodicTrimStationBounds {
            location,
            point_bounds,
        })
    }

    fn enclose_contour_parameter_bounds(
        pieces: &[TrimInputPiece<'_>],
        parameter: [f64; 2],
    ) -> Result<[[f64; 2]; 3]> {
        let first = (parameter[0].ceil() as usize).saturating_sub(1);
        let last = parameter[1].floor() as usize;
        let start = pieces.partition_point(|(_, source)| source.segment < first);
        let end = pieces.partition_point(|(_, source)| source.segment <= last);
        let global = parameter.map(|t| BigRational::from_float(t).unwrap());
        let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        for &(curve, source) in &pieces[start..end] {
            let offset = BigRational::from_integer(source.segment.into());
            let [a, b] = &source.parameters;
            let [start, end] = [a.min(b) + &offset, a.max(b) + &offset];
            let lo = (&global[0]).max(&start);
            let hi = (&global[1]).min(&end);
            if lo > hi {
                continue;
            }
            let local = [lo - &offset, hi - &offset];
            let [p, q] = map_trim_input_interval(curve, source, [&local[0], &local[1]])?;
            let [lo, _] = trim_clock_bracket((&p).min(&q))?;
            let [_, hi] = trim_clock_bracket((&p).max(&q))?;
            for span in curve.degree..curve.control_points.len() {
                let [start, end] = [curve.knots[span], curve.knots[span + 1]];
                if start == end || hi < start || lo > end {
                    continue;
                }
                let [a, b] = [lo.max(start), hi.min(end)];
                let jets = if a < b {
                    crate::curve_jets::enclose(curve, span, [a, b])?
                } else if a == start {
                    crate::curve_jets::endpoint(curve, span, [start, end], false)?
                } else {
                    crate::curve_jets::endpoint(curve, span, [start, a], true)?
                };
                for (bound, axis) in bounds.iter_mut().zip(&jets[0]) {
                    bound[0] = bound[0].min(axis.lo);
                    bound[1] = bound[1].max(axis.hi);
                }
            }
        }
        crate::check(
            bounds
                .iter()
                .all(|[lo, hi]| lo.is_finite() && hi.is_finite() && lo <= hi),
            "Periodic trim station has no enclosing curve pieces",
        )?;
        Ok(bounds)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicTrimProjectedAreaRefusal {
    NonlinearEdge,
    Arithmetic(ExactRefusal),
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimProjectedArea {
    /// Signed projected areas onto the YZ, ZX and XY planes in input direction.
    pub projected_area: Option<[BigRational; 3]>,
    pub work_used: u64,
    pub refusal: Option<PeriodicTrimProjectedAreaRefusal>,
}

impl PeriodicTrimInputEdges {
    /// Integrate the oriented projection areas of a closed contour made
    /// of straight spatial edges, including rational degree-one edges.
    pub fn exact_input_contour_projected_area(
        &self,
        contour: usize,
        max_work: u64,
    ) -> Result<PeriodicTrimProjectedArea> {
        self.input_contour_projected_area(contour, max_work, false)
    }

    /// Integrate signed projection areas of straight and polynomial Bézier
    /// edges. Higher-degree pieces must have a constant weight.
    pub fn exact_input_contour_polynomial_projected_area(
        &self,
        contour: usize,
        max_work: u64,
    ) -> Result<PeriodicTrimProjectedArea> {
        self.input_contour_projected_area(contour, max_work, true)
    }

    fn input_contour_projected_area(
        &self,
        contour: usize,
        max_work: u64,
        allow_polynomial: bool,
    ) -> Result<PeriodicTrimProjectedArea> {
        crate::check(
            (1..=cad_predicates::MAX_WORK).contains(&max_work),
            "Exact periodic trim area work must be in 1..=1000000",
        )?;
        let mut work = Work {
            used: 0,
            limit: max_work,
        };
        let pieces = self.input_contour_pieces(contour)?;
        let result = if pieces.is_empty() {
            Ok(None)
        } else {
            trim_projected_area(&pieces, allow_polynomial, &mut work).map(Some)
        };
        let (projected_area, refusal) = match result {
            Ok(area) => (area, None),
            Err(reason) => (None, Some(reason)),
        };
        Ok(PeriodicTrimProjectedArea {
            projected_area,
            work_used: work.used,
            refusal,
        })
    }
}

impl From<ExactRefusal> for PeriodicTrimProjectedAreaRefusal {
    fn from(reason: ExactRefusal) -> Self {
        Self::Arithmetic(reason)
    }
}

fn trim_projected_area(
    pieces: &[TrimInputPiece<'_>],
    allow_polynomial: bool,
    work: &mut Work,
) -> std::result::Result<[BigRational; 3], PeriodicTrimProjectedAreaRefusal> {
    let mut area: [BigRational; 3] = std::array::from_fn(|_| BigRational::default());
    let mut first = None;
    let mut previous = None;
    let reversed = pieces[0].1.parameters[0] > pieces[0].1.parameters[1];
    for (curve, source) in pieces {
        work.charge(1).map_err(ExactRefusal::from)?;
        curve.validate().map_err(|_| ExactRefusal::InvalidOutput)?;
        let count = curve.control_points.len();
        let [lo, hi] = curve.domain();
        let bezier = !curve.periodic
            && count == curve.degree + 1
            && curve.knots[..count].iter().all(|&value| value == lo)
            && curve.knots[count..].iter().all(|&value| value == hi);
        if !bezier
            || (curve.degree != 1
                && (!allow_polynomial || curve.weights.iter().any(|&w| w != curve.weights[0])))
        {
            return Err(PeriodicTrimProjectedAreaRefusal::NonlinearEdge);
        }
        if curve.control_points[0].len() != 3
            || source.parameters[0] == source.parameters[1]
            || reversed != (source.parameters[0] > source.parameters[1])
        {
            return Err(ExactRefusal::InvalidOutput.into());
        }
        let mut poles = Vec::with_capacity(count);
        for pole in &curve.control_points {
            poles.push([
                work.lift(pole[0]).map_err(ExactRefusal::from)?,
                work.lift(pole[1]).map_err(ExactRefusal::from)?,
                work.lift(pole[2]).map_err(ExactRefusal::from)?,
            ]);
        }
        let start = &poles[0];
        let end = &poles[count - 1];
        if previous.as_ref().is_some_and(|point| point != start) {
            return Err(ExactRefusal::InvalidOutput.into());
        }
        if first.is_none() {
            first = Some(start.clone());
        }
        if curve.degree == 1 {
            accumulate_trim_cross(
                &mut area,
                start,
                end,
                &BigRational::from_integer(1.into()),
                work,
            )?;
        } else {
            integrate_polynomial_trim_cross(&mut area, &poles, work)?;
        }
        previous = Some(end.clone());
    }
    if first != previous {
        return Err(ExactRefusal::InvalidOutput.into());
    }
    let divisor = BigRational::from_integer(if reversed { (-2).into() } else { 2.into() });
    for sum in &mut area {
        *sum = work
            .op(sum, &divisor, |a, b| a / b)
            .map_err(ExactRefusal::from)?;
    }
    Ok(area)
}

fn accumulate_trim_cross(
    area: &mut [BigRational; 3],
    a: &[BigRational; 3],
    b: &[BigRational; 3],
    scale: &BigRational,
    work: &mut Work,
) -> std::result::Result<(), ExactRefusal> {
    for (axis, sum) in area.iter_mut().enumerate() {
        let i = (axis + 1) % 3;
        let j = (axis + 2) % 3;
        let x = work.op(&a[i], &b[j], |a, b| a * b)?;
        let y = work.op(&a[j], &b[i], |a, b| a * b)?;
        let cross = work.op(&x, &y, |a, b| a - b)?;
        let scaled = work.op(&cross, scale, |a, b| a * b)?;
        *sum = work.op(sum, &scaled, |a, b| a + b)?;
    }
    Ok(())
}

fn integrate_polynomial_trim_cross(
    area: &mut [BigRational; 3],
    poles: &[[BigRational; 3]],
    work: &mut Work,
) -> std::result::Result<(), ExactRefusal> {
    use num_bigint::BigInt;
    let binomial_row = |degree| {
        let mut row = vec![BigInt::from(1)];
        for k in 1..=degree {
            row.push(&row[k - 1] * (degree + 1 - k) / k);
        }
        row
    };
    let n = poles.len() - 1;
    let basis = binomial_row(n);
    let derivative_basis = binomial_row(n - 1);
    let product_basis = binomial_row(2 * n - 1);
    for (j, pair) in poles.windows(2).enumerate() {
        let [a, b, c] = [0, 1, 2].map(|axis| work.op(&pair[1][axis], &pair[0][axis], |x, y| x - y));
        let delta = [a?, b?, c?];
        for (i, pole) in poles.iter().enumerate() {
            // Integral of B_i^n B_j^{n-1}, multiplied by the derivative degree n.
            let scale =
                BigRational::new(&basis[i] * &derivative_basis[j], &product_basis[i + j] * 2);
            accumulate_trim_cross(area, pole, &delta, &scale, work)?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimProjectedAreaBounds {
    /// Signed YZ, ZX and XY areas in original contour direction.
    pub bounds: Option<[[f64; 2]; 3]>,
    pub cells: usize,
    pub minimum_cells: usize,
    /// True when each reported interval has width at most the requested tolerance.
    pub within_tolerance: bool,
}

impl PeriodicTrimInputEdges {
    /// Enclose the projection areas of a closed rational Bézier trim contour.
    /// Refinement may stop at the cell limit while retaining complete bounds.
    pub fn bound_input_contour_projected_area(
        &self,
        contour: usize,
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimProjectedAreaBounds> {
        let pieces = self.input_contour_pieces(contour)?;
        bound_trim_projected_area(&[pieces], true, tolerance, max_cells)
    }

    /// Enclose the oriented projections of the patch selected by its outer
    /// input contour. Canonical face orientation accounts for hole subtraction.
    pub fn bound_input_patch_projected_area(
        &self,
        outer_contour: usize,
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimProjectedAreaBounds> {
        let mut loops = Vec::new();
        if let Some(patches) = &self.geometry.patches {
            crate::check(
                patches.len() == self.sources.len(),
                "Periodic trim area patch count differs",
            )?;
            for (patch, sources) in patches.iter().zip(&self.sources) {
                if patch.patch.trim.contours.first() != Some(&outer_contour) {
                    continue;
                }
                crate::check(
                    loops.is_empty()
                        && patch.loops.len() == sources.len()
                        && patch.loops.len() == patch.patch.trim.contours.len(),
                    "Periodic trim area patch has inconsistent loops",
                )?;
                for ((edges, sources), &contour) in patch
                    .loops
                    .iter()
                    .zip(sources)
                    .zip(&patch.patch.trim.contours)
                {
                    crate::check(
                        !edges.is_empty()
                            && edges.len() == sources.len()
                            && sources.iter().all(|source| source.contour == contour),
                        "Periodic trim area loop has inconsistent edge bindings",
                    )?;
                    loops.push(
                        edges
                            .iter()
                            .zip(sources)
                            .map(|(edge, source)| (&edge.curve, source))
                            .collect(),
                    );
                }
            }
        }
        bound_trim_projected_area(&loops, false, tolerance, max_cells)
    }
}

fn bound_trim_projected_area(
    loops: &[Vec<TrimInputPiece<'_>>],
    original_direction: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<PeriodicTrimProjectedAreaBounds> {
    use crate::distance_bounds::Interval;
    crate::check(
        tolerance.is_finite() && tolerance > 0. && (1..=100_000).contains(&max_cells),
        "Periodic trim area needs positive finite tolerance and 1..100000 cells",
    )?;
    let minimum_cells = loops.iter().map(Vec::len).sum();
    let mut report = PeriodicTrimProjectedAreaBounds {
        bounds: None,
        cells: 0,
        minimum_cells,
        within_tolerance: false,
    };
    if minimum_cells == 0 || minimum_cells > max_cells {
        return Ok(report);
    }

    let mut stack = Vec::with_capacity(minimum_cells);
    let share = tolerance / minimum_cells as f64;
    for pieces in loops {
        let first = pieces
            .first()
            .ok_or_else(|| crate::input("Periodic trim area has an empty loop"))?;
        let reversed = first.1.parameters[0] > first.1.parameters[1];
        let negate = original_direction && reversed;
        for ((curve, source), (next, _)) in pieces.iter().zip(pieces.iter().cycle().skip(1)) {
            curve.validate()?;
            let count = curve.control_points.len();
            let [lo, hi] = curve.domain();
            crate::check(
                !curve.periodic
                    && count == curve.degree + 1
                    && curve.control_points[0].len() == 3
                    && curve.knots[..count].iter().all(|&k| k == lo)
                    && curve.knots[count..].iter().all(|&k| k == hi)
                    && curve.control_points.last() == next.control_points.first()
                    && source.parameters[0] != source.parameters[1]
                    && reversed == (source.parameters[0] > source.parameters[1]),
                "Periodic trim area needs a closed, consistently directed spatial Bézier contour",
            )?;
            stack.push((
                *curve,
                [lo, hi],
                trim_projected_area_cell(curve, [lo, hi])?,
                share,
                negate,
            ));
            report.cells += 1;
        }
    }

    let mut sum = [Interval::point(0.); 3];
    while let Some((curve, [lo, hi], bounds, share, negate)) = stack.pop() {
        let mid = crate::distance_bounds::midpoint(lo, hi);
        if report.cells + 2 <= max_cells
            && lo < mid
            && mid < hi
            && bounds.iter().any(|b| (b.hi - b.lo).next_up() > share)
        {
            for domain in [[lo, mid], [mid, hi]] {
                stack.push((
                    curve,
                    domain,
                    trim_projected_area_cell(curve, domain)?,
                    share * 0.5,
                    negate,
                ));
            }
            report.cells += 2;
        } else {
            for (sum, bound) in sum.iter_mut().zip(bounds) {
                let directed = if negate {
                    Interval::new(-bound.hi, -bound.lo)?
                } else {
                    bound
                };
                *sum = sum.add(directed)?;
            }
        }
    }
    let bounds = sum.map(|b| [b.lo, b.hi]);
    report.within_tolerance = bounds
        .iter()
        .all(|[lo, hi]| (hi - lo).next_up() <= tolerance);
    report.bounds = Some(bounds);
    Ok(report)
}

fn trim_projected_area_cell(
    curve: &Curve,
    domain: [f64; 2],
) -> Result<[crate::distance_bounds::Interval; 3]> {
    use crate::distance_bounds::Interval;
    let span = curve.degree;
    // The evaluator differentiates in local cell coordinates on [0,1].
    let jets = crate::curve_jets::enclose(curve, span, domain)?;
    let start = crate::curve_jets::endpoint(curve, span, domain, false)?;
    let end = crate::curve_jets::endpoint(curve, span, domain, true)?;
    let mut bounds = [Interval::point(0.); 3];
    for (axis, output) in bounds.iter_mut().enumerate() {
        let i = (axis + 1) % 3;
        let j = (axis + 2) % 3;
        let cross = |a: &[Interval], b: &[Interval]| a[i].mul(b[j])?.sub(a[j].mul(b[i])?);
        let trapezoid = cross(&start[0], &start[1])?
            .add(cross(&end[0], &end[1])?)?
            .mul(Interval::point(0.5))?;
        // (r × r')'' = r' × r'' + r × r'''. The trapezoid error
        // is at most the supremum of its magnitude divided by 12.
        let second = cross(&jets[1], &jets[2])?.add(cross(&jets[0], &jets[3])?)?;
        let remainder = (second.lo.abs().max(second.hi.abs()) / 12.).next_up();
        *output = trapezoid
            .add(Interval::new(-remainder, remainder)?)?
            .mul(Interval::point(0.5))?;
    }
    Ok(bounds)
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimDirectionalAreaBounds {
    /// Signed projected area in the orientation of the supplied normal.
    pub bounds: Option<[f64; 2]>,
    pub cells: usize,
    pub minimum_cells: usize,
    pub within_tolerance: bool,
}

impl PeriodicTrimInputEdges {
    /// Enclose the signed projection of a trimmed patch onto a plane with
    /// the given normal. The plane position and the normal magnitude do not
    /// enter the result. Holes retain their canonical opposite orientation.
    pub fn bound_input_patch_area_along_normal(
        &self,
        outer_contour: usize,
        normal: [f64; 3],
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimDirectionalAreaBounds> {
        use crate::distance_bounds::{Interval, box_distance};

        let scale = normal.iter().map(|n| n.abs()).fold(0.0_f64, f64::max);
        crate::check(
            normal.iter().all(|n| n.is_finite()) && scale > 0.,
            "Projected area needs a nonzero finite normal",
        )?;
        crate::check(
            tolerance.is_finite() && tolerance > 0.,
            "Projected area needs positive finite tolerance",
        )?;
        let scale = BigRational::from_float(scale).unwrap();
        let mut scaled = [Interval::point(0.); 3];
        for (out, component) in scaled.iter_mut().zip(normal) {
            let component = BigRational::from_float(component).unwrap() / &scale;
            let [lo, hi] = trim_clock_bracket(&component)?;
            *out = Interval::new(lo, hi)?;
        }
        let (lo, hi) = box_distance(&scaled, &[Interval::point(0.); 3])?;
        let magnitude = Interval::new(lo, hi)?;
        let component_tolerance = (tolerance * 0.5).max(f64::from_bits(1));
        let area =
            self.bound_input_patch_projected_area(outer_contour, component_tolerance, max_cells)?;
        let bounds = area
            .bounds
            .map(|bounds| -> Result<[f64; 2]> {
                let mut dot = Interval::point(0.);
                for (component, [lo, hi]) in scaled.into_iter().zip(bounds) {
                    if component.lo != 0. || component.hi != 0. {
                        dot = dot.add(component.mul(Interval::new(lo, hi)?)?)?;
                    }
                }
                let projected = dot.div(magnitude)?;
                Ok([projected.lo, projected.hi])
            })
            .transpose()?;
        Ok(PeriodicTrimDirectionalAreaBounds {
            within_tolerance: bounds.is_some_and(|[lo, hi]| (hi - lo).next_up() <= tolerance),
            bounds,
            cells: area.cells,
            minimum_cells: area.minimum_cells,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimMaximumProjection {
    /// Bounds on the maximum signed projected area over all unit normals.
    /// The lower endpoint also bounds the surface area from below.
    pub maximum_bounds: Option<[f64; 2]>,
    pub cells: usize,
    pub minimum_cells: usize,
    pub within_tolerance: bool,
}

impl PeriodicTrimInputEdges {
    /// Enclose the largest oriented area projection of a trimmed patch.
    /// The optimum is the norm of its oriented area vector by
    /// Cauchy–Schwarz. The upper bound concerns the projection alone.
    pub fn bound_input_patch_max_projected_area(
        &self,
        outer_contour: usize,
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimMaximumProjection> {
        let area = self.maximum_projection_components(outer_contour, tolerance, max_cells)?;
        Ok(maximum_trim_projection(&area, tolerance)?.0)
    }

    fn maximum_projection_components(
        &self,
        outer_contour: usize,
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimProjectedAreaBounds> {
        crate::check(
            tolerance.is_finite() && tolerance > 0.,
            "Maximum projected area needs positive finite tolerance",
        )?;
        let component_tolerance = (tolerance * 0.5).max(f64::from_bits(1));
        self.bound_input_patch_projected_area(outer_contour, component_tolerance, max_cells)
    }
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimProjectionDirection {
    pub maximum: PeriodicTrimMaximumProjection,
    /// Component bounds of the maximizing unit normal. Present when the
    /// oriented area vector is proved to be nonzero.
    pub normal_bounds: Option<[[f64; 2]; 3]>,
}

impl PeriodicTrimInputEdges {
    /// Enclose the direction that maximizes the signed projected area.
    /// The tolerance controls the area enclosure; inspect the direction
    /// intervals separately for angular precision.
    pub fn bound_input_patch_max_projection_direction(
        &self,
        outer_contour: usize,
        tolerance: f64,
        max_cells: usize,
    ) -> Result<PeriodicTrimProjectionDirection> {
        use crate::distance_bounds::Interval;

        let area = self.maximum_projection_components(outer_contour, tolerance, max_cells)?;
        let (maximum, components) = maximum_trim_projection(&area, tolerance)?;
        let mut normal_bounds = None;
        if let (Some([lo, hi]), Some(components)) = (maximum.maximum_bounds, components)
            && lo > 0.
        {
            let magnitude = Interval::new(lo, hi)?;
            let mut normal = [[0.; 2]; 3];
            for (out, component) in normal.iter_mut().zip(components) {
                let normalized = component.div(magnitude)?.intersect(-1., 1.)?;
                *out = [normalized.lo, normalized.hi];
            }
            normal_bounds = Some(normal);
        }
        Ok(PeriodicTrimProjectionDirection {
            maximum,
            normal_bounds,
        })
    }
}

fn maximum_trim_projection(
    area: &PeriodicTrimProjectedAreaBounds,
    tolerance: f64,
) -> Result<(
    PeriodicTrimMaximumProjection,
    Option<[crate::distance_bounds::Interval; 3]>,
)> {
    use crate::distance_bounds::{Interval, box_distance};

    let components = area
        .bounds
        .map(|[[x0, x1], [y0, y1], [z0, z1]]| -> Result<_> {
            Ok([
                Interval::new(x0, x1)?,
                Interval::new(y0, y1)?,
                Interval::new(z0, z1)?,
            ])
        })
        .transpose()?;
    let maximum_bounds = components
        .map(|components| {
            box_distance(&components, &[Interval::point(0.); 3]).map(|(lo, hi)| [lo, hi])
        })
        .transpose()?;
    Ok((
        PeriodicTrimMaximumProjection {
            within_tolerance: maximum_bounds
                .is_some_and(|[lo, hi]| (hi - lo).next_up() <= tolerance),
            maximum_bounds,
            cells: area.cells,
            minimum_cells: area.minimum_cells,
        },
        components,
    ))
}
