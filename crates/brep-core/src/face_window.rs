//! Retain the outside of an interior face window and sew an authored replacement.
//! No inferred trim branch, endpoint snapping or implicit solid admission.
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, TopoId, TopologyLineageRecord, Vertex};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    curve_surface_agreement::{self, Status},
    surface::Surface,
};
#[derive(Clone)]
pub struct Boundary {
    /// Shared curve follows the counterclockwise window traversal.
    pub curve: Curve,
    pub source_pcurve: Curve,
    pub replacement_pcurve: Curve,
}
pub struct Replacement {
    /// Candidate requiring fresh global admission after the geometry change.
    pub model: Model,
    pub retained_face: usize,
    pub replacement_face: usize,
    pub edges: Vec<usize>,
    pub source_face_identity: TopoId,
    pub face_identities: [TopoId; 2],
    pub cells: usize,
    pub exact_work: u64,
    pub trim_pairs: usize,
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_FACE_WINDOW_INPUT", message)
}
fn unproven(message: &str) -> Error {
    Error::new("BREP_FACE_WINDOW_UNPROVEN", message)
}
fn trim_audit(
    loops: &[Vec<Curve>],
    uv: f64,
    cells: &mut usize,
    pairs: &mut usize,
    max: usize,
) -> Result<()> {
    if max - *cells < 2 || *pairs == 10000 {
        return Err(unproven("Shared trim audit budget exhausted"));
    }
    let stage = ((max - *cells) / 2).min(100000);
    let proof = nurbs_core::trim_region_audit::inspect(loops, uv, 10000 - *pairs, stage, stage)?;
    *cells += proof.cells + proof.domain_cells;
    *pairs += proof.pairs;
    if proof.valid != Some(true) {
        return Err(unproven(&format!(
            "Window trim region invalid or unresolved: {} loops={:?}",
            proof.reason, proof.problem_loops
        )));
    }
    Ok(())
}
fn clamped(c: &Curve) -> bool {
    !c.periodic
        && c.knots[..=c.degree].iter().all(|&k| k == c.domain()[0])
        && c.knots[c.control_points.len()..]
            .iter()
            .all(|&k| k == c.domain()[1])
}
/// Replace one strictly interior, simple closed material window. Boundary curves
/// and both original pcurves are explicit authored inputs; all are independently
/// checked. The retained source surface is copied unchanged, with a clockwise
/// hole, and the replacement is added to the same oriented shell. Every new edge
/// has one owner on each side, one shared endpoint vertex and Split face lineage.
/// This is not the general open-cut/Boolean retained-sector classifier. Periodic
/// chart ownership and global embedding/G2/radius qualification are separate.
pub fn replace(
    source: &Model,
    face_id: usize,
    replacement: &Surface,
    boundary: &[Boundary],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Replacement> {
    replace_with_exact_limit(
        source, face_id, replacement, boundary, tolerance_uv, max_cells, 1000000,
    )
}
/// Window replacement with a caller-owned shared exact-predicate limit.
/// A zero exact limit still requires complete interval lift verification.
pub fn replace_with_exact_limit(
    source: &Model,
    face_id: usize,
    replacement: &Surface,
    boundary: &[Boundary],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Replacement> {
    source.validate()?;
    replacement.validate()?;
    if !source.persistent_naming_complete()
        || !(2..=256).contains(&boundary.len())
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(1..=1000000).contains(&max_cells)
        || max_exact > 1000000
    {
        return Err(invalid(
            "Window replacement needs complete identities, 2..256 boundaries and bounded proof work",
        ));
    }
    let face = source
        .faces
        .get(face_id)
        .ok_or_else(|| invalid("Source face is out of range"))?;
    let uses: Vec<_> = source
        .shells
        .iter()
        .enumerate()
        .flat_map(|(shell, s)| {
            s.faces
                .iter()
                .filter(move |u| u.face == face_id)
                .map(move |u| (shell, u.reversed))
        })
        .collect();
    if uses.len() != 1
        || face.holes.len() >= 15
        || face.surface.periodic_u
        || face.surface.periodic_v
        || replacement.periodic_u
        || replacement.periodic_v
    {
        return Err(invalid(
            "Window needs one source shell use, at most 14 existing holes and nonperiodic surface charts",
        ));
    }
    for b in boundary {
        b.curve.validate()?;
        b.source_pcurve.validate()?;
        b.replacement_pcurve.validate()?;
    }
    for (i, b) in boundary.iter().enumerate() {
        if b.curve.control_points[0].len() != 3
            || b.source_pcurve.control_points[0].len() != 2
            || b.replacement_pcurve.control_points[0].len() != 2
            || !clamped(&b.curve)
            || b.source_pcurve.periodic
            || b.replacement_pcurve.periodic
            || b.curve.control_points[0] == *b.curve.control_points.last().unwrap()
            || b.curve.control_points.last().unwrap()
                != &boundary[(i + 1) % boundary.len()].curve.control_points[0]
        {
            return Err(invalid(
                "Window boundaries require distinct, exactly shared clamped 3D endpoints and 2D pcurves",
            ));
        }
    }
    let hole_curves = boundary
        .iter()
        .rev()
        .map(|b| b.source_pcurve.reverse())
        .collect::<Result<Vec<_>>>()?;
    let mut retained: Vec<Vec<Curve>> = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .map(|w| {
            source.loops[w]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect();
    retained.push(hole_curves.clone());
    let target = boundary
        .iter()
        .map(|b| b.replacement_pcurve.clone())
        .collect::<Vec<_>>();
    let mut cells = 0;
    let mut exact_work = 0_u64;
    let mut pairs = 0;
    trim_audit(&retained, tolerance_uv, &mut cells, &mut pairs, max_cells)?;
    trim_audit(&[target], tolerance_uv, &mut cells, &mut pairs, max_cells)?;
    // Verify the actual saved reversed definitions, not an unrounded reversal.
    for (i, b) in boundary.iter().enumerate() {
        for (pc, surface, reverse) in [
            (&hole_curves[boundary.len() - 1 - i], &face.surface, true),
            (&b.replacement_pcurve, replacement, false),
        ] {
            if exact_work < max_exact {
                if let Some(exact) = curve_surface_agreement::verify_exact(
                    &b.curve,
                    pc,
                    surface,
                    reverse,
                    max_exact - exact_work,
                )? {
                    exact_work += exact.work_used;
                    if exact.outcome == cad_predicates::BezierIdentity::Equal {
                        continue;
                    }
                }
            }
            if cells == max_cells {
                return Err(unproven("Shared lift verification budget exhausted"));
            }
            let proof = curve_surface_agreement::verify(
                &b.curve,
                pc,
                surface,
                reverse,
                source.tolerance_mm,
                (max_cells - cells).min(100000),
            )?;
            cells += proof.cells;
            if proof.status != Status::WithinTolerance {
                return Err(unproven(&format!(
                    "Window boundary {i} lift is unproven or mismatching"
                )));
            }
        }
    }
    let mut model = source.clone();
    let vertex_start = model.vertices.len();
    let edge_start = model.edges.len();
    for (i, b) in boundary.iter().enumerate() {
        let p = &b.curve.control_points[0];
        model.vertices.push(Vertex {
            point: [p[0], p[1], p[2]],
        });
        model.edges.push(Edge {
            vertices: [vertex_start + i, vertex_start + (i + 1) % boundary.len()],
            curve: b.curve.clone(),
            degenerate: false,
        });
    }
    let hole = model.loops.len();
    model.loops.push(Loop {
        coedges: hole_curves
            .into_iter()
            .enumerate()
            .map(|(i, pcurve)| Coedge::new(
                edge_start + boundary.len() - 1 - i,
                true,
                pcurve,
            ))
            .collect(),
    });
    model.faces[face_id].holes.push(hole);
    let outer = model.loops.len();
    model.loops.push(Loop {
        coedges: boundary
            .iter()
            .enumerate()
            .map(|(i, b)| Coedge::new(
                edge_start + i,
                false,
                b.replacement_pcurve.clone(),
            ))
            .collect(),
    });
    let replacement_face = model.faces.len();
    model.faces.push(Face {
        surface: replacement.clone(),
        outer,
        holes: vec![],
    });
    model.shells[uses[0].0].faces.push(FaceUse {
        face: replacement_face,
        reversed: uses[0].1,
    });
    model.inherit_topology_ids(&[source]);
    let source_face_identity = source.1.faces[face_id];
    let face_identities = [model.1.faces[face_id], model.1.faces[replacement_face]];
    model.1.lineage.push(TopologyLineageRecord {
        operation: "split".into(),
        entity_kind: "face".into(),
        parents: vec![source_face_identity],
        children: face_identities.to_vec(),
    });
    model.refresh_change_set(&[source]);
    if !model.persistent_naming_complete() {
        return Err(unproven("Window replacement naming is incomplete"));
    }
    model.validate()?;
    Ok(Replacement {
        model,
        retained_face: face_id,
        replacement_face,
        edges: (edge_start..edge_start + boundary.len()).collect(),
        source_face_identity,
        face_identities,
        cells,
        exact_work,
        trim_pairs: pairs,
    })
}
