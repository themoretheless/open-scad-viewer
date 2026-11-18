//! Closed interior cuts retain both sectors. Authored curves never grant
//! intersection, material-side, global embedding or closed-solid authority.
use super::*;

/// Partition a face by a literal closed CCW path in its original UV chart.
/// Every 3D/UV segment, both orientations and the two complete trim regions
/// need fresh proofs. Existing holes must belong to exactly one sector.
pub fn partition_closed(
    source: &Model,
    face: usize,
    path: &[Segment],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Partition> {
    closed_with_budget(source, face, path, tolerance_uv, max_cells, 1000000)
}

pub(crate) fn closed_with_budget(
    source: &Model,
    face_id: usize,
    path: &[Segment],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Partition> {
    source.validate()?;
    if !source.persistent_naming_complete()
        || !(2..=256).contains(&path.len())
        || !(1..=1000000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(invalid(
            "Closed cut needs complete identities and bounded work",
        ));
    }
    let face = source
        .faces
        .get(face_id)
        .ok_or_else(|| invalid("Missing closed-cut face"))?;
    if face.surface.periodic_u || face.surface.periodic_v || face.holes.len() > 14 {
        return Err(invalid(
            "Closed cut needs a nonperiodic chart with at most fourteen holes",
        ));
    }
    let uses = source
        .shells
        .iter()
        .enumerate()
        .flat_map(|(s, shell)| {
            shell
                .faces
                .iter()
                .filter(move |u| u.face == face_id)
                .map(move |u| (s, u.reversed))
        })
        .collect::<Vec<_>>();
    if uses.len() != 1 {
        return Err(invalid("Closed-cut face needs one oriented shell use"));
    }
    for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        if source
            .faces
            .iter()
            .flat_map(|f| std::iter::once(f.outer).chain(f.holes.iter().copied()))
            .filter(|&w| w == wire)
            .count()
            != 1
        {
            return Err(invalid("Source trim loops must have unique face ownership"));
        }
    }
    for s in path {
        s.curve.validate()?;
        s.pcurve.validate()?;
        if s.curve.control_points[0].len() != 3
            || s.pcurve.control_points[0].len() != 2
            || !clamped(&s.curve)
            || !clamped(&s.pcurve)
            || s.curve.control_points[0] == *s.curve.control_points.last().unwrap()
        {
            return Err(invalid(
                "Closed cut requires clamped 3D/2D noncollapsed segments",
            ));
        }
    }
    for (i, s) in path.iter().enumerate() {
        let next = &path[(i + 1) % path.len()];
        if s.curve.control_points.last() != Some(&next.curve.control_points[0])
            || s.pcurve.control_points.last() != Some(&next.pcurve.control_points[0])
        {
            return Err(invalid(
                "Closed-cut endpoints must share literal world and UV coordinates",
            ));
        }
    }
    let edge_start = source.edges.len();
    let forward = path
        .iter()
        .enumerate()
        .map(|(i, s)| Coedge::new(edge_start + i, false, s.pcurve.clone()))
        .collect::<Vec<_>>();
    let backward = path
        .iter()
        .enumerate()
        .rev()
        .map(|(i, s)| Ok(Coedge::new(edge_start + i, true, s.pcurve.reverse()?)))
        .collect::<Result<Vec<_>>>()?;
    let curves = |r: &[Coedge]| r.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>();
    let mut cells = 0;
    let mut pairs = 0;
    let mut exact_work = 0;
    let original = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .map(|w| curves(&source.loops[w].coedges))
        .collect::<Vec<_>>();
    if audit(&original, tolerance_uv, &mut cells, &mut pairs, max_cells, 100000)? != Some(true) {
        return Err(unproven(
            "Original closed-cut trim region is not valid and positive",
        ));
    }
    // Side zero keeps the original outer boundary, with the new reverse ring
    // as a hole. Side one owns the positive interior ring.
    let mut regions = [
        vec![curves(&source.loops[face.outer].coedges), curves(&backward)],
        vec![curves(&forward)],
    ];
    for region in &regions {
        if audit(region, tolerance_uv, &mut cells, &mut pairs, max_cells, 100000)? != Some(true) {
            return Err(unproven(
                "Closed cut is not simple, positive and strictly inside the original outer trim",
            ));
        }
    }
    let mut holes = [Vec::new(), Vec::new()];
    for &hole in &face.holes {
        let h = curves(&source.loops[hole].coedges);
        let mut valid = [None; 2];
        for side in 0..2 {
            let mut trial = regions[side].clone();
            trial.push(h.clone());
            valid[side] = audit(&trial, tolerance_uv, &mut cells, &mut pairs, max_cells, 100000)?;
        }
        let side = match valid {
            [Some(true), Some(false)] => 0,
            [Some(false), Some(true)] => 1,
            _ => {
                return Err(unproven(
                    "Existing hole crosses the closed cut or has unresolved ownership",
                ));
            }
        };
        regions[side].push(h);
        holes[side].push(hole);
    }
    // Reversed knot arithmetic is checked against the actual saved 3D curve,
    // independently of the successful forward proof.
    for c in forward.iter().chain(&backward) {
        let segment = &path[c.edge - edge_start];
        let mut proven = false;
        if exact_work < max_exact {
            if let Some(r) = curve_surface_agreement::verify_exact(
                &segment.curve,
                &c.pcurve,
                &face.surface,
                c.reversed,
                max_exact - exact_work,
            )? {
                exact_work += r.work_used;
                proven = r.outcome == cad_predicates::BezierIdentity::Equal;
            }
        }
        if !proven {
            if cells >= max_cells {
                return Err(unproven("Closed-cut lift work exhausted"));
            }
            let r = curve_surface_agreement::verify(
                &segment.curve,
                &c.pcurve,
                &face.surface,
                c.reversed,
                source.tolerance_mm,
                (max_cells - cells).min(100000),
            )?;
            cells += r.cells;
            if r.status != Status::WithinTolerance {
                return Err(unproven(
                    "Actual closed-cut lift disagrees or remains unresolved",
                ));
            }
        }
    }
    let mut model = source.clone();
    let vertex_start = model.vertices.len();
    for s in path {
        model.vertices.push(Vertex {
            point: s.curve.control_points[0].clone().try_into().unwrap(),
        });
    }
    for (i, s) in path.iter().enumerate() {
        model.edges.push(Edge {
            curve: s.curve.clone(),
            vertices: [vertex_start + i, vertex_start + (i + 1) % path.len()],
            degenerate: false,
        });
    }
    let reverse_loop = model.loops.len();
    model.loops.push(Loop { coedges: backward });
    let inner_loop = model.loops.len();
    model.loops.push(Loop { coedges: forward });
    model.faces[face_id].holes = std::iter::once(reverse_loop)
        .chain(holes[0].iter().copied())
        .collect();
    let second = model.faces.len();
    model.faces.push(Face {
        surface: face.surface.clone(),
        outer: inner_loop,
        holes: holes[1].clone(),
    });
    model.shells[uses[0].0].faces.push(FaceUse {
        face: second,
        reversed: uses[0].1,
    });
    model.inherit_topology_ids(&[source]);
    let source_identity = source.1.faces[face_id];
    let face_identities = [model.1.faces[face_id], model.1.faces[second]];
    model.1.lineage.push(TopologyLineageRecord {
        operation: "split".into(),
        entity_kind: "face".into(),
        parents: vec![source_identity],
        children: face_identities.to_vec(),
    });
    model.refresh_change_set(&[source]);
    model.validate()?;
    if !model.persistent_naming_complete() {
        return Err(unproven("Closed-cut persistent naming is incomplete"));
    }
    Ok(Partition {
        model,
        faces: [face_id, second],
        edges: (edge_start..edge_start + path.len()).collect(),
        source_identity,
        face_identities,
        cells,
        exact_work,
        trim_pairs: pairs,
    })
}
