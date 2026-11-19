//! Locate the connected wall between two selected hole loops by shell incidence.
use crate::{Error, Model, Result};
use std::collections::{BTreeMap, BTreeSet};

/// Each endpoint is (cap face index, hole slot on that face). Return the wall
/// faces in index order when the component meets the caps only at these rims.
/// This tests connectivity, independently of cap planarity or wall shape.
pub fn between_holes(model: &Model, shell: usize, ends: [(usize, usize); 2]) -> Result<Vec<usize>> {
    model.validate()?;
    let refuse = || {
        Error::new(
            "BREP_THROUGH_WALL_UNPROVED",
            "Expected one connected wall bounded by both selected cap holes",
        )
    };
    let shell = model
        .shells
        .get(shell)
        .filter(|s| s.closed)
        .ok_or_else(refuse)?;
    let caps = [ends[0].0, ends[1].0];
    if caps[0] == caps[1] {
        return Err(refuse());
    }
    let mut rims = BTreeSet::new();
    for (face, hole) in ends {
        if !shell.faces.iter().any(|f| f.face == face) {
            return Err(refuse());
        }
        let rim = model.faces[face].holes.get(hole).ok_or_else(refuse)?;
        for coedge in &model.loops[*rim].coedges {
            if model.edges[coedge.edge].degenerate || !rims.insert(coedge.edge) {
                return Err(refuse());
            }
        }
    }
    let mut incidence = BTreeMap::<usize, Vec<usize>>::new();
    for usage in &shell.faces {
        let face = &model.faces[usage.face];
        for wire in std::iter::once(&face.outer).chain(&face.holes) {
            for coedge in &model.loops[*wire].coedges {
                if !model.edges[coedge.edge].degenerate {
                    incidence.entry(coedge.edge).or_default().push(usage.face);
                }
            }
        }
    }
    let first = rims
        .first()
        .and_then(|e| incidence.get(e))
        .and_then(|faces| faces.iter().find(|f| !caps.contains(f)))
        .ok_or_else(refuse)?;
    let mut pending = vec![*first];
    let mut wall = BTreeSet::new();
    let mut boundary = BTreeSet::new();
    while let Some(face) = pending.pop() {
        if !wall.insert(face) {
            continue;
        }
        for edge in model.face_edges(face) {
            if model.edges[edge].degenerate {
                continue;
            }
            let Some([a, b]) = incidence.get(&edge).map(Vec::as_slice) else {
                return Err(refuse());
            };
            let other = if *a == face { *b } else { *a };
            if caps.contains(&other) {
                boundary.insert(edge);
            } else if !wall.contains(&other) {
                pending.push(other);
            }
        }
    }
    if boundary != rims {
        return Err(refuse());
    }
    Ok(wall.into_iter().collect())
}
