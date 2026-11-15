//! Exact tensor split with a shared, identically stored seam.
use super::*;
use crate::check;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SplitRefusal {
    ResourceLimit,
    Insertion(Refusal),
    InvalidOutput,
}

#[derive(Clone, Debug)]
pub struct TensorSplit {
    pub parts: Option<[Surface; 2]>,
    pub work_used: u64,
    pub refusal: Option<SplitRefusal>,
}

/// Split a clamped nonperiodic selected axis at an original parameter.
/// Exact insertion raises the seam to degree multiplicity first. Then
/// both parts share the same seam poles and weights, absolute UV clocks,
/// and one global rational gauge. The other axis is unchanged.
pub fn split_tensor_surface(
    source: &Surface,
    axis: Axis,
    parameter: f64,
    max_work: u64,
) -> Result<TensorSplit> {
    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact tensor split work must be in [1,1000000]",
    )?;
    let (degree, knots, count, periodic) = basis(source, axis);
    let [a, b] = [knots[degree], knots[count]];
    check(
        !periodic
            && knots[..=degree].iter().all(|&u| u == a)
            && knots[count..].iter().all(|&u| u == b),
        "Exact tensor split needs a clamped nonperiodic selected axis",
    )?;
    check(
        parameter.is_finite() && a < parameter && parameter < b,
        "Exact tensor split must lie strictly inside the active domain",
    )?;

    let need = degree - knots.iter().filter(|&&k| k == parameter).count();
    let mut work_used = 0;
    let refined;
    let input = if need == 0 {
        source
    } else {
        let inserted = insert(source, axis, parameter, need, max_work)?;
        work_used = inserted.work_used;
        if let Some(reason) = inserted.refusal {
            return Ok(refused(work_used, SplitRefusal::Insertion(reason)));
        }
        refined = inserted.surface;
        &refined
    };

    let copy = (2 * input.control_points.len() * input.control_points[0].len()) as u64;
    if copy > max_work - work_used {
        return Ok(refused(work_used, SplitRefusal::ResourceLimit));
    }
    work_used += copy;
    let (_, knots, _, _) = basis(input, axis);
    let Some(first) = knots.iter().position(|&k| k == parameter) else {
        return Ok(refused(work_used, SplitRefusal::InvalidOutput));
    };
    let mut left = input.clone();
    let mut right = input.clone();
    let split_knots = |left: &mut Vec<f64>, right: &mut Vec<f64>| {
        left.truncate(first + degree);
        left.push(parameter);
        right.drain(..first);
        right.insert(0, parameter);
    };
    match axis {
        Axis::U => {
            split_knots(&mut left.knots_u, &mut right.knots_u);
            left.control_points.truncate(first);
            left.weights.truncate(first);
            right.control_points.drain(..first - 1);
            right.weights.drain(..first - 1);
        }
        Axis::V => {
            split_knots(&mut left.knots_v, &mut right.knots_v);
            for (points, weights) in left.control_points.iter_mut().zip(&mut left.weights) {
                points.truncate(first);
                weights.truncate(first);
            }
            for (points, weights) in right.control_points.iter_mut().zip(&mut right.weights) {
                points.drain(..first - 1);
                weights.drain(..first - 1);
            }
        }
    }
    if left.validate().is_err() || right.validate().is_err() {
        return Ok(refused(work_used, SplitRefusal::InvalidOutput));
    }
    Ok(TensorSplit {
        parts: Some([left, right]),
        work_used,
        refusal: None,
    })
}

fn basis(surface: &Surface, axis: Axis) -> (usize, &[f64], usize, bool) {
    match axis {
        Axis::U => (
            surface.degree_u,
            &surface.knots_u,
            surface.control_points.len(),
            surface.periodic_u,
        ),
        Axis::V => (
            surface.degree_v,
            &surface.knots_v,
            surface.control_points[0].len(),
            surface.periodic_v,
        ),
    }
}

fn refused(work_used: u64, reason: SplitRefusal) -> TensorSplit {
    TensorSplit {
        parts: None,
        work_used,
        refusal: Some(reason),
    }
}
