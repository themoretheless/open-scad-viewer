use super::{
    AttachedSectionCaps, BoundedSectionAttachment, SectionAttachmentOptions, Support, attach,
    attach_section_caps_with_support_tolerance, invalid,
};
use crate::Model;
use num_rational::BigRational;
use nurbs_core::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct DiscoveredSectionCaps {
    pub attachment: AttachedSectionCaps,
    /// Source coedge addresses in shell traversal order, one list per ring.
    pub rings: Vec<Vec<[usize; 2]>>,
    /// Discovery work. Add the attachment's exact work for the total.
    pub discovery_work: u64,
}

/// Attach caps to the free cycles of one open shell in the plane n · x = offset.
/// A candidate must have all its spatial control points on that exact plane.
/// Vertices are matched by identity. A branch or an incomplete cycle is an error.
/// The exact work budget is shared with cap construction.
pub fn discover_and_attach_section_caps(
    source: &Model,
    shell: usize,
    normal: [f64; 3],
    offset: f64,
    max_exact_work: u64,
    max_verification_cells: usize,
) -> Result<DiscoveredSectionCaps> {
    discover_exact(
        source,
        shell,
        SectionPlane { normal, offset },
        max_exact_work,
        max_verification_cells,
        Support::Tight,
    )
}

/// Find all free cycles exactly on a specified plane and cap them using
/// expandable exact supports. Any free rim in another plane remains open.
pub fn discover_and_attach_section_caps_with_expanded_exact_support(
    source: &Model,
    shell: usize,
    plane: SectionPlane,
    max_exact_work: u64,
    max_verification_cells: usize,
) -> Result<DiscoveredSectionCaps> {
    discover_exact(
        source,
        shell,
        plane,
        max_exact_work,
        max_verification_cells,
        Support::ExpandedExact,
    )
}

fn discover_exact(
    source: &Model,
    shell: usize,
    plane: SectionPlane,
    max_exact_work: u64,
    max_verification_cells: usize,
    support: Support,
) -> Result<DiscoveredSectionCaps> {
    let mut remaining = max_exact_work;
    let rings = discover_rings(source, shell, plane, 0., &mut remaining)?;
    let addresses: Vec<_> = rings.iter().map(Vec::as_slice).collect();
    let attachment = attach(
        source,
        shell,
        &addresses,
        plane.normal,
        plane.offset,
        remaining,
        max_verification_cells,
        support,
    )?
    .attachment;
    Ok(DiscoveredSectionCaps {
        attachment,
        rings,
        discovery_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug)]
pub struct DiscoveredBoundedSectionCaps {
    pub bounded: BoundedSectionAttachment,
    /// Source coedge addresses in shell traversal order, one list per ring.
    pub rings: Vec<Vec<[usize; 2]>>,
    pub discovery_work: u64,
}

/// Discover complete free cycles inside a slab about the requested plane, then
/// attach certified planar caps within the independent support tolerance.
/// Positive rational weights keep each admitted curve inside the slab because
/// every spatial control point is within the specified perpendicular distance.
pub fn discover_and_attach_section_caps_with_support_tolerance(
    source: &Model,
    shell: usize,
    plane: SectionPlane,
    selection_tolerance_mm: f64,
    options: SectionAttachmentOptions,
) -> Result<DiscoveredBoundedSectionCaps> {
    let mut remaining = options.max_exact_work;
    let rings = discover_rings(source, shell, plane, selection_tolerance_mm, &mut remaining)?;
    let addresses: Vec<_> = rings.iter().map(Vec::as_slice).collect();
    let bounded = attach_section_caps_with_support_tolerance(
        source,
        shell,
        &addresses,
        plane.normal,
        plane.offset,
        SectionAttachmentOptions {
            max_exact_work: remaining,
            ..options
        },
    )?;
    Ok(DiscoveredBoundedSectionCaps {
        bounded,
        rings,
        discovery_work: options.max_exact_work - remaining,
    })
}

fn discover_rings(
    source: &Model,
    shell: usize,
    plane: SectionPlane,
    selection_tolerance_mm: f64,
    remaining: &mut u64,
) -> Result<Vec<Vec<[usize; 2]>>> {
    let SectionPlane { normal, offset } = plane;
    if !selection_tolerance_mm.is_finite()
        || !(0. ..=source.tolerance_mm).contains(&selection_tolerance_mm)
    {
        return Err(invalid(
            "Section selection tolerance must be within model tolerance",
        ));
    }
    if !normal.iter().all(|value| value.is_finite()) || normal == [0.; 3] || !offset.is_finite() {
        return Err(invalid(
            "A finite section plane with nonzero normal is required",
        ));
    }
    let exact_normal = normal.map(|value| BigRational::from_float(value).unwrap());
    let exact_offset = BigRational::from_float(offset).unwrap();
    let distance = BigRational::from_float(selection_tolerance_mm).unwrap();
    let normal_squared: BigRational = exact_normal.iter().map(|n| n * n).sum();
    let residual_squared = &distance * &distance * normal_squared;
    collect_free_rings(source, shell, remaining, |curve, remaining| {
        for point in &curve.control_points {
            charge(remaining, 8)?;
            let dot: BigRational = exact_normal
                .iter()
                .zip(point)
                .map(|(n, x)| n * BigRational::from_float(*x).unwrap())
                .sum();
            let residual = dot - &exact_offset;
            if &residual * &residual > residual_squared {
                return Ok(false);
            }
        }
        Ok(true)
    })
}

fn collect_free_rings(
    source: &Model,
    shell: usize,
    remaining: &mut u64,
    mut select: impl FnMut(&nurbs_core::curve::Curve, &mut u64) -> Result<bool>,
) -> Result<Vec<Vec<[usize; 2]>>> {
    if !(1..=1_000_000).contains(remaining) {
        return Err(invalid("Section work budget must be 1..1000000"));
    }
    source.validate()?;
    let original_shell = source
        .shells
        .get(shell)
        .filter(|shell| !shell.closed)
        .ok_or_else(|| invalid("Section discovery requires an open shell"))?;
    let mut incidence = vec![0_u8; source.edges.len()];
    for coedge in source.loops.iter().flat_map(|wire| &wire.coedges) {
        charge(remaining, 1)?;
        incidence[coedge.edge] = incidence[coedge.edge].saturating_add(1);
    }

    let mut outgoing = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for face_use in &original_shell.faces {
        let face = &source.faces[face_use.face];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for (index, coedge) in source.loops[wire].coedges.iter().enumerate() {
                charge(remaining, 1)?;
                if incidence[coedge.edge] != 1 {
                    continue;
                }
                let edge = &source.edges[coedge.edge];
                if !select(&edge.curve, remaining)? {
                    continue;
                }
                let reversed = coedge.reversed ^ face_use.reversed;
                let start = edge.vertices[usize::from(reversed)];
                let end = edge.vertices[usize::from(!reversed)];
                if outgoing.insert(start, (end, [wire, index])).is_some() || !incoming.insert(end) {
                    return Err(invalid("Section boundary branches at an existing vertex"));
                }
            }
        }
    }

    let mut rings = Vec::new();
    while let Some((&start, _)) = outgoing.first_key_value() {
        let mut ring = Vec::new();
        let mut current = start;
        loop {
            charge(remaining, 1)?;
            let (end, address) = outgoing
                .remove(&current)
                .ok_or_else(|| invalid("Section boundary has an open chain"))?;
            ring.push(address);
            if ring.len() > 256 {
                return Err(invalid("A section ring must have at most 256 uses"));
            }
            current = end;
            if current == start {
                break;
            }
        }
        rings.push(ring);
        if rings.len() > 256 {
            return Err(invalid("A section must have at most 256 rings"));
        }
    }
    if rings.is_empty() {
        return Err(invalid(
            "No free boundary cycles satisfy the section plane bound",
        ));
    }
    Ok(rings)
}

fn charge(remaining: &mut u64, work: u64) -> Result<()> {
    *remaining = remaining
        .checked_sub(work)
        .filter(|left| *left > 0)
        .ok_or_else(|| {
            Error::new(
                "BREP_RESOURCE_LIMIT",
                "Section discovery exhausted its work budget",
            )
        })?;
    Ok(())
}

/// A plane expressed as `normal · point = offset`.
#[derive(Clone, Copy, Debug)]
pub struct SectionPlane {
    pub normal: [f64; 3],
    pub offset: f64,
}

/// Close one shell using all supplied section planes in order.
/// The budgets cover the entire operation. Partial closure is an error.
/// Cap face indices in the result follow the input plane order.
pub fn close_shell_with_section_caps(
    source: &Model,
    shell: usize,
    planes: &[SectionPlane],
    max_exact_work: u64,
    max_verification_cells: usize,
) -> Result<AttachedSectionCaps> {
    close_exact(
        source,
        shell,
        planes,
        max_exact_work,
        max_verification_cells,
        Support::Tight,
    )
}

/// Close one shell with exact supports that may expand in UV to fit the
/// original rims. Plane order determines cap order. An incomplete closure
/// or an exhausted shared budget returns an error.
pub fn close_shell_with_expanded_exact_section_caps(
    source: &Model,
    shell: usize,
    planes: &[SectionPlane],
    max_exact_work: u64,
    max_verification_cells: usize,
) -> Result<AttachedSectionCaps> {
    close_exact(
        source,
        shell,
        planes,
        max_exact_work,
        max_verification_cells,
        Support::ExpandedExact,
    )
}

fn close_exact(
    source: &Model,
    shell: usize,
    planes: &[SectionPlane],
    max_exact_work: u64,
    max_verification_cells: usize,
    support: Support,
) -> Result<AttachedSectionCaps> {
    close_shell(
        source,
        shell,
        planes,
        max_exact_work,
        max_verification_cells,
        |current, plane, work, cells| {
            let report = discover_exact(current, shell, *plane, work, cells, support)?;
            Ok((report.attachment, report.discovery_work))
        },
    )
}

#[derive(Clone, Copy, Debug)]
pub struct BoundedSectionPlane {
    pub plane: SectionPlane,
    pub selection_tolerance_mm: f64,
    pub support_tolerance_mm: f64,
}

/// Close a shell with independently bounded section planes.
/// A zero tolerance requires exact selection or exact support, respectively.
/// The report lists the certified support errors in the same order as cap faces.
pub fn close_shell_with_bounded_section_caps(
    source: &Model,
    shell: usize,
    planes: &[BoundedSectionPlane],
    max_exact_work: u64,
    max_verification_cells: usize,
) -> Result<BoundedSectionAttachment> {
    let mut support_error_upper = Vec::new();
    let attachment = close_shell(
        source,
        shell,
        planes,
        max_exact_work,
        max_verification_cells,
        |current, plane, work, cells| {
            let report = discover_and_attach_section_caps_with_support_tolerance(
                current,
                shell,
                plane.plane,
                plane.selection_tolerance_mm,
                SectionAttachmentOptions {
                    max_exact_work: work,
                    max_verification_cells: cells,
                    support_tolerance_mm: plane.support_tolerance_mm,
                },
            )?;
            support_error_upper.extend(report.bounded.support_error_upper);
            Ok((report.bounded.attachment, report.discovery_work))
        },
    )?;
    Ok(BoundedSectionAttachment {
        attachment,
        support_error_upper,
    })
}

fn close_shell<P>(
    source: &Model,
    shell: usize,
    planes: &[P],
    max_exact_work: u64,
    max_verification_cells: usize,
    mut stage: impl FnMut(&Model, &P, u64, usize) -> Result<(AttachedSectionCaps, u64)>,
) -> Result<AttachedSectionCaps> {
    if !(1..=256).contains(&planes.len()) {
        return Err(invalid("Shell closure requires 1..256 section planes"));
    }

    let mut current = source;
    let mut staged = None;
    let mut cap_faces = Vec::new();
    let mut exact_work = 0;
    let mut verification_cells = 0;
    for plane in planes {
        if exact_work == max_exact_work || verification_cells == max_verification_cells {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "Shell closure exhausted the shared section budget",
            ));
        }
        let (attachment, discovery_work) = stage(
            current,
            plane,
            max_exact_work - exact_work,
            max_verification_cells - verification_cells,
        )?;
        exact_work += discovery_work + attachment.exact_work;
        verification_cells += attachment.verification_cells;
        cap_faces.extend(attachment.cap_faces);
        current = staged.insert(attachment.model);
    }
    if !current.shells[shell].closed {
        return Err(invalid(
            "Section planes leave free edges on the selected shell",
        ));
    }
    Ok(AttachedSectionCaps {
        model: staged.expect("nonempty section planes produced a staged model"),
        cap_faces,
        exact_work,
        verification_cells,
    })
}

mod infer;
pub(crate) use infer::close_inferred;
pub use infer::{
    InferredSectionPlane, InferredSectionPlanes, close_shell_with_bounded_inferred_section_caps,
    close_shell_with_inferred_section_caps, infer_section_planes,
};
