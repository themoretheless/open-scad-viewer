use nurbs_core::{
    editing::exact_surface_derivative::{
        CurvatureGapEdge, EdgeThresholdRoots, Refusal, curvature_gap_edge_roots,
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn parabola() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![2., 2., 2., 6., 6., 6.],
        knots_v: vec![7., 7., 9., 9.],
        control_points: [(0., 0.), (0.5, 0.), (1., 1.)]
            .map(|(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
            .to_vec(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn isolates_one_simple_crossing_with_the_original_parameter_clock() {
    let s = parabola();
    for edge in [CurvatureGapEdge::VMin, CurvatureGapEdge::VMax] {
        let r = curvature_gap_edge_roots(&s, [2, 1], edge, 1., 12, WORK).unwrap();
        assert_eq!(r.varying_domain, [2., 6.]);
        let EdgeThresholdRoots::Isolated {
            exact_points,
            brackets,
        } = r.roots.unwrap_or_else(|| panic!("{:?}", r.refusal))
        else {
            panic!("expected one simple crossing")
        };
        assert!(exact_points.is_empty());
        assert_eq!(brackets.len(), 1);
        assert_eq!(brackets[0].variation, 1);
        let [lo, hi] = brackets[0].unit_interval;
        let root = ((4f64.cbrt() - 1.) / 4.).sqrt();
        assert!(lo < root && root < hi);
        assert_eq!(hi - lo, 1. / 4096.);
    }
}

#[test]
fn represents_an_exact_endpoint_without_an_interior_bracket() {
    let r = curvature_gap_edge_roots(&parabola(), [2, 1], CurvatureGapEdge::VMin, 4., 12, WORK)
        .unwrap();
    assert_eq!(
        r.roots,
        Some(EdgeThresholdRoots::Isolated {
            exact_points: vec![0.],
            brackets: vec![]
        })
    );
}

#[test]
fn distinguishes_coincident_and_disjoint_edges() {
    let s = parabola();
    let left = curvature_gap_edge_roots(&s, [2, 1], CurvatureGapEdge::UMin, 4., 8, WORK).unwrap();
    assert_eq!(left.varying_domain, [7., 9.]);
    assert_eq!(left.roots, Some(EdgeThresholdRoots::Coincident));
    let right = curvature_gap_edge_roots(&s, [2, 1], CurvatureGapEdge::UMax, 4., 8, WORK).unwrap();
    assert_eq!(
        right.roots,
        Some(EdgeThresholdRoots::Isolated {
            exact_points: vec![],
            brackets: vec![]
        })
    );
}

#[test]
fn validates_original_span_and_refuses_short_work_atomically() {
    let s = parabola();
    let edge = CurvatureGapEdge::VMin;
    assert!(curvature_gap_edge_roots(&s, [0, 1], edge, 1., 12, WORK).is_err());
    assert!(curvature_gap_edge_roots(&s, [2, 1], edge, 1., 17, WORK).is_err());
    let full = curvature_gap_edge_roots(&s, [2, 1], edge, 1., 12, WORK).unwrap();
    let short = curvature_gap_edge_roots(&s, [2, 1], edge, 1., 12, full.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.roots.is_none());
}
