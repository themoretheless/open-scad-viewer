//! Closed skins with bilinear nonrectangular original-UV cap correspondence.
//! Offset error and actual retained topology receive independent admission.
pub use super::planar_network::{Boundary, Cut, CutKind, Limits, OffsetSettings};
use crate::feature_ops::region_pattern::Verification;
use crate::{Error, Model, Result};
use nurbs_core::{
    surface::Surface,
    surface_offset::{self, CandidateCertificate, CandidateStatus},
};
/// Original UV corners `[u_end][v_end]` corresponding to candidate corners.
pub type SourceRegion = [[[f64; 2]; 2]; 2];
fn refused(s: &str) -> Error {
    Error::new("BREP_QUADRILATERAL_OFFSET_NETWORK_REFUSED", s)
}
pub struct Thickness {
    sources: Vec<Surface>,
    caps: Vec<Surface>,
    regions: Vec<SourceRegion>,
    reversed: Vec<bool>,
    distance: f64,
    normal_tolerance: f64,
    normal_limit: usize,
    sewing_tolerance: f64,
    limits: Limits,
    certificates: Vec<CandidateCertificate>,
    normal_cells: usize,
    built: super::network::Built,
}
impl Thickness {
    pub fn model(&self) -> &Model {
        &self.built.model
    }
    pub fn source_patches(&self) -> &[Surface] {
        &self.sources
    }
    pub fn cap_patches(&self) -> &[Surface] {
        &self.caps
    }
    pub fn source_regions(&self) -> &[SourceRegion] {
        &self.regions
    }
    pub fn exposed_boundaries(&self) -> &[Boundary] {
        &self.built.boundaries
    }
    pub fn cuts(&self) -> &[Cut] {
        &self.built.cuts
    }
    pub fn evidence(&self) -> &Verification {
        &self.built.evidence
    }
    pub fn normal_certificates(&self) -> &[CandidateCertificate] {
        &self.certificates
    }
    pub fn normal_cells(&self) -> usize {
        self.normal_cells
    }
    /// Whole saved caps compared to normal offsets on the corresponding
    /// original UV quadrilaterals. Does not cover omitted original source parameters,
    /// or certify closest thickness, support extension, or global embedding.
    pub fn mapped_normal_offset_error_upper(&self) -> f64 {
        self.certificates
            .iter()
            .map(|c| c.error_upper.unwrap())
            .fold(0., f64::max)
    }
    pub fn exact_work(&self) -> u64 {
        self.built.exact_work
    }
    pub fn reverify(&self) -> Result<Verification> {
        let actual = thicken_with_caps(
            &self.sources,
            &self.caps,
            &self.regions,
            &self.reversed,
            OffsetSettings {
                signed_thickness: self.distance,
                normal_tolerance: self.normal_tolerance,
                normal_cells: self.normal_limit,
                sewing_tolerance: self.sewing_tolerance,
            },
            self.limits,
        )?;
        if actual.built.model != self.built.model
            || actual.built.boundaries != self.built.boundaries
            || actual.built.cuts != self.built.cuts
        {
            return Err(refused(
                "Saved quadrilateral skin differs from original geometry/UV reconstruction",
            ));
        }
        Ok(actual.built.evidence)
    }
}
/// Close an original full-patch network against authored independently based
/// caps. Each full cap needs a positive-orientation bilinear map into its
/// original UV quadrilateral and a complete normal-offset error bound. Four
/// straight source UV sides may be nonparallel. This does not infer a region.
///
/// Retains literal full source caps; omitted parameters of a source cap have
/// no normal-offset assertion. Only exposed original boundary sides get ruled
/// walls, aligned to the corresponding cap sides. Internal cap seams must
/// freshly sew. Partitioned boundary clocks still require the shared network
/// owner's exact compatibility contract. Complete chart, edge/lift, closed
/// incidence and saved-model replay gates remain mandatory. Global solid
/// embedding, self-trimming and arbitrary curved source trims remain open.
pub fn thicken_with_caps(
    sources: &[Surface],
    caps: &[Surface],
    regions: &[SourceRegion],
    reversed: &[bool],
    offset: OffsetSettings,
    limits: Limits,
) -> Result<Thickness> {
    let OffsetSettings {
        signed_thickness,
        normal_tolerance,
        normal_cells,
        sewing_tolerance,
    } = offset;
    super::freeform_network::validate(sources, caps, reversed, offset, limits)?;
    if regions.len() != sources.len() {
        return Err(refused(
            "Each cap requires its explicit original UV quadrilateral",
        ));
    }
    let mut used = 0;
    let mut certificates = Vec::with_capacity(sources.len());
    for (((source, cap), region), reverse) in sources.iter().zip(caps).zip(regions).zip(reversed) {
        let remaining = normal_cells
            .checked_sub(used)
            .filter(|n| *n > 0)
            .ok_or_else(|| refused("Shared mapped normal-offset allowance exhausted"))?;
        let certificate = surface_offset::certify_quadrilateral_candidate(
            source,
            cap,
            *region,
            if *reverse {
                -signed_thickness
            } else {
                signed_thickness
            },
            normal_tolerance,
            remaining,
        )?;
        used += certificate.cells;
        if certificate.status != CandidateStatus::Within {
            return Err(refused(
                "Original-quadrilateral normal offset exceeds tolerance or remains unresolved",
            ));
        }
        certificates.push(certificate);
    }
    let built = super::network::construct(
        sources,
        caps,
        reversed,
        signed_thickness,
        sewing_tolerance,
        limits,
        &mut 0,
    )?;
    Ok(Thickness {
        sources: sources.to_vec(),
        caps: caps.to_vec(),
        regions: regions.to_vec(),
        reversed: reversed.to_vec(),
        distance: signed_thickness,
        normal_tolerance,
        normal_limit: normal_cells,
        sewing_tolerance,
        limits,
        certificates,
        normal_cells: used,
        built,
    })
}
