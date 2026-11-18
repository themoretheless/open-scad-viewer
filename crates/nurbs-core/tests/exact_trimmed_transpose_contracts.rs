use nurbs_core::{
    curve::Curve,
    editing::exact_projective::{
        PcurvePullbackRefusal, TrimmedMobiusRefusal, transpose_trimmed_surface,
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn face() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![2., 2., 6., 6.],
        knots_v: vec![-4., -4., -4., 0., 0., 0.],
        control_points: [2., 6.]
            .into_iter()
            .map(|u| {
                [-4., -2., 0.]
                    .into_iter()
                    .map(|v| vec![u, v, u * v])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., 2., 3.], vec![4., 1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn loop_curve(points: [[f64; 2]; 4]) -> Vec<Curve> {
    vec![Curve {
        degree: 1,
        knots: vec![3., 3., 4., 5., 6., 7., 7.],
        control_points: points
            .into_iter()
            .chain([points[0]])
            .map(Vec::from)
            .collect(),
        weights: vec![1., 2., 4., 2., 1.],
        periodic: false,
    }]
}

#[test]
fn transposes_unequal_degrees_and_closed_rational_outer_and_inner_loops() {
    let face = face();
    let loops = [
        loop_curve([[2., -4.], [6., -4.], [6., 0.], [2., 0.]]),
        loop_curve([[3., -3.], [3., -1.], [5., -1.], [5., -3.]]),
    ];
    let report = transpose_trimmed_surface(&face, &loops, WORK).unwrap();
    let out = report
        .geometry
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!([out.surface.degree_u, out.surface.degree_v], [2, 1]);
    assert_eq!(out.surface.knots_u, face.knots_v);
    assert_eq!(out.surface.knots_v, face.knots_u);
    assert!(report.work_used <= WORK);
    for (old, new) in loops.iter().zip(&out.loops) {
        assert_eq!(old[0].knots, new[0].knots);
        assert_eq!(old[0].weights, new[0].weights);
        for (p, q) in old[0].control_points.iter().zip(&new[0].control_points) {
            assert_eq!([p[1], p[0]], [q[0], q[1]]);
        }
        for t in [3., 3.5, 4.25, 5., 5.75, 6.5, 7.] {
            let p = old[0].evaluate(t).unwrap().point;
            let q = new[0].evaluate(t).unwrap().point;
            let a = face.evaluate(p[0], p[1]).unwrap().point;
            let b = out.surface.evaluate(q[0], q[1]).unwrap().point;
            assert!(a.iter().zip(b).all(|(p, q)| (p - q).abs() < 1e-12));
        }
    }
    let recovered = transpose_trimmed_surface(&out.surface, &out.loops, WORK)
        .unwrap()
        .geometry
        .unwrap();
    assert_eq!(recovered.surface.control_points, face.control_points);
    assert_eq!(recovered.surface.weights, face.weights);
    assert_eq!(
        recovered.loops[1][0].control_points,
        loops[1][0].control_points
    );
}

#[test]
fn withholds_the_transposed_face_for_an_outside_trim_or_spent_budget() {
    let patch = face();
    let loops = [loop_curve([[2., -4.], [7., -4.], [6., 0.], [2., 0.]])];
    let report = transpose_trimmed_surface(&patch, &loops, WORK).unwrap();
    assert!(report.geometry.is_none());
    assert_eq!(
        report.refusal,
        Some(TrimmedMobiusRefusal::Curve {
            loop_index: 0,
            curve_index: 0,
            reason: PcurvePullbackRefusal::UnprovedChartDomain,
        })
    );
    let limited = transpose_trimmed_surface(&patch, &loops, 1).unwrap();
    assert!(limited.geometry.is_none());
    assert!(limited.work_used <= 1);
    assert!(limited.refusal.is_some());
}
