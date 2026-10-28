use super::*;
use crate::ss_face_membership::tests::{curves, fixtures, model};
const HOLE: [[f64; 2]; 2] = [[0.5625, 0.875], [0.4375, 0.5625]];
fn roots<'a>(c: &'a ss::QualifiedGraphCurves, m: &'a [Model; 2]) -> QualifiedFacePairRoots<'a> {
    qualify_coedge_roots(
        c.original(),
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap()
}
#[test]
fn actual_hole_events_split_edges_and_preserve_immutable_source() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let before = m.clone();
    let r = roots(&c, &m);
    let cuts = qualify_cut_representatives(&r, Default::default()).unwrap();
    let out = partition_cut_representatives(&cuts, Default::default()).unwrap();
    assert_eq!(out.vertices().len(), 2);
    assert_eq!(out.models()[0].edges.len(), m[0].edges.len() + 2);
    assert_eq!(out.models()[0].vertices.len(), m[0].vertices.len() + 2);
    assert_eq!(out.models()[1], m[1]);
    for v in out.vertices() {
        assert_eq!(
            out.models()[v.model].vertices[v.vertex].point,
            cuts.nodes()[v.node].position
        );
    }
    out.reverify().unwrap();
    assert_eq!(m, before);
}
#[test]
fn two_roots_on_one_actual_edge_use_descending_clocks_and_final_original_checks() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some([[0.625, 0.875], [0.125, 0.875]]));
    let r = roots(&c, &m);
    let cuts = qualify_cut_representatives(&r, Default::default()).unwrap();
    assert_eq!(cuts.cuts().len(), 2);
    let edges: Vec<_> = cuts
        .cuts()
        .iter()
        .map(|cut| r.records[cut.record].address.edge)
        .collect();
    assert_eq!(edges[0], edges[1]);
    let out = partition_cut_representatives(&cuts, Default::default()).unwrap();
    assert_eq!(out.vertices().len(), 2);
    assert!(out.vertices()[0].source_parameter > out.vertices()[1].source_parameter);
    out.reverify().unwrap();
}
#[test]
fn common_operand_boundaries_share_canonical_positions_without_topological_welding() {
    let c = curves(0.0625);
    let m = [
        model(c.original().source().first(), Some(HOLE)),
        model(c.original().source().second(), Some(HOLE)),
    ];
    let r = roots(&c, &m);
    let cuts = qualify_cut_representatives(&r, Default::default()).unwrap();
    let out = partition_cut_representatives(&cuts, Default::default()).unwrap();
    assert_eq!(out.vertices().len(), 4);
    assert_eq!(cuts.nodes().len(), 2);
    for node in 0..2 {
        let vertices: Vec<_> = out.vertices().iter().filter(|v| v.node == node).collect();
        assert_eq!(vertices.len(), 2);
        let a = out.models()[vertices[0].model].vertices[vertices[0].vertex].point;
        let b = out.models()[vertices[1].model].vertices[vertices[1].vertex].point;
        assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
    }
    out.reverify().unwrap();
}
#[test]
fn no_eligible_boundary_events_preserve_models_and_do_not_erase_closed_ss() {
    let c = curves(0.0625);
    let m = fixtures(&c, None);
    let r = roots(&c, &m);
    let cuts = qualify_cut_representatives(&r, Default::default()).unwrap();
    let out = partition_cut_representatives(&cuts, Default::default()).unwrap();
    assert!(out.vertices().is_empty());
    assert_eq!(out.models(), m);
    assert_eq!(r.original.kind(), QuadraticGraphSSKind::ClosedEllipse);
    out.reverify().unwrap();
}
fn add_shared_wall(m: &mut Model) -> usize {
    use crate::{Coedge, Edge, Face, FaceUse, Loop, Vertex};
    let wire = m.faces[0].holes[0];
    let edge = m.loops[wire].coedges[0].edge;
    let e = m.edges[edge].clone();
    let [a, b] = e.curve.domain();
    let lowered = |mut p: Vec<f64>| {
        p[2] -= 1.;
        p
    };
    let surface = Surface {
        degree_u: e.curve.degree,
        degree_v: 1,
        knots_u: e.curve.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: e
            .curve
            .control_points
            .iter()
            .map(|p| vec![p.clone(), lowered(p.clone())])
            .collect(),
        weights: e.curve.weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let v = m.vertices.len();
    for &i in &e.vertices {
        let mut p = m.vertices[i].point;
        p[2] -= 1.;
        m.vertices.push(Vertex { point: p });
    }
    let first = m.edges.len();
    let right = crate::line(
        m.vertices[e.vertices[1]].point.to_vec(),
        m.vertices[v + 1].point.to_vec(),
    );
    m.edges.push(Edge {
        vertices: [e.vertices[1], v + 1],
        curve: right,
        degenerate: false,
    });
    let mut bottom = e.curve.clone();
    for p in &mut bottom.control_points {
        p[2] -= 1.;
    }
    m.edges.push(Edge {
        vertices: [v, v + 1],
        curve: bottom,
        degenerate: false,
    });
    let left = crate::line(
        m.vertices[e.vertices[0]].point.to_vec(),
        m.vertices[v].point.to_vec(),
    );
    m.edges.push(Edge {
        vertices: [e.vertices[0], v],
        curve: left,
        degenerate: false,
    });
    let mut top = crate::line(vec![a, 0.], vec![b, 0.]);
    top.knots = vec![20., 20., 24., 24.];
    let outer = m.loops.len();
    m.loops.push(Loop {
        coedges: vec![
            Coedge::new(edge, false, top),
            Coedge::new(first, false, crate::line(vec![b, 0.], vec![b, 1.])),
            Coedge::new(first + 1, true, crate::line(vec![b, 1.], vec![a, 1.])),
            Coedge::new(first + 2, true, crate::line(vec![a, 1.], vec![a, 0.])),
        ],
    });
    let face = m.faces.len();
    m.faces.push(Face {
        surface,
        outer,
        holes: vec![],
    });
    m.shells[0].faces.push(FaceUse {
        face,
        reversed: true,
    });
    m.rebuild_topology_ids();
    m.validate().unwrap();
    edge
}
#[test]
fn actual_shared_edge_updates_secondary_surface_and_independent_pcurve_clock() {
    let c = curves(0.0625);
    let mut m = fixtures(&c, Some(HOLE));
    let edge = add_shared_wall(&mut m[0]);
    let before = m.clone();
    let edge_id = m[0].1.edges[edge];
    m[0].1.local_tolerances_mm.insert(edge_id, 1e-7);
    m[0].1.sewed_edges.insert(edge_id);
    let r = roots(&c, &m);
    let cuts = qualify_cut_representatives(&r, Default::default()).unwrap();
    let out = partition_cut_representatives(&cuts, Default::default()).unwrap();
    let vertex = out
        .vertices()
        .iter()
        .find(|v| v.source_edge == edge)
        .unwrap();
    let model = &out.models()[0];
    let descendants: Vec<_> = model
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&vertex.vertex))
        .collect();
    assert_eq!(descendants.len(), 2);
    for (ei, _) in descendants {
        assert_eq!(
            model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == ei)
                .count(),
            2
        );
        assert!(model.1.sewed_edges.contains(&model.1.edges[ei]));
    }
    let wall = &model.loops[model.faces[1].outer];
    let top: Vec<_> = wall
        .coedges
        .iter()
        .filter(|c| model.edges[c.edge].vertices.contains(&vertex.vertex))
        .collect();
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].pcurve.domain()[0], 20.);
    assert_eq!(top[1].pcurve.domain()[1], 24.);
    out.reverify().unwrap();
    assert_eq!(before[0].edges, m[0].edges);
}
#[test]
fn shared_partition_budget_exhaustion_and_mutation_do_not_publish_authority() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let before = m.clone();
    let r = roots(&c, &m);
    let cuts = qualify_cut_representatives(&r, Default::default()).unwrap();
    let mut out = partition_cut_representatives(&cuts, Default::default()).unwrap();
    let exact = CutPartitionLimits {
        operations: out.operations(),
        cells: out.cells(),
        curve_spans: out.curve_spans(),
        jobs: 2,
    };
    assert!(partition_cut_representatives(&cuts, exact).is_ok());
    assert!(
        partition_cut_representatives(
            &cuts,
            CutPartitionLimits {
                operations: exact.operations - 1,
                ..exact
            }
        )
        .is_err()
    );
    assert!(
        partition_cut_representatives(
            &cuts,
            CutPartitionLimits {
                cells: exact.cells - 1,
                ..exact
            }
        )
        .is_err()
    );
    assert!(partition_cut_representatives(&cuts, CutPartitionLimits { jobs: 1, ..exact }).is_err());
    let v = out.vertices()[0].clone();
    out.models[v.model].vertices[v.vertex].point[2] += 0.01;
    assert!(out.reverify().is_err());
    assert_eq!(m, before);
}
#[test]
fn same_model_operand_faces_are_partitioned_in_one_atomic_clone() {
    let c = curves(0.0625);
    let mut m = model(c.original().source().first(), Some(HOLE));
    let other = model(c.original().source().second(), Some(HOLE));
    let vo = m.vertices.len();
    let eo = m.edges.len();
    let lo = m.loops.len();
    m.vertices.extend(other.vertices.clone());
    m.edges.extend(other.edges.iter().cloned().map(|mut e| {
        for v in &mut e.vertices {
            *v += vo;
        }
        e
    }));
    m.loops.extend(other.loops.iter().cloned().map(|mut l| {
        for c in &mut l.coedges {
            c.edge += eo;
        }
        l
    }));
    let mut face = other.faces[0].clone();
    face.outer += lo;
    for h in &mut face.holes {
        *h += lo;
    }
    m.faces.push(face);
    m.shells.push(crate::Shell {
        faces: vec![crate::FaceUse {
            face: 1,
            reversed: false,
        }],
        closed: false,
    });
    m.rebuild_topology_ids();
    m.validate().unwrap();
    let before = m.clone();
    let r =
        qualify_coedge_roots(c.original(), [(&m, 0), (&m, 1)], 1e-8, Default::default()).unwrap();
    let cuts = qualify_cut_representatives(&r, Default::default()).unwrap();
    let out = partition_cut_representatives(&cuts, Default::default()).unwrap();
    assert_eq!(out.side_model(), [0, 0]);
    assert_eq!(out.models().len(), 1);
    assert_eq!(out.vertices().len(), 4);
    out.reverify().unwrap();
    assert_eq!(m, before);
}

#[test]
fn rational_original_queries_and_independent_world_knots_survive_actual_subdivision() {
    let c = curves(0.0625);
    let mut m = fixtures(&c, Some(HOLE));
    let wire = m[0].faces[0].holes[0];
    let edge = m[0].loops[wire].coedges[0].edge;
    m[0].loops[wire].coedges[0].pcurve.weights[1] *= 2.;
    for (i, w) in m[0].edges[edge].curve.weights.iter_mut().enumerate() {
        *w *= 2f64.powi(i as i32);
    }
    for knot in &mut m[0].edges[edge].curve.knots {
        *knot = 10. + (*knot - HOLE[0][0]) / (HOLE[0][1] - HOLE[0][0]) * 4.;
    }
    let r = roots(&c, &m);
    let cuts = qualify_cut_representatives(&r, Default::default()).unwrap();
    let out = partition_cut_representatives(&cuts, Default::default()).unwrap();
    let v = out
        .vertices()
        .iter()
        .find(|v| v.source_edge == edge)
        .unwrap();
    assert!(v.source_parameter > 10. && v.source_parameter < 14.);
    let uses: Vec<_> = out.models()[0].loops[wire]
        .coedges
        .iter()
        .filter(|c| out.models()[0].edges[c.edge].vertices.contains(&v.vertex))
        .collect();
    assert_eq!(uses.len(), 2);
    assert!(
        uses.iter()
            .all(|c| c.pcurve.domain()[0] >= 0. && c.pcurve.domain()[1] <= 1.)
    );
    out.reverify().unwrap();
}
