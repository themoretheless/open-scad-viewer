use super::{SectionPlane, close_shell_with_section_caps, invalid};
use crate::{
    Body, Model,
    analysis::{CertifiedFreeformVolume, certified_freeform_volume},
    closed_solid::{AdmissionPolicy, QualifiedClosedSolid, qualify},
};
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
    if source.shells.len() != 1 || !source.bodies.is_empty() {
        return Err(invalid("Section solid requires one standalone shell"));
    }
    let attachment = close_shell_with_section_caps(
        source,
        0,
        planes,
        options.max_exact_work,
        options.max_verification_cells,
    )?;
    let mut model = attachment.model;
    model.bodies.push(Body {
        outer_shell: 0,
        inner_shells: Vec::new(),
    });
    model.inherit_topology_ids(&[source]);
    let solid = qualify(model, options.admission)?
        .into_solid()
        .ok_or_else(|| {
            Error::new(
                "BREP_CLOSED_SOLID_UNPROVED",
                "Section caps did not establish an embedded outward material boundary",
            )
        })?;
    let volume = certified_freeform_volume(
        solid.model(),
        options.volume_tolerance_mm3,
        options.max_volume_cells,
    )?;
    Ok(SectionSolid {
        solid,
        cap_faces: attachment.cap_faces,
        volume,
        attachment_exact_work: attachment.exact_work,
        attachment_verification_cells: attachment.verification_cells,
    })
}
