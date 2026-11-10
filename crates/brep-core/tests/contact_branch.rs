use brep_core::{fillet_contact::{self, branch}, region_replacement::{EdgeRadius, plan_radius_network}};
use nurbs_core::{curve::Curve, surface::Surface};
fn fixture() -> (
    brep_core::Model,
    EdgeRadius,
    Surface,
    [fillet_contact::Contact; 2],
) {
    let m = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let edge = m
        .edges
        .iter()
        .position(|e| {
            e.vertices
                .iter()
                .all(|&v| m.vertices[v].point[0] == 0. && m.vertices[v].point[1] == 0.)
        })
        .unwrap();
    let r = EdgeRadius {
        edge,
        radius_mm: 1.,
    };
    let plan = plan_radius_network(&m, 0, &[r], 10000).unwrap();
    let mut pc = Vec::new();
    let mut curves = Vec::new();
    for &f in &plan.edges[0].supports {
        let face = &m.faces[f];
        let co = m.loops[face.outer]
            .coedges
            .iter()
            .find(|c| c.edge == edge)
            .unwrap();
        let mut p = if co.reversed {
            co.pcurve.reverse().unwrap()
        } else {
            co.pcurve.clone()
        };
        let axis = (0..2)
            .find(|&k| p.control_points[0][k] == p.control_points[1][k])
            .unwrap();
        let fixed = p.control_points[0][axis];
        let at = if fixed == 0. { 0.125 } else { 0.875 };
        for q in &mut p.control_points {
            q[axis] = at;
        }
        let points = p
            .control_points
            .iter()
            .map(|uv| face.surface.evaluate(uv[0], uv[1]).unwrap().point.to_vec())
            .collect();
        curves.push(Curve::from_polyline(points).unwrap());
        pc.push(p);
    }
    let skin = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: curves.iter().map(|c| c.control_points.clone()).collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let contacts = std::array::from_fn(|i| {
        let p = Curve::from_polyline(vec![vec![i as f64, 0.], vec![i as f64, 1.]]).unwrap();
        let positions = nurbs_core::surface_g2_pcurves::verify_positions(
            &curves[i],
            [&pc[i], &p],
            [&m.faces[plan.edges[0].supports[i]].surface, &skin],
            [false; 2],
            1e-7,
            1000,
            100000,
        )
        .unwrap();
        fillet_contact::Contact {
            curve: curves[i].clone(),
            support_pcurve: pc[i].clone(),
            fillet_pcurve: p,
            positions,
        }
    });
    (m, r, skin, contacts)
}
#[test]
fn actual_contact_collars_bind_to_original_selected_edge() {
    let (m, r, skin, c) = fixture();
    let before = m.clone();
    let report = branch::inspect(&m, 0, r, &skin, [&c[0], &c[1]], 10000, 10000, 1000000).unwrap();
    assert_eq!(
        report.decision,
        branch::Decision::Bound,
        "{}",
        report.reason
    );
    assert_eq!(report.edge, m.1.edges[r.edge]);
    assert_eq!(report.collars.len(), 2);
    assert_eq!(report.positions.len(), 3);
    assert!(report.positions.iter().all(|p| p.proven));
    assert_eq!(m, before);
    let exhausted = branch::inspect(&m, 0, r, &skin, [&c[0], &c[1]], 10000, 0, 0).unwrap();
    assert_eq!(exhausted.decision, branch::Decision::Unresolved);
}
#[test]
fn another_edge_opposite_side_and_forged_contact_proof_cannot_inherit_binding() {
    let (m, r, skin, mut c) = fixture();
    let mut other = r;
    other.edge = (r.edge + 1) % m.edges.len();
    assert_eq!(
        branch::inspect(&m, 0, other, &skin, [&c[0], &c[1]], 10000, 10000, 1000000)
            .unwrap()
            .decision,
        branch::Decision::Unresolved
    );
    // Preserve the caller-supplied positive receipt while changing actual skin.
    let mut changed = skin.clone();
    for row in &mut changed.control_points {
        for p in row {
            p[2] += 1.;
        }
    }
    assert!(c[0].positions.proven);
    assert_eq!(
        branch::inspect(&m, 0, r, &changed, [&c[0], &c[1]], 10000, 10000, 1000000)
            .unwrap()
            .decision,
        branch::Decision::Unresolved
    );
    let plan = plan_radius_network(&m, 0, &[r], 10000).unwrap();
    let face = &m.faces[plan.edges[0].supports[0]];
    let co = m.loops[face.outer]
        .coedges
        .iter()
        .find(|c| c.edge == r.edge)
        .unwrap();
    let axis = (0..2)
        .find(|&k| co.pcurve.control_points[0][k] == co.pcurve.control_points[1][k])
        .unwrap();
    let far = if co.pcurve.control_points[0][axis] == 0. {
        0.875
    } else {
        0.125
    };
    for p in &mut c[0].support_pcurve.control_points {
        p[axis] = far;
    }
    assert_eq!(
        branch::inspect(&m, 0, r, &skin, [&c[0], &c[1]], 10000, 10000, 1000000)
            .unwrap()
            .decision,
        branch::Decision::Unresolved
    );
}
#[test]
fn nonlinear_rational_contact_clock_keeps_original_edge_ownership() {
    let (m, r, mut skin, mut c) = fixture();
    let elevate_trace = |curve: &mut Curve| {
        let a = curve.control_points[0].clone();
        let b = curve.control_points[1].clone();
        curve.control_points = vec![
            a.clone(),
            a.iter().zip(&b).map(|(x, y)| (x + y) * 0.5).collect(),
            b,
        ];
        curve.degree = 2;
        curve.knots = vec![0., 0., 0., 1., 1., 1.];
        curve.weights = vec![1., 0.875, 1.];
    };
    for contact in &mut c {
        elevate_trace(&mut contact.curve);
        elevate_trace(&mut contact.support_pcurve);
    }
    skin.degree_v = 2;
    skin.knots_v = vec![0., 0., 0., 1., 1., 1.];
    skin.control_points = c.iter().map(|c| c.curve.control_points.clone()).collect();
    skin.weights = vec![vec![1., 0.875, 1.]; 2];
    let report = branch::inspect(&m, 0, r, &skin, [&c[0], &c[1]], 10000, 10000, 1000000).unwrap();
    assert_eq!(
        report.decision,
        branch::Decision::Bound,
        "{}",
        report.reason
    );
    // Fold back along the same chart line: sampled ownership cannot admit it.
    let free = 1 - report.collars[0].cross_axis;
    c[0].support_pcurve.control_points[1][free] = c[0].support_pcurve.control_points[2][free] + 10.;
    assert_eq!(
        branch::inspect(&m, 0, r, &skin, [&c[0], &c[1]], 10000, 10000, 1000000)
            .unwrap()
            .decision,
        branch::Decision::Unresolved
    );
}

#[path = "../examples/support/nurbs_contact_stock.rs"]
mod rational_stock;
#[test]
fn original_rational_stock_contacts_bind_without_rebuilding_supports() {
    let f = rational_stock::build();
    let edge = match f.cuts[1].anchor {
        brep_core::region_replacement::RemovalAnchor::Edge(e) => e,
        _ => panic!(),
    };
    let request = EdgeRadius {
        edge,
        radius_mm: 1.,
    };
    let plan = plan_radius_network(&f.source, 0, &[request], 10000).unwrap();
    let contacts: Vec<_> = plan.edges[0]
        .supports
        .iter()
        .map(|&face| {
            let cut = f.cuts.iter().find(|c| c.face == face).unwrap();
            let mut curve = cut.segments[0].curve.clone();
            let mut support_pcurve = cut.segments[0].pcurve.clone();
            let mut fillet_pcurve = cut.replacement[0].clone();
            let z = f.source.vertices[f.source.edges[edge].vertices[0]].point[2];
            if curve.control_points[0][2] != z {
                curve = curve.reverse().unwrap();
                support_pcurve = support_pcurve.reverse().unwrap();
                fillet_pcurve = fillet_pcurve.reverse().unwrap();
            }
            let positions = nurbs_core::surface_g2_pcurves::verify_positions(
                &curve,
                [&support_pcurve, &fillet_pcurve],
                [&f.source.faces[face].surface, &f.blend],
                [false; 2],
                1e-7,
                10000,
                1000000,
            )
            .unwrap();
            fillet_contact::Contact {
                curve,
                support_pcurve,
                fillet_pcurve,
                positions,
            }
        })
        .collect();
    let result = branch::inspect(
        &f.source,
        0,
        request,
        &f.blend,
        [&contacts[0], &contacts[1]],
        10000,
        10000,
        1000000,
    )
    .unwrap();
    assert_eq!(
        result.decision,
        branch::Decision::Bound,
        "{}",
        result.reason
    );
    assert!(result.collars.iter().all(|c| c.injectivity.certified));
}

#[test]
fn nonparallel_rational_multispan_contacts_have_owned_variable_width_ribbons() {
    let (m, r, mut skin, mut contacts) = fixture();
    let plan = plan_radius_network(&m, 0, &[r], 10000).unwrap();
    for i in 0..2 {
        let surface = &m.faces[plan.edges[0].supports[i]].surface;
        let first = contacts[i].support_pcurve.control_points[0].clone();
        let last = contacts[i].support_pcurve.control_points[1].clone();
        let cross = (0..2).find(|&k| first[k] == last[k]).unwrap();
        let side = if first[cross] < 0.5 { 1. } else { -1. };
        let mut cp: Vec<_> = [0., 0.25, 0.75, 1.]
            .iter()
            .map(|&t| {
                vec![
                    first[0] + t * (last[0] - first[0]),
                    first[1] + t * (last[1] - first[1]),
                ]
            })
            .collect();
        cp[1][cross] += side * 0.125;
        cp[2][cross] += side * 0.0625;
        let support = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
            control_points: cp,
            weights: vec![1., 0.875, 1.125, 1.],
            periodic: false,
        };
        let mut curve = support.clone();
        curve.control_points = support
            .control_points
            .iter()
            .map(|uv| surface.evaluate(uv[0], uv[1]).unwrap().point.to_vec())
            .collect();
        contacts[i].support_pcurve = support;
        contacts[i].curve = curve;
    }
    skin.degree_v = 2;
    skin.knots_v = contacts[0].curve.knots.clone();
    skin.control_points = contacts
        .iter()
        .map(|c| c.curve.control_points.clone())
        .collect();
    skin.weights = contacts.iter().map(|c| c.curve.weights.clone()).collect();
    let proof = branch::inspect(
        &m,
        0,
        r,
        &skin,
        [&contacts[0], &contacts[1]],
        10000,
        10000,
        1000000,
    )
    .unwrap();
    assert_eq!(proof.decision, branch::Decision::Bound, "{}", proof.reason);
    for collar in &proof.collars {
        assert!(collar.clock.proven);
        assert_eq!(collar.clock.spans, 2);
        assert!(collar.contact_band[0] < collar.contact_band[1]);
    }
    // Skin/pcurves still form a valid stored curve definition, but a contact
    // strip that crosses the opposite half of the original face has no receipt.
    let axis = proof.collars[0].cross_axis;
    contacts[0].support_pcurve.control_points[1][axis] = 0.5;
    assert_eq!(
        branch::inspect(
            &m,
            0,
            r,
            &skin,
            [&contacts[0], &contacts[1]],
            10000,
            10000,
            1000000
        )
        .unwrap()
        .decision,
        branch::Decision::Unresolved
    );
}
