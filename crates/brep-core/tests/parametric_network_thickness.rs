use brep_core::shell_thickness::parametric_network::{Limits, OffsetSettings, thicken_with_maps};
use nurbs_core::surface::Surface;

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
        weights: vec![vec![1., 1.05, 1.]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn cap(x: f64, m: &Surface) -> Surface {
    let mut c = m.clone();
    for p in c.control_points.iter_mut().flatten() {
        p[0] += x;
        p[2] = 0.25;
    }
    c
}
#[test]
fn curved_original_uv_map_closes_a_literal_source_with_independent_wall_bases() {
    let s = plane(0.);
    let mut m = uv([[0.1, 0.16, 0.1], [0.8, 0.86, 0.8]]);
    m.knots_u = vec![-4., -4., 2., 2.];
    m.knots_v = vec![10., 10., 10., 14., 14., 14.];
    let c = cap(0., &m);
    let before = (s.clone(), c.clone(), m.clone());
    let result = thicken_with_maps(
        &[s.clone()],
        &[c.clone()],
        &[m.clone()],
        &[false],
        OffsetSettings {
            signed_thickness: 0.25,
            normal_tolerance: 0.005,
            normal_cells: 100000,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(result.model().faces.len(), 6);
    assert_eq!(result.exposed_boundaries().len(), 4);
    assert_eq!(result.model().faces[0].surface, s);
    assert_eq!(result.source_maps(), &[m.clone()]);
    assert!(result.mapped_normal_offset_error_upper() <= 0.005);
    assert!(result.evidence().charts_injective && result.evidence().boundary_complete);
    assert_eq!(result.model().validate().unwrap().boundary_edge_count, 0);
    assert!(result.reverify().unwrap().audit.ok);
    assert_eq!((s, c, m), before);
}
fn adjacent() -> ([Surface; 2], [Surface; 2], [Surface; 2]) {
    let maps = [
        uv([[0.1, 0.16, 0.1], [1.; 3]]),
        uv([[0.; 3], [0.9, 0.84, 0.9]]),
    ];
    let caps = [cap(0., &maps[0]), cap(1., &maps[1])];
    ([plane(0.), plane(1.)], caps, maps)
}
#[test]
fn curved_caps_share_the_actual_seam_without_an_internal_wall() {
    let (sources, caps, maps) = adjacent();
    let saved = sources.clone();
    let r = thicken_with_maps(
        &sources,
        &caps,
        &maps,
        &[false; 2],
        OffsetSettings {
            signed_thickness: 0.25,
            normal_tolerance: 0.005,
            normal_cells: 100000,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(r.exposed_boundaries().len(), 6);
    assert_eq!(r.model().faces.len(), 10);
    assert_eq!(r.model().validate().unwrap().boundary_edge_count, 0);
    assert!(r.reverify().unwrap().audit.ok);
    assert_eq!(sources, saved);
}
#[test]
fn normal_bounds_cannot_authorize_a_gap_fold_or_exhausted_construction() {
    let (sources, mut caps, maps) = adjacent();
    let saved = sources.clone();
    for p in caps[1].control_points.iter_mut().flatten() {
        p[0] += 1e-4;
    }
    // The offset comparison itself allows this displacement. Actual cap seam
    // sewing has its independent tighter tolerance and must refuse the body.
    assert_eq!(
        nurbs_core::surface_offset::certify_parametric_candidate(
            &sources[1],
            &caps[1],
            &maps[1],
            0.25,
            0.005,
            100000
        )
        .unwrap()
        .status,
        nurbs_core::surface_offset::CandidateStatus::Within
    );
    assert!(
        thicken_with_maps(
            &sources,
            &caps,
            &maps,
            &[false; 2],
            OffsetSettings {
                signed_thickness: 0.25,
                normal_tolerance: 0.005,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    let (_, caps, mut maps) = adjacent();
    maps[0].control_points.reverse();
    maps[0].weights.reverse();
    assert!(
        thicken_with_maps(
            &sources,
            &caps,
            &maps,
            &[false; 2],
            OffsetSettings {
                signed_thickness: 0.25,
                normal_tolerance: 0.005,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    let (_, caps, maps) = adjacent();
    assert!(
        thicken_with_maps(
            &sources,
            &caps,
            &maps,
            &[false; 2],
            OffsetSettings {
                signed_thickness: 0.25,
                normal_tolerance: 1e-12,
                normal_cells: 1,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    assert_eq!(sources, saved);
}
