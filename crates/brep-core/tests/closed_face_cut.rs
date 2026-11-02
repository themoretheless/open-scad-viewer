use brep_core::{
    Model,
    face_cut::{self, Segment},
};
use nurbs_core::{curve::Curve, surface::Surface};

fn rectangle(lo: [f64; 2], hi: [f64; 2]) -> Vec<Curve> {
    let p = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]];
    (0..4)
        .map(|i| {
            let mut c = Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap();
            let a = 3. * i as f64 - 7.;
            c.knots = vec![a, a, a + 2., a + 2.];
            c
        })
        .collect()
}
fn path(surface: &Surface, uv: &[Curve]) -> Vec<Segment> {
    // The native fixtures use an affine cuboid chart. Extrapolate its authored
    // plane for the outside-contour refusal case instead of asking the surface
    // evaluator to accept out-of-domain parameters.
    let origin = surface.evaluate(0., 0.).unwrap().point;
    let u = surface.evaluate(1., 0.).unwrap().point;
    let v = surface.evaluate(0., 1.).unwrap().point;
    uv.iter()
        .map(|c| Segment {
            curve: Curve {
                control_points: c
                    .control_points
                    .iter()
                    .map(|p| {
                        (0..3)
                            .map(|k| {
                                origin[k] + p[0] * (u[k] - origin[k]) + p[1] * (v[k] - origin[k])
                            })
                            .collect()
                    })
                    .collect(),
                ..c.clone()
            },
            pcurve: c.clone(),
        })
        .collect()
}
fn cube() -> (Model, usize) {
    let m = brep_core::cuboid([0.; 3], [8.; 3]).unwrap();
    let face = m
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 8.)
        })
        .unwrap();
    (m, face)
}
#[test]
fn closed_interior_cut_preserves_original_outer_and_shares_new_edges_twice() {
    let (m, f) = cube();
    let before = m.clone();
    let uv = rectangle([0.25, 0.25], [0.75, 0.75]);
    let out =
        face_cut::partition_closed(&m, f, &path(&m.faces[f].surface, &uv), 1e-8, 100000).unwrap();
    assert_eq!(out.model.faces.len(), m.faces.len() + 1);
    assert_eq!(out.model.loops[m.faces[f].outer], m.loops[m.faces[f].outer]);
    assert_eq!(out.model.faces[f].holes.len(), 1);
    assert!(out.model.faces[out.faces[1]].holes.is_empty());
    assert_eq!(out.edges.len(), 4);
    for (&edge, c) in out.edges.iter().zip(&uv) {
        assert_eq!(out.model.edges[edge].curve.domain(), c.domain());
        let uses = out
            .model
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .filter(|c| c.edge == edge)
            .collect::<Vec<_>>();
        assert_eq!(uses.len(), 2);
        assert_ne!(uses[0].reversed, uses[1].reversed);
    }
    assert_eq!(out.model.faces[f].surface, m.faces[f].surface);
    assert!(out.model.persistent_naming_complete());
    let lifted = brep_core::boundary_agreement::verify(&out.model, 100000).unwrap();
    assert!(lifted.complete);
    assert_eq!(m, before);
}
#[test]
fn nested_closed_cuts_reassign_the_existing_hole_without_abandoning_loops() {
    let (m, f) = cube();
    let a = face_cut::partition_closed(
        &m,
        f,
        &path(&m.faces[f].surface, &rectangle([0.4, 0.4], [0.6, 0.6])),
        1e-8,
        100000,
    )
    .unwrap();
    let hole = a.model.faces[f].holes[0];
    let b = face_cut::partition_closed(
        &a.model,
        f,
        &path(
            &a.model.faces[f].surface,
            &rectangle([0.2, 0.2], [0.8, 0.8]),
        ),
        1e-8,
        100000,
    )
    .unwrap();
    assert_eq!(b.model.faces[b.faces[1]].holes, vec![hole]);
    assert!(!b.model.faces[f].holes.contains(&hole));
    for wire in 0..b.model.loops.len() {
        assert_eq!(
            b.model
                .faces
                .iter()
                .flat_map(|f| std::iter::once(f.outer).chain(f.holes.iter().copied()))
                .filter(|&w| w == wire)
                .count(),
            1
        );
    }
    assert!(
        brep_core::face_domain::audit_trim_regions(&b.model, 1e-8, 100000, 100000, 1000000)
            .unwrap()
            .all_valid
    );
}
#[test]
fn crossing_touching_outside_clockwise_and_nonclosing_contours_refuse() {
    let (m, f) = cube();
    let before = m.clone();
    let original = rectangle([0.2, 0.2], [0.8, 0.8]);
    let mut clockwise = original
        .iter()
        .rev()
        .map(|c| c.reverse().unwrap())
        .collect::<Vec<_>>();
    for uv in [
        rectangle([-0.2, 0.2], [0.5, 0.8]),
        rectangle([0., 0.2], [0.5, 0.8]),
        std::mem::take(&mut clockwise),
    ] {
        assert!(
            face_cut::partition_closed(&m, f, &path(&m.faces[f].surface, &uv), 1e-8, 100000)
                .is_err()
        );
    }
    let mut p = path(&m.faces[f].surface, &original);
    p[0].curve.control_points[0][2] += 0.01;
    assert!(face_cut::partition_closed(&m, f, &p, 1e-8, 100000).is_err());
    let a = face_cut::partition_closed(
        &m,
        f,
        &path(&m.faces[f].surface, &rectangle([0.4, 0.4], [0.6, 0.6])),
        1e-8,
        100000,
    )
    .unwrap();
    assert!(
        face_cut::partition_closed(
            &a.model,
            f,
            &path(
                &a.model.faces[f].surface,
                &rectangle([0.3, 0.3], [0.5, 0.5])
            ),
            1e-8,
            100000
        )
        .is_err()
    );
    assert_eq!(m, before);
}
#[test]
fn rational_closed_contour_requires_whole_lifts_and_bounded_work() {
    let (m, f) = cube();
    let endpoints = [[0.7, 0.5], [0.5, 0.7], [0.3, 0.5], [0.5, 0.3]];
    let controls = [[0.7, 0.7], [0.3, 0.7], [0.3, 0.3], [0.7, 0.3]];
    let uv = (0..4)
        .map(|i| Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 2., 2., 2.],
            control_points: vec![
                endpoints[i].to_vec(),
                controls[i].to_vec(),
                endpoints[(i + 1) % 4].to_vec(),
            ],
            weights: vec![1., 0.5_f64.sqrt(), 1.],
            periodic: false,
        })
        .collect::<Vec<_>>();
    let p = path(&m.faces[f].surface, &uv);
    let out = face_cut::partition_closed(&m, f, &p, 1e-8, 100000).unwrap();
    assert_eq!(out.edges.len(), 4);
    let mut bad = path(&m.faces[f].surface, &uv);
    bad[0].curve.control_points[1][2] += 0.1;
    assert!(face_cut::partition_closed(&m, f, &bad, 1e-8, 100000).is_err());
    assert!(face_cut::partition_closed(&m, f, &p, 1e-8, 1).is_err());
}

use brep_core::shell_thickness::contact_pair::cap::{self as cap_sheet, regions};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn cap() -> regions::Regions {
    use nurbs_core::surface_offset::contact_curve::chain::{self, Span};
    let a = plane();
    let mut b = a.clone();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    let span = Span {
        owner: 0,
        axis: 0,
        drive: [0.1, 0.9],
        free: [0.7, 0.9],
        other: [[0.2, 0.8], [0.7, 0.9]],
        reversed: false,
    };
    let r = chain::build(
        [&a, &b],
        [0.8, -0.8],
        &[span],
        1e-7,
        1e-7,
        Default::default(),
    )
    .unwrap()
    .close_to_boundary(0, 0, Default::default())
    .unwrap()
    .with_nested(&[], Default::default())
    .unwrap();
    regions::build(
        cap_sheet::build(
            r.cap(0.005, 1e-6, Default::default()).unwrap(),
            Default::default(),
        )
        .unwrap(),
    )
}
#[test]
fn actual_cap_closed_regions_select_both_sides_and_replay_full_source() {
    let source = cap();
    let before = source.model().clone();
    let r = source
        .split_closed_uv(0, &rectangle([0.3, 0.2], [0.7, 0.6]), Default::default())
        .unwrap();
    assert_eq!(r.model().faces.len(), 2);
    assert_eq!(r.source().model(), &before);
    assert!(r.model().bodies.is_empty());
    let s = r.select(&[[0.2, 0.2], [0.5, 0.4]], 100000).unwrap();
    assert_eq!(s.retained_faces().len(), 2);
    s.reverify().unwrap();
    let inner = r.cuts()[0].faces[1];
    let r = r
        .split_closed_uv(
            inner,
            &rectangle([0.4, 0.3], [0.6, 0.5]),
            Default::default(),
        )
        .unwrap();
    assert_eq!(r.model().faces.len(), 3);
    r.reverify().unwrap();
}
#[test]
fn closed_cap_stage_limits_and_invalid_faces_do_not_publish_partial_results() {
    for stage in 0..3 {
        let mut l = regions::Limits::default();
        match stage {
            0 => l.cut_cells = 1,
            1 => l.boundary_cells = 1,
            _ => l.trim_pairs = 1,
        }
        let r = cap().split_closed_uv(0, &rectangle([0.3, 0.2], [0.7, 0.6]), l);
        assert!(r.is_err(), "stage {stage}");
    }
    assert!(
        cap()
            .split_closed_uv(999, &rectangle([0.3, 0.2], [0.7, 0.6]), Default::default())
            .is_err()
    );
}

#[test]
fn nonplanar_actual_cylindrical_cap_accepts_a_closed_cut_and_full_replay() {
    use nurbs_core::surface_offset::contact_curve::chain::{self, Span};
    let mut a = plane();
    a.degree_u = 2;
    a.knots_u = vec![0., 0., 0., 1., 1., 1.];
    a.control_points = [[1., 0.], [1., 1.], [0., 1.]]
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
        .to_vec();
    a.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
        .map(|w| vec![w; 2])
        .to_vec();
    let mut b = plane();
    b.control_points = vec![
        vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        vec![vec![0., 2., 0.], vec![0.5, 2., 1.]],
    ];
    let span = Span {
        owner: 0,
        axis: 1,
        drive: [0.35, 0.55],
        free: [0.88, 0.96],
        other: [[0.07, 0.22], [0.3, 0.6]],
        reversed: false,
    };
    let r = chain::build([&a, &b], [0.1; 2], &[span], 1e-7, 1e-7, Default::default())
        .unwrap()
        .close_to_boundary(0, 3, Default::default())
        .unwrap()
        .with_nested(&[], Default::default())
        .unwrap();
    let source = regions::build(
        cap_sheet::build(
            r.cap(0.005, 1e-6, Default::default()).unwrap(),
            Default::default(),
        )
        .unwrap(),
    );
    let before = source.model().clone();
    let cut = source
        .split_closed_uv(0, &rectangle([0.2, 0.4], [0.6, 0.5]), Default::default())
        .unwrap();
    assert_eq!(cut.model().faces.len(), 2);
    assert_eq!(cut.source().model(), &before);
    assert!(cut.cuts()[0].boundary.complete && cut.cuts()[0].trim.all_valid);
    let selection = cut.select(&[[0.1, 0.45], [0.4, 0.45]], 100000).unwrap();
    assert_eq!(selection.retained_faces().len(), 2);
    selection.reverify().unwrap();
    assert!(cut.model().bodies.is_empty());
}
