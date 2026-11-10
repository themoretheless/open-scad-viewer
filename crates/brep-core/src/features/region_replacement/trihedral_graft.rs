//! Candidate graft of three finite strips and their central trimmed graph.
//! No proximity welding and no inherited G2/solid receipt. The finite strips
//! have planar far stop caps; those stops are intentionally NOT smooth blends.
use super::{CentralGraph, CornerCollars, TrihedralStrips, build_trihedral_rim};
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};
fn refuse(s: &str) -> Error {
    Error::new("BREP_TRIHEDRAL_GRAFT_UNPROVEN", s)
}
fn start(m: &Model, c: &Coedge) -> usize {
    m.edges[c.edge].vertices[usize::from(c.reversed)]
}
fn end(m: &Model, c: &Coedge) -> usize {
    m.edges[c.edge].vertices[usize::from(!c.reversed)]
}
fn point(pc: &Curve, last: bool) -> Vec<f64> {
    if last {
        pc.control_points.last().unwrap().clone()
    } else {
        pc.control_points[0].clone()
    }
}
fn line_pc(a: Vec<f64>, b: Vec<f64>) -> Result<Curve> {
    Curve::from_polyline(vec![a, b])
}
fn canonical_use(m: &Model, face: usize, edge: usize) -> Result<Curve> {
    let f = &m.faces[face];
    let uses = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|l| m.loops[l].coedges.iter())
        .filter(|c| c.edge == edge)
        .collect::<Vec<_>>();
    if uses.len() != 1 {
        return Err(refuse("Expected one source face-local edge use"));
    }
    if uses[0].reversed {
        uses[0].pcurve.reverse()
    } else {
        Ok(uses[0].pcurve.clone())
    }
}
fn boundary_pc(
    collars: &CornerCollars,
    curve: &Curve,
    central: &CentralGraph,
    on_central: bool,
) -> Result<Curve> {
    let reverse = curve.reverse()?;
    let mut found = None;
    for (i, c) in collars.boundaries.iter().enumerate() {
        let pc = if on_central {
            &central.pcurves[i]
        } else {
            &c.original_pcurve
        };
        let next = if c.curve == *curve {
            Some(pc.clone())
        } else if c.curve == reverse {
            Some(pc.reverse()?)
        } else {
            None
        };
        if let Some(pc) = next {
            if found.is_some() {
                return Err(refuse("Ambiguous original rim definition"));
            }
            found = Some(pc);
        }
    }
    found.ok_or_else(|| refuse("Missing original rim curve definition"))
}
fn append_edge(m: &mut Model, vertices: [usize; 2], curve: Curve) -> usize {
    let index = m.edges.len();
    m.edges.push(Edge {
        vertices,
        curve,
        degenerate: false,
    });
    index
}
fn project_line(curve: &Curve, surface: &Surface, tolerance: f64) -> Result<Curve> {
    let result = nurbs_core::curve_pullback::project(
        curve,
        surface,
        nurbs_core::curve_pullback::Limits {
            tolerance,
            max_segments: 64,
            max_iterations: 32,
            agreement_cells: 10000,
        },
    )?;
    if !result.within_tolerance {
        return Err(refuse("Far cap line lift remains unresolved"));
    }
    result
        .pcurve
        .ok_or_else(|| refuse("Far cap line has no saved pcurve"))
}
fn oriented_loop(
    m: &Model,
    segments: &[(usize, Curve)],
    from: usize,
    to: usize,
) -> Result<Vec<Coedge>> {
    let mut remaining = (0..segments.len()).collect::<BTreeSet<_>>();
    let mut vertex = from;
    let mut out = Vec::new();
    while !remaining.is_empty() {
        let next = remaining
            .iter()
            .copied()
            .filter(|&i| m.edges[segments[i].0].vertices.contains(&vertex))
            .collect::<Vec<_>>();
        if next.len() != 1 {
            return Err(refuse("Graft boundary ownership is ambiguous"));
        }
        let i = next[0];
        remaining.remove(&i);
        let (edge, pc) = &segments[i];
        let reversed = m.edges[*edge].vertices[0] != vertex;
        out.push(Coedge::new(
            *edge,
            reversed,
            if reversed { pc.reverse()? } else { pc.clone() },
        ));
        vertex = m.edges[*edge].vertices[usize::from(!reversed)];
    }
    if vertex != to {
        return Err(refuse(
            "Graft boundary does not terminate at its original owned vertex",
        ));
    }
    Ok(out)
}
fn ccw(mut wire: Vec<Coedge>) -> Result<Vec<Coedge>> {
    let area = wire
        .iter()
        .map(|c| {
            let a = &c.pcurve.control_points[0];
            let b = c.pcurve.control_points.last().unwrap();
            a[0] * b[1] - a[1] * b[0]
        })
        .sum::<f64>();
    if !area.is_finite() || area == 0. {
        return Err(refuse("Graft face has no orientation proposal"));
    }
    if area < 0. {
        wire.reverse();
        for c in &mut wire {
            c.reversed = !c.reversed;
            c.pcurve = c.pcurve.reverse()?;
        }
    }
    Ok(wire)
}
fn append_face(m: &mut Model, surface: Surface, wire: Vec<Coedge>) -> Result<usize> {
    let outer = m.loops.len();
    m.loops.push(Loop {
        coedges: ccw(wire)?,
    });
    let face = m.faces.len();
    m.faces.push(Face {
        surface,
        outer,
        holes: Vec::new(),
    });
    Ok(face)
}
fn bind_new_uv_ports(m: &Model, retained: &[Coedge], added: &mut [Coedge]) -> Result<()> {
    let mut ports: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
    // Original retained uses own their UV endpoints. A different original
    // endpoint at the same topological vertex is not silently welded.
    for c in retained {
        for (vertex, last) in [(start(m, c), false), (end(m, c), true)] {
            let p = point(&c.pcurve, last);
            if ports.get(&vertex).is_some_and(|previous| *previous != p) {
                return Err(refuse("Original retained UV port ownership is ambiguous"));
            }
            ports.insert(vertex, p);
        }
    }
    // New uses share one authored UV port by vertex identity. This changes
    // only their rounded pcurves; final whole-interval lift verification is
    // mandatory below. World curves/vertices remain unchanged.
    for c in added {
        for (vertex, last) in [(start(m, c), false), (end(m, c), true)] {
            let index = if last {
                c.pcurve.control_points.len() - 1
            } else {
                0
            };
            let proposed = c.pcurve.control_points[index].clone();
            let owned = ports.entry(vertex).or_insert(proposed);
            c.pcurve.control_points[index] = owned.clone();
        }
    }
    Ok(())
}
/// Structural candidate only. A closed flag is a declaration, never a closure,
/// material, self-intersection, wall-thickness or G2 certificate. The caller
/// MUST freshly inspect trims, watertightness and global material validity.
pub struct TrihedralGraftCandidate {
    pub model: Model,
    pub central_face: usize,
    pub strip_faces: [usize; 3],
    pub far_stop_faces: [usize; 3],
    pub modified_source_faces: Vec<usize>,
    pub partition_cells: usize,
}
/// Cut the three original source edges at their independently proved far
/// cutter roots, retain their tails and original supports, and splice the
/// contact/connector chains into those supports. All shared endpoints derive
/// from explicit source/strip roles, never a distance tolerance.
pub fn propose_trihedral_graft(
    source: &Model,
    strips: &TrihedralStrips,
    collars: &CornerCollars,
    central: &CentralGraph,
    tolerance_uv: f64,
) -> Result<TrihedralGraftCandidate> {
    source.validate()?;
    central.surface.validate()?;
    if strips.strips.len() != 3
        || collars.boundaries.len() != 6
        || central.pcurves.len() != 6
        || source.faces.len() > 256
        || source.edges.len() > 1024
        || source.vertices.len() > 1024
        || source.shells.len() != 1
        || !source.shells[0].closed
        || source.bodies.len() != 1
        || !source.bodies[0].inner_shells.is_empty()
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || strips.plan.vertex >= source.vertices.len()
        || source.1.vertices[strips.plan.vertex] != strips.plan.identity
    {
        return Err(refuse(
            "Graft needs one owned closed shell and a three-strip corner",
        ));
    }
    for s in &strips.strips {
        if s.source_edge >= source.edges.len()
            || s.support_faces.iter().any(|&f| f >= source.faces.len())
            || s.cutter_faces
                .iter()
                .enumerate()
                .any(|(i, &f)| f >= s.cutters[i].faces.len())
            || !(s.end_order == [0, 1] || s.end_order == [1, 0])
            || s.patch.edges.iter().any(|&e| e >= 4)
        {
            return Err(refuse("Strip source, cutter or port selection is invalid"));
        }
    }
    let rim = build_trihedral_rim(strips, [source.tolerance_mm, 1e-5, 1e-4])?;
    let support_faces = rim
        .connectors
        .iter()
        .map(|c| c.source_face)
        .collect::<Vec<_>>();
    if support_faces.len() != 3
        || support_faces
            .iter()
            .any(|&f| !source.faces[f].holes.is_empty())
    {
        return Err(refuse(
            "Initial graft supports need three simply connected source faces",
        ));
    }
    let mut m = source.clone();
    let mut roots = [0; 3];
    let mut tails = [0; 3];
    let mut removed = BTreeSet::new();
    let mut partition_cells = 0;
    for (i, s) in strips.strips.iter().enumerate() {
        if source.1.edges.get(s.source_edge) != Some(&s.source_identity)
            || s.support_faces
                .iter()
                .zip(s.support_identities)
                .any(|(&f, id)| source.1.faces.get(f) != Some(&id))
        {
            return Err(refuse("Strip source identities changed before grafting"));
        }
        let result = crate::edge_partition::partition_at_face(
            &m,
            s.source_edge,
            &s.cutters[1],
            s.cutter_faces[1],
            0,
            tolerance_uv,
            100000,
        )?;
        partition_cells += result.result.partition.cells;
        let p = result.result.partition;
        roots[i] = p.vertex;
        let children = p.child_edges;
        let near = children
            .iter()
            .copied()
            .filter(|&e| p.model.edges[e].vertices.contains(&strips.plan.vertex))
            .collect::<Vec<_>>();
        if near.len() != 1 {
            return Err(refuse(
                "Root partition did not retain one explicit source-corner child",
            ));
        }
        removed.insert(near[0]);
        tails[i] = *children.iter().find(|&&e| e != near[0]).unwrap();
        m = p.model;
    }
    let partitioned = m.clone();
    let mut strip_faces = [0; 3];
    let mut mapped_edges = [[0; 4]; 3];
    let mut near_ports = [[0; 2]; 3];
    let mut far_ports = [[0; 2]; 3];
    for (i, s) in strips.strips.iter().enumerate() {
        let p = &s.patch.model;
        p.validate()?;
        if p.vertices.len() != 4 || p.edges.len() != 4 || p.faces.len() != 1 || p.loops.len() != 1 {
            return Err(refuse(
                "Graft expects one four-boundary original strip patch",
            ));
        }
        let vertex = m.vertices.len();
        m.vertices.extend(p.vertices.clone());
        for e in 0..4 {
            mapped_edges[i][e] = append_edge(
                &mut m,
                p.edges[e].vertices.map(|v| vertex + v),
                p.edges[e].curve.clone(),
            );
        }
        let near = usize::from(s.end_order[0] != 0);
        let far = 1 - near;
        near_ports[i] = p.edges[s.patch.edges[if near == 0 { 0 } else { 2 }]]
            .vertices
            .map(|v| vertex + v);
        far_ports[i] = p.edges[s.patch.edges[if far == 0 { 0 } else { 2 }]]
            .vertices
            .map(|v| vertex + v);
        let coedges = p.loops[p.faces[0].outer]
            .coedges
            .iter()
            .map(|c| Coedge::new(mapped_edges[i][c.edge], c.reversed, c.pcurve.clone()))
            .collect();
        strip_faces[i] = append_face(&mut m, p.faces[0].surface.clone(), coedges)?;
    }
    let mut per_face: BTreeMap<usize, Vec<(usize, Curve)>> = BTreeMap::new();
    let mut connector_edges = Vec::new();
    for connector in &rim.connectors {
        let vertices = connector.ports.map(|(strip, side)| near_ports[strip][side]);
        for k in 0..2 {
            let p = if k == 0 {
                connector.curve.control_points.first()
            } else {
                connector.curve.control_points.last()
            }
            .unwrap();
            if p.as_slice() != m.vertices[vertices[k]].point.as_slice() {
                return Err(refuse("Connector lost its literal source strip endpoint"));
            }
        }
        let pc = boundary_pc(collars, &connector.curve, central, false)?;
        let edge = append_edge(&mut m, vertices, connector.curve.clone());
        connector_edges.push(edge);
        per_face
            .entry(connector.source_face)
            .or_default()
            .push((edge, pc));
    }
    let mut far_stop_faces = [0; 3];
    for (i, s) in strips.strips.iter().enumerate() {
        let near = usize::from(s.end_order[0] != 0);
        let far = 1 - near;
        let surface = &s.cutters[1].faces[s.cutter_faces[1]].surface;
        let mut cap_lines = Vec::new();
        for side in 0..2 {
            let root_pc = canonical_use(&partitioned, s.support_faces[side], tails[i])?;
            let root_last = partitioned.edges[tails[i]].vertices[1] == roots[i];
            let source_pc = &s.contacts[side].support_pcurve;
            let curve = Curve::from_polyline(vec![
                m.vertices[roots[i]].point.to_vec(),
                m.vertices[far_ports[i][side]].point.to_vec(),
            ])?;
            let edge = append_edge(&mut m, [roots[i], far_ports[i][side]], curve);
            let cap_pc = project_line(&m.edges[edge].curve, surface, source.tolerance_mm / 2.)?;
            cap_lines.push((edge, cap_pc));
            per_face.entry(s.support_faces[side]).or_default().push((
                edge,
                line_pc(point(&root_pc, root_last), point(source_pc, far == 1))?,
            ));
            let contact = s.patch.edges[if side == 0 { 3 } else { 1 }];
            per_face
                .entry(s.support_faces[side])
                .or_default()
                .push((mapped_edges[i][contact], source_pc.clone()));
        }
        let arc = s.patch.edges[if far == 0 { 0 } else { 2 }];
        let segments = vec![
            (
                mapped_edges[i][arc],
                s.patch.ends[far].end_face_pcurve.clone(),
            ),
            cap_lines[0].clone(),
            cap_lines[1].clone(),
        ];
        // A closed triangular topological ring has two incident segments at
        // its initial vertex; seed the arc explicitly, then follow the lines.
        let mut wire = vec![Coedge::new(segments[0].0, false, segments[0].1.clone())];
        wire.extend(oriented_loop(
            &m,
            &segments[1..],
            far_ports[i][1],
            far_ports[i][0],
        )?);
        far_stop_faces[i] = append_face(&mut m, surface.clone(), wire)?;
    }
    for (&face, segments) in &per_face {
        let original = m.loops[m.faces[face].outer].coedges.clone();
        let mut kept = original
            .into_iter()
            .filter(|c| !removed.contains(&c.edge))
            .collect::<Vec<_>>();
        let ends = kept.iter().map(|c| end(&m, c)).collect::<BTreeSet<_>>();
        let first = kept
            .iter()
            .enumerate()
            .filter(|(_, c)| !ends.contains(&start(&m, c)))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if first.len() != 1 {
            return Err(refuse(
                "Retained support boundary is not one original open path",
            ));
        }
        kept.rotate_left(first[0]);
        if kept.windows(2).any(|p| end(&m, &p[0]) != start(&m, &p[1])) {
            return Err(refuse("Retained support path lost original incidence"));
        }
        let from = end(&m, kept.last().unwrap());
        let to = start(&m, &kept[0]);
        let mut added = oriented_loop(&m, segments, from, to)?;
        bind_new_uv_ports(&m, &kept, &mut added)?;
        kept.extend(added);
        let wire = m.faces[face].outer;
        m.loops[wire].coedges = kept;
    }
    let mut central_segments = Vec::new();
    for (i, s) in strips.strips.iter().enumerate() {
        let near = usize::from(s.end_order[0] != 0);
        let arc = s.patch.edges[if near == 0 { 0 } else { 2 }];
        let curve = &s.patch.ends[near].curve;
        central_segments.push((
            mapped_edges[i][arc],
            boundary_pc(collars, curve, central, true)?,
        ));
    }
    for (edge, c) in connector_edges.iter().zip(&rim.connectors) {
        central_segments.push((*edge, boundary_pc(collars, &c.curve, central, true)?));
    }
    let seed = central_segments.remove(0);
    let vertices = m.edges[seed.0].vertices;
    let mut wire = vec![Coedge::new(seed.0, false, seed.1)];
    wire.extend(oriented_loop(
        &m,
        &central_segments,
        vertices[1],
        vertices[0],
    )?);
    let central_face = append_face(&mut m, central.surface.clone(), wire)?;
    // Propagate shell orientation from unchanged original face uses across
    // every actual shared edge. A contradictory cycle is a refusal.
    let mut incidence: BTreeMap<usize, Vec<(usize, bool)>> = BTreeMap::new();
    for (f, face) in m.faces.iter().enumerate() {
        for l in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for c in &m.loops[l].coedges {
                incidence.entry(c.edge).or_default().push((f, c.reversed));
            }
        }
    }
    let mut orientation = vec![None; m.faces.len()];
    for u in &source.shells[0].faces {
        orientation[u.face] = Some(u.reversed);
    }
    for _ in 0..m.faces.len() {
        let mut changed = false;
        for uses in incidence.values() {
            if uses.len() != 2 {
                return Err(refuse(
                    "Every graft boundary edge needs exactly two face uses",
                ));
            }
            let [(a, ra), (b, rb)] = [uses[0], uses[1]];
            let parity = ra == rb;
            match (orientation[a], orientation[b]) {
                (Some(x), Some(y)) if x ^ y != parity => {
                    return Err(refuse("Graft shell has contradictory orientation"));
                }
                (Some(x), None) => {
                    orientation[b] = Some(x ^ parity);
                    changed = true;
                }
                (None, Some(y)) => {
                    orientation[a] = Some(y ^ parity);
                    changed = true;
                }
                _ => {}
            }
        }
        if !changed {
            break;
        }
    }
    if orientation.iter().any(Option::is_none) {
        return Err(refuse("Graft has a disconnected face component"));
    }
    m.shells[0].faces = orientation
        .iter()
        .enumerate()
        .map(|(face, r)| FaceUse {
            face,
            reversed: r.unwrap(),
        })
        .collect();
    let used = incidence.keys().copied().collect::<BTreeSet<_>>();
    let old_edges = m.edges.clone();
    let old_vertices = m.vertices.clone();
    let vertex_set = used
        .iter()
        .flat_map(|&e| old_edges[e].vertices)
        .collect::<BTreeSet<_>>();
    let vertices = vertex_set
        .iter()
        .enumerate()
        .map(|(i, &v)| (v, i))
        .collect::<BTreeMap<_, _>>();
    m.vertices = vertex_set
        .iter()
        .map(|&v| old_vertices[v].clone())
        .collect();
    let edges = used
        .iter()
        .enumerate()
        .map(|(i, &e)| (e, i))
        .collect::<BTreeMap<_, _>>();
    m.edges = used
        .iter()
        .map(|&e| {
            let mut edge = old_edges[e].clone();
            edge.vertices = edge.vertices.map(|v| vertices[&v]);
            edge
        })
        .collect();
    for c in m.loops.iter_mut().flat_map(|l| &mut l.coedges) {
        c.edge = edges[&c.edge];
    }
    m.inherit_topology_ids(&[&partitioned, source]);
    m.refresh_change_set(&[source]);
    m.validate_continuous(100000)?;
    Ok(TrihedralGraftCandidate {
        model: m,
        central_face,
        strip_faces,
        far_stop_faces,
        modified_source_faces: support_faces,
        partition_cells,
    })
}
