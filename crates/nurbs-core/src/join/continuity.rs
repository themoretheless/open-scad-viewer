//! Boundary jet matching in homogeneous coordinates. The basis along the seam
//! must agree after affine parameter normalization. Regularity of each seam
//! is checked separately with outward-rounded Bernstein normal enclosures.
//! Representation reference: MIT Hyperbook, sections 1.4.2 and 1.5.
use crate::{Result, check, curve::basis, surface::Surface};
mod report;
pub use report::*;
#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
pub use serialization::{
    match_surface_jets, match_surface_jets_checked, match_surface_jets_oriented,
};
mod bounds;
mod exact_strip;
mod projective_collection;
pub use exact_strip::{
    ExactStripJetReport, inspect_surface_exact_strip_jets, inspect_surface_projective_strip_jets,
};
pub use projective_collection::{
    ProjectiveSeam, ProjectiveSeamCollectionReport, inspect_exact_seam_collection,
    inspect_projective_seam_collection,
};
pub mod curve_match;
pub mod deviation;
pub mod preparation;
mod regularity;
mod station_scale;
pub(crate) use exact_strip::inspect_proposed_station_projective_strip_jets;
pub(crate) use exact_strip::inspect_station_projective_strip_jets;
pub use projective_collection::inspect_projective_seam_collection_until_refused;
pub use projective_collection::inspect_projective_seam_collection_with_rational_scale_proposals;
pub use station_scale::propose_boundary_normal_scale;
pub use station_scale::propose_station_normal_scale;
pub(crate) use station_scale::propose_station_scale_scalar;
type H = [f64; 4];
#[derive(Clone, Copy)]
struct Boundary {
    cross_u: bool,
    max: bool,
}
impl Boundary {
    fn parse(name: &str) -> Result<Self> {
        match name {
            "uMin" => Ok(Self {
                cross_u: true,
                max: false,
            }),
            "uMax" => Ok(Self {
                cross_u: true,
                max: true,
            }),
            "vMin" => Ok(Self {
                cross_u: false,
                max: false,
            }),
            "vMax" => Ok(Self {
                cross_u: false,
                max: true,
            }),
            _ => Err(crate::input("Unknown surface boundary")),
        }
    }
    fn cross<'a>(&self, s: &'a Surface) -> (usize, usize, &'a [f64], bool) {
        if self.cross_u {
            (s.degree_u, s.control_points.len(), &s.knots_u, s.periodic_u)
        } else {
            (
                s.degree_v,
                s.control_points[0].len(),
                &s.knots_v,
                s.periodic_v,
            )
        }
    }
    fn along<'a>(&self, s: &'a Surface) -> (usize, usize, &'a [f64], bool) {
        Self {
            cross_u: !self.cross_u,
            max: false,
        }
        .cross(s)
    }
    fn index(&self, s: &Surface, i: usize, layer: usize) -> (usize, usize) {
        let n = self.cross(s).1;
        let c = if self.max { n - 1 - layer } else { layer };
        if self.cross_u { (c, i) } else { (i, c) }
    }
    fn get(&self, s: &Surface, i: usize, layer: usize) -> H {
        let (u, v) = self.index(s, i, layer);
        let w = s.weights[u][v];
        let p = &s.control_points[u][v];
        [p[0] * w, p[1] * w, p[2] * w, w]
    }
    fn set(&self, s: &mut Surface, i: usize, layer: usize, h: H) -> Result<()> {
        check(
            h.iter().all(|x| x.is_finite()) && h[3] >= 1e-12 && h[3] <= 1e12,
            "Surface matching produced inadmissible weights; reduce the normal scale or change the reference strip",
        )?;
        let (u, v) = self.index(s, i, layer);
        s.weights[u][v] = h[3];
        s.control_points[u][v] = h[..3].iter().map(|x| x / h[3]).collect();
        Ok(())
    }
    fn coefficients(&self, s: &Surface, order: usize) -> Result<Vec<Vec<f64>>> {
        let (p, n, k, periodic) = self.cross(s);
        check(
            !periodic,
            "Surface matching requires a non-periodic cross direction",
        )?;
        check(
            p >= order,
            "Surface matching requires cross degree at least the requested continuity order",
        )?;
        let k = normalized(k, p, n)?;
        let endpoint = if self.max { 1. } else { 0. };
        let edge = if self.max {
            &k[k.len() - p - 1..]
        } else {
            &k[..p + 1]
        };
        check(
            edge.iter().all(|&x| x == endpoint),
            "Surface matching requires a clamped cross boundary",
        )?;
        let b = basis(p, &k, n, endpoint, false)?;
        let sign = if self.max { -1. } else { 1. };
        let mut rows = vec![b.basis, b.d1.iter().map(|x| x * sign).collect()];
        if order == 2 {
            rows.push(b.d2);
        }
        let coefficients: Vec<Vec<f64>> = rows
            .iter()
            .map(|row| {
                (0..=order)
                    .map(|l| row[if self.max { n - 1 - l } else { l }])
                    .collect()
            })
            .collect();
        for q in 1..=order {
            check(
                coefficients[q][q].is_finite() && coefficients[q][q].abs() > 1e-14,
                "Surface boundary derivative system is singular",
            )?;
        }
        Ok(coefficients)
    }
}
fn normalized(knots: &[f64], degree: usize, count: usize) -> Result<Vec<f64>> {
    let a = knots[degree];
    let span = knots[count] - a;
    check(
        span.is_finite() && span > 0.,
        "Surface matching parameter span is invalid",
    )?;
    let output: Vec<f64> = knots.iter().map(|k| (k - a) / span).collect();
    check(
        output.iter().all(|x| x.is_finite()),
        "Surface matching normalized knots overflowed",
    )?;
    Ok(output)
}
// The represented binary64 knots are the input definition of this operation.
// The predicate certifies their affine correspondence, not any earlier unrounded construction.
fn affine_knots(a: &[f64], ad: [f64; 2], b: &[f64], bd: [f64; 2]) -> Result<()> {
    use cad_predicates::{
        AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
        orient2d,
    };
    check(a.len() == b.len(), "Surface seam knot counts differ")?;
    let values: Vec<f64> = [ad[0], bd[0], ad[1], bd[1]]
        .into_iter()
        .chain(a.iter().zip(b).flat_map(|(&x, &y)| [x, y]))
        .collect();
    let arena = SourceArena::authored(
        "represented-surface-knot-basis",
        1,
        values
            .iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|e| crate::input(format!("Invalid seam knot basis: {e:?}")))?;
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let pair = |i| [arena.leaf(i).unwrap(), arena.leaf(i + 1).unwrap()];
    for i in 0..a.len() {
        let decision = orient2d(&mut ctx, pair(0), pair(2), pair(4 + 2 * i))
            .map_err(|e| crate::input(format!("Seam knot predicate failed: {e:?}")))?;
        check(
            matches!(decision.outcome, Outcome::Sign(Sign::Zero)),
            "Surface boundaries need exactly affine-compatible seam knots",
        )?;
    }
    Ok(())
}
/// Matches inward normalized derivatives as E'= -scale R', E''=scale^2 R''.
/// Positive weights make projection from the matched homogeneous jets valid.
/// C1/C2 parametric matching implies geometric continuity only on regular seams.
/// Callers must inspect regularityCertified before applying a geometric match.

pub fn match_surface_jets_report(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    order: usize,
    scale: f64,
) -> Result<SurfaceJetMatch> {
    reference.validate()?;
    edited.validate()?;
    check(
        order == 1 || order == 2,
        "Surface continuity order must be 1 or 2",
    )?;
    check(
        scale.is_finite() && scale > 0.,
        "Surface normal scale must be positive and finite",
    )?;
    let r = Boundary::parse(reference_boundary)?;
    let e = Boundary::parse(edited_boundary)?;
    let (rp, rn, rk, _) = r.along(reference);
    let (ep, en, ek, _) = e.along(edited);
    check(
        rp == ep && rn == en,
        "Surface boundaries need compatible control counts and degrees",
    )?;
    affine_knots(rk, [rk[rp], rk[rn]], ek, [ek[ep], ek[en]])?;
    let rc = r.coefficients(reference, order)?;
    let ec = e.coefficients(edited, order)?;
    let mut output = edited.clone();
    for i in 0..rn {
        let rh: Vec<H> = (0..=order).map(|l| r.get(reference, i, l)).collect();
        let mut eh = vec![rh[0]];
        for q in 1..=order {
            let multiplier = if q == 1 { -scale } else { scale * scale };
            let h = std::array::from_fn(|axis| {
                let target = multiplier * (0..=order).map(|l| rc[q][l] * rh[l][axis]).sum::<f64>();
                (target - (0..q).map(|l| ec[q][l] * eh[l][axis]).sum::<f64>()) / ec[q][q]
            });
            eh.push(h);
        }
        let (ru, rv) = r.index(reference, i, 0);
        let (eu, ev) = e.index(&output, i, 0);
        output.control_points[eu][ev] = reference.control_points[ru][rv].clone();
        output.weights[eu][ev] = reference.weights[ru][rv];
        for (layer, h) in eh.into_iter().enumerate().skip(1) {
            e.set(&mut output, i, layer, h)?;
        }
    }
    output.validate()?;
    let reference_regularity = regularity::certify(reference, r)?;
    let edited_regularity = regularity::certify(&output, e)?;
    let error_bounds = bounds::certify(reference, &output, r, e, order, scale)?;
    Ok(SurfaceJetMatch {
        surface: output,
        report: SurfaceJetReport {
            continuity_order: order,
            normal_scale: scale,
            reference_regularity,
            edited_regularity,
            error_bounds,
            reversed: false,
            decision: None,
        },
    })
}

/// Reverse only the seam parameter while matching, then restore the edited
/// parameter direction and map diagnostic intervals back to its original domain.
pub fn match_surface_jets_oriented_report(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    order: usize,
    scale: f64,
    reverse: bool,
) -> Result<SurfaceJetMatch> {
    if !reverse {
        return match_surface_jets_report(
            reference,
            edited,
            reference_boundary,
            edited_boundary,
            order,
            scale,
        );
    }
    let edge = Boundary::parse(edited_boundary)?;
    let axis = if edge.cross_u {
        crate::surface::Axis::V
    } else {
        crate::surface::Axis::U
    };
    let aligned = edited.edit_axis(axis, |c| c.reverse())?;
    let mut value = match_surface_jets_report(
        reference,
        &aligned,
        reference_boundary,
        edited_boundary,
        order,
        scale,
    )?;
    let restored = value.surface.edit_axis(axis, |c| c.reverse())?;
    let (p, n, ak, _) = edge.along(&aligned);
    let (_, _, rk, _) = edge.along(&restored);
    let reversed_knots: Vec<f64> = rk.iter().copied().rev().collect();
    affine_knots(ak, [ak[p], ak[n]], &reversed_knots, [rk[n], rk[p]])?;
    value.surface = restored;
    value.report.reversed = true;
    let (p, n, k, _) = edge.along(edited);
    let a = k[p];
    let b = k[n];
    for interval in value.report.edited_regularity.unresolved_intervals_mut() {
        *interval = [a + (b - interval[1]), a + (b - interval[0])];
    }
    Ok(value)
}

/// Inspect represented boundary jets without editing either input surface.
/// The seam basis must already agree; returned bounds concern these inputs.
pub fn inspect_surface_jets_checked_report(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    order: usize,
    scale: f64,
    max_error: f64,
) -> Result<SurfaceJetReport> {
    reference.validate()?;
    edited.validate()?;
    check(
        (order == 1 || order == 2)
            && scale.is_finite()
            && scale > 0.
            && max_error.is_finite()
            && max_error >= 0.,
        "Invalid surface jet inspection options",
    )?;
    let r = Boundary::parse(reference_boundary)?;
    let e = Boundary::parse(edited_boundary)?;
    let (rp, rn, rk, _) = r.along(reference);
    let (ep, en, ek, _) = e.along(edited);
    check(
        rp == ep && rn == en,
        "Surface jet inspection requires a common seam basis",
    )?;
    affine_knots(rk, [rk[rp], rk[rn]], ek, [ek[ep], ek[en]])?;
    // Inspection does not need an editable second control layer for a ruled
    // strip. Its second homogeneous jet is identically zero.
    r.coefficients(reference, order.min(r.cross(reference).0))?;
    e.coefficients(edited, order.min(e.cross(edited).0))?;
    let reference_regularity = regularity::certify(reference, r)?;
    let edited_regularity = regularity::certify(edited, e)?;
    let error_bounds = bounds::certify(reference, edited, r, e, order, scale)?;
    let smooth = [(reference, r), (edited, e)].iter().all(|(s, b)| {
        let (p, n, k, periodic) = b.along(s);
        let a = k[p];
        let z = k[n];
        k.iter().all(|&u| {
            !((u > a || periodic && u == a)
                && u < z
                && k.iter().filter(|&&x| x == u).count() > p.saturating_sub(order))
        })
    });
    let regular = reference_regularity.certified() && edited_regularity.certified();
    let error = error_bounds.maximum();
    let accepted = regular && smooth && error <= max_error;
    Ok(SurfaceJetReport {
        continuity_order: order,
        normal_scale: scale,
        reference_regularity,
        edited_regularity,
        error_bounds,
        reversed: false,
        decision: Some(SurfaceJetDecision {
            accepted,
            max_error,
            error_upper: error,
            tangential_smoothness_certified: smooth,
            reason: if !regular {
                SurfaceJetDecisionReason::UnprovenRegularity
            } else if !smooth {
                SurfaceJetDecisionReason::UnprovenTangentialSmoothness
            } else if error > max_error {
                SurfaceJetDecisionReason::DeviationExceedsBudget
            } else {
                SurfaceJetDecisionReason::Accepted
            },
        }),
    })
}

/// Application gate: require regularity, the requested tangential smoothness,
/// and a continuous bound on all independently varying jet components.
pub fn match_surface_jets_checked_report(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    order: usize,
    scale: f64,
    reverse: bool,
    max_error: f64,
) -> Result<SurfaceJetMatch> {
    check(
        max_error.is_finite() && max_error >= 0.,
        "Surface matching tolerance must be finite and nonnegative",
    )?;
    let mut value = match_surface_jets_oriented_report(
        reference,
        edited,
        reference_boundary,
        edited_boundary,
        order,
        scale,
        reverse,
    )?;
    let mut smooth = true;
    for (s, name) in [(reference, reference_boundary), (edited, edited_boundary)] {
        let b = Boundary::parse(name)?;
        let (p, n, k, periodic) = b.along(s);
        let a = k[p];
        let z = k[n];
        for &u in k {
            if (u > a || periodic && u == a)
                && u < z
                && k.iter().filter(|&&x| x == u).count() > p.saturating_sub(order)
            {
                smooth = false;
            }
        }
    }
    let error = value.report.error_bounds.maximum();
    let regular = value.report.regularity_certified();
    let accepted = regular && smooth && error <= max_error;
    value.report.decision = Some(SurfaceJetDecision {
        accepted,
        max_error,
        error_upper: error,
        tangential_smoothness_certified: smooth,
        reason: if !regular {
            SurfaceJetDecisionReason::UnprovenRegularity
        } else if !smooth {
            SurfaceJetDecisionReason::UnprovenTangentialSmoothness
        } else if error > max_error {
            SurfaceJetDecisionReason::DeviationExceedsBudget
        } else {
            SurfaceJetDecisionReason::Accepted
        },
    });
    Ok(value)
}

#[cfg(test)]
#[path = "continuity/tests.rs"]
mod tests;
