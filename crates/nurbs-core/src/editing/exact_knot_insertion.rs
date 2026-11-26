//! Exact internal knot refinement of actual original B-spline definitions.
use super::rational_edit::{Work, store};
pub use super::rational_factor::Refusal;
use crate::{Result, check, core::curve_identity::compare_definitions, curve::Curve};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};
mod strict_box;
pub use strict_box::{closed_box_containment_on, strict_box_containment, strict_box_containment_on};
#[derive(Clone, Debug)]
pub struct Report {
    /// Literal source on every refusal; no partly refined curve is emitted.
    pub curve: Curve,
    pub inserted: usize,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
type PResult<T> = std::result::Result<T, Refusal>;
pub(super) use crate::core::exact_curve_arithmetic::{homogeneous, refine_homogeneous};

fn proposal(source: &Curve, parameter: f64, count: usize, w: &mut Work) -> PResult<Curve> {
    let (net, knots) = homogeneous(source, parameter, count, w)?;
    let dim = source.control_points[0].len();
    let channels = (0..=dim)
        .map(|axis| net.iter().map(|row| row[axis].clone()).collect())
        .collect::<Vec<Vec<Q>>>();
    let (control_points, weights) = store(&channels, w)?;
    let output = Curve {
        degree: source.degree,
        knots,
        control_points,
        weights,
        periodic: false,
    };
    if output.validate().is_err() {
        return Err(Refusal::InvalidOutput);
    }
    Ok(output)
}
/// Insert1..degree copies of an INTERNAL knot in a validated nonperiodic
/// curve, without exceeding existing multiplicity/control limits. Exact rational
/// Boehm refinement precedes global storage conversion; the full actual output
/// parameter law is verified independently. Same original absolute domain.
/// Boundary clamping and periodic seam refinement are separate operations.
pub fn insert(source: &Curve, parameter: f64, count: usize, max_work: u64) -> Result<Report> {
    validate_request(source, parameter, count, max_work)?;
    let refused = |reason, work_used| Report {
        curve: source.clone(),
        inserted: 0,
        work_used,
        refusal: Some(reason),
    };
    if source.control_points.len() + count > 256 {
        return Ok(refused(Refusal::ResourceLimit, 0));
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let candidate = match proposal(source, parameter, count, &mut work) {
        Ok(c) => c,
        Err(r) => return Ok(refused(r, work.used)),
    };
    if work.used == work.limit {
        return Ok(refused(Refusal::ResourceLimit, work.used));
    }
    let proof = compare_definitions(source, &candidate, parameter, work.limit - work.used, false)?;
    work.used += proof.work_used;
    match proof.outcome {
        cad_predicates::BezierIdentity::Equal => Ok(Report {
            curve: candidate,
            inserted: count,
            work_used: work.used,
            refusal: None,
        }),
        cad_predicates::BezierIdentity::Different => {
            Ok(refused(Refusal::DifferentDefinition, work.used))
        }
        cad_predicates::BezierIdentity::Indeterminate(cad_predicates::Reason::ResourceLimit) => {
            Ok(refused(Refusal::ResourceLimit, work.used))
        }
        cad_predicates::BezierIdentity::Indeterminate(r) => {
            Ok(refused(Refusal::PredicateIndeterminate(r), work.used))
        }
    }
}

pub(super) fn validate_request(
    source: &Curve,
    parameter: f64,
    count: usize,
    max_work: u64,
) -> Result<()> {
    source.validate()?;
    let p = source.degree;
    let [a, b] = source.domain();
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact insertion work must be in1..=1000000",
    )?;
    check(
        !source.periodic,
        "Exact internal insertion requires nonperiodic storage",
    )?;
    check(
        parameter.is_finite() && a < parameter && parameter < b,
        "Exact internal knot must lie strictly inside the original domain",
    )?;
    check(
        (1..=p).contains(&count),
        "Exact internal insertion count must be in1..=degree",
    )?;
    let multiplicity = source.knots.iter().filter(|x| **x == parameter).count();
    check(
        multiplicity + count <= p,
        "Exact insertion exceeds existing internal knot multiplicity",
    )?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct SplitReport {
    /// Both restrictions are published together, in source parameter order.
    pub pieces: Option<[Curve; 2]>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Split a nonperiodic curve at an interior parameter without changing
/// its geometry or its absolute parameter law. Exact knot refinement
/// creates a shared seam control and both restrictions retain its weight.
pub fn split(source: &Curve, parameter: f64, max_work: u64) -> Result<SplitReport> {
    source.validate()?;
    let degree = source.degree;
    let [start, end] = source.domain();
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact split work must be in 1..=1000000",
    )?;
    check(!source.periodic, "Exact split requires nonperiodic storage")?;
    check(
        parameter.is_finite() && start < parameter && parameter < end,
        "Exact split parameter must be strictly inside the original domain",
    )?;
    let multiplicity = source
        .knots
        .iter()
        .filter(|&&knot| knot == parameter)
        .count();
    check(
        multiplicity <= degree,
        "Exact split requires a continuous seam",
    )?;
    let count = degree - multiplicity;
    let refused = |reason, work_used| SplitReport {
        pieces: None,
        work_used,
        refusal: Some(reason),
    };
    let copy_work = ((source.control_points.len() + count + 1)
        * (source.control_points[0].len() + 2)
        + source.knots.len()
        + count
        + 2) as u64;
    if max_work < copy_work || (count > 0 && max_work == copy_work) {
        return Ok(refused(Refusal::ResourceLimit, 0));
    }
    let refinement = if count == 0 {
        None
    } else {
        let report = insert(source, parameter, count, max_work - copy_work)?;
        if let Some(reason) = report.refusal {
            return Ok(refused(reason, report.work_used));
        }
        Some(report)
    };
    let (refined, work_used) = refinement.as_ref().map_or((source, copy_work), |report| {
        (&report.curve, report.work_used + copy_work)
    });
    let first_knot = refined.knots.partition_point(|&knot| knot < parameter);
    let seam = first_knot - 1;

    // Degree-fold multiplicity makes the seam a single shared pole.
    // Add one copy of the seam knot to clamp each new endpoint.
    let mut left_knots = refined.knots[..first_knot + degree].to_vec();
    left_knots.push(parameter);
    let mut right_knots = vec![parameter];
    right_knots.extend_from_slice(&refined.knots[first_knot..]);
    let left = Curve {
        degree,
        knots: left_knots,
        control_points: refined.control_points[..=seam].to_vec(),
        weights: refined.weights[..=seam].to_vec(),
        periodic: false,
    };
    let right = Curve {
        degree,
        knots: right_knots,
        control_points: refined.control_points[seam..].to_vec(),
        weights: refined.weights[seam..].to_vec(),
        periodic: false,
    };
    if left.validate().is_err() || right.validate().is_err() {
        return Ok(refused(Refusal::InvalidOutput, work_used));
    }
    Ok(SplitReport {
        pieces: Some([left, right]),
        work_used,
        refusal: None,
    })
}

#[derive(Clone, Debug)]
pub struct RestrictionReport {
    /// The complete restricted curve, absent on any exact refinement refusal.
    pub curve: Option<Curve>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Restrict a nonperiodic curve to a closed subinterval of its original
/// domain. Interior cuts use exact split and share one work budget.
pub fn restrict(source: &Curve, interval: [f64; 2], max_work: u64) -> Result<RestrictionReport> {
    use std::borrow::Cow;

    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact restriction work must be in 1..=1000000",
    )?;
    check(
        !source.periodic,
        "Exact restriction requires nonperiodic storage",
    )?;
    let [start, end] = source.domain();
    let [lo, hi] = interval;
    check(
        lo.is_finite() && hi.is_finite() && start <= lo && lo < hi && hi <= end,
        "Exact restriction requires a nonempty interval inside the original domain",
    )?;
    let mut curve = Cow::Borrowed(source);
    let mut work_used = 0;
    for (cut, keep_right) in [(lo, true), (hi, false)] {
        if (keep_right && cut == start) || (!keep_right && cut == end) {
            continue;
        }
        if work_used == max_work {
            return Ok(RestrictionReport {
                curve: None,
                work_used,
                refusal: Some(Refusal::ResourceLimit),
            });
        }
        let report = split(&curve, cut, max_work - work_used)?;
        work_used += report.work_used;
        let Some([left, right]) = report.pieces else {
            return Ok(RestrictionReport {
                curve: None,
                work_used,
                refusal: report.refusal,
            });
        };
        curve = Cow::Owned(if keep_right { right } else { left });
    }
    Ok(RestrictionReport {
        curve: Some(curve.into_owned()),
        work_used,
        refusal: None,
    })
}

#[derive(Clone, Debug)]
pub struct PartitionReport {
    /// Pieces in source parameter order, absent on any refinement refusal.
    pub pieces: Option<Vec<Curve>>,
    pub work_used: u64,
    /// Zero-based index into the requested cuts for a refinement refusal.
    pub refused_cut: Option<usize>,
    pub refusal: Option<Refusal>,
}

/// Partition a nonperiodic curve at strictly increasing interior parameters.
/// The complete partition shares one exact work budget and is atomic.
pub fn partition(source: &Curve, parameters: &[f64], max_work: u64) -> Result<PartitionReport> {
    use std::borrow::Cow;

    source.validate()?;
    check(
        !source.periodic,
        "Exact partition requires nonperiodic storage",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact partition work must be in 1..=1000000",
    )?;
    let [start, end] = source.domain();
    let mut previous = start;
    for &parameter in parameters {
        check(
            parameter.is_finite() && previous < parameter && parameter < end,
            "Exact partition cuts must increase strictly inside the original domain",
        )?;
        previous = parameter;
    }

    let mut pieces = Vec::new();
    let mut tail = Cow::Borrowed(source);
    let mut work_used = 0;
    let refused = |cut, work_used, refusal| PartitionReport {
        pieces: None,
        work_used,
        refused_cut: Some(cut),
        refusal: Some(refusal),
    };
    for (index, &parameter) in parameters.iter().enumerate() {
        if work_used == max_work {
            return Ok(refused(index, work_used, Refusal::ResourceLimit));
        }
        let report = split(&tail, parameter, max_work - work_used)?;
        work_used += report.work_used;
        let Some([left, right]) = report.pieces else {
            return Ok(refused(
                index,
                work_used,
                report.refusal.unwrap_or(Refusal::InvalidOutput),
            ));
        };
        pieces.push(left);
        tail = Cow::Owned(right);
    }
    pieces.push(tail.into_owned());
    Ok(PartitionReport {
        pieces: Some(pieces),
        work_used,
        refused_cut: None,
        refusal: None,
    })
}

/// Elevate a clamped nonperiodic NURBS curve in exact homogeneous
/// arithmetic. The output uses a common gauge and degree-fold internal
/// knots. It retains the full original rational parameter law.
pub fn elevate_exact(source: &Curve, degree: usize, max_work: u64) -> Result<RestrictionReport> {
    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact elevation work must be in 1..=1000000",
    )?;
    let p = source.degree;
    check(
        degree >= p && degree <= 25,
        "Exact elevation degree must be between the source degree and 25",
    )?;
    let [start, end] = source.domain();
    check(
        !source.periodic
            && source.knots[..=p].iter().all(|&u| u == start)
            && source.knots[source.control_points.len()..]
                .iter()
                .all(|&u| u == end),
        "Exact elevation requires clamped nonperiodic storage",
    )?;
    if p == degree {
        return Ok(RestrictionReport {
            curve: Some(source.clone()),
            work_used: 0,
            refusal: None,
        });
    }
    let mut breaks = source.knots.clone();
    breaks.dedup();
    let refused = |reason, work_used| RestrictionReport {
        curve: None,
        work_used,
        refusal: Some(reason),
    };
    if (breaks.len() - 1) * degree + 1 > 256 {
        return Ok(refused(Refusal::ResourceLimit, 0));
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let candidate = match elevated_exact_proposal(source, degree, &breaks, &mut work) {
        Ok(curve) => curve,
        Err(reason) => return Ok(refused(reason, work.used)),
    };
    if work.used == max_work {
        return Ok(refused(Refusal::ResourceLimit, work.used));
    }
    let proof = compare_definitions(source, &candidate, start, max_work - work.used, false)?;
    work.used += proof.work_used;
    let refusal = match proof.outcome {
        cad_predicates::BezierIdentity::Equal => None,
        cad_predicates::BezierIdentity::Different => Some(Refusal::DifferentDefinition),
        cad_predicates::BezierIdentity::Indeterminate(cad_predicates::Reason::ResourceLimit) => {
            Some(Refusal::ResourceLimit)
        }
        cad_predicates::BezierIdentity::Indeterminate(reason) => {
            Some(Refusal::PredicateIndeterminate(reason))
        }
    };
    Ok(RestrictionReport {
        curve: refusal.is_none().then_some(candidate),
        work_used: work.used,
        refusal,
    })
}

fn elevated_exact_proposal(
    source: &Curve,
    degree: usize,
    breaks: &[f64],
    work: &mut Work,
) -> PResult<Curve> {
    let (net, elevated_knots) = elevated_homogeneous(source, degree, breaks, work)?;
    let channels: Vec<Vec<Q>> = (0..net[0].len())
        .map(|axis| net.iter().map(|row| row[axis].clone()).collect())
        .collect();
    let (control_points, weights) = store(&channels, work)?;
    let curve = Curve {
        degree,
        knots: elevated_knots,
        control_points,
        weights,
        periodic: false,
    };
    curve.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(curve)
}

pub(super) fn elevated_homogeneous(
    source: &Curve,
    degree: usize,
    breaks: &[f64],
    work: &mut Work,
) -> PResult<(Vec<Vec<Q>>, Vec<f64>)> {
    let p = source.degree;
    let (mut net, mut knots) = homogeneous(source, breaks[0], 0, work)?;
    for &parameter in &breaks[1..breaks.len() - 1] {
        let count = p
            .checked_sub(knots.iter().filter(|&&knot| knot == parameter).count())
            .ok_or(Refusal::InvalidOutput)?;
        if count > 0 {
            refine_homogeneous(&mut net, &mut knots, p, parameter, count, work)?;
        }
    }
    let mut elevated = Vec::new();
    let mut elevated_knots = vec![breaks[0]; degree + 1];
    for (index, &end) in breaks.iter().skip(1).enumerate() {
        let mut poles = net[index * p..=(index + 1) * p].to_vec();
        for order in p..degree {
            let mut next = vec![poles[0].clone()];
            for i in 1..=order {
                let alpha = Q::new(i.into(), (order + 1).into());
                let beta = work.op(&Q::one(), &alpha, |a, b| a - b)?;
                let mut pole = Vec::new();
                for (a, b) in poles[i - 1].iter().zip(&poles[i]) {
                    let a = work.op(a, &alpha, |a, b| a * b)?;
                    let b = work.op(b, &beta, |a, b| a * b)?;
                    pole.push(work.op(&a, &b, |a, b| a + b)?);
                }
                next.push(pole);
            }
            next.push(poles[order].clone());
            poles = next;
        }
        elevated.extend(poles.into_iter().skip(usize::from(index != 0)));
        elevated_knots.extend(std::iter::repeat_n(
            end,
            degree + usize::from(index + 2 == breaks.len()),
        ));
    }
    Ok((elevated, elevated_knots))
}
