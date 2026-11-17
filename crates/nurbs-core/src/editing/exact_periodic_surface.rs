//! Exact edits of periodic tensor-product surface axes.
use super::{
    exact_curve_merge::{GaugeMergeRefusal, Refusal, rotate_periodic_curve_at_exact_knot},
    exact_tensor_edit::row,
    rational_edit::Work,
    rational_factor::Refusal as ExactRefusal,
};
use crate::{
    Result,
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
    crate::check(
        [u, v]
            .iter()
            .all(|[lo, hi]| lo.is_finite() && hi.is_finite() && lo < hi),
        "Periodic surface rectangle needs increasing finite intervals",
    )?;
    let first = restrict_exact_periodic_surface_axis(source, Axis::U, u, max_work)?;
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
    let mut second =
        restrict_exact_periodic_surface_axis(&strip, Axis::V, v, max_work - first.work_used)?;
    second.work_used += first.work_used;
    Ok(second)
}

#[derive(Clone, Debug)]
pub struct StripPartitionReport {
    pub strips: Option<Vec<Surface>>,
    pub work_used: u64,
    /// Zero-based strip index, absent if rotating the seam failed.
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
    let [start, end] = row(&surface, axis, 0).domain();
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
    crate::check(
        u_cuts.len() < 256 && v_cuts.len() < 256 && (u_cuts.len() + 1) * (v_cuts.len() + 1) <= 256,
        "Periodic surface grid supports at most 256 patches",
    )?;
    let u = partition_exact_periodic_surface_axis(source, Axis::U, seams[0], u_cuts, max_work)?;
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
        let v = partition_exact_periodic_surface_axis(
            strip,
            Axis::V,
            seams[1],
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
