//! One constructor owns the retained Solid AND the original Ck embedding.
//! This does not upgrade open cap-wall joins or retained approximation seams.
use super::*;
use nurbs_core::progressive_sweep::{
    OriginalSmoothVolume, OriginalSmoothVolumeAttempt, OriginalSmoothVolumeLimits,
    OriginalSmoothVolumeReport,
};
#[derive(Clone, Copy, Debug, Default)]
pub struct OriginalSmoothBodyWork {
    pub cells: usize,
    pub exact_work: u64,
    pub products: usize,
}
/// No model is exposed on a failed stronger admission. `retained` may contain
/// successful weaker diagnostics; these cannot manufacture the stronger body.
pub struct OriginalSmoothBodyRefusal {
    pub certified: bool,
    pub retained: Report,
    pub original: Option<Box<OriginalSmoothVolumeReport>>,
    pub work: OriginalSmoothBodyWork,
    pub reason: &'static str,
}
/// Owns an immutable retained BRep and borrows the exact original source
/// packet, including UV trims. Source report flags cannot construct/rebind it.
pub struct OriginalSmoothBody<'s, 'a> {
    retained: Box<CertifiedBody>,
    original: Box<OriginalSmoothVolume<'s, 'a>>,
    source_caps: [&'s LoftCap; 2],
    work: OriginalSmoothBodyWork,
}
impl<'s, 'a> OriginalSmoothBody<'s, 'a> {
    pub fn model(&self) -> &Model {
        self.retained.model()
    }
    pub fn retained_report(&self) -> &Report {
        self.retained.report()
    }
    pub fn original(&self) -> &OriginalSmoothVolume<'s, 'a> {
        &self.original
    }
    pub fn source_caps(&self) -> [&'s LoftCap; 2] {
        self.source_caps
    }
    pub fn work(&self) -> OriginalSmoothBodyWork {
        self.work
    }
    /// Minimum owned boundary classification. Original volume C2 is not a
    /// proof of G2 for cap-wall corners or retained station/profile seams.
    pub fn cap_wall_continuity(&self) -> &'static str {
        "C0"
    }
    pub fn into_model(self) -> Model {
        (*self.retained).into_model()
    }
}
pub enum OriginalSmoothBodyAttempt<'s, 'a> {
    Certified(Box<OriginalSmoothBody<'s, 'a>>),
    Refused(Box<OriginalSmoothBodyRefusal>),
}
fn refused<'s, 'a>(
    retained: Report,
    original: Option<Box<OriginalSmoothVolumeReport>>,
    work: OriginalSmoothBodyWork,
    reason: &'static str,
) -> OriginalSmoothBodyAttempt<'s, 'a> {
    OriginalSmoothBodyAttempt::Refused(Box::new(OriginalSmoothBodyRefusal {
        certified: false,
        retained,
        original,
        work,
        reason,
    }))
}
/// Constructor-owned open filled Solid with a C1/C2 ORIGINAL volume embedding.
/// The retained boundary/error/material proof uses `limits`. All stronger
/// original trim correspondence, profile regularity, global embedding, cap
/// and motion stages share `original_limits`; no budget resets per stage.
/// Original and retained smoothness remain distinct, cap-wall class stays C0.
/// A refused stage returns no model and accepts no caller-supplied reports.
pub fn construct_with_original_smooth_volume<'s, 'a>(
    source: SweepSource<'s, 'a, '_>,
    retained_caps: [&LoftCap; 2],
    accuracy: SweepAccuracy,
    limits: Limits,
    projection: [[f64; 3]; 3],
    original_limits: OriginalSmoothVolumeLimits,
) -> Result<OriginalSmoothBodyAttempt<'s, 'a>> {
    let SweepSource { sweep, loop_sizes, sections, source_caps } = source;
    let SweepAccuracy { tolerance_uv, error_budget } = accuracy;
    validate(sweep, loop_sizes, projection, original_limits)?;
    let attempt = construct(
        sweep,
        loop_sizes,
        sections,
        source_caps,
        retained_caps,
        tolerance_uv,
        error_budget,
        limits,
    )?;
    finish(
        sweep,
        loop_sizes,
        source_caps,
        projection,
        original_limits,
        attempt,
    )
}
fn validate(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    projection: [[f64; 3]; 3],
    original_limits: OriginalSmoothVolumeLimits,
) -> Result<()> {
    let l = original_limits;
    if !matches!(l.order, 1 | 2)
        || l.cells > 1000000
        || l.exact_work > 1000000
        || l.products > 1000000
        || projection.iter().flatten().any(|x| !x.is_finite())
        || loop_sizes.is_empty()
        || loop_sizes.len() > 16
        || loop_sizes.iter().any(|&n| n == 0 || n > 64)
        || loop_sizes.iter().sum::<usize>() != sweep.source_profiles().len()
    {
        return Err(fail(
            "Invalid original smooth filled-body limits or partition",
        ));
    }
    Ok(())
}
fn finish<'s, 'a>(
    sweep: &'s MultiSweep<'a>,
    loop_sizes: &[usize],
    source_caps: [&'s LoftCap; 2],
    projection: [[f64; 3]; 3],
    original_limits: OriginalSmoothVolumeLimits,
    attempt: Attempt,
) -> Result<OriginalSmoothBodyAttempt<'s, 'a>> {
    let l = original_limits;
    let body = match attempt {
        Attempt::Certified(body) => body,
        Attempt::Refused(r) => {
            let reason = r.reason.unwrap_or("retained-boundary-unproved");
            return Ok(refused(*r, None, OriginalSmoothBodyWork::default(), reason));
        }
    };
    let mut work = OriginalSmoothBodyWork::default();
    if !common_trim_region::certify(source_caps, &mut work.exact_work, l.exact_work)? {
        return Ok(refused(
            body.report,
            None,
            work,
            "original-smooth-body-common-trim-region-unproved",
        ));
    }
    for profile in sweep.source_profiles() {
        if work.cells == l.cells {
            return Ok(refused(
                body.report,
                None,
                work,
                "original-smooth-body-work-limit",
            ));
        }
        let proof =
            nurbs_core::curve_regularity::inspect(profile, (l.cells - work.cells).min(100000))?;
        work.cells += proof.cells;
        if !proof.spanwise_regular {
            return Ok(refused(
                body.report,
                None,
                work,
                "original-smooth-body-profile-regularity-unproved",
            ));
        }
    }
    let proof = sweep.certify_original_smooth_volume_atlas(
        [&source_caps[0].surface, &source_caps[1].surface],
        loop_sizes,
        projection,
        OriginalSmoothVolumeLimits {
            cells: l.cells - work.cells,
            exact_work: l.exact_work - work.exact_work,
            products: l.products - work.products,
            ..l
        },
    )?;
    let original = match proof {
        OriginalSmoothVolumeAttempt::Certified(proof) => proof,
        OriginalSmoothVolumeAttempt::Refused(r) => {
            work.cells += r.cells;
            work.exact_work += r.exact_work;
            work.products += r.products;
            let reason = r
                .reason
                .unwrap_or("original-smooth-body-embedding-unproved");
            return Ok(refused(body.report, Some(r), work, reason));
        }
    };
    work.cells += original.report().cells;
    work.exact_work += original.report().exact_work;
    work.products += original.report().products;
    Ok(OriginalSmoothBodyAttempt::Certified(Box::new(
        OriginalSmoothBody {
            retained: body,
            original,
            source_caps,
            work,
        },
    )))
}

/// Construct a coherent exact retained affine packet for general source frames
/// and filled caps. Complete original and retained proofs are freshly run;
/// candidate station corrections enter the full wall/cap error bound.
/// Here `affine.matrix_cells` is shared by fresh UV, actual filled-atlas and
/// orientation visits. `affine.work` includes candidate preparation and fresh
/// source/actual identities; no successful report from another model is used.
pub fn construct_affine_with_original_smooth_volume<'s, 'a>(
    source: SweepSource<'s, 'a, '_>,
    accuracy: SweepAccuracy,
    limits: Limits,
    projection: [[f64; 3]; 3],
    original_limits: OriginalSmoothVolumeLimits,
    affine: crate::AffineStationLimits,
) -> Result<OriginalSmoothBodyAttempt<'s, 'a>> {
    let SweepSource { sweep, loop_sizes, sections, source_caps } = source;
    let SweepAccuracy { tolerance_uv, error_budget } = accuracy;
    validate(sweep, loop_sizes, projection, original_limits)?;
    if affine.work > 1000000
        || affine.matrix_cells > 100000
        || !affine.quantum.is_normal()
        || affine.quantum <= 0.
        || affine.quantum.to_bits() & ((1u64 << 52) - 1) != 0
        || !affine.tolerance.is_finite()
        || affine.tolerance < 0.
    {
        return Err(fail("Invalid coherent affine boundary limits"));
    }
    let attempt = super::construct_impl(
        sweep,
        loop_sizes,
        sections,
        source_caps,
        source_caps,
        tolerance_uv,
        error_budget,
        limits,
        Some((affine, projection)),
    )?;
    finish(
        sweep,
        loop_sizes,
        source_caps,
        projection,
        original_limits,
        attempt,
    )
}
