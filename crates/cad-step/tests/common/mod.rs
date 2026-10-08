//! Shared assertions for the STEP identity tests.
#![allow(dead_code)]
use brep_core::face_senses::canonical_face_senses;
use std::collections::BTreeMap;

pub fn assert_step_identity(original: &brep_core::Model, imported: &brep_core::Model) {
    let original = canonical_face_senses(original).unwrap();
    let table = |ids: &[brep_core::TopoId]| {
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
