use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{
            PcurvePullbackRefusal, TrimmedMobiusRefusal, affine_trimmed_surface_clock,
        },
    },
    surface::Surface,
    trim::trim_domain::exact_loop_joins,
};

const WORK: u64 = 1_000_000;
const NEW: [[f64; 2]; 2] = [[10., 2.], [-3., -1.]];

fn patch(u: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., u, u],
        knots_v: vec![0., 0., 4., 4.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 1.]],
            vec![vec![u, 0., 2.], vec![u, 4., 9.]],
        ],
        weights: vec![vec![1., 2.], vec![4., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn polygon(points: [[f64; 2]; 4]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![3., 3., 4., 5., 6., 7., 7.],
        control_points: points
            .into_iter()
            .chain([points[0]])
            .map(Vec::from)
            .collect(),
        weights: vec![1., 2., 4., 2., 1.],
        periodic: false,
    }
}

fn outer() -> Curve {
    polygon([[0., 0.], [4., 0.], [4., 4.], [0., 4.]])
}

#[test]
fn preserves_rational_outer_and_inner_loop_traces_and_their_original_curve_clocks() {
    let source = patch(4.);
    let loops = [
        vec![outer()],
        vec![polygon([[1., 1.], [1., 3.], [3., 3.], [3., 1.]])],
    ];
    let result = affine_trimmed_surface_clock(&source, &loops, NEW, WORK).unwrap();
    let out = result
        .geometry
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    for (before, after) in loops.iter().zip(&out.loops) {
        assert_eq!(after.len(), 1);
        assert_eq!(exact_loop_joins(after).unwrap(), Some(true));
        let original = &before[0];
        let changed = &after[0];
        assert_eq!(original.knots, changed.knots);
        assert_eq!(original.weights, changed.weights);
        for t in [3., 3.5, 4., 4.5, 5., 5.5, 6., 6.5, 7.] {
            let p = original.evaluate(t).unwrap().point;
            let q = changed.evaluate(t).unwrap().point;
            assert!((q[0] - (10. - 2. * p[0])).abs() < 1e-13);
            assert!((q[1] - (-3. + p[1] / 2.)).abs() < 1e-13);
            let a = source.evaluate(p[0], p[1]).unwrap().point;
            let b = out.surface.evaluate(q[0], q[1]).unwrap().point;
            assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12));
        }
    }
}

#[test]
fn refuses_a_nonrepresentable_trim_pole_even_when_surface_knots_fit() {
    let loops = [vec![polygon([[0., 0.], [1., 0.], [1., 3.], [0., 3.]])]];
    let result =
        affine_trimmed_surface_clock(&patch(3.), &loops, [[0., 1.], [0., 4.]], WORK).unwrap();
    assert!(result.geometry.is_none());
    assert_eq!(
        result.refusal,
        Some(TrimmedMobiusRefusal::Curve {
            loop_index: 0,
            curve_index: 0,
            reason: PcurvePullbackRefusal::Geometry(Refusal::NotRepresentable),
        })
    );
}

#[test]
fn checks_chart_containment_and_shared_work_budget() {
    let input = patch(4.);
    let loops = [vec![polygon([[0., 0.], [5., 0.], [4., 4.], [0., 4.]])]];
    let result = affine_trimmed_surface_clock(&input, &loops, NEW, WORK).unwrap();
    assert!(result.geometry.is_none());
    assert_eq!(
        result.refusal,
        Some(TrimmedMobiusRefusal::Curve {
            loop_index: 0,
            curve_index: 0,
            reason: PcurvePullbackRefusal::UnprovedChartDomain,
        })
    );
    let budget = nurbs_core::editing::exact_projective::affine_surface_clock(&input, NEW, WORK)
        .unwrap()
        .work_used;
    let limited = affine_trimmed_surface_clock(&input, &[vec![outer()]], NEW, budget).unwrap();
    assert!(limited.geometry.is_none());
    assert_eq!(limited.refusal, Some(TrimmedMobiusRefusal::ResourceLimit));
    assert_eq!(limited.work_used, budget);
}
