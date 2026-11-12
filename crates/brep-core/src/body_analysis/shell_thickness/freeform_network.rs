//! Closed skins of coherent authored nonplanar cap networks. Continuous normal
//! displacement is checked on original charts; no global solid embedding or
//! closest-thickness claim is made. Creased offset intersections are not solved.
pub use super::planar_network::{Boundary, Cut, CutKind, Limits, OffsetSettings};
use crate::feature_ops::region_pattern::{Verification, qualification};
use crate::{Error, Model, Result};
use nurbs_core::{
    surface::Surface,
    surface_offset::{self, CandidateCertificate, CandidateStatus},
};
fn refused(message: &str) -> Error {
    Error::new("BREP_FREEFORM_NETWORK_REFUSED", message)
}

pub struct Thickness {
    sources: Vec<Surface>,
    caps: Vec<Surface>,
    reversed: Vec<bool>,
    distance: f64,
    normal_tolerance: f64,
    normal_limit: usize,
    sewing_tolerance: f64,
    limits: Limits,
    built: super::network::Built,
    certificates: Vec<CandidateCertificate>,
    normal_cells: usize,
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
    pub fn exact_work(&self) -> u64 {
        self.built.exact_work
    }
    /// Pointwise displacement error at paired original UV, not closest distance.
    pub fn normal_offset_error_upper(&self) -> f64 {
        self.certificates
            .iter()
            .map(|c| c.error_upper.unwrap())
            .fold(0., f64::max)
    }
    pub fn reverify(&self) -> Result<Verification> {
        let fresh = thicken_with_caps(
            &self.sources,
            &self.caps,
            &self.reversed,
            OffsetSettings {
                signed_thickness: self.distance,
                normal_tolerance: self.normal_tolerance,
                normal_cells: self.normal_limit,
                sewing_tolerance: self.sewing_tolerance,
            },
            self.limits,
        )?;
        if fresh.built.model != self.built.model
            || fresh.built.boundaries != self.built.boundaries
            || fresh.built.cuts != self.built.cuts
        {
            return Err(refused(
                "Saved freeform skin differs from original reconstruction",
            ));
        }
        Ok(fresh.built.evidence)
    }
}
/// Close 1..32 connected full Bezier source charts with coherent authored caps.
/// Each cap must have the identical positive rational basis as its source.
/// A single normal-cell allowance is shared across ALL original charts, with
/// orientation applied before normal comparison. All saved caps and walls need
/// independent full-chart, boundary/lift, trim and material-role qualification.
/// Only exposed source pieces receive walls; original interior seams remain.
/// Supplied caps must already meet across seams: this does not intersect offset
/// supports or heal gaps, and tolerance never substitutes for a proof.
pub fn thicken_with_caps(sources: &[Surface], caps: &[Surface], reversed: &[bool], offset: crate::shell_thickness::planar_network::OffsetSettings, limits: Limits) -> Result<Thickness> {
        let crate::shell_thickness::planar_network::OffsetSettings { signed_thickness, normal_tolerance, normal_cells, sewing_tolerance } = offset;
    validate(
        sources,
        caps,
        reversed,
        crate::shell_thickness::planar_network::OffsetSettings {
            signed_thickness,
            normal_tolerance,
            normal_cells,
            sewing_tolerance,
        },
        limits,
    )?;
    let mut used = 0;
    let mut certificates = Vec::with_capacity(sources.len());
    for ((source, cap), reverse) in sources.iter().zip(caps).zip(reversed) {
        let remaining = normal_cells
            .checked_sub(used)
            .filter(|n| *n > 0)
            .ok_or_else(|| refused("Shared normal-offset proof allowance exhausted"))?;
        let proof = surface_offset::certify_candidate(
            source,
            cap,
            if *reverse {
                -signed_thickness
            } else {
                signed_thickness
            },
            normal_tolerance,
            remaining,
        )?;
        used += proof.cells;
        if proof.status != CandidateStatus::Within {
            return Err(refused(
                "Original-chart continuous normal offset exceeds tolerance or remains unresolved",
            ));
        }
        certificates.push(proof);
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
        reversed: reversed.to_vec(),
        distance: signed_thickness,
        normal_tolerance,
        normal_limit: normal_cells,
        sewing_tolerance,
        limits,
        built,
        certificates,
        normal_cells: used,
    })
}

pub(super) fn validate(
    sources: &[Surface],
    caps: &[Surface],
    reversed: &[bool],
    offset: OffsetSettings,
    limits: Limits,
) -> Result<()> {
    let OffsetSettings {
        signed_thickness,
        normal_tolerance,
        normal_cells,
        sewing_tolerance,
    } = offset;
    qualification::limits(limits.proof)?;
    if sources.is_empty()
        || sources.len() > 32
        || caps.len() != sources.len()
        || reversed.len() != sources.len()
        || !signed_thickness.is_finite()
        || signed_thickness == 0.
        || signed_thickness.abs() > 1e6
        || !normal_tolerance.is_finite()
        || normal_tolerance <= 0.
        || normal_tolerance >= signed_thickness.abs()
        || !(1..=1_000_000).contains(&normal_cells)
        || !sewing_tolerance.is_finite()
        || !(1e-10..=1e-2).contains(&sewing_tolerance)
        || !(1..=1_000_000).contains(&limits.exact_work)
    {
        return Err(refused(
            "Expected bounded coherent source/cap network and independent positive proof limits",
        ));
    }
    for s in sources.iter().chain(caps) {
        s.validate()?;
        if s.periodic_u
            || s.periodic_v
            || !(1..=32).contains(&s.degree_u)
            || !(1..=32).contains(&s.degree_v)
            || !crate::rounded_patch_network::clamped_single(
                &s.knots_u,
                s.degree_u,
                s.control_points.len(),
            )
            || !crate::rounded_patch_network::clamped_single(
                &s.knots_v,
                s.degree_v,
                s.control_points[0].len(),
            )
            || s.control_points
                .iter()
                .flatten()
                .any(|p| p.len() != 3 || p.iter().any(|v| v.abs() > 1e6))
        {
            return Err(refused(
                "Expected bounded full nonperiodic rational Bezier charts",
            ));
        }
    }
    Ok(())
}

/// Propose offset caps with the native offset owner, then apply the SAME full
/// original-domain and closed-network gates as `thicken_with_caps`. Sampled
/// recognition is proposal-only. Changed bases fall back to same-basis poles.
/// Compatible seams must survive; discontinuous proposals are refused rather
/// than bridged by internal walls or silently healed.
#[allow(clippy::too_many_arguments)]
pub fn thicken(
    sources: &[Surface],
    reversed: &[bool],
    signed_thickness: f64,
    normal_tolerance: f64,
    normal_cells: usize,
    sewing_tolerance: f64,
    limits: Limits,
) -> Result<Thickness> {
    validate(
        sources,
        sources,
        reversed,
        crate::shell_thickness::planar_network::OffsetSettings {
            signed_thickness,
            normal_tolerance,
            normal_cells,
            sewing_tolerance,
        },
        limits,
    )?;
    let mut caps = Vec::with_capacity(sources.len());
    for (source, reverse) in sources.iter().zip(reversed) {
        let distance = if *reverse {
            -signed_thickness
        } else {
            signed_thickness
        };
        let proposal = surface_offset::offset(source, distance, normal_tolerance * 0.125)?;
        let p = proposal.surface;
        let cap = if p.degree_u == source.degree_u
            && p.degree_v == source.degree_v
            && p.knots_u == source.knots_u
            && p.knots_v == source.knots_v
            && p.weights == source.weights
        {
            p
        } else {
            surface_offset::offset_preserving_basis(source, distance)?.surface
        };
        caps.push(cap);
    }
    thicken_with_caps(
        sources,
        &caps,
        reversed,
        OffsetSettings {
            signed_thickness,
            normal_tolerance,
            normal_cells,
            sewing_tolerance,
        },
        limits,
    )
}
