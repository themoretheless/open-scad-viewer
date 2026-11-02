use brep_core::shell_thickness::planar_network::{self, Limits};
use nurbs_core::surface::Surface;
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize};
fn patch(x: [f64; 2], y: [f64; 2]) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![x[0], y[0], 0.], vec![x[0], y[1], 0.]],
            vec![vec![x[1], y[0], 0.], vec![x[1], y[1], 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn source() -> Vec<Surface> {
    vec![
        patch([0., 1.], [0., 2.]),
        patch([1., 2.], [0., 1.]),
        patch([1., 2.], [1., 2.]),
    ]
}
#[test]
fn a_source_t_junction_splits_actual_coedges_without_splitting_original_cap_carriers() {
    let s = source();
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.cuts().len(), 1);
    assert_eq!(r.cuts()[0].patch, 0);
    assert_eq!(r.cuts()[0].side, 1);
    assert_eq!(r.cuts()[0].stored_parameter, 0.5);
    assert_eq!(r.exposed_boundaries().len(), 7);
    assert_eq!(r.model().faces.len(), 13);
    assert_eq!(r.model().loops[r.model().faces[0].outer].coedges.len(), 5);
    for i in 0..3 {
        assert_eq!(r.model().faces[i].surface, s[i]);
    }
    assert!((r.evidence().numerical_volume_mm3 - 1.).abs() < 1e-6);
    assert!(r.reverify().unwrap().trim_regions_valid);
    #[cfg(feature = "codec")]
    {
        assert_eq!(
            brep_core::Model::from_value(r.model().to_value()).unwrap(),
            *r.model()
        );
    }
    assert_eq!(s, saved);
}
#[test]
fn multiple_partial_neighbors_preserve_independent_clocks_and_negative_thickness() {
    let mut s = vec![patch([0., 1.], [0., 3.])];
    for y in 0..3 {
        s.push(patch([1., 2.], [y as f64, y as f64 + 1.]));
    }
    s[0].knots_v = vec![-2., -2., 7., 7.];
    s[1].knots_v = vec![2., 2., 5., 5.];
    s[2] = nurbs_core::surface_edit::transpose(&s[2]).unwrap();
    let saved = s.clone();
    let r = planar_network::thicken(
        &s,
        &[false, false, true, false],
        -0.25,
        1e-7,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(r.cuts().iter().filter(|c| c.patch == 0).count(), 2);
    assert_eq!(r.model().loops[r.model().faces[0].outer].coedges.len(), 6);
    for i in 0..4 {
        assert_eq!(r.model().faces[i].surface, s[i]);
    }
    assert!((r.evidence().numerical_volume_mm3 - 1.5).abs() < 1e-6);
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn rational_boundary_clocks_get_exact_original_t_roots_and_saved_uv_clock_proposals() {
    use num_rational::BigRational as Q;
    let mut s = source();
    s[0].weights = vec![vec![1., 2.]; 2];
    s[1].weights = vec![vec![1., 1.25]; 2];
    s[2].weights = vec![vec![1.5, 1.]; 2];
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.cuts()[0].parameter, Q::new(1.into(), 3.into()));
    assert_ne!(
        r.cuts()[0].parameter,
        Q::from_float(r.cuts()[0].stored_parameter).unwrap()
    );
    assert!(
        r.model().loops.iter().flat_map(|l| &l.coedges).any(|c| c
            .pcurve
            .weights
            .windows(2)
            .any(|w| w[0] != w[1]))
    );
    assert!((r.evidence().numerical_volume_mm3 - 1.).abs() < 1e-5);
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert_eq!(s, saved);
}
#[test]
fn an_oblique_t_network_retains_all_original_carriers_under_rounding_inclusive_thickness() {
    let mut s = source();
    for surface in &mut s {
        for p in surface.control_points.iter_mut().flatten() {
            let [x, y, z] = [p[0], p[1], p[2]];
            p.copy_from_slice(&[x + z + 5., y + z - 3., x + y - z + 2.]);
        }
    }
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, Limits::default()).unwrap();
    for i in 0..3 {
        assert_eq!(r.model().faces[i].surface, s[i]);
    }
    assert!(r.cap_distance_bounds()[0] > 0.);
    assert!((r.evidence().numerical_volume_mm3 - 3f64.sqrt()).abs() < 1e-5);
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn shared_t_corners_do_not_authorize_nonmanifold_or_disconnected_source_overlap() {
    let s = source();
    let saved = s.clone();
    let mut duplicate = s.clone();
    duplicate.push(s[1].clone());
    assert!(
        planar_network::thicken(&duplicate, &[false; 4], 0.25, 1e-7, Limits::default()).is_err()
    );
    let mut gap = s.clone();
    for p in gap[1].control_points.iter_mut().flatten() {
        p[0] += 3.;
    }
    assert!(planar_network::thicken(&gap, &[false; 3], 0.25, 1e-7, Limits::default()).is_err());
    assert!(
        planar_network::thicken(
            &s,
            &[false; 3],
            0.25,
            1e-7,
            Limits {
                exact_work: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        planar_network::thicken(
            &s,
            &[false; 3],
            0.25,
            1e-7,
            Limits {
                proof: brep_core::feature_ops::region_pattern::Limits {
                    max_cells: 25,
                    ..Limits::default().proof
                },
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert_eq!(s, saved);
}

#[test]
fn a_connected_slit_stays_open_instead_of_being_healed_by_t_vertex_matching() {
    let mut s = source();
    for p in s[1].control_points.iter_mut().flatten() {
        p[0] += 1e-3;
    }
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, Limits::default()).unwrap();
    assert!(r.model().vertices.iter().any(|v| v.point == [1., 1., 0.]));
    assert!(
        r.model()
            .vertices
            .iter()
            .any(|v| v.point == [1.001, 1., 0.])
    );
    for i in 0..3 {
        assert_eq!(r.model().faces[i].surface, s[i]);
    }
    assert!((r.evidence().numerical_volume_mm3 - 1.).abs() < 1e-6);
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert_eq!(s, saved);
}
#[test]
fn partial_uv_coedges_retain_the_exact_original_natural_rectangle_image() {
    let mut s = source();
    s[0].knots_u = vec![1e8, 1e8, 1e8 + 4., 1e8 + 4.];
    s[0].knots_v = vec![-2., -2., 7., 7.];
    s[0].weights = vec![vec![1., 2.]; 2];
    let r = planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, Limits::default()).unwrap();
    for i in 0..3 {
        let surface = &s[i];
        let natural = [
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        ];
        let coedges = &r.model().loops[r.model().faces[i].outer].coedges;
        for (slot, c) in coedges.iter().enumerate() {
            assert!((0..2).any(|axis| {
                natural[axis]
                    .iter()
                    .any(|end| c.pcurve.control_points.iter().all(|p| p[axis] == *end))
            }));
            assert_eq!(
                c.pcurve.control_points.last().unwrap(),
                &coedges[(slot + 1) % coedges.len()].pcurve.control_points[0]
            );
        }
    }
    assert!(r.reverify().unwrap().boundary_complete);
}
#[test]
fn coincident_t_endpoints_cannot_replace_a_different_curved_boundary_by_a_straight_seam() {
    let mut s = source();
    s[1] = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1.25, 0.5, 0.], vec![1., 1., 0.]],
            vec![vec![2., 0., 0.], vec![2., 0.5, 0.], vec![2., 1., 0.]],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let saved = s.clone();
    assert!(planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, Limits::default()).is_err());
    assert_eq!(s, saved);
}
