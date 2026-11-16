use nurbs_core::{
    curve::Curve,
    editing::exact_projective::{
        TrimmedAxisOrder, TrimmedMobiusRefusal, remap_trimmed_surface_axes,
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;
const CLOCKS: [[f64; 2]; 2] = [[9., 1.], [-2., 0.]];

fn input() -> (Surface, Vec<Vec<Curve>>) {
    let patch = Surface {
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
        weights: vec![vec![1., 2., 3.], vec![4., 2., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let loop_curve = |points: [[f64; 2]; 4]| {
        vec![Curve {
            degree: 1,
            knots: vec![3., 3., 4., 5., 6., 7., 7.],
            weights: vec![1., 2., 4., 2., 1.],
            periodic: false,
            control_points: points
                .into_iter()
                .chain([points[0]])
                .map(Vec::from)
                .collect(),
        }]
    };
    (
        patch,
        vec![
            loop_curve([[2., -4.], [6., -4.], [6., 0.], [2., 0.]]),
            loop_curve([[3., -3.], [3., -1.], [5., -1.], [5., -3.]]),
        ],
    )
}

#[test]
fn applies_signed_reversed_axis_mapping_in_output_order() {
    let (source, loops) = input();
    let report =
        remap_trimmed_surface_axes(&source, &loops, TrimmedAxisOrder::Vu, CLOCKS, WORK).unwrap();
    let out = report
        .geometry
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!(out.surface.knots_u, [1., 1., 1., 9., 9., 9.]);
    assert_eq!(out.surface.knots_v, [-2., -2., 0., 0.]);
    assert_eq!([out.surface.degree_u, out.surface.degree_v], [2, 1]);
    assert_eq!(out.surface.weights[0], [3., 1.]);
    for (before, after) in loops.iter().zip(&out.loops) {
        assert_eq!(before[0].knots, after[0].knots);
        assert_eq!(before[0].weights, after[0].weights);
        for t in [3., 3.5, 4., 5.5, 6.5, 7.] {
            let uv = before[0].evaluate(t).unwrap().point;
            let mapped = after[0].evaluate(t).unwrap().point;
            assert!((mapped[0] - (1. - 2. * uv[1])).abs() < 1e-13);
            assert!((mapped[1] - (uv[0] / 2. - 3.)).abs() < 1e-13);
            let p = source.evaluate(uv[0], uv[1]).unwrap().point;
            let q = out.surface.evaluate(mapped[0], mapped[1]).unwrap().point;
            assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-12));
        }
    }
    let restored = remap_trimmed_surface_axes(
        &out.surface,
        &out.loops,
        TrimmedAxisOrder::Vu,
        [[2., 6.], [0., -4.]],
        WORK,
    )
    .unwrap()
    .geometry
    .unwrap();
    assert_eq!(restored.surface.control_points, source.control_points);
    assert_eq!(restored.surface.weights, source.weights);
    assert_eq!(
        restored.loops[1][0].control_points,
        loops[1][0].control_points
    );
}

#[test]
fn a_later_contour_refusal_has_no_partial_surface() {
    let (source, mut loops) = input();
    loops[1][0].control_points[2][1] = 1.;
    let report =
        remap_trimmed_surface_axes(&source, &loops, TrimmedAxisOrder::Vu, CLOCKS, WORK).unwrap();
    assert!(report.geometry.is_none());
    assert!(matches!(
        report.refusal,
        Some(TrimmedMobiusRefusal::Curve {
            loop_index: 1,
            curve_index: 0,
            ..
        })
    ));
}
