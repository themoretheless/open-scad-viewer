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
