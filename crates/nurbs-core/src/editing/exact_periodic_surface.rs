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
