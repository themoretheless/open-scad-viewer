use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{
            PcurvePullbackRefusal, PcurveTangentOutcome, compare_pcurve_tangent_planes,
        },
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn face(u: [f64; 2], v: [f64; 2], mapping: impl Fn(f64, f64) -> Vec<f64>) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![u[0], u[0], u[1], u[1]],
        knots_v: vec![v[0], v[0], v[1], v[1]],
        control_points: u
            .iter()
            .map(|&u| v.iter().map(|&v| mapping(u, v)).collect())
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn trace(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![3., 3., 7., 7.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1.; 2],
        periodic: false,
    }
}

fn check(right: &Surface, work: u64) -> PcurveTangentOutcome {
    let left = face([0., 2.], [0., 4.], |u, v| vec![u, v, 0.]);
    let a = trace([2., 0.], [2., 4.]);
    let b = trace([0., 0.], [4., 0.]);
    compare_pcurve_tangent_planes((&left, &a), (right, &b), work)
        .unwrap()
        .outcome
}

#[test]
fn accepts_opposite_normal_orientation_and_distinct_uv_axes() {
    let right = face([0., 4.], [0., 2.], |u, v| vec![2. + v, u, 0.]);
    assert_eq!(check(&right, WORK), PcurveTangentOutcome::Equal);
}

#[test]
fn distinguishes_a_crease_from_a_gap() {
    let crease = face([0., 4.], [0., 2.], |u, v| vec![2. + v, u, v]);
    assert_eq!(
        check(&crease, WORK),
        PcurveTangentOutcome::TangentPlaneMismatch {
            parameter: [3., 7.]
        }
    );
    let gap = face([0., 4.], [0., 2.], |u, v| vec![3. + v, u, 0.]);
    assert_eq!(
        check(&gap, WORK),
        PcurveTangentOutcome::PositionMismatch {
            parameter: [3., 7.],
            coordinate: 0
        }
    );
}

#[test]
fn finds_an_isolated_singular_normal_and_preserves_the_budget() {
    let folded = face([0., 4.], [0., 2.], |u, v| vec![2. + v * (u - 2.), u, 0.]);
    assert_eq!(
        check(&folded, WORK),
        PcurveTangentOutcome::Singular {
            parameter: [3., 7.],
            face: 1
        }
    );
    assert_eq!(
        check(&folded, 1),
        PcurveTangentOutcome::Refused(PcurvePullbackRefusal::Geometry(Refusal::ResourceLimit))
    );
}

#[test]
fn matches_varying_normals_on_a_rational_saddle_with_swapped_axes() {
    let mut left = face([0., 2.], [0., 4.], |u, v| vec![u, v, u * v]);
    left.weights = vec![vec![1., 2.], vec![2., 4.]];
    let mut right = face([0., 4.], [0., 2.], |v, u| vec![u, v, u * v]);
    right.weights = vec![vec![1., 2.], vec![2., 4.]];
    let a = trace([0., 0.], [2., 4.]);
    let b = trace([0., 0.], [4., 2.]);
    let report = compare_pcurve_tangent_planes((&left, &a), (&right, &b), WORK).unwrap();
    assert_eq!(report.outcome, PcurveTangentOutcome::Equal);
    assert!(report.work_used <= WORK);
}
