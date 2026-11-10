//! Conditional source-trace transport through freshly bound invertible affine maps.
//! The caller MUST subsequently prove complete actual boundary agreement.
use super::*;
use nurbs_core::curve::Curve;
fn clamped(c: &Curve) -> bool {
    let p = c.degree;
    !c.periodic
        && c.control_points.len() == p + 1
        && c.knots[..=p].iter().all(|k| *k == c.knots[p])
        && c.knots[p + 1..].iter().all(|k| *k == c.knots[p + 1])
        && c.knots[p] < c.knots[p + 1]
}
fn step(out: &mut AffineFilledVolumeReport, max: u64) -> bool {
    if out.work == max {
        out.reason = "actual-affine-boundary-work-limit";
        false
    } else {
        out.work += 1;
        true
    }
}
pub(super) fn prepare(
    model: &Model,
    c: &affine_candidate::Candidate,
    caps: [&LoftCap; 2],
    out: &mut AffineFilledVolumeReport,
    max: u64,
) -> Option<bool> {
    let Some(first) = c.pieces.first().and_then(|p| p.controls.first()) else {
        return Some(false);
    };
    let Some(last) = c.pieces.last().and_then(|p| p.controls.last()) else {
        return Some(false);
    };
    let poses = [*first, *last];
    for pose in poses {
        for _ in 0..9 {
            if !step(out, max) {
                return None;
            }
        }
        if crate::rational_cap_affine::inverse(pose).is_none() {
            return Some(false);
        }
    }
    let sides = c.sides.iter().map(Vec::len).sum::<usize>();
    for endpoint in 0..2 {
        let source = &caps[endpoint].surface;
        let target = &c.caps[endpoint].surface;
        if source.degree_u != target.degree_u
            || source.degree_v != target.degree_v
            || source.knots_u != target.knots_u
            || source.knots_v != target.knots_v
            || source.weights != target.weights
            || source.periodic_u != target.periodic_u
            || source.periodic_v != target.periodic_v
            || source.control_points.len() != target.control_points.len()
            || source
                .control_points
                .iter()
                .zip(&target.control_points)
                .any(|(a, b)| a.len() != b.len())
        {
            return Some(false);
        }
        for (a, b) in source
            .control_points
            .iter()
            .flatten()
            .zip(target.control_points.iter().flatten())
        {
            if !step(out, max) {
                return None;
            }
            if crate::rational_cap_affine::image(poses[endpoint], a, c.origin)
                .as_ref()
                .map(|p| p.as_slice())
                != Some(b.as_slice())
            {
                return Some(false);
            }
        }
        let face = &model.faces[sides + endpoint];
        let wires = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .collect::<Vec<_>>();
        if wires.len() != caps[endpoint].trims.len() {
            return Some(false);
        }
        for (ring, &wire) in wires.iter().enumerate() {
            let uses = &model.loops[wire].coedges;
            let trims = &caps[endpoint].trims[ring];
            if uses.len() != trims.len() || trims.len() != c.original[ring].len() {
                return Some(false);
            }
            let images = if endpoint == 0 {
                &c.start[ring]
            } else {
                &c.end[ring]
            };
            for (span, (use_, trim)) in uses.iter().zip(trims).enumerate() {
                if !step(out, max) {
                    return None;
                }
                // Same normalized Bernstein clock, with all ORIGINAL UV poles
                // and weights. Merely equal boundary images are insufficient.
                let uv = &use_.pcurve;
                if use_.trim().is_some()
                    || !clamped(uv)
                    || !clamped(trim)
                    || uv.degree != trim.degree
                    || uv.control_points != trim.control_points
                    || uv.weights != trim.weights
                {
                    return Some(false);
                }
                let p = &c.original[ring][span];
                let q = &images[span];
                if p.degree != q.degree
                    || p.weights != q.weights
                    || !clamped(p)
                    || !clamped(q)
                    || p.periodic != q.periodic
                    || p.control_points.len() != q.control_points.len()
                {
                    return Some(false);
                }
                for (a, b) in p.control_points.iter().zip(&q.control_points) {
                    if !step(out, max) {
                        return None;
                    }
                    if crate::rational_cap_affine::image(poses[endpoint], a, c.origin)
                        .as_ref()
                        .map(|p| p.as_slice())
                        != Some(b.as_slice())
                    {
                        return Some(false);
                    }
                }
            }
        }
    }
    // Actual full lifts give A(C(uv)-origin)+T=A(P-origin)+T.
    // Exact invertibility of A proves C(uv)=P on the SAME normalized clock.
    // This is one fresh native proof chain, not credit from a prior report.
    Some(true)
}
