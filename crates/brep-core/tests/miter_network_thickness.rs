use brep_core::shell_thickness::miter_network::{Limits, thicken};
use nurbs_core::surface::Surface;
fn patch(p: [[[f64; 3]; 2]; 2]) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: p.map(|r| r.map(|p| p.to_vec()).to_vec()).to_vec(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn floor() -> Surface {
    patch([[[0., 0., 0.], [0., 1., 0.]], [[1., 0., 0.], [1., 1., 0.]]])
}
fn side(slope: f64) -> Surface {
    patch([
        [[1., 0., 0.], [1., 1., 0.]],
        [[1. + slope, 0., 1.], [1. + slope, 1., 1.]],
    ])
}
#[test]
fn perpendicular_supports_intersect_and_close_for_both_thickness_signs() {
    for d in [-0.1, 0.1] {
        let sources = vec![floor(), side(0.)];
        let saved = sources.clone();
        let r = thicken(&sources, &[false; 2], d, 1e-7, 8., Limits::default()).unwrap();
        assert_eq!(r.model().faces.len(), 10);
        assert_eq!(r.exposed_boundaries().len(), 6);
        assert!(r.support_offset_error_upper() <= 1e-7);
        assert!(r.miter_displacement_upper() >= 2. * d.abs());
        let shared = &r.cap_patches()[0].control_points[1][0];
        assert_eq!(shared, &r.cap_patches()[1].control_points[0][0]);
        assert!((shared[0] - (1. - d)).abs() < 1e-12);
        assert!((shared[2] - d).abs() < 1e-12);
        assert!((r.evidence().numerical_volume_mm3 - (2. * d - d * d).abs()).abs() < 1e-6);
        assert_eq!(r.model().validate().unwrap().boundary_edge_count, 0);
        assert!(r.reverify().unwrap().audit.ok);
        assert_eq!(sources, saved);
    }
}
#[test]
fn oblique_support_distance_is_not_misreported_as_pointwise_normal_offset() {
    let sources = vec![floor(), side(1.)];
    let r = thicken(&sources, &[false; 2], 0.1, 1e-7, 8., Limits::default()).unwrap();
    let p = &r.cap_patches()[0].control_points[1][0];
    assert!((p[0] - (1. + 0.1 * (1. - 2_f64.sqrt()))).abs() < 1e-12);
    assert!((p[2] - 0.1).abs() < 1e-12);
    assert!(r.support_offset_error_upper() < 1e-7);
    assert!(r.evidence().charts_injective);
    assert!(r.reverify().unwrap().boundary_complete);
}
#[test]
fn three_incident_support_planes_have_one_shared_miter_vertex() {
    let wall = patch([[[0., 1., 0.], [0., 1., 1.]], [[1., 1., 0.], [1., 1., 1.]]]);
    let sources = vec![floor(), side(0.), wall];
    let r = thicken(&sources, &[false; 3], 0.1, 1e-7, 8., Limits::default()).unwrap();
    assert_eq!(r.model().faces.len(), 12);
    let p = &r.cap_patches()[0].control_points[1][1];
    assert_eq!(p, &r.cap_patches()[1].control_points[0][1]);
    assert_eq!(p, &r.cap_patches()[2].control_points[1][0]);
    for (actual, expected) in p.iter().zip([0.9, 0.9, 0.1]) {
        assert!((actual - expected).abs() < 1e-12);
    }
    assert!((r.evidence().numerical_volume_mm3 - 0.271).abs() < 1e-6);
    assert!(r.reverify().unwrap().audit.ok);
}
#[test]
fn large_miters_nonplanar_supports_and_exhausted_work_refuse() {
    let sources = vec![floor(), side(0.)];
    assert!(thicken(&sources, &[false; 2], 0.1, 1e-7, 1.5, Limits::default()).is_err());
    let mut nonplanar = sources.clone();
    nonplanar[0].control_points[1][1][2] = 1e-10;
    assert!(thicken(&nonplanar, &[false; 2], 0.1, 1e-7, 8., Limits::default()).is_err());
    let mut low = Limits::default();
    low.proof.max_cells = 1;
    assert!(thicken(&sources, &[false; 2], 0.1, 1e-7, 8., low).is_err());
    let mut low = Limits::default();
    low.exact_work = 1;
    assert!(thicken(&sources, &[false; 2], 0.1, 1e-7, 8., low).is_err());
}

#[test]
fn rational_bases_and_reversed_parameter_orientation_survive_mitering() {
    let mut a = floor();
    a.weights = vec![vec![1., 2.], vec![1., 2.]];
    let mut b = side(0.);
    b.weights = vec![vec![1., 2.], vec![1., 2.]];
    b.control_points.reverse();
    b.weights.reverse();
    let sources = vec![a, b];
    let r = thicken(&sources, &[false, true], 0.1, 1e-7, 8., Limits::default()).unwrap();
    assert_eq!(r.model().faces.len(), 10);
    for (i, s) in sources.iter().enumerate() {
        assert_eq!(r.model().faces[i].surface, *s);
        assert_eq!(r.cap_patches()[i].weights, s.weights);
        assert_eq!(r.cap_patches()[i].knots_u, s.knots_u);
        assert_eq!(r.cap_patches()[i].knots_v, s.knots_v);
    }
    assert!(r.reverify().unwrap().audit.ok);
    #[cfg(feature = "codec")]
    {
        use value_codec::{Deserialize, Serialize};
        assert_eq!(
            brep_core::Model::from_value(r.model().to_value()).unwrap(),
            *r.model()
        );
    }
}

#[test]
fn nearly_opposed_normals_and_consumed_caps_are_rejected() {
    assert!(
        thicken(
            &[floor(), side(-1000.)],
            &[false; 2],
            0.1,
            1e-7,
            8.,
            Limits::default()
        )
        .is_err()
    );
    // The proposed cap consumes the unit-width original floor/side chart;
    // full saved-chart rank refuses it, even though support distances fit.
    assert!(
        thicken(
            &[floor(), side(0.)],
            &[false; 2],
            1.,
            1e-7,
            8.,
            Limits::default()
        )
        .is_err()
    );
    assert!(
        thicken(
            &[floor(), side(0.)],
            &[false; 2],
            1.1,
            1e-7,
            8.,
            Limits::default()
        )
        .is_err()
    );
    let mut doubled = vec![floor(), side(0.), side(0.)];
    assert!(thicken(&doubled, &[false; 3], 0.1, 1e-7, 8., Limits::default()).is_err());
    doubled[2].control_points[1][0][0] += 0.01;
    assert!(thicken(&doubled, &[false; 3], 0.1, 1e-7, 8., Limits::default()).is_err());
}
