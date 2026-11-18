use brep_core::shell_thickness::parametric_network::{Limits, MappedSources, thicken_from_maps};
use nurbs_core::{
    surface::Surface,
    surface_offset::{ProposalLimits as P, ProposalStop},
};
fn plane(x: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![x, 0., 0.], vec![x, 1., 0.]],
            vec![vec![x + 1., 0., 0.], vec![x + 1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn uv(rows: [[f64; 3]; 2]) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: rows
            .map(|r| {
                r.into_iter()
                    .zip([0.1, 0.5, 0.9])
                    .map(|(u, v)| vec![u, v, 0.])
                    .collect()
            })
            .to_vec(),
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn automatic_cap_refines_and_closes_with_replay_and_all_work_charged() {
    let source = plane(0.);
    let mut map = uv([[0.1, 0.16, 0.1], [0.8, 0.86, 0.8]]);
    map.knots_u = vec![-4., -4., 2., 2.];
    map.knots_v = vec![10., 10., 10., 14., 14., 14.];
    let saved = (source.clone(), map.clone());
    let budget = P::default();
    let r = thicken_from_maps(
        MappedSources {
            sources: &[source.clone()],
            maps: &[map.clone()],
            reversed: &[false],
        },
        0.1,
        0.0005,
        budget,
        1e-7,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(r.model().faces.len(), 6);
    assert_eq!(r.model().faces[0].surface, source);
    assert_eq!(r.model().validate().unwrap().boundary_edge_count, 0);
    assert!(r.constructed().cap_patches()[0].degree_u > 1);
    assert_eq!(r.proposals()[0].stop, ProposalStop::Certified);
    assert!(
        r.normal_cells() <= budget.proof_cells
            && r.coefficient_work() <= budget.coefficient_work
            && r.evaluations() <= budget.evaluations
    );
    assert_eq!(
        r.normal_cells(),
        r.proposals().iter().map(|p| p.proof_cells).sum::<usize>() + r.constructed().normal_cells()
    );
    assert!(r.reverify().unwrap().audit.ok);
    assert_eq!((source, map), saved);
}
#[test]
fn automatic_adjacent_caps_share_a_seam_and_only_exposed_sides_get_walls() {
    let sources = [plane(0.), plane(1.)];
    let maps = [
        uv([[0.1, 0.16, 0.1], [1.; 3]]),
        uv([[0.; 3], [0.9, 0.84, 0.9]]),
    ];
    let r = thicken_from_maps(
        MappedSources {
            sources: &sources,
            maps: &maps,
            reversed: &[false; 2],
        },
        0.1,
        0.0005,
        P::default(),
        1e-7,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(r.constructed().exposed_boundaries().len(), 6);
    assert_eq!(r.model().faces.len(), 10);
    assert_eq!(r.model().validate().unwrap().boundary_edge_count, 0);
    assert_eq!(r.proposals().len(), 2);
    let caps = r.constructed().cap_patches();
    let first = caps[0].control_points.last().unwrap();
    let second = &caps[1].control_points[0];
    assert_eq!(first.first(), second.first());
    assert_eq!(first.last(), second.last());
    assert!(r.reverify().unwrap().audit.ok);
}
#[test]
fn automatic_curved_source_cap_has_fresh_original_normal_and_closed_model_proofs() {
    let mut s = plane(0.);
    s.degree_u = 2;
    s.knots_u = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = [[1., 0.], [1., 1.], [0., 1.]]
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
        .to_vec();
    s.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
        .map(|w| vec![w; 2])
        .to_vec();
    let mut m = plane(0.);
    m.control_points = [0.1, 0.9]
        .map(|u| [0.1, 0.9].map(|v| vec![u, v, 0.]).to_vec())
        .to_vec();
    let r = thicken_from_maps(
        MappedSources {
            sources: &[s.clone()],
            maps: &[m],
            reversed: &[false],
        },
        0.1,
        0.005,
        P::default(),
        1e-7,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(r.model().faces[0].surface, s);
    assert_eq!(r.model().faces.len(), 6);
    assert!(r.constructed().mapped_normal_offset_error_upper() <= 0.005);
    assert_eq!(r.model().validate().unwrap().boundary_edge_count, 0);
    assert!(r.reverify().unwrap().audit.ok);
}
#[test]
fn degree_and_each_shared_work_budget_refuse_without_publishing_a_body() {
    let source = plane(0.);
    let map = uv([[0.1, 0.16, 0.1], [0.8, 0.86, 0.8]]);
    let saved = (source.clone(), map.clone());
    for budget in [
        P {
            max_degree: 1,
            ..P::default()
        },
        P {
            evaluations: 1,
            ..P::default()
        },
        P {
            coefficient_work: 1,
            ..P::default()
        },
        P {
            proof_cells: 1,
            ..P::default()
        },
    ] {
        assert!(
            thicken_from_maps(
                MappedSources {
                    sources: &[source.clone()],
                    maps: &[map.clone()],
                    reversed: &[false],
                },
                0.1,
                1e-12,
                budget,
                1e-7,
                Limits::default(),
            )
            .is_err()
        );
    }
    assert_eq!((source, map), saved);
}
