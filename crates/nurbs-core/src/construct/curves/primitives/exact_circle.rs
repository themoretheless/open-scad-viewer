//! A private reference must be freshly proved; no caller-owned proof is used.
use super::exact_conics::Refusal;
use crate::core::bounded_rational::Work;
type PResult<T> = std::result::Result<T, Refusal>;
use crate::curve::Curve;
pub(super) struct Template {
    circle: Curve,
}
impl Template {
    pub(super) fn load(w: &mut Work) -> PResult<Self> {
        if w.used == w.limit {
            return Err(Refusal::ResourceLimit);
        }
        let report =
            super::exact_conics::ellipse([0.; 3], [1., 0., 0.], [0., 1., 0.], w.limit - w.used)
                .map_err(|_| Refusal::InvalidOutput)?;
        w.charge(report.work_used)?;
        match (report.curve, report.refusal) {
            (Some(circle), None) => Ok(Self { circle }),
            (None, Some(e)) => Err(e),
            _ => Err(Refusal::UnprovedIdentity),
        }
    }
    pub(super) fn circle(&self) -> &Curve {
        &self.circle
    }
}
