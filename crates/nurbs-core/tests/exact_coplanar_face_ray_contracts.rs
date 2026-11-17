use num_rational::BigRational as Q;
use nurbs_core::{
    analysis::{
        planar_coplanar_ray::{
            CoplanarRayContactKind, CoplanarRayOutcome, contact_coplanar_face_ray,
        },
        planar_trim_projection::{PlanarTrimOutcome, project_coplanar_trim_loops},
    },
    curve::Curve,
    surface::Surface,
};
const WORK: u64 = 1_000_000;
fn contour(a: f64, b: f64, reverse: bool) -> Vec<Curve> {
    let mut points = vec![vec![a, a], vec![b, a], vec![b, b], vec![a, b], vec![a, a]];
    if reverse {
        points.reverse();
    }
    vec![Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 4., 4.],
        control_points: points,
        weights: vec![1.; 5],
        periodic: false,
    }]
}

fn face() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 1.]],
            vec![vec![1., 0., 1.], vec![1., 1., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn projected(reverse: bool) -> nurbs_core::analysis::planar_trim_projection::ProjectedTrimmedFace {
    let report = project_coplanar_trim_loops(
        &[face()],
        &[vec![
            contour(0., 1., reverse),
            contour(0.25, 0.75, !reverse),
        ]],
        1_000_000,
    )
    .unwrap();
    let PlanarTrimOutcome::Projected(mut charts) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    charts.remove(0)
}

#[test]
fn finds_forward_contacts_with_hole_and_outer_boundary() {
    let report =
        contact_coplanar_face_ray(&projected(false), [0.5, 0.5, 1.], [1., 0., 1.], 1e-9, WORK)
            .unwrap();
    let CoplanarRayOutcome::Contacts(contacts) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(contacts.len(), 2);
    for contact in contacts {
        assert_eq!(contact.kind, CoplanarRayContactKind::Crossing);
        let expected = Q::new(1.into(), if contact.contour == 0 { 2 } else { 4 }.into());
        assert!(contact.ray_parameter[0] <= expected && expected <= contact.ray_parameter[1]);
    }
}

#[test]
fn retains_coincident_span_and_both_end_contacts() {
    let report = contact_coplanar_face_ray(
        &projected(false),
        [-0.5, 0., -0.5],
        [1., 0., 1.],
        1e-9,
        WORK,
    )
    .unwrap();
    let CoplanarRayOutcome::Contacts(contacts) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(contacts.len(), 3);
    let span = contacts
        .iter()
        .find(|c| c.kind == CoplanarRayContactKind::CoincidentSpan)
        .unwrap();
    assert!(span.ray_parameter[0] <= Q::new(1.into(), 2.into()));
    assert!(span.ray_parameter[1] >= Q::new(3.into(), 2.into()));
    assert_eq!(
        contacts
            .iter()
            .filter(|c| c.kind == CoplanarRayContactKind::Endpoint)
            .count(),
        2
    );
}

#[test]
fn rejects_noncoplanar_and_returns_no_partial_events_on_exhaustion() {
    let face = projected(false);
    let other = contact_coplanar_face_ray(&face, [0., 0., 1.], [1., 0., 1.], 1e-9, WORK).unwrap();
    assert_eq!(other.outcome, CoplanarRayOutcome::NotCoplanar);
    let limited = contact_coplanar_face_ray(&face, [0., 0., 0.], [1., 0., 1.], 1e-9, 1).unwrap();
    assert!(matches!(limited.outcome, CoplanarRayOutcome::Refused(_)));
    assert!(limited.work_used <= 1);
}
