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

fn partition_options() -> nurbs_core::analysis::planar_coplanar_ray::CoplanarRayPartitionOptions {
    nurbs_core::analysis::planar_coplanar_ray::CoplanarRayPartitionOptions {
        max_curve_parameter_width: 1e-9,
        winding_tolerance: 1e-8,
        max_winding_cells: 1024,
        max_work: WORK,
    }
}

#[test]
fn certifies_two_material_gaps_separated_by_a_hole() {
    use nurbs_core::{
        analysis::planar_coplanar_ray::{CoplanarRayPartitionOutcome, partition_coplanar_face_ray},
        trim::trim_domain::Location,
    };
    let report = partition_coplanar_face_ray(
        &projected(false),
        [-1., 0.5, -0.5],
        [1., 0., 1.],
        partition_options(),
    )
    .unwrap();
    let CoplanarRayPartitionOutcome::Partition(partition) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(partition.contact_bands.len(), 4);
    assert_eq!(
        partition
            .gaps
            .iter()
            .map(|g| g.location)
            .collect::<Vec<_>>(),
        [
            Location::Outside,
            Location::Inside,
            Location::Outside,
            Location::Inside
        ]
    );
    for (range, t) in partition.contact_bands.into_iter().zip([
        Q::from_integer(1.into()),
        Q::new(5.into(), 4.into()),
        Q::new(7.into(), 4.into()),
        Q::from_integer(2.into()),
    ]) {
        assert!(range[0] <= t && t <= range[1]);
    }
    assert!(partition.outside_after >= Q::from_integer(2.into()));
    assert!(report.winding_cells <= 1024);
}

#[test]
fn merges_coincident_spans_and_retains_unresolved_gaps() {
    use nurbs_core::{
        analysis::planar_coplanar_ray::{
            CoplanarRayPartitionOptions, CoplanarRayPartitionOutcome, partition_coplanar_face_ray,
        },
        trim::trim_domain::Location,
    };
    let opts = CoplanarRayPartitionOptions {
        max_winding_cells: 1,
        ..partition_options()
    };
    let report =
        partition_coplanar_face_ray(&projected(false), [-0.5, 0., -0.5], [1., 0., 1.], opts)
            .unwrap();
    let CoplanarRayPartitionOutcome::Partition(partition) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(partition.contact_bands.len(), 1);
    assert_eq!(partition.gaps.len(), 1);
    assert_eq!(partition.gaps[0].location, Location::Unresolved);
    assert_eq!(report.winding_cells, 1);
}
