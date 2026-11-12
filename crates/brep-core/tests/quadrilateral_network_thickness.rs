use brep_core::shell_thickness::quadrilateral_network::{Limits, OffsetSettings, SourceRegion, thicken_with_caps};
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
fn cap(x: f64, region: SourceRegion) -> Surface {
    let mut s = plane(x);
    s.control_points = region
        .map(|row| row.map(|uv| vec![x + uv[0], uv[1], 0.25]).to_vec())
        .to_vec();
    s
}
const REGION: SourceRegion = [[[0.1, 0.15], [0.15, 0.85]], [[0.9, 0.2], [0.85, 0.8]]];

#[test]
fn nonparallel_original_uv_region_closes_with_literal_source_and_replay() {
    let s = plane(0.);
    let c = cap(0., REGION);
    let before = (s.clone(), c.clone());
    let result = thicken_with_caps(
        &[s.clone()],
        &[c.clone()],
        &[REGION],
        &[false],
        OffsetSettings {
            signed_thickness: 0.25,
            normal_tolerance: 0.002,
            normal_cells: 100000,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(result.model().faces.len(), 6);
    assert_eq!(result.exposed_boundaries().len(), 4);
    assert_eq!(result.model().faces[0].surface, s);
    assert_eq!(result.source_regions(), &[REGION]);
    assert_eq!(result.cap_patches(), &[c.clone()]);
    assert!(result.mapped_normal_offset_error_upper() <= 0.002);
    assert!(result.evidence().charts_injective && result.evidence().boundary_complete);
    assert_eq!(result.model().validate().unwrap().boundary_edge_count, 0);
    assert!(result.reverify().unwrap().audit.ok);
    assert_eq!((s, c), before);
}
#[test]
fn adjacent_quadrilateral_caps_share_a_seam_without_an_internal_wall() {
    let regions = [
        [[[0.1, 0.15], [0.15, 0.85]], [[1., 0.1], [1., 0.9]]],
        [[[0., 0.1], [0., 0.9]], [[0.9, 0.2], [0.85, 0.8]]],
    ];
    let sources = [plane(0.), plane(1.)];
    let caps = [cap(0., regions[0]), cap(1., regions[1])];
    let saved = sources.clone();
    let result = thicken_with_caps(
        &sources,
        &caps,
        &regions,
        &[false; 2],
        OffsetSettings {
            signed_thickness: 0.25,
            normal_tolerance: 0.002,
            normal_cells: 100000,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(result.exposed_boundaries().len(), 6);
    assert_eq!(result.model().faces.len(), 10);
    assert_eq!(result.model().validate().unwrap().boundary_edge_count, 0);
    assert!(result.reverify().unwrap().audit.ok);
    assert_eq!(sources, saved);
}
#[test]
fn wrong_maps_changed_caps_and_incomplete_normal_budget_publish_no_skin() {
    let s = plane(0.);
    let c = cap(0., REGION);
    let saved = (s.clone(), c.clone());
    let mut folded = REGION;
    folded[1].swap(0, 1);
    assert!(
        thicken_with_caps(
            &[s.clone()],
            &[c.clone()],
            &[folded],
            &[false],
            OffsetSettings {
                signed_thickness: 0.25,
                normal_tolerance: 0.002,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    assert!(
        thicken_with_caps(
            &[s.clone()],
            &[c.clone()],
            &[REGION],
            &[false],
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
    let mut broken = c.clone();
    broken.control_points[1][1][2] += 0.1;
    assert!(
        thicken_with_caps(
            &[s.clone()],
            &[broken],
            &[REGION],
            &[false],
            OffsetSettings {
                signed_thickness: 0.25,
                normal_tolerance: 0.002,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    assert_eq!((s, c), saved);
}
