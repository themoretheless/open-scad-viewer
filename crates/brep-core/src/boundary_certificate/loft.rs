//! Constructor-owned paired loft qualification. A refused candidate is never
//! promoted into a certified body; diagnostics keep the exact failing stage.
use super::*;
use nurbs_core::{curve::Curve, surface::Surface};

pub struct Loft<'a> {
    pub start: &'a [Vec<Curve>],
    pub end: &'a [Vec<Curve>],
    pub sides: &'a [Vec<Surface>],
    pub caps: [&'a crate::LoftCap; 2],
}
/// This value can only be obtained by building both bodies and running the
/// full native conjunction. No transport-provided boolean can construct it.
pub struct CertifiedBody {
    model: Model,
    reference: Model,
    report: Report,
}
impl CertifiedBody {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn reference(&self) -> &Model {
        &self.reference
    }
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
pub enum Attempt {
    Certified(Box<CertifiedBody>),
    Refused(Box<Report>),
}

pub fn construct(
    reference: Loft<'_>,
    retained: Loft<'_>,
    tolerance_uv: f64,
    error_budget: f64,
    limits: Limits,
) -> Result<Attempt> {
    let reference = crate::capped_loft_with_caps(
        reference.start,
        reference.end,
        reference.sides,
        reference.caps,
    )?;
    let model =
        crate::capped_loft_with_caps(retained.start, retained.end, retained.sides, retained.caps)?;
    let report = inspect(&reference, &model, tolerance_uv, error_budget, limits)?;
    Ok(if report.certified {
        Attempt::Certified(Box::new(CertifiedBody {
            model,
            reference,
            report,
        }))
    } else {
        Attempt::Refused(Box::new(report))
    })
}
