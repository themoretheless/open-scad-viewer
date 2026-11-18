//! Closed skins with globally admitted curved rational original-UV maps.
//! Offset error and actual retained topology receive independent admission.
pub use super::planar_network::{Boundary, Cut, CutKind, Limits, OffsetSettings};
use crate::feature_ops::region_pattern::Verification;
use crate::{Error, Model, Result};
use nurbs_core::{
    surface::Surface,
    surface_offset::{self, CandidateCertificate, CandidateStatus},
};
mod automatic;
pub use automatic::{AutomaticThickness, MappedSources, thicken_from_maps};
fn refused(s: &str) -> Error {
    Error::new("BREP_PARAMETRIC_OFFSET_NETWORK_REFUSED", s)
}
pub struct Thickness {
    sources: Vec<Surface>,
    caps: Vec<Surface>,
    maps: Vec<Surface>,
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
    pub fn source_maps(&self) -> &[Surface] {
        &self.maps
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
    /// original rational UV maps. Does not cover omitted original source parameters,
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
        let actual = thicken_with_maps(
            &self.sources,
            &self.caps,
            &self.maps,
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
                "Saved parametric skin differs from original geometry/UV reconstruction",
            ));
        }
        Ok(actual.built.evidence)
    }
}
/// Close a full-patch source network against authored caps with rational UV
/// maps `(source_u,source_v,0)` on each literal candidate domain. Curved map
/// boundaries and independent candidate degrees/weights are allowed. A whole
/// source-map injectivity/orientation proof and continuous normal-offset error
/// bound admit each cap before any topology is published.
///
/// Sources remain literal full patches. Normal displacement is asserted only
/// over each mapped cap region, not omitted source parameters or side walls.
/// The existing network owner adds aligned ruled walls only on exposed sides,
/// freshly checks all actual shared cap seams, chart/lift/closed incidence and
/// returns immutable saved data with complete replay. Partitioned clocks must
/// satisfy that owner's compatibility contract. This does not discover maps,
/// continue contact branches, self-trim offsets or certify global embedding.
pub fn thicken_with_maps(
    sources: &[Surface],
    caps: &[Surface],
    maps: &[Surface],
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
    if maps.len() != sources.len() {
        return Err(refused(
            "Each cap requires its explicit original rational UV map",
        ));
    }
    let mut used = 0;
    let mut certificates = Vec::with_capacity(sources.len());
    for (((source, cap), uv_map), reverse) in sources.iter().zip(caps).zip(maps).zip(reversed) {
        let remaining = normal_cells
            .checked_sub(used)
            .filter(|n| *n > 0)
            .ok_or_else(|| refused("Shared mapped normal-offset allowance exhausted"))?;
        let certificate = surface_offset::certify_parametric_candidate(
            source,
            cap,
            uv_map,
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
                "Original-parametric normal offset exceeds tolerance or remains unresolved",
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
        maps: maps.to_vec(),
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
