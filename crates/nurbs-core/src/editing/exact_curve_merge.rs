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
    if !clamped(left) || !clamped(right) {
        return Some(Refusal::NonClamped);
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
fn clamped(curve: &Curve) -> bool {
    let [start, end] = curve.domain();
    curve.knots[..=curve.degree].iter().all(|&u| u == start)
        && curve.knots[curve.control_points.len()..]
            .iter()
            .all(|&u| u == end)
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
    Clock(super::exact_projective::Refusal),
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
    merge_chain_with_weight_scaling(&[left, right], max_work)
}

/// Concatenate an ordered chain with one rational weight scale per source.
/// The complete chain is stored once, using a single common exact gauge.
pub fn merge_chain_with_weight_scaling(
    sources: &[&Curve],
    max_work: u64,
) -> Result<GaugeMergeReport> {
    use super::rational_edit::{Work, store};
    use num_rational::BigRational as Q;

    crate::check(
        !sources.is_empty() && sources.len() <= 256,
        "Curve chain needs 1..=256 sources",
    )?;
    for source in sources {
        source.validate()?;
    }
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact gauge merge work must be in 1..=1000000",
    )?;
    let refused = |reason, work_used| GaugeMergeReport {
        curve: None,
        work_used,
        refusal: Some(reason),
    };
    if sources.len() == 1 {
        return Ok(GaugeMergeReport {
            curve: Some(sources[0].clone()),
            work_used: 0,
            refusal: None,
        });
    }
    let merged_controls = 1 + sources
        .iter()
        .map(|source| source.control_points.len() - 1)
        .sum::<usize>();
    if merged_controls > 256 {
        return Ok(refused(GaugeMergeRefusal::Join(Refusal::ControlLimit), 0));
    }
    for pair in sources.windows(2) {
        if let Some(reason) = incompatibility(pair[0], pair[1], false) {
            return Ok(refused(GaugeMergeRefusal::Join(reason), 0));
        }
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut build = || {
        let mut curve = sources[0].clone();
        let mut weights = curve
            .weights
            .iter()
            .map(|&weight| work.lift(weight))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for right in &sources[1..] {
            let seam = work.lift(right.weights[0])?;
            let scale = work.op(weights.last().expect("validated weights"), &seam, |a, b| {
                a / b
            })?;
            for &weight in &right.weights[1..] {
                let weight = work.lift(weight)?;
                weights.push(work.op(&weight, &scale, |a, b| a * b)?);
            }
            append(&mut curve, right);
        }
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

/// Join two clamped NURBS curves, elevating the lower degree exactly and
/// reconciling the weights in a common gauge. Both operations share one
/// work budget. Each source parameter interval is retained.
pub fn merge_with_exact_degree_elevation(
    left: &Curve,
    right: &Curve,
    max_work: u64,
) -> Result<GaugeMergeReport> {
    if left.degree == right.degree {
        return merge_with_weight_scaling(left, right, max_work);
    }
    left.validate()?;
    right.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact degree merge work must be in 1..=1000000",
    )?;
    let refused = |reason, work_used| GaugeMergeReport {
        curve: None,
        work_used,
        refusal: Some(reason),
    };
    for (incompatible, reason) in [
        (left.periodic || right.periodic, Refusal::Periodic),
        (
            left.control_points[0].len() != right.control_points[0].len(),
            Refusal::DimensionMismatch,
        ),
        (!clamped(left) || !clamped(right), Refusal::NonClamped),
        (
            left.domain()[1] != right.domain()[0],
            Refusal::NonAdjacentDomain,
        ),
        (
            left.control_points.last() != right.control_points.first(),
            Refusal::SeamMismatch,
        ),
    ] {
        if incompatible {
            return Ok(refused(GaugeMergeRefusal::Join(reason), 0));
        }
    }
    let elevate_left = left.degree < right.degree;
    let (lower, degree) = if elevate_left {
        (left, right.degree)
    } else {
        (right, left.degree)
    };
    let elevation = super::exact_knot_insertion::elevate_exact(lower, degree, max_work)?;
    let Some(elevated) = elevation.curve else {
        return Ok(refused(
            GaugeMergeRefusal::Exact(
                elevation
                    .refusal
                    .unwrap_or(super::rational_factor::Refusal::InvalidOutput),
            ),
            elevation.work_used,
        ));
    };
    if elevation.work_used == max_work {
        return Ok(refused(
            GaugeMergeRefusal::Exact(super::rational_factor::Refusal::ResourceLimit),
            max_work,
        ));
    }
    let (left, right) = if elevate_left {
        (&elevated, right)
    } else {
        (left, &elevated)
    };
    let mut report = merge_with_weight_scaling(left, right, max_work - elevation.work_used)?;
    report.work_used += elevation.work_used;
    Ok(report)
}

#[derive(Clone, Debug)]
pub struct CompactSeamMergeReport {
    pub curve: Option<Curve>,
    /// Remaining copies of the shared parameter after proven removals.
    pub seam_multiplicity: Option<usize>,
    pub work_used: u64,
    pub join_refusal: Option<GaugeMergeRefusal>,
    /// First failed removal, if the seam did not disappear completely.
    pub removal_refusal: Option<super::exact_knot_removal::Refusal>,
}

/// Join adjacent rational curves, then remove redundant copies of the seam.
/// Each accepted removal proves full parameter identity. If a removal is
/// refused, the curve retains all completed, verified edits.
pub fn merge_and_simplify_exact_seam(
    left: &Curve,
    right: &Curve,
    max_work: u64,
) -> Result<CompactSeamMergeReport> {
    let joined = merge_with_exact_degree_elevation(left, right, max_work)?;
    let mut work_used = joined.work_used;
    let Some(mut curve) = joined.curve else {
        return Ok(CompactSeamMergeReport {
            curve: None,
            seam_multiplicity: None,
            work_used,
            join_refusal: joined.refusal,
            removal_refusal: None,
        });
    };
    let seam = compact_seam(&mut curve, left.domain()[1], &mut work_used, max_work)?;
    Ok(CompactSeamMergeReport {
        curve: Some(curve),
        seam_multiplicity: Some(seam.remaining_multiplicity),
        work_used,
        join_refusal: None,
        removal_refusal: seam.refusal,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeamCompaction {
    pub parameter: f64,
    pub removed: usize,
    pub remaining_multiplicity: usize,
    pub refusal: Option<super::exact_knot_removal::Refusal>,
}

fn compact_seam(
    curve: &mut Curve,
    parameter: f64,
    work_used: &mut u64,
    max_work: u64,
) -> Result<SeamCompaction> {
    use super::exact_knot_removal::{Refusal as RemovalRefusal, remove};

    let mut seam = SeamCompaction {
        parameter,
        removed: 0,
        remaining_multiplicity: curve
            .knots
            .iter()
            .filter(|&&knot| knot == parameter)
            .count(),
        refusal: None,
    };
    while seam.remaining_multiplicity > 0 {
        if *work_used == max_work {
            seam.refusal = Some(RemovalRefusal::ResourceLimit);
            break;
        }
        let report = remove(curve, parameter, max_work - *work_used)?;
        *work_used += report.work_used;
        if !report.removed {
            seam.refusal = report.refusal;
            break;
        }
        *curve = report.curve;
        seam.remaining_multiplicity -= 1;
        seam.removed += 1;
    }
    Ok(seam)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactCurveSeamJetReport {
    /// Left-minus-right Cartesian residuals for position and two derivatives.
    pub residuals: Option<[Vec<num_rational::BigRational>; 3]>,
    /// Highest consecutive equal order, or none if positions differ or proof was refused.
    pub continuity_order: Option<usize>,
    pub work_used: u64,
    pub refusal: Option<GaugeMergeRefusal>,
}

/// Compare one-sided Cartesian jets at a shared absolute parameter.
/// Rational weights and knot spacing are included in the quotient derivatives.
pub fn compare_exact_curve_seam_jets(
    left: &Curve,
    right: &Curve,
    max_work: u64,
) -> Result<ExactCurveSeamJetReport> {
    compare_curve_jets(left, right, max_work, true)
}

fn compare_curve_jets(
    left: &Curve,
    right: &Curve,
    max_work: u64,
    adjacent_domains: bool,
) -> Result<ExactCurveSeamJetReport> {
    use super::rational_edit::Work;
    use num_traits::Zero;

    left.validate()?;
    right.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact seam jet work must be in 1..=1000000",
    )?;
    let refused = |reason, work_used| ExactCurveSeamJetReport {
        residuals: None,
        continuity_order: None,
        work_used,
        refusal: Some(reason),
    };
    if let Some(reason) = seam_geometry_incompatibility(left, right) {
        return Ok(refused(GaugeMergeRefusal::Join(reason), 0));
    }
    if adjacent_domains && left.domain()[1] != right.domain()[0] {
        return Ok(refused(
            GaugeMergeRefusal::Join(Refusal::NonAdjacentDomain),
            0,
        ));
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut compare = || {
        let a = exact_endpoint_jets(left, true, &mut work)?;
        let b = exact_endpoint_jets(right, false, &mut work)?;
        let mut differences = [Vec::new(), Vec::new(), Vec::new()];
        for ((a, b), residual) in a.into_iter().zip(b).zip(&mut differences) {
            for (a, b) in a.into_iter().zip(b) {
                residual.push(work.op(&a, &b, |a, b| a - b)?);
            }
        }
        Ok::<_, super::rational_factor::Refusal>(differences)
    };
    let residuals = match compare() {
        Ok(residuals) => residuals,
        Err(reason) => return Ok(refused(GaugeMergeRefusal::Exact(reason), work.used)),
    };
    let continuity_order = residuals
        .iter()
        .take_while(|row| row.iter().all(Zero::is_zero))
        .count()
        .checked_sub(1);
    Ok(ExactCurveSeamJetReport {
        residuals: Some(residuals),
        continuity_order,
        work_used: work.used,
        refusal: None,
    })
}

fn seam_geometry_incompatibility(left: &Curve, right: &Curve) -> Option<Refusal> {
    [
        (left.periodic || right.periodic, Refusal::Periodic),
        (
            left.control_points[0].len() != right.control_points[0].len(),
            Refusal::DimensionMismatch,
        ),
        (!clamped(left) || !clamped(right), Refusal::NonClamped),
    ]
    .into_iter()
    .find_map(|(incompatible, reason)| incompatible.then_some(reason))
}

fn exact_endpoint_jets(
    curve: &Curve,
    end: bool,
    work: &mut super::rational_edit::Work,
) -> std::result::Result<[Vec<num_rational::BigRational>; 3], super::rational_factor::Refusal> {
    use num_rational::BigRational as Q;
    use num_traits::{Signed, Zero};

    let p = curve.degree;
    let dimension = curve.control_points[0].len();
    let [lo, hi] = curve.domain();
    let endpoint = work.lift(if end { hi } else { lo })?;
    let span = |offset: usize, work: &mut super::rational_edit::Work| {
        let index = p + 1 + offset;
        let index = if end {
            curve.knots.len() - 1 - index
        } else {
            index
        };
        let knot = work.lift(curve.knots[index])?;
        let width = work.op(&knot, &endpoint, |a, b| (a - b).abs())?;
        if width.is_zero() {
            return Err(super::rational_factor::Refusal::InvalidOutput);
        }
        Ok(width)
    };
    let mut poles = Vec::new();
    for offset in 0..=p.min(2) {
        let index = if end {
            curve.control_points.len() - 1 - offset
        } else {
            offset
        };
        let weight = work.lift(curve.weights[index])?;
        let mut pole = Vec::new();
        for &coordinate in &curve.control_points[index] {
            let coordinate = work.lift(coordinate)?;
            pole.push(work.op(&coordinate, &weight, |a, b| a * b)?);
        }
        pole.push(weight);
        poles.push(pole);
    }
    let width = span(0, work)?;
    let scale = Q::from_integer(p.into()) / &width;
    let mut first = Vec::new();
    let mut second = vec![Q::zero(); dimension + 1];
    for axis in 0..=dimension {
        first.push(work.op(&poles[1][axis], &poles[0][axis], |a, b| (a - b) * &scale)?);
    }
    if p >= 2 {
        let next_scale = Q::from_integer(p.into()) / span(1, work)?;
        let second_scale = Q::from_integer((p - 1).into()) / &width;
        for axis in 0..=dimension {
            let next = work.op(&poles[2][axis], &poles[1][axis], |a, b| {
                (a - b) * &next_scale
            })?;
            second[axis] = work.op(&next, &first[axis], |a, b| (a - b) * &second_scale)?;
        }
    }
    if end {
        first.iter_mut().for_each(|value| *value = -value.clone());
    }
    let mut jets = [Vec::new(), Vec::new(), Vec::new()];
    let weight = &poles[0][dimension];
    for axis in 0..dimension {
        let position = work.op(&poles[0][axis], weight, |a, b| a / b)?;
        let correction = work.op(&position, &first[dimension], |a, b| a * b)?;
        let numerator = work.op(&first[axis], &correction, |a, b| a - b)?;
        let tangent = work.op(&numerator, weight, |a, b| a / b)?;
        let linear = work.op(&tangent, &first[dimension], |a, b| {
            a * b * Q::from_integer(2.into())
        })?;
        let quadratic = work.op(&position, &second[dimension], |a, b| a * b)?;
        let numerator = work.op(&second[axis], &linear, |a, b| a - b)?;
        let numerator = work.op(&numerator, &quadratic, |a, b| a - b)?;
        let acceleration = work.op(&numerator, weight, |a, b| a / b)?;
        jets[0].push(position);
        jets[1].push(tangent);
        jets[2].push(acceleration);
    }
    Ok(jets)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactCurveGeometricSeamReport {
    /// Highest verified geometric continuity order, up to G2.
    pub continuity_order: Option<usize>,
    /// Positive ratio of the right tangent to the left tangent for G1 or G2.
    pub speed_scale: Option<num_rational::BigRational>,
    /// Tangential correction for G2, in the left tangent basis.
    pub acceleration_scale: Option<num_rational::BigRational>,
    pub work_used: u64,
    pub refusal: Option<GaugeMergeRefusal>,
}

/// Prove geometric continuity between the end of the first clamped curve
/// and the start of the second, allowing unrelated parameter intervals.
/// Stationary endpoints can establish G0 but require higher-order analysis
/// to establish a tangent direction.
pub fn compare_exact_curve_geometric_seam(
    left: &Curve,
    right: &Curve,
    max_work: u64,
) -> Result<ExactCurveGeometricSeamReport> {
    use super::rational_edit::Work;
    use num_traits::{Signed, Zero};

    left.validate()?;
    right.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact geometric seam work must be in 1..=1000000",
    )?;
    let mut report = ExactCurveGeometricSeamReport {
        continuity_order: None,
        speed_scale: None,
        acceleration_scale: None,
        work_used: 0,
        refusal: None,
    };
    if let Some(reason) = seam_geometry_incompatibility(left, right) {
        report.refusal = Some(GaugeMergeRefusal::Join(reason));
        return Ok(report);
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut compare = || {
        let [a, da, dda] = exact_endpoint_jets(left, true, &mut work)?;
        let [b, db, ddb] = exact_endpoint_jets(right, false, &mut work)?;
        if a != b {
            return Ok(());
        }
        report.continuity_order = Some(0);
        let Some(axis) = da.iter().position(|value| !value.is_zero()) else {
            return Ok(());
        };
        let scale = work.op(&db[axis], &da[axis], |a, b| a / b)?;
        if !scale.is_positive() {
            return Ok(());
        }
        for (a, b) in da.iter().zip(db) {
            if work.op(a, &scale, |a, scale| a * scale)? != b {
                return Ok(());
            }
        }
        let squared = work.op(&scale, &scale, |a, b| a * b)?;
        report.continuity_order = Some(1);
        report.speed_scale = Some(scale);
        let mut difference = Vec::new();
        for (a, b) in dda.iter().zip(ddb) {
            let corrected = work.op(a, &squared, |a, square| a * square)?;
            difference.push(work.op(&b, &corrected, |a, b| a - b)?);
        }
        let acceleration = work.op(&difference[axis], &da[axis], |a, b| a / b)?;
        for (a, b) in da.iter().zip(difference) {
            if work.op(a, &acceleration, |a, acceleration| a * acceleration)? != b {
                return Ok(());
            }
        }
        report.continuity_order = Some(2);
        report.acceleration_scale = Some(acceleration);
        Ok::<_, super::rational_factor::Refusal>(())
    };
    if let Err(reason) = compare() {
        report.refusal = Some(GaugeMergeRefusal::Exact(reason));
    }
    report.work_used = work.used;
    Ok(report)
}

/// Merge clamped curves with a regular G1 seam into one curve with a C1 seam.
/// The left clock is retained. A positive affine map places the right curve
/// at the seam and matches its first derivative without changing its shape.
pub fn merge_with_exact_c1_clock(
    left: &Curve,
    right: &Curve,
    max_work: u64,
) -> Result<GaugeMergeReport> {
    merge_with_matching_clock(left, right, max_work, ClockMatch::AffineC1)
}

/// Merge clamped curves whose regular seam can reach C2 by a positive
/// affine change of the right parameter. The left clock stays fixed.
/// Geometric G2 with a nonzero tangential acceleration correction requires
/// a nonlinear change and is refused by this operation.
pub fn merge_with_exact_c2_clock(
    left: &Curve,
    right: &Curve,
    max_work: u64,
) -> Result<GaugeMergeReport> {
    merge_with_matching_clock(left, right, max_work, ClockMatch::AffineC2)
}

/// Match a regular G2 seam to C2 with an exact fractional-linear change
/// of the right clock. The map must be increasing and pole-free over the
/// whole right domain, with exactly storable knots and weights.
pub fn merge_with_exact_projective_c2_clock(
    left: &Curve,
    right: &Curve,
    max_work: u64,
) -> Result<GaugeMergeReport> {
    merge_with_matching_clock(left, right, max_work, ClockMatch::ProjectiveC2)
}

#[derive(Clone, Copy)]
enum ClockMatch {
    AffineC1,
    AffineC2,
    ProjectiveC2,
}

fn merge_with_matching_clock(
    left: &Curve,
    right: &Curve,
    max_work: u64,
    matching: ClockMatch,
) -> Result<GaugeMergeReport> {
    use num_traits::Zero;

    use super::{
        exact_projective::{
            affine_curve::scale_curve_clock, mobius_spline::match_curve_seam_clock,
        },
        rational_edit::Work,
    };

    let seam = compare_exact_curve_geometric_seam(left, right, max_work)?;
    let refused = |reason, work_used| GaugeMergeReport {
        curve: None,
        work_used,
        refusal: Some(reason),
    };
    if let Some(reason) = seam.refusal {
        return Ok(refused(reason, seam.work_used));
    }
    let acceleration = seam.acceleration_scale;
    let compatible = match matching {
        ClockMatch::AffineC1 => true,
        ClockMatch::AffineC2 => acceleration.as_ref().is_some_and(Zero::is_zero),
        ClockMatch::ProjectiveC2 => acceleration.is_some(),
    };
    if !compatible {
        return Ok(refused(
            GaugeMergeRefusal::Join(Refusal::SeamMismatch),
            seam.work_used,
        ));
    }
    let Some(scale) = seam.speed_scale else {
        return Ok(refused(
            GaugeMergeRefusal::Join(Refusal::SeamMismatch),
            seam.work_used,
        ));
    };
    let mut work = Work {
        used: seam.work_used,
        limit: max_work,
    };
    let start = left.domain()[1];
    let transformed = match acceleration
        .filter(|a| matches!(matching, ClockMatch::ProjectiveC2) && !a.is_zero())
    {
        Some(acceleration) => {
            match_curve_seam_clock(right, start, &scale, &acceleration, &mut work)
                .map_err(GaugeMergeRefusal::Clock)
        }
        None => scale_curve_clock(right, start, scale, &mut work).map_err(GaugeMergeRefusal::Exact),
    };
    let aligned = match transformed {
        Ok(curve) => curve,
        Err(reason) => return Ok(refused(reason, work.used)),
    };
    finish_clock_merge(left, aligned, work)
}

fn finish_clock_merge(
    left: &Curve,
    right: Curve,
    work: super::rational_edit::Work,
) -> Result<GaugeMergeReport> {
    if work.used == work.limit {
        return Ok(GaugeMergeReport {
            curve: None,
            work_used: work.used,
            refusal: Some(GaugeMergeRefusal::Exact(
                super::rational_factor::Refusal::ResourceLimit,
            )),
        });
    }
    let mut merged = merge_with_exact_degree_elevation(left, &right, work.limit - work.used)?;
    merged.work_used += work.used;
    Ok(merged)
}

/// Merge curves whose two seam tangents vanish. Match the second derivatives
/// by a positive affine right clock; if both second derivatives also vanish,
/// translate the right clock with unit scale.
pub fn merge_with_exact_stationary_c2_clock(
    left: &Curve,
    right: &Curve,
    max_work: u64,
) -> Result<GaugeMergeReport> {
    use super::{exact_projective::affine_curve::scale_curve_clock, rational_edit::Work};

    left.validate()?;
    right.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact stationary seam work must be in 1..=1000000",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut transform = || {
        if let Some(reason) = seam_geometry_incompatibility(left, right) {
            return Err(GaugeMergeRefusal::Join(reason));
        }
        let scale = stationary_seam_scale(left, right, &mut work)?;
        scale_curve_clock(right, left.domain()[1], scale, &mut work)
            .map_err(GaugeMergeRefusal::Exact)
    };
    match transform() {
        Ok(aligned) => finish_clock_merge(left, aligned, work),
        Err(reason) => Ok(GaugeMergeReport {
            curve: None,
            work_used: work.used,
            refusal: Some(reason),
        }),
    }
}

fn stationary_seam_scale(
    left: &Curve,
    right: &Curve,
    work: &mut super::rational_edit::Work,
) -> std::result::Result<num_rational::BigRational, GaugeMergeRefusal> {
    use super::rational_factor::Refusal as ExactRefusal;
    use num_rational::BigRational as Q;
    use num_traits::{One, Signed, Zero};

    let [a, da, dda] = exact_endpoint_jets(left, true, work).map_err(GaugeMergeRefusal::Exact)?;
    let [b, db, ddb] = exact_endpoint_jets(right, false, work).map_err(GaugeMergeRefusal::Exact)?;
    let mismatch = GaugeMergeRefusal::Join(Refusal::SeamMismatch);
    if a != b || da.iter().chain(&db).any(|value| !value.is_zero()) {
        return Err(mismatch);
    }
    let Some(axis) = dda.iter().position(|value| !value.is_zero()) else {
        return if ddb.iter().all(Zero::is_zero) {
            Ok(Q::one())
        } else {
            Err(mismatch)
        };
    };
    let solve = || {
        let ratio = work.op(&ddb[axis], &dda[axis], |a, b| a / b)?;
        if !ratio.is_positive() {
            return Ok(None);
        }
        for (a, b) in dda.iter().zip(ddb) {
            if work.op(a, &ratio, |a, ratio| a * ratio)? != b {
                return Ok(None);
            }
        }
        let numerator = work.int(ratio.numer(), ratio.numer(), |n, _| n.sqrt())?;
        let denominator = work.int(ratio.denom(), ratio.denom(), |d, _| d.sqrt())?;
        let scale = Q::new(numerator, denominator);
        if work.op(&scale, &scale, |a, b| a * b)? != ratio {
            return Err(ExactRefusal::NotRepresentable);
        }
        Ok(Some(scale))
    };
    solve().map_err(GaugeMergeRefusal::Exact)?.ok_or(mismatch)
}

#[derive(Clone, Debug)]
pub struct C2ClockChainReport {
    pub curve: Option<Curve>,
    pub joins_verified: usize,
    pub work_used: u64,
    /// Zero-based index of the right source at the first failed join.
    pub refusal: Option<(usize, GaugeMergeRefusal)>,
}

/// Concatenate an ordered chain with regular C2 seams, preserving each
/// source's shape. Each right clock is matched against the accumulated
/// curve. All joins share one budget; a failed join emits no partial curve.
pub fn merge_chain_with_exact_projective_c2_clock(
    sources: &[&Curve],
    max_work: u64,
) -> Result<C2ClockChainReport> {
    merge_projective_c2_chain(sources, max_work, &mut Vec::new())
}

fn merge_projective_c2_chain(
    sources: &[&Curve],
    max_work: u64,
    seams: &mut Vec<f64>,
) -> Result<C2ClockChainReport> {
    crate::check(
        (1..=256).contains(&sources.len()),
        "C2 curve chain needs 1..=256 sources",
    )?;
    let controls = sources
        .iter()
        .try_fold(0usize, |n, curve| n.checked_add(curve.control_points.len()));
    crate::check(
        controls.is_some_and(|total| total <= 512),
        "C2 curve chain source controls must not exceed 512",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "C2 curve chain work must be in 1..=1000000",
    )?;
    for source in sources {
        source.validate()?;
    }
    let mut curve = sources[0].clone();
    let mut work_used = 0;
    for (index, right) in sources.iter().enumerate().skip(1) {
        let joined = if work_used == max_work {
            GaugeMergeReport {
                curve: None,
                work_used: 0,
                refusal: Some(GaugeMergeRefusal::Exact(
                    super::rational_factor::Refusal::ResourceLimit,
                )),
            }
        } else {
            merge_with_exact_projective_c2_clock(&curve, right, max_work - work_used)?
        };
        work_used += joined.work_used;
        let Some(merged) = joined.curve else {
            return Ok(C2ClockChainReport {
                curve: None,
                joins_verified: index - 1,
                work_used,
                refusal: Some((
                    index,
                    joined
                        .refusal
                        .unwrap_or(GaugeMergeRefusal::Join(Refusal::InvalidOutput)),
                )),
            });
        };
        seams.push(curve.domain()[1]);
        curve = merged;
    }
    Ok(C2ClockChainReport {
        curve: Some(curve),
        joins_verified: sources.len() - 1,
        work_used,
        refusal: None,
    })
}

#[derive(Clone, Debug)]
pub struct CompactC2ClockChainReport {
    pub chain: C2ClockChainReport,
    /// One entry per successful structural seam, in input order.
    pub seams: Vec<SeamCompaction>,
}

/// Build a regular C2 chain and compact only its join knots. Each accepted
/// removal preserves the complete curve in its final parameter domain.
/// Failed removals retain the last verified representation and record the
/// reason per seam; a failed join yields no curve or compaction results.
pub fn merge_and_compact_exact_projective_c2_chain(
    sources: &[&Curve],
    max_work: u64,
) -> Result<CompactC2ClockChainReport> {
    let mut parameters = Vec::new();
    let mut chain = merge_projective_c2_chain(sources, max_work, &mut parameters)?;
    let mut seams = Vec::new();
    if let Some(curve) = chain.curve.as_mut() {
        for parameter in parameters {
            seams.push(compact_seam(
                curve,
                parameter,
                &mut chain.work_used,
                max_work,
            )?);
        }
    }
    Ok(CompactC2ClockChainReport { chain, seams })
}

#[derive(Clone, Debug)]
pub struct ClosedC2ClockChainReport {
    pub chain: C2ClockChainReport,
    /// End-minus-start residuals in the joined curve's absolute parameter.
    /// Absent if an internal join failed.
    pub closure: Option<ExactCurveSeamJetReport>,
}

/// Assemble a curve chain and require an exact C2 wrap at the final seam.
/// The closure test matches position and both derivatives at the two ends
/// in the final parameter clock. The result is a closed clamped curve.
pub fn merge_closed_exact_projective_c2_chain(
    sources: &[&Curve],
    max_work: u64,
) -> Result<ClosedC2ClockChainReport> {
    let mut chain = merge_chain_with_exact_projective_c2_clock(sources, max_work)?;
    let Some(curve) = chain.curve.as_ref() else {
        return Ok(ClosedC2ClockChainReport {
            chain,
            closure: None,
        });
    };
    let remaining = max_work - chain.work_used;
    let closure = if remaining == 0 {
        ExactCurveSeamJetReport {
            residuals: None,
            continuity_order: None,
            work_used: 0,
            refusal: Some(GaugeMergeRefusal::Exact(
                super::rational_factor::Refusal::ResourceLimit,
            )),
        }
    } else {
        compare_curve_jets(curve, curve, remaining, false)?
    };
    chain.work_used += closure.work_used;
    if closure.continuity_order != Some(2) {
        chain.curve = None;
    }
    Ok(ClosedC2ClockChainReport {
        chain,
        closure: Some(closure),
    })
}

/// Store a clamped curve with an exactly verified C2 wrap as a cyclic spline
/// on the same domain. The coincident endpoint poles must share one weight.
/// The seam retains degree multiplicity in the cyclic knot vector, so the
/// evaluator's knot-based continuity metadata remains conservative there.
pub fn store_exact_periodic_c2_curve(source: &Curve, max_work: u64) -> Result<GaugeMergeReport> {
    use super::rational_edit::Work;

    let seam = compare_curve_jets(source, source, max_work, false)?;
    let refused = |reason, work_used| GaugeMergeReport {
        curve: None,
        work_used,
        refusal: Some(reason),
    };
    if let Some(reason) = seam.refusal {
        return Ok(refused(reason, seam.work_used));
    }
    if seam.continuity_order != Some(2) || source.weights.first() != source.weights.last() {
        return Ok(refused(
            GaugeMergeRefusal::Join(Refusal::SeamMismatch),
            seam.work_used,
        ));
    }
    let mut work = Work {
        used: seam.work_used,
        limit: max_work,
    };
    let out = match repeat_clamped_cycle(source, &mut work) {
        Ok(curve) => curve,
        Err(reason) => return Ok(refused(GaugeMergeRefusal::Exact(reason), work.used)),
    };
    Ok(GaugeMergeReport {
        curve: Some(out),
        work_used: work.used,
        refusal: None,
    })
}

pub(super) fn repeat_clamped_cycle(
    source: &Curve,
    work: &mut super::rational_edit::Work,
) -> std::result::Result<Curve, super::rational_factor::Refusal> {
    use super::rational_factor::Refusal as ExactRefusal;
    use num_rational::BigRational as Q;

    let degree = source.degree;
    let cycle = source.control_points.len() - 1;
    if cycle <= degree || cycle + degree > 256 {
        return Err(ExactRefusal::InvalidOutput);
    }
    let [lo, hi] = source.domain();
    let start = work.lift(lo)?;
    let end = work.lift(hi)?;
    let width = work.op(&end, &start, |a, b| a - b)?;
    let pattern = &source.knots[1..=cycle];
    let mut knots = Vec::with_capacity(cycle + 2 * degree + 1);
    for index in 0..=cycle + 2 * degree {
        let relative = index as isize - 1;
        let turn = relative.div_euclid(cycle as isize);
        let knot = pattern[relative.rem_euclid(cycle as isize) as usize];
        if turn == 0 {
            knots.push(knot);
        } else {
            let value = work.lift(knot)?;
            let shift = Q::from_integer(turn.into());
            let mapped = work.op(&value, &width, |a, b| a + &shift * b)?;
            knots.push(work.float(&mapped)?);
        }
    }
    // Each nonempty active span i uses the controls i-p..=i and
    // de Boor knots i-p+1..=i+p. They are all in the unchanged control
    // prefix and the knot window strictly between the two exterior ends
    // of the clamped vector. Thus the stored cyclic law is identical
    // on each span, including the one-sided endpoint limits.
    let active_window = 1..source.knots.len() - 1;
    work.charge(active_window.len() as u64)?;
    if knots[active_window.clone()] != source.knots[active_window] {
        return Err(ExactRefusal::DifferentDefinition);
    }
    let mut curve = source.clone();
    curve.periodic = true;
    curve.knots = knots;
    curve
        .control_points
        .extend_from_slice(&source.control_points[1..degree]);
    curve.weights.extend_from_slice(&source.weights[1..degree]);
    curve.validate().map_err(|_| ExactRefusal::InvalidOutput)?;
    Ok(curve)
}

/// Rotate a periodic spline to an existing knot in its half-open domain.
/// All translated knots must be exactly representable. The returned domain
/// starts at the new seam and retains the original period and parameter units.
pub fn rotate_periodic_curve_at_exact_knot(
    source: &Curve,
    parameter: f64,
    max_work: u64,
) -> Result<GaugeMergeReport> {
    use super::{rational_edit::Work, rational_factor::Refusal as ExactRefusal};

    source.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact periodic rotation work must be in 1..=1000000",
    )?;
    let refused = |reason, work_used| GaugeMergeReport {
        curve: None,
        work_used,
        refusal: Some(reason),
    };
    if !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(source)? {
        return Ok(refused(GaugeMergeRefusal::Join(Refusal::InvalidOutput), 0));
    }
    let degree = source.degree;
    let n = source.control_points.len();
    let Some(shift) = source.knots[degree..n].iter().position(|&k| k == parameter) else {
        return Ok(refused(GaugeMergeRefusal::Join(Refusal::SeamMismatch), 0));
    };
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    match rotate_periodic_storage(source, shift, &mut work) {
        Err(ExactRefusal::InvalidOutput) => Ok(refused(
            GaugeMergeRefusal::Join(Refusal::InvalidOutput),
            work.used,
        )),
        Ok(rotated) => Ok(GaugeMergeReport {
            curve: Some(rotated),
            work_used: work.used,
            refusal: None,
        }),
        Err(reason) => Ok(refused(GaugeMergeRefusal::Exact(reason), work.used)),
    }
}

fn rotate_periodic_storage(
    source: &Curve,
    shift: usize,
    work: &mut super::rational_edit::Work,
) -> std::result::Result<Curve, super::rational_factor::Refusal> {
    use super::rational_factor::Refusal as ExactRefusal;

    let n = source.control_points.len();
    let cycle = n - source.degree;
    let mut rotated = source.clone();
    let [lo, hi] = source.domain();
    let lo = work.lift(lo)?;
    let hi = work.lift(hi)?;
    let period = work.op(&hi, &lo, |a, b| a - b)?;
    for (i, knot) in rotated.knots.iter_mut().enumerate() {
        let index = i + shift;
        // Keep all overlapping knots literal and translate only the
        // newly exposed suffix of the infinite periodic knot sequence.
        *knot = if index < source.knots.len() {
            source.knots[index]
        } else {
            let base = work.lift(source.knots[index - cycle])?;
            let mapped = work.op(&base, &period, |a, b| a + b)?;
            work.float(&mapped)?
        };
    }
    for i in 0..n {
        let original = (i + shift) % cycle;
        rotated.control_points[i].clone_from(&source.control_points[original]);
        rotated.weights[i] = source.weights[original];
    }
    rotated
        .validate()
        .map_err(|_| ExactRefusal::InvalidOutput)?;
    Ok(rotated)
}

/// Rotate a periodic spline to a seam in the active domain. If the seam
/// is inside a span, insert one exact cyclic Boehm knot first. Refuse any
/// knot or homogeneous net that cannot be stored without changing the law.
pub fn insert_and_rotate_periodic_curve_seam(
    source: &Curve,
    parameter: f64,
    max_work: u64,
) -> Result<GaugeMergeReport> {
    use super::rational_edit::Work;

    source.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact periodic seam work must be in 1..=1000000",
    )?;
    let [lo, hi] = source.domain();
    if !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(source)? {
        return Ok(GaugeMergeReport {
            curve: None,
            work_used: 0,
            refusal: Some(GaugeMergeRefusal::Join(Refusal::InvalidOutput)),
        });
    }
    if !(lo <= parameter && parameter < hi) {
        return Ok(GaugeMergeReport {
            curve: None,
            work_used: 0,
            refusal: Some(GaugeMergeRefusal::Join(Refusal::SeamMismatch)),
        });
    }
    if source.knots.contains(&parameter) {
        return rotate_periodic_curve_at_exact_knot(source, parameter, max_work);
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = insert_periodic_seam(source, parameter, &mut work);
    Ok(match result {
        Ok(curve) => GaugeMergeReport {
            curve: Some(curve),
            work_used: work.used,
            refusal: None,
        },
        Err(reason) => GaugeMergeReport {
            curve: None,
            work_used: work.used,
            refusal: Some(GaugeMergeRefusal::Exact(reason)),
        },
    })
}

fn insert_periodic_seam(
    source: &Curve,
    parameter: f64,
    work: &mut super::rational_edit::Work,
) -> std::result::Result<Curve, super::rational_factor::Refusal> {
    use super::{rational_edit::store, rational_factor::Refusal};
    use num_rational::BigRational as Q;

    let p = source.degree;
    let (net, knots) = refine_periodic_seam_net(source, parameter, work)?;
    let channels: Vec<Vec<Q>> = (0..net[0].len())
        .map(|axis| net.iter().map(|row| row[axis].clone()).collect())
        .collect();
    let (mut points, mut weights) = store(&channels, work)?;
    points.extend_from_within(..p);
    weights.extend_from_within(..p);

    let refined = Curve {
        degree: p,
        knots,
        control_points: points,
        weights,
        periodic: true,
    };
    refined.validate().map_err(|_| Refusal::InvalidOutput)?;
    rotate_periodic_storage(&refined, 1, work)
}

pub(super) fn refine_periodic_seam_net(
    source: &Curve,
    parameter: f64,
    work: &mut super::rational_edit::Work,
) -> std::result::Result<
    (Vec<Vec<num_rational::BigRational>>, Vec<f64>),
    super::rational_factor::Refusal,
> {
    use super::{exact_knot_insertion::homogeneous, rational_factor::Refusal};
    use num_rational::BigRational as Q;

    let p = source.degree;
    let n = source.control_points.len();
    if n >= 256 {
        return Err(Refusal::InvalidOutput);
    }
    let span = source.knots.partition_point(|&k| k < parameter) - 1;
    // Pin the last occurrence of the preceding knot at index p so the
    // insertion changes only the first p cyclic controls.
    let base = rotate_periodic_storage(source, span - p, work)?;
    let (net, linear_knots) = homogeneous(&base, parameter, 1, work)?;
    let cycle = n - p + 1;
    let [lo, hi] = base.domain();
    let start = work.lift(lo)?;
    let end = work.lift(hi)?;
    let period = work.op(&end, &start, |a, b| a - b)?;
    let active = &linear_knots[p..p + cycle];
    let knots = (0..cycle + 2 * p + 1)
        .map(|i| {
            let index = i as isize - p as isize;
            let turn = index.div_euclid(cycle as isize);
            let knot = active[index.rem_euclid(cycle as isize) as usize];
            if turn == 0 {
                return Ok(knot);
            }
            let value = work.lift(knot)?;
            let turn = Q::from_integer(turn.into());
            let mapped = work.op(&value, &period, |a, b| a + &turn * b)?;
            Ok(work.float(&mapped)?)
        })
        .collect::<std::result::Result<Vec<_>, Refusal>>()?;
    Ok((net.into_iter().take(cycle).collect(), knots))
}

/// Open one complete period at the current seam by exactly clamping the
/// two endpoint spans. The active interval and interior knots stay fixed.
pub fn open_exact_periodic_curve(source: &Curve, max_work: u64) -> Result<GaugeMergeReport> {
    use super::rational_edit::Work;

    source.validate()?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact periodic opening work must be in 1..=1000000",
    )?;
    if !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(source)? {
        return Ok(GaugeMergeReport {
            curve: None,
            work_used: 0,
            refusal: Some(GaugeMergeRefusal::Join(Refusal::InvalidOutput)),
        });
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = clamp_curve_interval(source, source.domain(), &mut work);
    Ok(match result {
        Ok(curve) => GaugeMergeReport {
            curve: Some(curve),
            work_used: work.used,
            refusal: None,
        },
        Err(reason) => GaugeMergeReport {
            curve: None,
            work_used: work.used,
            refusal: Some(GaugeMergeRefusal::Exact(reason)),
        },
    })
}

/// Store the complete active interval of a nonperiodic curve in clamped form.
/// Refuses when the exact homogeneous restriction has no binary64 storage.
pub fn clamp_exact_curve(source: &Curve, max_work: u64) -> Result<GaugeMergeReport> {
    clamp_exact_curve_impl(source, None, max_work)
}

/// Store an exact clamped restriction on the original nonperiodic clock.
pub fn clamp_exact_curve_on(
    source: &Curve,
    range: [f64; 2],
    max_work: u64,
) -> Result<GaugeMergeReport> {
    clamp_exact_curve_impl(source, Some(range), max_work)
}

fn clamp_exact_curve_impl(
    source: &Curve,
    range: Option<[f64; 2]>,
    max_work: u64,
) -> Result<GaugeMergeReport> {
    let report = crate::core::exact_curve_arithmetic::clamp_active(source, range, max_work)?;
    Ok(GaugeMergeReport {
        curve: report.curve,
        work_used: report.work_used,
        refusal: report.refusal.map(GaugeMergeRefusal::Exact),
    })
}

pub(super) use crate::core::exact_curve_arithmetic::clamp_curve_net;
use crate::core::exact_curve_arithmetic::clamp_curve_interval;

/// Extract a clamped subcurve from one lifted period. The start lies in the
/// original half-open domain and the end is expressed on that same unwrapped
/// clock, possibly beyond the original seam. A full turn is allowed.
pub fn restrict_exact_periodic_curve(
    source: &Curve,
    interval: [f64; 2],
    max_work: u64,
) -> Result<GaugeMergeReport> {
    restrict_periodic_interval(source, interval, max_work, false)
}

/// Extract a clamped NURBS segment over multiple periods, with the start in
/// the source's half-open domain. Extend the exact cyclic basis until the
/// requested end is covered, subject to the curve control and work limits.
pub fn restrict_exact_periodic_curve_over_turns(
    source: &Curve,
    interval: [f64; 2],
    max_work: u64,
) -> Result<GaugeMergeReport> {
    restrict_periodic_interval(source, interval, max_work, true)
}

fn restrict_periodic_interval(
    source: &Curve,
    interval: [f64; 2],
    max_work: u64,
    allow_multiple_turns: bool,
) -> Result<GaugeMergeReport> {
    use super::rational_edit::Work;

    let [lo, hi] = interval;
    crate::check(
        hi.is_finite() && lo < hi,
        "Periodic restriction needs an increasing finite interval",
    )?;
    let rotated = insert_and_rotate_periodic_curve_seam(source, lo, max_work)?;
    let mut work = Work {
        used: rotated.work_used,
        limit: max_work,
    };
    let Some(mut curve) = rotated.curve else {
        return Ok(rotated);
    };
    if !allow_multiple_turns && hi > curve.domain()[1] {
        return Ok(GaugeMergeReport {
            curve: None,
            work_used: work.used,
            refusal: Some(GaugeMergeRefusal::Join(Refusal::NonAdjacentDomain)),
        });
    }
    let out = extend_periodic_support(&mut curve, hi, &mut work)
        .and_then(|()| clamp_curve_interval(&curve, interval, &mut work));
    Ok(match out {
        Ok(curve) => GaugeMergeReport {
            curve: Some(curve),
            work_used: work.used,
            refusal: None,
        },
        Err(reason) => GaugeMergeReport {
            curve: None,
            work_used: work.used,
            refusal: Some(GaugeMergeRefusal::Exact(reason)),
        },
    })
}

fn extend_periodic_support(
    curve: &mut Curve,
    end: f64,
    work: &mut super::rational_edit::Work,
) -> std::result::Result<(), super::rational_factor::Refusal> {
    use super::rational_factor::Refusal as ExactRefusal;

    let [lo, hi] = curve.domain();
    if end <= hi {
        return Ok(());
    }
    let cycle = curve.control_points.len() - curve.degree;
    let lo = work.lift(lo)?;
    let hi = work.lift(hi)?;
    let period = work.op(&hi, &lo, |a, b| a - b)?;
    // The extension is a finite window of the infinite cyclic basis. It
    // has more active controls than one period, so it is not cyclic storage.
    curve.periodic = false;
    while end > curve.domain()[1] {
        let count = curve.control_points.len();
        if count == 256 {
            return Err(ExactRefusal::InvalidOutput);
        }
        let index = curve.knots.len() - cycle;
        let prior = work.lift(curve.knots[index])?;
        let shifted = work.op(&prior, &period, |a, b| a + b)?;
        curve.knots.push(work.float(&shifted)?);
        curve
            .control_points
            .push(curve.control_points[count % cycle].clone());
        curve.weights.push(curve.weights[count % cycle]);
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct PeriodicPartitionReport {
    pub pieces: Option<Vec<Curve>>,
    pub work_used: u64,
    /// Zero-based interval index. Absent if seaming itself failed.
    pub refused_piece: Option<usize>,
    pub refusal: Option<GaugeMergeRefusal>,
}

/// Open one period at the chosen seam and extract each successive clamped
/// interval with its own exact weight gauge. Cuts use the lifted parameter
/// clock and must strictly increase inside the new period. A failed piece
/// withholds the entire partition.
pub fn partition_exact_periodic_curve(
    source: &Curve,
    seam: f64,
    cuts: &[f64],
    max_work: u64,
) -> Result<PeriodicPartitionReport> {
    use super::rational_edit::Work;

    crate::check(
        cuts.len() < 256,
        "Periodic partition supports at most 256 pieces",
    )?;
    let rotated = insert_and_rotate_periodic_curve_seam(source, seam, max_work)?;
    let Some(curve) = rotated.curve else {
        return Ok(PeriodicPartitionReport {
            pieces: None,
            work_used: rotated.work_used,
            refused_piece: None,
            refusal: rotated.refusal,
        });
    };
    let [start, end] = curve.domain();
    let mut prior = start;
    for &cut in cuts {
        crate::check(
            prior < cut && cut < end,
            "Periodic partition cuts must increase inside the lifted period",
        )?;
        prior = cut;
    }

    let mut work = Work {
        used: rotated.work_used,
        limit: max_work,
    };
    let mut pieces = Vec::with_capacity(cuts.len() + 1);
    let mut left = start;
    for (index, right) in cuts.iter().copied().chain([end]).enumerate() {
        match clamp_curve_interval(&curve, [left, right], &mut work) {
            Ok(piece) => pieces.push(piece),
            Err(reason) => {
                return Ok(PeriodicPartitionReport {
                    pieces: None,
                    work_used: work.used,
                    refused_piece: Some(index),
                    refusal: Some(GaugeMergeRefusal::Exact(reason)),
                });
            }
        }
        left = right;
    }
    Ok(PeriodicPartitionReport {
        pieces: Some(pieces),
        work_used: work.used,
        refused_piece: None,
        refusal: None,
    })
}

/// Join an ordered mixed-degree chain on its original parameter intervals,
/// then remove provably redundant knots and degrees from the complete profile.
/// Only a completed result is emitted. Every intermediate representation must
/// fit the storage limits and the same shared proof budget.
pub fn compress_mixed_degree_chain(sources: &[Curve], max_work: u64) -> Result<GaugeMergeReport> {
    crate::check((2..=256).contains(&sources.len()), "Expected 2..=256 curve segments")?;
    crate::check((1..=cad_predicates::MAX_WORK).contains(&max_work), "Invalid chain compression work limit")?;
    for source in sources { source.validate()?; }
    let exhausted = |work_used| GaugeMergeReport {
        curve: None, work_used, refusal: Some(GaugeMergeRefusal::Exact(super::rational_factor::Refusal::ResourceLimit)),
    };
    let mut current = sources[0].clone();
    let mut used = 0;
    for next in &sources[1..] {
        if used == max_work { return Ok(exhausted(used)); }
        let mut join = merge_with_exact_degree_elevation(&current, next, max_work - used)?;
        used += join.work_used;
        let Some(curve) = join.curve.take() else { join.work_used = used; return Ok(join); };
        current = curve;
    }
    if used == max_work { return Ok(exhausted(used)); }
    let compressed = super::exact_knot_removal::compress_curve(&current, max_work - used)?;
    used += compressed.work_used;
    if !compressed.completed { return Ok(exhausted(used)); }
    Ok(GaugeMergeReport { curve: Some(compressed.curve), work_used: used, refusal: None })
}

/// Keep the first segment's orientation and the supplied segment order.
/// Orient each subsequent open segment by its unique matching endpoint,
/// reflecting its absolute knot interval exactly when necessary. Refuses
/// ambiguous joins and reflected knots that cannot be stored in binary64.
pub fn orient_and_compress_chain(sources: &[Curve], max_work: u64) -> Result<GaugeMergeReport> {
    use super::{rational_edit::Work, rational_factor::Refusal as Exact};
    crate::check((2..=256).contains(&sources.len()), "Expected 2..=256 curve segments")?;
    crate::check((1..=cad_predicates::MAX_WORK).contains(&max_work), "Invalid orientation work limit")?;
    for curve in sources { curve.validate()?; }
    let mut work = Work { used: 0, limit: max_work };
    let mut oriented = vec![sources[0].clone()];
    for next in &sources[1..] {
        let end = oriented.last().unwrap().control_points.last();
        let forward = next.control_points.first() == end;
        let backward = next.control_points.last() == end;
        crate::check(!(forward && backward), "Both endpoints match; segment orientation is ambiguous")?;
        if !forward && !backward {
            return Ok(GaugeMergeReport { curve: None, work_used: work.used, refusal: Some(GaugeMergeRefusal::Join(Refusal::SeamMismatch)) });
        }
        let mut curve = next.clone();
        if backward {
            if let Err(reason) = reflect_segment(&mut curve, &mut work) {
                return Ok(GaugeMergeReport { curve: None, work_used: work.used, refusal: Some(GaugeMergeRefusal::Exact(reason)) });
            }
        }
        oriented.push(curve);
    }
    if work.used == max_work {
        return Ok(GaugeMergeReport {curve: None, work_used: work.used, refusal: Some(GaugeMergeRefusal::Exact(Exact::ResourceLimit))});
    }
    let mut report = compress_mixed_degree_chain(&oriented, max_work-work.used)?;
    report.work_used += work.used;
    Ok(report)
}

fn reflect_segment(curve: &mut Curve, work: &mut super::rational_edit::Work)
    -> std::result::Result<(), super::rational_factor::Refusal> {
                let [a,b] = curve.domain();
                let left = work.lift(a)?;
                let right = work.lift(b)?;
                let sum = work.op(&left,&right,|x,y| x+y)?;
                for knot in &mut curve.knots {
                    let old = work.lift(*knot)?;
                    let new = work.op(&sum,&old,|x,y| x-y)?;
                    *knot = work.float(&new)?;
                }
    curve.knots.reverse();
    curve.weights.reverse();
    curve.control_points.reverse();
    Ok(())
}

/// Recover a chain from unordered records with contiguous absolute domains.
/// Infer the first orientation from the next segment's endpoints. Ambiguous
/// initial joins, including a two-piece loop sharing both ends, need an anchor.
pub fn assemble_parameter_partition(sources: &[Curve], max_work: u64) -> Result<GaugeMergeReport> {
    crate::check((2..=256).contains(&sources.len()), "Expected 2..=256 partition segments")?;
    crate::check((1..=cad_predicates::MAX_WORK).contains(&max_work), "Invalid partition work limit")?;
    for source in sources { source.validate()?; }
    let mut ordered = sources.to_vec();
    ordered.sort_by(|a,b| a.domain()[0].total_cmp(&b.domain()[0]));
    if ordered.windows(2).any(|p| p[0].domain()[1] != p[1].domain()[0]) {
        return Ok(GaugeMergeReport {curve: None, work_used: 0, refusal: Some(GaugeMergeRefusal::Join(Refusal::NonAdjacentDomain))});
    }
    let peer = &ordered[1].control_points;
    let matches = |point| peer.first() == point || peer.last() == point;
    let first = &ordered[0].control_points;
    let forward = matches(first.last());
    let backward = matches(first.first());
    crate::check(!(forward && backward), "Ambiguous partition orientation")?;
    let mut work = super::rational_edit::Work { used: 0, limit: max_work };
    if backward {
        if let Err(reason) = reflect_segment(&mut ordered[0], &mut work) {
            return Ok(GaugeMergeReport { curve: None, work_used: work.used, refusal: Some(GaugeMergeRefusal::Exact(reason)) });
        }
    }
    if work.used == max_work {
        return Ok(GaugeMergeReport { curve: None, work_used: work.used, refusal: Some(GaugeMergeRefusal::Exact(super::rational_factor::Refusal::ResourceLimit)) });
    }
    let mut result = orient_and_compress_chain(&ordered, max_work-work.used)?;
    result.work_used += work.used;
    Ok(result)
}

mod endpoint_chain;
pub use endpoint_chain::{assemble_closed_endpoint_chain, assemble_open_endpoint_chain, assemble_simple_planar_loop, assemble_wound_planar_loop, assemble_simple_spatial_loop, assemble_oblique_simple_loop, assemble_pairwise_simple_spatial_loop};
