//! Whole original material owner plus exact declared longitudinal G1/G2 jets.
use super::*;
use nurbs_core::progressive_sweep::{
    OriginalProfileBoundary, OriginalProfileBoundaryAttempt, OriginalProfileBoundaryLimits,
    OriginalProfileBoundaryReport, OriginalProfileClass,
};
pub struct OriginalBoundaryBody<'b, 's, 'a> {
    body: &'b OriginalSmoothBody<'s, 'a>,
    profiles: Box<OriginalProfileBoundary<'b, 's, 'a>>,
    work: OriginalSmoothBodyWork,
}
impl<'b, 's, 'a> OriginalBoundaryBody<'b, 's, 'a> {
    pub fn body(&self) -> &'b OriginalSmoothBody<'s, 'a> {
        self.body
    }
    pub fn profiles(&self) -> &OriginalProfileBoundary<'b, 's, 'a> {
        &self.profiles
    }
    pub fn work(&self) -> OriginalSmoothBodyWork {
        self.work
    }
    /// End-cap and wall corners of this open nondegenerate product remain C0.
    pub fn cap_wall_continuity(&self) -> &'static str {
        "C0"
    }
}
pub struct OriginalBoundaryBodyRefusal {
    pub certified: bool,
    pub work: OriginalSmoothBodyWork,
    /// Already measured source proof prefix; not charged if it cannot fit.
    pub required_prefix: OriginalSmoothBodyWork,
    pub profiles: Option<Box<OriginalProfileBoundaryReport>>,
    pub reason: &'static str,
}
pub enum OriginalBoundaryBodyAttempt<'b, 's, 'a> {
    Certified(Box<OriginalBoundaryBody<'b, 's, 'a>>),
    Refused(Box<OriginalBoundaryBodyRefusal>),
}
/// The actual immutable body already owns complete cap/profile UV binding,
/// material loops/holes, regular source volume and retained BRep admission.
/// Fresh original geometric profile jets then transport through the SAME Ck
/// motion to regular longitudinal wall Gk. The affine-in-profile transport
/// preserves positive first- and second-order geometric reparameterizations;
/// source volume rank and trace binding supply wall regularity. Cap interiors
/// retain the body's Ck; sharp end joins remain C0. Retained approximations do
/// not inherit this ORIGINAL boundary classification.
///
/// Cells/work/products are TOTAL original allowances: the body's complete
/// original ledger is debited first. `steps` bounds this new profile census.
/// Only an opaque same-body object is accepted, never caller report flags.
pub fn qualify_original_smooth_boundary<'b, 's, 'a>(
    body: &'b OriginalSmoothBody<'s, 'a>,
    classes: &[OriginalProfileClass],
    l: OriginalProfileBoundaryLimits,
) -> Result<OriginalBoundaryBodyAttempt<'b, 's, 'a>> {
    if l.steps > 1000000
        || l.cells > 1000000
        || l.exact_work > 1000000
        || l.products > 1000000
        || classes.is_empty()
        || classes.len() > 1000000
    {
        return Err(fail("Invalid complete original boundary work or classes"));
    }
    let mut work = body.work();
    if work.cells > l.cells || work.exact_work > l.exact_work || work.products > l.products {
        return Ok(OriginalBoundaryBodyAttempt::Refused(Box::new(
            OriginalBoundaryBodyRefusal {
                certified: false,
                work: OriginalSmoothBodyWork::default(),
                required_prefix: work,
                profiles: None,
                reason: "original-boundary-body-prefix-work-limit",
            },
        )));
    }
    let attempt = body.original().certify_profile_boundaries(
        classes,
        OriginalProfileBoundaryLimits {
            cells: l.cells - work.cells,
            exact_work: l.exact_work - work.exact_work,
            products: l.products - work.products,
            ..l
        },
    )?;
    match attempt {
        OriginalProfileBoundaryAttempt::Certified(profiles) => {
            work.cells += profiles.report().cells;
            work.exact_work += profiles.report().exact_work;
            work.products += profiles.report().products;
            Ok(OriginalBoundaryBodyAttempt::Certified(Box::new(
                OriginalBoundaryBody {
                    body,
                    profiles,
                    work,
                },
            )))
        }
        OriginalProfileBoundaryAttempt::Refused(r) => {
            work.cells += r.cells;
            work.exact_work += r.exact_work;
            work.products += r.products;
            let reason = r
                .reason
                .unwrap_or("original-boundary-body-profile-jets-unproved");
            Ok(OriginalBoundaryBodyAttempt::Refused(Box::new(
                OriginalBoundaryBodyRefusal {
                    certified: false,
                    work,
                    required_prefix: body.work(),
                    profiles: Some(r),
                    reason,
                },
            )))
        }
    }
}
