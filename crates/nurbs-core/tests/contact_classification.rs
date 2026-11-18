use nurbs_core::{
    intersection::{
        ContactClass,
        surface_contact_class::{ContactSideEvidence, classify_surface_contact},
    },
    surface::Surface,
};

fn plane(vertical: bool) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        weights: vec![vec![1.; 2]; 2],
        control_points: vec![
            vec![
                vec![0., 0., 0.],
                if vertical { vec![0., 0., 1.] } else { vec![0., 1., 0.] },
            ],
            vec![
                vec![1., 0., 0.],
                if vertical { vec![1., 0., 1.] } else { vec![1., 1., 0.] },
            ],
        ],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn normal_alignment_alone_does_not_invent_tangency_parity() {
    let plane = plane(false);
    let report = classify_surface_contact(
        &plane,
        [0.5, 0.5],
        &plane,
        [0.25, 0.75],
        1e-9,
        1e-8,
        None,
    )
    .unwrap();
    assert_eq!(report.class, ContactClass::HigherOrderContact);
    assert!(!report.parity_proven);
    assert_eq!(report.normal_alignment, 1.);
}

#[test]
fn signed_side_intervals_distinguish_even_odd_and_unresolved_contact() {
    let plane = plane(false);
    for (evidence, expected, proven) in [
        (
            ContactSideEvidence {
                first: [1e-6, 2e-6],
                second: [-2e-6, -1e-6],
            },
            ContactClass::EvenTangency,
            true,
        ),
        (
            ContactSideEvidence {
                first: [-1e-6, 1e-6],
                second: [-2e-6, 2e-6],
            },
            ContactClass::OddTangency,
            true,
        ),
        (
            ContactSideEvidence {
                first: [0., 1e-6],
                second: [-1e-6, 1e-6],
            },
            ContactClass::HigherOrderContact,
            false,
        ),
    ] {
        let report = classify_surface_contact(
            &plane,
            [0.5, 0.5],
            &plane,
            [0.5, 0.5],
            1e-9,
            1e-8,
            Some(evidence),
        )
        .unwrap();
        assert_eq!(report.class, expected);
        assert_eq!(report.parity_proven, proven);
    }
}

#[test]
fn transverse_contact_does_not_require_side_evidence() {
    let report = classify_surface_contact(
        &plane(false),
        [0.5, 0.],
        &plane(true),
        [0.5, 0.],
        1e-9,
        1e-8,
        None,
    )
    .unwrap();
    assert_eq!(report.class, ContactClass::Transverse);
}
