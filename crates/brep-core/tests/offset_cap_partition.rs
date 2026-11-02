use nurbs_core::{
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
fn one_long_contact_rim_subdivides_and_sews_two_short_contact_caps() {
    let mut sheets = vec![];
    for drive in [[0.1, 0.5], [0.5, 0.9]] {
        let r = outer_on(drive, 1)
            .with_nested(&[], Default::default())
            .unwrap();
        sheets.push(
            cap_sheet::build(
                r.cap(0.005, 1e-6, Default::default()).unwrap(),
                Default::default(),
            )
            .unwrap(),
        );
    }
    let vertex = sheets[0].model().vertices[1].point;
    let a = sheet([0.1, 0.9])
        .split_edge_at_vertex(0, vertex, 128, Default::default())
        .unwrap();
    assert_eq!(a.partitions().len(), 1);
    assert!(a.partitions()[0].search.unwrap().evaluations <= 128);
    assert_eq!(a.model().edges.len(), 5);
    assert!(a.partitions()[0].nesting.proven);
    assert!(a.model().1.lineage.iter().any(|r| r.operation == "split"));
    assert_eq!(a.model().vertices[4].point, vertex);
    sheets.insert(0, a);
    let n = sewing::build(sheets, Default::default()).unwrap();
    assert_eq!(n.joins().len(), 3);
    assert_eq!(n.model().faces.len(), 3);
    assert_eq!(n.model().edges.len(), 10);
    assert_eq!(n.model().vertices.len(), 8);
    assert_eq!(n.model().shells.len(), 1);
    assert!(!n.model().shells[0].closed);
    assert!(n.boundary_proof().complete);
    assert!(n.model().bodies.is_empty());
    n.reverify().unwrap();
}
#[test]
fn repeated_splits_retain_original_cap_and_direct_original_ownership() {
    let a = sheet([0.1, 0.9]);
    let raw = a.cap().region().world_loops().to_vec();
    let a = a
        .split_edge(0, 0.3, Default::default())
        .unwrap()
        .split_edge(4, 0.6, Default::default())
        .unwrap();
    assert_eq!(a.partitions().len(), 2);
    assert_eq!(a.model().edges.len(), 6);
    assert_eq!(a.cap().region().world_loops(), raw);
    assert!(
        a.partitions()
            .iter()
            .all(|p| p.nesting.proven && p.original_cells > 0)
    );
    a.reverify().unwrap();
}
#[test]
fn invalid_parameters_and_exhausted_proof_budgets_publish_no_subdivided_sheet() {
    for p in [f64::NAN, 0.1, 0.9, 2.] {
        assert!(
            sheet([0.1, 0.9])
                .split_edge(0, p, Default::default())
                .is_err()
        );
    }
    for kind in 0..4 {
        let mut l = cap_sheet::partition::Limits::default();
        match kind {
            0 => l.edge_cells = 1,
            1 => l.original_cells = 1,
            2 => l.boundary_cells = 1,
            _ => l.nesting.cells = 1,
        }
        assert!(sheet([0.1, 0.9]).split_edge(0, 0.5, l).is_err());
    }
}
#[test]
fn rational_reversed_hole_subdivision_keeps_raw_weights_and_nonunit_clock() {
    use nurbs_core::curve::Curve;
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
    let r = outer([0.1, 0.9])
        .with_nested(&[hole.clone()], Default::default())
        .unwrap();
    let s = cap_sheet::build(
        r.cap(0.005, 1e-6, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    assert!(s.model().loops[1].coedges.iter().all(|c| c.reversed));
    let s = s.split_edge(4, -0.5, Default::default()).unwrap();
    assert_eq!(s.cap().region().uv_domain().loops()[1], hole);
    assert_eq!(s.model().loops[1].coedges.len(), 5);
    assert!(s.model().loops[1].coedges.iter().all(|c| c.reversed));
    assert_eq!(s.model().edges[4].curve.domain(), [-3., -0.5]);
    assert_eq!(s.model().edges[8].curve.domain(), [-0.5, 2.]);
    assert!(s.boundary_proof().complete);
    s.reverify().unwrap();
}

#[test]
fn shared_vertex_proposal_cannot_waive_continuous_original_or_actual_lifts() {
    for p in [[0.5, 0.8, 0.81], [0.51, 0.8, 0.8], [f64::NAN, 0.8, 0.8]] {
        assert!(
            sheet([0.1, 0.9])
                .split_edge_to_vertex(0, 0.5, p, Default::default())
                .is_err()
        );
    }
}

#[test]
fn sampled_parameter_proposals_cannot_admit_off_curve_vertices() {
    assert!(
        sheet([0.1, 0.9])
            .split_edge_at_vertex(0, [0.5, 0.8, 0.81], 128, Default::default())
            .is_err()
    );
    assert!(
        sheet([0.1, 0.9])
            .split_edge_at_vertex(0, [0.5, 0.8, 0.8], 1, Default::default())
            .is_err()
    );
}
