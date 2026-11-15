use nurbs_core::{
    curve::Curve,
    editing::exact_projective::{
        PcurvePullbackRefusal, TrimmedMobiusRefusal, mobius_spline_surface, mobius_trimmed_surface,
    },
    surface::Surface,
    trim::trim_domain::exact_loop_joins,
};

const WORK: u64 = 1_000_000;
const CLOCKS: [f64; 2] = [2., 0.5];

fn patch() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 15., 15.],
        knots_v: vec![0., 0., 15., 15.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 15., 1.]],
            vec![vec![15., 0., 2.], vec![15., 15., 9.]],
        ],
        weights: vec![vec![1., 2.], vec![4., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn curve(points: &[[f64; 2]]) -> Curve {
    let last = points.len() as f64 + 2.;
    let mut knots = vec![3.];
    knots.extend((3..=last as usize).map(|t| t as f64));
    knots.push(last);
    Curve {
        degree: 1,
        knots,
        control_points: points.iter().map(|point| point.to_vec()).collect(),
        weights: vec![1.; points.len()],
        periodic: false,
    }
}

fn contour(points: [[f64; 2]; 4]) -> Vec<Curve> {
    (0..4)
        .map(|i| curve(&[points[i], points[(i + 1) % 4]]))
        .collect()
}

fn outer() -> Vec<Curve> {
    let mut curves = contour([[0., 0.], [15., 0.], [15., 15.], [0., 15.]]);
    curves[0] = curve(&[[0., 0.], [5., 0.], [15., 0.]]);
    curves
}

#[test]
fn preserves_the_world_space_outer_boundary_and_hole_with_original_parameters() {
    let source = patch();
    let original = [outer(), contour([[5., 3.], [5., 9.], [6., 9.], [6., 3.]])];
    let report = mobius_trimmed_surface(&source, &original, CLOCKS, WORK).unwrap();
    let reclocked = report
        .geometry
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert!(report.work_used <= WORK);
    assert_eq!(reclocked.loops[0].len(), 5);
    assert_eq!(reclocked.loops[1].len(), 4);

    for (old_loop, new_loop) in original.iter().zip(&reclocked.loops) {
        assert_eq!(exact_loop_joins(new_loop).unwrap(), Some(true));
        let mut pieces = new_loop.iter();
        for old in old_loop {
            for piece in pieces.by_ref().take(old.control_points.len() - 1) {
                let [a, b] = piece.domain();
                for param in [a, (a + b) / 2., b] {
                    let uv_old = old.evaluate(param).unwrap().point;
                    let uv_new = piece.evaluate(param).unwrap().point;
                    let before = source.evaluate(uv_old[0], uv_old[1]).unwrap().point;
                    let after = reclocked
                        .surface
                        .evaluate(uv_new[0], uv_new[1])
                        .unwrap()
                        .point;
                    assert!(
                        before
                            .iter()
                            .zip(after)
                            .all(|(left, right)| (left - right).abs() < 1e-12)
                    );
                }
            }
        }
    }
}

#[test]
fn late_out_of_chart_contour_withholds_every_result() {
    let loops = [outer(), contour([[5., 3.], [5., 9.], [16., 9.], [16., 3.]])];
    let report = mobius_trimmed_surface(&patch(), &loops, CLOCKS, WORK).unwrap();
    assert!(report.geometry.is_none());
    assert_eq!(
        report.refusal,
        Some(TrimmedMobiusRefusal::Curve {
            loop_index: 1,
            curve_index: 1,
            reason: PcurvePullbackRefusal::UnprovedChartDomain,
        }),
    );
}

#[test]
fn open_source_and_exhausted_shared_budget_return_no_geometry() {
    let source = patch();
    let mut open = outer();
    open.pop();
    let report = mobius_trimmed_surface(&source, &[open], CLOCKS, WORK).unwrap();
    assert!(report.geometry.is_none());
    assert_eq!(
        report.refusal,
        Some(TrimmedMobiusRefusal::OriginalLoopClosure { loop_index: 0 })
    );
    let budget = mobius_spline_surface(&source, CLOCKS, WORK)
        .unwrap()
        .work_used;
    let report = mobius_trimmed_surface(&source, &[outer()], CLOCKS, budget).unwrap();
    assert!(report.geometry.is_none());
    assert_eq!(report.work_used, budget);
    assert_eq!(report.refusal, Some(TrimmedMobiusRefusal::ResourceLimit));
}
