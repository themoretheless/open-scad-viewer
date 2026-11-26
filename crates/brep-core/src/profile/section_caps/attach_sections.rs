//! Attach section caps directly to an open shell's existing boundary edges.
use super::{
    SectionSheetOptions,
    section_sheets::{Support, build},
};
use crate::{Coedge, Face, FaceUse, Loop, Model};
use num_rational::BigRational;
use nurbs_core::{Error, Result, curve::Curve};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct AttachedSectionCaps {
    pub model: Model,
    pub cap_faces: Vec<usize>,
    pub exact_work: u64,
    pub verification_cells: usize,
}

#[derive(Clone, Copy)]
struct BoundaryUse {
    edge: usize,
    /// Physical orientation in the source shell.
    reversed: bool,
}

/// Close the selected section-boundary cycles with oriented B-rep cap faces.
/// Each address is [loop, coedge] in the immutable source model. The listed
/// uses must form cycles in shell traversal order. Existing edge and vertex
/// identities are retained, and the resulting shell is fully validated.
pub fn attach_section_caps(
    source: &Model,
    shell: usize,
    rings: &[&[[usize; 2]]],
    normal: [f64; 3],
    offset: f64,
    max_exact_work: u64,
    max_verification_cells: usize,
) -> Result<AttachedSectionCaps> {
    attach(
        source,
        shell,
        rings,
        normal,
        offset,
        max_exact_work,
        max_verification_cells,
        Support::Tight,
    )
    .map(|report| report.attachment)
}

/// Sew exact caps onto the selected free cycles, allowing their supports to
/// extend beyond the rim in UV. Other free cycles can remain open.
pub fn attach_section_caps_with_expanded_exact_support(
    source: &Model,
    shell: usize,
    rings: &[&[[usize; 2]]],
    normal: [f64; 3],
    offset: f64,
    max_exact_work: u64,
    max_verification_cells: usize,
) -> Result<AttachedSectionCaps> {
    attach(
        source,
        shell,
        rings,
        normal,
        offset,
        max_exact_work,
        max_verification_cells,
        Support::ExpandedExact,
    )
    .map(|report| report.attachment)
}

#[derive(Clone, Copy, Debug)]
pub struct SectionAttachmentOptions {
    pub max_exact_work: u64,
    pub max_verification_cells: usize,
    pub support_tolerance_mm: f64,
}

#[derive(Clone, Debug)]
pub struct BoundedSectionAttachment {
    pub attachment: AttachedSectionCaps,
    /// Continuous support error bounds in cap face order.
    pub support_error_upper: Vec<BigRational>,
}

/// Attach explicit section cycles using exactly planar stored supports within
/// the given support tolerance. Retained spatial edges keep their geometry;
/// agreement with each rounded face is checked at the source model tolerance.
pub fn attach_section_caps_with_support_tolerance(
    source: &Model,
    shell: usize,
    rings: &[&[[usize; 2]]],
    normal: [f64; 3],
    offset: f64,
    options: SectionAttachmentOptions,
) -> Result<BoundedSectionAttachment> {
    attach(
        source,
        shell,
        rings,
        normal,
        offset,
        options.max_exact_work,
        options.max_verification_cells,
        Support::Rounded(options.support_tolerance_mm),
    )
}

fn attach(
    source: &Model,
    shell: usize,
    rings: &[&[[usize; 2]]],
    normal: [f64; 3],
    offset: f64,
    max_exact_work: u64,
    max_verification_cells: usize,
    support: Support,
) -> Result<BoundedSectionAttachment> {
    source.validate()?;
    let original_shell = source
        .shells
        .get(shell)
        .ok_or_else(|| invalid("Unknown section shell"))?;
    if original_shell.closed || rings.is_empty() || rings.len() > 256 {
        return Err(invalid(
            "Section attachment needs an open shell and a bounded ring set",
        ));
    }
    let mut loop_sense = vec![None; source.loops.len()];
    for face_use in &original_shell.faces {
        let face = &source.faces[face_use.face];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            loop_sense[wire] = Some(face_use.reversed);
        }
    }
    let mut incidence = vec![0_u32; source.edges.len()];
    for coedge in source.loops.iter().flat_map(|wire| &wire.coedges) {
        incidence[coedge.edge] += 1;
    }
    let mut selected = BTreeSet::new();
    let mut uses = Vec::with_capacity(rings.len());
    let mut curves = Vec::with_capacity(rings.len());
    for ring in rings {
        if ring.is_empty() || ring.len() > 256 {
            return Err(invalid("A section ring must have 1..256 uses"));
        }
        let mut owned = Vec::with_capacity(ring.len());
        let mut oriented = Vec::with_capacity(ring.len());
        for &[wire, index] in *ring {
            let sense = loop_sense
                .get(wire)
                .copied()
                .flatten()
                .ok_or_else(|| invalid("Section use belongs to another shell"))?;
            let coedge = source.loops[wire]
                .coedges
                .get(index)
                .ok_or_else(|| invalid("Unknown section coedge"))?;
            if incidence[coedge.edge] != 1
                || !selected.insert(coedge.edge)
                || source.edges[coedge.edge].degenerate
                || coedge.trim().is_some()
            {
                return Err(invalid(
                    "Section use must own a distinct full boundary edge",
                ));
            }
            let reversed = coedge.reversed ^ sense;
            let curve = &source.edges[coedge.edge].curve;
            oriented.push(if reversed {
                curve.reverse()?
            } else {
                curve.clone()
            });
            owned.push(BoundaryUse {
                edge: coedge.edge,
                reversed,
            });
        }
        for i in 0..owned.len() {
            let a = owned[i];
            let b = owned[(i + 1) % owned.len()];
            if source.edges[a.edge].vertices[usize::from(!a.reversed)]
                != source.edges[b.edge].vertices[usize::from(b.reversed)]
            {
                return Err(invalid(
                    "Section ring must join existing vertex identities exactly",
                ));
            }
        }
        uses.push(owned);
        curves.push(oriented);
    }

    let rings: Vec<_> = curves.iter().map(Vec::<Curve>::as_slice).collect();
    let options = SectionSheetOptions {
        tolerance: source.tolerance_mm,
        max_exact_work,
        max_verification_cells,
    };
    let bounded = build(&rings, normal, offset, options, support)?;
    let report = bounded.sheet;
    let support_error_upper = bounded.support_error_upper;
    let mut model = source.clone();
    let mut cap_faces = Vec::new();
    for (face, region) in report.model.faces.iter().zip(report.face_rings) {
        let mut direction = None;
        let mut loop_ids = Vec::new();
        for (wire_id, ring) in std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .zip(region)
        {
            let wire = &report.model.loops[wire_id];
            let boundaries = &uses[ring];
            let mut coedges = Vec::with_capacity(boundaries.len());
            for (i, coedge) in wire.coedges.iter().enumerate() {
                let cap_flip = coedge.reversed;
                let j = if cap_flip {
                    boundaries.len() - 1 - i
                } else {
                    i
                };
                let boundary = boundaries[j];
                let face_reversed = !cap_flip;
                if direction.is_some_and(|old| old != face_reversed) {
                    return Err(invalid(
                        "Section rings require incompatible face orientations",
                    ));
                }
                direction = Some(face_reversed);
                coedges.push(Coedge::new(
                    boundary.edge,
                    boundary.reversed ^ cap_flip,
                    coedge.pcurve.clone(),
                ));
                incidence[boundary.edge] += 1;
            }
            loop_ids.push(model.loops.len());
            model.loops.push(Loop { coedges });
        }
        let index = model.faces.len();
        model.faces.push(Face {
            surface: face.surface.clone(),
            outer: loop_ids[0],
            holes: loop_ids[1..].to_vec(),
        });
        model.shells[shell].faces.push(FaceUse {
            face: index,
            reversed: direction.ok_or_else(|| invalid("Empty section cap"))?,
        });
        cap_faces.push(index);
    }
    model.shells[shell].closed = loop_sense
        .iter()
        .enumerate()
        .filter(|(_, sense)| sense.is_some())
        .flat_map(|(i, _)| &model.loops[i].coedges)
        .all(|use_| incidence[use_.edge] == 2);
    model.inherit_topology_ids(&[source]);
    model.validate()?;
    Ok(BoundedSectionAttachment {
        attachment: AttachedSectionCaps {
            model,
            cap_faces,
            exact_work: report.exact_work,
            verification_cells: report.verification_cells,
        },
        support_error_upper,
    })
}

pub(crate) fn invalid(message: &str) -> Error {
    Error::new("BREP_INVALID_SECTION_ATTACHMENT", message)
}

mod discover;
pub use discover::{
    BoundedSectionPlane, DiscoveredBoundedSectionCaps, DiscoveredSectionCaps, SectionPlane,
    close_shell_with_bounded_section_caps, close_shell_with_expanded_exact_section_caps,
    close_shell_with_section_caps, discover_and_attach_section_caps,
    discover_and_attach_section_caps_with_expanded_exact_support,
    discover_and_attach_section_caps_with_support_tolerance,
};

pub use discover::{
    InferredSectionPlane, InferredSectionPlanes, close_shell_with_bounded_inferred_section_caps,
    close_shell_with_inferred_section_caps, infer_section_planes,
};

pub(crate) use discover::close_inferred;
