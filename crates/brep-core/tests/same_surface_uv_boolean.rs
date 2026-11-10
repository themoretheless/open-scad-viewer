use brep_core::{boolean_robustness::{
        certify_nurbs_uv_equivalence, preclassify_coincident_faces, CoincidentFaceClass,
    }, cuboid, same_surface_uv_boolean::{
        boolean_same_surface_uv, boolean_same_surface_uv_with_limits,
        verify_same_surface_uv_boolean, SameSurfaceUvLimits, UvBooleanOperation,
    }, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{curve::line, surface::Surface};

fn surface() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-10., -10., 10., 10.],
        knots_v: vec![-10., -10., 10., 10.],
        control_points: vec![
            vec![vec![-10., -10., 0.], vec![-10., 10., 0.]],
            vec![vec![10., -10., 0.], vec![10., 10., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn face_model(loops: &[Vec<Vec<[f64; 2]>>]) -> Model {
    let s = surface();
    let mut model = Model(
        brep_topology::Model {
            vertices: vec![],
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    let mut wires = vec![];
    for loop_ in loops {
        let mut coedges = vec![];
        for points in loop_ {
            let first_vertex = model.vertices.len();
            for a in points.iter() {
                model.vertices.push(Vertex {
                    point: [a[0], a[1], 0.],
                });
            }
            for i in 0..points.len() {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                let edge = model.edges.len();
                model.edges.push(Edge {
                    degenerate: false,
                    vertices: [first_vertex + i, first_vertex + (i + 1) % points.len()],
                    curve: line(vec![a[0], a[1], 0.], vec![b[0], b[1], 0.]),
                });
                coedges.push(Coedge::new(edge, false, line(a.to_vec(), b.to_vec())));
            }
        }
        wires.push(model.loops.len());
        model.loops.push(Loop { coedges });
    }
    model.faces.push(Face {
        surface: s,
        outer: wires[0],
        holes: wires[1..].to_vec(),
    });
    model.shells.push(Shell {
        faces: vec![FaceUse {
            face: 0,
            reversed: false,
        }],
        closed: false,
    });
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}

fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn signed_area(loop_: &[[f64; 2]]) -> f64 {
    loop_
        .iter()
        .zip(loop_.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
        .sum::<f64>()
        / 2.
}

fn result_area(result: &brep_core::same_surface_uv_boolean::SameSurfaceUvBoolean) -> f64 {
    result
        .regions()
        .iter()
        .map(|r| signed_area(&r.outer) - r.holes.iter().map(|h| signed_area(h).abs()).sum::<f64>())
        .sum()
}

fn run(
    a: &Model,
    b: &Model,
    operation: UvBooleanOperation,
) -> brep_core::same_surface_uv_boolean::SameSurfaceUvBoolean {
    boolean_same_surface_uv(
        &a.faces[0].surface,
        a,
        0,
        &b.faces[0].surface,
        b,
        0,
        None,
        operation,
    )
    .unwrap()
}

#[test]
fn overlapping_regions_support_all_operations_and_replay() {
    let a = face_model(&[vec![rectangle(0., 0., 2., 2.)]]);
    let b = face_model(&[vec![rectangle(1., 1., 3., 3.)]]);
    for (op, expected) in [
        (UvBooleanOperation::Union, 7.),
        (UvBooleanOperation::Intersection, 1.),
        (UvBooleanOperation::Difference, 3.),
        (UvBooleanOperation::Xor, 6.),
    ] {
        let result = run(&a, &b, op);
        assert!(
            (result_area(&result) - expected).abs() < 1e-7,
            "{op:?}: {:?}",
            result.regions()
        );
        verify_same_surface_uv_boolean(
            &a.faces[0].surface,
            &a,
            0,
            &b.faces[0].surface,
            &b,
            0,
            &result,
        )
        .unwrap();
        for region in result.regions() {
            assert!(signed_area(&region.outer) > 0.);
            assert!(region.holes.iter().all(|h| signed_area(h) < 0.));
        }
    }
}

#[test]
fn holes_are_subtracted_and_can_generate_multiple_regions() {
    let a = face_model(&[
        vec![rectangle(0., 0., 4., 4.)],
        vec![rectangle(1., 1., 3., 3.).into_iter().rev().collect()],
    ]);
    let b = face_model(&[vec![rectangle(2., -1., 5., 5.)]]);
    assert!((result_area(&run(&a, &b, UvBooleanOperation::Intersection)) - 6.).abs() < 1e-7);
    assert!((result_area(&run(&a, &b, UvBooleanOperation::Difference)) - 6.).abs() < 1e-7);
    assert!((result_area(&run(&a, &b, UvBooleanOperation::Union)) - 24.).abs() < 1e-7);
}

#[test]
fn exact_normalized_nurbs_equivalence_maps_transposition_and_reversals() {
    let a = cuboid([0.; 3], [1.; 3]).unwrap();
    let mut b = a.clone();
    b.faces[0].surface.knots_u = b.faces[0]
        .surface
        .knots_u
        .iter()
        .map(|k| 2. + 3. * k)
        .collect();
    b.faces[0].surface.weights = b.faces[0]
        .surface
        .weights
        .iter()
        .map(|r| r.iter().map(|w| 7. * w).collect())
        .collect();
    let map = certify_nurbs_uv_equivalence(&a.faces[0].surface, &b.faces[0].surface).unwrap();
    assert!(matches!(
        preclassify_coincident_faces(&a, 0, &b, 0).unwrap(),
        CoincidentFaceClass::EquivalentNurbs(_)
    ));
    let transposed = nurbs_core::surface_edit::transpose(&b.faces[0].surface).unwrap();
    b.faces[0].surface = transposed;
    let other = certify_nurbs_uv_equivalence(&a.faces[0].surface, &b.faces[0].surface).unwrap();
    assert_ne!(map, other);
    assert!(other.transpose || other.reverse_u || other.reverse_v);
}

#[test]
fn periodic_domain_stays_in_one_certified_unwrapped_chart() {
    let periodic = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., 0., 1., 2., 3.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: true,
        periodic_v: false,
    };
    periodic.validate().unwrap();
    let mut a = face_model(&[vec![rectangle(0., 0., 1., 1.)]]);
    a.faces[0].surface = periodic;
    let mut b = face_model(&[vec![rectangle(0.5, 0., 1.5, 1.)]]);
    b.faces[0].surface = a.faces[0].surface.clone();
    let result = boolean_same_surface_uv(
        &a.faces[0].surface,
        &a,
        0,
        &a.faces[0].surface,
        &b,
        0,
        None,
        UvBooleanOperation::Union,
    )
    .unwrap();
    assert!((result_area(&result) - 1.5).abs() < 1e-7);
    let mut seam = face_model(&[vec![rectangle(1.8, 0., 2.2, 1.)]]);
    seam.faces[0].surface = a.faces[0].surface.clone();
    let seam_other = seam.clone();
    let result = run(&seam, &seam_other, UvBooleanOperation::Union);
    assert!((result_area(&result) - 0.4).abs() < 1e-7);
    assert!(result.regions()[0].outer.iter().any(|p| p[0] > 2.));
}

#[test]
fn nan_and_nonlinear_trims_are_rejected_atomically() {
    let a = face_model(&[vec![rectangle(0., 0., 1., 1.)]]);
    let snapshot = a.clone();
    let mut b = face_model(&[vec![rectangle(0., 0., 1., 1.)]]);
    b.loops[0].coedges[0].pcurve.control_points[0][0] = f64::NAN;
    assert!(run_or_err(&a, &b).is_err());
    assert_eq!(a, snapshot);
    let mut c = face_model(&[vec![rectangle(0., 0., 1., 1.)]]);
    c.loops[0].coedges[0].pcurve.degree = 2;
    assert!(run_or_err(&a, &c).is_err());
}

fn run_or_err(
    a: &Model,
    b: &Model,
) -> nurbs_core::Result<brep_core::same_surface_uv_boolean::SameSurfaceUvBoolean> {
    boolean_same_surface_uv(
        &a.faces[0].surface,
        a,
        0,
        &b.faces[0].surface,
        b,
        0,
        None,
        UvBooleanOperation::Union,
    )
}

#[test]
fn property_union_intersection_and_difference_preserve_area_identity() {
    let rectangles = [
        [0., 0., 2., 2.],
        [0.5, -0.5, 2.5, 1.5],
        [-1., 0.5, 1., 3.],
        [0.25, 0.25, 0.75, 0.75],
    ];
    for x in rectangles {
        let a = face_model(&[vec![rectangle(0., 0., 2., 2.)]]);
        let b = face_model(&[vec![rectangle(x[0], x[1], x[2], x[3])]]);
        let u = result_area(&run(&a, &b, UvBooleanOperation::Union));
        let i = result_area(&run(&a, &b, UvBooleanOperation::Intersection));
        let d = result_area(&run(&a, &b, UvBooleanOperation::Difference));
        let symmetric = result_area(&run(&a, &b, UvBooleanOperation::Xor));
        assert!((u + i - 4. - (x[2] - x[0]) * (x[3] - x[1])).abs() < 1e-7);
        assert!((d + i - 4.).abs() < 1e-7);
        assert!((symmetric - u + i).abs() < 1e-7);
    }
}

#[test]
fn disjoint_inputs_create_multiple_deterministic_regions() {
    let a = face_model(&[vec![rectangle(0., 0., 1., 1.)]]);
    let b = face_model(&[vec![rectangle(2., 0., 3., 1.)]]);
    let result = run(&a, &b, UvBooleanOperation::Union);
    assert_eq!(result.regions().len(), 2);
    assert!((result_area(&result) - 2.).abs() < 1e-12);
    assert_eq!(result, run(&a, &b, UvBooleanOperation::Union));
}

#[test]
fn reversed_face_use_does_not_reverse_uv_pcurves_twice() {
    let a = face_model(&[vec![rectangle(0., 0., 2., 2.)]]);
    let mut b = face_model(&[vec![rectangle(1., 0., 3., 2.)]]);
    b.shells[0].faces[0].reversed = true;
    b.loops[0]
        .coedges
        .iter_mut()
        .for_each(|c| c.reversed = true);
    let result = run(&a, &b, UvBooleanOperation::Union);
    assert!((result_area(&result) - 6.).abs() < 1e-12);
}

#[test]
fn resource_limits_are_replayed_and_work_is_actual() {
    let a = face_model(&[vec![rectangle(0., 0., 2., 2.)]]);
    let b = face_model(&[vec![rectangle(1., 1., 3., 3.)]]);
    let before = (a.clone(), b.clone());
    let limited = boolean_same_surface_uv_with_limits(
        &a.faces[0].surface,
        &a,
        0,
        &b.faces[0].surface,
        &b,
        0,
        None,
        UvBooleanOperation::Union,
        SameSurfaceUvLimits {
            tolerance_uv: 1e-9,
            max_segments: 2048,
            max_work: 1,
        },
    )
    .unwrap_err();
    assert_eq!(limited.code, "BREP_RESOURCE_LIMIT");
    assert_eq!((a, b), before);
    let a = face_model(&[vec![rectangle(0., 0., 2., 2.)]]);
    let b = face_model(&[vec![rectangle(1., 1., 3., 3.)]]);
    let result = run(&a, &b, UvBooleanOperation::Union);
    assert!(
        result.certificate().work > 0
            && result.certificate().work < result.certificate().limits.max_work
    );
    let mut forged = result.certificate().clone();
    forged.work += 1;
    let forged = brep_core::same_surface_uv_boolean::SameSurfaceUvBoolean::from_parts(
        result.regions().to_vec(),
        forged,
    );
    assert!(verify_same_surface_uv_boolean(
        &a.faces[0].surface,
        &a,
        0,
        &b.faces[0].surface,
        &b,
        0,
        &forged
    )
    .is_err());
}
