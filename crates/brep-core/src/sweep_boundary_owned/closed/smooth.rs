//! Constructor-owned G1/G2 Solid from one original filled material chart and
//! one exact affine quintic motion. Audit recomputes supplied geometry; no
//! caller report or allowed pair list supplies proof credit.
use super::*;
use crate::{AffinePose, AffineStationLimits, affine_station_walls};
use nurbs_core::{
    affine_volume,
    retained_wall_domain_certificate::{self, Coedge},
};

#[derive(Clone, Copy)]
pub struct SmoothClosedLimits {
    pub source: OriginalClosedLimits,
    /// When requested, original differential compatibility is a strict stage.
    pub original_smoothness:
        Option<nurbs_core::progressive_sweep::OriginalBoundarySmoothnessLimits>,
    pub candidate: AffineStationLimits,
    pub smoothness: crate::sweep_smoothness::complete::Limits,
    pub atlas: IdealLimits,
    pub binding_work: u64,
    pub orientation_cells: usize,
}
/// Original regularity/embedding/topology and boundary-error stages. Optional
/// differential compatibility completes Gk only together with these owned
/// original volume, material and trace proofs.
pub struct OriginalReport {
    pub certified: bool,
    pub material: MaterialReport,
    pub volume: Option<nurbs_core::progressive_sweep::IdealVolumeReport>,
    pub wall: Option<LevelReport>,
    pub boundary_error_upper: Option<f64>,
    pub decomposition_error_upper: Option<f64>,
    pub cells: usize,
    pub products: usize,
    pub exact_work: u64,
    pub reason: Option<&'static str>,
    pub smoothness: Option<nurbs_core::progressive_sweep::OriginalBoundarySmoothnessReport>,
}
impl OriginalReport {
    pub fn g1_certified(&self) -> bool {
        self.certified
            && self
                .smoothness
                .as_ref()
                .is_some_and(|p| p.differential_compatibility_certified)
    }
    pub fn g2_certified(&self) -> bool {
        self.g1_certified() && self.smoothness.as_ref().is_some_and(|p| p.order == 2)
    }
}
impl From<ClosedReport> for OriginalReport {
    fn from(r: ClosedReport) -> Self {
        let certified = r.reason.is_none()
            && r.within_budget
            && r.material.certified
            && r.ideal_volume.as_ref().is_some_and(|p| {
                p.embedding_certified && p.regularity_certified && p.parameter_topology_preserved
            })
            && r.wall.as_ref().is_some_and(|w| w.continuous_bound)
            && r.boundary_error_upper.is_some()
            && r.decomposition_error_upper.is_some();
        Self {
            certified,
            material: r.material,
            volume: r.ideal_volume,
            wall: r.wall,
            boundary_error_upper: r.boundary_error_upper,
            decomposition_error_upper: r.decomposition_error_upper,
            cells: r.source_cells,
            products: r.source_products,
            exact_work: r.ownership_exact_work,
            reason: r.reason,
            smoothness: None,
        }
    }
}
pub struct BindingReport {
    pub certified: bool,
    pub faces: usize,
    pub exact_work: u64,
    pub reason: &'static str,
    // Source-to-actual signed tensor coordinate permutations, one per face.
    gauges: Vec<u8>,
    source_slots: Vec<usize>,
}
impl BindingReport {
    pub(super) fn source_face(&self, slot: usize) -> Option<usize> {
        self.certified
            .then(|| self.source_slots.get(slot).copied())
            .flatten()
            .filter(|i| *i != usize::MAX)
    }
    pub fn reparameterized_faces(&self) -> usize {
        self.gauges.iter().filter(|&&g| g != 0).count()
    }
    pub(super) fn orientation_reversed(&self, face: usize) -> Option<bool> {
        self.gauges.get(face).map(|g| g.count_ones() % 2 != 0)
    }
}
pub struct SmoothClosedReport {
    pub certified: bool,
    pub original: OriginalReport,
    pub continuous_bound: bool,
    pub within_budget: bool,
    pub boundary_error_upper: Option<f64>,
    pub displacement_upper: Option<f64>,
    pub candidate_work: u64,
    pub matrix_cells: usize,
    pub candidate_reason: Option<&'static str>,
    pub binding: Option<BindingReport>,
    pub smoothness: Option<crate::sweep_smoothness::complete::Report>,
    pub volume: Option<nurbs_core::progressive_sweep::IdealVolumeReport>,
    pub orientation: Option<affine_volume::OrientationReport>,
    pub orientation_cells: usize,
    pub shell_roles_certified: bool,
    pub topology_preserved: bool,
    pub reason: Option<&'static str>,
}
pub struct SmoothClosedBody {
    model: Model,
    report: SmoothClosedReport,
}
impl SmoothClosedBody {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn report(&self) -> &SmoothClosedReport {
        &self.report
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
pub enum SmoothClosedAttempt {
    Certified(Box<SmoothClosedBody>),
    Refused(Box<SmoothClosedReport>),
}
fn refused(mut report: SmoothClosedReport, reason: &'static str) -> SmoothClosedAttempt {
    report.reason = Some(reason);
    SmoothClosedAttempt::Refused(Box::new(report))
}
fn report(original: ClosedReport) -> SmoothClosedReport {
    SmoothClosedReport {
        certified: false,
        original: original.into(),
        continuous_bound: false,
        within_budget: false,
        boundary_error_upper: None,
        displacement_upper: None,
        candidate_work: 0,
        matrix_cells: 0,
        candidate_reason: None,
        binding: None,
        smoothness: None,
        volume: None,
        orientation: None,
        orientation_cells: 0,
        shell_roles_certified: false,
        topology_preserved: false,
        reason: None,
    }
}
fn exact_add(a: f64, b: f64) -> Option<f64> {
    let s = a + b;
    let bv = s - a;
    let av = s - bv;
    let br = b - bv;
    let ar = a - av;
    (s.is_finite() && ar + br == 0.).then_some(s)
}
pub(super) fn exact_mul(a: f64, b: f64) -> Option<f64> {
    if a == 1. && b.is_finite() {
        return Some(b);
    }
    if b == 1. && a.is_finite() {
        return Some(a);
    }
    if a == 0. || b == 0. {
        return Some(0.);
    }
    let p = a * b;
    (p.is_finite()
        && p.abs() >= 1e-100
        && a.abs() >= 1e-100
        && b.abs() >= 1e-100
        && a.mul_add(b, -p) == 0.)
        .then_some(p)
}
pub(super) fn image(pose: AffinePose, p: &[f64], origin: [f64; 3]) -> Option<[f64; 3]> {
    let p = [
        exact_add(p[0], -origin[0])?,
        exact_add(p[1], -origin[1])?,
        exact_add(p[2], -origin[2])?,
    ];
    let mut q = pose[3];
    for col in 0..3 {
        for k in 0..3 {
            q[k] = exact_add(q[k], exact_mul(pose[col][k], p[col])?)?;
        }
    }
    Some(q)
}
fn matches_gauge(
    model: &Model,
    index: usize,
    source: &Curve,
    pose: &[AffinePose],
    time_weights: &[f64],
    origin: [f64; 3],
    out: &mut BindingReport,
    max: u64,
    gauge: u8,
) -> Result<bool> {
    let swap = gauge & 4 != 0;
    let flip_u = gauge & 1 != 0;
    let flip_v = gauge & 2 != 0;
    let degrees = if swap {
        [pose.len() - 1, source.degree]
    } else {
        [source.degree, pose.len() - 1]
    };
    let shape = if swap {
        [pose.len(), source.control_points.len()]
    } else {
        [source.control_points.len(), pose.len()]
    };
    let face = &model.faces[index];
    let s = &face.surface;
    out.reason = "affine-wall-coefficient-identity-unproved";
    if s.degree_u != degrees[0]
        || s.degree_v != degrees[1]
        || s.periodic_u
        || s.periodic_v
        || s.knots_u != [vec![0.; degrees[0] + 1], vec![1.; degrees[0] + 1]].concat()
        || s.knots_v != [vec![0.; degrees[1] + 1], vec![1.; degrees[1] + 1]].concat()
        || s.control_points.len() != shape[0]
        || s.control_points[0].len() != shape[1]
        || !face.holes.is_empty()
    {
        return Ok(false);
    }
    for (i, p) in source.control_points.iter().enumerate() {
        for j in 0..pose.len() {
            let Some(q) = image(pose[j], p, origin) else {
                return Ok(false);
            };
            let [mut u, mut v] = if swap { [j, i] } else { [i, j] };
            if flip_u {
                u = shape[0] - 1 - u;
            }
            if flip_v {
                v = shape[1] - 1 - v;
            }
            let Some(weight) = exact_mul(source.weights[i], time_weights[j]) else {
                return Ok(false);
            };
            for k in 0..3 {
                if out.exact_work == max {
                    out.reason = "affine-wall-binding-work-limit";
                    return Ok(false);
                }
                out.exact_work += 1;
                if s.weights[u][v] != weight || s.control_points[u][v][k] != q[k] {
                    return Ok(false);
                }
            }
        }
    }
    out.reason = "affine-wall-full-domain-unproved";
    let coedges = model.loops[face.outer]
        .coedges
        .iter()
        .map(|c| Coedge {
            world: &model.edges[c.edge].curve,
            uv: &c.pcurve,
            reversed: c.reversed,
        })
        .collect::<Vec<_>>();
    let proof =
        retained_wall_domain_certificate::inspect(s, &coedges, false, max - out.exact_work)?;
    out.exact_work += proof.work;
    if !proof.domain_certified {
        return Ok(false);
    }
    Ok(true)
}
// The eight exact symmetries of the normalized tensor square. Native
// construction tries identity first and retains its existing exact-work cost.
// Other gauges copy/reindex coefficients only: no rounded knot or pole refit.
fn matches_face(
    model: &Model,
    index: usize,
    source: &Curve,
    pose: &[AffinePose],
    time_weights: &[f64],
    origin: [f64; 3],
    out: &mut BindingReport,
    max: u64,
) -> Result<Option<u8>> {
    for gauge in 0..8 {
        if matches_gauge(
            model,
            index,
            source,
            pose,
            time_weights,
            origin,
            out,
            max,
            gauge,
        )? {
            return Ok(Some(gauge));
        }
        if out.exact_work == max {
            return Ok(None);
        }
    }
    Ok(None)
}
fn binding(
    model: &Model,
    profiles: &[Vec<Curve>],
    poses: &[[AffinePose; 6]],
    origin: [f64; 3],
    max: u64,
) -> Result<BindingReport> {
    let p = poses.iter().map(|p| p.as_slice()).collect::<Vec<_>>();
    let weights = vec![&[1.; 6][..]; poses.len()];
    binding_impl(model, profiles, &p, &weights, origin, max)
}
pub(super) fn binding_rational(
    model: &Model,
    profiles: &[Vec<Curve>],
    poses: &[nurbs_core::affine_volume::RationalPiece],
    origin: [f64; 3],
    max: u64,
) -> Result<BindingReport> {
    let p = poses
        .iter()
        .map(|p| p.controls.as_slice())
        .collect::<Vec<_>>();
    let w = poses
        .iter()
        .map(|p| p.weights.as_slice())
        .collect::<Vec<_>>();
    binding_impl(model, profiles, &p, &w, origin, max)
}
fn binding_impl(
    model: &Model,
    profiles: &[Vec<Curve>],
    poses: &[&[AffinePose]],
    time_weights: &[&[f64]],
    origin: [f64; 3],
    max: u64,
) -> Result<BindingReport> {
    model.validate()?;
    let mut out = BindingReport {
        certified: false,
        faces: 0,
        exact_work: 0,
        reason: "affine-wall-coverage-unproved",
        gauges: vec![0; model.faces.len()],
        source_slots: vec![usize::MAX; model.faces.len()],
    };
    let spans = profiles.iter().map(Vec::len).sum::<usize>();
    if model.faces.len() != spans * poses.len()
        || !crate::sweep_retained_walls::covers(model)
        || model.shells.len() != profiles.len()
        || model.bodies.len() != 1
        || model.bodies[0].outer_shell != 0
        || model.bodies[0].inner_shells != (1..profiles.len()).collect::<Vec<_>>()
    {
        return Ok(out);
    }
    let mut used = vec![false; model.faces.len()];
    let mut offset = 0;
    for (ring, wire) in profiles.iter().enumerate() {
        if model.shells[ring].faces.len() != poses.len() * wire.len() {
            return Ok(out);
        }
        for (layer, pose) in poses.iter().enumerate() {
            for (span, source) in wire.iter().enumerate() {
                let index = layer * spans + offset + span;
                // Native ordering is the first proposal. STEP may group outer
                // and inner shell faces differently. Every fallback comparison
                // shares this exact ledger; every actual face is used once.
                let shell = &model.shells[ring];
                let candidates = std::iter::once(index)
                    .filter(|&i| shell.faces.iter().any(|u| u.face == i))
                    .chain(shell.faces.iter().map(|u| u.face).filter(|&i| i != index));
                let mut matched = None;
                for actual in candidates {
                    if used[actual] {
                        continue;
                    }
                    if let Some(gauge) = matches_face(
                        model,
                        actual,
                        source,
                        pose,
                        time_weights[layer],
                        origin,
                        &mut out,
                        max,
                    )? {
                        matched = Some((actual, gauge));
                        break;
                    }
                    if out.exact_work == max {
                        return Ok(out);
                    }
                }
                let Some((actual, gauge)) = matched else {
                    return Ok(out);
                };
                used[actual] = true;
                out.gauges[actual] = gauge;
                out.source_slots[index] = actual;
                out.faces += 1;
            }
        }
        offset += wire.len();
    }
    if used.iter().any(|u| !*u) {
        return Ok(out);
    }
    out.certified = true;
    out.reason = "exact-affine-filled-carrier-wall-union";
    Ok(out)
}

/// Original ideal-volume proof, bounded exact affine reconstruction, complete
/// G1/G2 and actual filled-volume atlas own one returned immutable Solid.
/// A source-stage C0 retained Solid is not required or borrowed as a global
/// proof for the new model. Unknown stages return no model. Physical caps are
/// absent; material is the original filled reference cross-section.
pub fn construct_smooth_closed_with_ideal_atlas(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    material: &LoftCap,
    tolerance_uv: f64,
    error_budget: f64,
    sharp: &[usize],
    limits: SmoothClosedLimits,
) -> Result<SmoothClosedAttempt> {
    construct_impl(
        sweep,
        loop_sizes,
        sections,
        material,
        tolerance_uv,
        error_budget,
        sharp,
        limits,
        None,
    )
}
/// Recompute source, coefficients, complete smoothness, global volume and
/// orientation for a supplied model. No certificate/report from it is trusted.
pub fn verify_smooth_closed_with_ideal_atlas(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    material: &LoftCap,
    tolerance_uv: f64,
    error_budget: f64,
    sharp: &[usize],
    limits: SmoothClosedLimits,
    model: &Model,
) -> Result<SmoothClosedAttempt> {
    model.validate()?;
    construct_impl(
        sweep,
        loop_sizes,
        sections,
        material,
        tolerance_uv,
        error_budget,
        sharp,
        limits,
        Some(model),
    )
}
fn construct_impl(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    material: &LoftCap,
    tolerance_uv: f64,
    error_budget: f64,
    sharp: &[usize],
    limits: SmoothClosedLimits,
    supplied: Option<&Model>,
) -> Result<SmoothClosedAttempt> {
    if limits.binding_work > 1000000
        || limits.orientation_cells > 100000
        || limits.atlas.cells > 1000000
        || limits.atlas.exact_work > 1000000
        || limits
            .atlas
            .projection
            .iter()
            .flatten()
            .any(|x| !x.is_finite() || x.abs() > 1e9)
        || !matches!(limits.smoothness.order, 1 | 2)
        || limits.smoothness.exact_work
            > nurbs_core::continuity::MAX_PROJECTIVE_SEAM_COLLECTION_WORK
        || limits.smoothness.agreement_work > 1000000
        || limits.smoothness.regularity_cells > 100000
        || limits.original_smoothness.is_some_and(|l| {
            l.order != limits.smoothness.order
                || l.cells > 100000
                || l.exact_work > 1000000
                || l.products > 1000000
        })
    {
        return Err(fail("Invalid smooth closed material limits"));
    }
    let prepared = match prepare_closed_source(
        sweep,
        loop_sizes,
        sections,
        material,
        tolerance_uv,
        error_budget,
        limits.source,
    )? {
        PreparedAttempt::Ready(p) => *p,
        PreparedAttempt::Refused(r) => {
            let reason = r.reason.unwrap_or("original-closed-source-unproved");
            return Ok(refused(report(*r), reason));
        }
    };
    let PreparedSource {
        report: source,
        profiles,
        material,
        sections: rows,
    } = prepared;
    let mut out = report(source);
    if !out.original.certified {
        return Ok(refused(out, "original-closed-source-unproved"));
    }
    if let Some(limits) = limits.original_smoothness {
        let proof = sweep.certify_original_closed_boundary_smoothness(loop_sizes, limits)?;
        let certified = proof.differential_compatibility_certified;
        out.original.smoothness = Some(proof);
        if !certified {
            return Ok(refused(out, "original-boundary-smoothness-unproved"));
        }
    }
    let candidate = affine_station_walls(&profiles, &rows, sharp, true, limits.candidate)?;
    out.displacement_upper = candidate.displacement_upper;
    out.candidate_work = candidate.work;
    out.matrix_cells = candidate.matrix_cells;
    out.candidate_reason = Some(candidate.reason);
    let (Some(sides), Some(stations), Some(poses), Some(origin), Some(delta)) = (
        candidate.sides,
        candidate.sections,
        candidate.poses,
        candidate.source_origin,
        candidate.displacement_upper,
    ) else {
        return Ok(refused(out, "exact-affine-candidate-unproved"));
    };
    let Some(error) = out
        .original
        .boundary_error_upper
        .and_then(|e| error_upper::add(e, delta))
    else {
        return Ok(refused(out, "affine-original-boundary-bound-unproved"));
    };
    out.boundary_error_upper = Some(error);
    out.within_budget = error <= error_budget;
    if !out.within_budget {
        return Ok(refused(out, "smooth-original-boundary-error-budget"));
    }
    let model = if let Some(model) = supplied {
        model.clone()
    } else {
        match crate::analytic::periodic_material_loft_surfaces(&stations, &sides) {
            Ok(m) => m,
            Err(e) if e.code == "BREP_RATIONAL_SWEEP_REFUSED" => {
                return Ok(refused(out, "affine-wall-topology-unproved"));
            }
            Err(e) => return Err(e),
        }
    };
    let owned = binding(&model, &profiles, &poses, origin, limits.binding_work)?;
    let proven = owned.certified;
    out.binding = Some(owned);
    if !proven {
        return Ok(refused(out, "affine-retained-binding-unproved"));
    }
    out.continuous_bound = true;
    let smooth = crate::sweep_smoothness::complete::inspect(&model, limits.smoothness)?;
    let proven = if limits.smoothness.order == 2 {
        smooth.g2_certified
    } else {
        smooth.g1_certified
    };
    out.smoothness = Some(smooth);
    if !proven {
        return Ok(refused(out, "complete-boundary-smoothness-unproved"));
    }
    let atlas = affine_volume::certify(
        &material.surface,
        origin,
        &poses,
        true,
        limits.atlas.projection,
        limits.atlas.cells,
        limits.atlas.exact_work,
    )?;
    let proven = atlas.embedding_certified
        && atlas.regularity_certified
        && atlas.parameter_topology_preserved;
    out.volume = Some(atlas);
    if !proven {
        return Ok(refused(out, "actual-affine-filled-volume-unproved"));
    }
    // A global embedding of the filled chart transports its VALID original
    // UV material/hole roles. Its Jacobian sign and authored UV winding then
    // determine each actual face-use orientation, not a sample normal guess.
    let Some(trim) = out.original.material.trim.as_ref() else {
        return Ok(refused(out, "original-material-roles-unproved"));
    };
    let Some(winding) = trim
        .winding
        .first()
        .copied()
        .flatten()
        .filter(|w| w.abs() == 1)
    else {
        return Ok(refused(out, "original-material-roles-unproved"));
    };
    if trim.valid != Some(true)
        || trim.winding.len() != profiles.len()
        || trim.winding.iter().skip(1).any(|w| *w != Some(-winding))
    {
        return Ok(refused(out, "original-material-roles-unproved"));
    }
    let orientation = affine_volume::jacobian_orientation(
        &material.surface,
        origin,
        &poses,
        [0.5; 3],
        limits.orientation_cells,
    )?;
    out.orientation_cells = orientation.cells;
    let sign = orientation.sign;
    out.orientation = Some(orientation);
    let Some(sign) = sign else {
        return Ok(refused(out, "actual-affine-orientation-unproved"));
    };
    let reversed = i32::from(sign) * winding < 0;
    for u in model.shells.iter().flat_map(|s| &s.faces) {
        if out.orientation_cells == limits.orientation_cells {
            return Ok(refused(out, "actual-affine-orientation-work-limit"));
        }
        out.orientation_cells += 1;
        let Some(gauge_reversed) = out
            .binding
            .as_ref()
            .and_then(|b| b.orientation_reversed(u.face))
        else {
            return Ok(refused(out, "actual-affine-chart-orientation-unproved"));
        };
        if u.reversed != (reversed ^ gauge_reversed) {
            return Ok(refused(out, "actual-affine-material-orientation-unproved"));
        }
    }
    out.shell_roles_certified = true;
    out.topology_preserved = true;
    out.certified = true;
    Ok(SmoothClosedAttempt::Certified(Box::new(SmoothClosedBody {
        model,
        report: out,
    })))
}

#[cfg(test)]
mod tests;
