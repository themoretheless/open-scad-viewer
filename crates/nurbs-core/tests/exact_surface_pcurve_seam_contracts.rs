use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{
            PcurveEdgeAgreement, PcurvePullbackRefusal, compare_surface_pcurve_seam,
        },
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn left_face() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 2., 2.],
        knots_v: vec![0., 0., 4., 4.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![2., 0., 0.], vec![2., 4., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn right_face() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 2., 4., 4.],
        knots_v: vec![0., 0., 2., 2.],
        control_points: [0., 2., 4.]
            .into_iter()
            .map(|u| vec![vec![2., u, 0.], vec![4., u, 0.]])
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

fn left_uv() -> Curve {
    Curve {
        degree: 1,
        knots: vec![3., 3., 4., 7., 7.],
        control_points: vec![vec![2., 0.], vec![2., 1.], vec![2., 4.]],
        weights: vec![1.; 3],
        periodic: false,
    }
}

fn right_uv() -> Curve {
    Curve {
        degree: 1,
        knots: vec![3., 3., 7., 7.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    }
}

#[test]
fn confirms_same_seam_on_different_uv_axes_and_different_knot_partitions() {
    let report = compare_surface_pcurve_seam(
        (&left_face(), &left_uv()),
        (&right_face(), &right_uv()),
        WORK,
    )
    .unwrap();
    assert_eq!(report.outcome, PcurveEdgeAgreement::Equal);
    assert!(report.work_used <= WORK);
}

#[test]
fn locates_a_crack_between_matching_boundary_endpoints() {
    let mut right = right_face();
    right.control_points[1][0][2] = 0.125;
    let report =
        compare_surface_pcurve_seam((&left_face(), &left_uv()), (&right, &right_uv()), WORK)
            .unwrap();
    assert_eq!(
        report.outcome,
        PcurveEdgeAgreement::Different {
            parameter: [3., 4.],
            coordinate: 2
        }
    );
}

#[test]
fn domain_and_budget_failures_are_not_geometric_cracks() {
    let mut outside = right_uv();
    outside.control_points[1][1] = -1.;
    let report =
        compare_surface_pcurve_seam((&left_face(), &left_uv()), (&right_face(), &outside), WORK)
            .unwrap();
    assert_eq!(
        report.outcome,
        PcurveEdgeAgreement::Refused(PcurvePullbackRefusal::UnprovedChartDomain)
    );
    let limited =
        compare_surface_pcurve_seam((&left_face(), &left_uv()), (&right_face(), &right_uv()), 1)
            .unwrap();
    assert_eq!(
        limited.outcome,
        PcurveEdgeAgreement::Refused(PcurvePullbackRefusal::Geometry(Refusal::ResourceLimit))
    );
    assert!(limited.work_used <= 1);
}
