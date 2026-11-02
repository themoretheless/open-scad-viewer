use brep_core::{
    Model,
    feature_ops::edge_transition::{self, Limits, RadiusPatch},
    watertight,
};
use nurbs_core::{
    curve::Curve,
    curve_surface_agreement as agreement,
    surface::{Axis, Surface},
    surface_join::Decision,
};
fn original(alpha: f64) -> Model {
    original_anisotropic(alpha, 1.)
}
fn original_anisotropic(alpha: f64, anisotropy: f64) -> Model {
    let base = brep_core::cuboid([0.; 3], [4., 4., 3.]).unwrap();
    let mut model = base.clone();
    let h =
        |x: f64, y: f64| 3. + alpha * ((x - 3.5).powi(2) + anisotropy * (y - 0.5).powi(2) - 0.25);
    for v in &mut model.vertices {
        v.point[2] = v.point[2] / 3. * h(v.point[0], v.point[1]);
    }
    for (f, old) in model.faces.iter_mut().zip(&base.faces) {
        let s = &old.surface;
        let a = s.evaluate(0., 0.).unwrap().point;
        let u = s.evaluate(1., 0.).unwrap().point;
        let v = s.evaluate(0., 1.).unwrap().point;
        let pu = if a[0] != u[0] || a[1] != u[1] { 2 } else { 1 };
        let pv = if a[0] != v[0] || a[1] != v[1] { 2 } else { 1 };
        let square = |start: f64, end: f64, center: f64, index: usize| {
            [
                (start - center).powi(2),
                (start - center) * (end - center),
                (end - center).powi(2),
            ][index]
        };
        let points = (0..=pu)
            .map(|i| {
                (0..=pv)
                    .map(|j| {
                        let p = s
                            .evaluate(i as f64 / pu as f64, j as f64 / pv as f64)
                            .unwrap()
                            .point;
                        let x2 = if a[0] != u[0] {
                            square(a[0], u[0], 3.5, i)
                        } else if a[0] != v[0] {
                            square(a[0], v[0], 3.5, j)
                        } else {
                            (a[0] - 3.5).powi(2)
                        };
                        let y2 = if a[1] != u[1] {
                            square(a[1], u[1], 0.5, i)
                        } else if a[1] != v[1] {
                            square(a[1], v[1], 0.5, j)
                        } else {
                            (a[1] - 0.5).powi(2)
                        };
                        vec![
                            p[0],
                            p[1],
                            p[2] / 3. * (3. + alpha * (x2 + anisotropy * y2 - 0.25)),
                        ]
                    })
                    .collect()
            })
            .collect();
        f.surface = Surface {
            degree_u: pu,
            degree_v: pv,
            knots_u: [vec![0.; pu + 1], vec![1.; pu + 1]].concat(),
            knots_v: [vec![0.; pv + 1], vec![1.; pv + 1]].concat(),
            control_points: points,
            weights: vec![vec![1.; pv + 1]; pu + 1],
            periodic_u: false,
            periodic_v: false,
        };
    }
    for edge in 0..model.edges.len() {
        let (face, c) = model
            .faces
            .iter()
            .find_map(|f| {
                model.loops[f.outer]
                    .coedges
                    .iter()
                    .find(|c| c.edge == edge)
                    .map(|c| (f, c))
            })
            .unwrap();
        let mut endpoints = model.edges[edge].vertices.map(|v| model.vertices[v].point);
        if c.reversed {
            endpoints.reverse();
        }
        let lift = agreement::lift(
            &c.pcurve,
            &face.surface,
            Some(endpoints),
            model.tolerance_mm / 4.,
            100000,
        )
        .unwrap();
        let curve = lift.curve.unwrap();
        model.edges[edge].curve = if c.reversed {
            curve.reverse().unwrap()
        } else {
            curve
        };
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
fn patch() -> (Surface, [Curve; 4], Curve, Curve) {
    let controls = [[3.5, 0.], [4., 0.], [4., 0.5]];
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: controls
            .into_iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 3.]])
            .collect(),
        weights: vec![vec![1.; 2], vec![0.5f64.sqrt(); 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    let corners = [[0., 1.], [0., 0.], [1., 0.], [1., 1.]];
    let pc = std::array::from_fn(|i| {
        Curve::from_polyline(vec![corners[i].to_vec(), corners[(i + 1) % 4].to_vec()]).unwrap()
    });
    let centers = Curve::from_polyline(vec![vec![3.5, 0.5, 0.], vec![3.5, 0.5, 3.]]).unwrap();
    let radius = Curve::from_polyline(vec![vec![0.5, 0.], vec![0.5, 0.]]).unwrap();
    (s, pc, centers, radius)
}
fn edge(source: &Model) -> usize {
    source
        .edges
        .iter()
        .position(|e| {
            e.vertices.iter().all(|&v| {
                let p = source.vertices[v].point;
                (p[0] - 4.).hypot(p[1]) < 1e-12
            })
        })
        .unwrap()
}
#[test]
fn curved_original_end_face_is_retrimmed_and_closed_with_the_radius_patch() {
    let alpha = 0.125;
    let source = original(alpha);
    let before = source.clone();
    assert!(brep_core::prism::recognize(&source).unwrap().is_none());
    let (s, pc, centers, radius) = patch();
    let out = edge_transition::insert_radius_patch(
        &source,
        edge(&source),
        RadiusPatch {
            surface: &s,
            contour: pc.each_ref(),
            centers: &centers,
            radius: &radius,
            spine_axis: Axis::V,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(source, before);
    assert_eq!(out.model().faces.len(), 7);
    assert!(out.audit().ok);
    assert_eq!(
        out.closure().closure.status,
        watertight::Closure::WithinTolerance
    );
    assert_eq!(out.source_edge(), source.1.edges[edge(&source)]);
    assert_eq!(out.tangent_seams().len(), 2);
    assert!(
        out.tangent_seams()
            .iter()
            .all(|p| p.positions.proven && p.decision == Decision::WithinTolerance)
    );
    assert!(out.radius_certificate().error_upper() <= source.tolerance_mm);
    for id in out.source_ends() {
        let original = source.1.faces.iter().position(|v| *v == id).unwrap();
        assert!(
            out.model()
                .faces
                .iter()
                .any(|f| f.surface == source.faces[original].surface)
        );
    }
    for v in out.corner_vertices() {
        assert_eq!(
            out.model()
                .edges
                .iter()
                .filter(|e| e.vertices.contains(&v))
                .count(),
            3
        );
    }
    let actual = brep_core::analysis::mass_properties(out.model(), 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3;
    let pi = std::f64::consts::PI;
    let expected =
        48. + alpha * 332. / 3. - 3. * 0.25 * (1. - pi / 4.) - alpha * 0.0625 * (pi / 8. - 1. / 3.);
    assert!(
        (actual - expected).abs() < 1e-5,
        "{actual} versus {expected}"
    );
    let (cells, work) = out.work();
    assert!(cells <= 100000 && work <= 1000000);
}
#[test]
fn ordinary_planar_ends_use_the_same_owned_four_face_pipeline() {
    let source = original(0.);
    let (s, pc, centers, radius) = patch();
    let out = edge_transition::insert_radius_patch(
        &source,
        edge(&source),
        RadiusPatch {
            surface: &s,
            contour: pc.each_ref(),
            centers: &centers,
            radius: &radius,
            spine_axis: Axis::V,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(out.model().faces.len(), 7);
    assert!(out.audit().ok);
}
#[test]
fn invalid_geometry_or_work_cannot_publish_a_partially_cut_body() {
    let source = original(0.125);
    let before = source.clone();
    let (s, pc, centers, radius) = patch();
    let make = || RadiusPatch {
        surface: &s,
        contour: pc.each_ref(),
        centers: &centers,
        radius: &radius,
        spine_axis: Axis::V,
    };
    assert!(edge_transition::insert_radius_patch(&source, 999, make(), Limits::default()).is_err());
    assert!(
        edge_transition::insert_radius_patch(
            &source,
            edge(&source),
            make(),
            Limits {
                cells: 256,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let mut shifted = s.clone();
    for p in shifted.control_points.iter_mut().flatten() {
        p[0] += 0.01;
    }
    assert!(
        edge_transition::insert_radius_patch(
            &source,
            edge(&source),
            RadiusPatch {
                surface: &shifted,
                contour: pc.each_ref(),
                centers: &centers,
                radius: &radius,
                spine_axis: Axis::V
            },
            Limits::default()
        )
        .is_err()
    );
    assert_eq!(source, before);
}

#[test]
fn rigid_placement_keeps_original_curved_end_ownership_and_radius() {
    let original = original(0.125);
    let selected = edge(&original);
    let matrix = [
        [0.36, -0.8, 0.48, 11.],
        [0.48, 0.6, 0.64, -7.],
        [-0.8, 0., 0.6, 5.],
        [0., 0., 0., 1.],
    ];
    let source = brep_core::transform::affine(&original, matrix).unwrap();
    let before = source.clone();
    let (mut s, pc, mut centers, radius) = patch();
    let transform = |p: &[f64]| {
        (0..3)
            .map(|i| matrix[i][3] + (0..3).map(|j| matrix[i][j] * p[j]).sum::<f64>())
            .collect()
    };
    for p in s.control_points.iter_mut().flatten() {
        *p = transform(p);
    }
    for p in &mut centers.control_points {
        *p = transform(p);
    }
    let out = edge_transition::insert_radius_patch(
        &source,
        selected,
        RadiusPatch {
            surface: &s,
            contour: pc.each_ref(),
            centers: &centers,
            radius: &radius,
            spine_axis: Axis::V,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(source, before);
    assert!(out.audit().ok);
    assert_eq!(out.model().faces.len(), 7);
    assert!(out.radius_certificate().error_upper() <= source.tolerance_mm);
}

#[test]
fn transposed_patch_coordinates_keep_the_same_closed_transition() {
    let source = original(0.125);
    let (s, pc, centers, radius) = patch();
    let s = nurbs_core::surface_edit::transpose(&s).unwrap();
    let mut contours = Vec::new();
    for i in [2, 1, 0, 3] {
        let mut p = pc[i].reverse().unwrap();
        for q in &mut p.control_points {
            q.swap(0, 1);
        }
        contours.push(p);
    }
    let out = edge_transition::insert_radius_patch(
        &source,
        edge(&source),
        RadiusPatch {
            surface: &s,
            contour: [&contours[0], &contours[1], &contours[2], &contours[3]],
            centers: &centers,
            radius: &radius,
            spine_axis: Axis::U,
        },
        Limits::default(),
    )
    .unwrap();
    assert!(out.audit().ok);
    assert_eq!(out.model().faces.len(), 7);
    assert_eq!(out.tangent_seams().len(), 2);
    assert!(out.radius_certificate().error_upper() <= source.tolerance_mm);
}

#[test]
fn automatic_round_constructs_contacts_from_only_edge_and_radius() {
    for anisotropy in [1., 0.] {
        let source = original_anisotropic(0.125, anisotropy);
        let before = source.clone();
        let out =
            edge_transition::round_straight_edge(&source, edge(&source), 0.5, Limits::default())
                .unwrap();
        assert_eq!(source, before);
        assert!(out.audit().ok);
        assert_eq!(out.model().faces.len(), 7);
        assert!(
            out.tangent_seams()
                .iter()
                .all(|r| r.decision == Decision::WithinTolerance)
        );
        assert!(out.radius_certificate().error_upper() <= source.tolerance_mm);
        assert!(out.work().0 <= 100000 && out.work().1 <= 1000000);
        if anisotropy == 0. {
            let actual = brep_core::analysis::mass_properties(out.model(), 1e-6, 2000000)
                .unwrap()
                .signed_volume_mm3;
            let pi = std::f64::consts::PI;
            let expected = 48. + 0.125 * 160. / 3.
                - 3. * 0.25 * (1. - pi / 4.)
                - 0.125 * 0.0625 * (-2. / 3. + 3. * pi / 16.);
            assert!(
                (actual - expected).abs() < 1e-5,
                "{actual} versus {expected}"
            );
            let skin = &out.model().faces[out.transition_face()];
            assert!(out.model().loops[skin.outer].coedges.iter().any(|c| {
                let p = &c.pcurve.control_points;
                p.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max)
                    - p.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min)
                    > 1e-3
                    && p.iter().any(|q| q[0] > 1e-3 && q[0] < 0.999)
            }));
        }
    }
}

#[test]
fn automatic_round_rejects_overlarge_radius_and_preserves_source() {
    let source = original_anisotropic(0.125, 0.);
    let before = source.clone();
    for radius in [0., -1., f64::NAN, 8.] {
        assert!(
            edge_transition::round_straight_edge(&source, edge(&source), radius, Limits::default())
                .is_err()
        );
    }
    let curved = source
        .edges
        .iter()
        .position(|e| {
            e.vertices.iter().all(|&v| source.vertices[v].point[2] > 2.)
                && (source.vertices[e.vertices[0]].point[0]
                    - source.vertices[e.vertices[1]].point[0])
                    .abs()
                    > 1.
        })
        .unwrap();
    assert!(edge_transition::round_straight_edge(&source, curved, 0.5, Limits::default()).is_err());
    let mut scarce = Limits::default();
    scarce.cells = 256;
    assert!(edge_transition::round_straight_edge(&source, edge(&source), 0.5, scarce).is_err());
    assert_eq!(source, before);
}

#[test]
fn automatic_round_handles_oblique_placement_and_nonright_support_angle() {
    for matrix in [
        [
            [0.36, -0.8, 0.48, 11.],
            [0.48, 0.6, 0.64, -7.],
            [-0.8, 0., 0.6, 5.],
            [0., 0., 0., 1.],
        ],
        [
            [1., 0.3, 0., 2.],
            [0., 1., 0., -1.],
            [0., 0., 1., 3.],
            [0., 0., 0., 1.],
        ],
    ] {
        let original = original_anisotropic(0.125, 0.);
        let selected = edge(&original);
        let source = brep_core::transform::affine(&original, matrix).unwrap();
        let before = source.clone();
        let out = edge_transition::round_straight_edge(&source, selected, 0.5, Limits::default())
            .unwrap();
        assert_eq!(source, before);
        assert!(out.audit().ok);
        assert_eq!(out.model().faces.len(), 7);
        assert_eq!(out.tangent_seams().len(), 2);
        assert!(out.radius_certificate().error_upper() <= source.tolerance_mm);
        for id in out.source_ends().into_iter().chain(out.source_supports()) {
            let face = source.1.faces.iter().position(|i| *i == id).unwrap();
            assert!(
                out.model()
                    .faces
                    .iter()
                    .any(|f| f.surface == source.faces[face].surface)
            );
        }
        assert!(out.work().0 <= 100000 && out.work().1 <= 1000000);
    }
}

#[test]
fn automatic_round_resolves_different_radii_on_the_same_curved_end_stock() {
    let source = original_anisotropic(0.125, 0.);
    let before = source.clone();
    for radius in [0.25, 1.] {
        let out =
            edge_transition::round_straight_edge(&source, edge(&source), radius, Limits::default())
                .unwrap();
        assert!(out.audit().ok);
        assert!(out.radius_certificate().error_upper() <= source.tolerance_mm);
        assert!(
            out.radius_certificate()
                .radius()
                .control_points
                .iter()
                .all(|p| p[0] == radius)
        );
        assert!(
            out.tangent_seams()
                .iter()
                .all(|r| r.regularity_proven && r.decision == Decision::WithinTolerance)
        );
    }
    assert_eq!(source, before);
}

#[test]
fn asymmetric_chamfer_closes_curved_ends_with_whole_saved_setback_proofs() {
    let alpha = 0.125;
    let source = original_anisotropic(alpha, 0.);
    let before = source.clone();
    let out = edge_transition::chamfer_straight_edge(
        &source,
        edge(&source),
        [0.5, 0.25],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(source, before);
    assert!(out.audit().ok);
    assert_eq!(out.model().faces.len(), 7);
    assert_eq!(
        out.closure().closure.status,
        watertight::Closure::WithinTolerance
    );
    for (side, distance) in [0.5, 0.25].into_iter().enumerate() {
        assert_eq!(out.setbacks()[side].len(), 1);
        let c = &out.setbacks()[side][0];
        assert_eq!(c.requested(), distance);
        assert_eq!(c.actual_curve(), &out.model().edges[c.edge_index()].curve);
        assert_eq!(c.original_curve(), &source.edges[edge(&source)].curve);
        assert!(c.error_upper() <= source.tolerance_mm);
        assert!(c.source_line_deviation() <= source.tolerance_mm / 8.);
        // Original shared-root binding is admitted within the source length
        // tolerance; the ACTUAL rail need not contain the ideal decimal value.
        assert!(
            c.distance_interval()[0] >= distance - source.tolerance_mm
                && c.distance_interval()[1] <= distance + source.tolerance_mm
        );
    }
    let p = out.setbacks()[0][0]
        .actual_curve()
        .evaluate(0.)
        .unwrap()
        .point;
    assert!((p[0] - 3.5).abs() < 1e-7 && p[1].abs() < 1e-7);
    let p = out.setbacks()[1][0]
        .actual_curve()
        .evaluate(0.)
        .unwrap()
        .point;
    assert!((p[0] - 4.).abs() < 1e-7 && (p[1] - 0.25).abs() < 1e-7);
    let actual = brep_core::analysis::mass_properties(out.model(), 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3;
    let (a, b) = (0.5, 0.25);
    let expected =
        48. + alpha * 160. / 3. - 3. * a * b / 2. - alpha * a * b * (a * a / 12. - a / 6.);
    assert!(
        (actual - expected).abs() < 1e-5,
        "{actual} versus {expected}"
    );
    assert!(out.work().0 <= 100000 && out.work().1 <= 1000000);
}

#[test]
fn chamfer_setback_proofs_survive_rotation_and_nonright_supports() {
    for matrix in [
        [
            [0.36, -0.8, 0.48, 11.],
            [0.48, 0.6, 0.64, -7.],
            [-0.8, 0., 0.6, 5.],
            [0., 0., 0., 1.],
        ],
        [
            [1., 0.3, 0., 2.],
            [0., 1., 0., -1.],
            [0., 0., 1., 3.],
            [0., 0., 0., 1.],
        ],
    ] {
        let original = original_anisotropic(0.125, 0.);
        let selected = edge(&original);
        let source = brep_core::transform::affine(&original, matrix).unwrap();
        let before = source.clone();
        let out = edge_transition::chamfer_straight_edge(
            &source,
            selected,
            [0.25, 0.5],
            Limits::default(),
        )
        .unwrap();
        assert_eq!(source, before);
        assert!(out.audit().ok);
        assert_eq!(out.model().faces.len(), 7);
        for proof in out.setbacks().iter().flatten() {
            assert!(proof.error_upper() <= source.tolerance_mm);
        }
        for id in out.source_ends().into_iter().chain(out.source_supports()) {
            let face = source.1.faces.iter().position(|i| *i == id).unwrap();
            assert!(
                out.model()
                    .faces
                    .iter()
                    .any(|f| f.surface == source.faces[face].surface)
            );
        }
        assert!(out.work().0 <= 100000 && out.work().1 <= 1000000);
    }
}

#[test]
fn chamfer_invalid_setbacks_and_exhaustion_publish_no_partial_body() {
    let source = original_anisotropic(0.125, 0.);
    let before = source.clone();
    for setbacks in [
        [0., 0.5],
        [-0.5, 0.5],
        [f64::NAN, 0.5],
        [0.5, f64::INFINITY],
        [8., 8.],
    ] {
        assert!(
            edge_transition::chamfer_straight_edge(
                &source,
                edge(&source),
                setbacks,
                Limits::default()
            )
            .is_err()
        );
    }
    let mut scarce = Limits::default();
    scarce.cells = 256;
    assert!(
        edge_transition::chamfer_straight_edge(&source, edge(&source), [0.5, 0.25], scarce)
            .is_err()
    );
    assert_eq!(source, before);
}
