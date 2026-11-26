//! Exact rational UV curve pieces divided at coordinate seam contacts.
use super::{
    PcurveSeamIsolation, UvAxis,
    pcurve_mobius::{active_spans, original_span, scale, store_target},
    pcurve_seam::validate_seam,
    pcurve_seam_isolation::isolate,
};
use crate::{
    Result,
    core::bounded_rational::Work,
    curve::Curve,
    exact_geometry::exact_derivative::{PResult, Refusal, add},
};
use num_traits::{Signed, Zero};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcurveSeamSide {
    Below,
    On,
    Above,
}

#[derive(Clone, Debug)]
pub struct PcurveSeamPiece {
    pub curve: Curve,
    /// Side on the open parameter interval of this piece.
    pub side: PcurveSeamSide,
}

#[derive(Clone, Debug)]
pub struct PcurveSeamSplitReport {
    /// Complete ordered cover in the original absolute curve parameter.
    pub pieces: Option<Vec<PcurveSeamPiece>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Split each original span at all interior seam contacts, including
/// isolated tangencies. Both the split parameters and the derived clamped
/// pieces must be exactly representable in binary64. Coincident spans are
/// retained as whole pieces. Each stored piece is verified against the
/// original curve over its complete parameter interval.
pub fn split_pcurve_at_seam(
    source: &Curve,
    axis: UvAxis,
    seam: f64,
    max_work: u64,
) -> Result<PcurveSeamSplitReport> {
    let coordinate = validate_seam(source, axis, seam, max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let (pieces, refusal) = match split(source, coordinate, seam, &mut work) {
        Ok(pieces) => (Some(pieces), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(PcurveSeamSplitReport {
        pieces,
        refusal,
        work_used: work.used,
    })
}

fn split(
    source: &Curve,
    coordinate: usize,
    seam: f64,
    work: &mut Work,
) -> PResult<Vec<PcurveSeamPiece>> {
    let isolated = isolate(source, coordinate, seam, None, work)?;
    let seam = work.lift(seam)?;
    let mut pieces = Vec::new();
    for ((span, [a, b]), isolated) in active_spans(source).zip(isolated) {
        let mut breaks = vec![a];
        if let PcurveSeamIsolation::Discrete { roots, .. } = isolated.contact {
            for root in roots {
                let [lo, hi] = root.parameter;
                if lo != hi {
                    return Err(Refusal::UnprovedIdentity);
                }
                breaks.push(lo);
            }
        }
        breaks.push(b);
        for interval in breaks.windows(2) {
            let interval = [interval[0], interval[1]];
            if interval[0] >= interval[1] {
                return Err(Refusal::UnprovedIdentity);
            }
            let h = original_span(source, span, interval, work)?;
            let offset = scale(&h[2], &seam, work)?;
            let power = add(&h[coordinate], &offset, true, work)?;
            // The full interval has no unaccounted interior root, so the
            // first nonzero coefficient decides its open-interval sign.
            let side = match power.iter().find(|q| !q.is_zero()) {
                Some(q) if q.is_negative() => PcurveSeamSide::Below,
                Some(_) => PcurveSeamSide::Above,
                None => PcurveSeamSide::On,
            };
            pieces.push(PcurveSeamPiece {
                curve: store_target(interval, &h, work)?,
                side,
            });
        }
    }
    Ok(pieces)
}
