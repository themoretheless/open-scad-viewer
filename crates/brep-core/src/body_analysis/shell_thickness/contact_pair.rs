//! Original contact-band UV/world ownership bound into actual closed cap nets.
use super::{parametric_network, planar_network};
use crate::{Error, Model, Result, feature_ops::region_pattern::Verification};
use nurbs_core::{
    curve::Curve,
    offset_contact_pcurve::FitLimits,
    surface::Surface,
    surface_offset::{self, ProposalLimits, ProposalReport, contact_curve, regions},
};
/// UV proposal, normal-displacement and boundary sewing tolerances of one
/// contact-pair skin.
#[derive(Clone, Copy, Debug)]
pub struct Tolerances {
    pub uv: f64,
    pub normal: f64,
    pub sewing: f64,
}
pub mod cap;
mod automatic;
mod binding;
pub use automatic::{Attempt, AutomaticReport, DiscoveryLimits, discover};
fn refused(s: &str) -> Error {
    Error::new("BREP_CONTACT_CAP_PAIR_REFUSED", s)
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    pub axis: usize,
    pub drive: [f64; 2],
    pub first_other: [f64; 2],
    pub second: [[f64; 2]; 2],
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub sewing: planar_network::Limits,
    pub branch_fit: FitLimits,
    pub caps: ProposalLimits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            sewing: planar_network::Limits::default(),
            branch_fit: FitLimits::default(),
            caps: ProposalLimits::default(),
        }
    }
}
pub struct Thickness {
    sources: [Surface; 2],
    reversed: [bool; 2],
    distance: f64,
    band: Band,
    uv_tolerance: f64,
    normal_tolerance: f64,
    sewing_tolerance: f64,
    limits: Limits,
    seam: [usize; 2],
    regions: regions::Pair,
    world_fit: contact_curve::FitReport,
    curve: Curve,
    world_proof: contact_curve::Report,
    proposals: Vec<ProposalReport>,
    skin: parametric_network::Thickness,
    root_queries: usize,
    branch_cells: usize,
    branch_coefficients: usize,
    cap_evaluations: usize,
    cap_coefficients: usize,
    proposal_normal_cells: usize,
    source_cells: usize,
    source_exact: u64,
}
impl Thickness {
    pub fn model(&self) -> &Model {
        self.skin.model()
    }
    pub fn constructed(&self) -> &parametric_network::Thickness {
        &self.skin
    }
    pub fn source_seam(&self) -> [usize; 2] {
        self.seam
    }
    pub fn uv_regions(&self) -> &regions::Pair {
        &self.regions
    }
    pub fn contact_curve(&self) -> &Curve {
        &self.curve
    }
    pub fn world_fit(&self) -> &contact_curve::FitReport {
        &self.world_fit
    }
    pub fn world_proof(&self) -> &contact_curve::Report {
        &self.world_proof
    }
    pub fn cap_proposals(&self) -> &[ProposalReport] {
        &self.proposals
    }
    pub fn root_queries(&self) -> usize {
        self.root_queries
    }
    pub fn branch_proof_cells(&self) -> usize {
        self.branch_cells
    }
    pub fn branch_coefficient_work(&self) -> usize {
        self.branch_coefficients
    }
    pub fn cap_evaluations(&self) -> usize {
        self.cap_evaluations
    }
    pub fn cap_coefficient_work(&self) -> usize {
        self.cap_coefficients
    }
    pub fn normal_cells(&self) -> usize {
        self.proposal_normal_cells + self.skin.normal_cells()
    }
    pub fn sewing_cells(&self) -> usize {
        self.source_cells + self.skin.evidence().cells
    }
    pub fn exact_work(&self) -> u64 {
        self.source_exact + self.skin.exact_work()
    }
    pub fn reverify(&self) -> Result<Verification> {
        let fresh = thicken(
            &self.sources,
            self.reversed,
            self.distance,
            self.band,
            Tolerances {
                uv: self.uv_tolerance,
                normal: self.normal_tolerance,
                sewing: self.sewing_tolerance,
            },
            self.limits,
        )?;
        if fresh.model() != self.model() || fresh.curve != self.curve || fresh.seam != self.seam {
            return Err(refused(
                "Saved contact skin differs from complete original reconstruction",
            ));
        }
        Ok(fresh.skin.evidence().clone())
    }
}
fn validate(l: Limits) -> Result<()> {
    let f = l.branch_fit;
    let c = l.caps;
    if !(1..=16).contains(&f.max_degree)
        || !(1..=10000000).contains(&f.root_queries)
        || !(1..=10000000).contains(&f.coefficient_work)
        || !(1..=100000).contains(&f.proof_cells)
        || !(1..=100000).contains(&f.spans)
        || f.root_refinements > 16
        || !(1..=16).contains(&c.max_degree)
        || !(1..=100000).contains(&c.evaluations)
        || !(1..=10000000).contains(&c.coefficient_work)
        || !(1..=1000000).contains(&c.proof_cells)
    {
        return Err(refused(
            "Choose bounded contact/normal proposal and proof work",
        ));
    }
    if f.root_queries < 3
        || f.coefficient_work < 3
        || f.proof_cells < 3
        || c.evaluations < 8
        || c.coefficient_work < 112
        || c.proof_cells < 4
    {
        return Err(refused(
            "Reserve work for both UV curves, world binding and both final caps",
        ));
    }
    Ok(())
}
/// Construct one closed two-patch skin on a specified original contact BAND.
/// Caller supplies no caps, UV curves/maps, world seam or positive receipts.
/// Original complete shared-boundary discovery chooses the far sides. Fresh UV
/// region/trace and world-curve proofs propose caps; the SAME actual world net
/// binds both contact boundaries, including clock reversal and basis elevation.
/// Elevation/binding are proposals only: world error, complete original normal
/// displacement and actual closed boundary/lift/material-role gates repeat.
///
/// Branch section/coefficient/cell budgets are shared by UV fitting, world fit
/// and final world proof. Cap normal cells cover proposals AND final cap proofs;
/// original-seam work is deducted from final sewing allowances. This does not
/// discover the band, extend supports, cover omitted original cap parameters or
/// prove full-root coverage/global ideal-offset embedding or arbitrary solids.
pub fn thicken(
    sources: &[Surface; 2],
    reversed: [bool; 2],
    signed_thickness: f64,
    band: Band,
    tolerances: Tolerances,
    limits: Limits,
) -> Result<Thickness> {
    let Tolerances {
        uv: uv_tolerance,
        normal: normal_tolerance,
        sewing: sewing_tolerance,
    } = tolerances;
    validate(limits)?;
    super::freeform_network::validate(
        sources,
        sources,
        &reversed,
        crate::shell_thickness::planar_network::OffsetSettings {
            signed_thickness,
            normal_tolerance,
            normal_cells: limits.caps.proof_cells,
            sewing_tolerance,
        },
        limits.sewing,
    )?;
    if !uv_tolerance.is_finite() || uv_tolerance <= 0. || normal_tolerance * 0.5 == 0. {
        return Err(refused(
            "Choose positive finite UV and normal proposal tolerances",
        ));
    }
    let (seam, source_cells, source_exact) =
        super::intersected_pair::discover(sources, reversed, sewing_tolerance, limits.sewing)?;
    let distances = reversed.map(|r| {
        if r {
            -signed_thickness
        } else {
            signed_thickness
        }
    });
    let f = limits.branch_fit;
    let uv_work = FitLimits {
        root_queries: f.root_queries / 3,
        coefficient_work: f.coefficient_work / 3,
        proof_cells: f.proof_cells / 3,
        ..f
    };
    let regions = regions::fit_pair(
        nurbs_core::offset_contact_pcurve::OffsetFaces { surfaces: sources.each_ref(), distances },
        nurbs_core::surface_offset::contact_curve::ContactDomain { axis: band.axis, drive: band.drive, free: band.first_other, second: band.second },
        seam.map(|s| (s + 2) % 4),
        uv_tolerance,
        uv_work,
    )?;
    let mut root_queries = regions.fitting().root_queries;
    let mut branch_cells = regions.fitting().proof_cells;
    let mut branch_coefficients = regions.fitting().coefficient_work;
    let world_work = FitLimits {
        root_queries: (f.root_queries - root_queries) / 2,
        proof_cells: (f.proof_cells - branch_cells) / 2,
        coefficient_work: f.coefficient_work - branch_coefficients,
        ..f
    };
    let world_fit = contact_curve::fit(
        nurbs_core::offset::contact_pcurve::OffsetFaces {
            surfaces: sources.each_ref(),
            distances,
        },
        band.axis,
        band.drive,
        band.first_other,
        band.second,
        sewing_tolerance,
        world_work,
    )?;
    root_queries += world_fit.root_queries;
    branch_cells += world_fit.proof_cells;
    branch_coefficients += world_fit.coefficient_work;
    let initial_curve = world_fit
        .curve
        .as_ref()
        .ok_or_else(|| refused("Original world contact curve remains unqualified"))?;
    let quota = limits.caps.proof_cells / 2;
    let mut proposals = Vec::with_capacity(2);
    let (mut cap_evaluations, mut cap_coefficients, mut proposal_normal_cells) =
        (0usize, 0usize, 0usize);
    let mut candidates = vec![];
    for i in 0..2 {
        let remaining = ProposalLimits {
            evaluations: limits
                .caps
                .evaluations
                .checked_sub(cap_evaluations)
                .filter(|n| *n > 0)
                .ok_or_else(|| refused("Shared cap station work exhausted"))?,
            coefficient_work: limits
                .caps
                .coefficient_work
                .checked_sub(cap_coefficients)
                .filter(|n| *n > 0)
                .ok_or_else(|| refused("Shared cap coefficient work exhausted"))?,
            proof_cells: quota
                .checked_sub(proposal_normal_cells)
                .and_then(|n| n.checked_sub(1 - i))
                .filter(|n| *n > 0)
                .ok_or_else(|| refused("Shared cap proposal normal work exhausted"))?,
            ..limits.caps
        };
        let report = surface_offset::propose_parametric_candidate(
            &sources[i],
            regions.strips()[i].map(),
            distances[i],
            normal_tolerance * 0.5,
            remaining,
        )?;
        cap_evaluations += report.evaluations;
        cap_coefficients += report.coefficient_work;
        proposal_normal_cells += report.proof_cells;
        candidates.push(
            report
                .cap
                .as_ref()
                .ok_or_else(|| refused("Original mapped cap proposal remains unqualified"))?
                .clone(),
        );
        proposals.push(report);
    }
    let mut caps: [Surface; 2] = candidates
        .try_into()
        .map_err(|_| refused("Invalid internal cap count"))?;
    let curve = binding::bind(&mut caps, &regions, initial_curve)?;
    let remaining_world_cells = (f.proof_cells - branch_cells)
        .min((f.root_queries - root_queries) / (f.root_refinements + 1));
    if remaining_world_cells == 0 {
        return Err(refused("Final actual world-curve proof work exhausted"));
    }
    let world_proof = contact_curve::certify(&curve, nurbs_core::offset::contact_pcurve::OffsetFaces { surfaces: sources.each_ref(), distances }, contact_curve::ContactDomain { axis: band.axis, drive: band.drive, free: band.first_other, second: band.second }, sewing_tolerance, contact_curve::Limits {
            cells: remaining_world_cells,
            spans: f.spans,
            section_refinements: f.root_refinements,
            band_refinements: f.root_refinements,
        })?;
    root_queries += world_proof.section_queries;
    branch_cells += world_proof.visited;
    if !world_proof.world_correspondence_proven {
        return Err(refused(
            "Elevated actual contact boundary lacks complete world authority",
        ));
    }
    let mut sewing = limits.sewing;
    sewing.proof.max_cells = sewing
        .proof
        .max_cells
        .checked_sub(source_cells)
        .filter(|n| *n > 0)
        .ok_or_else(|| refused("Source-seam discovery consumed sewing proof work"))?;
    sewing.exact_work = sewing
        .exact_work
        .checked_sub(source_exact)
        .filter(|n| *n > 0)
        .ok_or_else(|| refused("Source-seam discovery consumed exact sewing work"))?;
    let maps = regions.strips().each_ref().map(|r| r.map().clone());
    let remaining_normal = limits.caps.proof_cells - proposal_normal_cells;
    let skin = parametric_network::thicken_with_maps(
        sources,
        &caps,
        &maps,
        &reversed,
        parametric_network::OffsetSettings {
            signed_thickness,
            normal_tolerance,
            normal_cells: remaining_normal,
            sewing_tolerance,
        },
        sewing,
    )?;
    Ok(Thickness {
        sources: sources.clone(),
        reversed,
        distance: signed_thickness,
        band,
        uv_tolerance,
        normal_tolerance,
        sewing_tolerance,
        limits,
        seam,
        regions,
        world_fit,
        curve,
        world_proof,
        proposals,
        skin,
        root_queries,
        branch_cells,
        branch_coefficients,
        cap_evaluations,
        cap_coefficients,
        proposal_normal_cells,
        source_cells,
        source_exact,
    })
}
