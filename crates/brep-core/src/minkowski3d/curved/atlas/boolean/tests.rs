use super::*;
// Independent exact polynomial homeomorphisms for the two operands. Surfaces
// and edges are authored with matching original clocks, not fitted samples.
fn deform(mut m: Model, axis: usize, coordinate: usize, k: f64) -> Model {
    for f in &mut m.faces {
        let old = f.surface.clone();
        assert_eq!((old.degree_u, old.degree_v), (1, 1));
        let o = &old.control_points[0][0];
        let du = old.control_points[1][0][coordinate] - o[coordinate];
        let dv = old.control_points[0][1][coordinate] - o[coordinate];
        f.surface.degree_u = 2;
        f.surface.degree_v = 2;
        let (ua, ub) = (
            old.knots_u[old.degree_u],
            old.knots_u[old.control_points.len()],
        );
        let (va, vb) = (
            old.knots_v[old.degree_v],
            old.knots_v[old.control_points[0].len()],
        );
        f.surface.knots_u = vec![ua, ua, ua, ub, ub, ub];
        f.surface.knots_v = vec![va, va, va, vb, vb, vb];
        f.surface.weights = vec![vec![1.; 3]; 3];
        f.surface.control_points = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let mut p = (0..3)
                            .map(|a| {
                                o[a] + (old.control_points[1][0][a] - o[a]) * i as f64 / 2.
                                    + (old.control_points[0][1][a] - o[a]) * j as f64 / 2.
                            })
                            .collect::<Vec<_>>();
                        p[axis] += k
                            * (o[coordinate] * o[coordinate]
                                + o[coordinate] * du * i as f64
                                + o[coordinate] * dv * j as f64
                                + if i == 2 { du * du } else { 0. }
                                + if j == 2 { dv * dv } else { 0. }
                                + du * dv * (i * j) as f64 / 2.);
                        p
                    })
                    .collect()
            })
            .collect();
    }
    for e in &mut m.edges {
        assert_eq!(e.curve.degree, 1);
        let [ta, tb] = e.curve.domain();
        let a = e.curve.control_points[0].clone();
        let b = e.curve.control_points[1].clone();
        let mut x = a.clone();
        let mut y = b.clone();
        x[axis] += k * a[coordinate] * a[coordinate];
        y[axis] += k * b[coordinate] * b[coordinate];
        let mut mid = (0..3).map(|i| (x[i] + y[i]) / 2.).collect::<Vec<_>>();
        mid[axis] -= k * (b[coordinate] - a[coordinate]).powi(2) / 2.;
        e.curve.degree = 2;
        e.curve.knots = vec![ta, ta, ta, tb, tb, tb];
        e.curve.control_points = vec![x, mid, y];
        e.curve.weights = vec![1.; 3];
    }
    for v in &mut m.vertices {
        v.point[axis] += k * v.point[coordinate] * v.point[coordinate];
    }
    m.rebuild_topology_ids();
    m.validate().unwrap();
    m
}
fn container() -> Model {
    deform(
        crate::operations::extrude_polygon(
            &[[0., 0.], [4., 0.], [4., 1.], [1., 1.], [1., 4.], [0., 4.]],
            0.,
            2.,
        )
        .unwrap(),
        2,
        0,
        1. / 64.,
    )
}
fn tool(lo: [f64; 3], hi: [f64; 3]) -> Model {
    deform(crate::cuboid(lo, hi).unwrap(), 1, 2, 1. / 64.)
}
#[test]
fn nonconvex_independent_nurbs_classify_and_select_all_four_operations() {
    let a = container();
    let b = tool([0.25, 0.25, 0.5], [0.5, 0.5, 0.75]);
    assert!(
        super::super::super::try_common_quadratic_boolean(&a, &b, "difference")
            .unwrap()
            .is_none()
    );
    for op in ["union", "difference", "intersection", "xor"] {
        let r = certify_nonintersecting_nurbs_boolean(&a, &b, op, 0).unwrap();
        assert_eq!(r.certificate.relation, NurbsMaterialRelation::AContainsB);
        assert_eq!(r.certificate.winding, [0, 1]);
        assert_eq!(
            r.certificate.checked_cross_pairs,
            r.certificate.source_atlases[0].triangles.len()
                * r.certificate.source_atlases[1].triangles.len()
        );
        r.model().validate_continuous(32768).unwrap();
        match op {
            "union" => assert_eq!(r.model().faces.len(), a.faces.len()),
            "intersection" => assert_eq!(r.model().faces.len(), b.faces.len()),
            _ => assert_eq!(r.model().bodies[0].inner_shells.len(), 1),
        }
        if op == "difference" {
            verify_nonintersecting_nurbs_boolean(&a, &b, r.model(), r.certificate()).unwrap();
            assert_eq!(&crate::operations::boolean(&a, &b, op).unwrap(), r.model());
            let mut forged = r.certificate().clone();
            forged.winding = [0, 0];
            assert!(verify_nonintersecting_nurbs_boolean(&a, &b, r.model(), &forged).is_err());
        }
    }
}
#[test]
fn overlapping_boxes_do_not_turn_a_nonconvex_concavity_into_material() {
    let a = container();
    // Inside the overall control AABB but outside the L-shaped material.
    let b = tool([2., 2., 0.5], [2.5, 2.5, 1.]);
    for op in ["union", "difference", "intersection", "xor"] {
        let r = certify_nonintersecting_nurbs_boolean(&a, &b, op, 0).unwrap();
        assert_eq!(r.certificate.relation, NurbsMaterialRelation::Disjoint);
        assert_eq!(r.certificate.winding, [0, 0]);
        match op {
            "union" | "xor" => assert_eq!(r.model().bodies.len(), 2),
            "intersection" => assert!(r.model().is_empty()),
            _ => assert_eq!(r.model().faces.len(), a.faces.len()),
        }
        if op == "union" {
            verify_nonintersecting_nurbs_boolean(&a, &b, r.model(), r.certificate()).unwrap();
        }
    }
}
#[test]
fn reverse_containment_and_unproved_crossings_are_not_misclassified() {
    let a = container();
    let b = tool([0.25, 0.25, 0.5], [0.5, 0.5, 0.75]);
    let r = certify_nonintersecting_nurbs_boolean(&b, &a, "xor", 0).unwrap();
    assert_eq!(r.certificate.relation, NurbsMaterialRelation::BContainsA);
    assert_eq!(r.model().bodies[0].inner_shells.len(), 1);
    assert!(
        certify_nonintersecting_nurbs_boolean(&b, &a, "difference", 0)
            .unwrap()
            .model()
            .is_empty()
    );
    let crossing = tool([0.5, 0.5, 0.5], [1.5, 1.5, 1.5]);
    assert!(certify_nonintersecting_nurbs_boolean(&a, &crossing, "difference", 0).is_err());
    let mut inward = b.clone();
    for f in &mut inward.shells[0].faces {
        f.reversed ^= true;
    }
    inward.rebuild_topology_ids();
    assert!(certify_nonintersecting_nurbs_boolean(&a, &inward, "difference", 0).is_err());
}

#[test]
fn a_sample_invisible_edge_deviation_cannot_acquire_a_boolean_certificate() {
    let mut a = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut power = vec![1.];
    // All nine positions used by the sampled validator are exact roots.
    for i in 0..=8 {
        let r = i as f64 / 8.;
        let mut next = vec![0.; power.len() + 1];
        for (k, c) in power.iter().enumerate() {
            next[k] -= c * r;
            next[k + 1] += c;
        }
        power = next;
    }
    let choose = |n: usize, k: usize| (0..k).fold(1., |a, i| a * (n - i) as f64 / (i + 1) as f64);
    let edge = &mut a.edges[0];
    let p = edge.curve.control_points[0].clone();
    let q = edge.curve.control_points[1].clone();
    let axis = (0..3).find(|i| p[*i] == q[*i]).unwrap();
    let d = power.len() - 1;
    edge.curve.degree = d;
    edge.curve.knots = vec![0.; d + 1];
    edge.curve.knots.extend(vec![1.; d + 1]);
    edge.curve.weights = vec![1.; d + 1];
    edge.curve.control_points = (0..=d)
        .map(|i| {
            let mut c = (0..3)
                .map(|k| p[k] + (q[k] - p[k]) * i as f64 / d as f64)
                .collect::<Vec<_>>();
            c[axis] += 100.
                * (0..=i)
                    .map(|k| power[k] * choose(i, k) / choose(d, k))
                    .sum::<f64>();
            c
        })
        .collect();
    a.rebuild_topology_ids();
    a.validate().unwrap();
    let b = crate::cuboid([0.25; 3], [0.5; 3]).unwrap();
    let e = certify_nonintersecting_nurbs_boolean(&a, &b, "difference", 0).unwrap_err();
    assert!(e.message.contains("boundary agreement"), "{}", e.message);
}

#[test]
fn a_partial_use_report_is_not_whole_atlas_boundary_authority() {
    let mut a = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let wire = a.faces[0].outer;
    let coedge = a.loops[wire].coedges[0].clone();
    let [ca, cb] = a.edges[coedge.edge].curve.domain();
    let [pa, pb] = coedge.pcurve.domain();
    let middle = (ca + cb) / 2.;
    let curve_parameter = if coedge.reversed {
        [middle, cb]
    } else {
        [ca, middle]
    };
    a.loops[wire].coedges[0] = coedge
        .with_trim(brep_topology::CoedgeTrim {
            curve_parameter,
            pcurve_parameter: [pa, (pa + pb) / 2.],
            periodic_lift: [[0; 2]; 2],
        })
        .unwrap();
    a.rebuild_topology_ids();
    a.validate().unwrap();
    assert!(
        crate::boundary_agreement::verify_with_exact(&a, 1000, 1_000_000)
            .unwrap()
            .agreement
            .complete
    );
    let b = crate::cuboid([0.25; 3], [0.5; 3]).unwrap();
    let e = certify_nonintersecting_nurbs_boolean(&a, &b, "difference", 0).unwrap_err();
    assert!(e.message.contains("full forward authored"), "{}", e.message);
}

#[test]
fn a_specialized_family_refusal_does_not_hide_a_complete_material_proof() {
    let mut a = crate::cuboid([0.; 3], [4., 4., 2.]).unwrap();
    let face = 1;
    let old = a.faces[face].surface.clone();
    let range = [
        [
            old.knots_u[old.degree_u],
            old.knots_u[old.control_points.len()],
        ],
        [
            old.knots_v[old.degree_v],
            old.knots_v[old.control_points[0].len()],
        ],
    ];
    let mut top = old.clone();
    top.degree_u = 2;
    top.degree_v = 2;
    top.knots_u = vec![0., 0., 0., 1., 1., 1.];
    top.knots_v = top.knots_u.clone();
    top.weights = vec![vec![1.; 3]; 3];
    top.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    let mut p = (0..3)
                        .map(|k| {
                            old.control_points[0][0][k]
                                + (old.control_points[1][0][k] - old.control_points[0][0][k])
                                    * i as f64
                                    / 2.
                                + (old.control_points[0][1][k] - old.control_points[0][0][k])
                                    * j as f64
                                    / 2.
                        })
                        .collect::<Vec<_>>();
                    if i == 1 && j == 1 {
                        p[2] += 0.25;
                    }
                    p
                })
                .collect()
        })
        .collect();
    let wire = a.faces[face].outer;
    for c in &mut a.loops[wire].coedges {
        for p in &mut c.pcurve.control_points {
            for k in 0..2 {
                p[k] = (p[k] - range[k][0]) / (range[k][1] - range[k][0]);
            }
        }
    }
    a.faces[face].surface = top;
    a.rebuild_topology_ids();
    a.validate().unwrap();
    let b = tool([0.25, 0.25, 0.5], [0.5, 0.5, 0.75]);
    assert!(crate::nurbs_ss_g6::is_nurbs_boolean_v3_candidate(&a, &b));
    assert!(crate::nurbs_ss_g6::nurbs_boolean_graph_patch_v3(&a, &b, "difference").is_err());
    let expected = certify_nonintersecting_nurbs_boolean(&a, &b, "difference", 0).unwrap();
    assert_eq!(
        expected.certificate.relation,
        NurbsMaterialRelation::AContainsB
    );
    assert_eq!(
        crate::operations::boolean(&a, &b, "difference").unwrap(),
        *expected.model()
    );
}
