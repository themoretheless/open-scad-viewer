use super::{Limits, Piece, Span, ordered};
use crate::{Result, check, offset_contact_pcurve as uv, surface::Surface};
pub(super) fn fit(
    sources: [&Surface; 2],
    distances: [f64; 2],
    span: Span,
    tol_uv: f64,
    tol_mm: f64,
    limits: Limits,
) -> Result<Piece> {
    let (s, d) = ordered(sources, distances, span);
    let uv_fit = uv::fit_branch(
        s,
        d,
        span.axis,
        span.drive,
        span.free,
        span.other,
        tol_uv,
        limits.uv_fit,
    )?;
    let mut curves = uv_fit
        .curves
        .as_ref()
        .ok_or_else(|| crate::input("Original chain UV proposals remain unqualified"))?
        .clone();
    // The independent coordinate is the literal clock. Exact affine-clock
    // Bezier proposals avoid tiny interpolation overshoots at source boundaries.
    // Modified nets still require the final complete UV/world/lift proofs.
    let n = curves[0].control_points.len() - 1;
    for (i, p) in curves[0].control_points.iter_mut().enumerate() {
        p[span.axis] = if i == 0 {
            span.drive[0]
        } else if i == n {
            span.drive[1]
        } else {
            let f = i as f64 / n as f64;
            span.drive[0] * (1. - f) + span.drive[1] * f
        };
    }
    if span.owner == 1 {
        curves.swap(0, 1);
    }
    let world_fit = super::super::fit(
        s,
        d,
        span.axis,
        span.drive,
        span.free,
        span.other,
        tol_mm,
        limits.world_fit,
    )?;
    let world = world_fit
        .curve
        .as_ref()
        .ok_or_else(|| crate::input("Original chain world proposal remains unqualified"))?
        .clone();
    Ok(Piece {
        spec: span,
        curves,
        world,
        uv_fit,
        world_fit,
        uv_proofs: vec![],
        world_proof: None,
        lift_proofs: vec![],
    })
}
pub(super) fn qualify(
    sources: [&Surface; 2],
    distances: [f64; 2],
    piece: &mut Piece,
    tol_uv: f64,
    tol_mm: f64,
    l: Limits,
) -> Result<()> {
    let span = piece.spec;
    let (s, d) = ordered(sources, distances, span);
    for original in 0..2 {
        let side = if span.owner == 0 {
            original
        } else {
            1 - original
        };
        let mut proof = uv::certify_refined(&piece.curves[original], side, uv::OffsetFaces { surfaces: s, distances: d }, uv::ContactParameters { axis: span.axis, drive: span.drive, free: span.free, second: span.other, spans: l.spans }, tol_uv, uv::PcurveWork { max_cells: l.final_cells_per_curve, refinements: l.refinements, band_refinements: l.refinements })?;
        check(
            proof.correspondence_proven,
            "Bound actual chain UV curve lacks full original-root authority",
        )?;
        proof.side = original;
        piece.uv_proofs.push(proof);
    }
    let proof = super::super::certify(
        &piece.world,
        s,
        d,
        span.axis,
        span.drive,
        span.free,
        span.other,
        tol_mm,
        super::super::Limits {
            cells: l.final_cells_per_curve,
            spans: l.spans,
            section_refinements: l.refinements,
            band_refinements: l.refinements,
        },
    )?;
    check(
        proof.world_correspondence_proven,
        "Bound actual chain world curve lacks full original-root authority",
    )?;
    piece.world_proof = Some(proof);
    for original in 0..2 {
        let proof = super::super::lift::certify(
            &piece.world,
            &piece.curves[original],
            sources[original],
            distances[original],
            tol_mm,
            super::super::lift::Limits {
                cells: l.final_cells_per_curve,
                spans: l.spans,
            },
        )?;
        check(
            proof.lift_correspondence_proven,
            "Actual paired chain UV/world curves lack complete millimetre lift agreement",
        )?;
        piece.lift_proofs.push(proof);
    }
    Ok(())
}
