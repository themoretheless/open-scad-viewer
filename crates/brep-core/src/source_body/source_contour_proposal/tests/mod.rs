use super::*;
#[test]
fn automatic_crossings_clip_source_expressions_without_rounded_curve_trims() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 1.], vec![0., 1., 1.]],
            vec![vec![1., 0., 1.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let p = [vec![1., 0.], vec![1., 1.], vec![0., 1.], vec![0., 0.]];
    let wire = (0..4)
        .map(|i| {
            let pcurve =
                Curve::from_polyline(vec![p[i].clone(), p[(i + 1) % 4].clone()]).unwrap();
            let mut curve = pcurve.clone();
            for p in &mut curve.control_points {
                p.push(1.);
            }
            Boundary {
                curve,
                pcurve,
                reversed: false,
            }
        })
        .collect::<Vec<_>>();
    let contact = Curve::from_polyline(vec![vec![-0.2, 0.3], vec![1.2, 0.3]]).unwrap();
    let limits = Limits {
        pairs: 10000,
        region_cells: 100000,
        domain_cells: 100000,
        agreement_cells: 100000,
    };
    let out = propose(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone()],
        &contact,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
    )
    .unwrap();
    assert!(out.source_joins_proven);
    let loops = out.candidate_loops.unwrap();
    assert_eq!(loops[0].len(), 4);
    assert_eq!(loops[0][0].curve(), &contact);
    assert!(loops[0][0].reversed());
    assert_eq!(loops[0][1].curve(), &wire[2].pcurve);
    assert_eq!(loops[0][2].curve(), &wire[3].pcurve);
    assert_eq!(loops[0][3].curve(), &wire[0].pcurve);
    let interior = qualify_interior_contact(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone()],
        &contact,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
    )
    .unwrap();
    assert!(
        interior.only_owned_crossings_proven
            && interior.contact_simple_proven
            && interior.contact_inside_proven
    );
    assert!(!interior.region_subset_proven);
    let hole_points = [
        vec![0.4, 0.2],
        vec![0.4, 0.4],
        vec![0.6, 0.4],
        vec![0.6, 0.2],
    ];
    let hole = (0..4)
        .map(|i| {
            let pcurve = Curve::from_polyline(vec![
                hole_points[i].clone(),
                hole_points[(i + 1) % 4].clone(),
            ])
            .unwrap();
            let mut curve = pcurve.clone();
            for p in &mut curve.control_points {
                p.push(1.);
            }
            Boundary {
                curve,
                pcurve,
                reversed: false,
            }
        })
        .collect::<Vec<_>>();
    let bad = qualify_interior_contact(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone(), hole.clone()],
        &contact,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
    )
    .unwrap();
    assert!(bad.proposal.search.complete);
    assert_eq!(bad.proposal.search.points.len(), 4);
    let across_hole = Curve::from_polyline(vec![vec![0.3, 0.3], vec![0.7, 0.3]]).unwrap();
    let void = qualify_interior_contact(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone(), hole],
        &across_hole,
        1,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
    )
    .unwrap();
    assert!(void.only_owned_crossings_proven && void.contact_simple_proven);
    assert!(!void.contact_inside_proven);
    assert_eq!(
        void.membership.as_ref().unwrap().location,
        nurbs_core::trim_domain::Location::Outside
    );

    assert!(
        !bad.only_owned_crossings_proven
            && !bad.contact_inside_proven
            && !bad.region_subset_proven
    );

    let region = qualify_linear_region(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone()],
        &contact,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
        1000,
    )
    .unwrap();
    assert!(region.kept_side_proven && region.removed_side_proven && region.holes_kept_proven);
    assert!(region.interior.region_subset_proven && region.region.is_some());
    crate::source_region_restore::assert_replay(region.region.as_ref().unwrap());
    let make_hole = |y0: f64, y1: f64| {
        let points = [vec![0.4, y0], vec![0.4, y1], vec![0.6, y1], vec![0.6, y0]];
        (0..4)
            .map(|i| {
                let pcurve =
                    Curve::from_polyline(vec![points[i].clone(), points[(i + 1) % 4].clone()])
                        .unwrap();
                let mut curve = pcurve.clone();
                for p in &mut curve.control_points {
                    p.push(1.);
                }
                Boundary {
                    curve,
                    pcurve,
                    reversed: false,
                }
            })
            .collect::<Vec<_>>()
    };
    let kept = qualify_linear_region(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone(), make_hole(0.05, 0.15)],
        &contact,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
        1000,
    )
    .unwrap();
    assert!(kept.holes_kept_proven && kept.region.is_some());
    crate::source_region_restore::assert_replay(kept.region.as_ref().unwrap());
    assert_eq!(kept.region.unwrap().loops().len(), 2);
    let removed_hole = qualify_linear_region(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone(), make_hole(0.5, 0.7)],
        &contact,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
        1000,
    )
    .unwrap();
    assert!(removed_hole.interior.contact_inside_proven);
    assert!(!removed_hole.holes_kept_proven && removed_hole.hole_placement_proven);
    assert!(removed_hole.interior.region_subset_proven);
    assert_eq!(removed_hole.removed_holes, vec![1]);
    let region = removed_hole.region.unwrap();
    assert_eq!(region.loops().len(), 1);
    assert_eq!(region.source_loop_indices(), &[0]);
    assert_eq!(
        removed_hole
            .interior
            .proposal
            .candidate_loops
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        removed_hole
            .interior
            .proposal
            .candidate_source_loops
            .as_ref()
            .unwrap(),
        &vec![0]
    );
    let mixed = qualify_linear_region(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone(), make_hole(0.5, 0.7), make_hole(0.05, 0.15)],
        &contact,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
        1000,
    )
    .unwrap();
    assert_eq!(mixed.removed_holes, vec![1]);
    let region = mixed.region.unwrap();
    assert_eq!(region.source_loop_indices(), &[0, 2]);
    assert_eq!(region.loops().len(), 2);

    let limited = qualify_linear_region(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone()],
        &contact,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
        1,
    )
    .unwrap();
    assert!(limited.region.is_none() && !limited.interior.region_subset_proven);
    assert_eq!(limited.controls, 1);

    let mut clockwise = wire.clone();
    clockwise.reverse();
    for b in &mut clockwise {
        b.pcurve.control_points.reverse();
        b.curve.control_points.reverse();
    }
    let clockwise = qualify_linear_region(
        &ToleranceContext::default_valid(),
        &s,
        &[clockwise],
        &contact,
        0,
        1,
        3,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
        1000,
    )
    .unwrap();
    assert!(clockwise.region.is_none() && !clockwise.interior.region_subset_proven);
    assert_eq!(
        clockwise.interior.proposal.search.search.original.reason,
        "face-region-orientation-invalid"
    );
    let vertical = Curve::from_polyline(vec![vec![0.3, -0.2], vec![0.3, 1.2]]).unwrap();
    let vertical = qualify_linear_region(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone()],
        &vertical,
        0,
        3,
        1,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        1,
        1000,
        10000,
        1000,
    )
    .unwrap();
    assert!(vertical.region.is_some() && vertical.interior.region_subset_proven);

    let curved = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![-0.2, 0.25], vec![0.5, 0.3], vec![1.2, 0.6]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let curved_region = qualify_curved_region(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone(), make_hole(0.05, 0.15), make_hole(0.7, 0.85)],
        &curved,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
        10000,
    )
    .unwrap();
    assert!(
        curved_region.region.is_some(),
        "{} / {}",
        curved_region.reason,
        curved_region.interior.reason
    );
    crate::source_region_restore::assert_replay(curved_region.region.as_ref().unwrap());
    assert_eq!(curved_region.removed_holes, vec![2]);
    assert_eq!(
        curved_region.region.as_ref().unwrap().source_loop_indices(),
        &[0, 1]
    );
    assert_eq!(
        curved_region.region.as_ref().unwrap().loops()[0][0].curve(),
        &curved
    );

    let mut rational = curved.clone();
    rational.weights[1] = 0.7;
    let rational_region = qualify_curved_region(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone(), make_hole(0.05, 0.15), make_hole(0.7, 0.85)],
        &rational,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
        10000,
    )
    .unwrap();
    assert!(
        rational_region.region.is_some(),
        "{} / {}",
        rational_region.reason,
        rational_region.interior.reason
    );
    crate::source_region_restore::assert_replay(rational_region.region.as_ref().unwrap());
    assert_eq!(rational_region.removed_holes, vec![2]);
    let world_wires = rational_region
        .region
        .as_ref()
        .unwrap()
        .world_wires()
        .unwrap();
    assert_eq!(world_wires.len(), 2);
    for wire in &world_wires {
        assert!(wire.world_mapping(1000).unwrap().complete);
        assert_eq!(wire.vertices().last().unwrap()[1], 0);
    }
    assert_eq!(world_wires[0].edges()[0].curve(), &rational);
    assert_eq!(
        rational_region.region.as_ref().unwrap().loops()[0][0].curve(),
        &rational
    );
    let stopped_curved = qualify_curved_region(
        &ToleranceContext::default_valid(),
        &s,
        &[wire.clone(), make_hole(0.05, 0.15), make_hole(0.7, 0.85)],
        &curved,
        0,
        0,
        2,
        1e-8,
        limits,
        10000,
        [1e-7, 1e-7],
        16,
        16,
        0,
        1000,
        10000,
        1,
    )
    .unwrap();
    assert!(stopped_curved.region.is_none() && !stopped_curved.interior.region_subset_proven);
    let stopped = propose(
        &ToleranceContext::default_valid(),
        &s,
        &[wire],
        &contact,
        0,
        0,
        2,
        1e-8,
        limits,
        1,
        [1e-7, 1e-7],
        16,
        16,
    )
    .unwrap();
    assert!(!stopped.source_joins_proven && stopped.candidate_loops.is_none());
}
