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
    let refined = match rebuild_surface_axis(source, axis, true, &mut work, |row, work| {
        super::exact_curve_merge::refine_periodic_seam_net(row, parameter, work)
    }) {
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
    let [lo, hi] = interval;
    crate::check(
        lo.is_finite() && hi.is_finite() && lo < hi,
        "Periodic surface restriction needs an increasing finite interval",
    )?;
    let rotated = insert_and_rotate_periodic_surface_seam(source, axis, lo, max_work)?;
    let Some(surface) = rotated.surface else {
        return Ok(rotated);
    };
    if hi > row(&surface, axis, 0).domain()[1] {
        return Ok(Report {
            surface: None,
            work_used: rotated.work_used,
            refusal: Some(GaugeMergeRefusal::Join(Refusal::NonAdjacentDomain)),
        });
    }
    let mut work = Work {
        used: rotated.work_used,
        limit: max_work,
    };
    Ok(clamp_surface_axis(&surface, axis, interval, &mut work))
}

fn clamp_surface_axis(source: &Surface, axis: Axis, interval: [f64; 2], work: &mut Work) -> Report {
    let opened = rebuild_surface_axis(source, axis, false, work, |row, work| {
        super::exact_curve_merge::clamp_curve_net(row, interval, work)
    });
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

fn rebuild_surface_axis(
    source: &Surface,
    axis: Axis,
    periodic: bool,
    work: &mut Work,
    transform: impl Fn(
        &crate::curve::Curve,
        &mut Work,
    ) -> std::result::Result<(Vec<Vec<BigRational>>, Vec<f64>), ExactRefusal>,
) -> std::result::Result<Surface, ExactRefusal> {
    use super::rational_edit::store;

    let [nu, nv] = [source.control_points.len(), source.control_points[0].len()];
    let (transverse, degree) = match axis {
        Axis::U => (nv, source.degree_u),
        Axis::V => (nu, source.degree_v),
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
        let result = assemble_surface_axis(row, Axis::V, max_work - work_used, false)?;
        work_used += result.work_used;
        let Some(strip) = result.surface else {
            return Ok(reject(Some(index), work_used, result.refusal));
        };
        strips.push(strip);
    }
    if work_used == max_work {
        return Ok(reject(None, work_used, exhausted));
    }
    let result = assemble_surface_axis(&strips, Axis::U, max_work - work_used, false)?;
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
