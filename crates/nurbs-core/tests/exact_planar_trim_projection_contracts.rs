use nurbs_core::{
    analysis::planar_trim_projection::{PlanarTrimOutcome, project_coplanar_trim_loops},
    curve::Curve,
    surface::Surface,
    trim::trim_domain::exact_loop_joins,
};

fn face(x: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [x, x + 1.]
            .into_iter()
            .map(|u| [0., 1.].into_iter().map(|v| vec![u, v, u + v]).collect())
            .collect(),
        weights: vec![vec![1., 2.], vec![4., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn contour(lo: f64, hi: f64) -> Vec<Curve> {
    vec![Curve {
        degree: 1,
        knots: vec![3., 3., 4., 5., 6., 7., 7.],
        control_points: vec![
            vec![lo, lo],
            vec![hi, lo],
            vec![hi, hi],
            vec![lo, hi],
            vec![lo, lo],
        ],
        weights: vec![1.; 5],
        periodic: false,
    }]
}

#[test]
fn shares_exact_plane_coordinates_for_two_trimmed_faces() {
    let patches = [face(0.), face(1.)];
    let trims = [vec![contour(0., 1.)], vec![contour(0., 1.)]];
    let report = project_coplanar_trim_loops(&patches, &trims, 1_000_000).unwrap();
    let PlanarTrimOutcome::Projected(out) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(
        out[0].projection.retained_axes,
        out[1].projection.retained_axes
    );
    for (face, &(i, j)) in out.iter().zip(&[(0, 0), (1, 0)]) {
        let pieces = &face.loops[j];
        assert_eq!(pieces.len(), 4);
        assert_eq!(exact_loop_joins(pieces).unwrap(), Some(true));
        for piece in pieces {
            let [a, b] = piece.domain();
            let t = a.midpoint(b);
            let uv = trims[i][j][0].evaluate(t).unwrap().point;
            let p = patches[i].evaluate(uv[0], uv[1]).unwrap().point;
            let q = piece.evaluate(t).unwrap().point;
            let [x, y] = face.projection.retained_axes;
            assert!((p[x] - q[0]).abs() < 1e-13 && (p[y] - q[1]).abs() < 1e-13);
        }
    }
}

#[test]
fn refuses_an_open_or_outside_contour_without_partially_returning_faces() {
    let patches = [face(0.), face(1.)];
    let mut open = contour(0., 1.);
    open[0].control_points[4] = vec![0., 0.5];
    let report =
        project_coplanar_trim_loops(&patches, &[vec![contour(0., 1.)], vec![open]], 1_000_000)
            .unwrap();
    assert!(matches!(
        report.outcome,
        PlanarTrimOutcome::OpenLoop {
            face: 1,
            contour: 0
        }
    ));
    let report = project_coplanar_trim_loops(
        &patches,
        &[vec![contour(0., 1.)], vec![contour(0., 1.5)]],
        1_000_000,
    )
    .unwrap();
    assert!(matches!(
        report.outcome,
        PlanarTrimOutcome::TraceRefused { face: 1, .. }
    ));
}
