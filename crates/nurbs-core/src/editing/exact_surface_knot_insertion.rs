//! Atomic tensor refinement with one global gauge and original UV proof.
use super::{
    exact_knot_insertion as curve_edit,
    rational_edit::{Work, store},
};
use crate::{
    Result,
    core::curve_identity::compare_definitions,
    curve::Curve,
    surface::{Axis, Surface},
};
use cad_predicates::{BezierIdentity, Reason};
use num_rational::BigRational as Q;
pub(super) mod denominator;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    Arithmetic(curve_edit::Refusal),
    Curve {
        row: usize,
        decision: BezierIdentity,
    },
    Denominator {
        row: usize,
    },
    InvalidOutput,
}
#[derive(Clone, Debug)]
pub struct Report {
    /// Literal source on any refusal, even after several rows were verified.
    pub surface: Surface,
    pub inserted: usize,
    pub rows_verified: usize,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
fn row(s: &Surface, axis: Axis, index: usize) -> Curve {
    match axis {
        Axis::U => Curve {
            degree: s.degree_u,
            knots: s.knots_u.clone(),
            control_points: s.control_points.iter().map(|r| r[index].clone()).collect(),
            weights: s.weights.iter().map(|r| r[index]).collect(),
            periodic: s.periodic_u,
        },
        Axis::V => Curve {
            degree: s.degree_v,
            knots: s.knots_v.clone(),
            control_points: s.control_points[index].clone(),
            weights: s.weights[index].clone(),
            periodic: s.periodic_v,
        },
    }
}
type PResult<T> = std::result::Result<T, curve_edit::Refusal>;
fn proposal(
    source: &Surface,
    axis: Axis,
    parameter: f64,
    count: usize,
    work: &mut Work,
) -> PResult<Surface> {
    let rows = match axis {
        Axis::U => source.control_points[0].len(),
        Axis::V => source.control_points.len(),
    };
    let mut nets = Vec::new();
    let mut knots = None;
    for index in 0..rows {
        let (net, k) = curve_edit::homogeneous(&row(source, axis, index), parameter, count, work)?;
        if let Some(ref old) = knots {
            if *old != k {
                return Err(curve_edit::Refusal::InvalidOutput);
            }
        } else {
            knots = Some(k);
        }
        nets.push(net);
    }
    let channels = (0..4)
        .map(|axis| {
            nets.iter()
                .flat_map(|net| net.iter().map(move |p| p[axis].clone()))
                .collect()
        })
        .collect::<Vec<Vec<Q>>>();
    // Crucial: normalize the COMPLETE tensor net once, never each row.
    let (controls, weights) = store(&channels, work)?;
    let length = nets[0].len();
    let mut output = source.clone();
    match axis {
        Axis::U => {
            output.knots_u = knots.unwrap();
            output.control_points = (0..length)
                .map(|i| {
                    (0..rows)
                        .map(|j| controls[j * length + i].clone())
                        .collect()
                })
                .collect();
            output.weights = (0..length)
                .map(|i| (0..rows).map(|j| weights[j * length + i]).collect())
                .collect();
        }
        Axis::V => {
            output.knots_v = knots.unwrap();
            output.control_points = controls.chunks_exact(length).map(|r| r.to_vec()).collect();
            output.weights = weights.chunks_exact(length).map(|r| r.to_vec()).collect();
        }
    }
    if output.validate().is_err() {
        return Err(curve_edit::Refusal::InvalidOutput);
    }
    Ok(output)
}
/// Refine one nonperiodic axis at a strictly internal original parameter.
/// Geometry and denominator laws are proved for every row under ONE global
/// scale, preserving the whole tensor surface, including an untouched periodic
/// axis. All proposal and proof arithmetic shares the existing1m work ceiling.
pub fn insert(
    source: &Surface,
    axis: Axis,
    parameter: f64,
    count: usize,
    max_work: u64,
) -> Result<Report> {
    source.validate()?;
    let first = row(source, axis, 0);
    curve_edit::validate_request(&first, parameter, count, max_work)?;
    let refused = |reason, verified, used| Report {
        surface: source.clone(),
        inserted: 0,
        rows_verified: verified,
        work_used: used,
        refusal: Some(reason),
    };
    if first.control_points.len() + count > 256 {
        return Ok(refused(
            Refusal::Arithmetic(curve_edit::Refusal::ResourceLimit),
            0,
            0,
        ));
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let candidate = match proposal(source, axis, parameter, count, &mut work) {
        Ok(c) => c,
        Err(r) => return Ok(refused(Refusal::Arithmetic(r), 0, work.used)),
    };
    let rows = match axis {
        Axis::U => source.control_points[0].len(),
        Axis::V => source.control_points.len(),
    };
    for index in 0..rows {
        if work.used == max_work {
            return Ok(refused(
                Refusal::Arithmetic(curve_edit::Refusal::ResourceLimit),
                index,
                work.used,
            ));
        }
        let input = row(source, axis, index);
        let output = row(&candidate, axis, index);
        let proof = compare_definitions(&input, &output, parameter, max_work - work.used, false)?;
        work.used += proof.work_used;
        match proof.outcome {
            BezierIdentity::Equal => {}
            BezierIdentity::Indeterminate(Reason::ResourceLimit) => {
                return Ok(refused(
                    Refusal::Arithmetic(curve_edit::Refusal::ResourceLimit),
                    index,
                    work.used,
                ));
            }
            decision => {
                return Ok(refused(
                    Refusal::Curve {
                        row: index,
                        decision,
                    },
                    index,
                    work.used,
                ));
            }
        }
        match denominator::proportional(
            &input,
            &output,
            source.weights[0][0],
            candidate.weights[0][0],
            &mut work,
        ) {
            Ok(true) => {}
            Ok(false) => {
                return Ok(refused(
                    Refusal::Denominator { row: index },
                    index,
                    work.used,
                ));
            }
            Err(r) => return Ok(refused(Refusal::Arithmetic(r), index, work.used)),
        }
    }
    Ok(Report {
        surface: candidate,
        inserted: count,
        rows_verified: rows,
        work_used: work.used,
        refusal: None,
    })
}
