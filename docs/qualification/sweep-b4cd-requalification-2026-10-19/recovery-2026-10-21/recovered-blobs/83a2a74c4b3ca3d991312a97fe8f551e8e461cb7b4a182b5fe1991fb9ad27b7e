use brep_core::{Model, contact_bind, contact_end, fillet_contact};
use nurbs_core::{offset::fillet_surfaces::FilletOptions, surface::Surface};
fn plane(u: [f64; 3], v: [f64; 3]) -> Surface {
    let p = |a: f64, b: f64| (0..3).map(|k| u[k] * a + v[k] * b).collect::<Vec<_>>();
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![vec![p(0., 0.), p(0., 10.)], vec![p(10., 0.), p(10., 10.)]],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn cutter(y: f64) -> (Model, usize) {
    let m = brep_core::cuboid([-5., y, -5.], [15., y + 0.5, 15.]).unwrap();
    let f = m
        .faces
        .iter()
        .position(|f| f.surface.control_points.iter().flatten().all(|p| p[1] == y))
        .unwrap();
    (m, f)
}
fn proposals(
    source: &Model,
    contact: &fillet_contact::Contact,
    ys: [f64; 2],
    side: usize,
) -> [(usize, f64); 2] {
    let d = contact.curve.domain();
    let point = contact.curve.evaluate((d[0] + d[1]) * 0.5).unwrap().point;
    std::array::from_fn(|k| {
        let edge = source
            .edges
            .iter()
            .position(|e| {
                let a = source.vertices[e.vertices[0]].point;
                let b = source.vertices[e.vertices[1]].point;
                a[1] == ys[k]
                    && b[1] == ys[k]
                    && a[if side == 0 { 2 } else { 0 }] == 0.
                    && b[if side == 0 { 2 } else { 0 }] == 0.
                    && a[if side == 0 { 0 } else { 2 }] != b[if side == 0 { 0 } else { 2 }]
            })
            .unwrap();
        let axis = if side == 0 { 0 } else { 2 };
        let e = &source.edges[edge];
        let a = source.vertices[e.vertices[0]].point[axis];
        let b = source.vertices[e.vertices[1]].point[axis];
        let [lo, hi] = e.curve.domain();
        (edge, lo + (hi - lo) * (point[axis] - a) / (b - a))
    })
}
#[test]
fn recomputed_contact_roots_bind_to_shared_body_vertices_on_both_incident_faces() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let built = fillet_contact::build_g2(
        [&a, &b],
        1.5,
        &FilletOptions {
            max_sections: 5,
            ..Default::default()
        },
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    let contacts = built.contacts.as_ref().unwrap();
    let skin = built.fillet.surface.as_ref().unwrap();
    let d = contacts[0].curve.domain();
    let up = contacts[0].curve.evaluate(d[1]).unwrap().point[1]
        > contacts[0].curve.evaluate(d[0]).unwrap().point[1];
    let ys = if up { [2., 8.] } else { [8., 2.] };
    let (first, f) = cutter(ys[0]);
    let (last, l) = cutter(ys[1]);
    let body = brep_core::cuboid([0., 2., 0.], [10., 8., 10.]).unwrap();
    let before = body.clone();
    let p0 = proposals(&body, &contacts[0], ys, 0);
    assert!(
        contact_bind::automatically_between_faces(
            &body,
            &contacts[0],
            [&a, skin],
            [(&first, f, 0), (&last, l, 0)],
            1e-8,
            1,
            1000000
        )
        .is_err()
    );
    let bound0 = contact_bind::automatically_between_faces(
        &body,
        &contacts[0],
        [&a, skin],
        [(&first, f, 0), (&last, l, 0)],
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let bound1 = contact_bind::automatically_between_faces(
        &bound0.source,
        &contacts[1],
        [&b, skin],
        [(&first, f, 0), (&last, l, 0)],
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(bound1.source.vertices.len(), body.vertices.len() + 4);
    assert_eq!(bound1.source.edges.len(), body.edges.len() + 4);
    assert!(bound1.source.persistent_naming_complete());
    for (side, bound) in [&bound0, &bound1].into_iter().enumerate() {
        assert!(bound.cells <= 100000 && bound.exact_work <= 1000000);
        assert!(
            bound
                .restriction
                .root_position_error_upper
                .iter()
                .all(|&e| e <= body.tolerance_mm)
        );
        for k in 0..2 {
            let vertex = bound.vertices[k];
            assert_eq!(
                bound1.source.vertices[vertex].point,
                bound.restriction.endpoints[k]
            );
            let incident = bound1
                .source
                .faces
                .iter()
                .filter(|face| {
                    std::iter::once(face.outer)
                        .chain(face.holes.iter().copied())
                        .flat_map(|wire| &bound1.source.loops[wire].coedges)
                        .any(|c| bound1.source.edges[c.edge].vertices.contains(&vertex))
                })
                .count();
            assert_eq!(incident, 2);
        }
        let proof = nurbs_core::surface_g2_pcurves::inspect(
            &bound.restriction.contact.curve,
            [
                &bound.restriction.contact.support_pcurve,
                &bound.restriction.contact.fillet_pcurve,
            ],
            [if side == 0 { &a } else { &b }, skin],
            [false, false],
            1e-6,
            1e-5,
            1e-4,
            100000,
            1000000,
        )
        .unwrap();
        assert_eq!(
            proof.geometric.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
    }
    assert_eq!(
        brep_core::watertight::inspect_with_exact(&bound1.source, 100000, 1000000)
            .unwrap()
            .closure
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let patch = contact_end::patch(
        [&bound0.restriction, &bound1.restriction],
        skin,
        [(&first, f), (&last, l)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    for (local, source) in [
        (0, bound0.vertices[0]),
        (1, bound1.vertices[0]),
        (2, bound1.vertices[1]),
        (3, bound0.vertices[1]),
    ] {
        assert_eq!(
            patch.model.vertices[local].point,
            bound1.source.vertices[source].point
        );
    }
    let mut wrong = p0;
    wrong[0].1 = 0.5;
    assert!(
        contact_bind::between_faces(
            &body,
            &contacts[0],
            [&a, skin],
            [(&first, f, 0), (&last, l, 0)],
            wrong,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        contact_bind::between_faces(
            &body,
            &contacts[0],
            [&a, skin],
            [(&first, f, 0), (&last, l, 0)],
            p0,
            1e-8,
            1,
            1000000
        )
        .is_err()
    );
    assert_eq!(body, before);
}

#[test]
fn two_end_roots_on_one_original_edge_keep_all_three_children_and_atomic_failure() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let built = fillet_contact::build_g2(
        [&a, &b],
        1.5,
        &FilletOptions {
            max_sections: 5,
            ..Default::default()
        },
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    let contact = &built.contacts.as_ref().unwrap()[0];
    let skin = built.fillet.surface.as_ref().unwrap();
    let d = contact.curve.domain();
    let up = contact.curve.evaluate(d[1]).unwrap().point[1]
        > contact.curve.evaluate(d[0]).unwrap().point[1];
    let ys = if up { [2., 8.] } else { [8., 2.] };
    let (first, f) = cutter(ys[0]);
    let (last, l) = cutter(ys[1]);
    let source = brep_core::cuboid([1.5, 0., 0.], [10., 10., 10.]).unwrap();
    let before = source.clone();
    let edge = source
        .edges
        .iter()
        .position(|e| {
            let a = source.vertices[e.vertices[0]].point;
            let b = source.vertices[e.vertices[1]].point;
            a[0] == 1.5 && b[0] == 1.5 && a[2] == 0. && b[2] == 0. && a[1] != b[1]
        })
        .unwrap();
    let e = &source.edges[edge];
    let y0 = source.vertices[e.vertices[0]].point[1];
    let y1 = source.vertices[e.vertices[1]].point[1];
    let d = e.curve.domain();
    let parameters = ys.map(|y| (edge, d[0] + (d[1] - d[0]) * (y - y0) / (y1 - y0)));
    let out = contact_bind::between_faces(
        &source,
        contact,
        [&a, skin],
        [(&first, f, 0), (&last, l, 0)],
        parameters,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(out.source.edges.len(), source.edges.len() + 2);
    assert_eq!(out.source.vertices.len(), source.vertices.len() + 2);
    assert_eq!(out.source_edge_identities, [source.1.edges[edge]; 2]);
    assert_eq!(out.source_edge_parameters, parameters.map(|(_, t)| t));
    for k in 0..2 {
        assert!((out.source.vertices[out.vertices[k]].point[1] - ys[k]).abs() < 1e-6);
    }
    assert_eq!(
        brep_core::watertight::inspect_with_exact(&out.source, 100000, 1000000)
            .unwrap()
            .closure
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let mut invalid = parameters;
    invalid[1] = invalid[0];
    assert!(
        contact_bind::between_faces(
            &source,
            contact,
            [&a, skin],
            [(&first, f, 0), (&last, l, 0)],
            invalid,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    assert_eq!(source, before);
}

#[test]
fn automatic_binding_reuses_corner_vertices_and_handles_rational_edge_parameters() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let built = fillet_contact::build_g2(
        [&a, &b],
        1.5,
        &FilletOptions {
            max_sections: 5,
            ..Default::default()
        },
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    let contact = &built.contacts.as_ref().unwrap()[0];
    let skin = built.fillet.surface.as_ref().unwrap();
    let d = contact.curve.domain();
    let up = contact.curve.evaluate(d[1]).unwrap().point[1]
        > contact.curve.evaluate(d[0]).unwrap().point[1];
    let ys = if up { [2., 8.] } else { [8., 2.] };
    let (first, f) = cutter(ys[0]);
    let (last, l) = cutter(ys[1]);
    for (yrange, splits, rational) in [
        ([2., 8.], 0, false),
        ([2., 10.], 1, false),
        ([0., 10.], 2, true),
    ] {
        let mut source = brep_core::cuboid([1.5, yrange[0], 0.], [10., yrange[1], 10.]).unwrap();
        if rational {
            let edge = source
                .edges
                .iter()
                .position(|e| {
                    let p = source.vertices[e.vertices[0]].point;
                    let q = source.vertices[e.vertices[1]].point;
                    p[0] == 1.5 && q[0] == 1.5 && p[2] == 0. && q[2] == 0. && p[1] != q[1]
                })
                .unwrap();
            source.edges[edge].curve.weights = vec![1., 3.];
            source.edges[edge].curve.knots = vec![2., 2., 5., 5.];
            for c in source
                .loops
                .iter_mut()
                .flat_map(|l| &mut l.coedges)
                .filter(|c| c.edge == edge)
            {
                c.pcurve.weights = if c.reversed {
                    vec![3., 1.]
                } else {
                    vec![1., 3.]
                };
            }
            source.rebuild_topology_ids();
        }
        let before = source.clone();
        let out = contact_bind::automatically_between_faces(
            &source,
            contact,
            [&a, skin],
            [(&first, f, 0), (&last, l, 0)],
            1e-8,
            100000,
            1000000,
        )
        .unwrap();
        assert_eq!(source, before);
        assert_eq!(out.source.vertices.len(), source.vertices.len() + splits);
        assert_eq!(out.source.edges.len(), source.edges.len() + splits);
        assert_ne!(out.vertices[0], out.vertices[1]);
        assert!(out.cells <= 100000 && out.exact_work <= 1000000);
        for k in 0..2 {
            let p = out.source.vertices[out.vertices[k]].point;
            assert!((p[1] - ys[k]).abs() < source.tolerance_mm);
        }
        assert_eq!(
            brep_core::watertight::inspect_with_exact(&out.source, 100000, 1000000)
                .unwrap()
                .closure
                .status,
            brep_core::watertight::Closure::WithinTolerance
        );
        if splits == 0 {
            assert_eq!(out.source, source);
            let mut bad = source.clone();
            let vertex = out.vertices[0];
            let c = bad
                .loops
                .iter_mut()
                .flat_map(|l| &mut l.coedges)
                .find(|c| source.edges[c.edge].vertices.contains(&vertex))
                .unwrap();
            let uv_a = c.pcurve.control_points[0].clone();
            let uv_b = c.pcurve.control_points[1].clone();
            let axis = (0..2).find(|&j| uv_a[j] == uv_b[j]).unwrap();
            // A defect vanishing at the legacy nine diagnostic samples.
            // Whole-interval incident admission must still reject it.
            let mut power = vec![1.];
            for i in 0..=8 {
                let root = i as f64 / 8.;
                let mut next = vec![0.; power.len() + 1];
                for (j, &v) in power.iter().enumerate() {
                    next[j] -= root * v;
                    next[j + 1] += v;
                }
                power = next;
            }
            let mut square = vec![0.; 19];
            for (i, &x) in power.iter().enumerate() {
                for (j, &y) in power.iter().enumerate() {
                    square[i + j] += x * y;
                }
            }
            let choose =
                |n: usize, k: usize| (0..k).fold(1., |v, j| v * (n - j) as f64 / (j + 1) as f64);
            c.pcurve.degree = 18;
            c.pcurve.knots = [vec![0.; 19], vec![1.; 19]].concat();
            c.pcurve.control_points = (0..=18)
                .map(|i| {
                    let t = i as f64 / 18.;
                    let mut p = vec![
                        uv_a[0] * (1. - t) + uv_b[0] * t,
                        uv_a[1] * (1. - t) + uv_b[1] * t,
                    ];
                    let offset = (0..=i)
                        .map(|k| square[k] * choose(i, k) / choose(18, k))
                        .sum::<f64>()
                        * 1e6;
                    p[axis] += if uv_a[axis] == 0. { offset } else { -offset };
                    p
                })
                .collect();
            c.pcurve.weights = vec![1.; 19];
            bad.rebuild_topology_ids();
            bad.validate().unwrap();
            let saved = bad.clone();
            let err = contact_bind::automatically_between_faces(
                &bad,
                contact,
                [&a, skin],
                [(&first, f, 0), (&last, l, 0)],
                1e-8,
                100000,
                1000000,
            );
            assert!(err.is_err());
            assert_eq!(bad, saved);
        }
    }
}
