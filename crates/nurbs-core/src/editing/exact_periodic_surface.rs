//! Exact edits of periodic tensor-product surface axes.
use super::{
    exact_curve_merge::{GaugeMergeRefusal, Refusal, rotate_periodic_curve_at_exact_knot},
    exact_tensor_edit::row,
};
use crate::{
    Result,
    surface::{Axis, Surface},
};

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
    use super::{rational_edit::Work, rational_factor::Refusal as ExactRefusal};

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
    let refined = match refine_surface_axis(source, axis, parameter, &mut work) {
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

fn refine_surface_axis(
    source: &Surface,
    axis: Axis,
    parameter: f64,
    work: &mut super::rational_edit::Work,
) -> std::result::Result<Surface, super::rational_factor::Refusal> {
    use super::{
        exact_curve_merge::refine_periodic_seam_net, rational_edit::store, rational_factor::Refusal,
    };
    use num_rational::BigRational;

    let [nu, nv] = [source.control_points.len(), source.control_points[0].len()];
    let (transverse, degree, cycle) = match axis {
        Axis::U => (nv, source.degree_u, nu + 1 - source.degree_u),
        Axis::V => (nu, source.degree_v, nv + 1 - source.degree_v),
    };
    let mut channels: [Vec<BigRational>; 4] =
        std::array::from_fn(|_| Vec::with_capacity(transverse * cycle));
    let mut knots = Vec::new();
    for index in 0..transverse {
        let (net, refined_knots) =
            refine_periodic_seam_net(&row(source, axis, index), parameter, work)?;
        knots = refined_knots;
        for pole in net {
            for (channel, value) in channels.iter_mut().zip(pole) {
                channel.push(value);
            }
        }
    }
    let (points, weights) = store(&channels, work)?;
    let count = cycle + degree;
    let mut refined = source.clone();
    match axis {
        Axis::U => {
            refined.knots_u = knots;
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
    refined.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(refined)
}
