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

use brep_core::shell_thickness::contact_pair::cap::{self as cap_sheet, sewing::crossings};
fn sheet(rotated: bool, tilt: bool, z: f64) -> cap_sheet::Sheet {
    sheet_holes(rotated, tilt, z, false)
}
fn sheet_holes(rotated: bool, tilt: bool, z: f64, holes: bool) -> cap_sheet::Sheet {
    let mut a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    for s in [&mut a, &mut b] {
        for p in s.control_points.iter_mut().flatten() {
            if rotated {
                let x = p[0];
                p[0] = 1.3 - p[1];
                p[1] = x + 0.4;
            }
            if tilt {
                p[2] += 0.1 * p[0];
            }
            p[2] += z;
        }
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
        Limits::default(),
    )
    .unwrap()
    .close_to_boundary(0, 0, region::Limits::default())
    .unwrap()
    .with_nested(
        &if holes {
            use nurbs_core::curve::Curve;
            [0.2, 0.3, 0.4]
                .map(|x| {
                    let p = [[x, 0.1], [x + 0.04, 0.1], [x + 0.04, 0.14], [x, 0.14]];
                    (0..4)
                        .map(|i| {
                            Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()])
                                .unwrap()
                        })
                        .collect::<Vec<_>>()
                })
                .to_vec()
        } else {
            vec![]
        },
        Default::default(),
    )
    .unwrap();
    cap_sheet::build(
        r.cap(0.005, 1e-6, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap()
}
#[test]
fn interior_original_roots_imprint_both_complete_rims_transactionally() {
    let a = sheet(false, false, 0.);
    let b = sheet(true, false, 0.);
    let raw = [
        a.cap().region().world_loops().to_vec(),
        b.cap().region().world_loops().to_vec(),
    ];
    let r = crossings::build(vec![a, b], Default::default()).unwrap();
    assert!(!r.events().is_empty());
    assert!(
        r.events()
            .iter()
            .any(|e| matches!(e.origin, crossings::Origin::Exact(_)))
    );
    for e in r.events() {
        for m in e.marks {
            assert_eq!(
                r.sheets()[m.sheet].model().vertices[m.vertex].point,
                e.point
            );
        }
    }
    for (s, raw) in r.sheets().iter().zip(raw) {
        assert_eq!(s.cap().region().world_loops(), raw);
        assert!(s.model().bodies.is_empty());
    }
    r.reverify().unwrap();
}
#[test]
fn spatial_numerical_roots_remain_proposals_after_complete_imprint_admission() {
    let r = crossings::build(
        vec![sheet(false, true, 0.), sheet(true, true, 0.)],
        Default::default(),
    )
    .unwrap();
    assert!(
        r.events()
            .iter()
            .any(|e| matches!(e.origin, crossings::Origin::Proposal(_)))
    );
    assert!(
        r.pairs()
            .iter()
            .any(|p| p.exact.as_ref().is_some_and(|e| !e.complete))
    );
    r.reverify().unwrap();
}
#[test]
fn separated_original_hulls_exclude_contacts_without_inventing_roots() {
    let r = crossings::build(
        vec![sheet(false, false, 0.), sheet(true, false, 0.02)],
        Default::default(),
    )
    .unwrap();
    assert!(r.events().is_empty());
    assert!(r.hull_exclusions() > 0);
    assert!(r.pairs().is_empty());
    r.reverify().unwrap();
}
#[test]
fn budgets_keep_discovery_and_construction_refusals_explicit() {
    for kind in 0..3 {
        let mut l = crossings::Limits::default();
        match kind {
            0 => l.pairs = 1,
            1 => l.candidates = 0,
            _ => l.partition.boundary_cells = 0,
        }
        assert!(
            crossings::build(vec![sheet(false, false, 0.), sheet(true, false, 0.)], l).is_err()
        );
    }
    let r = crossings::build(
        vec![sheet(false, false, 0.), sheet(true, false, 0.)],
        crossings::Limits {
            max_splits: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(r.events().is_empty());
    assert!(!r.refusals().is_empty());
    assert!(r.sheets().iter().all(|s| s.partitions().is_empty()));
    r.reverify().unwrap();
}

#[test]
fn failed_second_subdivision_rolls_back_the_successful_first_candidate() {
    let a = sheet(false, false, 0.);
    let b = sheet_holes(true, false, 0., true);
    let before = [a.model().clone(), b.model().clone()];
    let l = crossings::Limits {
        partition: cap_sheet::partition::Limits {
            original_cells: 12,
            ..Default::default()
        },
        ..Default::default()
    };
    let r = crossings::build(vec![a, b], l).unwrap();
    assert!(r.events().is_empty());
    assert!(
        r.refusals()
            .iter()
            .any(|r| r.reason.contains("sheet 1") && r.reason.contains("direct-original"))
    );
    for (s, old) in r.sheets().iter().zip(before) {
        assert_eq!(s.model(), &old);
        assert!(s.partitions().is_empty());
    }
    r.reverify().unwrap();
}
#[test]
fn unresolved_spatial_pairs_without_fallback_never_become_proved_absence() {
    let r = crossings::build(
        vec![sheet(false, true, 0.), sheet(true, true, 0.)],
        crossings::Limits {
            proposal_fallback: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(r.pairs().iter().any(|p| {
        p.exact
            .as_ref()
            .is_some_and(|e| !e.complete && !e.unresolved.is_empty())
    }));
    assert!(r.pairs().iter().all(|p| p.proposal.is_none()));
    assert_eq!(r.proposal_boxes_upper(), 0);
    assert!(
        r.events()
            .iter()
            .all(|e| matches!(e.origin, crossings::Origin::Exact(_)))
    );
    r.reverify().unwrap();
}
