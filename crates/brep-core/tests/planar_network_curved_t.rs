use brep_core::shell_thickness::planar_network::{self, CutKind, Limits};
use nurbs_core::surface::Surface;
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize};
fn source(fraction: f64, weight: f64) -> Vec<Surface> {
    let surface = |left: bool| Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![2., 2., 5., 5.],
        knots_v: vec![-4., -4., -4., 8., 8., 8.],
        control_points: (0..2)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let seam = [1., 1.5, 1.][j];
                        let x = if left {
                            if i == 0 { 0. } else { seam }
                        } else if i == 0 {
                            seam
                        } else {
                            2.
                        };
                        vec![x, j as f64 / 2., 0.]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., weight, 1.]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let left = surface(true);
    let mut right = surface(false);
    right.knots_u = vec![-1., -1., 3., 3.];
    right.knots_v = vec![3., 3., 3., 11., 11., 11.];
    let t = 3. + 8. * fraction;
    vec![
        left,
        right.trim([-1., 3., 3., t]).unwrap(),
        right.trim([-1., 3., t, 11.]).unwrap(),
    ]
}
#[test]
fn original_curved_t_witnesses_partition_actual_uses_and_keep_all_cap_carriers() {
    let s = source(0.5, 1.);
    let saved = s.clone();
    for thickness in [0.25, -0.25] {
        let r =
            planar_network::thicken(&s, &[false; 3], thickness, 1e-7, Limits::default()).unwrap();
        assert_eq!(r.cuts().len(), 1);
        let c = &r.cuts()[0];
        assert_eq!(c.kind, CutKind::CurvedWitness);
        assert!(c.source_distance_upper <= 1e-7);
        assert!(c.discovery_cells > 0);
        assert!((c.stored_parameter - 2.).abs() < 1e-6);
        assert_eq!(r.model().loops[r.model().faces[0].outer].coedges.len(), 5);
        assert_eq!(r.model().faces.len(), 13);
        for i in 0..3 {
            assert_eq!(r.model().faces[i].surface, s[i]);
        }
        assert!((r.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-6);
        assert!(r.reverify().unwrap().trim_regions_valid);
        #[cfg(feature = "codec")]
        {
            assert_eq!(
                brep_core::Model::from_value(r.model().to_value()).unwrap(),
                *r.model()
            );
        }
    }
    assert_eq!(s, saved);
}
#[test]
fn quarter_curve_witnesses_keep_nonunit_clocks_and_transposed_neighbor_orientation() {
    let mut s = source(0.25, 1.);
    s[2] = nurbs_core::surface_edit::transpose(&s[2]).unwrap();
    let saved = s.clone();
    let r =
        planar_network::thicken(&s, &[false, false, true], 0.25, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.cuts().len(), 1);
    assert!((r.cuts()[0].stored_parameter + 1.).abs() < 1e-6);
    assert_eq!(r.cuts()[0].kind, CutKind::CurvedWitness);
    for i in 0..3 {
        assert_eq!(r.model().faces[i].surface, s[i]);
    }
    assert!((r.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-6);
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn rational_curved_witnesses_are_certified_points_not_claimed_exact_roots() {
    let s = source(0.5, 0.75);
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.cuts().len(), 1);
    let c = &r.cuts()[0];
    assert_eq!(c.kind, CutKind::CurvedWitness);
    assert!(c.source_distance_upper > 0. && c.source_distance_upper < 1e-7);
    assert!(c.discovery_cells > 0);
    for i in 0..3 {
        assert_eq!(r.model().faces[i].surface, s[i]);
    }
    assert!((r.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-5);
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert_eq!(s, saved);
}
#[test]
fn a_point_inside_the_control_hull_does_not_create_a_false_curved_t_join() {
    let mut s = source(0.5, 1.);
    for surface in &mut s[1..] {
        for p in surface.control_points.iter_mut().flatten() {
            p[0] += 0.01;
        }
    }
    // The middle witness (1.26,.5,0) is in the main curve's control box,
    // but is strictly separated from its entire actual original image.
    let saved = s.clone();
    let e = planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, Limits::default())
        .err()
        .unwrap();
    assert!(e.to_string().contains("connected"), "{e}");
    assert_eq!(s, saved);
}
#[test]
fn curved_search_work_comes_from_the_same_operation_allowance() {
    let s = source(0.5, 1.);
    let saved = s.clone();
    let limits = Limits {
        proof: brep_core::feature_ops::region_pattern::Limits {
            max_cells: 40,
            ..Limits::default().proof
        },
        ..Limits::default()
    };
    let e = planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, limits)
        .err()
        .unwrap();
    assert!(
        e.to_string().contains("allowance") || e.to_string().contains("exhausted"),
        "{e}"
    );
    assert_eq!(s, saved);
}
#[test]
fn several_curved_t_witnesses_remain_ordered_and_share_original_vertices() {
    let left = source(0.5, 1.)[0].clone();
    let rows = [
        left.control_points[1].clone(),
        left.control_points[1]
            .iter()
            .map(|p| vec![2., p[1], 0.])
            .collect(),
    ];
    let parts = rows
        .iter()
        .map(|points| {
            let c = nurbs_core::curve::Curve {
                degree: 2,
                knots: vec![3., 3., 3., 11., 11., 11.],
                control_points: points.clone(),
                weights: vec![1.; 3],
                periodic: false,
            };
            let p = nurbs_core::curve_partition_agreement::partition(&c, &[5., 9.], 1e-10, 10000)
                .unwrap();
            assert!(p.geometry_preserved && p.endpoint_joins_exact);
            p.pieces.into_iter().map(|p| p.curve).collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut s = vec![left];
    for i in 0..3 {
        s.push(Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![-1., -1., 3., 3.],
            knots_v: parts[0][i].knots.clone(),
            control_points: vec![
                parts[0][i].control_points.clone(),
                parts[1][i].control_points.clone(),
            ],
            weights: vec![parts[0][i].weights.clone(), parts[1][i].weights.clone()],
            periodic_u: false,
            periodic_v: false,
        });
    }
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false; 4], 0.25, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.cuts().len(), 2);
    assert!(
        r.cuts()
            .windows(2)
            .all(|w| w[0].stored_parameter < w[1].stored_parameter)
    );
    assert!(r.cuts().iter().all(|c| c.kind == CutKind::CurvedWitness));
    assert_eq!(r.model().loops[r.model().faces[0].outer].coedges.len(), 6);
    for i in 0..4 {
        assert_eq!(r.model().faces[i].surface, s[i]);
    }
    assert!((r.evidence().numerical_volume_mm3 - 0.5).abs() < 1e-6);
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn oblique_curved_t_witnesses_keep_rounding_inclusive_cap_separation() {
    let mut s = source(0.5, 1.);
    for surface in &mut s {
        for p in surface.control_points.iter_mut().flatten() {
            let [x, y, z] = [p[0], p[1], p[2]];
            p.copy_from_slice(&[x + z + 5., y + z - 3., x + y - z + 2.]);
        }
    }
    let saved = s.clone();
    let r = planar_network::thicken(&s, &[false; 3], 0.25, 1e-7, Limits::default()).unwrap();
    assert_eq!(r.cuts().len(), 1);
    assert_eq!(r.cuts()[0].kind, CutKind::CurvedWitness);
    assert!(r.cap_distance_bounds()[0] > 0.);
    assert!((r.evidence().numerical_volume_mm3 - 0.5 * 3f64.sqrt()).abs() < 1e-6);
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert_eq!(s, saved);
}
