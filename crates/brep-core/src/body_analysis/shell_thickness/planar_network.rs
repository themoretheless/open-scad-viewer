//! Closed normal-thickness skins of connected original coplanar patch networks.
//! Original rational cap charts and interior seams survive; exposed boundaries
//! alone receive ruled walls. Continuous original normal-offset error is bounded.
use crate::feature_ops::region_pattern::{Limits as ProofLimits, Verification, qualification};
use crate::{Error, Model, Result, rounded_patch_network as network};
use nurbs_core::{curve::Curve, interval_eval::Interval as I, surface::Surface};
pub(super) mod frame;
pub(super) mod prepare;
pub use prepare::{Cut, CutKind};
pub(super) mod topology;
fn refused(message: &str) -> Error {
    Error::new("BREP_PLANAR_THICKNESS_REFUSED", message)
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub proof: ProofLimits,
    pub exact_work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            proof: ProofLimits::default(),
            exact_work: 1000000,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Boundary {
    pub patch: usize,
    pub side: usize,
    /// Original directed boundary-curve clock covered by this actual segment.
    pub parameters: [f64; 2],
}
pub struct Thickness {
    originals: Vec<Surface>,
    orientations: Vec<bool>,
    thickness: f64,
    tolerance: f64,
    limits: Limits,
    model: Model,
    evidence: Verification,
    boundaries: Vec<Boundary>,
    cuts: Vec<Cut>,
    offset_error_upper: f64,
    exact_work: u64,
}
impl Thickness {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn source_patches(&self) -> &[Surface] {
        &self.originals
    }
    pub fn exposed_boundaries(&self) -> &[Boundary] {
        &self.boundaries
    }
    pub fn cuts(&self) -> &[Cut] {
        &self.cuts
    }
    pub fn evidence(&self) -> &Verification {
        &self.evidence
    }
    /// Whole original rational carriers versus their actual saved normal-offset caps.
    pub fn normal_offset_error_upper(&self) -> f64 {
        self.offset_error_upper
    }
    /// Closest distances in both directions between the ORIGINAL cap union
    /// and the actual offset cap union. The source is exactly coplanar and
    /// every paired saved cap point is within the normal-offset error bound;
    /// plane separation supplies the lower bound even across different patches.
    /// This does not certify distances to the connecting side walls.
    pub fn cap_distance_bounds(&self) -> [f64; 2] {
        [
            (self.thickness.abs() - self.offset_error_upper)
                .next_down()
                .max(0.),
            (self.thickness.abs() + self.offset_error_upper).next_up(),
        ]
    }
    pub fn exact_work(&self) -> u64 {
        self.exact_work
    }
    pub fn reverify(&self) -> Result<Verification> {
        let actual = thicken(
            &self.originals,
            &self.orientations,
            self.thickness,
            self.tolerance,
            self.limits,
        )?;
        if actual.model != self.model
            || actual.boundaries != self.boundaries
            || actual.cuts != self.cuts
        {
            return Err(refused(
                "Saved planar skin differs from fresh original reconstruction",
            ));
        }
        Ok(actual.evidence)
    }
}
/// One coherently oriented connected network of 1..32 coplanar full rational
/// Bezier charts, with at most two uses per admitted boundary segment. Original
/// straight roots or bounded curved witnesses propose boundary partitions. Complete
/// source chart regularity/injectivity and actual closed BRep qualification are
/// mandatory. This is not general curved-face offset or global solid embedding.
pub fn thicken(
    sources: &[Surface],
    reversed: &[bool],
    signed_thickness: f64,
    tolerance: f64,
    limits: Limits,
) -> Result<Thickness> {
    qualification::limits(limits.proof)?;
    if sources.is_empty()
        || sources.len() > 32
        || reversed.len() != sources.len()
        || !signed_thickness.is_finite()
        || signed_thickness == 0.
        || signed_thickness.abs() > 1e6
        || !tolerance.is_finite()
        || !(1e-10..=1e-2).contains(&tolerance)
        || !(1..=1000000).contains(&limits.exact_work)
    {
        return Err(refused(
            "Choose bounded nonzero thickness, tolerance and 1..32 oriented planar patches",
        ));
    }
    for s in sources {
        s.validate()?;
        if s.periodic_u
            || s.periodic_v
            || !network::clamped_single(&s.knots_u, s.degree_u, s.control_points.len())
            || !network::clamped_single(&s.knots_v, s.degree_v, s.control_points[0].len())
            || s.control_points
                .iter()
                .flatten()
                .any(|p| p.len() != 3 || p.iter().any(|v| v.abs() > 1e6))
        {
            return Err(refused(
                "Planar skin requires bounded full nonperiodic rational Bezier source charts",
            ));
        }
    }
    let mut cells = 0;
    let offset = frame::offset(
        sources,
        reversed,
        signed_thickness,
        tolerance,
        limits.proof.max_cells,
        &mut cells,
    )?;
    let built = super::network::construct(
        sources,
        &offset.patches,
        reversed,
        signed_thickness,
        tolerance,
        limits,
        &mut cells,
    )?;
    Ok(Thickness {
        originals: sources.to_vec(),
        orientations: reversed.to_vec(),
        thickness: signed_thickness,
        tolerance,
        limits,
        model: built.model,
        evidence: built.evidence,
        boundaries: built.boundaries,
        cuts: built.cuts,
        offset_error_upper: offset.error_upper,
        exact_work: built.exact_work,
    })
}
