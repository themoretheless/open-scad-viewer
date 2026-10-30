use super::*;
use crate::{Shell, TopologyIds, cuboid, cylinder};

fn set_domain(curve: &mut Curve, domain: [f64; 2]) {
    let old = curve.domain();
    for knot in &mut curve.knots {
        *knot = domain[0] + (*knot - old[0]) / (old[1] - old[0]) * (domain[1] - domain[0]);
    }
}

fn coedge(model: &Model, face: usize, index: usize) -> &Coedge {
    let wire = model.faces[face].outer;
    &model.loops[wire].coedges[index]
}

fn coedge_mut(model: &mut Model, face: usize, index: usize) -> &mut Coedge {
    let wire = model.faces[face].outer;
    &mut model.loops[wire].coedges[index]
}

fn scale_use(model: &mut Model, face: usize, index: usize) {
    let edge = coedge_mut(model, face, index).edge;
    set_domain(&mut model.edges[edge].curve, [2., 6.]);
    set_domain(&mut coedge_mut(model, face, index).pcurve, [10., 14.]);
}

fn set_trim(model: &mut Model, face: usize, index: usize, c: [f64; 2], p: [f64; 2]) {
    let coedge = coedge_mut(model, face, index);
    let lifts = coedge.trim().map_or([[0; 2]; 2], |trim| trim.periodic_lift);
    *coedge = coedge
        .clone()
        .with_trim(CoedgeTrim {
            curve_parameter: c,
            pcurve_parameter: p,
            periodic_lift: lifts,
        })
        .unwrap();
}

fn partial_model(reversed: bool) -> (Model, usize) {
    let mut model = cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
    let index = (0..model.loops[model.faces[0].outer].coedges.len())
        .find(|&i| coedge_mut(&mut model, 0, i).reversed == reversed)
        .unwrap();
    scale_use(&mut model, 0, index);
    set_trim(&mut model, 0, index, [3., 5.], [11., 13.]);
    (model, index)
}

fn shrunk_face(mut model: Model, face: usize) -> Model {
    let count = model.loops[model.faces[face].outer].coedges.len();
    for i in 0..count {
        scale_use(&mut model, face, i);
        let c = if coedge(&model, face, i).reversed {
            [4., 6.]
        } else {
            [2., 4.]
        };
        set_trim(&mut model, face, i, c, [10., 12.]);
    }
    model
}

fn emit_open(model: &Model, face: usize) -> Model {
    let other = cuboid([10., 10., 10.], [11., 11., 11.]).unwrap();
    let mut asm = Assembler::new(model, &other, "union").unwrap();
    let mut chart = ChartVertices::default();
    let mut pieces = Vec::new();
    let mut links = Vec::new();
    for (ka, kb, pcurve, key, reversed) in asm.boundary_pieces(0, face).unwrap() {
        let ua = pcurve.evaluate(0.).unwrap().point;
        let ub = pcurve.evaluate(1.).unwrap().point;
        let ia = chart.index(asm.chart_key(ka), [ua[0], ua[1]]);
        let ib = chart.index(asm.chart_key(kb), [ub[0], ub[1]]);
        links.push((ka, kb, ia, ib));
        pieces.push(Piece {
            v: [ia, ib],
            pcurve,
            key,
            reversed,
        });
    }
    // Close the gaps left by authored partial uses with the exact remainder
    // of each source curve. This keeps the open-assembly agreement check
    // meaningful on curved as well as planar faces.
    let wire = model.faces[face].outer;
    for i in 0..links.len() {
        let next = (i + 1) % links.len();
        let (_, from, _, ia) = links[i];
        let (to, _, ib, _) = links[next];
        let source = &model.loops[wire].coedges[i];
        let interval = if source.reversed { [2., 4.] } else { [4., 6.] };
        let mut curve = normalized(
            model.edges[source.edge]
                .curve
                .trim(interval[0], interval[1])
                .unwrap(),
        );
        if source.reversed {
            curve = curve.reverse().unwrap();
        }
        let pcurve = normalized(source.pcurve.trim(12., 14.).unwrap());
        let key = asm.push_net([from, to], curve);
        pieces.push(Piece {
            v: [ia, ib],
            pcurve,
            key: EKey::Net(key),
            reversed: false,
        });
    }
    asm.emit_chart(0, face, false, pieces, chart.len(), &|_| Ok(true))
        .unwrap();
    let mut output = Model(
        brep_topology::Model {
            vertices: asm.vertices,
            edges: asm.edges,
            loops: asm.loops,
            faces: asm.faces,
            shells: vec![Shell {
                faces: asm.shell,
                closed: false,
            }],
            bodies: Vec::new(),
            tolerance_mm: model.tolerance_mm,
        },
        TopologyIds::default(),
    );
    output.rebuild_topology_ids();
    output
}

#[test]
fn partial_forward_and_reversed_use_native_authored_parameters() {
    for reversed in [false, true] {
        let (model, index) = partial_model(reversed);
        let other = cuboid([10., 10., 10.], [11., 11., 11.]).unwrap();
        let mut asm = Assembler::new(&model, &other, "union").unwrap();
        assert_eq!(asm.splits[&(0, coedge(&model, 0, index).edge)].len(), 2);
        let pieces = asm.boundary_pieces(0, 0).unwrap();
        let (_, _, ref pcurve, key, rev) = pieces[index];
        assert_eq!(rev, reversed);
        let EKey::Piece(_, edge, _) = key else {
            panic!("partial 3D trim must produce an edge piece")
        };
        assert_eq!(edge, coedge(&model, 0, index).edge);
        let output_edge = asm.edge(key).unwrap();
        assert_eq!(asm.edges[output_edge].curve.domain(), [3., 5.]);
        let original = &coedge(&model, 0, index).pcurve;
        for (t, q) in [(0., 11.), (1., 13.)] {
            let got = pcurve.evaluate(t).unwrap().point;
            let want = original.evaluate(q).unwrap().point;
            assert_eq!(got, want, "reversed={reversed}");
        }
    }
}

#[test]
fn pcurve_only_partial_use_does_not_split_3d_edge() {
    let (mut model, index) = partial_model(false);
    set_trim(&mut model, 0, index, [2., 6.], [11., 13.]);
    let other = cuboid([10., 10., 10.], [11., 11., 11.]).unwrap();
    let asm = Assembler::new(&model, &other, "union").unwrap();
    assert!(asm.splits.is_empty());
    let pieces = asm.boundary_pieces(0, 0).unwrap();
    assert!(matches!(pieces[index].3, EKey::Orig(0, _)));
}

#[test]
fn authored_original_endpoint_is_reused_and_external_splits_are_ignored() {
    let (mut model, index) = partial_model(false);
    set_trim(&mut model, 0, index, [2., 5.], [10., 13.]);
    let other = cuboid([10., 10., 10.], [11., 11., 11.]).unwrap();
    let asm = Assembler::new(&model, &other, "union").unwrap();
    let pieces = asm.boundary_pieces(0, 0).unwrap();
    assert!(matches!(pieces[index].0, VKey::Orig(0, _)));
    assert!(matches!(pieces[index].1, VKey::Special(_)));

    set_trim(&mut model, 0, index, [3., 5.], [11., 13.]);
    let edge = coedge(&model, 0, index).edge;
    let mut asm = Assembler::new(&model, &other, "union").unwrap();
    for t in [2.5, 4., 5.5] {
        let point = model.edges[edge].curve.evaluate(t).unwrap().point;
        let id = asm.special([point[0], point[1], point[2]]);
        asm.attach_edge(id, 0, edge, t);
    }
    let pieces = asm.boundary_pieces(0, 0).unwrap();
    let selected: Vec<_> = pieces
        .iter()
        .filter(|piece| matches!(piece.3, EKey::Piece(0, e, _) if e == edge))
        .collect();
    assert_eq!(selected.len(), 2);
    assert!(matches!(selected[0].3, EKey::Piece(0, _, 2)));
    assert!(matches!(selected[1].3, EKey::Piece(0, _, 3)));
}

#[test]
fn overlapping_partial_uses_share_physical_edge_pieces() {
    let (mut model, index) = partial_model(false);
    let wire = model.faces[0].outer;
    let mut second = model.loops[wire].coedges[index].clone();
    second = second
        .with_trim(CoedgeTrim {
            curve_parameter: [4., 6.],
            pcurve_parameter: [12., 14.],
            periodic_lift: [[0; 2]; 2],
        })
        .unwrap();
    model.loops[wire].coedges.push(second);
    let other = cuboid([10., 10., 10.], [11., 11., 11.]).unwrap();
    let asm = Assembler::new(&model, &other, "union").unwrap();
    let pieces = asm.boundary_pieces(0, 0).unwrap();
    let keys: Vec<_> = pieces.iter().map(|piece| piece.3).collect();
    let common: Vec<_> = keys.iter().filter(|key| keys.iter().filter(|k| k == key).count() > 1).collect();
    assert_eq!(common.len(), 2);
    assert!(matches!(common[0], EKey::Piece(0, _, _)));
}

#[test]
fn emit_chart_attaches_full_trims_and_open_output_validates_continuously() {
    let model = shrunk_face(cuboid([0., 0., 0.], [2., 2., 2.]).unwrap(), 0);
    let output = emit_open(&model, 0);
    for coedge in &output.loops[output.faces[0].outer].coedges {
        let trim = coedge.trim().unwrap();
        assert_eq!(trim.curve_parameter, output.edges[coedge.edge].curve.domain());
        assert_eq!(trim.pcurve_parameter, coedge.pcurve.domain());
    }
    output.validate_continuous(10000).unwrap();
}

#[test]
fn same_chart_periodic_emit_and_lifts_do_not_wrap_or_follow_reversal() {
    let model = shrunk_face(cylinder(2., 4.).unwrap(), 0);
    let output = emit_open(&model, 0);
    output.validate_continuous(10000).unwrap();

    let source = cylinder(2., 4.).unwrap();
    let coedge = &source.loops[source.faces[0].outer].coedges[0];
    let mut pcurve = coedge.pcurve.clone();
    for point in &mut pcurve.control_points {
        point[0] += 1.25;
    }
    let forward = Coedge::new(coedge.edge, false, pcurve.clone());
    let reversed = Coedge::new(coedge.edge, true, pcurve);
    let a = derive_full_trim(&source.edges[coedge.edge], &forward, &source.faces[0].surface)
        .unwrap();
    let b = derive_full_trim(&source.edges[coedge.edge], &reversed, &source.faces[0].surface)
        .unwrap();
    assert_eq!(a.periodic_lift, b.periodic_lift);
    assert_eq!(forward.pcurve.control_points[0][0], coedge.pcurve.control_points[0][0] + 1.25);
}
