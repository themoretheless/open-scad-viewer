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
    use super::exact_knot_removal::{Refusal as RemovalRefusal, remove};

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
    let seam = left.domain()[1];
    let mut multiplicity = curve.knots.iter().filter(|&&knot| knot == seam).count();
    let mut removal_refusal = None;
    while multiplicity > 0 {
        if work_used == max_work {
            removal_refusal = Some(RemovalRefusal::ResourceLimit);
            break;
        }
        let report = remove(&curve, seam, max_work - work_used)?;
        work_used += report.work_used;
        if !report.removed {
            removal_refusal = report.refusal;
            break;
        }
        curve = report.curve;
        multiplicity -= 1;
    }
    Ok(CompactSeamMergeReport {
        curve: Some(curve),
        seam_multiplicity: Some(multiplicity),
        work_used,
        join_refusal: None,
        removal_refusal,
    })
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
    if left.domain()[1] != right.domain()[0] {
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
        curve = merged;
    }
    Ok(C2ClockChainReport {
        curve: Some(curve),
        joins_verified: sources.len() - 1,
        work_used,
        refusal: None,
    })
}
