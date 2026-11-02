use brep_core::feature_ops::full_round_curved::build;
use nurbs_core::{curve::Curve, trim_domain::curve_cover::Limits};

#[test]
fn source_arc_cover_keeps_actual_source_knot_clocks_and_actual_result_edges() {
    let c = |p: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: p.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., 0.7, 1.],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    let mut source = brep_core::prism::extrude(
        &[vec![
            line([0., 0.], [4., 0.]),
            c([[4., 0.], [5., 4.], [4., 8.]]),
            line([4., 8.], [0., 8.]),
            c([[0., 8.], [-1., 4.], [0., 0.]]),
        ]],
        0.,
        6.,
    )
    .unwrap();
    for e in &mut source.edges {
        for k in &mut e.curve.knots {
            *k = if *k == 0. { 0.1 } else { 1.9 };
        }
    }
    let edge = source
        .edges
        .iter()
        .position(|e| {
            e.curve.degree == 2
                && e.curve
                    .control_points
                    .iter()
                    .all(|p| p[2] == 6. && p[0] >= 4.)
        })
        .unwrap();
    let e = &mut source.edges[edge];
    e.vertices.swap(0, 1);
    e.curve.control_points.reverse();
    e.curve.weights.reverse();
    e.curve.knots = e.curve.knots.iter().rev().map(|k| -k).collect();
    for w in &mut source.loops {
        for c in &mut w.coedges {
            if c.edge == edge {
                c.reversed = !c.reversed;
            }
        }
    }
    assert_eq!(source.edges[edge].curve.domain(), [-1.9, -0.1]);
    source.validate().unwrap();
    let before = source.clone();
    let r = build(&source, [3, 2, 1]).unwrap();
    let cover = r
        .qualify_source_arc_cover(Limits {
            cells: 100_000,
            classification_work: 1_000_000,
            witness_work: 1_000_000,
        })
        .unwrap();
    assert_eq!(source, before);
    for (face, cert) in cover.arcs() {
        assert!(
            cert.loops()
                .iter()
                .flatten()
                .all(|c| c.domain() == [0.1, 1.9])
        );
        let f = &cover.result().faces[*face];
        let c = cover.result().loops[f.outer]
            .coedges
            .iter()
            .find_map(|u| {
                let mut c = cover.result().edges[u.edge].curve.clone();
                if c.control_points.iter().any(|p| p[2] != 6.) {
                    return None;
                }
                for p in &mut c.control_points {
                    p.truncate(2);
                }
                Some(c)
            })
            .unwrap();
        assert_eq!(cert.curve(), &c);
    }
}
