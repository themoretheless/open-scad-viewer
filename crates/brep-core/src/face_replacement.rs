//! Replace one retained sector's surface while preserving its owned boundary.
//! Successful authoring is a candidate; global admission must be recomputed.
use crate::{Model, TopoId, TopologyLineageRecord};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    curve_surface_agreement::{self, Status},
    surface::Surface,
};
pub struct Replacement {
    pub model: Model,
    pub face: usize,
    pub source_identity: TopoId,
    pub face_identity: TopoId,
    pub cells: usize,
    pub exact_work: u64,
    pub trim_pairs: usize,
}
fn invalid(s: &str) -> Error {
    Error::new("BREP_FACE_REPLACEMENT_INPUT", s)
}
fn unproven(s: &str) -> Error {
    Error::new("BREP_FACE_REPLACEMENT_UNPROVEN", s)
}
/// Supply new pcurves in existing outer/hole and coedge traversal order. No
/// owned curve or endpoint is fitted, moved, duplicated or silently welded.
/// Every actual saved use must lift to its existing 3D edge over the whole
/// interval. The new chart's entire trim region must be valid and positive.
/// For material removal, partition source faces first and replace the selected
/// sector only after its contact/end boundaries have been authored.
pub fn replace(
    source: &Model,
    face_id: usize,
    surface: &Surface,
    pcurves: &[Vec<Curve>],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Replacement> {
    source.validate()?;
    surface.validate()?;
    if !source.persistent_naming_complete()
        || surface.periodic_u
        || surface.periodic_v
        || !(1..=1000000).contains(&max_cells)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(invalid(
            "Need complete identities, nonperiodic chart and bounded positive work",
        ));
    }
    let face = source
        .faces
        .get(face_id)
        .ok_or_else(|| invalid("Source face is missing"))?;
    let wires = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .collect::<Vec<_>>();
    if pcurves.len() != wires.len()
        || wires.len() > 16
        || wires
            .iter()
            .zip(pcurves)
            .any(|(&w, pc)| pc.len() != source.loops[w].coedges.len())
    {
        return Err(invalid(
            "One new pcurve is required for each existing owned use",
        ));
    }
    let uses = source
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .filter(|u| u.face == face_id)
        .count();
    if uses != 1 {
        return Err(invalid("Source face must have one oriented shell use"));
    }
    for &wire in &wires {
        let owners = source
            .faces
            .iter()
            .flat_map(|f| std::iter::once(f.outer).chain(f.holes.iter().copied()))
            .filter(|&w| w == wire)
            .count();
        if owners != 1 {
            return Err(invalid("Trim loops require unique face ownership"));
        }
    }
    for pc in pcurves.iter().flatten() {
        pc.validate()?;
        if pc.control_points[0].len() != 2 || pc.periodic {
            return Err(invalid(
                "Replacement pcurves require nonperiodic original-chart coordinates",
            ));
        }
    }
    if max_cells < 2 {
        return Err(unproven("Trim budget exhausted"));
    }
    let stage = (max_cells / 2).min(100000);
    let trim = nurbs_core::trim_region_audit::inspect(pcurves, tolerance_uv, 10000, stage, stage)?;
    let mut cells = trim.cells + trim.domain_cells;
    if trim.valid != Some(true) || trim.winding[0] != Some(1) {
        return Err(unproven(&format!(
            "Replacement trim region is invalid or unresolved: {}",
            trim.reason
        )));
    }
    let domain = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    // Positive rational hull enclosure is sufficient, without sampling or
    // clamping an out-of-chart trace back onto the replacement surface.
    if pcurves.iter().flatten().any(|c| {
        c.control_points
            .iter()
            .any(|p| (0..2).any(|k| p[k] < domain[k][0] || p[k] > domain[k][1]))
    }) {
        return Err(unproven("Replacement trace chart containment is unproven"));
    }
    let mut exact_work = 0;
    for (&wire, pcs) in wires.iter().zip(pcurves) {
        for (c, pc) in source.loops[wire].coedges.iter().zip(pcs) {
            let edge = &source.edges[c.edge];
            let mut proven = false;
            if exact_work < 1000000 {
                if let Some(r) = curve_surface_agreement::verify_exact(
                    &edge.curve,
                    pc,
                    surface,
                    c.reversed,
                    1000000 - exact_work,
                )? {
                    exact_work += r.work_used;
                    proven = r.outcome == cad_predicates::BezierIdentity::Equal;
                }
            }
            if !proven {
                if cells == max_cells {
                    return Err(unproven("Shared replacement lift budget exhausted"));
                }
                let r = curve_surface_agreement::verify(
                    &edge.curve,
                    pc,
                    surface,
                    c.reversed,
                    source.tolerance_mm,
                    (max_cells - cells).min(100000),
                )?;
                cells += r.cells;
                if r.status != Status::WithinTolerance {
                    return Err(unproven(
                        "Replacement surface does not match an owned boundary",
                    ));
                }
            }
        }
    }
    let mut model = source.clone();
    model.faces[face_id].surface = surface.clone();
    for (&wire, pcs) in wires.iter().zip(pcurves) {
        for (c, pc) in model.loops[wire].coedges.iter_mut().zip(pcs) {
            c.pcurve = pc.clone();
        }
    }
    model.inherit_topology_ids(&[source]);
    let source_identity = source.1.faces[face_id];
    let face_identity = model.1.faces[face_id];
    if face_identity != source_identity {
        model.1.lineage.push(TopologyLineageRecord {
            operation: "modify".into(),
            entity_kind: "face".into(),
            parents: vec![source_identity],
            children: vec![face_identity],
        });
    }
    model.refresh_change_set(&[source]);
    model.validate()?;
    if !model.persistent_naming_complete() {
        return Err(unproven("Replacement naming is incomplete"));
    }
    Ok(Replacement {
        model,
        face: face_id,
        source_identity,
        face_identity,
        cells,
        exact_work,
        trim_pairs: trim.pairs,
    })
}
