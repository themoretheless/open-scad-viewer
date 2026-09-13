use brep_core::{
    Model,
    analysis::mass_properties,
    prism::{extrude, recognize},
};
use nurbs_core::curve::Curve;
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}
fn rectangle(a: [f64; 2], b: [f64; 2]) -> Vec<Curve> {
    let points = [a, [b[0], a[1]], b, [a[0], b[1]]];
    (0..4)
        .map(|i| line(points[i], points[(i + 1) % 4]))
        .collect()
}
fn circle(center: [f64; 2], radius: f64, ccw: bool) -> Vec<Curve> {
    let q = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
    let mut arcs: Vec<_> = (0..4)
        .map(|i| {
            let (a, b) = (q[i], q[(i + 1) % 4]);
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![
                    vec![center[0] + radius * a[0], center[1] + radius * a[1]],
                    vec![
                        center[0] + radius * (a[0] + b[0]),
                        center[1] + radius * (a[1] + b[1]),
                    ],
                    vec![center[0] + radius * b[0], center[1] + radius * b[1]],
                ],
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            }
        })
        .collect();
    if !ccw {
        arcs.reverse();
        arcs = arcs.into_iter().map(|c| c.reverse().unwrap()).collect();
    }
    arcs
}
fn roundtrip(model: &Model) {
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    let json = value_codec::to_string(model).unwrap();
    let decoded: Model = value_codec::from_str(&json).unwrap();
    decoded.validate().unwrap();
    assert_eq!(json, value_codec::to_string(&decoded).unwrap());
}
#[test]
fn circular_holes_preserve_curves_and_exact_mass() {
    let loops = vec![circle([0., 0.], 3., true), circle([0., 0.], 1., false)];
    let model = extrude(&loops, -2., 3.).unwrap();
    roundtrip(&model);
    assert_eq!((model.bodies.len(), model.faces.len()), (1, 10));
    let prism = recognize(&model).unwrap().unwrap();
    assert_eq!((prism.loops.len(), prism.z_min, prism.z_max), (2, -2., 3.));
    let mass = mass_properties(&model, 1e-7, 200_000).unwrap();
    assert!((mass.signed_volume_mm3 - 40. * std::f64::consts::PI).abs() < 1e-5);
    assert!(
        model
            .edges
            .iter()
            .filter(|e| e.curve.degree == 2)
            .all(|e| e.curve.weights[1] == std::f64::consts::FRAC_1_SQRT_2)
    );
}
#[test]
fn unordered_nested_islands_and_disconnected_outers_form_correct_bodies() {
    let loops = vec![
        circle([0., 0.], 3., false),
        rectangle([8., 0.], [10., 2.]),
        circle([0., 0.], 5., true),
        circle([0., 0.], 1., true),
    ];
    let model = extrude(&loops, 0., 2.).unwrap();
    roundtrip(&model);
    assert_eq!(model.bodies.len(), 3);
    assert_eq!(recognize(&model).unwrap().unwrap().loops.len(), 4);
    let mass = mass_properties(&model, 1e-7, 300_000).unwrap();
    assert!((mass.signed_volume_mm3 - (34. * std::f64::consts::PI + 8.)).abs() < 1e-5);
}
#[test]
fn knot_domains_and_rational_line_parameterization_are_retained() {
    let mut outer = circle([4., -7.], 3., true);
    for curve in &mut outer {
        for k in &mut curve.knots {
            *k = 2. + 5. * *k;
        }
        for w in &mut curve.weights {
            *w *= 3.;
        }
    }
    let model = extrude(&[outer], 11., 15.).unwrap();
    roundtrip(&model);
    assert!(recognize(&model).unwrap().is_some());
    let mut weighted = rectangle([0., 0.], [3., 4.]);
    for curve in &mut weighted {
        curve.weights[1] = 2.;
    }
    let model = extrude(&[weighted], 0., 5.).unwrap();
    roundtrip(&model);
    assert!(recognize(&model).unwrap().is_some());
    let mass = mass_properties(&model, 1e-7, 200_000).unwrap();
    assert!((mass.signed_volume_mm3 - 60.).abs() < 1e-5);
}
#[test]
fn recognizer_proves_existing_primitives_and_translations() {
    for mut model in [
        brep_core::cylinder(3., 5.).unwrap(),
        brep_core::tube(3., 1., 5.).unwrap(),
        brep_core::cuboid([0., 0., 0.], [3., 4., 5.]).unwrap(),
        brep_core::extrude_polygon(
            &[[0., 0.], [3., 0.], [3., 1.], [1., 1.], [1., 4.], [0., 4.]],
            0.,
            5.,
        )
        .unwrap(),
    ] {
        assert!(
            recognize(&model).unwrap().is_some(),
            "original {} faces",
            model.faces.len()
        );
        let delta = [7., -9., 11.];
        for vertex in &mut model.vertices {
            for i in 0..3 {
                vertex.point[i] += delta[i];
            }
        }
        for edge in &mut model.edges {
            for p in &mut edge.curve.control_points {
                for i in 0..3 {
                    p[i] += delta[i];
                }
            }
        }
        for face in &mut model.faces {
            for p in face.surface.control_points.iter_mut().flatten() {
                for i in 0..3 {
                    p[i] += delta[i];
                }
            }
        }
        model.rebuild_topology_ids();
        let recognized = recognize(&model).unwrap().unwrap();
        assert_eq!((recognized.z_min, recognized.z_max), (11., 16.));
    }
}
#[test]
fn recognizer_rejects_deformed_nets_and_trim_geometry_despite_loose_validation_tolerance() {
    let mut bulged = brep_core::cylinder(3., 5.).unwrap();
    bulged.tolerance_mm = 1e-3;
    bulged.faces[0].surface.control_points[1][1][0] += 1e-5;
    bulged.rebuild_topology_ids();
    bulged.validate().unwrap();
    assert!(recognize(&bulged).unwrap().is_none());
    let mut trim = brep_core::cylinder(3., 5.).unwrap();
    trim.tolerance_mm = 1e-3;
    let wire = trim.faces[4].outer;
    trim.loops[wire].coedges[0].pcurve.control_points[1][0] += 1e-6;
    trim.rebuild_topology_ids();
    trim.validate().unwrap();
    assert!(recognize(&trim).unwrap().is_none());
    let mut inverted = brep_core::cylinder(3., 5.).unwrap();
    for usage in &mut inverted.shells[0].faces {
        usage.reversed ^= true;
    }
    inverted.rebuild_topology_ids();
    inverted.validate().unwrap();
    assert!(recognize(&inverted).unwrap().is_none());
    assert!(
        recognize(&brep_core::frustum(3., 2., 5.).unwrap())
            .unwrap()
            .is_none()
    );
    assert!(
        recognize(&brep_core::sphere(3.).unwrap())
            .unwrap()
            .is_none()
    );
}
#[test]
fn input_resource_and_orientation_contracts_are_explicit() {
    let empty = extrude(&[], 0., 2.).unwrap();
    empty.validate().unwrap();
    assert!(empty.faces.is_empty());
    assert!(recognize(&empty).unwrap().is_none());
    assert_eq!(
        extrude(&vec![rectangle([0., 0.], [1., 1.]); 65], 0., 2.)
            .unwrap_err()
            .code,
        "BREP_RESOURCE_LIMIT"
    );
    assert!(extrude(&[circle([0., 0.], 1., false)], 0., 2.).is_err());
    assert!(
        extrude(
            &[circle([0., 0.], 2., true), circle([5., 0.], 1., false)],
            0.,
            2.
        )
        .is_err()
    );
    assert!(extrude(&[rectangle([0., 0.], [1., 1.])], 2., 2.).is_err());
    let mut open = rectangle([0., 0.], [1., 1.]);
    open.pop();
    assert!(extrude(&[open], 0., 2.).is_err());
}
