use super::*;
use nurbs_core::intersection::certified_curve_surface::{
    all_surface_iso_in_window, first_surface_iso_in_window,
};
#[test]
fn rational_source_iso_has_two_contacts_with_a_distinct_original_target() {
    for axis in 0..2 {
        let mut source = super::spatial_windows::source();
        for row in &mut source.weights {
            row[1] = 2.;
        }
        if axis == 1 {
            source.control_points = (0..2)
                .map(|j| source.control_points.iter().map(|r| r[j].clone()).collect())
                .collect();
            source.weights = (0..2)
                .map(|j| source.weights.iter().map(|r| r[j]).collect())
                .collect();
            std::mem::swap(&mut source.degree_u, &mut source.degree_v);
            std::mem::swap(&mut source.knots_u, &mut source.knots_v);
        }
        let target = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-2., -2., 2., 2.],
            knots_v: vec![10., 10., 14., 14.],
            control_points: vec![
                vec![vec![0., -1., 0.], vec![0., -1., 2.]],
                vec![vec![0., 1., 0.], vec![0., 1., 2.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let before = source.clone();
        let before_target = target.clone();
        let first = first_surface_iso_in_window(
            &source,
            axis,
            0.5,
            &target,
            [[0., 1.], [-2., 2.], [10., 14.]],
            10000,
        )
        .unwrap();
        assert_eq!(first.roots.len(), 1);
        assert!(!first.complete);
        let r = all_surface_iso_in_window(
            &source,
            axis,
            0.5,
            &target,
            [[0., 1.], [-2., 2.], [10., 14.]],
            10000,
        )
        .unwrap();
        assert_eq!(r.roots.len(), 2, "{r:?}");
        assert!(r.complete, "{r:?}");
        assert!(r.unresolved.is_empty());
        for t in [0.25, 0.75] {
            let root = r
                .roots
                .iter()
                .find(|x| x.parameter_box[0][0] <= t && t <= x.parameter_box[0][1])
                .unwrap();
            let z = 4. / 3. + t * t * t / 4.;
            for (i, v) in [t, 0., 10. + 2. * z].into_iter().enumerate() {
                assert!(root.parameter_box[i][0] <= v && v <= root.parameter_box[i][1]);
            }
            assert!(root.position_bounds[2][0] <= z && z <= root.position_bounds[2][1]);
        }
        assert_eq!(source.control_points, before.control_points);
        assert_eq!(source.weights, before.weights);
        assert_eq!(target.control_points, before_target.control_points);
        let mut separated = target.clone();
        for row in &mut separated.control_points {
            for p in row {
                p[0] = 10.;
            }
        }
        let absent = all_surface_iso_in_window(
            &source,
            axis,
            0.5,
            &separated,
            [[0., 1.], [-2., 2.], [10., 14.]],
            100,
        )
        .unwrap();
        assert!(absent.complete);
        assert!(absent.roots.is_empty());
        assert!(absent.unresolved.is_empty());
        let short = all_surface_iso_in_window(
            &source,
            axis,
            0.5,
            &target,
            [[0., 1.], [-2., 2.], [10., 14.]],
            1,
        )
        .unwrap();
        assert!(!short.complete);
        assert!(!short.unresolved.is_empty());
        assert_eq!(short.cells, 1);
    }
}
