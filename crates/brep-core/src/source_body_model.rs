//! Transfer privately admitted full-source bodies without geometric welding.
use crate::source_boundary_fragment::{Endpoint, Fragment};
use std::collections::BTreeMap;
fn unsupported(message: &str) -> crate::Error {
    crate::Error::new("BREP_SOURCE_MODEL_UNSUPPORTED", message)
}
fn full(fragment: &Fragment) -> crate::Result<()> {
    let d = fragment.curve().domain();
    let expected = if fragment.reversed() { [d[1], d[0]] } else { d };
    if !(0..2).all(|i| matches!(fragment.endpoints()[i],Endpoint::Parameter(t) if t==expected[i])) {
        return Err(unsupported(
            "Root-valued and partial source trims need exact Model restriction storage",
        ));
    }
    Ok(())
}
fn directed(fragment: &Fragment) -> crate::Result<nurbs_core::curve::Curve> {
    full(fragment)?;
    let mut c = fragment.curve().clone();
    if fragment.reversed() {
        let d = c.domain();
        if c.control_points.len() != c.degree + 1
            || c.knots[..=c.degree].iter().any(|&k| k != d[0])
            || c.knots[c.control_points.len()..].iter().any(|&k| k != d[1])
        {
            return Err(unsupported(
                "Reverse multispan pcurves need exact source parameter transport",
            ));
        }
        // A clamped Bezier reflection only reverses coefficients: no knot arithmetic.
        c.control_points.reverse();
        c.weights.reverse();
    }
    Ok(c)
}
fn canonical_vertex_ids(
    shared: &crate::source_shared_edge::SharedEdge,
    slot: usize,
    mut ids: [usize; 2],
) -> [usize; 2] {
    if shared.reversed()[slot] {
        ids.reverse();
    }
    ids
}
/// Only a private native Body can authorize conversion. Entity addresses and
/// shell vertex identities are preserved; coordinates never merge vertices.
pub fn convert(
    body: &crate::source_volume::Body,
    tolerance_mm: f64,
) -> crate::Result<crate::Model> {
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(unsupported("Choose a positive finite Model tolerance"));
    }
    let shell = body.geometry().shell();
    let mut points = BTreeMap::new();
    let mut uses = BTreeMap::new();
    let mut edges = Vec::new();
    let insert = |points: &mut BTreeMap<usize, [f64; 3]>, id, point| -> crate::Result<()> {
        if points.get(&id).is_some_and(|p| *p != point) {
            return Err(unsupported(
                "One source vertex has conflicting exact world endpoints",
            ));
        }
        points.insert(id, point);
        Ok(())
    };
    for (index, shared) in shell.edges().iter().enumerate() {
        if !shared.covers_complete_canonical_source() {
            return Err(unsupported(
                "Partial canonical carriers need exact root restriction storage",
            ));
        }
        let c = shared.world();
        let d = c.domain();
        if c.knots[..=c.degree].iter().any(|&k| k != d[0])
            || c.knots[c.control_points.len()..].iter().any(|&k| k != d[1])
        {
            return Err(unsupported(
                "Canonical endpoints must be original clamped controls",
            ));
        }
        let mut canonical = None;
        for (slot, address) in shell.uses()[index].iter().enumerate() {
            let fragment = &shell.faces()[address.face][address.wire].edges()[address.edge];
            full(fragment)?;
            let reversed = shared.reversed()[slot];
            let ids = canonical_vertex_ids(
                shared,
                slot,
                shell.vertices()[address.face][address.wire][address.edge],
            );
            if canonical.is_some_and(|v| v != ids) {
                return Err(unsupported("Shared canonical endpoint ownership conflicts"));
            }
            canonical = Some(ids);
            uses.insert(
                (address.face, address.wire, address.edge),
                (index, reversed),
            );
        }
        let ids = canonical.unwrap();
        for end in 0..2 {
            let p = &c.control_points[if end == 0 {
                0
            } else {
                c.control_points.len() - 1
            }];
            insert(&mut points, ids[end], [p[0], p[1], p[2]])?;
        }
        edges.push(crate::Edge {
            vertices: ids,
            curve: c.clone(),
            degenerate: false,
        });
    }
    for (address, pole) in shell.poles() {
        let ids = shell.vertices()[address.face][address.wire][address.edge];
        if ids[0] != ids[1] {
            return Err(unsupported("Collapsed source use must own one vertex"));
        }
        insert(&mut points, ids[0], pole.point())?;
        let curve = nurbs_core::curve::Curve::from_polyline(vec![pole.point().to_vec(); 2])
            .map_err(|e| crate::Error::new(e.code, e.message))?;
        uses.insert(
            (address.face, address.wire, address.edge),
            (edges.len(), false),
        );
        edges.push(crate::Edge {
            vertices: ids,
            curve,
            degenerate: true,
        });
    }
    let ids = points
        .keys()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect::<BTreeMap<_, _>>();
    for edge in &mut edges {
        edge.vertices = edge.vertices.map(|id| ids[&id]);
    }
    let mut loops = Vec::new();
    let mut faces = Vec::new();
    for (face, wires) in shell.faces().iter().enumerate() {
        let first = loops.len();
        for (wire, source) in wires.iter().enumerate() {
            let mut coedges = Vec::new();
            for (edge, fragment) in source.edges().iter().enumerate() {
                let (index, reversed) = uses[&(face, wire, edge)];
                coedges.push(crate::Coedge {
                    edge: index,
                    reversed,
                    pcurve: directed(fragment)?,
                });
            }
            loops.push(crate::Loop { coedges });
        }
        faces.push(crate::Face {
            surface: wires[0].edges()[0].surface().clone(),
            outer: first,
            holes: (first + 1..loops.len()).collect(),
        });
    }
    let face_uses = (0..faces.len())
        .map(|face| crate::FaceUse {
            face,
            reversed: body.reverse_orientation()
                ^ (shell.regions().unwrap()[face].chart_winding() < 0),
        })
        .collect();
    let mut model = crate::Model(
        brep_topology::Model {
            vertices: points
                .values()
                .map(|&point| crate::Vertex { point })
                .collect(),
            edges,
            loops,
            faces,
            shells: vec![crate::Shell {
                faces: face_uses,
                closed: true,
            }],
            bodies: vec![crate::Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm,
        },
        Default::default(),
    );
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}

#[cfg(test)]
pub(crate) fn assert_step_identity(original: &crate::Model, imported: &crate::Model) {
    let original = crate::step_interchange_v3::canonical_face_senses(original).unwrap();
    let table = |ids: &[crate::TopoId]| {
        ids.iter()
            .copied()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect::<BTreeMap<_, _>>()
    };
    for (left, right) in [
        (&original.1.vertices, &imported.1.vertices),
        (&original.1.edges, &imported.1.edges),
        (&original.1.loops, &imported.1.loops),
        (&original.1.faces, &imported.1.faces),
        (&original.1.shells, &imported.1.shells),
        (&original.1.bodies, &imported.1.bodies),
    ] {
        assert_eq!(
            table(left).keys().collect::<Vec<_>>(),
            table(right).keys().collect::<Vec<_>>()
        );
    }
    let vertices = table(&imported.1.vertices);
    for (i, id) in original.1.vertices.iter().enumerate() {
        assert_eq!(original.vertices[i], imported.vertices[vertices[id]]);
    }
    let edges = table(&imported.1.edges);
    for (i, id) in original.1.edges.iter().enumerate() {
        let a = &original.edges[i];
        let b = &imported.edges[edges[id]];
        assert_eq!(a.curve, b.curve);
        assert_eq!(a.degenerate, b.degenerate);
        assert_eq!(
            a.vertices.map(|v| original.1.vertices[v]),
            b.vertices.map(|v| imported.1.vertices[v])
        );
    }
    let loops = table(&imported.1.loops);
    for (i, id) in original.1.loops.iter().enumerate() {
        let a = original.loops[i]
            .coedges
            .iter()
            .map(|c| (original.1.edges[c.edge], c.reversed, c.pcurve.clone()))
            .collect::<Vec<_>>();
        let b = imported.loops[loops[id]]
            .coedges
            .iter()
            .map(|c| (imported.1.edges[c.edge], c.reversed, c.pcurve.clone()))
            .collect::<Vec<_>>();
        assert_eq!(a.len(), b.len());
        assert!(
            (0..b.len()).any(|shift| (0..a.len()).all(|j| a[j] == b[(j + shift) % b.len()])),
            "STEP changed directed source loop"
        );
    }
    let faces = table(&imported.1.faces);
    for (i, id) in original.1.faces.iter().enumerate() {
        let a = &original.faces[i];
        let b = &imported.faces[faces[id]];
        assert_eq!(a.surface, b.surface);
        assert_eq!(original.1.loops[a.outer], imported.1.loops[b.outer]);
        assert_eq!(
            a.holes
                .iter()
                .map(|&j| original.1.loops[j])
                .collect::<Vec<_>>(),
            b.holes
                .iter()
                .map(|&j| imported.1.loops[j])
                .collect::<Vec<_>>()
        );
    }
    let shells = table(&imported.1.shells);
    for (i, id) in original.1.shells.iter().enumerate() {
        let a = &original.shells[i];
        let b = &imported.shells[shells[id]];
        assert_eq!(a.closed, b.closed);
        let mut aa = a
            .faces
            .iter()
            .map(|c| (original.1.faces[c.face], c.reversed))
            .collect::<Vec<_>>();
        let mut bb = b
            .faces
            .iter()
            .map(|c| (imported.1.faces[c.face], c.reversed))
            .collect::<Vec<_>>();
        aa.sort();
        bb.sort();
        assert_eq!(aa, bb);
    }
    let bodies = table(&imported.1.bodies);
    for (i, id) in original.1.bodies.iter().enumerate() {
        let a = &original.bodies[i];
        let b = &imported.bodies[bodies[id]];
        assert_eq!(
            original.1.shells[a.outer_shell],
            imported.1.shells[b.outer_shell]
        );
        assert_eq!(
            a.inner_shells
                .iter()
                .map(|&j| original.1.shells[j])
                .collect::<Vec<_>>(),
            b.inner_shells
                .iter()
                .map(|&j| imported.1.shells[j])
                .collect::<Vec<_>>()
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directed_fragment_reversal_is_counted_once_in_canonical_vertex_ownership() {
        let surface = nurbs_core::polynomial::graph([0., 1., 0., 1.], &[vec![0.]]).unwrap();
        let uv = nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let world =
            nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]])
                .unwrap();
        let forward = Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let backward = Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(1.),
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let edge = crate::source_shared_edge::qualify(
            &world,
            [&forward, &backward],
            [false, false],
            1000000,
        )
        .unwrap()
        .edge
        .unwrap();
        assert_eq!(
            canonical_vertex_ids(&edge, 0, [4, 5]),
            canonical_vertex_ids(&edge, 1, [5, 4])
        );
        let restored = crate::source_shared_edge_restore::restore(
            edge.definition(),
            crate::source_shared_edge_restore::Limits {
                mapping_cells_per_use: 100,
                exact_work: 1000000,
                driver_cells: 100,
            },
        )
        .unwrap()
        .edge
        .unwrap();
        assert_eq!(canonical_vertex_ids(&restored, 1, [5, 4]), [4, 5]);
    }
    #[test]
    fn partial_trims_refuse_and_bezier_reversal_preserves_original_coefficients() {
        let surface = nurbs_core::polynomial::graph([0., 1., 0., 1.], &[vec![0.]]).unwrap();
        let curve =
            nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let partial = Fragment::new(
            &surface,
            &curve,
            Endpoint::Parameter(0.25),
            Endpoint::Parameter(0.75),
        )
        .unwrap();
        assert_eq!(
            directed(&partial).unwrap_err().code,
            "BREP_SOURCE_MODEL_UNSUPPORTED"
        );
        let complete = Fragment::new(
            &surface,
            &curve,
            Endpoint::Parameter(1.),
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let reverse = directed(&complete).unwrap();
        assert_eq!(reverse.knots, curve.knots);
        assert_eq!(
            reverse.control_points,
            curve.control_points.into_iter().rev().collect::<Vec<_>>()
        );
    }
}
