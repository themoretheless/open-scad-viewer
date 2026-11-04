use brep_core::{closed_solid as solid, mixed_root_model as mixed};
#[path = "../../nurbs-core/tests/support/ss_root_circle.rs"]
mod fixture;
#[path = "support/closed_solid_policy.rs"]
mod policy;
fn cube(lo: f64, hi: f64) -> solid::QualifiedClosedSolid {
    solid::qualify(
        brep_core::cuboid([lo; 3], [hi; 3]).unwrap(),
        policy::policy(),
    )
    .unwrap()
    .into_solid()
    .unwrap()
}
fn rooted() -> mixed::MixedBrep {
    let source = solid::qualify(
        brep_core::cuboid([0., 0., -1.], [1., 1., 0.]).unwrap(),
        policy::policy(),
    )
    .unwrap()
    .into_solid()
    .unwrap();
    mixed::split_closed_contour_candidate(
        &source,
        1,
        &fixture::wire(),
        0,
        [0.5; 2],
        Default::default(),
    )
    .unwrap()
    .model()
    .clone()
}
#[test]
fn root_edge_and_both_uv_bindings_transfer_together() {
    let left = mixed::MixedBrep::from_closed_solid(&cube(-2., 2.)).unwrap();
    let right = rooted();
    let joined = left.concatenate(&right).unwrap();
    let [a, b] = joined.sources();
    let t = joined.model().topology();
    let source = right.topology();
    assert_eq!(
        t.vertices.len(),
        left.topology().vertices.len() + source.vertices.len()
    );
    assert_eq!(t.bodies.len(), 2);
    for (i, e) in source.edges.iter().enumerate() {
        let child = &t.edges[b.edge(i).unwrap()];
        assert_eq!(child.vertices, e.vertices.map(|v| b.vertex(v).unwrap()));
        if let mixed::MixedEdge::Root(old) = &e.curve {
            let mixed::MixedEdge::Root(new) = &child.curve else {
                panic!("Root curve fitted")
            };
            assert!(new.curve().same_definition(old.curve()).unwrap());
            assert!(new.start().point().same_selector(old.start().point()));
        }
    }
    for (l, wire) in source.loops.iter().enumerate() {
        for (i, c) in wire.coedges.iter().enumerate() {
            let [loop_id, coedge_id] = b.coedge(l, i).unwrap();
            let child = &t.loops[loop_id].coedges[coedge_id];
            assert_eq!(child.edge, b.edge(c.edge).unwrap());
            assert_eq!(child.reversed, c.reversed);
            assert_eq!(child.trim(), c.trim());
            if let mixed::MixedPcurve::Root(p) = &child.pcurve {
                assert_eq!(p.world_edge, child.edge);
                let mixed::MixedPcurve::Root(old) = &c.pcurve else {
                    panic!()
                };
                assert_eq!(p.support, old.support);
            }
        }
    }
    assert_eq!(a.body(0), Some(0));
    assert_eq!(b.body(0), Some(1));
    assert_eq!(b.shell_orientation_reversed(0), Some(false));
    assert_eq!(b.face_use(0, 0), Some([left.topology().shells.len(), 0]));
    for invalid in [source.edges.len(), usize::MAX] {
        assert_eq!(b.edge(invalid), None);
    }
    assert_eq!(b.coedge(usize::MAX, 0), None);
    assert_eq!(b.coedge(0, usize::MAX), None);
    assert_eq!(b.face_use(usize::MAX, 0), None);
    assert_eq!(b.body(usize::MAX), None);
    assert_eq!(left.topology().bodies.len(), 1);
    assert_eq!(right.topology().bodies.len(), 1);
    let mut forged = t.clone();
    let loop_id = b.loop_index(source.faces[1].holes[0]).unwrap();
    let root = forged.loops[loop_id]
        .coedges
        .iter_mut()
        .find(|c| matches!(c.pcurve, mixed::MixedPcurve::Root(_)))
        .unwrap();
    if let mixed::MixedPcurve::Root(p) = &mut root.pcurve {
        p.world_edge = 0;
    }
    assert!(mixed::MixedBrep::from_topology(forged).is_err());
}
#[test]
fn cavity_shells_and_rational_subranges_keep_their_owners() {
    let a = mixed::MixedBrep::from_closed_solid(&cube(-2., 2.)).unwrap();
    let b = mixed::MixedBrep::from_closed_solid(&cube(-1., 1.)).unwrap();
    let (merged, maps) = a.concatenate(&b).unwrap().into_parts();
    let mut t = merged.topology().clone();
    let inner = maps[1].shell(0).unwrap();
    for f in &mut t.shells[inner].faces {
        f.reversed = !f.reversed;
    }
    t.bodies.pop();
    t.bodies[0].inner_shells.push(inner);
    // Storage-level trim/lift metadata must survive independently of geometry
    // admission; this transfer API deliberately grants no material owner.
    let trim = brep_topology::CoedgeTrim {
        curve_parameter: [0.25, 0.75],
        pcurve_parameter: [0.125, 0.875],
        periodic_lift: [[-1, 2], [3, -4]],
    };
    t.loops[0].coedges[0] = t.loops[0].coedges[0].clone().with_trim(trim).unwrap();
    let source = mixed::MixedBrep::from_topology(t).unwrap();
    let combined = source.concatenate(&source).unwrap();
    let [left, right] = combined.sources();
    let target = combined.model().topology();
    for map in [left, right] {
        let body = &target.bodies[map.body(0).unwrap()];
        assert_eq!(
            body.outer_shell,
            map.shell(source.topology().bodies[0].outer_shell).unwrap()
        );
        assert_eq!(body.inner_shells, vec![map.shell(inner).unwrap()]);
        let [l, c] = map.coedge(0, 0).unwrap();
        assert_eq!(target.loops[l].coedges[c].trim(), Some(trim));
    }
    assert_eq!(source.topology().bodies.len(), 1);
    assert_eq!(target.bodies.len(), 2);
}
#[test]
fn repeated_transfer_composes_indices_without_rebinding_root_selectors() {
    let source = rooted();
    let first = source.concatenate(&source).unwrap();
    let second = first.model().concatenate(&source).unwrap();
    let composed = first.sources()[1].then(second.sources()[0]).unwrap();
    assert!(composed.source_matches(&source));
    assert!(composed.target_matches(second.model()));
    assert!(composed.source_matches(&source.clone()));
    let foreign = mixed::MixedBrep::from_topology(first.model().topology().clone()).unwrap();
    let unrelated = foreign.concatenate(&source).unwrap();
    assert!(first.sources()[1].then(unrelated.sources()[0]).is_err());
    assert!(!first.sources()[1].target_matches(&foreign));
    for i in 0..source.topology().edges.len() {
        let middle = first.sources()[1].edge(i).unwrap();
        let child = second.sources()[0].edge(middle).unwrap();
        assert_eq!(composed.edge(i), Some(child));
        if let mixed::MixedEdge::Root(old) = &source.topology().edges[i].curve {
            let mixed::MixedEdge::Root(new) = &second.model().topology().edges[child].curve else {
                panic!()
            };
            assert!(new.start().point().same_selector(old.start().point()));
        }
    }
    assert_eq!(second.model().topology().bodies.len(), 3);
}
