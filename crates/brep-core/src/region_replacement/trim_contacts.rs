//! Atomic original-support trimming and replacement with authored common rims.
use super::{Replacement, refused};
use crate::{Coedge, Model, TopoId, face_cut};
use nurbs_core::{Result, curve::Curve, surface::Surface};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub enum RemovalAnchor {
    Edge(usize),
    Vertex(usize),
}
pub struct ContactCut<'a> {
    pub face: usize,
    /// Existing shared roots in the supplied model. Root isolation/splitting
    /// is separate; no nearest-vertex welding is performed here.
    pub endpoints: [usize; 2],
    pub segments: &'a [face_cut::Segment],
    /// Same direction/clock as each 3D segment, on the replacement surface.
    /// Cuts must be ordered as one positive replacement-chart boundary loop.
    pub replacement_pcurves: &'a [Curve],
    /// Authored topology determines which of the two sectors is removed.
    pub remove: RemovalAnchor,
    pub smooth: bool,
}
pub struct ContactReplacement {
    pub replacement: Replacement,
    /// Original input face identity -> final retained, trimmed descendant.
    pub retained_supports: Vec<(TopoId, usize)>,
    pub contact_edges: Vec<usize>,
    pub smooth_edges: Vec<usize>,
    pub cells: usize,
    pub exact_work: u64,
}
fn incident(model: &Model, face: usize, anchor: RemovalAnchor) -> bool {
    let f = &model.faces[face];
    std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|w| &model.loops[w].coedges)
        .any(|c| match anchor {
            RemovalAnchor::Edge(e) => c.edge == e,
            RemovalAnchor::Vertex(v) => model.edges[c.edge].vertices.contains(&v),
        })
}
/// Partition immutable original NURBS support charts, remove topology-owned
/// sectors and sew one replacement. Surface definitions of retained supports
/// are unchanged. Every cut and final edge/lift/trim/closure check is fresh.
/// All intermediate sectors stay private on refusal; one shared construction
/// cell/exact ledger spans every cut and final replacement.
///
/// This establishes C0 closure only. `smooth_edges` identifies requested G2
/// interfaces for independent qualification. Curvature, global embedding,
/// contact generation and automatic arbitrary-corner construction are separate.
pub fn trim_and_replace_contacts(
    source: &Model,
    cuts: &[ContactCut<'_>],
    surface: &Surface,
    reversed: bool,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<ContactReplacement> {
    source.validate()?;
    surface.validate()?;
    if cuts.len() < 3
        || cuts.len() > 16
        || max_cells < 8
        || max_cells > 100_000
        || max_exact > 1_000_000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(refused(
            "Need a bounded ordered contact loop and shared construction work",
        ));
    }
    let mut seen = BTreeSet::new();
    for c in cuts {
        if c.face >= source.faces.len()
            || !seen.insert(c.face)
            || c.segments.is_empty()
            || c.segments.len() != c.replacement_pcurves.len()
            || c.segments.len() > 256
            || c.endpoints.iter().any(|v| *v >= source.vertices.len())
            || !incident(source, c.face, c.remove)
            || matches!(c.remove,RemovalAnchor::Vertex(v) if c.endpoints.contains(&v))
        {
            return Err(refused(
                "Contact needs distinct original supports, bound endpoints and an unambiguous removed-side anchor",
            ));
        }
    }
    let mut model = source.clone();
    let mut cells = 0;
    let mut exact_work = 0;
    let mut selected = Vec::new();
    let mut retained = Vec::new();
    let mut ring = Vec::new();
    let mut smooth = Vec::new();
    for cut in cuts {
        if cells >= max_cells {
            return Err(refused("Contact construction work exhausted"));
        }
        let part = face_cut::partition_with_budget(
            &model,
            cut.face,
            cut.endpoints,
            cut.segments,
            tolerance_uv,
            max_cells - cells,
            max_exact - exact_work,
        )?;
        cells += part.cells;
        exact_work += part.exact_work;
        let matching = part
            .faces
            .iter()
            .copied()
            .filter(|f| incident(&part.model, *f, cut.remove))
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(refused(
                "Removed-side anchor is not unique in the actual cut sectors",
            ));
        }
        let removed = matching[0];
        let keep = *part.faces.iter().find(|f| **f != removed).unwrap();
        if part.model.faces[keep].surface != source.faces[cut.face].surface {
            return Err(refused("Retained original support definition changed"));
        }
        selected.push(removed);
        retained.push((source.1.faces[cut.face], cut.face, keep));
        for (&edge, pc) in part.edges.iter().zip(cut.replacement_pcurves) {
            ring.push(Coedge::new(edge, false, pc.clone()));
            if cut.smooth {
                smooth.push(edge);
            }
        }
        model = part.model;
    }
    if cells + 3 > max_cells {
        return Err(refused("No remaining closed replacement allowance"));
    }
    let mut replacement = super::replace(
        &model,
        &selected,
        surface,
        &[ring],
        reversed,
        tolerance_uv,
        max_cells - cells,
        max_exact - exact_work,
    )?;
    cells += replacement.cells;
    exact_work += replacement.exact_work;
    let mapped = |edge: usize| {
        replacement.source_edges[edge]
            .ok_or_else(|| refused("A contact edge did not survive sewing"))
    };
    // Final contacts are the sewn replacement rim, never proximity matches.
    let contact_edges = replacement.model.loops[replacement.model.faces[replacement.face].outer]
        .coedges
        .iter()
        .map(|c| c.edge)
        .collect();
    let smooth_edges = smooth.into_iter().map(mapped).collect::<Result<Vec<_>>>()?;
    let retained_supports = retained
        .iter()
        .map(|&(id, _, f)| {
            replacement.source_faces[f]
                .map(|f| (id, f))
                .ok_or_else(|| refused("Retained support was removed"))
        })
        .collect::<Result<Vec<_>>>()?;
    // Expose maps against the caller's immutable input, never the private
    // partitioned intermediate. Each cut has exactly one surviving descendant.
    let source_faces = (0..source.faces.len())
        .map(|original| {
            let kept = retained
                .iter()
                .find(|(_, f, _)| *f == original)
                .map(|(_, _, kept)| *kept)
                .unwrap_or(original);
            replacement.source_faces[kept]
        })
        .collect();
    replacement.source_faces = source_faces;
    replacement.source_edges.truncate(source.edges.len());
    replacement.source_vertices.truncate(source.vertices.len());
    replacement.model.refresh_change_set(&[source]);
    Ok(ContactReplacement {
        replacement,
        retained_supports,
        contact_edges,
        smooth_edges,
        cells,
        exact_work,
    })
}
