//! Full Round construction on original rational profile supports of XY prisms.
//! Contact solving proposes; original stored surface proofs admit the result.
use super::{FullRoundReport, error, full_round_caps, full_round_seams};
use crate::{Model, Result, solid_audit::audit_solid};
use nurbs_core::{
    curve::Curve,
    offset::profile_corner::full_round::{self, Proposal},
};
use std::collections::BTreeSet;
pub mod source_cover;
pub use source_cover::SourceArcCover;
pub mod material_relation;
pub mod raw_profiles;
pub mod source_supports;
pub mod source_caps;
pub mod source_boundary;
pub use material_relation::{NoAddedProfile, SourceProfileRelation};

pub struct CurvedFullRound {
    source: Model,
    raw_sources: Vec<Vec<raw_profiles::RawProfileCurve>>,
    proposal: Proposal,
    construction: FullRoundReport,
    retained_supports: [crate::profile_certificate::original_restriction::Restriction; 2],
    exact_profile: Vec<crate::profile_certificate::original_restriction::wire::Wire>,
    exact_smoothness: crate::profile_certificate::original_restriction::wire::SmoothEdits,
    exact_shell: crate::profile_certificate::original_restriction::wire::prism::ShellGeometry,
    exact_radius: crate::profile_certificate::original_restriction::wire::radius::Radius,
}
impl CurvedFullRound {
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn raw_profile_sources(&self) -> &[Vec<raw_profiles::RawProfileCurve>] {
        &self.raw_sources
    }
    pub fn proposal(&self) -> &Proposal {
        &self.proposal
    }
    pub fn construction(&self) -> &FullRoundReport {
        &self.construction
    }
    pub fn model(&self) -> &Model {
        &self.construction.model
    }
    /// Exact source restrictions. The stored construction still has independent
    /// rounded edge geometry; these owners never authorize its material relation.
    pub fn retained_supports(&self) -> &[crate::profile_certificate::original_restriction::Restriction; 2] {
        &self.retained_supports
    }
    /// Source-preserving rational G0 wires. The stored Model, G1, material
    /// inclusion and solid admission require separate proofs on this geometry.
    pub fn exact_radius(&self) -> &crate::profile_certificate::original_restriction::wire::radius::Radius { &self.exact_radius }
    pub fn exact_profile(&self) -> &[crate::profile_certificate::original_restriction::wire::Wire] {
        &self.exact_profile
    }
    pub fn exact_smoothness(&self) -> &crate::profile_certificate::original_restriction::wire::SmoothEdits {
        &self.exact_smoothness
    }
    /// Exact rational closed shell and continuous edge/pcurve agreement.
    /// Embedding, material roles and solid admission remain separate.
    pub fn exact_shell(&self) -> &crate::profile_certificate::original_restriction::wire::prism::ShellGeometry {
        &self.exact_shell
    }
    /// Admit an embedded extrusion solid from the NEW rational geometry.
    /// This does not prove source/result material inclusion or qualify the old Model.
    pub fn qualify_exact_solid(
        &self,
        jordan_limits: crate::profile_certificate::original_restriction::wire::jordan::Limits,
        nesting_limits: crate::profile_certificate::original_restriction::wire::nesting::Limits,
    ) -> Result<crate::profile_certificate::original_restriction::wire::prism::Solid> {
        self.exact_shell.qualify_solid(jordan_limits, nesting_limits)
    }
}
/// Infer the three consecutive profile owners from original face identities.
/// The middle is straight; both flanking supports may be rational curves.
/// Every active span of the contact arcs is separately authored and qualified,
/// including all additional longitudinal joins and original side surfaces.
pub fn build(source: &Model, faces: [usize; 3]) -> Result<CurvedFullRound> {
    source.validate()?;
    if source.bodies.len() != 1
        || source.shells.len() != 1
        || faces.into_iter().collect::<BTreeSet<_>>().len() != 3
    {
        return Err(error(
            "Curved Full Round needs one source body and three distinct side faces",
        ));
    }
    let mut p = crate::prism::recognize(source)?
        .ok_or_else(|| error("Curved Full Round needs a proven XY prism"))?;
    let raw_sources = raw_profiles::bind(source, p.z_max, &p.loops)?;
    let wire = &p.loops[0];
    let mut candidates = Vec::new();
    for (i, c) in raw_sources[0].iter().enumerate() {
        if c.belongs_to_face(source, faces[1])? {
            candidates.push(i);
        }
    }
    if candidates.len() != 1 || wire.len() < 4 {
        return Err(error(
            "Curved Full Round middle must own one complete outer profile segment",
        ));
    }
    let i = candidates[0];
    let prev = (i + wire.len() - 1) % wire.len();
    let next = (i + 1) % wire.len();
    let bound = (raw_sources[0][prev].belongs_to_face(source, faces[0])?
        && raw_sources[0][next].belongs_to_face(source, faces[2])?)
        || (raw_sources[0][prev].belongs_to_face(source, faces[2])?
            && raw_sources[0][next].belongs_to_face(source, faces[0])?);
    if !bound {
        return Err(error(
            "Curved Full Round selected supports do not own the adjoining original curves",
        ));
    }
    let proposal = full_round::propose(
        &wire[prev],
        &wire[i],
        &wire[next],
        source.tolerance_mm / 32.,
    )?;
    let parameters = proposal.parameters();
    let retained_supports = [
        raw_sources[0][prev].restrict_hint([wire[prev].domain()[0], parameters[0]])?,
        raw_sources[0][next].restrict_hint([parameters[1], wire[next].domain()[1]])?,
    ];
    use crate::profile_certificate::original_restriction as exact;
    let full = |c: &Curve| exact::restrict_binary64(c, c.domain());
    let mut exact_sources = Vec::new();
    let mut editable = Vec::new();
    let mut crown_after_piece = None;
    for j in 0..wire.len() {
        if j == prev || j == next {
            exact_sources.push(retained_supports[usize::from(j == next)].clone());
            editable.push(false);
        } else if j == i {
            for (bridge_index,bridge) in proposal.bridges().iter().enumerate() {
                exact_sources.push(full(bridge)?);
                editable.push(true);
                if bridge_index == 0 { crown_after_piece = Some(exact_sources.len()-1); }
            }
        } else {
            exact_sources.push(raw_sources[0][j].full()?);
            editable.push(false);
        }
    }
    // Preserve the exact binary64 tolerance as a rational.
    let tolerance = {
        use num_traits::FromPrimitive;
        let q = num_rational::BigRational::from_f64(source.tolerance_mm / 32.)
            .ok_or_else(|| error("Invalid exact Full Round wire tolerance"))?;
        [q.numer().to_string(), q.denom().to_string()]
    };
    let mut exact_profile = vec![exact::wire::sew_tangent_with_contacts(exact_sources, editable, tolerance.clone(),
        vec![exact::wire::LineContact::new(crown_after_piece.ok_or_else(||error("Missing original crown junction"))?,raw_sources[0][i].full()?)])?];
    let exact_smoothness = exact_profile[0].qualify_smooth_edits([
        "0".into(), "1".into(),
    ])?;
    let to_ratio = |v:f64| -> Result<crate::profile_certificate::ExactRatio> {
        let q=num_rational::BigRational::from_float(v).ok_or_else(||error("Nonfinite Full Round radius target"))?;
        Ok([q.numer().to_string(),q.denom().to_string()])
    };
    let center=proposal.center();
    let exact_radius=exact_profile[0].qualify_edited_radius(
        [to_ratio(center[0])?,to_ratio(center[1])?],to_ratio(proposal.radius())?,tolerance.clone())?;
    for hole in &raw_sources[1..] {
        let sources = hole.iter().map(raw_profiles::RawProfileCurve::full).collect::<Result<Vec<_>>>()?;
        exact_profile.push(exact::wire::sew(sources, vec![false; hole.len()], tolerance.clone())?);
    }
    let mut replacement = Vec::new();
    for (j, c) in wire.iter().enumerate() {
        if j == prev {
            replacement.push(proposal.incoming().clone());
        } else if j == i {
            replacement.extend(proposal.bridges().iter().cloned());
        } else if j == next {
            replacement.push(proposal.outgoing().clone());
        } else {
            replacement.push(c.clone());
        }
    }
    p.loops[0] = replacement;
    let exact_shell = exact::wire::prism::extrude_binary64(
        &exact_profile, [p.z_min, p.z_max], source.tolerance_mm,
    )?;
    let mut model = crate::prism::extrude(&p.loops, p.z_min, p.z_max)?;
    full_round_caps::world_charts(&mut model)?;
    model.tolerance_mm = source.tolerance_mm;
    model.inherit_topology_ids(&[source]);
    model.validate()?;
    let audit = audit_solid(&model)?;
    if !audit.ok {
        return Err(error("Curved Full Round solid audit remains unproven"));
    }
    let center = proposal.center();
    let mut nodes = Vec::new();
    let mut crown = 0;
    for (bridge, arc) in proposal.bridges().iter().enumerate() {
        let [lo, hi] = arc.domain();
        let mut cuts = arc
            .knots
            .iter()
            .copied()
            .filter(|t| *t >= lo && *t <= hi)
            .collect::<Vec<_>>();
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        for pair in cuts.windows(2) {
            let span = arc.trim(pair[0], pair[1])?;
            if nodes.is_empty() {
                let p = &span.control_points[0];
                nodes.push([p[0], p[1]]);
            }
            let p = span.control_points.last().unwrap();
            nodes.push([p[0], p[1]]);
        }
        if bridge == 0 {
            crown = nodes.len() - 1;
        }
    }
    let endpoints = nodes
        .into_iter()
        .map(|q| [p.z_min, p.z_max].map(|z| [q[0], q[1], z]))
        .collect::<Vec<_>>();
    let centers = [p.z_min, p.z_max].map(|z| [center[0], center[1], z]);
    let tangency =
        full_round_seams::qualify_chain(&model, &endpoints, crown, centers, proposal.radius())?;
    Ok(CurvedFullRound {
        source: source.clone(),
        raw_sources,
        retained_supports,
        exact_profile,
        exact_smoothness,
        exact_shell,
        exact_radius,
        construction: FullRoundReport {
            model,
            replaced_face: faces[1],
            support_faces: [faces[0], faces[2]],
            radius_mm: proposal.radius(),
            audit,
            tangency,
        },
        proposal,
    })
}
