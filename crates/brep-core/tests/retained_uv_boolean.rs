use brep_core::{analysis::mass_properties, boolean_robustness::{classify_boolean_operand, BooleanOperandKind}, cuboid, retained_uv_boolean::{
        boolean_same_surface_uv_retained, boolean_same_surface_uv_retained_with_limits,
        merge_same_surface_uv_faces, replace_face_with_uv_regions,
        verify_same_surface_uv_retained,
    }, same_surface_uv_boolean::{SameSurfaceUvLimits, UvBooleanOperation, UvRegion}, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{curve::line, surface::Surface};

fn surface() -> Surface {
    Surface {
        degree_u: 1, degree_v: 1,
        knots_u: vec![-10., -10., 10., 10.], knots_v: vec![-10., -10., 10., 10.],
        control_points: vec![
            vec![vec![-10., -10., 0.], vec![-10., 10., 0.]],
            vec![vec![10., -10., 0.], vec![10., 10., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2], periodic_u: false, periodic_v: false,
    }
}

fn face_model_with_surface(s: Surface, loops: &[Vec<Vec<[f64; 2]>>]) -> Model {
    let mut model = Model(brep_topology::Model {
        vertices: vec![], edges: vec![], loops: vec![], faces: vec![],
        shells: vec![], bodies: vec![], tolerance_mm: 1e-7,
    }, TopologyIds::default());
    let evaluate = |p: [f64; 2]| {
        let uv = if s.periodic_u || s.periodic_v {
            nurbs_core::periodic_trim::wrapped_surface_parameters(&s, p).unwrap()
        } else { p };
        s.evaluate(uv[0], uv[1]).unwrap().point
    };
    let mut wires = vec![];
    for loop_ in loops {
        let mut coedges = vec![];
        for points in loop_ {
            let first = model.vertices.len();
            for p in points {
                let xyz = evaluate(*p);
                model.vertices.push(Vertex { point: xyz.try_into().unwrap() });
            }
            for i in 0..points.len() {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                let edge = model.edges.len();
                let pa = evaluate(a);
                let pb = evaluate(b);
                model.edges.push(Edge { degenerate: false, vertices: [first + i, first + (i + 1) % points.len()], curve: line(pa.to_vec(), pb.to_vec()) });
                coedges.push(Coedge::new(edge, false, line(a.to_vec(), b.to_vec())));
            }
        }
        wires.push(model.loops.len());
        model.loops.push(Loop { coedges });
    }
    model.faces.push(Face { surface: s, outer: wires[0], holes: wires[1..].to_vec() });
    model.shells.push(Shell { faces: vec![FaceUse { face: 0, reversed: false }], closed: false });
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}

fn face_model(loops: &[Vec<Vec<[f64; 2]>>]) -> Model { face_model_with_surface(surface(), loops) }
fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}
fn ring_area(loop_: &[[f64; 2]]) -> f64 {
    loop_.iter().zip(loop_.iter().cycle().skip(1)).map(|(a,b)| a[0]*b[1]-a[1]*b[0]).sum::<f64>() / 2.
}
fn model_area(model: &Model) -> f64 {
    model.faces.iter().map(|face| {
        let outer = model.loop_uv(face.outer, 1).unwrap();
        ring_area(&outer) - face.holes.iter().map(|&hole| ring_area(&model.loop_uv(hole, 1).unwrap()).abs()).sum::<f64>()
    }).sum()
}
fn run(a: &Model, b: &Model, op: UvBooleanOperation) -> Model {
    boolean_same_surface_uv_retained(a, 0, b, 0, None, op).unwrap().into_model()
}

#[test]
fn retained_all_operations_holes_lineage_and_replay() {
    let a = face_model(&[vec![rectangle(0., 0., 4., 4.)], vec![rectangle(1., 1., 3., 3.).into_iter().rev().collect()]]);
    let b = face_model(&[vec![rectangle(2., -1., 5., 5.)]]);
    for (op, expected) in [(UvBooleanOperation::Union, 24.), (UvBooleanOperation::Intersection, 6.), (UvBooleanOperation::Difference, 6.), (UvBooleanOperation::Xor, 18.)] {
        let result = boolean_same_surface_uv_retained(&a, 0, &b, 0, None, op).unwrap();
        verify_same_surface_uv_retained(&a, 0, &b, 0, &result).unwrap();
        let model = result.model();
        model.validate_continuous(100_000).unwrap();
        assert_eq!(classify_boolean_operand(model).unwrap(), BooleanOperandKind::Sheet);
        assert!(model.bodies.is_empty() && model.shells.iter().all(|s| !s.closed));
        assert!((model_area(model) - expected).abs() < 1e-7, "{op:?}");
        assert_eq!(result.certificate().source_faces, [a.1.faces[0], b.1.faces[0]]);
        assert_eq!(result.certificate().result_faces, model.1.faces);
        assert!(model.1.lineage.iter().any(|r| r.operation == "merge" && r.parents.len() == 2));
    }
}

#[test]
fn nonplanar_nurbs_segments_are_exact_lifts_not_chords() {
    let mut s = surface();
    s.control_points[1][1][2] = 2.;
    let a = face_model_with_surface(s.clone(), &[vec![rectangle(0., 0., 2., 2.)]]);
    let b = face_model_with_surface(s, &[vec![rectangle(1., 1., 3., 3.)]]);
    let result = run(&a, &b, UvBooleanOperation::Union);
    result.validate_continuous(100_000).unwrap();
    assert!(result.edges.iter().any(|e| {
        if e.curve.degree != 2 { return false; }
        let mid = e.curve.evaluate(0.5).unwrap().point;
        let end0 = &e.curve.control_points[0];
        let end1 = e.curve.control_points.last().unwrap();
        (0..3).any(|i| mid[i] != (end0[i] + end1[i]) / 2.)
    }));
}

#[test]
fn periodic_lifted_loop_is_retained_and_closes_on_surface() {
    let periodic = Surface {
        degree_u: 1, degree_v: 1,
        knots_u: vec![-1., 0., 1., 2., 3.], knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 3], periodic_u: true, periodic_v: false,
    };
    let a = face_model_with_surface(periodic.clone(), &[vec![rectangle(1.8, 0., 2.2, 1.)]]);
    let result = run(&a, &a, UvBooleanOperation::Union);
    result.validate_continuous(100_000).unwrap();
    assert_eq!(result.faces.len(), 1);
    let uv = result.loop_uv(result.faces[0].outer, 1).unwrap();
    assert!(uv.iter().any(|p| p[0] > 2.));
    assert!((model_area(&result) - 0.4).abs() < 1e-7);
}

#[test]
fn closed_solid_split_merge_preserves_volume_manifold_and_lineage() {
    let solid = cuboid([0.; 3], [2., 1., 1.]).unwrap();
    let top = solid.faces.iter().position(|f| f.surface.control_points.iter().flatten().all(|p| p[2] == 1.)).unwrap();
    let uv = solid.loops[solid.faces[top].outer].coedges.iter().map(|c| c.pcurve.control_points[0].clone()).collect::<Vec<_>>();
    let uv = uv.into_iter().map(|p| [p[0], p[1]]).collect::<Vec<_>>();
    let lo = [uv.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min), uv.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min)];
    let hi = [uv.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max), uv.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max)];
    let mid = (lo[0] + hi[0]) / 2.;
    let parts = vec![
        UvRegion { outer: rectangle(lo[0], lo[1], mid, hi[1]), holes: vec![] },
        UvRegion { outer: rectangle(mid, lo[1], hi[0], hi[1]), holes: vec![] },
    ];
    let split = replace_face_with_uv_regions(&solid, top, &parts, SameSurfaceUvLimits::default()).unwrap();
    assert_eq!(split.faces.len(), solid.faces.len() + 1);
    assert_eq!(split.shells[0].closed, true);
    assert!(split.loops.iter().flat_map(|w| &w.coedges).fold(vec![0; split.edges.len()], |mut n,c| {n[c.edge]+=1;n}).into_iter().all(|n| n == 2));
    let before = mass_properties(&split, 1e-8, 100_000).unwrap().signed_volume_mm3;
    assert!((before - 2.).abs() < 1e-7);
    let selected = split.shells[0].faces.iter().filter(|u| split.faces[u.face].surface == solid.faces[top].surface).map(|u| u.face).collect::<Vec<_>>();
    assert_eq!(selected.len(), 2);
    let merged = merge_same_surface_uv_faces(&split, &selected).unwrap();
    assert_eq!(merged.faces.len(), solid.faces.len());
    assert!(merged.loops.iter().flat_map(|w| &w.coedges).fold(vec![0; merged.edges.len()], |mut n,c| {n[c.edge]+=1;n}).into_iter().all(|n| n == 2));
    let after = mass_properties(&merged, 1e-8, 100_000).unwrap().signed_volume_mm3;
    assert!((after - before).abs() < 1e-7);
    assert!(merged.1.lineage.iter().any(|r| r.operation == "merge" && r.parents.len() == 2));
}

#[test]
fn retained_limits_nan_and_opposite_sense_fail_atomically() {
    let a = face_model(&[vec![rectangle(0., 0., 2., 2.)]]);
    let b = face_model(&[vec![rectangle(1., 0., 3., 2.)]]);
    let snapshots = [a.clone(), b.clone()];
    let error = boolean_same_surface_uv_retained_with_limits(&a, 0, &b, 0, None, UvBooleanOperation::Union, SameSurfaceUvLimits { max_segments: 3, ..Default::default() }).unwrap_err();
    assert_eq!(error.code, "BREP_RESOURCE_LIMIT");
    assert_eq!([a.clone(), b.clone()], snapshots);
    let mut opposite = b.clone();
    opposite.shells[0].faces[0].reversed = true;
    assert!(boolean_same_surface_uv_retained(&a, 0, &opposite, 0, None, UvBooleanOperation::Union).is_err());
    let bad = UvRegion { outer: vec![[0.,0.],[f64::NAN,0.],[1.,1.]], holes: vec![] };
    assert!(brep_core::retained_uv_boolean::build_uv_region_sheet(&a.faces[0].surface, &[bad], false, 1e-7, SameSurfaceUvLimits::default()).is_err());
}
