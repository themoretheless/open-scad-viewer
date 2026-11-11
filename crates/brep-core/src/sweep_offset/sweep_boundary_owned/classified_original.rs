//! One immutable owner for classified ORIGINAL material and its retained Solid.
//! Both stages freshly use the same sweep, source caps and complete partition.
use super::*;
use nurbs_core::progressive_sweep::OriginalProfileClass;

/// Original proof and retained proof have independently explicit aggregate
/// allowances. No failed stage returns a model or accepts successful reports.
pub struct OriginalClassifiedBodyRefusal {
    pub original: Option<Box<OriginalClassifiedCappedMaterialRefusal>>,
    pub retained: Option<Box<Report>>,
    pub original_work: OriginalClassifiedCappedMaterialWork,
    pub reason: &'static str,
}
/// The private constructor retains source-law embedding, exact material trims,
/// classified original joins, full boundary error and actual Solid admission.
/// Original Gk is not inherited by the retained approximation's own seams.
pub struct OriginalClassifiedBody<'s, 'a> {
    original: Box<OriginalClassifiedCappedMaterial<'s, 'a>>,
    retained: Box<CertifiedBody>,
}
impl<'s, 'a> OriginalClassifiedBody<'s, 'a> {
    pub fn original(&self) -> &OriginalClassifiedCappedMaterial<'s, 'a> {
        &self.original
    }
    pub fn model(&self) -> &Model {
        self.retained.model()
    }
    pub fn retained_report(&self) -> &Report {
        self.retained.report()
    }
    pub fn boundary_classes(&self) -> &[OriginalProfileClass] {
        self.original.boundary_classes()
    }
    pub fn cap_wall_continuity(&self) -> &'static str {
        "C0"
    }
    pub fn into_model(self) -> Model {
        (*self.retained).into_model()
    }
}
pub enum OriginalClassifiedBodyAttempt<'s, 'a> {
    Certified(Box<OriginalClassifiedBody<'s, 'a>>),
    Refused(Box<OriginalClassifiedBodyRefusal>),
}
/// Fresh original material/embedding and complete retained boundary proof.
/// Each source cap bounds the SAME original profile rings under normalized
/// clocks. Their common material region transports through the globally
/// regular injective ideal map. The retained walls come from that same sweep;
/// endpoint cap errors cover the complete filled interiors, including holes.
/// There is no exact retained/ideal identity or ambient-isotopy claim: the
/// retained body owns its separate geometry/topology proof and error bound.
/// Sharp authored joins and open cap/wall corners remain explicitly C0.
pub fn construct_with_original_classified_material<'s, 'a>(
    source: SweepSource<'s, 'a, '_>,
    retained_caps: [&LoftCap; 2],
    classes: &[OriginalProfileClass],
    accuracy: SweepAccuracy,
    limits: Limits,
    projection: [[f64; 3]; 3],
    original_limits: OriginalClassifiedCappedMaterialLimits,
) -> Result<OriginalClassifiedBodyAttempt<'s, 'a>> {
    let SweepSource { sweep, loop_sizes, sections, source_caps } = source;
    let SweepAccuracy { tolerance_uv, error_budget } = accuracy;
    finish(
        sweep,
        loop_sizes,
        source_caps,
        classes,
        tolerance_uv,
        projection,
        original_limits,
        || {
            construct(
                sweep,
                loop_sizes,
                sections,
                source_caps,
                retained_caps,
                tolerance_uv,
                error_budget,
                limits,
            )
        },
    )
}
/// Same original classified owner with the coherent affine retained route.
/// Affine packet correction, actual filled admission and all wall/cap errors
/// retain their existing aggregate limits. Source classes are never inferred
/// from the corrected retained coefficients or from a successful report.
pub fn construct_affine_with_original_classified_material<'s, 'a>(
    source: SweepSource<'s, 'a, '_>,
    classes: &[OriginalProfileClass],
    accuracy: SweepAccuracy,
    limits: Limits,
    projection: [[f64; 3]; 3],
    original_limits: OriginalClassifiedCappedMaterialLimits,
    affine: crate::AffineStationLimits,
) -> Result<OriginalClassifiedBodyAttempt<'s, 'a>> {
    let SweepSource { sweep, loop_sizes, sections, source_caps } = source;
    let SweepAccuracy { tolerance_uv, error_budget } = accuracy;
    if affine.work > 1000000
        || affine.matrix_cells > 100000
        || !affine.quantum.is_normal()
        || affine.quantum <= 0.
        || affine.quantum.to_bits() & ((1u64 << 52) - 1) != 0
        || !affine.tolerance.is_finite()
        || affine.tolerance < 0.
    {
        return Err(fail("Invalid coherent classified boundary limits"));
    }
    finish(
        sweep,
        loop_sizes,
        source_caps,
        classes,
        tolerance_uv,
        projection,
        original_limits,
        || {
            super::construct_impl(
                sweep,
                loop_sizes,
                sections,
                source_caps,
                source_caps,
                tolerance_uv,
                error_budget,
                limits,
                Some((affine, projection)),
            )
        },
    )
}
fn finish<'s, 'a>(
    sweep: &'s MultiSweep<'a>,
    loop_sizes: &[usize],
    source_caps: [&'s LoftCap; 2],
    classes: &[OriginalProfileClass],
    tolerance_uv: f64,
    projection: [[f64; 3]; 3],
    original_limits: OriginalClassifiedCappedMaterialLimits,
    retained: impl FnOnce() -> Result<Attempt>,
) -> Result<OriginalClassifiedBodyAttempt<'s, 'a>> {
    let original = certify_original_classified_capped_material(
        sweep,
        loop_sizes,
        source_caps,
        classes,
        projection,
        tolerance_uv,
        original_limits,
    )?;
    let original = match original {
        OriginalClassifiedCappedMaterialAttempt::Certified(p) => p,
        OriginalClassifiedCappedMaterialAttempt::Refused(r) => {
            return Ok(OriginalClassifiedBodyAttempt::Refused(Box::new(
                OriginalClassifiedBodyRefusal {
                    original_work: r.work,
                    reason: r.reason.unwrap_or("original-classified-material-unproved"),
                    original: Some(r),
                    retained: None,
                },
            )));
        }
    };
    let retained = match retained()? {
        Attempt::Certified(p) => p,
        Attempt::Refused(r) => {
            return Ok(OriginalClassifiedBodyAttempt::Refused(Box::new(
                OriginalClassifiedBodyRefusal {
                    original_work: original.work(),
                    reason: r.reason.unwrap_or("retained-classified-material-unproved"),
                    original: None,
                    retained: Some(r),
                },
            )));
        }
    };
    Ok(OriginalClassifiedBodyAttempt::Certified(Box::new(
        OriginalClassifiedBody { original, retained },
    )))
}
