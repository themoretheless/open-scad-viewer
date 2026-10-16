//! Closed constructor-owned material volume. The carrier is a reference filled
//! cross-section, not a physical end cap. Every original/retained gate is owned
//! here; callers cannot supply a model, a successful report or an allowed pair.
use super::*;
use nurbs_core::{curve_regularity, trim_region_audit};

#[derive(Clone, Copy)]
pub struct MaterialLimits {
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
}
#[derive(Clone, Copy)]
pub struct ClosedLimits {
    pub boundary: Limits,
    pub material: MaterialLimits,
    pub ideal: IdealLimits,
    pub wall_faces: usize,
}
/// Only original material, ideal volume and original/retained boundary-error
/// preparation. Actual-model regularity/volume/smoothness have separate owners.
#[derive(Clone, Copy)]
pub struct OriginalClosedLimits {
    pub source_cells: usize,
    pub source_products: usize,
    pub source_exact_work: u64,
    pub material: MaterialLimits,
    pub ideal: IdealLimits,
    pub wall_faces: usize,
}
impl From<ClosedLimits> for OriginalClosedLimits {
    fn from(l: ClosedLimits) -> Self {
        Self {
            source_cells: l.boundary.source_cells,
            source_products: l.boundary.source_products,
            source_exact_work: l.boundary.source_exact_work,
            material: l.material,
            ideal: l.ideal,
            wall_faces: l.wall_faces,
        }
    }
}
pub struct MaterialReport {
    /// Exact original contour/carrier binding and a valid UV material region.
    /// Embedding of the transported filled region is the separate ideal proof.
    pub certified: bool,
    pub trim: Option<trim_region_audit::Report>,
    pub profiles: Vec<curve_regularity::Report>,
    pub bound_spans: usize,
    pub exact_work: u64,
}
pub struct ClosedReport {
    pub certified: bool,
    pub continuous_bound: bool,
    pub within_budget: bool,
    pub boundary_error_upper: Option<f64>,
    pub wall: Option<LevelReport>,
    pub decomposition_error_upper: Option<f64>,
    pub source_cells: usize,
    pub source_products: usize,
    /// Original binding and retained domain ownership share this exact ledger.
    pub ownership_exact_work: u64,
    pub material: MaterialReport,
    pub retained_walls: Option<crate::sweep_retained_walls::Report>,
    pub regularity: Option<regularity::Report>,
    pub volume: Option<volume_validity::Report>,
    pub ideal_volume: Option<nurbs_core::progressive_sweep::IdealVolumeReport>,
    pub reason: Option<&'static str>,
}
pub struct ClosedBody {
    model: Model,
    report: ClosedReport,
}
impl ClosedBody {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn report(&self) -> &ClosedReport {
        &self.report
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
pub enum ClosedAttempt {
    Certified(Box<ClosedBody>),
    Refused(Box<ClosedReport>),
}
fn refused(mut report: ClosedReport, reason: &'static str) -> ClosedAttempt {
    report.reason = Some(reason);
    ClosedAttempt::Refused(Box::new(report))
}

struct PreparedSource {
    report: ClosedReport,
    profiles: Vec<Vec<Curve>>,
    material: LoftCap,
    sections: Vec<Vec<Vec<Curve>>>,
}
enum PreparedAttempt {
    Ready(Box<PreparedSource>),
    Refused(Box<ClosedReport>),
}
fn refused_source(mut report: ClosedReport, reason: &'static str) -> PreparedAttempt {
    report.reason = Some(reason);
    PreparedAttempt::Refused(Box::new(report))
}
fn prepare_closed_source(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    material: &LoftCap,
    tolerance_uv: f64,
    error_budget: f64,
    limits: OriginalClosedLimits,
) -> Result<PreparedAttempt> {
    let b = limits;
    if !(4..=1025).contains(&sections)
        || !error_budget.is_finite()
        || error_budget <= 0.
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || b.source_cells > 100000
        || b.source_products > 1000000
        || b.source_exact_work > 1000000
        || limits.wall_faces > 1024
        || limits.material.pairs > 100000
        || limits.material.cells > 100000
        || limits.material.domain_cells > 1000000
        || limits.ideal.cells > 1000000
        || limits.ideal.exact_work > 1000000
        || limits
            .ideal
            .projection
            .iter()
            .flatten()
            .any(|x| !x.is_finite())
    {
        return Err(fail("Invalid closed material sweep limits"));
    }
    material.surface.validate()?;
    if material
        .surface
        .control_points
        .iter()
        .flatten()
        .any(|p| p.len() != 3)
        || material.trims.len() > 16
        || material.trims.iter().map(Vec::len).sum::<usize>() > 256
    {
        return Err(fail(
            "Original material requires XYZ carrier and bounded UV contours",
        ));
    }
    for trim in material.trims.iter().flatten() {
        trim.validate()?;
        if trim.control_points.iter().any(|p| p.len() != 2) {
            return Err(fail("Original material trims require UV controls"));
        }
    }
    let profiles = sweep.source_profiles().collect::<Vec<_>>();
    if loop_sizes.is_empty()
        || loop_sizes.len() > 16
        || loop_sizes.contains(&0)
        || loop_sizes.iter().any(|&n| n > profiles.len())
        || loop_sizes.iter().sum::<usize>() != profiles.len()
        || material.trims.len() != loop_sizes.len()
    {
        return Err(fail(
            "Closed sweep requires complete original material loop ownership",
        ));
    }
    let mut out = ClosedReport {
        certified: false,
        continuous_bound: false,
        within_budget: false,
        boundary_error_upper: None,
        wall: None,
        decomposition_error_upper: None,
        source_cells: 0,
        source_products: 0,
        ownership_exact_work: 0,
        material: MaterialReport {
            certified: false,
            trim: None,
            profiles: Vec::new(),
            bound_spans: 0,
            exact_work: 0,
        },
        retained_walls: None,
        regularity: None,
        volume: None,
        ideal_volume: None,
        reason: None,
    };
    if !sweep.source_is_closed()? {
        return Ok(refused_source(out, "original-closed-source-required"));
    }
    // The exact original UV chain may itself be multispan or one closed
    // curve. Normalize only through exact coefficient decomposition; rounded
    // UV extraction cannot prove the original material region or its binding.
    let mut trims = Vec::with_capacity(material.trims.len());
    let mut trim_count = 0usize;
    for ring in &material.trims {
        let mut pieces = Vec::new();
        for curve in ring {
            let rows = (curve.degree..curve.control_points.len())
                .filter(|&i| curve.knots[i] < curve.knots[i + 1])
                .count()
                * (curve.degree + 1);
            if rows > b.source_products - out.source_products {
                return Ok(refused_source(
                    out,
                    "original-material-uv-decomposition-work-limit",
                ));
            }
            out.source_products += rows;
            let Some(parts) =
                retained_wall_coefficients::exact_bezier_controls(curve, rows.min(4096))
            else {
                return Ok(refused_source(
                    out,
                    "original-material-exact-uv-decomposition-unproved",
                ));
            };
            trim_count += parts.len();
            if trim_count > 256 {
                return Ok(refused_source(
                    out,
                    "original-material-trim-complexity-unproved",
                ));
            }
            pieces.extend(parts);
        }
        trims.push(pieces);
    }
    if limits.material.pairs == 0 || limits.material.cells == 0 || limits.material.domain_cells == 0
    {
        return Ok(refused_source(out, "original-material-region-work-limit"));
    }
    let trim = trim_region_audit::inspect(
        &trims,
        tolerance_uv,
        limits.material.pairs,
        limits.material.cells,
        limits.material.domain_cells,
    )?;
    let valid = trim.valid == Some(true);
    out.material.trim = Some(trim);
    if !valid {
        return Ok(refused_source(out, "original-material-region-unproved"));
    }
    let mut original_profiles = Vec::with_capacity(loop_sizes.len());
    let mut offset = 0;
    for (ring, &size) in loop_sizes.iter().enumerate() {
        let mut spans = Vec::new();
        for profile in &profiles[offset..offset + size] {
            let regular = curve_regularity::inspect(profile, b.source_cells - out.source_cells)?;
            out.source_cells += regular.cells;
            let proven = regular.spanwise_regular;
            out.material.profiles.push(regular);
            if !proven {
                return Ok(refused_source(
                    out,
                    "original-material-profile-regularity-unproved",
                ));
            }
            let rows = (profile.degree..profile.control_points.len())
                .filter(|&i| profile.knots[i] < profile.knots[i + 1])
                .count()
                * (profile.degree + 1);
            if rows > b.source_products - out.source_products {
                return Ok(refused_source(
                    out,
                    "original-material-decomposition-work-limit",
                ));
            }
            out.source_products += rows;
            let Some(parts) =
                retained_wall_coefficients::exact_bezier_controls(profile, rows.min(4096))
            else {
                return Ok(refused_source(
                    out,
                    "original-material-exact-decomposition-unproved",
                ));
            };
            spans.extend(parts);
        }
        offset += size;
        if spans.len() != trims[ring].len() {
            return Ok(refused_source(
                out,
                "original-material-contour-count-mismatch",
            ));
        }
        for (world, uv) in spans.iter().zip(&trims[ring]) {
            let remaining = b.source_exact_work - out.ownership_exact_work;
            let Some(proof) = nurbs_core::curve_surface_agreement::verify_exact(
                world,
                uv,
                &material.surface,
                false,
                remaining,
            )?
            else {
                out.ownership_exact_work = b.source_exact_work;
                out.material.exact_work = out.ownership_exact_work;
                return Ok(refused_source(out, "original-material-boundary-unproved"));
            };
            out.ownership_exact_work += proof.work_used;
            out.material.exact_work = out.ownership_exact_work;
            if proof.outcome != cad_predicates::BezierIdentity::Equal {
                return Ok(refused_source(out, "original-material-boundary-unproved"));
            }
            out.material.bound_spans += 1;
        }
        original_profiles.push(spans);
    }
    out.material.certified = true;
    let Some(faces) = out.material.bound_spans.checked_mul(sections - 1) else {
        return Err(fail("Closed periodic face count overflow"));
    };
    if faces > limits.wall_faces || limits.wall_faces == 0 {
        return Ok(refused_source(out, "periodic-wall-face-work-limit"));
    }
    let proof = sweep.certify_ideal_volume_atlas(
        [&material.surface, &material.surface],
        limits.ideal.projection,
        limits.ideal.cells,
        limits.ideal.exact_work,
    )?;
    let proven = proof.embedding_certified
        && proof.regularity_certified
        && proof.parameter_topology_preserved;
    let reason = proof.reason;
    out.ideal_volume = Some(proof);
    if !proven {
        return Ok(refused_source(
            out,
            reason.unwrap_or("original-closed-volume-unproved"),
        ));
    }
    let wall = sweep
        .preview_at_with_error_limits(
            sections,
            b.source_cells - out.source_cells,
            b.source_products - out.source_products,
        )?
        .report;
    out.source_cells += wall.error_certificate_cells;
    out.source_products += wall.decomposition_products;
    let error = wall
        .continuous_error_upper
        .filter(|_| wall.continuous_bound);
    out.wall = Some(wall);
    let Some(error) = error else {
        return Ok(refused_source(out, "original-periodic-wall-bound-unproved"));
    };
    // Bound copied station coefficients before allocating them. This work is
    // additional to source decomposition/error products, not an implicit budget.
    let rows_to_copy = profiles
        .iter()
        .map(|p| p.control_points.len())
        .sum::<usize>()
        * sections;
    if rows_to_copy > b.source_products - out.source_products {
        return Ok(refused_source(
            out,
            "periodic-section-construction-work-limit",
        ));
    }
    out.source_products += rows_to_copy;
    let rows = sweep.sections_at(sections)?;
    let mut original_sections = Vec::with_capacity(sections);
    for station in 0..sections {
        let mut rings = Vec::new();
        let mut offset = 0;
        for &size in loop_sizes {
            rings.push(
                rows[offset..offset + size]
                    .iter()
                    .map(|r| r[station].clone())
                    .collect(),
            );
            offset += size;
        }
        original_sections.push(rings);
    }
    let (retained, decomposition, products) = crate::analytic::retained_section_partition(
        &original_sections,
        b.source_products - out.source_products,
    )?;
    out.source_products += products;
    out.decomposition_error_upper = decomposition;
    let Some(upper) = decomposition.and_then(|d| error_upper::add(error, d)) else {
        return Ok(refused_source(
            out,
            "periodic-wall-decomposition-bound-unproved",
        ));
    };
    // The common rational basis is checked by the partition owner before this
    // bound propagates over each linear station interval. No sampled gap credit.
    out.boundary_error_upper = Some(upper);
    out.within_budget = upper <= error_budget;
    if !out.within_budget {
        return Ok(refused_source(out, "boundary-error-budget-exceeded"));
    }
    Ok(PreparedAttempt::Ready(Box::new(PreparedSource {
        report: out,
        profiles: original_profiles,
        material: LoftCap {
            surface: material.surface.clone(),
            trims,
        },
        sections: retained,
    })))
}

/// Prove an actual closed periodic Solid and its ORIGINAL filled source volume.
/// Sharp profile/station joins remain C0; this operation issues no G1/G2 claim.
/// `material` must describe all original outer/hole contours in its own UV
/// domain. Its interior travels around S1 and creates no physical end caps.
/// Budgets are explicit per stage, as in the retained volume verifier.
pub fn construct_closed_with_ideal_atlas(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    material: &LoftCap,
    tolerance_uv: f64,
    error_budget: f64,
    limits: ClosedLimits,
) -> Result<ClosedAttempt> {
    if limits.boundary.regularity_cells > 100000 || limits.boundary.linear_cells > 100000 {
        return Err(fail("Invalid closed material sweep limits"));
    }
    let prepared = match prepare_closed_source(
        sweep,
        loop_sizes,
        sections,
        material,
        tolerance_uv,
        error_budget,
        limits.into(),
    )? {
        PreparedAttempt::Ready(p) => p,
        PreparedAttempt::Refused(r) => return Ok(ClosedAttempt::Refused(r)),
    };
    let b = limits.boundary;
    let mut out = prepared.report;
    let retained = prepared.sections;
    let faces = retained[0].iter().map(Vec::len).sum::<usize>() * (sections - 1);
    if faces > limits.wall_faces || limits.wall_faces == 0 {
        return Ok(refused(out, "periodic-wall-face-work-limit"));
    }
    let model = match crate::analytic::periodic_material_section_loft(&retained) {
        Ok(model) => model,
        Err(e) if e.code == "BREP_RATIONAL_SWEEP_REFUSED" => {
            return Ok(refused(out, "periodic-wall-construction-unproved"));
        }
        Err(e) => return Err(e),
    };
    let walls = crate::sweep_retained_walls::inspect(
        &model,
        &retained,
        true,
        limits.wall_faces,
        b.source_exact_work - out.ownership_exact_work,
    )?;
    out.ownership_exact_work += walls.exact_work;
    let owned = walls.certified;
    out.retained_walls = Some(walls);
    if !owned {
        return Ok(refused(out, "periodic-wall-ownership-unproved"));
    }
    out.continuous_bound = true;
    let regular = regularity::inspect(&model, b.regularity_cells)?;
    let proven = regular.all_faces_regular_modulo_poles;
    out.regularity = Some(regular);
    if !proven {
        return Ok(refused(out, "retained-periodic-regularity-unproved"));
    }
    let v = b.volume;
    if v.boundary.exact_work == 0
        || v.boundary.trim_pairs == 0
        || v.boundary.trim_cells == 0
        || v.boundary.trim_domain_cells == 0
        || v.boundary.spans == 0
        || v.boundary.contacts.pairs == 0
        || v.boundary.contacts.cells == 0
        || v.boundary.contacts.domain_cells == 0
        || v.boundary.contacts.cells_per_pair == 0
        || v.boundary.contacts.domain_cells_per_pair == 0
        || v.nesting_pairs == 0
        || v.nesting_cells == 0
        || v.nesting_domain_cells == 0
        || v.orientation_cells == 0
        || v.orientation_domain_cells == 0
        || v.orientation_spans == 0
    {
        return Ok(refused(out, "retained-periodic-volume-work-limit"));
    }
    // Closed wall hull ranges are independently certified on actual stored
    // coefficients. Empty cap selection cannot authorize a cap/wall shortcut.
    let volume = volume_validity::inspect_sweep(
        &model,
        tolerance_uv,
        v,
        b.linear_cells,
        &[],
        crate::sweep_cap_contacts::Budgets {
            max_walls: 0,
            max_exact_work: 0,
            max_chart_cells: 0,
            max_trim_pairs: 0,
            max_trim_cells: 0,
            max_trim_domain_cells: 0,
        },
    )?;
    let proven = volume.proven;
    out.volume = Some(volume);
    if !proven {
        return Ok(refused(out, "retained-periodic-volume-unproved"));
    }
    out.certified = true;
    Ok(ClosedAttempt::Certified(Box::new(ClosedBody {
        model,
        report: out,
    })))
}

#[cfg(test)]
mod tests;

mod smooth;
mod rational;
pub use rational::{RationalClosedLimits,RationalClosedReport,RationalClosedBody,RationalClosedAttempt,FilledGraphCapReport,
    construct_rational_closed_material};
pub use smooth::{
    BindingReport as AffineWallBindingReport, OriginalReport as OriginalClosedReport,
    SmoothClosedAttempt, SmoothClosedBody, SmoothClosedLimits, SmoothClosedReport,
    construct_smooth_closed_with_ideal_atlas, verify_smooth_closed_with_ideal_atlas,
};
