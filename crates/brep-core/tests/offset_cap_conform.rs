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

use sewing::conform;
fn contact_sheets() -> Vec<cap_sheet::Sheet> {
    contact_sheets_for(&[[0.1, 0.5], [0.5, 0.9]])
}
fn contact_sheets_for(drives: &[[f64; 2]]) -> Vec<cap_sheet::Sheet> {
    let mut s = vec![sheet([0.1, 0.9])];
    for &drive in drives {
        let r = outer_on(drive, 1)
            .with_nested(&[], Default::default())
            .unwrap();
        s.push(
            cap_sheet::build(
                r.cap(0.005, 1e-6, Default::default()).unwrap(),
                Default::default(),
            )
            .unwrap(),
        );
    }
    s
}
#[test]
fn automatic_vertex_discovery_conforms_and_sews_contact_t_junction() {
    let l = conform::Limits::default();
    let r = conform::build(contact_sheets(), l).unwrap();
    assert_eq!(r.insertions().len(), 1);
    assert_eq!(r.network().joins().len(), 3);
    let m = r.network().model();
    assert_eq!(m.faces.len(), 3);
    assert_eq!(m.edges.len(), 10);
    assert_eq!(m.vertices.len(), 8);
    assert_eq!(m.shells.len(), 1);
    assert!(m.bodies.is_empty());
    assert!(r.pairs() <= l.pairs && r.evaluations_upper() <= l.evaluations);
    assert!(r.hull_exclusions() > 0);
    r.reverify().unwrap();
}
#[test]
fn input_order_does_not_hide_partial_contact_candidates() {
    let mut sheets = contact_sheets();
    sheets.reverse();
    let r = conform::build(sheets, Default::default()).unwrap();
    assert_eq!(r.insertions().len(), 1);
    assert_eq!(r.network().joins().len(), 3);
    r.reverify().unwrap();
}
#[test]
fn separated_and_near_endpoint_inputs_do_not_become_fabricated_contacts() {
    let r = conform::build(
        vec![sheet([0.1, 0.4]), sheet([0.6, 0.9])],
        Default::default(),
    )
    .unwrap();
    assert!(r.insertions().is_empty());
    assert!(r.network().joins().is_empty());
    assert!(r.hull_exclusions() > 0);
    r.reverify().unwrap();
    let r = conform::build(
        vec![sheet([0.1, 0.5]), sheet([0.5 + 1e-8, 0.9])],
        Default::default(),
    )
    .unwrap();
    assert!(r.insertions().is_empty());
    assert!(r.network().joins().is_empty());
    assert!(!r.refusals().is_empty());
    r.reverify().unwrap();
}
#[test]
fn shared_resource_limits_and_invalid_nested_options_publish_no_partial_network() {
    for kind in 0..5 {
        let mut l = conform::Limits::default();
        match kind {
            0 => l.pairs = 1,
            1 => l.evaluations = 17,
            2 => l.candidates = 0,
            3 => l.partition.nesting.cells = 0,
            _ => l.sewing.boundary_cells = 1,
        }
        let input = if kind == 1 {
            contact_sheets_for(&[[0.1, 0.3], [0.3, 0.6], [0.6, 0.9]])
        } else {
            contact_sheets()
        };
        assert!(conform::build(input, l).is_err(), "kind={kind}");
    }
}

#[test]
fn sufficient_small_search_budget_is_accepted_for_a_single_insertion() {
    let r = conform::build(
        contact_sheets(),
        conform::Limits {
            evaluations: 17,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(r.insertions().len(), 1);
    assert_eq!(r.network().joins().len(), 3);
    assert!(r.evaluations_upper() <= 17);
    r.reverify().unwrap();
}
fn rational_sheet(drive: [f64; 2]) -> cap_sheet::Sheet {
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
fn rational_partial_rims_conform_without_changing_raw_definitions() {
    let a = rational_sheet([0.1, 0.8]);
    let raw = a.cap().region().uv_domain().loops().to_vec();
    let b = rational_sheet([0.1, 0.9])
        .split_edge(4, -0.5, Default::default())
        .unwrap();
    let target = b.model().vertices.len() - 1;
    let r = conform::build(vec![a, b], Default::default()).unwrap();
    assert!(
        r.insertions()
            .iter()
            .any(|i| i.sheet == 0 && i.target_sheet == 1 && i.target_vertex == target)
    );
    assert_eq!(
        r.network().sheets()[0].cap().region().uv_domain().loops(),
        raw
    );
    assert!(r.network().joins().len() >= 5);
    // These overlapping regions test curve correspondence; no embedding follows.
    assert!(r.network().model().bodies.is_empty());
    r.reverify().unwrap();
}
#[test]
fn in_hull_off_curve_candidates_remain_explicit_complete_proof_refusals() {
    use nurbs_core::curve::Curve;
    let a = rational_sheet([0.1, 0.8]);
    let p = [[0.44, 0.44], [0.47, 0.44], [0.47, 0.47], [0.44, 0.47]];
    let hole = (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
        .collect::<Vec<_>>();
    let raw = hole.clone();
    let r = outer([0.1, 0.9])
        .with_nested(&[hole], Default::default())
        .unwrap();
    let b = cap_sheet::build(
        r.cap(0.005, 1e-6, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let result = conform::build(vec![a, b], Default::default()).unwrap();
    assert!(
        result
            .refusals()
            .iter()
            .any(|r| matches!(&r.stage, conform::Stage::CompleteSubdivision))
    );
    assert_eq!(
        result.network().sheets()[1]
            .cap()
            .region()
            .uv_domain()
            .loops()[1],
        raw
    );
    assert!(result.network().model().bodies.is_empty());
    result.reverify().unwrap();
}
