use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{
            PcurveEdgeAgreement, PcurvePullbackRefusal, compare_pcurve_edge, lift_pcurve_to_surface,
        },
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn patch(end: f64, twisted: bool) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 2., end, end],
        knots_v: vec![0., 0., 4., 4.],
        control_points: [0., 2., end]
            .into_iter()
            .map(|u| {
                vec![
                    vec![u, 0., 0.],
                    vec![u, 4., if twisted { 4. * u } else { 0. }],
                ]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

fn line(a: Vec<f64>, b: Vec<f64>, weights: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![3., 3., 7., 7.],
        control_points: vec![a, b],
        weights,
        periodic: false,
    }
}

fn quadratic_edge() -> Curve {
    Curve {
        degree: 2,
        knots: vec![3., 3., 3., 4., 4., 7., 7., 7.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![0.5, 0.5, 0.],
            vec![1., 1., 1.],
            vec![2.5, 2.5, 4.],
            vec![4., 4., 16.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    }
}

#[test]
fn proves_equality_through_different_curve_and_surface_knot_partitions() {
    let surface = patch(4., true);
    let uv = line(vec![0., 0.], vec![4., 4.], vec![1.; 2]);
    let result = compare_pcurve_edge(&surface, &uv, &quadratic_edge(), WORK).unwrap();
    assert_eq!(result.outcome, PcurveEdgeAgreement::Equal);
    assert!(result.work_used <= WORK);
}

#[test]
fn identifies_the_first_nonzero_polynomial_even_with_matching_endpoints() {
    let surface = patch(4., true);
    let uv = line(vec![0., 0.], vec![4., 4.], vec![1.; 2]);
    let mut edge = quadratic_edge();
    edge.control_points[3][2] += 1.;
    let result = compare_pcurve_edge(&surface, &uv, &edge, WORK).unwrap();
    assert_eq!(
        result.outcome,
        PcurveEdgeAgreement::Different {
            parameter: [4., 5.],
            coordinate: 2
        }
    );
}

#[test]
fn verifies_an_authored_edge_when_a_lifted_piece_cannot_be_stored() {
    let surface = patch(3., false);
    let uv = line(vec![0., 0.], vec![3., 1.], vec![1., 2.]);
    let edge = line(vec![0., 0., 0.], vec![3., 1., 0.], vec![0.5, 1.]);
    let lifting = lift_pcurve_to_surface(&surface, &uv, WORK).unwrap();
    assert!(lifting.pieces.is_none());
    assert_eq!(
        lifting.refusal,
        Some(PcurvePullbackRefusal::Geometry(Refusal::NotRepresentable))
    );
    let comparison = compare_pcurve_edge(&surface, &uv, &edge, WORK).unwrap();
    assert_eq!(comparison.outcome, PcurveEdgeAgreement::Equal);
    let limited = compare_pcurve_edge(&surface, &uv, &edge, 1).unwrap();
    assert_eq!(
        limited.outcome,
        PcurveEdgeAgreement::Refused(PcurvePullbackRefusal::Geometry(Refusal::ResourceLimit))
    );
    assert!(limited.work_used <= 1);
}
