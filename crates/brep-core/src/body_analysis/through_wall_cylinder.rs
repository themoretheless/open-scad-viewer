//! Rebuild a separated through-wall on an axis-aligned cylinder.
use crate::profile_certificate::original_restriction::plane;
use crate::{Error, Model, Result};
use nurbs_core::radial_bounds::piecewise_axis_radius_bounds;
use std::collections::BTreeSet;

/// Sew the cylindrical replacement into a single shell bounded by affine
/// horizontal caps at 0 and height. Returns a validated B-rep; does not
/// estimate the removed volume or establish global embedding of the input.
pub fn replace(source: &Model, caps: [usize; 2], radius: f64, height: f64) -> Result<Model> {
    let refuse = || {
        Error::new(
            "BREP_CYLINDRICAL_WALL_UNPROVED",
            "Unsupported cap, wall or radial separation",
        )
    };
    let old_wall = super::through_wall::between_holes(source, 0, caps.map(|cap| (cap, 0)))?;
    if source.bodies.len() != 1
        || source.shells.len() != 1
        || !source.bodies[0].inner_shells.is_empty()
        || !source.1.local_tolerances_mm.is_empty()
        || !source.1.sewed_edges.is_empty()
        || source
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .any(|c| c.trim().is_some())
    {
        return Err(refuse());
    }
    let cutter = crate::cylinder(radius, height)?;
    let mut removed_faces: BTreeSet<_> = old_wall.into_iter().collect();
    let mut removed_loops = BTreeSet::new();
    let mut new_loops = Vec::new();
    let mut cutter_caps = BTreeSet::new();
    for (end, cap) in caps.into_iter().enumerate() {
        let face = &source.faces[cap];
        let z = if end == 0 { 0. } else { height };
        let chart = plane::chart(&face.surface)?;
        if face.holes.len() != 1
            || face.surface.control_points[0][0][2] != z
            || chart.jacobian_sign() != 1
            || !source.shells[0]
                .faces
                .iter()
                .any(|f| f.face == cap && f.reversed == (end == 0))
        {
            return Err(refuse());
        }
        let (index, tool_cap) = cutter
            .faces
            .iter()
            .enumerate()
            .find(|(_, f)| f.surface.control_points.iter().flatten().all(|p| p[2] == z))
            .ok_or_else(refuse)?;
        cutter_caps.insert(index);
        let rim = plane::pullback::stored_affine_loop(
            &cutter,
            tool_cap.outer,
            &face.surface,
            true,
            source.tolerance_mm,
        )?;
        new_loops.push((cap, tool_cap.outer, rim));
        removed_loops.insert(face.holes[0]);
    }
    let radial = |f: &crate::Face| {
        piecewise_axis_radius_bounds(&f.surface, 2, [0.; 3], 256)?.ok_or_else(refuse)
    };
    let mut tool = [f64::INFINITY, 0_f64];
    for (i, face) in cutter.faces.iter().enumerate() {
        if !cutter_caps.contains(&i) {
            let r = radial(face)?;
            tool = [tool[0].min(r[0]), tool[1].max(r[1])];
        }
    }
    for (index, f) in source.faces.iter().enumerate() {
        if caps.contains(&index) {
            continue;
        }
        let r = radial(f)?;
        if removed_faces.contains(&index) {
            if r[1] >= tool[0] {
                return Err(refuse());
            }
            removed_loops.extend(std::iter::once(f.outer).chain(f.holes.iter().copied()));
        } else if r[0] <= tool[1] {
            return Err(refuse());
        }
    }
    let mut stage = crate::boolean_support::concatenate_bodies(source, &cutter)?;
    for (cap, l, mut wire) in new_loops {
        for c in &mut wire.coedges {
            c.edge += source.edges.len();
        }
        let l = l + source.loops.len();
        stage.loops[l] = wire;
        stage.faces[cap].holes[0] = l;
    }
    removed_faces.extend(cutter_caps.iter().map(|f| f + source.faces.len()));
    let sides: Vec<_> = stage.shells[1]
        .faces
        .iter()
        .cloned()
        .filter(|f| !removed_faces.contains(&f.face))
        .map(|mut f| {
            f.reversed = !f.reversed;
            f
        })
        .collect();
    stage.shells.truncate(1);
    stage.shells[0].faces.extend(sides);
    stage.bodies.truncate(1);
    let kept_edges: BTreeSet<_> = stage
        .loops
        .iter()
        .enumerate()
        .filter(|(i, _)| !removed_loops.contains(i))
        .flat_map(|(_, l)| l.coedges.iter().map(|c| c.edge))
        .collect();
    let kept_vertices: BTreeSet<_> = kept_edges
        .iter()
        .flat_map(|e| stage.edges[*e].vertices)
        .collect();
    let edges = (0..stage.edges.len())
        .filter(|e| !kept_edges.contains(e))
        .collect();
    let vertices = (0..stage.vertices.len())
        .filter(|v| !kept_vertices.contains(v))
        .collect();
    let mut result = crate::features::direct_edit::suppression::compact(
        &stage,
        &removed_faces,
        &removed_loops,
        &edges,
        &vertices,
    )?;
    result.inherit_topology_ids(&[source]);
    result.validate()?;
    Ok(result)
}
