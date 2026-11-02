use brep_core::{
    Model,
    fillet_contact::{self, Contact, convex_branch as b},
    region_replacement::{self as r, EdgeRadius},
};
use nurbs_core::{curve::Curve, surface::Surface};
fn fixture() -> (Model, EdgeRadius, Surface, [Contact; 2]) {
    fixture_spans([[0.25, 0.5, 0.75]; 2])
}
fn fixture_spans(spans: [[f64; 3]; 2]) -> (Model, EdgeRadius, Surface, [Contact; 2]) {
    let source = r::six_edge_corner_stock().unwrap();
    let apex = source
        .vertices
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.point[2].total_cmp(&b.point[2]))
        .unwrap()
        .0;
    let edge = source
        .edges
        .iter()
        .position(|e| {
            e.vertices.contains(&apex)
                && e.vertices.iter().any(|&v| {
                    source.vertices[v].point[0] > 0.
                        && source.vertices[v].point[1] < 0.
                        && source.vertices[v].point[0] + source.vertices[v].point[1] == 0.
                })
        })
        .unwrap();
    let request = EdgeRadius {
        edge,
        radius_mm: 1.,
    };
    fixture_for_edge(source, request, spans, 1.)
}
fn fixture_for_edge(source: Model, request: EdgeRadius, spans: [[f64; 3]; 2], bulge: f64)
    -> (Model, EdgeRadius, Surface, [Contact; 2]) {
    let edge = request.edge;
    let plan = r::plan_radius_network(&source, 0, &[request], 10000).unwrap();
    let mut traces = Vec::new();
    let mut curves = Vec::new();
    for (side, &f) in plan.edges[0].supports.iter().enumerate() {
        let face = &source.faces[f];
        let co = source.loops[face.outer]
            .coedges
            .iter()
            .find(|c| c.edge == edge)
            .unwrap();
        let global = if co.reversed {
            co.pcurve.reverse().unwrap()
        } else {
            co.pcurve.clone()
        };
        let a = &global.control_points[0];
        let z = &global.control_points[1];
        let sign = if co.pcurve.control_points[1][0] > co.pcurve.control_points[0][0] {
            1.
        } else {
            -1.
        };
        let cp: Vec<_> = spans[side]
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                vec![
                    a[0] + t * (z[0] - a[0]),
                    a[1] + t * (z[1] - a[1]) + sign * bulge * [0.25, 0.5, 0.25][i],
                ]
            })
            .collect();
        let pc = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: cp,
            weights: vec![1.; 3],
            periodic: false,
        };
        let mut curve = pc.clone();
        curve.control_points = pc
            .control_points
            .iter()
            .map(|uv| face.surface.evaluate(uv[0], uv[1]).unwrap().point.to_vec())
            .collect();
        traces.push(pc);
        curves.push(curve);
    }
    let skin = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: curves.iter().map(|c| c.control_points.clone()).collect(),
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let contacts = std::array::from_fn(|i| {
        let pc = Curve::from_polyline(vec![vec![i as f64, 0.], vec![i as f64, 1.]]).unwrap();
        let positions = nurbs_core::surface_g2_pcurves::verify_positions(
            &curves[i],
            [&traces[i], &pc],
            [&source.faces[plan.edges[0].supports[i]].surface, &skin],
            [false; 2],
            1e-7,
            10000,
            1000000,
        )
        .unwrap();
        Contact {
            curve: curves[i].clone(),
            support_pcurve: traces[i].clone(),
            fillet_pcurve: pc,
            positions,
        }
    });
    (source, request, skin, contacts)
}
#[test]
fn partial_curved_branch_binds_to_oblique_edge_on_two_original_triangular_faces() {
    let (source, r, skin, c) = fixture();
    let before = source.clone();
    assert_eq!(
        fillet_contact::branch::inspect(
            &source,
            0,
            r,
            &skin,
            [&c[0], &c[1]],
            10000,
            10000,
            1000000
        )
        .unwrap()
        .decision,
        fillet_contact::branch::Decision::Unresolved
    );
    let proof = b::inspect(
        &source,
        0,
        r,
        &skin,
        [&c[0], &c[1]],
        10000,
        10000,
        1000000,
        100000,
    )
    .unwrap();
    assert_eq!(
        proof.decision,
        b::Decision::BoundPartial,
        "{}",
        proof.reason
    );
    assert_eq!(proof.contacts.len(), 2);
    assert_eq!(
        proof.common_source_clock.unwrap(),
        [[0.25, 0.25], [0.75, 0.75]]
    );
    assert!(proof.uv_work > 0 && proof.uv_work <= 100000);
    assert_eq!(source, before);
}
#[test]
fn opposite_carrier_stale_geometry_and_work_exhaustion_refuse_partial_ownership() {
    let (source, r, skin, mut c) = fixture();
    let exhausted = b::inspect(
        &source,
        0,
        r,
        &skin,
        [&c[0], &c[1]],
        10000,
        10000,
        1000000,
        0,
    )
    .unwrap();
    assert_eq!(exhausted.decision, b::Decision::Unresolved);
    let mut changed = skin.clone();
    for row in &mut changed.control_points {
        for p in row {
            p[2] += 1.;
        }
    }
    assert_eq!(
        b::inspect(
            &source,
            0,
            r,
            &changed,
            [&c[0], &c[1]],
            10000,
            10000,
            1000000,
            100000
        )
        .unwrap()
        .decision,
        b::Decision::Unresolved
    );
    let plan = r::plan_radius_network(&source, 0, &[r], 10000).unwrap();
    let face = &source.faces[plan.edges[0].supports[0]];
    let co = source.loops[face.outer]
        .coedges
        .iter()
        .find(|c| c.edge == r.edge)
        .unwrap();
    let y = if co.pcurve.control_points[1][0] > co.pcurve.control_points[0][0] {
        -0.125
    } else {
        -11.875
    };
    for p in &mut c[0].support_pcurve.control_points {
        p[1] = y;
    }
    let proof = b::inspect(
        &source,
        0,
        r,
        &skin,
        [&c[0], &c[1]],
        10000,
        10000,
        1000000,
        100000,
    )
    .unwrap();
    assert_eq!(proof.decision, b::Decision::Unresolved);
    assert_eq!(proof.reason, "Selected carrier dominance is not separated");
}

#[test]
fn elevated_multispan_affine_source_trim_keeps_exact_owned_clock() {
    let (mut source, request, skin, contacts) = fixture();
    for wire in &mut source.loops {
        for co in &mut wire.coedges {
            if co.pcurve.degree != 1 { continue; }
            co.pcurve = co.pcurve.elevate(2).unwrap();
            let d = co.pcurve.domain();
            co.pcurve = co.pcurve.insert(d[0] + (d[1] - d[0]) * 0.5, 1).unwrap();
        }
    }
    source.rebuild_topology_ids();
    let report = b::inspect(&source, 0, request, &skin, [&contacts[0], &contacts[1]],
        10000, 10000, 1000000, 100000).unwrap();
    assert_eq!(report.decision, b::Decision::BoundPartial, "{}", report.reason);
    assert_eq!(report.common_source_clock, Some([[0.25, 0.25], [0.75, 0.75]]));
}

fn nonlinear_fixture(weight: f64) -> (Model, EdgeRadius, Surface, [Contact; 2]) {
    nonlinear_from_parts(weight, fixture())
}
fn nonlinear_from_parts(weight: f64, parts: (Model, EdgeRadius, Surface, [Contact; 2]))
    -> (Model, EdgeRadius, Surface, [Contact; 2]) {
    let (mut source, request, skin, contacts) = parts;
    let edge = &mut source.edges[request.edge];
    edge.curve = Curve::from_polyline(vec![edge.curve.control_points[0].clone(),
        edge.curve.control_points.last().unwrap().clone()]).unwrap().elevate(2).unwrap();
    for k in 0..3 {
        edge.curve.control_points[1][k] = 0.75 * edge.curve.control_points[0][k]
            + 0.25 * edge.curve.control_points[2][k];
    }
    edge.curve.weights[1] = weight;
    for wire in &mut source.loops {
        for co in &mut wire.coedges {
            if co.edge != request.edge { continue; }
            co.pcurve = co.pcurve.elevate(2).unwrap();
            co.pcurve.weights[1] = weight;
            let t = if co.reversed { 0.75 } else { 0.25 };
            for k in 0..2 {
                co.pcurve.control_points[1][k] = (1. - t) * co.pcurve.control_points[0][k]
                    + t * co.pcurve.control_points[2][k];
            }
        }
    }
    source.rebuild_topology_ids();
    // The same geometric edge still lifts onto both original supports.
    assert!(fillet_contact::topology::inspect_source_edge(&source, 0, request,
        10000, 10000, 1000000).unwrap().proven);
    (source, request, skin, contacts)
}

#[test]
fn nonlinear_source_clocks_return_inverse_brackets_not_affine_guesses() {
    use num_rational::BigRational as Q;
    use num_traits::FromPrimitive;
    for (weight, scaled, refine) in [(1., false, false), (2., false, false),
        (1., true, false), (2., true, false), (1., false, true)] {
        let (mut source, request, skin, contacts) = nonlinear_fixture(weight);
        if refine {
            source.edges[request.edge].curve = source.edges[request.edge].curve.insert(0.5, 1).unwrap();
            for wire in &mut source.loops {
                for co in &mut wire.coedges {
                    if co.edge == request.edge { co.pcurve = co.pcurve.insert(0.5, 1).unwrap(); }
                }
            }
            source.rebuild_topology_ids();
        }
        if scaled {
            for k in &mut source.edges[request.edge].curve.knots { *k = -2. + 8. * *k; }
            for wire in &mut source.loops {
                for co in &mut wire.coedges {
                    if co.edge == request.edge {
                        for k in &mut co.pcurve.knots { *k = 8. + 4. * *k; }
                    }
                }
            }
            source.rebuild_topology_ids();
        }
        let before = source.clone();
        let report = b::inspect(&source, 0, request, &skin, [&contacts[0], &contacts[1]],
            10000, 10000, 1000000, 100000).unwrap();
        assert_eq!(report.decision, b::Decision::BoundPartial, "{}", report.reason);
        assert!(report.contacts.iter().all(|c| c.source_clock.as_ref().is_some_and(|r| r.proven)));
        let inverse = report.common_source_clock.unwrap();
        eprintln!("source_inverse weight={weight} scaled={scaled} refine={refine} brackets={inverse:?} cells={} uv_work={}", report.cells, report.uv_work);
        let q = |x| Q::from_f64(x).unwrap();
        let evaluate = |t: Q| {
            let one = q(1.);
            let mid = q(2. * weight) * &t * (&one - &t);
            (q(0.25) * &mid + &t * &t) /
                ((&one - &t) * (&one - &t) + mid + &t * &t)
        };
        let domain = source.edges[request.edge].curve.domain();
        let width = domain[1] - domain[0];
        for (bounds, target) in inverse.iter().zip([0.25, 0.75]) {
            let normalized = bounds.map(|t| (q(t) - q(domain[0])) / q(width));
            assert!(evaluate(normalized[0].clone()) <= q(target));
            assert!(evaluate(normalized[1].clone()) >= q(target));
            assert!((bounds[1] - bounds[0]) / width < 1e-13);
            // A stored midpoint/old affine correspondence would fail this.
            let guessed = domain[0] + width * target;
            assert!(guessed < bounds[0] || guessed > bounds[1]);
        }
        assert_eq!(source, before);
        let exhausted = b::inspect(&source, 0, request, &skin, [&contacts[0], &contacts[1]],
            10000, 10000, 1000000, report.uv_work - 1).unwrap();
        assert_eq!(exhausted.decision, b::Decision::Unresolved);
        assert!(exhausted.common_source_clock.is_none());
    }
}

#[test]
fn stationary_source_clock_never_inherits_inverse_admission() {
    let (mut source, request, skin, contacts) = nonlinear_fixture(1.);
    source.edges[request.edge].curve.control_points[1] =
        source.edges[request.edge].curve.control_points[0].clone();
    for wire in &mut source.loops {
        for co in &mut wire.coedges {
            if co.edge == request.edge {
                co.pcurve.control_points[1] = co.pcurve.control_points[if co.reversed { 2 } else { 0 }].clone();
            }
        }
    }
    source.rebuild_topology_ids();
    assert!(fillet_contact::topology::inspect_source_edge(&source, 0, request,
        10000, 10000, 1000000).unwrap().proven);
    let report = b::inspect(&source, 0, request, &skin, [&contacts[0], &contacts[1]],
        10000, 64, 1000000, 100000).unwrap();
    assert_eq!(report.decision, b::Decision::Unresolved);
    assert!(report.common_source_clock.is_none());
    assert!(report.cells <= 64 && report.uv_work <= 100000);
}

#[test]
fn source_parameter_rounding_cannot_publish_overlapping_endpoint_brackets() {
    let (mut source, request, skin, contacts) = nonlinear_fixture(1.);
    let lo = 1.0_f64;
    let hi = lo.next_up().next_up();
    for k in &mut source.edges[request.edge].curve.knots {
        *k = if *k == 0. { lo } else { hi };
    }
    for wire in &mut source.loops {
        for co in &mut wire.coedges {
            if co.edge == request.edge {
                for k in &mut co.pcurve.knots { *k = if *k == 0. { lo } else { hi }; }
            }
        }
    }
    source.rebuild_topology_ids();
    let report = b::inspect(&source, 0, request, &skin, [&contacts[0], &contacts[1]],
        10000, 10000, 1000000, 100000).unwrap();
    assert_eq!(report.decision, b::Decision::Unresolved);
    assert!(report.common_source_clock.is_none());
}

#[test]
fn unequal_neighbor_contact_extents_intersect_in_original_nonlinear_edge_clock() {
    use num_rational::BigRational as Q;
    use num_traits::FromPrimitive;
    let (source, request, skin, contacts) = nonlinear_from_parts(1.,
        fixture_spans([[0.25, 0.5, 0.75], [0.375, 0.5, 0.625]]));
    let report = b::inspect(&source, 0, request, &skin, [&contacts[0], &contacts[1]],
        10000, 10000, 1000000, 100000).unwrap();
    assert_eq!(report.decision, b::Decision::BoundPartial, "{}", report.reason);
    let common = report.common_source_clock.unwrap();
    let q = |x| Q::from_f64(x).unwrap();
    let evaluate = |t: Q| (t.clone() + &t * &t) / q(2.);
    for (bounds, target) in common.iter().zip([0.375, 0.625]) {
        assert!(evaluate(q(bounds[0])) <= q(target));
        assert!(evaluate(q(bounds[1])) >= q(target));
    }
    assert!(common[0][1] < common[1][0]);
    assert!(report.contacts[0].source_clock_interval[0][1] < common[0][0]);
    assert!(report.contacts[0].source_clock_interval[1][0] > common[1][1]);
}

#[test]
fn nonlinear_contact_roots_author_one_shared_middle_edge_on_both_original_faces() {
    use brep_core::edge_partition::contact_interval as interval;
    for weight in [1., 2.] {
    let (source, request, skin, contacts) = nonlinear_fixture(weight);
    let before = source.clone();
    let out = interval::between_contacts(&source, 0, request, &skin,
        [&contacts[0], &contacts[1]], "owned-contact-root-interval", 1e-8,
        Default::default()).unwrap();
    assert_eq!(source, before);
    assert_eq!(out.model().edges[out.middle_edge()].vertices, out.endpoints());
    assert_ne!(out.endpoint_identities()[0], out.endpoint_identities()[1]);
    eprintln!("contact_root_refinement bands={:?} root_error_mm={:?} cells={} endpoints={:?}",
        out.root_parameters(), out.root_error_upper(), out.cells(), out.endpoints());
    assert!(out.root_error_upper().iter().all(|e| e.is_finite() && *e <= source.tolerance_mm));
    let uses = out.model().faces.iter().filter(|face| {
        std::iter::once(face.outer).chain(face.holes.iter().copied()).any(|wire|
            out.model().loops[wire].coedges.iter().any(|co| co.edge == out.middle_edge()))
    }).count();
    assert_eq!(uses, 2);
    out.reverify().unwrap();
    }
}

#[test]
fn stale_contact_skin_and_exhausted_root_refinement_leave_source_unchanged() {
    use brep_core::edge_partition::contact_interval as interval;
    let (source, request, mut skin, mut contacts) = nonlinear_fixture(1.);
    let before = source.clone();
    let mut limits = interval::Limits::default();
    limits.branch_uv_work = 0;
    assert!(interval::between_contacts(&source, 0, request, &skin,
        [&contacts[0], &contacts[1]], "no-root-work", 1e-8, limits).is_err());
    for row in &mut skin.control_points {
        for point in row { point[2] += 1.; }
    }
    // A caller's stale/forged flag cannot qualify changed literal geometry.
    for c in &mut contacts { c.positions.proven = true; }
    assert!(interval::between_contacts(&source, 0, request, &skin,
        [&contacts[0], &contacts[1]], "stale-contact-root", 1e-8,
        Default::default()).is_err());
    assert_eq!(source, before);
}

fn contact_material_paths(
    source: &Model, request: EdgeRadius, contacts: &[Contact; 2],
    roots: &brep_core::edge_partition::contact_interval::ContactInterval,
) -> Vec<brep_core::face_cut::ContactFaceCut> {
    let plan = r::plan_radius_network(source, 0, &[request], 10000).unwrap();
    plan.edges[0].supports.iter().enumerate().map(|(side, &face)| {
        let wire = &source.loops[source.faces[face].outer].coedges;
        let keep = std::array::from_fn(|k| wire.iter()
            .map(|co| co.pcurve.control_points[0][k]).sum::<f64>() / wire.len() as f64);
        let mut endpoints = roots.endpoints().map(|v| roots.model().vertices[v].point.to_vec());
        if roots.branch().contacts[side].contact_reversed { endpoints.reverse(); }
        let contact = &contacts[side].curve;
        brep_core::face_cut::ContactFaceCut { face,
            curves: vec![
                Curve::from_polyline(vec![endpoints[0].clone(), contact.control_points[0].clone()]).unwrap(),
                contact.clone(),
                Curve::from_polyline(vec![contact.control_points.last().unwrap().clone(), endpoints[1].clone()]).unwrap(),
            ], keep_uv: vec![keep] }
    }).collect()
}

#[test]
fn root_bound_contacts_cut_both_supports_and_select_the_middle_edge_material_sectors() {
    use brep_core::face_cut;
    for weight in [1., 2.] {
        let (source, request, skin, contacts) = nonlinear_fixture(weight);
        let before = source.clone();
        let result = face_cut::plan_contact_retention(&source, 0, request, &skin,
            [&contacts[0], &contacts[1]], "root-contact-material", 1e-8, Default::default(),
            |roots| Ok(contact_material_paths(&source, request, &contacts, roots))).unwrap();
        assert_eq!(source, before);
        assert_eq!(result.plan().regions.len(), 2);
        assert_eq!(result.plan().model.faces.len(), source.faces.len() + 2);
        for region in &result.plan().regions {
            assert_eq!(region.discard_faces.len(), 1);
            assert_eq!(region.retained_faces.len(), 1);
            assert_eq!(region.cut_edges.len(), 3);
            assert!(region.descendants.iter().all(|&f|
                result.plan().model.faces[f].surface == source.faces[region.source_face].surface));
        }
        assert!(result.cells() <= 100_000);
        eprintln!("contact_retention weight={weight} faces={} cells={}",
            result.plan().model.faces.len(), result.cells());
        result.reverify().unwrap();
    }
}

#[test]
fn contact_material_plan_rejects_missing_actual_contact_and_wrong_retained_side() {
    use brep_core::face_cut;
    let (source, request, skin, contacts) = nonlinear_fixture(1.);
    let before = source.clone();
    let missing = face_cut::plan_contact_retention(&source, 0, request, &skin,
        [&contacts[0], &contacts[1]], "missing-actual-contact", 1e-8, Default::default(),
        |roots| {
            let mut paths = contact_material_paths(&source, request, &contacts, roots);
            paths[0].curves.remove(1);
            Ok(paths)
        });
    assert!(matches!(missing, Err(e) if e.message.contains("Actual complete contact curve")));
    let wrong_side = face_cut::plan_contact_retention(&source, 0, request, &skin,
        [&contacts[0], &contacts[1]], "wrong-contact-material", 1e-8, Default::default(),
        |roots| {
            let mut paths = contact_material_paths(&source, request, &contacts, roots);
            for (side, path) in paths.iter_mut().enumerate() {
                let model = roots.model();
                let co = model.loops[model.faces[path.face].outer].coedges.iter()
                    .find(|c| c.edge == roots.middle_edge()).unwrap();
                let d = co.pcurve.domain();
                let edge_uv = co.pcurve.evaluate(d[0] + (d[1] - d[0]) * 0.5).unwrap().point;
                let d = contacts[side].support_pcurve.domain();
                let contact_uv = contacts[side].support_pcurve.evaluate(d[0] + (d[1] - d[0]) * 0.5).unwrap().point;
                path.keep_uv = vec![std::array::from_fn(|k| 0.5 * (edge_uv[k] + contact_uv[k]))];
            }
            Ok(paths)
        });
    assert!(matches!(wrong_side, Err(e) if e.message.contains("discarded sector")));
    assert_eq!(source, before);
}

fn straight_stop_patch_set(retention: &brep_core::face_cut::ContactRetention, skin: &Surface) -> r::OwnedPatchSet {
    use brep_core::{Coedge, Edge};
    let plan = retention.plan();
    let model = &plan.model;
    let [a, b] = [&plan.regions[0].cut_edges, &plan.regions[1].cut_edges];
    let roots = retention.roots().endpoints();
    let endpoints = |edges: &[usize]| [model.edges[edges[0]].vertices[1], model.edges[edges[2]].vertices[0]];
    let [a0, a1] = endpoints(a);
    let [b0, b1] = endpoints(b);
    let base = model.edges.len();
    let extra = [(a0, b0), (a1, b1)].map(|(x, y)| Edge {
        vertices: [x, y], degenerate: false,
        curve: Curve::from_polyline(vec![model.vertices[x].point.to_vec(), model.vertices[y].point.to_vec()]).unwrap(),
    });
    let make = |surface: Surface, nodes: &[usize], uv: &[[f64; 2]], edges: &[usize]| {
        let loop_: Vec<_> = edges.iter().enumerate().map(|(i, &edge)| {
            let e = if edge < base { &model.edges[edge] } else { &extra[edge - base] };
            let pair = [nodes[i], nodes[(i + 1) % nodes.len()]];
            let reversed = e.vertices != pair;
            assert!(e.vertices == pair || e.vertices == [pair[1], pair[0]]);
            Coedge::new(edge, reversed, Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % uv.len()].to_vec()]).unwrap())
        }).collect();
        let mut orientation = None;
        for co in &loop_ {
            for region in &plan.regions {
                for &face in &region.discard_faces {
                    let f = &model.faces[face];
                    for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                        if let Some(prior) = model.loops[wire].coedges.iter().find(|c| c.edge == co.edge) {
                            let use_ = model.shells.iter().flat_map(|s| &s.faces).find(|u| u.face == face).unwrap();
                            let reversed = prior.reversed ^ use_.reversed ^ co.reversed;
                            if let Some(expected) = orientation { assert_eq!(expected, reversed); }
                            orientation = Some(reversed);
                        }
                    }
                }
            }
        }
        r::OwnedSewPatch { surface, loops: vec![loop_], reversed: orientation.unwrap() }
    };
    let plane = |root: usize, x: usize, y: usize| {
        let [o, a, b] = [root, x, y].map(|v| model.vertices[v].point);
        Surface { degree_u: 1, degree_v: 1, knots_u: vec![0.,0.,1.,1.], knots_v: vec![0.,0.,1.,1.],
            control_points: vec![vec![o.to_vec(), b.to_vec()],
                vec![a.to_vec(), (0..3).map(|k| a[k] + b[k] - o[k]).collect()]],
            weights: vec![vec![1.;2];2], periodic_u: false, periodic_v: false }
    };
    let patches = vec![
        make(skin.clone(), &[a0,b0,b1,a1], &[[0.,0.],[1.,0.],[1.,1.],[0.,1.]], &[base,b[1],base+1,a[1]]),
        make(plane(roots[0],a0,b0), &[roots[0],a0,b0], &[[0.,0.],[1.,0.],[0.,1.]], &[a[0],base,b[0]]),
        make(plane(roots[1],a1,b1), &[roots[1],a1,b1], &[[0.,0.],[1.,0.],[0.,1.]], &[a[2],base+1,b[2]]),
    ];
    r::OwnedPatchSet { vertices: vec![], edges: extra.to_vec(), patches }
}

#[test]
fn closed_straight_end_caps_do_not_inherit_g2_admission_from_contact_roots() {
    use brep_core::face_cut;
    let (source, request, skin, contacts) = nonlinear_fixture(1.);
    let before = source.clone();
    let retention = face_cut::plan_contact_retention(&source, 0, request, &skin,
        [&contacts[0], &contacts[1]], "stop-cap-g2-control", 1e-8, Default::default(),
        |roots| Ok(contact_material_paths(&source, request, &contacts, roots))).unwrap();
    let limits = r::EmbeddedPatchLimits {
        construction: Default::default(), linear_cells: 100_000,
        global: brep_core::boundary_embedding::Limits {
            exact_work: 1_000_000, trim_pairs: 10_000, trim_cells: 100_000,
            trim_domain_cells: 1_000_000, spans: 10_000,
            contacts: brep_core::face_contacts::Limits { pairs: 10_000, cells: 1_000_000,
                domain_cells: 1_000_000, cells_per_pair: 1000, domain_cells_per_pair: 10_000 },
        },
    };
    let authored = straight_stop_patch_set(&retention, &skin);
    let cap = &authored.patches[1];
    let cap_co = &cap.loops[0][0];
    let plan = retention.plan();
    let retained = &plan.model.faces[plan.regions[0].retained_faces[0]];
    let support_co = plan.model.loops[retained.outer].coedges.iter()
        .find(|co| co.edge == cap_co.edge).unwrap();
    let seam = nurbs_core::surface_g2_pcurves::inspect(
        &plan.model.edges[cap_co.edge].curve, [&support_co.pcurve, &cap_co.pcurve],
        [&retained.surface, &cap.surface], [support_co.reversed, cap_co.reversed],
        source.tolerance_mm, 1e-5, 1e-4, 100_000, 1_000_000).unwrap();
    assert!(seam.positions.proven);
    assert_eq!(seam.geometric.decision, nurbs_core::surface_join::Decision::CorrespondenceExceedsTolerance);
    eprintln!("stop_cap_g2 edge={} normal_lower={:?} position_upper={:?}", cap_co.edge,
        seam.geometric.branches.each_ref().map(|b| b.normal_lower), seam.positions.upper);
    // The low-level candidate still reaches closure and fails the actual G2 gate.
    let selected = plan.regions.iter().flat_map(|r| r.discard_faces.iter().copied()).collect::<Vec<_>>();
    let patches = authored.patches.iter().map(|p| r::SewPatch {
        surface: &p.surface, loops: &p.loops, reversed: p.reversed,
    }).collect::<Vec<_>>();
    match r::sew_g2_patch_set(&plan.model, &selected, &authored.vertices, &authored.edges,
        &patches, "closed-c0-not-g2", 1e-8, limits.construction) {
        Err(e) => {
            assert!(e.message.contains("G2 interfaces"), "{e:?}");
            assert!(e.message.contains("edge=") && e.message.contains("cells="),"{e:?}");
        },
        Ok(_) => panic!("Flat stops cannot inherit G2"),
    }
    let constructed = std::cell::Cell::new(false);
    match r::sew_contact_g2_patch_set(retention, "no-false-g2-stop", 1e-8, limits,
        |retention| { constructed.set(true); Ok(straight_stop_patch_set(retention, &skin)) }) {
        Err(r::ContactPatchFailure::EndRegion(report)) => {
            assert!(report.needs_expansion());
            assert_eq!(report.ends.len(), 2);
            assert!(report.ends.iter().all(|e| e.decision == r::EndNormalDecision::ExpandEndRegion));
            eprintln!("source_end_normals={report:?}");
        }
        Err(e) => panic!("Expected a source endpoint obstruction: {e:?}"),
        Ok(_) => panic!("A regular finite G2 fan cannot reconcile these retained endpoint normals"),
    }
    assert!(!constructed.get());
    assert_eq!(source, before);
}

#[test]
fn coplanar_retained_supports_do_not_inherit_a_sharp_endpoint_obstruction() {
    use brep_core::face_cut;
    let source = brep_core::cuboid([0.;3], [12.;3]).unwrap();
    let face = 0;
    let ring = &source.loops[source.faces[face].outer].coedges;
    let endpoints = [0,2].map(|i| source.edges[ring[i].edge].vertices[usize::from(ring[i].reversed)]);
    let segment = face_cut::Segment {
        curve: Curve::from_polyline(endpoints.map(|v| source.vertices[v].point.to_vec()).to_vec()).unwrap(),
        pcurve: Curve::from_polyline([0,2].map(|i| ring[i].pcurve.control_points[0].clone()).to_vec()).unwrap(),
    };
    let split = face_cut::partition(&source, face, endpoints, &[segment], 1e-8, 100_000).unwrap();
    let request = EdgeRadius { edge: split.edges[0], radius_mm: 1. };
    let (source, request, skin, contacts) = fixture_for_edge(split.model, request, [[0.25,0.5,0.75];2], 0.125);
    let retention = face_cut::plan_contact_retention(&source, 0, request, &skin,
        [&contacts[0], &contacts[1]], "coplanar-end-control", 1e-8, Default::default(),
        |roots| Ok(contact_material_paths(&source, request, &contacts, roots))).unwrap();
    let diagnostic = r::inspect_contact_end_normals(&retention).unwrap();
    assert!(!diagnostic.needs_expansion());
    assert!(diagnostic.ends.iter().all(|end|
        end.decision == r::EndNormalDecision::NoObstructionProven));
    assert!(diagnostic.ends.iter().all(|end| end.unsigned_normal_gap_lower == Some(0.)));
}

#[test]
fn endpoint_normal_witnesses_keep_source_face_roles_when_cut_order_reverses() {
    use brep_core::face_cut;
    let (source, request, skin, contacts) = nonlinear_fixture(1.);
    let build = |reverse: bool| face_cut::plan_contact_retention(&source, 0, request, &skin,
        [&contacts[0], &contacts[1]], "end-normal-role-order", 1e-8, Default::default(),
        |roots| {
            let mut paths = contact_material_paths(&source, request, &contacts, roots);
            if reverse { paths.reverse(); }
            Ok(paths)
        }).unwrap();
    let first = r::inspect_contact_end_normals(&build(false)).unwrap();
    let reversed = r::inspect_contact_end_normals(&build(true)).unwrap();
    assert!(first.needs_expansion() && reversed.needs_expansion());
    for (a,b) in first.ends.iter().zip(&reversed.ends) {
        assert_eq!(a.source_faces, b.source_faces);
        assert_eq!(a.normals, b.normals);
        assert_eq!(a.unsigned_normal_gap_lower, b.unsigned_normal_gap_lower);
        assert_eq!(a.towards_source_vertex, b.towards_source_vertex);
    }
}

#[test]
fn endpoint_obstruction_is_preserved_by_a_rigid_world_pose() {
    use brep_core::face_cut;
    let mut gaps = Vec::new();
    for posed in [false, true] {
        let (mut source, request, mut skin, mut contacts) = nonlinear_fixture(1.);
        if posed {
            let pose = |p: &mut Vec<f64>| { let old = p.clone(); *p = vec![old[1]+3., -old[0]-2., old[2]+1.]; };
            for v in &mut source.vertices {
                let p = v.point; v.point = [p[1]+3., -p[0]-2., p[2]+1.];
            }
            for e in &mut source.edges { for p in &mut e.curve.control_points { pose(p); } }
            for f in &mut source.faces { for p in f.surface.control_points.iter_mut().flatten() { pose(p); } }
            for p in skin.control_points.iter_mut().flatten() { pose(p); }
            for c in &mut contacts { for p in &mut c.curve.control_points { pose(p); } }
            source.rebuild_topology_ids();
        }
        let retention = face_cut::plan_contact_retention(&source, 0, request, &skin,
            [&contacts[0], &contacts[1]], "posed-end-normal-control", 1e-8, Default::default(),
            |roots| Ok(contact_material_paths(&source, request, &contacts, roots))).unwrap();
        let diagnostic = r::inspect_contact_end_normals(&retention).unwrap();
        assert!(diagnostic.needs_expansion());
        gaps.push(diagnostic.ends.iter().map(|e| e.unsigned_normal_gap_lower.unwrap()).collect::<Vec<_>>());
    }
    for (a,b) in gaps[0].iter().zip(&gaps[1]) { assert!((a-b).abs() < 1e-10); }
}

#[test]
fn reversed_contact_traversal_preserves_original_shared_root_ownership() {
    for (source, request, skin, contacts) in [fixture(), nonlinear_fixture(1.), nonlinear_fixture(0.5)] {
    let before = source.clone();
    let base = brep_core::edge_partition::contact_interval::between_contacts(
        &source,0,request,&skin,[&contacts[0],&contacts[1]],
        "orientation-independent-roots",1e-8,Default::default()).unwrap();
    for flags in [[true,false],[false,true],[true,true]] {
        let mut changed=contacts.clone();
        for side in 0..2 {
            if flags[side] {
                changed[side].curve=changed[side].curve.reverse().unwrap();
                changed[side].support_pcurve=changed[side].support_pcurve.reverse().unwrap();
                changed[side].fillet_pcurve=changed[side].fillet_pcurve.reverse().unwrap();
            }
        }
        let actual=brep_core::edge_partition::contact_interval::between_contacts(
            &source,0,request,&skin,[&changed[0],&changed[1]],
            "orientation-independent-roots",1e-8,Default::default()).unwrap();
        assert_eq!(actual.root_parameters(),base.root_parameters());
        assert_eq!(actual.model(),base.model());
        for side in 0..2 {
            assert_eq!(actual.branch().contacts[side].contact_reversed,flags[side]);
        }
        actual.reverify().unwrap();
    }
    assert_eq!(source,before);
    }
}

#[test]
fn material_cuts_keep_literal_contacts_with_independent_reversed_traversals() {
    let (source,request,skin,contacts)=nonlinear_fixture(0.5);
    let before=source.clone();
    for flags in [[true,false],[true,true]] {
        let mut c=contacts.clone();
        for side in 0..2 {if flags[side] {
            c[side].curve=c[side].curve.reverse().unwrap();
            c[side].support_pcurve=c[side].support_pcurve.reverse().unwrap();
            c[side].fillet_pcurve=c[side].fillet_pcurve.reverse().unwrap();
        }}
        let retained=brep_core::face_cut::plan_contact_retention(&source,0,request,&skin,
            [&c[0],&c[1]],"reversed-material",1e-8,Default::default(),
            |roots|Ok(contact_material_paths(&source,request,&c,roots))).unwrap();
        for region in &retained.plan().regions {
            assert_eq!(region.discard_faces.len(),1);
            assert_eq!(region.retained_faces.len(),1);
            let side=retained.roots().branch().contacts.iter().position(|o|
                o.face==retained.roots().model().1.faces[region.source_face]).unwrap();
            assert!(region.cut_edges.iter().any(|&e|
                retained.plan().model.edges[e].curve==c[side].curve));
        }
        retained.reverify().unwrap();
    }
    assert_eq!(source,before);
}

#[test]
fn unequal_contact_extents_use_fresh_partitions_and_keep_outside_edge_pieces() {
    let (source,request,skin,c)=nonlinear_from_parts(1.,
        fixture_spans([[0.25,0.5,0.75],[0.375,0.5,0.625]]));
    let before=source.clone();
    let p=brep_core::face_cut::plan_contact_retention(&source,0,request,&skin,
        [&c[0],&c[1]],"unequal-material-extents",1e-8,Default::default(),
        |roots|Ok(contact_material_paths(&source,request,&c,roots))).unwrap();
    assert_ne!(p.roots().branch().contacts[0].source_clock_interval,p.roots().root_parameters());
    for region in &p.plan().regions {
        for &edge in p.roots().partition().edges() {
            if edge==p.roots().middle_edge() {continue;}
            let incident=|face:usize|p.plan().model.loops[p.plan().model.faces[face].outer]
                .coedges.iter().any(|co|co.edge==edge);
            assert!(region.retained_faces.iter().any(|&f|incident(f)));
            assert!(!region.discard_faces.iter().any(|&f|incident(f)));
        }
    }
    p.reverify().unwrap();assert_eq!(source,before);
}
