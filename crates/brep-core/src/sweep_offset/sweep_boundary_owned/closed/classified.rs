//! One owned closed Solid with classified ORIGINAL and actual boundary proofs.
use super::*;
use crate::affine_station_walls;
use nurbs_core::{affine_volume, progressive_sweep::OriginalProfileClass};
mod roles;
pub use roles::{Report as EdgeRoles, Role as EdgeRole};
#[derive(Clone, Copy)]
pub struct ClassifiedClosedLimits {
    pub original: OriginalClassifiedClosedMaterialLimits,
    /// Existing bounded source/error, affine candidate, binding, actual-volume
    /// and orientation stages. smoothness.order requests STATION seam order;
    /// profile orders come from the independently proved source class array.
    /// original_smoothness must be None: source requests are classified above.
    pub retained: smooth::SmoothClosedLimits,
    /// ONE ledger for native edge-role derivation and complete actual census.
    pub incidence_steps: usize,
}
pub struct ClassifiedClosedReport {
    pub certified: bool,
    pub original_refusal: Option<Box<OriginalClassifiedClosedMaterialRefusal>>,
    pub original_work: Option<OriginalClassifiedClosedMaterialWork>,
    pub source: Option<Box<ClosedReport>>,
    pub candidate_work: u64,
    pub candidate_matrix_cells: usize,
    pub displacement_upper: Option<f64>,
    pub boundary_error_upper: Option<f64>,
    pub binding: Option<smooth::BindingReport>,
    pub edge_roles: Option<EdgeRoles>,
    pub boundary: Option<crate::sweep_smoothness::complete::classified::Report>,
    pub incidence_steps: usize,
    pub volume: Option<nurbs_core::progressive_sweep::IdealVolumeReport>,
    pub orientation: Option<affine_volume::OrientationReport>,
    pub orientation_cells: usize,
    pub reason: &'static str,
}
pub struct ClassifiedClosedBody<'s, 'a> {
    original: Box<OriginalClassifiedClosedMaterial<'s, 'a>>,
    model: Model,
    report: ClassifiedClosedReport,
}
impl<'s, 'a> ClassifiedClosedBody<'s, 'a> {
    pub fn original(&self) -> &OriginalClassifiedClosedMaterial<'s, 'a> {
        &self.original
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn report(&self) -> &ClassifiedClosedReport {
        &self.report
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
pub enum ClassifiedClosedAttempt<'s, 'a> {
    Certified(Box<ClassifiedClosedBody<'s, 'a>>),
    Refused(Box<ClassifiedClosedReport>),
}
fn refuse<'s, 'a>(
    mut out: ClassifiedClosedReport,
    reason: &'static str,
) -> ClassifiedClosedAttempt<'s, 'a> {
    out.reason = reason;
    ClassifiedClosedAttempt::Refused(Box::new(out))
}
/// Fresh complete native proofs on one original closed material packet.
/// Source profile classes and closed Ck motion remain original guarantees;
/// every actual edge is independently classified from exact coefficient-bound
/// source roles and freshly proved at that order. Explicit C0 survives even
/// when stronger geometry happens to hold. Unknown stages expose no model.
pub fn construct_classified_closed_with_original_material<'s, 'a>(
    sweep: &'s MultiSweep<'a>,
    loop_sizes: &[usize],
    sections: usize,
    material: &'s LoftCap,
    classes: &[OriginalProfileClass],
    tolerance_uv: f64,
    error_budget: f64,
    sharp_stations: &[usize],
    limits: ClassifiedClosedLimits,
) -> Result<ClassifiedClosedAttempt<'s, 'a>> {
    construct_impl(
        sweep,
        loop_sizes,
        sections,
        material,
        classes,
        tolerance_uv,
        error_budget,
        sharp_stations,
        limits,
        None,
    )
}
/// Supplied geometry is freshly rebound, classified and globally requalified.
/// No source/actual success reports or caller-proposed edge class list is used.
pub fn verify_classified_closed_with_original_material<'s, 'a>(
    sweep: &'s MultiSweep<'a>,
    loop_sizes: &[usize],
    sections: usize,
    material: &'s LoftCap,
    classes: &[OriginalProfileClass],
    tolerance_uv: f64,
    error_budget: f64,
    sharp_stations: &[usize],
    limits: ClassifiedClosedLimits,
    model: &Model,
) -> Result<ClassifiedClosedAttempt<'s, 'a>> {
    model.validate()?;
    construct_impl(
        sweep,
        loop_sizes,
        sections,
        material,
        classes,
        tolerance_uv,
        error_budget,
        sharp_stations,
        limits,
        Some(model),
    )
}
fn construct_impl<'s, 'a>(
    sweep: &'s MultiSweep<'a>,
    loop_sizes: &[usize],
    sections: usize,
    material: &'s LoftCap,
    classes: &[OriginalProfileClass],
    tolerance_uv: f64,
    error_budget: f64,
    sharp: &[usize],
    limits: ClassifiedClosedLimits,
    supplied: Option<&Model>,
) -> Result<ClassifiedClosedAttempt<'s, 'a>> {
    let l = limits.retained;
    if limits.incidence_steps > 1000000
        || l.binding_work > 1000000
        || l.orientation_cells > 100000
        || l.atlas.cells > 1000000
        || l.atlas.exact_work > 1000000
        || l.atlas
            .projection
            .iter()
            .flatten()
            .any(|x| !x.is_finite() || x.abs() > 1e9)
        || !matches!(l.smoothness.order, 1 | 2)
        || l.original_smoothness.is_some()
        || l.smoothness.exact_work > nurbs_core::continuity::MAX_PROJECTIVE_SEAM_COLLECTION_WORK
        || l.smoothness.agreement_work > 1000000
        || l.smoothness.regularity_cells > 100000
    {
        return Err(fail(
            "Choose bounded classified closed source and actual requests",
        ));
    }
    let mut out = ClassifiedClosedReport {
        certified: false,
        original_refusal: None,
        original_work: None,
        source: None,
        candidate_work: 0,
        candidate_matrix_cells: 0,
        displacement_upper: None,
        boundary_error_upper: None,
        binding: None,
        edge_roles: None,
        boundary: None,
        incidence_steps: 0,
        volume: None,
        orientation: None,
        orientation_cells: 0,
        reason: "classified-closed-unproven",
    };
    let original = match certify_original_classified_closed_material(
        sweep,
        loop_sizes,
        material,
        classes,
        l.atlas.projection,
        tolerance_uv,
        limits.original,
    )? {
        OriginalClassifiedClosedMaterialAttempt::Certified(p) => p,
        OriginalClassifiedClosedMaterialAttempt::Refused(r) => {
            let reason = r
                .reason
                .unwrap_or("classified-closed-original-material-unproved");
            out.original_work = Some(r.work);
            out.original_refusal = Some(r);
            return Ok(refuse(out, reason));
        }
    };
    out.original_work = Some(original.work());
    let prepared = match prepare_closed_source_with_binding(
        sweep,
        loop_sizes,
        sections,
        material,
        tolerance_uv,
        error_budget,
        l.source,
        MaterialBinding::OrientedImages,
    )? {
        PreparedAttempt::Ready(p) => *p,
        PreparedAttempt::Refused(r) => {
            let reason = r
                .reason
                .unwrap_or("classified-closed-source-error-unproved");
            out.source = Some(r);
            return Ok(refuse(out, reason));
        }
    };
    let PreparedSource {
        report: source,
        profiles,
        material: bound_material,
        sections: rows,
    } = prepared;
    out.source = Some(Box::new(source));
    let candidate = affine_station_walls(&profiles, &rows, sharp, true, l.candidate)?;
    out.candidate_work = candidate.work;
    out.candidate_matrix_cells = candidate.matrix_cells;
    out.displacement_upper = candidate.displacement_upper;
    let (Some(sides), Some(stations), Some(poses), Some(origin), Some(delta)) = (
        candidate.sides,
        candidate.sections,
        candidate.poses,
        candidate.source_origin,
        candidate.displacement_upper,
    ) else {
        return Ok(refuse(
            out,
            "classified-closed-exact-affine-candidate-unproved",
        ));
    };
    let Some(error) = out
        .source
        .as_ref()
        .unwrap()
        .boundary_error_upper
        .and_then(|e| error_upper::add(e, delta))
    else {
        return Ok(refuse(out, "classified-closed-boundary-error-unproved"));
    };
    out.boundary_error_upper = Some(error);
    if error > error_budget {
        return Ok(refuse(out, "classified-closed-boundary-error-budget"));
    }
    let model = if let Some(m) = supplied {
        m.clone()
    } else {
        match crate::analytic::periodic_material_loft_surfaces(&stations, &sides) {
            Ok(m) => m,
            Err(e) if e.code == "BREP_RATIONAL_SWEEP_REFUSED" => {
                return Ok(refuse(out, "classified-closed-actual-topology-unproved"));
            }
            Err(e) => return Err(e),
        }
    };
    let binding = smooth::binding(&model, &profiles, &poses, origin, l.binding_work)?;
    let proven = binding.certified;
    out.binding = Some(binding);
    if !proven {
        return Ok(refuse(out, "classified-closed-actual-binding-unproved"));
    }
    let roles = roles::inspect(
        &model,
        &profiles,
        poses.len(),
        out.binding.as_ref().unwrap(),
        original.boundary_classes(),
        sharp,
        l.smoothness.order,
        limits.incidence_steps,
    )?;
    let proven = roles.certified;
    out.incidence_steps = roles.steps;
    out.edge_roles = Some(roles);
    if !proven {
        return Ok(refuse(out, "classified-closed-source-edge-roles-unproved"));
    }
    let boundary = crate::sweep_smoothness::complete::classified::inspect(
        &model,
        &out.edge_roles.as_ref().unwrap().classes,
        crate::sweep_smoothness::complete::classified::Limits {
            incidence_steps: limits.incidence_steps - out.incidence_steps,
            exact_work: l.smoothness.exact_work,
            agreement_work: l.smoothness.agreement_work,
            regularity_cells: l.smoothness.regularity_cells,
        },
    )?;
    out.incidence_steps += boundary.steps;
    let proven = boundary.certified;
    out.boundary = Some(boundary);
    if !proven {
        return Ok(refuse(
            out,
            "classified-closed-complete-actual-boundary-unproved",
        ));
    }
    let atlas = affine_volume::certify(
        &bound_material.surface,
        origin,
        &poses,
        true,
        l.atlas.projection,
        l.atlas.cells,
        l.atlas.exact_work,
    )?;
    let proven = atlas.embedding_certified
        && atlas.regularity_certified
        && atlas.parameter_topology_preserved;
    out.volume = Some(atlas);
    if !proven {
        return Ok(refuse(
            out,
            "classified-closed-global-actual-volume-unproved",
        ));
    }
    let Some(trim) = out.source.as_ref().unwrap().material.trim.as_ref() else {
        return Ok(refuse(out, "classified-closed-material-roles-unproved"));
    };
    let Some(winding) = trim
        .winding
        .first()
        .copied()
        .flatten()
        .filter(|w| w.abs() == 1)
    else {
        return Ok(refuse(out, "classified-closed-material-roles-unproved"));
    };
    if trim.valid != Some(true)
        || trim.winding.len() != profiles.len()
        || trim.winding.iter().skip(1).any(|w| *w != Some(-winding))
    {
        return Ok(refuse(out, "classified-closed-material-roles-unproved"));
    }
    let orientation = affine_volume::jacobian_orientation(
        &bound_material.surface,
        origin,
        &poses,
        [0.5; 3],
        l.orientation_cells,
    )?;
    out.orientation_cells = orientation.cells;
    let sign = orientation.sign;
    out.orientation = Some(orientation);
    let Some(sign) = sign else {
        return Ok(refuse(out, "classified-closed-world-orientation-unproved"));
    };
    let reversed = i32::from(sign) * winding < 0;
    for u in model.shells.iter().flat_map(|s| &s.faces) {
        if out.orientation_cells == l.orientation_cells {
            return Ok(refuse(out, "classified-closed-orientation-work-limit"));
        }
        out.orientation_cells += 1;
        let Some(gauge) = out.binding.as_ref().unwrap().orientation_reversed(u.face) else {
            return Ok(refuse(out, "classified-closed-chart-orientation-unproved"));
        };
        if u.reversed != (reversed ^ gauge) {
            return Ok(refuse(
                out,
                "classified-closed-material-orientation-unproved",
            ));
        }
    }
    out.certified = true;
    out.reason = "classified-original-and-actual-closed-material-qualified";
    Ok(ClassifiedClosedAttempt::Certified(Box::new(
        ClassifiedClosedBody {
            original,
            model,
            report: out,
        },
    )))
}
#[cfg(test)]
mod tests;
