//! Exact original rational affine motion -> owned closed Solid. This is not
//! normalized sampled-frame interpolation. Annular caps are complete graphs in
//! the ORIGINAL carrier UV chart, transported by the same globally proved map.
use super::*;
use nurbs_core::{
    affine_volume::{self, RationalPiece},
    surface::Surface,
};
#[derive(Clone, Copy)]
pub struct RationalClosedLimits {
    pub source_cells: usize,
    pub source_exact_work: u64,
    pub construction_rows: usize,
    pub material: MaterialLimits,
    pub max_faces: usize,
    pub binding_work: u64,
    pub smoothness: crate::sweep_smoothness::complete::Limits,
    pub volume: IdealLimits,
    pub orientation_cells: usize,
    pub cap_cells: usize,
}
pub struct FilledGraphCapReport {
    pub ring: usize,
    pub span: usize,
    pub faces: Vec<usize>,
    pub source_u_domain: [f64; 2],
    pub source_v_bounds: [f64; 2],
    pub parameter_monotonicity: nurbs_core::curve_regularity::Report,
}
pub struct RationalClosedReport {
    pub certified: bool,
    pub source_boundary_exact: bool,
    pub boundary_error_upper: Option<f64>,
    pub seam_continuity: &'static str,
    pub source_g1: bool,
    pub source_g2: bool,
    pub topology_preserved: bool,
    pub source_cells: usize,
    pub source_exact_work: u64,
    pub construction_rows: usize,
    pub cap_cells: usize,
    pub orientation_cells: usize,
    pub material: Option<nurbs_core::trim_region_audit::Report>,
    pub volume: Option<nurbs_core::progressive_sweep::IdealVolumeReport>,
    pub proof_coordinates: Option<affine_volume::ProofCoordinates>,
    pub binding: Option<smooth::BindingReport>,
    pub smoothness: Option<crate::sweep_smoothness::complete::Report>,
    pub c0_regularity: Option<crate::boundary_embedding::regularity::Report>,
    pub c0_agreement: Option<crate::boundary_agreement::ExactReport>,
    pub caps: Vec<FilledGraphCapReport>,
    pub reason: Option<&'static str>,
}
pub struct RationalClosedBody {
    model: Model,
    report: RationalClosedReport,
}
impl RationalClosedBody {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn report(&self) -> &RationalClosedReport {
        &self.report
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
pub enum RationalClosedAttempt {
    Certified(Box<RationalClosedBody>),
    Refused(Box<RationalClosedReport>),
}
fn refuse(mut r: RationalClosedReport, reason: &'static str) -> RationalClosedAttempt {
    r.reason = Some(reason);
    RationalClosedAttempt::Refused(Box::new(r))
}
fn bezier(c: &Curve) -> bool {
    !c.periodic
        && c.degree > 0
        && c.degree <= 32
        && c.control_points.len() == c.degree + 1
        && c.knots == [vec![0.; c.degree + 1], vec![1.; c.degree + 1]].concat()
}
/// Every returned face is an exact tensor coefficient image of the ORIGINAL
/// source. Consequently the global source volume and complete actual-boundary
/// Gk also certify the ideal boundary: no approximation or sampled error term.
/// `cap_profiles` identifies complete annular graph caps (ring, span), never
/// seams to omit. Graphs use original UV coordinates, not a guessed world
/// plane or radius. The original global embedding transports the complete
/// graph-interval x S1 annulus; exact binding locates every actual cap chart.
/// Every wall/cap edge is still inspected. Unsupported cap laws
/// or exhausted stages return no Solid. Pole caps are not admitted here.
pub fn construct_rational_closed_material(
    profiles: &[Vec<Curve>],
    material: &LoftCap,
    origin: [f64; 3],
    motion: &[RationalPiece],
    cap_profiles: &[[usize; 2]],
    tolerance_uv: f64,
    l: RationalClosedLimits,
) -> Result<RationalClosedAttempt> {
    rational_closed_impl(
        profiles,
        material,
        origin,
        motion,
        cap_profiles,
        tolerance_uv,
        l,
        None,
    )
}

/// Reprove every original source, exact actual face, orientation and complete
/// boundary obligation for supplied geometry. The input model is never repaired
/// or reoriented. No constructor, source report or caller proof supplies credit.
pub fn verify_rational_closed_material(
    profiles: &[Vec<Curve>],
    material: &LoftCap,
    origin: [f64; 3],
    motion: &[RationalPiece],
    cap_profiles: &[[usize; 2]],
    tolerance_uv: f64,
    l: RationalClosedLimits,
    model: &Model,
) -> Result<RationalClosedAttempt> {
    model.validate()?;
    rational_closed_impl(
        profiles,
        material,
        origin,
        motion,
        cap_profiles,
        tolerance_uv,
        l,
        Some(model),
    )
}

fn rational_closed_impl(
    profiles: &[Vec<Curve>],
    material: &LoftCap,
    origin: [f64; 3],
    motion: &[RationalPiece],
    cap_profiles: &[[usize; 2]],
    tolerance_uv: f64,
    l: RationalClosedLimits,
    supplied: Option<&Model>,
) -> Result<RationalClosedAttempt> {
    material.surface.validate()?;
    for c in profiles
        .iter()
        .flatten()
        .chain(material.trims.iter().flatten())
    {
        c.validate()?;
    }
    // Full raw motion validation, even if a later owner budget is zero.
    affine_volume::certify_rational(
        &material.surface,
        origin,
        motion,
        true,
        l.volume.projection,
        0,
        0,
    )?;
    if profiles.is_empty()
        || profiles.len() > 16
        || profiles.iter().any(|p| p.len() < 2 || p.len() > 64)
        || profiles.iter().map(Vec::len).sum::<usize>() > 256
        || profiles.len() != material.trims.len()
        || profiles
            .iter()
            .zip(&material.trims)
            .any(|(a, b)| a.len() != b.len())
        || profiles
            .iter()
            .flatten()
            .any(|c| c.control_points.iter().any(|p| p.len() != 3))
        || material
            .trims
            .iter()
            .flatten()
            .any(|c| c.control_points.iter().any(|p| p.len() != 2))
        || cap_profiles.len() > 64
        || cap_profiles
            .iter()
            .any(|p| p[0] >= profiles.len() || p[1] >= profiles[p[0]].len())
        || cap_profiles
            .iter()
            .enumerate()
            .any(|(i, p)| cap_profiles[..i].contains(p))
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || l.source_cells > 100000
        || l.source_exact_work > 1000000
        || l.construction_rows > 1000000
        || l.binding_work > 1000000
        || l.cap_cells > 100000
        || l.orientation_cells > 100000
        || l.max_faces > 1024
        || l.material.pairs > 100000
        || l.material.cells > 100000
        || l.material.domain_cells > 1000000
        || l.volume.cells > 1000000
        || l.volume.exact_work > 1000000
        || !matches!(l.smoothness.order, 0 | 1 | 2)
        || l.smoothness.exact_work > nurbs_core::continuity::MAX_PROJECTIVE_SEAM_COLLECTION_WORK
        || l.smoothness.agreement_work > 1000000
        || l.smoothness.regularity_cells > 100000
    {
        return Err(fail("Invalid rational closed material source or limits"));
    }
    let mut r = RationalClosedReport {
        certified: false,
        source_boundary_exact: false,
        boundary_error_upper: None,
        seam_continuity: "C0",
        source_g1: false,
        source_g2: false,
        topology_preserved: false,
        source_cells: 0,
        source_exact_work: 0,
        construction_rows: 0,
        cap_cells: 0,
        orientation_cells: 0,
        material: None,
        volume: None,
        proof_coordinates: None,
        binding: None,
        smoothness: None,
        c0_regularity: None,
        c0_agreement: None,
        caps: Vec::new(),
        reason: None,
    };
    let spans = profiles.iter().map(Vec::len).sum::<usize>();
    if motion.len() < 3
        || spans * motion.len() > l.max_faces
        || profiles.iter().flatten().any(|c| !bezier(c))
    {
        return Ok(refuse(r, "rational-original-basis-or-face-limit"));
    }
    if l.material.pairs == 0 || l.material.cells == 0 || l.material.domain_cells == 0 {
        return Ok(refuse(r, "rational-material-work-limit"));
    }
    let trim = nurbs_core::trim_region_audit::inspect(
        &material.trims,
        tolerance_uv,
        l.material.pairs,
        l.material.cells,
        l.material.domain_cells,
    )?;
    let proven = trim.valid == Some(true);
    r.material = Some(trim);
    if !proven {
        return Ok(refuse(r, "rational-original-material-unproved"));
    }
    for (wire, uv) in profiles.iter().zip(&material.trims) {
        for (world, uv) in wire.iter().zip(uv) {
            let regular =
                nurbs_core::curve_regularity::inspect(world, l.source_cells - r.source_cells)?;
            r.source_cells += regular.cells;
            if !regular.spanwise_regular {
                return Ok(refuse(r, "rational-source-profile-regularity-unproved"));
            }
            let Some(proof) = nurbs_core::curve_surface_agreement::verify_exact(
                world,
                uv,
                &material.surface,
                false,
                l.source_exact_work - r.source_exact_work,
            )?
            else {
                return Ok(refuse(r, "rational-source-carrier-binding-unproved"));
            };
            r.source_exact_work += proof.work_used;
            if proof.outcome != cad_predicates::BezierIdentity::Equal {
                return Ok(refuse(r, "rational-source-carrier-binding-unproved"));
            }
        }
    }
    for &[ring, span] in cap_profiles {
        let c = &material.trims[ring][span];
        let d = c.domain();
        if c.periodic
            || c.control_points.len() != c.degree + 1
            || c.knots[..=c.degree].iter().any(|u| *u != d[0])
            || c.knots[c.control_points.len()..].iter().any(|u| *u != d[1])
        {
            return Ok(refuse(r, "rational-cap-single-graph-chart-unproved"));
        }
        // Exact source-UV coordinate projection. Nonzero derivative on the
        // entire analytic span proves a strictly monotone graph parameter.
        let mut radial = c.clone();
        radial.control_points = c
            .control_points
            .iter()
            .map(|p| vec![p[0], 0., 0.])
            .collect();
        let proof = nurbs_core::curve_regularity::inspect(&radial, l.cap_cells - r.cap_cells)?;
        r.cap_cells += proof.cells;
        if !proof.spanwise_regular {
            return Ok(refuse(r, "rational-annular-cap-radial-graph-unproved"));
        }
        let a = c.control_points.first().unwrap()[0];
        let b = c.control_points.last().unwrap()[0];
        if a == b {
            return Ok(refuse(r, "rational-annular-cap-radial-domain-unproved"));
        }
        r.caps.push(FilledGraphCapReport {
            ring,
            span,
            faces: Vec::new(),
            source_u_domain: [a.min(b), a.max(b)],
            source_v_bounds: [
                c.control_points
                    .iter()
                    .map(|p| p[1])
                    .fold(f64::INFINITY, f64::min),
                c.control_points
                    .iter()
                    .map(|p| p[1])
                    .fold(f64::NEG_INFINITY, f64::max),
            ],
            parameter_monotonicity: proof,
        });
    }
    let proof = affine_volume::certify_rational_preconditioned(
        &material.surface,
        origin,
        motion,
        true,
        l.volume.projection,
        l.volume.cells,
        l.volume.exact_work,
    )?;
    r.proof_coordinates = proof.coordinates;
    let volume = proof.volume;
    let proven = volume.embedding_certified
        && volume.regularity_certified
        && volume.parameter_topology_preserved;
    r.volume = Some(volume);
    if !proven {
        return Ok(refuse(r, "original-rational-filled-volume-unproved"));
    }
    let mut model = if let Some(model) = supplied {
        model.clone()
    } else {
        let rows = profiles
            .iter()
            .flatten()
            .map(|c| c.control_points.len())
            .sum::<usize>();
        let required =
            rows * (motion.iter().map(|p| p.controls.len()).sum::<usize>() + motion.len() + 1);
        if required > l.construction_rows {
            return Ok(refuse(r, "rational-tensor-construction-work-limit"));
        }
        let map_curve = |c: &Curve, p: nurbs_core::affine_volume::Pose| -> Option<Curve> {
            let mut c = c.clone();
            for q in &mut c.control_points {
                *q = smooth::image(p, q, origin)?.to_vec();
            }
            Some(c)
        };
        let mut sections = Vec::new();
        let mut sides = vec![Vec::new(); profiles.len()];
        for piece in motion {
            let mut section = Vec::new();
            for (ring_index, wire) in profiles.iter().enumerate() {
                let mut ring = Vec::new();
                for c in wire {
                    let Some(curve) = map_curve(c, piece.controls[0]) else {
                        return Ok(refuse(r, "rational-section-exact-arithmetic-unproved"));
                    };
                    ring.push(curve);
                    let mut cp = Vec::new();
                    let mut weights = Vec::new();
                    for (i, p) in c.control_points.iter().enumerate() {
                        let mut row = Vec::new();
                        let mut wr = Vec::new();
                        for (j, pose) in piece.controls.iter().enumerate() {
                            let (Some(q), Some(w)) = (
                                smooth::image(*pose, p, origin),
                                smooth::exact_mul(c.weights[i], piece.weights[j]),
                            ) else {
                                return Ok(refuse(r, "rational-tensor-exact-arithmetic-unproved"));
                            };
                            row.push(q.to_vec());
                            wr.push(w);
                            r.construction_rows += 1;
                        }
                        cp.push(row);
                        weights.push(wr);
                    }
                    sides[ring_index].push(Surface {
                        degree_u: c.degree,
                        degree_v: piece.controls.len() - 1,
                        knots_u: c.knots.clone(),
                        knots_v: [
                            vec![0.; piece.controls.len()],
                            vec![1.; piece.controls.len()],
                        ]
                        .concat(),
                        control_points: cp,
                        weights,
                        periodic_u: false,
                        periodic_v: false,
                    });
                }
                section.push(ring);
            }
            sections.push(section);
            r.construction_rows += rows;
        }
        // Closure was independently proved on ORIGINAL full poses. The final row
        // is now the literal common station, not a near-closure approximation.
        sections.push(sections[0].clone());
        r.construction_rows += rows;
        let model = match crate::analytic::periodic_material_loft_surfaces(&sections, &sides) {
            Ok(m) => m,
            Err(e) if e.code == "BREP_RATIONAL_SWEEP_REFUSED" => {
                return Ok(refuse(r, "rational-boundary-topology-unproved"));
            }
            Err(e) => return Err(e),
        };
        model
    };
    let binding = smooth::binding_rational(&model, profiles, motion, origin, l.binding_work)?;
    let proven = binding.certified;
    r.binding = Some(binding);
    if !proven {
        return Ok(refuse(
            r,
            "rational-actual-coefficient-domain-binding-unproved",
        ));
    }
    r.source_boundary_exact = true;
    r.boundary_error_upper = Some(0.);
    let sign = affine_volume::jacobian_orientation_rational(
        &material.surface,
        origin,
        motion,
        [0.5; 3],
        l.orientation_cells,
    )?;
    r.orientation_cells = sign.cells;
    let Some(sign) = sign.sign else {
        return Ok(refuse(r, "rational-source-world-orientation-unproved"));
    };
    let trim = r.material.as_ref().unwrap();
    let winding = trim.winding[0].unwrap();
    if winding.abs() != 1 || trim.winding.iter().skip(1).any(|w| *w != Some(-winding)) {
        return Ok(refuse(r, "rational-material-shell-roles-unproved"));
    }
    let reversed = i32::from(sign) * winding < 0;
    for shell in &mut model.shells {
        for u in &mut shell.faces {
            if r.orientation_cells == l.orientation_cells {
                return Ok(refuse(r, "rational-face-orientation-work-limit"));
            }
            r.orientation_cells += 1;
            let Some(parity) = r.binding.as_ref().unwrap().orientation_reversed(u.face) else {
                return Ok(refuse(r, "rational-face-chart-orientation-unproved"));
            };
            let expected = reversed ^ parity;
            if supplied.is_some() && u.reversed != expected {
                return Ok(refuse(r, "rational-supplied-face-orientation-unproved"));
            }
            if supplied.is_none() {
                u.reversed = expected;
            }
        }
    }
    model.validate()?;
    if l.smoothness.order == 0 {
        if !crate::sweep_smoothness::complete::vertex_links(&model)
            || l.smoothness.agreement_work == 0
        {
            return Ok(refuse(r, "rational-complete-c0-topology-unproved"));
        }
        let agreement =
            crate::boundary_agreement::verify_exact(&model, l.smoothness.agreement_work)?;
        let proven = agreement.all_equal && agreement.all_joins_exact;
        r.c0_agreement = Some(agreement);
        if !proven {
            return Ok(refuse(r, "rational-complete-c0-agreement-unproved"));
        }
        let regular =
            crate::boundary_embedding::regularity::inspect(&model, l.smoothness.regularity_cells)?;
        let proven = regular.all_faces_regular;
        r.c0_regularity = Some(regular);
        if !proven {
            return Ok(refuse(r, "rational-complete-c0-regularity-unproved"));
        }
    } else {
        let smoothness = crate::sweep_smoothness::complete::inspect(&model, l.smoothness)?;
        let proven = if l.smoothness.order == 2 {
            smoothness.g2_certified
        } else {
            smoothness.g1_certified
        };
        r.smoothness = Some(smoothness);
        if !proven {
            return Ok(refuse(
                r,
                "original-and-actual-rational-boundary-gk-unproved",
            ));
        }
    }
    for cap in &mut r.caps {
        let offset = profiles[..cap.ring].iter().map(Vec::len).sum::<usize>() + cap.span;
        for layer in 0..motion.len() {
            let Some(face) = r
                .binding
                .as_ref()
                .unwrap()
                .source_face(layer * spans + offset)
            else {
                return Ok(refuse(r, "rational-cap-chart-cover-unproved"));
            };
            cap.faces.push(face);
        }
    }
    r.topology_preserved = true;
    r.source_g1 = l.smoothness.order >= 1;
    r.source_g2 = l.smoothness.order == 2;
    r.seam_continuity = if r.source_g2 {
        "G2"
    } else if r.source_g1 {
        "G1"
    } else {
        "C0"
    };
    r.certified = true;
    Ok(RationalClosedAttempt::Certified(Box::new(
        RationalClosedBody { model, report: r },
    )))
}
#[cfg(test)]
mod tests;
