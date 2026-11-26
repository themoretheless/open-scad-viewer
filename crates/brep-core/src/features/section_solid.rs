use crate::profile::section_caps::{
    AttachedSectionCaps, BoundedSectionAttachment, BoundedSectionPlane, SectionAttachmentOptions,
    SectionPlane, attach_section_caps_with_expanded_exact_support, close_inferred,
    close_shell_with_bounded_inferred_section_caps, close_shell_with_bounded_section_caps,
    close_shell_with_expanded_exact_section_caps, close_shell_with_section_caps, invalid,
};
use crate::{
    Body, Model,
    analysis::{CertifiedFreeformVolume, qualified_section_volume},
    closed_solid::{AdmissionPolicy, ClosedSolidAdmission, QualifiedClosedSolid, qualify},
};
use num_rational::BigRational;
use nurbs_core::{Error, Result};

#[derive(Clone, Copy)]
pub struct SectionSolidOptions {
    pub max_exact_work: u64,
    pub max_verification_cells: usize,
    pub admission: AdmissionPolicy,
    pub volume_tolerance_mm3: f64,
    pub max_volume_cells: usize,
}

pub struct SectionSolid {
    pub solid: QualifiedClosedSolid,
    pub cap_faces: Vec<usize>,
    pub volume: CertifiedFreeformVolume,
    pub attachment_exact_work: u64,
    pub attachment_verification_cells: usize,
}

/// Cap one standalone shell, then prove its boundary embedding and outward
/// material orientation before returning a body with a positive volume enclosure.
pub fn build_section_solid(
    source: &Model,
    planes: &[SectionPlane],
    options: SectionSolidOptions,
) -> Result<SectionSolid> {
    require_standalone_shell(source)?;
    let attachment = close_shell_with_section_caps(
        source,
        0,
        planes,
        options.max_exact_work,
        options.max_verification_cells,
    )?;
    qualify_attachment(source, attachment, options)
}

/// Close a standalone shell along the supplied planes with exact supports
/// that may expand in UV. Return a solid only after boundary, material
/// orientation and positive volume verification.
pub fn build_expanded_section_solid(
    source: &Model,
    planes: &[SectionPlane],
    options: SectionSolidOptions,
) -> Result<SectionSolid> {
    require_standalone_shell(source)?;
    let attachment = close_shell_with_expanded_exact_section_caps(
        source,
        0,
        planes,
        options.max_exact_work,
        options.max_verification_cells,
    )?;
    qualify_attachment(source, attachment, options)
}

pub struct OrientedSectionSolid {
    pub section: SectionSolid,
    pub orientation_reversed: bool,
}

/// Close a standalone shell with expanded exact caps, and correct a proven
/// inward orientation by reversing all face uses together. Admission limits
/// apply independently to the initial and, if needed, corrected candidate.
pub fn build_outward_section_solid(
    source: &Model,
    planes: &[SectionPlane],
    options: SectionSolidOptions,
) -> Result<OrientedSectionSolid> {
    require_standalone_shell(source)?;
    let attachment = close_shell_with_expanded_exact_section_caps(
        source,
        0,
        planes,
        options.max_exact_work,
        options.max_verification_cells,
    )?;
    qualify_outward_attachment(source, attachment, options)
}

fn qualify_outward_attachment(
    source: &Model,
    attachment: AttachedSectionCaps,
    options: SectionSolidOptions,
) -> Result<OrientedSectionSolid> {
    let mut admission = admit_model(source, attachment.model, options.admission)?;
    let evidence = admission.geometry();
    let orientation_reversed = evidence.boundary.proven
        && evidence
            .nesting
            .as_ref()
            .is_some_and(|nesting| nesting.roles_consistent == Some(true))
        && evidence.orientations[0].outward == Some(false);
    if orientation_reversed {
        let mut candidate = admission.candidate().clone();
        for face in &mut candidate.shells[0].faces {
            face.reversed = !face.reversed;
        }
        candidate.inherit_topology_ids(&[source]);
        admission = qualify(candidate, options.admission)?;
    }
    let section = finish_admission(
        admission,
        attachment.cap_faces,
        attachment.exact_work,
        attachment.verification_cells,
        options,
    )?;
    Ok(OrientedSectionSolid {
        section,
        orientation_reversed,
    })
}

pub struct BoundedSectionSolid {
    pub section: SectionSolid,
    /// Continuous displacement bounds relative to the requested support
    /// planes, in the same order as the cap face indices.
    pub support_error_upper: Vec<BigRational>,
}

/// Cap a standalone shell using per-plane tolerances and qualify the stored
/// boundary. The volume enclosure describes that stored solid.
pub fn build_bounded_section_solid(
    source: &Model,
    planes: &[BoundedSectionPlane],
    options: SectionSolidOptions,
) -> Result<BoundedSectionSolid> {
    require_standalone_shell(source)?;
    let bounded = close_shell_with_bounded_section_caps(
        source,
        0,
        planes,
        options.max_exact_work,
        options.max_verification_cells,
    )?;
    qualify_bounded_attachment(source, bounded, options)
}

/// Infer all cap planes from the free rims of a standalone shell and qualify
/// the resulting solid. The same support tolerance applies to each inferred
/// cap. The returned volume bounds the stored geometry.
pub fn build_inferred_section_solid(
    source: &Model,
    support_tolerance_mm: f64,
    options: SectionSolidOptions,
) -> Result<BoundedSectionSolid> {
    let report = inspect_inferred_section_solid(
        source,
        SectionAttachmentOptions {
            max_exact_work: options.max_exact_work,
            max_verification_cells: options.max_verification_cells,
            support_tolerance_mm,
        },
        options.admission,
    )?;
    let section = finish_admission(
        report.admission,
        report.cap_faces,
        report.attachment_exact_work,
        report.attachment_verification_cells,
        options,
    )?;
    Ok(BoundedSectionSolid {
        section,
        support_error_upper: report.support_error_upper,
    })
}

/// Infer the section planes and qualify the resulting solid with exact caps.
/// The supports may expand in UV to preserve an exactly planar free rim.
pub fn build_expanded_inferred_section_solid(
    source: &Model,
    options: SectionSolidOptions,
) -> Result<SectionSolid> {
    require_standalone_shell(source)?;
    let attachment = close_expanded_inferred(source, options)?;
    qualify_attachment(source, attachment, options)
}

/// Infer exact cap planes, then correct the orientation of an embedded shell
/// when it is proven inward. Each admission has its own geometry budget.
pub fn build_outward_inferred_section_solid(
    source: &Model,
    options: SectionSolidOptions,
) -> Result<OrientedSectionSolid> {
    require_standalone_shell(source)?;
    let attachment = close_expanded_inferred(source, options)?;
    qualify_outward_attachment(source, attachment, options)
}

fn close_expanded_inferred(
    source: &Model,
    options: SectionSolidOptions,
) -> Result<AttachedSectionCaps> {
    close_inferred(
        source,
        0,
        options.max_exact_work,
        options.max_verification_cells,
        |current, rings, normal, offset, work, cells| {
            attach_section_caps_with_expanded_exact_support(
                current, 0, rings, normal, offset, work, cells,
            )
        },
    )
}

pub struct InferredSectionSolidAdmission {
    pub admission: ClosedSolidAdmission,
    pub cap_faces: Vec<usize>,
    pub support_error_upper: Vec<BigRational>,
    pub attachment_exact_work: u64,
    pub attachment_verification_cells: usize,
}

/// Inspect the capped candidate's boundary, nesting and orientation evidence.
/// An unproved admission retains the candidate and complete stage evidence.
/// Capping errors are returned directly. Volume integration is a separate stage.
pub fn inspect_inferred_section_solid(
    source: &Model,
    attachment_options: SectionAttachmentOptions,
    policy: AdmissionPolicy,
) -> Result<InferredSectionSolidAdmission> {
    require_standalone_shell(source)?;
    let bounded = close_shell_with_bounded_inferred_section_caps(source, 0, attachment_options)?;
    let attachment = bounded.attachment;
    let admission = admit_model(source, attachment.model, policy)?;
    Ok(InferredSectionSolidAdmission {
        admission,
        cap_faces: attachment.cap_faces,
        support_error_upper: bounded.support_error_upper,
        attachment_exact_work: attachment.exact_work,
        attachment_verification_cells: attachment.verification_cells,
    })
}

fn qualify_bounded_attachment(
    source: &Model,
    bounded: BoundedSectionAttachment,
    options: SectionSolidOptions,
) -> Result<BoundedSectionSolid> {
    let section = qualify_attachment(source, bounded.attachment, options)?;
    Ok(BoundedSectionSolid {
        section,
        support_error_upper: bounded.support_error_upper,
    })
}

fn require_standalone_shell(source: &Model) -> Result<()> {
    if source.shells.len() != 1 || !source.bodies.is_empty() {
        return Err(invalid("Section solid requires one standalone shell"));
    }
    Ok(())
}

fn qualify_attachment(
    source: &Model,
    attachment: AttachedSectionCaps,
    options: SectionSolidOptions,
) -> Result<SectionSolid> {
    let admission = admit_model(source, attachment.model, options.admission)?;
    finish_admission(
        admission,
        attachment.cap_faces,
        attachment.exact_work,
        attachment.verification_cells,
        options,
    )
}

fn admit_model(
    source: &Model,
    mut model: Model,
    policy: AdmissionPolicy,
) -> Result<ClosedSolidAdmission> {
    model.bodies.push(Body {
        outer_shell: 0,
        inner_shells: Vec::new(),
    });
    model.inherit_topology_ids(&[source]);
    qualify(model, policy)
}

fn finish_admission(
    admission: ClosedSolidAdmission,
    cap_faces: Vec<usize>,
    attachment_exact_work: u64,
    attachment_verification_cells: usize,
    options: SectionSolidOptions,
) -> Result<SectionSolid> {
    let solid = admission.into_solid().ok_or_else(|| {
        Error::new(
            "BREP_CLOSED_SOLID_UNPROVED",
            "Section caps did not establish an embedded outward material boundary",
        )
    })?;
    let volume = qualified_section_volume(
        &solid,
        options.volume_tolerance_mm3,
        options.max_volume_cells,
    )?;
    Ok(SectionSolid {
        solid,
        cap_faces,
        volume,
        attachment_exact_work,
        attachment_verification_cells,
    })
}
