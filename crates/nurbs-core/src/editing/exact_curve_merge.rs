//! Structural C0 concatenation retaining both original absolute parameter laws.
use crate::{Result, curve::Curve};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    Periodic,
    DegreeMismatch,
    DimensionMismatch,
    NonClamped,
    NonAdjacentDomain,
    SeamMismatch,
    ControlLimit,
    InvalidOutput,
}
#[derive(Clone, Debug)]
pub struct Report {
    /// No approximate or partly merged curve is emitted on refusal.
    pub curve: Option<Curve>,
    pub refusal: Option<Refusal>,
}
/// Merge same-degree clamped nonperiodic curves with exactly adjacent domains
/// and identical shared Cartesian control AND stored weight. No normalization,
/// fitting, snapping, gauge conversion or geometric matching is performed.
/// The output retains the seam at degree multiplicity; optional exact knot
/// simplification is separate. No G1/G2, simplicity or wire-topology claim.
/// Bounded structural copies use the unchanged256-control limit, with no
/// numeric geometry arithmetic or additional predicate/work budget.
pub fn merge(left: &Curve, right: &Curve) -> Result<Report> {
    left.validate()?;
    right.validate()?;
    if let Some(reason) = incompatibility(left, right, true) {
        return Ok(Report {
            curve: None,
            refusal: Some(reason),
        });
    }
    let mut curve = left.clone();
    append(&mut curve, right);
    if curve.validate().is_err() {
        return Ok(Report {
            curve: None,
            refusal: Some(Refusal::InvalidOutput),
        });
    }
    Ok(Report {
        curve: Some(curve),
        refusal: None,
    })
}
fn incompatibility(left: &Curve, right: &Curve, require_equal_weights: bool) -> Option<Refusal> {
    if left.periodic || right.periodic {
        return Some(Refusal::Periodic);
    }
    let p = left.degree;
    if p != right.degree {
        return Some(Refusal::DegreeMismatch);
    }
    if left.control_points[0].len() != right.control_points[0].len() {
        return Some(Refusal::DimensionMismatch);
    }
    let na = left.control_points.len();
    let nb = right.control_points.len();
    for curve in [left, right] {
        let n = curve.control_points.len();
        let [a, b] = curve.domain();
        if !curve.knots[..=p].iter().all(|&u| u == a) || !curve.knots[n..].iter().all(|&u| u == b) {
            return Some(Refusal::NonClamped);
        }
    }
    if left.domain()[1] != right.domain()[0] {
        return Some(Refusal::NonAdjacentDomain);
    }
    if na + nb - 1 > 256 {
        return Some(Refusal::ControlLimit);
    }
    if left.control_points[na - 1] != right.control_points[0]
        || (require_equal_weights && left.weights[na - 1] != right.weights[0])
    {
        return Some(Refusal::SeamMismatch);
    }
    None
}
// Only called with compatible validated storage. Mutates one private net;
// the chain never allocates another full candidate at each intermediate join.
fn append(curve: &mut Curve, right: &Curve) {
    let p = curve.degree;
    let seam = curve.domain()[1];
    curve.knots.truncate(curve.knots.len() - p - 1);
    curve.knots.extend(std::iter::repeat_n(seam, p));
    curve.knots.extend_from_slice(&right.knots[p + 1..]);
    curve
        .control_points
        .extend(right.control_points.iter().skip(1).cloned());
    curve.weights.extend_from_slice(&right.weights[1..]);
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainRefusal {
    /// Index of the right source curve at the rejected join (starting at1).
    pub right_source: usize,
    pub reason: Refusal,
}
#[derive(Clone, Debug)]
pub struct ChainReport {
    pub curve: Option<Curve>,
    pub joins_verified: usize,
    pub refusal: Option<ChainRefusal>,
}
/// Atomically assemble an ordered chain with the same exact structural rules.
/// Source count<=256, aggregate source controls<=512 (the maximum input data
/// of the existing two-curve API), output controls<=256. All source curves are
/// validated first. A failed late join emits no partial profile; inputs remain
/// borrowed and unchanged. A single valid source is returned unchanged, and
/// does not assert closure, topology or compatibility with any other source.
pub fn merge_chain(sources: &[Curve]) -> Result<ChainReport> {
    crate::check(
        !sources.is_empty() && sources.len() <= 256,
        "Curve chain needs1..=256 sources",
    )?;
    let total = sources
        .iter()
        .try_fold(0usize, |n, c| n.checked_add(c.control_points.len()));
    crate::check(
        total.is_some_and(|n| n <= 512),
        "Curve chain source controls must not exceed512",
    )?;
    for source in sources {
        source.validate()?;
    }
    let mut curve = sources[0].clone();
    let mut verified = 0;
    for (index, right) in sources.iter().enumerate().skip(1) {
        let failed = |reason| ChainReport {
            curve: None,
            joins_verified: verified,
            refusal: Some(ChainRefusal {
                right_source: index,
                reason,
            }),
        };
        if let Some(reason) = incompatibility(&curve, right, true) {
            return Ok(failed(reason));
        }
        append(&mut curve, right);
        if curve.validate().is_err() {
            return Ok(failed(Refusal::InvalidOutput));
        }
        verified += 1;
    }
    Ok(ChainReport {
        curve: Some(curve),
        joins_verified: verified,
        refusal: None,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GaugeMergeRefusal {
    Join(Refusal),
    Exact(super::rational_factor::Refusal),
}

#[derive(Clone, Debug)]
pub struct GaugeMergeReport {
    pub curve: Option<Curve>,
    pub work_used: u64,
    pub refusal: Option<GaugeMergeRefusal>,
}

/// Concatenate two clamped curves with the same degree and an identical
/// Cartesian seam pole. A uniform rational scale aligns the right weights
/// to the left seam; the complete net is stored with one exact common gauge.
/// Both curves retain their absolute parameter laws.
pub fn merge_with_weight_scaling(
    left: &Curve,
    right: &Curve,
    max_work: u64,
) -> Result<GaugeMergeReport> {
    use super::rational_edit::{Work, store};
    use num_rational::BigRational as Q;

    left.validate()?;
    right.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact gauge merge work must be in 1..=1000000",
    )?;
    let refused = |reason, work_used| GaugeMergeReport {
        curve: None,
        work_used,
        refusal: Some(reason),
    };
    if let Some(reason) = incompatibility(left, right, false) {
        return Ok(refused(GaugeMergeRefusal::Join(reason), 0));
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut build = || {
        let seam_left = work.lift(*left.weights.last().unwrap())?;
        let seam_right = work.lift(right.weights[0])?;
        let scale = work.op(&seam_left, &seam_right, |a, b| a / b)?;
        let left_weights = left
            .weights
            .iter()
            .map(|&weight| work.lift(weight))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut weights = left_weights;
        for &weight in &right.weights[1..] {
            let weight = work.lift(weight)?;
            weights.push(work.op(&weight, &scale, |a, b| a * b)?);
        }
        let mut curve = left.clone();
        append(&mut curve, right);
        let dimension = curve.control_points[0].len();
        let mut homogeneous = vec![Vec::<Q>::new(); dimension + 1];
        for (pole, weight) in curve.control_points.iter().zip(weights) {
            for (channel, &coordinate) in homogeneous.iter_mut().zip(pole) {
                let coordinate = work.lift(coordinate)?;
                channel.push(work.op(&coordinate, &weight, |a, b| a * b)?);
            }
            homogeneous[dimension].push(weight);
        }
        (curve.control_points, curve.weights) = store(&homogeneous, &mut work)?;
        Ok(curve)
    };
    let curve = match build() {
        Ok(curve) => curve,
        Err(reason) => return Ok(refused(GaugeMergeRefusal::Exact(reason), work.used)),
    };
    if curve.validate().is_err() {
        return Ok(refused(
            GaugeMergeRefusal::Join(Refusal::InvalidOutput),
            work.used,
        ));
    }
    Ok(GaugeMergeReport {
        curve: Some(curve),
        work_used: work.used,
        refusal: None,
    })
}
