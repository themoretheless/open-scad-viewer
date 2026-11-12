//! Automatic cap proposals followed by fresh normal and actual topology proofs.
use super::{Limits, Thickness, refused, thicken_with_maps};
use crate::{Model, Result, feature_ops::region_pattern::Verification};
use nurbs_core::{
    surface::Surface,
    surface_offset::{self, ProposalLimits, ProposalReport},
};

pub struct AutomaticThickness {
    skin: Thickness,
    proposals: Vec<ProposalReport>,
    proposal_limits: ProposalLimits,
    evaluations: usize,
    coefficient_work: usize,
    proposal_cells: usize,
}
impl AutomaticThickness {
    pub fn model(&self) -> &Model {
        self.skin.model()
    }
    pub fn constructed(&self) -> &Thickness {
        &self.skin
    }
    pub fn proposals(&self) -> &[ProposalReport] {
        &self.proposals
    }
    pub fn evaluations(&self) -> usize {
        self.evaluations
    }
    pub fn coefficient_work(&self) -> usize {
        self.coefficient_work
    }
    /// All residual cells, including rejected proposals and fresh final checks.
    pub fn normal_cells(&self) -> usize {
        self.proposal_cells + self.skin.normal_cells()
    }
    /// Repeat proposal generation, continuous bounds and actual closed sewing.
    pub fn reverify(&self) -> Result<Verification> {
        let fresh = thicken_from_maps(
            MappedSources {
                sources: &self.skin.sources,
                maps: &self.skin.maps,
                reversed: &self.skin.reversed,
            },
            self.skin.distance,
            self.skin.normal_tolerance,
            self.proposal_limits,
            self.skin.sewing_tolerance,
            self.skin.limits,
        )?;
        if fresh.model() != self.model() || fresh.skin.caps != self.skin.caps {
            return Err(refused(
                "Saved automatic cap skin differs from fresh proposal/reconstruction",
            ));
        }
        Ok(fresh.skin.built.evidence)
    }
}

/// Original source patches with their authored UV maps and orientations, one
/// entry of each per patch.
#[derive(Clone, Copy)]
pub struct MappedSources<'a> {
    pub sources: &'a [Surface],
    pub maps: &'a [Surface],
    pub reversed: &'a [bool],
}
/// Build caps automatically over authored original UV maps, then require fresh
/// continuous normal-offset and closed-model proofs. No cap or positive receipt
/// is supplied by the caller. The original full source patches stay literal.
/// Maps remain authored: region/root discovery and extension are separate.
///
/// Evaluation and coefficient allowances are shared by all patches. The single
/// `proposal_limits.proof_cells` residual allowance covers EVERY proposal attempt
/// and final cap check; half is reserved for final qualification. Sewing/chart
/// work has the independent existing `Limits` allowance. Failure publishes no
/// partial model. Independent cap proposals must actually sew; this does not
/// heal mismatches or authorize them by the normal-displacement tolerance.
#[allow(clippy::too_many_arguments)]
pub fn thicken_from_maps(
    mapped: MappedSources<'_>,
    signed_thickness: f64,
    normal_tolerance: f64,
    proposal_limits: ProposalLimits,
    sewing_tolerance: f64,
    limits: Limits,
) -> Result<AutomaticThickness> {
    let MappedSources {
        sources,
        maps,
        reversed,
    } = mapped;
    super::super::freeform_network::validate(
        sources,
        sources,
        reversed,
        crate::shell_thickness::planar_network::OffsetSettings {
            signed_thickness,
            normal_tolerance,
            normal_cells: proposal_limits.proof_cells,
            sewing_tolerance,
        },
        limits,
    )?;
    if maps.len() != sources.len() {
        return Err(refused("Each original source needs its authored UV map"));
    }
    let proposal_quota = proposal_limits.proof_cells / 2;
    if proposal_quota < sources.len() {
        return Err(refused(
            "Reserve proposal and final normal work for every source patch",
        ));
    }
    let mut proposals = Vec::with_capacity(sources.len());
    let mut caps = Vec::with_capacity(sources.len());
    let (mut evaluations, mut coefficient_work, mut proposal_cells) = (0usize, 0usize, 0usize);
    for (i, ((source, map), reverse)) in sources.iter().zip(maps).zip(reversed).enumerate() {
        let residual = proposal_quota
            .checked_sub(proposal_cells)
            .and_then(|n| n.checked_sub(sources.len() - i - 1))
            .filter(|n| *n > 0)
            .ok_or_else(|| refused("Shared cap proposal proof allowance exhausted"))?;
        let remaining = ProposalLimits {
            max_degree: proposal_limits.max_degree,
            evaluations: proposal_limits
                .evaluations
                .checked_sub(evaluations)
                .filter(|n| *n > 0)
                .ok_or_else(|| refused("Shared cap evaluation allowance exhausted"))?,
            coefficient_work: proposal_limits
                .coefficient_work
                .checked_sub(coefficient_work)
                .filter(|n| *n > 0)
                .ok_or_else(|| refused("Shared cap coefficient allowance exhausted"))?,
            proof_cells: residual,
        };
        let report = surface_offset::propose_parametric_candidate(
            source,
            map,
            if *reverse {
                -signed_thickness
            } else {
                signed_thickness
            },
            normal_tolerance,
            remaining,
        )?;
        evaluations += report.evaluations;
        coefficient_work += report.coefficient_work;
        proposal_cells += report.proof_cells;
        let cap = report.cap.as_ref().ok_or_else(|| {
            refused(&format!(
                "Automatic cap remains unqualified: {:?}",
                report.stop
            ))
        })?;
        caps.push(cap.clone());
        proposals.push(report);
    }
    // Interpolants may round the same original network vertex differently.
    // Couple only proposals with common literal source-corner coordinates and an
    // interval-bounded correction. This is no sewing receipt: every changed
    // cap below needs a fresh whole normal proof and actual closed-model gate.
    super::super::mapped_network::couple_corners(sources, &mut caps, sewing_tolerance)?;
    let remaining = proposal_limits
        .proof_cells
        .checked_sub(proposal_cells)
        .filter(|n| *n > 0)
        .ok_or_else(|| refused("Final original-normal qualification allowance exhausted"))?;
    let skin = thicken_with_maps(
        sources,
        &caps,
        maps,
        reversed,
        super::OffsetSettings {
            signed_thickness,
            normal_tolerance,
            normal_cells: remaining,
            sewing_tolerance,
        },
        limits,
    )?;
    Ok(AutomaticThickness {
        skin,
        proposals,
        proposal_limits,
        evaluations,
        coefficient_work,
        proposal_cells,
    })
}
