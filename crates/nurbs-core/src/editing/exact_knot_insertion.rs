//! Exact internal knot refinement of actual original B-spline definitions.
use super::rational_edit::{Work, store};
pub use super::rational_factor::Refusal;
use crate::{Result, check, core::curve_identity::compare_definitions, curve::Curve};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};
#[derive(Clone, Debug)]
pub struct Report {
    /// Literal source on every refusal; no partly refined curve is emitted.
    pub curve: Curve,
    pub inserted: usize,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
type PResult<T> = std::result::Result<T, Refusal>;
pub(super) fn homogeneous(
    source: &Curve,
    parameter: f64,
    count: usize,
    w: &mut Work,
) -> PResult<(Vec<Vec<Q>>, Vec<f64>)> {
    let p = source.degree;
    let dim = source.control_points[0].len();
    let u = w.lift(parameter)?;
    let mut knots = source.knots.clone();
    let mut net = Vec::new();
    for (cp, &weight) in source.control_points.iter().zip(&source.weights) {
        let weight = w.lift(weight)?;
        let mut row = Vec::new();
        for &x in cp {
            let x = w.lift(x)?;
            row.push(w.op(&x, &weight, |a, b| a * b)?);
        }
        row.push(weight);
        net.push(row);
    }
    for _ in 0..count {
        let k = knots.partition_point(|x| *x <= parameter) - 1;
        let s = knots.iter().filter(|x| **x == parameter).count();
        let n = net.len();
        let mut refined = vec![vec![Q::zero(); dim + 1]; n + 1];
        refined[..=k - p].clone_from_slice(&net[..=k - p]);
        refined[k - s + 1..].clone_from_slice(&net[k - s..]);
        for i in k - p + 1..=k - s {
            let left = w.lift(knots[i])?;
            let right = w.lift(knots[i + p])?;
            let denominator = w.op(&right, &left, |a, b| a - b)?;
            if denominator <= Q::zero() {
                return Err(Refusal::InvalidOutput);
            }
            let distance = w.op(&u, &left, |a, b| a - b)?;
            let alpha = w.op(&distance, &denominator, |a, b| a / b)?;
            if alpha < Q::zero() || alpha > Q::one() {
                return Err(Refusal::InvalidOutput);
            }
            let beta = w.op(&Q::one(), &alpha, |a, b| a - b)?;
            for axis in 0..=dim {
                let a = w.op(&alpha, &net[i][axis], |a, b| a * b)?;
                let b = w.op(&beta, &net[i - 1][axis], |a, b| a * b)?;
                refined[i][axis] = w.op(&a, &b, |a, b| a + b)?;
            }
        }
        knots.insert(k + 1, parameter);
        net = refined;
    }
    Ok((net, knots))
}
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
