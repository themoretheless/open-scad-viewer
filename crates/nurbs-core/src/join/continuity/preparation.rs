//! Open and periodic seam basis preparation. Candidates from `prepare` require
//! the whole-surface error certificate from `certify`; `checked` returns the
//! original pair whenever that acceptance gate fails.
use super::{Boundary, normalized};
use crate::{
    Result, check,
    curve::Curve,
    foundation::{periodic_knots, refit_curve, refit_periodic},
    surface::{Axis, Surface},
};

#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
pub use serialization::{certify, checked, checked_with_conversion};

pub enum PreparationReason {
    UnprovenParameterNormalization,
    DeviationExceedsBudget,
    Accepted,
}
pub struct PreparationCertificate {
    pub accepted: bool,
    pub reason: PreparationReason,
    pub budget: f64,
    pub errors: Option<[f64; 2]>,
}
pub struct PreparedBasis {
    pub degree: usize,
    pub control_count: usize,
    pub edited_reversed: bool,
    pub reference_domain: [f64; 2],
    pub edited_domain: [f64; 2],
    pub periodicity_removed: [bool; 2],
}
pub struct CheckedPreparation {
    pub reference: Surface,
    pub edited: Surface,
    pub report: PreparationCertificate,
    pub basis: PreparedBasis,
}

pub struct PreparedSeams {
    pub reference: Surface,
    pub edited: Surface,
    pub degree: usize,
    pub control_count: usize,
    pub reference_domain: [f64; 2],
    pub edited_domain: [f64; 2],
    pub edited_reversed: bool,
}
fn axis(boundary: Boundary) -> Axis {
    if boundary.cross_u { Axis::V } else { Axis::U }
}
fn normalize(source: &Surface, boundary: Boundary, reverse: bool) -> Result<(Surface, [f64; 2])> {
    let (p, n, k, periodic) = boundary.along(source);
    let domain = [k[p], k[n]];
    check(
        periodic
            || k[..=p].iter().all(|&x| x == domain[0]) && k[n..].iter().all(|&x| x == domain[1]),
        "Seam preparation requires clamped seam endpoints",
    )?;
    let knots = normalized(k, p, n)?;
    check(
        k.windows(2)
            .zip(knots.windows(2))
            .all(|(a, b)| a[0] == a[1] || b[0] < b[1]),
        "Seam normalization collapsed distinct knots",
    )?;
    let result = source.edit_axis(axis(boundary), |curve| {
        let mut result = curve.clone();
        result.knots = knots.clone();
        if reverse {
            result = result.reverse()?;
        }
        Ok(result)
    })?;
    Ok((result, domain))
}
fn elevated_multiplicities(surface: &Surface, b: Boundary, degree: usize) -> Vec<(f64, usize)> {
    let (p, n, knots, periodic) = b.along(surface);
    let knots = if periodic { &knots[p..n] } else { knots };
    let mut groups: Vec<(f64, usize)> = Vec::new();
    for &k in knots {
        if let Some(last) = groups.last_mut() {
            if last.0 == k {
                last.1 += 1;
                continue;
            }
        }
        groups.push((k, 1));
    }
    for (_, count) in &mut groups {
        *count += degree - p;
    }
    groups
}
/// Builds a common normalized seam basis without artificially reducing its
/// continuity to C0. Returned surfaces are candidates, not accepted edits.
/// Cross directions and input definitions remain unchanged.
pub fn prepare(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    reverse: bool,
) -> Result<PreparedSeams> {
    reference.validate()?;
    edited.validate()?;
    let r = Boundary::parse(reference_boundary)?;
    let e = Boundary::parse(edited_boundary)?;
    let periodic = r.along(reference).3;
    check(
        periodic == e.along(edited).3,
        "Seam preparation requires matching periodicity; convert the seam explicitly first",
    )?;
    let (reference, reference_domain) = normalize(reference, r, false)?;
    let (edited, edited_domain) = normalize(edited, e, reverse)?;
    let degree = r.along(&reference).0.max(e.along(&edited).0);
    let mut groups = elevated_multiplicities(&reference, r, degree);
    for (k, count) in elevated_multiplicities(&edited, e, degree) {
        if let Some(entry) = groups.iter_mut().find(|entry| entry.0 == k) {
            entry.1 = entry.1.max(count);
        } else {
            groups.push((k, count));
        }
    }
    groups.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut knots: Vec<f64> = groups
        .iter()
        .flat_map(|&(k, count)| std::iter::repeat_n(k, count))
        .collect();
    if periodic {
        knots.push(1.);
    }
    let active = knots.clone();
    if periodic {
        check(
            active.len() > degree + 1,
            "Periodic aligned basis has too few distinct controls",
        )?;
        knots = periodic_knots(&active, degree);
    }
    let control_count = knots
        .len()
        .checked_sub(degree + 1)
        .ok_or_else(|| crate::input("Invalid aligned seam basis"))?;
    check(
        control_count <= 256,
        "Aligned seam exceeds 256 control points",
    )?;
    let fit = |curve: &Curve| {
        if curve.degree == degree && curve.knots == knots {
            Ok(curve.clone())
        } else if periodic {
            refit_periodic(curve, degree, active.clone())
        } else {
            refit_curve(curve, degree, knots.clone())
        }
    };
    let reference = reference.edit_axis(axis(r), fit)?;
    let edited = edited.edit_axis(axis(e), fit)?;
    Ok(PreparedSeams {
        reference,
        edited,
        degree,
        control_count,
        reference_domain,
        edited_domain,
        edited_reversed: reverse,
    })
}

/// Certifies positional drift for a prepared pair. Exact predicates guard
/// domain normalization; a rounded non-affine knot map is not certified.
pub fn certify_report(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    candidate: &PreparedSeams,
    budget: f64,
) -> Result<PreparationCertificate> {
    check(
        budget.is_finite() && budget >= 0.,
        "Preparation budget must be finite and nonnegative",
    )?;
    let mut errors = Vec::new();
    for (source, target, boundary, reverse) in [
        (reference, &candidate.reference, reference_boundary, false),
        (
            edited,
            &candidate.edited,
            edited_boundary,
            candidate.edited_reversed,
        ),
    ] {
        let boundary = Boundary::parse(boundary)?;
        let (normalized, domain) = normalize(source, boundary, reverse)?;
        let (p, n, k, _) = boundary.along(source);
        let (_, _, mapped, _) = boundary.along(&normalized);
        let mapped: Vec<f64> = if reverse {
            mapped.iter().rev().copied().collect()
        } else {
            mapped.to_vec()
        };
        if super::affine_knots(
            k,
            [k[p], k[n]],
            &mapped,
            if reverse { [1., 0.] } else { [0., 1.] },
        )
        .is_err()
        {
            return Ok(PreparationCertificate {
                accepted: false,
                reason: PreparationReason::UnprovenParameterNormalization,
                budget,
                errors: None,
            });
        }
        check(domain[0] < domain[1], "Invalid preparation source domain")?;
        errors.push(super::deviation::positional_upper(&normalized, target)?);
    }
    let accepted = errors.iter().all(|&e| e <= budget);
    Ok(PreparationCertificate {
        accepted,
        reason: if accepted {
            PreparationReason::Accepted
        } else {
            PreparationReason::DeviationExceedsBudget
        },
        errors: Some([errors[0], errors[1]]),
        budget,
    })
}

/// Application boundary: failed proof returns the original pair.
pub fn checked_report(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    reverse: bool,
    budget: f64,
) -> Result<CheckedPreparation> {
    checked_with_conversion_report(
        reference,
        edited,
        reference_boundary,
        edited_boundary,
        reverse,
        budget,
        false,
    )
}

/// Explicitly removes periodic storage while preserving the whole active patch.
/// The final proof compares directly with the original pair, including rounding
/// from clamping, normalization and basis fitting.

pub fn checked_with_conversion_report(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    reverse: bool,
    budget: f64,
    open_periodic: bool,
) -> Result<CheckedPreparation> {
    reference.validate()?;
    edited.validate()?;
    check(
        budget.is_finite() && budget >= 0.,
        "Preparation budget must be finite and nonnegative",
    )?;
    let open = |surface: &Surface, name: &str| -> Result<Surface> {
        let boundary = Boundary::parse(name)?;
        if !open_periodic || !boundary.along(surface).3 {
            return Ok(surface.clone());
        }
        surface.edit_axis(axis(boundary), |curve| {
            let [a, b] = curve.domain();
            curve.trim(a, b)
        })
    };
    let a = open(reference, reference_boundary)?;
    let b = open(edited, edited_boundary)?;
    let candidate = prepare(&a, &b, reference_boundary, edited_boundary, reverse)?;
    let removed_reference = Boundary::parse(reference_boundary)?.along(reference).3
        && !Boundary::parse(reference_boundary)?
            .along(&candidate.reference)
            .3;
    let removed_edited = Boundary::parse(edited_boundary)?.along(edited).3
        && !Boundary::parse(edited_boundary)?.along(&candidate.edited).3;
    let proof = certify_report(
        reference,
        edited,
        reference_boundary,
        edited_boundary,
        &candidate,
        budget,
    )?;
    let accepted = proof.accepted;
    Ok(CheckedPreparation {
        reference: if accepted {
            candidate.reference
        } else {
            reference.clone()
        },
        edited: if accepted {
            candidate.edited
        } else {
            edited.clone()
        },
        report: proof,
        basis: PreparedBasis {
            degree: candidate.degree,
            control_count: candidate.control_count,
            edited_reversed: reverse,
            reference_domain: candidate.reference_domain,
            edited_domain: candidate.edited_domain,
            periodicity_removed: [accepted && removed_reference, accepted && removed_edited],
        },
    })
}

#[cfg(test)]
#[path = "preparation/tests.rs"]
mod tests;
