use nurbs_core::{
    curve::Curve,
    surface::Surface,
    surface_offset::contact_curve::chain::{
        self, Limits, Span,
        region::{self},
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
fn outer(drive: [f64; 2]) -> region::Region {
    outer_on(drive, 0)
}
fn outer_on(drive: [f64; 2], side: usize) -> region::Region {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    let span = Span {
        owner: 0,
        axis: 0,
        drive,
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
    .close_to_boundary(side, 0, region::Limits::default())
    .unwrap()
}
use brep_core::shell_thickness::contact_pair::cap::{self as cap_sheet, sewing};
fn sheet(drive: [f64; 2]) -> cap_sheet::Sheet {
    let region = outer(drive).with_nested(&[], Default::default()).unwrap();
    cap_sheet::build(
        region.cap(0.005, 1e-6, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap()
}
#[test]
fn adjacent_actual_caps_discover_and_sew_a_complete_original_edge() {
    let a = sheet([0.1, 0.5]);
    let b = sheet([0.5, 0.9]);
    let original = [a.model().clone(), b.model().clone()];
    let n = sewing::build(
        vec![a, b],
        sewing::Limits {
            correspondence_cells: 1000000,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(n.joins().len(), 1);
    assert!(n.joins()[0].proof.accepted);
    assert!(n.joins()[0].reversed);
    let m = n.model();
    assert_eq!(m.faces.len(), 2);
    assert_eq!(m.edges.len(), 7);
    assert_eq!(m.vertices.len(), 6);
    assert_eq!(m.shells.len(), 1);
    assert_eq!(m.shells[0].faces.len(), 2);
    assert!(!m.shells[0].closed);
    assert!(m.bodies.is_empty());
    assert!(n.boundary_proof().complete);
    for (sheet, saved) in n.sheets().iter().zip(&original) {
        assert_eq!(sheet.model(), saved);
    }
    n.reverify().unwrap();
}
#[test]
fn disconnected_caps_stay_separate_components() {
    let n = sewing::build(
        vec![sheet([0.1, 0.4]), sheet([0.6, 0.9])],
        Default::default(),
    )
    .unwrap();
    assert!(n.joins().is_empty());
    assert_eq!(n.model().shells.len(), 2);
    assert_eq!(n.model().edges.len(), 8);
    n.reverify().unwrap();
}
#[test]
fn ambiguous_three_way_mates_and_shared_work_limits_publish_no_network() {
    assert!(
        sewing::build(
            vec![sheet([0.1, 0.5]), sheet([0.1, 0.5])],
            Default::default()
        )
        .is_err()
    );
    assert!(
        sewing::build(
            vec![sheet([0.1, 0.5]), sheet([0.1, 0.5]), sheet([0.1, 0.5])],
            Default::default()
        )
        .is_err()
    );
    for kind in 0..3 {
        let mut l = sewing::Limits::default();
        match kind {
            0 => l.pairs = 1,
            1 => l.correspondence_cells = 0,
            _ => l.boundary_cells = 1,
        }
        assert!(sewing::build(vec![sheet([0.1, 0.5]), sheet([0.5, 0.9])], l).is_err());
    }
}
fn hole_sheet(clock: [f64; 2], bow: bool, drive: [f64; 2]) -> cap_sheet::Sheet {
    let p = [[0.2, 0.2], [0.4, 0.2], [0.4, 0.4], [0.2, 0.4]];
    let mut hole = (0..4)
        .map(|i| Curve {
            degree: 1,
            knots: vec![clock[0], clock[0], clock[1], clock[1]],
            control_points: vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()],
            weights: vec![1.; 2],
            periodic: false,
        })
        .collect::<Vec<_>>();
    if bow {
        hole[0] = Curve {
            degree: 2,
            knots: vec![clock[0]; 3]
                .into_iter()
                .chain(vec![clock[1]; 3])
                .collect(),
            control_points: vec![vec![0.2, 0.2], vec![0.3, 0.25], vec![0.4, 0.2]],
            weights: vec![1.; 3],
            periodic: false,
        };
    }
    let r = outer(drive)
        .with_nested(&[hole], Default::default())
        .unwrap();
    cap_sheet::build(
        r.cap(0.005, 1e-6, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap()
}
#[test]
fn whole_curves_with_independent_physical_clocks_are_compared_before_sewing() {
    let n = sewing::build(
        vec![
            hole_sheet([0., 1.], false, [0.1, 0.8]),
            hole_sheet([-3., 2.], false, [0.1, 0.9]),
        ],
        Default::default(),
    )
    .unwrap();
    assert_eq!(n.joins().len(), 5);
    assert!(n.joins().iter().all(|j| j.proof.accepted));
    // This checks correspondence only; overlapping faces grant no embedding.
    assert!(!n.model().shells[0].closed);
    assert!(n.model().bodies.is_empty());
    n.reverify().unwrap();
}
#[test]
fn identical_endpoints_do_not_establish_whole_curve_identity() {
    let e = sewing::build(
        vec![
            hole_sheet([0., 1.], false, [0.1, 0.8]),
            hole_sheet([0., 1.], true, [0.1, 0.9]),
        ],
        Default::default(),
    )
    .err()
    .unwrap();
    assert!(
        e.to_string()
            .contains("complete original curve correspondence"),
        "{e}"
    );
}

#[test]
fn two_actual_contact_caps_sew_the_original_shared_root_without_manual_pairs() {
    let mut sheets = vec![];
    for side in 0..2 {
        let r = outer_on([0.1, 0.9], side)
            .with_nested(&[], Default::default())
            .unwrap();
        let c = r.cap(0.005, 1e-6, Default::default()).unwrap();
        sheets.push(cap_sheet::build(c, Default::default()).unwrap());
    }
    let n = sewing::build(sheets, Default::default()).unwrap();
    assert_eq!(n.joins().len(), 1);
    assert_eq!(n.model().shells.len(), 1);
    assert_eq!(n.model().shells[0].faces.len(), 2);
    assert!(n.boundary_proof().complete);
    assert_ne!(n.model().faces[0].surface, n.model().faces[1].surface);
    n.reverify().unwrap();
}
#[test]
fn near_coincident_endpoints_are_not_silently_snapped_or_declared_shared() {
    let n = sewing::build(
        vec![sheet([0.1, 0.5]), sheet([0.5 + 1e-8, 0.9])],
        Default::default(),
    )
    .unwrap();
    assert!(n.joins().is_empty());
    assert_eq!(n.model().shells.len(), 2);
    assert_eq!(n.model().vertices.len(), 8);
    assert!(n.model().bodies.is_empty());
    n.reverify().unwrap();
}
