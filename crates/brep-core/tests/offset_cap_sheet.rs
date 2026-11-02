use nurbs_core::{
    curve::Curve,
    surface::Surface,
    surface_offset::contact_curve::chain::{
        self, Limits, Span,
        region::{
            self,
            compound::{self},
        },
    },
};
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
fn outer() -> region::Region {
    let a = plane();
    let mut b = plane();
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
    chain::build(
        [&a, &b],
        [0.8, -0.8],
        &[span],
        1e-7,
        1e-7,
        Limits::default(),
    )
    .unwrap()
    .close_to_boundary(0, 0, region::Limits::default())
    .unwrap()
}
fn square(lo: f64, hi: f64) -> Vec<Curve> {
    let p = [[lo, lo], [hi, lo], [hi, hi], [lo, hi]];
    (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
        .collect()
}

fn compound() -> compound::Compound {
    outer()
        .with_nested(
            &[square(0.2, 0.7), square(0.3, 0.6)],
            compound::Limits::default(),
        )
        .unwrap()
}

#[test]
fn actual_caps_become_open_faces_with_holes_and_islands() {
    let c = compound().cap(0.005, 1e-6, Default::default()).unwrap();
    let world = c.region().world_loops().to_vec();
    let sheet =
        brep_core::shell_thickness::contact_pair::cap::build(c, Default::default()).unwrap();
    let m = sheet.model();
    assert_eq!(m.faces.len(), 2);
    assert_eq!(m.faces[0].holes.len(), 1);
    assert_eq!(m.faces[1].holes.len(), 0);
    assert_eq!(m.edges.len(), 12);
    assert!(m.bodies.is_empty());
    assert_eq!(m.shells.len(), 2);
    assert!(m.shells.iter().all(|s| !s.closed));
    assert!(sheet.boundary_proof().complete);
    let face = brep_core::face_domain::FaceDomain::new(m, 0, 1e-7).unwrap();
    assert_eq!(
        face.classify([[0.25; 2], [0.4; 2]], 10000)
            .unwrap()
            .location,
        nurbs_core::trim_domain::Location::Outside
    );
    assert_eq!(
        face.classify([[0.15; 2], [0.4; 2]], 10000)
            .unwrap()
            .location,
        nurbs_core::trim_domain::Location::Inside
    );
    let island = brep_core::face_domain::FaceDomain::new(m, 1, 1e-7).unwrap();
    assert_eq!(
        island
            .classify([[0.4; 2], [0.4; 2]], 10000)
            .unwrap()
            .location,
        nurbs_core::trim_domain::Location::Inside
    );

    assert_eq!(sheet.cap().region().world_loops(), world);
    assert!(m.loops[1].coedges.iter().all(|c| c.reversed));
    m.validate_continuous(100000).unwrap();
    sheet.reverify().unwrap();
}
#[test]
fn orientation_budget_does_not_publish_partial_sheet() {
    let c = compound().cap(0.005, 1e-6, Default::default()).unwrap();
    let l = brep_core::shell_thickness::contact_pair::cap::Limits {
        orientation_cells: 1,
        ..Default::default()
    };
    assert!(brep_core::shell_thickness::contact_pair::cap::build(c, l).is_err());
}
#[test]
fn reversed_rational_hole_keeps_original_nonunit_clock_and_weights() {
    let hole = [
        [[1., 0.], [1., 1.], [0., 1.]],
        [[0., 1.], [-1., 1.], [-1., 0.]],
        [[-1., 0.], [-1., -1.], [0., -1.]],
        [[0., -1.], [1., -1.], [1., 0.]],
    ]
    .map(|p| Curve {
        degree: 2,
        knots: vec![-3., -3., -3., 2., 2., 2.],
        control_points: p
            .map(|p| vec![0.4 + 0.05 * p[0], 0.4 + 0.05 * p[1]])
            .to_vec(),
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    })
    .to_vec();
    let region = outer()
        .with_nested(&[hole.clone()], Default::default())
        .unwrap();
    let cap = region.cap(0.005, 1e-6, Default::default()).unwrap();
    let s = brep_core::shell_thickness::contact_pair::cap::build(cap, Default::default()).unwrap();
    assert_eq!(s.cap().region().uv_domain().loops()[1], hole);
    for c in &s.model().loops[1].coedges {
        assert!(c.reversed);
        assert_eq!(c.pcurve.domain(), [-3., 2.]);
        assert_eq!(
            c.pcurve.weights,
            vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
        );
    }
    s.reverify().unwrap();
}
