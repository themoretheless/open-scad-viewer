//! Exact conversion of active UV traversals to ordinary source coedges.
use crate::{Coedge, Curve, Error, Model, Result};
use num_rational::BigRational;
use nurbs_core::editing::exact_knot_insertion;
use std::borrow::Cow;

fn refusal(message: &str) -> Error {
    Error::new("BREP_SOURCE_SUPPORT_TRIM", message)
}

/// Full-world coedges may author an independent UV range. A partial world
/// range is a diagnostic window in Model and cannot authorize a complete use.
pub(super) fn active_pcurves(model: &Model, max_work: u64) -> Result<(Cow<'_, Model>, u64)> {
    if model
        .loops
        .iter()
        .flat_map(|w| &w.coedges)
        .all(|c| c.trim().is_none())
    {
        return Ok((Cow::Borrowed(model), 0));
    }
    let mut result = model.clone();
    let mut work = 0;
    for face in &model.faces {
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            for (slot, coedge) in model.loops[wire].coedges.iter().enumerate() {
                let Some(trim) = coedge.trim() else { continue };
                let world = &model.edges[coedge.edge].curve;
                if trim.curve_parameter != world.domain() {
                    return Err(refusal(
                        "A partial world interval cannot own the original shell edge",
                    ));
                }
                if crate::boundary_agreement::periodic_lifts(
                    &face.surface,
                    &coedge.pcurve,
                    trim.pcurve_parameter,
                )? != trim.periodic_lift
                {
                    return Err(refusal(
                        "Authored endpoint lifts disagree with the active pcurve",
                    ));
                }
                let [first, last] = trim.pcurve_parameter;
                let range = [first.min(last), first.max(last)];
                let mut pcurve = if range == coedge.pcurve.domain() {
                    coedge.pcurve.clone()
                } else {
                    let remaining = (max_work - work).min(cad_predicates::MAX_WORK);
                    if remaining == 0 {
                        return Err(refusal("Exact source pcurve restriction budget exhausted"));
                    }
                    let report = exact_knot_insertion::restrict(&coedge.pcurve, range, remaining)?;
                    work += report.work_used;
                    report
                        .curve
                        .ok_or_else(|| refusal("Exact source pcurve restriction is unproved"))?
                };
                if first > last {
                    let reversed = pcurve.reverse()?;
                    check_reverse(&pcurve, &reversed, &mut work, max_work)?;
                    pcurve = reversed;
                }
                result.loops[wire].coedges[slot] =
                    Coedge::new(coedge.edge, coedge.reversed, pcurve);
            }
        }
    }
    result.validate()?;
    Ok((Cow::Owned(result), work))
}

fn reflected_matches(domain: [f64; 2], original: f64, proposed: f64) -> bool {
    let [a, b, source, result] =
        [domain[0], domain[1], original, proposed].map(BigRational::from_float);
    match (a, b, source, result) {
        (Some(a), Some(b), Some(source), Some(result)) => a + b == source + result,
        _ => false,
    }
}

fn check_reflection(
    domain: [f64; 2],
    original: f64,
    proposed: f64,
    work: &mut u64,
    max_work: u64,
) -> Result<()> {
    if *work == max_work {
        return Err(refusal("Exact source chart reflection budget exhausted"));
    }
    *work += 1;
    if !reflected_matches(domain, original, proposed) {
        return Err(refusal("Source chart reflection is not exact"));
    }
    Ok(())
}

fn check_reverse(source: &Curve, reversed: &Curve, work: &mut u64, max_work: u64) -> Result<()> {
    let domain = source.domain();
    for (&k, &candidate) in source.knots.iter().rev().zip(&reversed.knots) {
        check_reflection(domain, k, candidate, work, max_work)?;
    }
    Ok(())
}

/// Face-sense normalization changes the parameter chart. Prove that its
/// binary coordinates preserve the physical authored surface and trims.
pub(super) fn exact_face_senses(
    model: &Model,
    oriented: &Model,
    work: &mut u64,
    max_work: u64,
) -> Result<()> {
    for usage in model
        .shells
        .iter()
        .flat_map(|shell| &shell.faces)
        .filter(|usage| usage.reversed)
    {
        let face = &model.faces[usage.face];
        let changed = &oriented.faces[usage.face];
        let u = [
            face.surface.knots_u[face.surface.degree_u],
            face.surface.knots_u[face.surface.control_points.len()],
        ];
        for (&k, &candidate) in face
            .surface
            .knots_u
            .iter()
            .rev()
            .zip(&changed.surface.knots_u)
        {
            check_reflection(u, k, candidate, work, max_work)?;
        }
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            for (source, target) in model.loops[wire]
                .coedges
                .iter()
                .zip(oriented.loops[wire].coedges.iter().rev())
            {
                check_reverse(&source.pcurve, &target.pcurve, work, max_work)?;
                for (p, q) in source
                    .pcurve
                    .control_points
                    .iter()
                    .rev()
                    .zip(&target.pcurve.control_points)
                {
                    check_reflection(u, p[0], q[0], work, max_work)?;
                }
            }
        }
    }
    Ok(())
}
