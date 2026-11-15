use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{
            BezierSpanLayout, PcurvePullbackRefusal, TrimmedMobiusRefusal, relayout_bezier_surface,
            relayout_trimmed_bezier_surface,
        },
    },
    surface::Surface,
    trim::trim_domain::exact_loop_joins,
};

const WORK: u64 = 1_000_000;
const LAYOUTS: [BezierSpanLayout<'_>; 2] = [
    BezierSpanLayout {
        breaks: &[10., 20., 40.],
        factors: &[2., 0.5],
    },
    BezierSpanLayout {
        breaks: &[-9., 1., 11.],
        factors: &[0.5, 2.],
    },
];

fn surface() -> Surface {
    let params = [0., 5., 10.];
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 5., 10., 10.],
        knots_v: vec![0., 0., 5., 10., 10.],
        control_points: params
            .iter()
            .map(|&u| params.iter().map(|&v| vec![u, v, u * v]).collect())
            .collect(),
        weights: vec![vec![1., 2., 1.], vec![2., 4., 2.], vec![1., 2., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn contour(corners: [[f64; 2]; 4]) -> Vec<Curve> {
    (0..4)
        .map(|i| Curve {
            degree: 1,
            knots: vec![0., 0., 2., 2.],
            control_points: vec![corners[i].to_vec(), corners[(i + 1) % 4].to_vec()],
            weights: vec![1.; 2],
            periodic: false,
        })
        .collect()
}

fn outer() -> Vec<Curve> {
    contour([[0., 0.], [10., 0.], [10., 10.], [0., 10.]])
}

#[test]
fn relayouts_outer_boundary_and_hole_across_both_original_grid_lines() {
    let source = surface();
    let contours = [outer(), contour([[2., 3.], [2., 7.], [8., 7.], [8., 3.]])];
    let report = relayout_trimmed_bezier_surface(&source, &contours, LAYOUTS, WORK).unwrap();
    let out = report
        .geometry
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    for (before, after) in contours.iter().zip(&out.loops) {
        assert_eq!(after.len(), 8);
        assert_eq!(exact_loop_joins(after).unwrap(), Some(true));
        for (original, pieces) in before.iter().zip(after.chunks_exact(2)) {
            assert_eq!(pieces[0].domain(), [0., 1.]);
            assert_eq!(pieces[1].domain(), [1., 2.]);
            for piece in pieces {
                let [a, b] = piece.domain();
                for t in [a, (a + b) / 2., b] {
                    let uv = original.evaluate(t).unwrap().point;
                    let before = source.evaluate(uv[0], uv[1]).unwrap().point;
                    let uv = piece.evaluate(t).unwrap().point;
                    let after = out.surface.evaluate(uv[0], uv[1]).unwrap().point;
                    assert!(
                        before
                            .iter()
                            .zip(after)
                            .all(|(p, q)| (p - q).abs() <= 1e-12)
                    );
                }
            }
        }
    }
}

#[test]
fn refuses_a_late_nonrepresentable_grid_cut_without_returning_the_surface() {
    let contours = [outer(), contour([[2., 3.], [2., 7.], [7., 7.], [7., 3.]])];
    let result = relayout_trimmed_bezier_surface(&surface(), &contours, LAYOUTS, WORK).unwrap();
    assert!(result.geometry.is_none());
    assert_eq!(
        result.refusal,
        Some(TrimmedMobiusRefusal::Curve {
            loop_index: 1,
            curve_index: 1,
            reason: PcurvePullbackRefusal::Geometry(Refusal::NotRepresentable),
        })
    );
}

#[test]
fn surface_and_all_trim_pieces_share_one_budget() {
    let source = surface();
    let budget = relayout_bezier_surface(&source, LAYOUTS, WORK)
        .unwrap()
        .work_used;
    let report = relayout_trimmed_bezier_surface(&source, &[outer()], LAYOUTS, budget).unwrap();
    assert_eq!(report.work_used, budget);
    assert_eq!(report.refusal, Some(TrimmedMobiusRefusal::ResourceLimit));
    assert!(report.geometry.is_none());
}
