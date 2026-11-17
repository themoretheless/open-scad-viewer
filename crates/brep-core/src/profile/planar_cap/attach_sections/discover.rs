use super::{AttachedSectionCaps, attach_section_caps, invalid};
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
    if !normal.iter().all(|value| value.is_finite()) || normal == [0.; 3] || !offset.is_finite() {
        return Err(invalid(
            "A finite section plane with nonzero normal is required",
        ));
    }
    if !(1..=1_000_000).contains(&max_exact_work) {
        return Err(invalid("Section work budget must be 1..1000000"));
    }
    source.validate()?;
    let original_shell = source
        .shells
        .get(shell)
        .filter(|shell| !shell.closed)
        .ok_or_else(|| invalid("Section discovery requires an open shell"))?;
    let exact_normal = normal.map(|value| BigRational::from_float(value).unwrap());
    let exact_offset = BigRational::from_float(offset).unwrap();
    let mut remaining = max_exact_work;
    let mut incidence = vec![0_u8; source.edges.len()];
    for coedge in source.loops.iter().flat_map(|wire| &wire.coedges) {
        charge(&mut remaining, 1)?;
        incidence[coedge.edge] = incidence[coedge.edge].saturating_add(1);
    }

    let mut outgoing = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for face_use in &original_shell.faces {
        let face = &source.faces[face_use.face];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for (index, coedge) in source.loops[wire].coedges.iter().enumerate() {
                charge(&mut remaining, 1)?;
                if incidence[coedge.edge] != 1 {
                    continue;
                }
                let edge = &source.edges[coedge.edge];
                let mut coplanar = true;
                for point in &edge.curve.control_points {
                    charge(&mut remaining, 7)?;
                    let dot: BigRational = exact_normal
                        .iter()
                        .zip(point)
                        .map(|(n, x)| n * BigRational::from_float(*x).unwrap())
                        .sum();
                    if dot != exact_offset {
                        coplanar = false;
                        break;
                    }
                }
                if !coplanar {
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
            charge(&mut remaining, 1)?;
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
        return Err(invalid("No free boundary cycles lie in the section plane"));
    }
    let addresses: Vec<_> = rings.iter().map(Vec::as_slice).collect();
    let attachment = attach_section_caps(
        source,
        shell,
        &addresses,
        normal,
        offset,
        remaining,
        max_verification_cells,
    )?;
    Ok(DiscoveredSectionCaps {
        attachment,
        rings,
        discovery_work: max_exact_work - remaining,
    })
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
