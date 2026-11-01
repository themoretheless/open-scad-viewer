//! Public opaque wrapper for the existing exact ORIGINAL normalized carrier owner.
use super::*;
#[derive(Clone, Copy)]
pub struct NormalizedCarrierIdentityLimits {
    pub cells: usize,
    pub exact_work: u64,
}
#[derive(Clone, Debug)]
pub struct NormalizedCarrierIdentityReport {
    pub certified: bool,
    pub cells: usize,
    pub exact_work: u64,
    pub reason: Option<&'static str>,
}
/// Exact whole-map equality in independently normalized UV domains. This gives
/// neither regularity nor material, orientation, filled-volume or Gk credit.
pub struct NormalizedCarrierIdentity<'s> {
    caps: [&'s Surface; 2],
    proof: cap_identity::Identity<'s>,
    report: NormalizedCarrierIdentityReport,
}
impl<'s> NormalizedCarrierIdentity<'s> {
    pub fn surfaces(&self) -> [&'s Surface; 2] {
        self.caps
    }
    pub fn matches(&self, caps: [&Surface; 2]) -> bool {
        self.proof.matches(caps)
    }
    pub fn report(&self) -> &NormalizedCarrierIdentityReport {
        &self.report
    }
}
pub enum NormalizedCarrierIdentityAttempt<'s> {
    Certified(Box<NormalizedCarrierIdentity<'s>>),
    Refused(Box<NormalizedCarrierIdentityReport>),
}
/// All failed affine/degree/basis trials consume the same explicit ledger.
/// No rounded normalized knots, elevated poles or equality report is accepted.
pub fn certify_normalized_carrier_identity<'s>(
    caps: [&'s Surface; 2],
    l: NormalizedCarrierIdentityLimits,
) -> Result<NormalizedCarrierIdentityAttempt<'s>> {
    check(
        l.cells <= 1000000 && l.exact_work <= 1000000,
        "Invalid normalized carrier identity work",
    )?;
    for s in caps {
        s.validate()?;
        check(
            s.control_points.iter().flatten().all(|p| p.len() == 3),
            "Normalized carrier identity requires XYZ source nets",
        )?;
    }
    let mut ledger = IdealVolumeReport {
        embedding_certified: false,
        regularity_certified: false,
        parameter_topology_preserved: false,
        cells: 0,
        exact_work: 0,
        boxes: 0,
        monotonicity_lower: None,
        projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        reason: None,
    };
    let proof = cap_identity::certify(caps, &mut ledger, l.cells, l.exact_work);
    let report = NormalizedCarrierIdentityReport {
        certified: proof.is_some(),
        cells: ledger.cells,
        exact_work: ledger.exact_work,
        reason: proof
            .is_none()
            .then_some("normalized-original-carrier-identity-unproved"),
    };
    Ok(match proof {
        Some(proof) => {
            NormalizedCarrierIdentityAttempt::Certified(Box::new(NormalizedCarrierIdentity {
                caps,
                proof,
                report,
            }))
        }
        None => NormalizedCarrierIdentityAttempt::Refused(Box::new(report)),
    })
}

#[cfg(test)]
mod tests;
