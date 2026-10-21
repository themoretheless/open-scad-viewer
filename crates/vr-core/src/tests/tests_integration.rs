    use super::*;
    fn mesh(offset: f32) -> Mesh {
        Mesh {
            vertices: vec![
                0., 0., 0., 0., 0., 1., 100., 0., 0., 0., 0., 1., 0., 0., 100., 0., 0., 1.,
            ],
            indices: vec![0, 1, 2],
            transform: [
                1., 0., 0., offset, 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ],
            color: [1.; 4],
        }
    }
    #[test]
    fn scene_spacing_and_axis() {
        let result = prepare_scene(vec![mesh(0.), mesh(100.)]).unwrap();
        assert!((result[0].positions[0] + 0.3).abs() < 1e-6);
        assert!((result[1].positions[3] - 0.3).abs() < 1e-6);
        assert!((result[0].positions[7] - 0.15).abs() < 1e-6);
    }
    #[test]
    fn rejects_invalid_inputs() {
        assert!(matches!(prepare_scene(vec![]), Err(Error::EmptyScene)));
        let mut m = mesh(0.);
        m.indices[0] = u32::MAX;
        assert!(matches!(
            prepare_scene(vec![m]),
            Err(Error::InvalidGeometry)
        ));
        let mut m = mesh(0.);
        m.vertices[0] = f32::NAN;
        assert!(prepare_scene(vec![m]).is_err());
        for input in [&[][..], &[255, 255, 255, 255][..], &[1, 0, 0, 0][..]] {
            assert!(prepare_bytes(input).is_err());
        }
    }
    #[test]
    fn rotated_anchor() {
        let p = [
            0., 0., -1., 0., 0., 1., 0., 0., 1., 0., 0., 0., 3., 2., 1., 1.,
        ];
        let a = anchor(p);
        assert!((a[12] - 1.8).abs() < 1e-6);
        assert_eq!(a[13], 2.);
        assert_eq!(a[14], 1.);
    }
    #[test]
    fn true_scale_normals_and_raycast() {
        let with_normals =
            prepare_scene_with_normals(vec![mesh(0.)], ScaleMode::TrueScaleMmToMeters).unwrap();
        assert_eq!(with_normals[0].normals.len(), 9);
        assert!((with_normals[0].normals[1] - 1.0).abs() < 1e-6);
        // 100 mm span in TrueScaleMmToMeters is 0.1 m (-0.05 to +0.05)
        assert!((with_normals[0].positions[0] + 0.05).abs() < 1e-6);
        assert!((with_normals[0].positions[3] - 0.05).abs() < 1e-6);

        let prepared = prepare_scene_with_scale(vec![mesh(0.)], ScaleMode::TrueScaleMmToMeters).unwrap();
        let hit = raycast_scene(&prepared, [-0.02, -0.02, 1.0], [0.0, 0.0, -1.0]).unwrap();
        assert_eq!(hit.mesh_index, 0);
        assert_eq!(hit.triangle_index, 0);
        assert!((hit.distance_m - 1.0).abs() < 1e-5);
    }

    #[test]
    fn vr_measurements_section_clip_and_exploded_view() {
        let m = measure_vr_distance([0.0, 0.0, 0.0], [0.03, 0.04, 0.0], 0.001).unwrap();
        assert!((m.distance_m - 0.05).abs() < 1e-6);
        assert!((m.cad_distance_mm - 50.0).abs() < 1e-4);

        let deg = measure_angle_deg([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]).unwrap();
        assert!((deg - 90.0).abs() < 1e-5);

        let scene = prepare_scene(vec![mesh(0.), mesh(200.)]).unwrap();
        let clipped = clip_scene_by_plane(&scene, [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(clipped.len(), 1);

        let exploded = vr_exploded_view(&scene, 0.5);
        assert!(exploded[1].positions[0] > scene[1].positions[0]);
        assert!(exploded[0].positions[0] < scene[0].positions[0]);

        let pose = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 1.6, 0., 1.,
        ];
        let grab = compute_ergonomic_grab_anchor(pose, 0.55, 0.25);
        assert!((grab[1] - 1.35).abs() < 1e-5);
        assert!((grab[2] + 0.55).abs() < 1e-5);

        let gesture = TwoHandTransform::solve(
            [-0.1, 0.0, 0.0],
            [0.1, 0.0, 0.0],
            [0.0, 0.2, -0.2],
            [0.0, 0.2, 0.2],
        )
        .unwrap();
        assert!((gesture.scale - 2.0).abs() < 1e-5);
        assert!((gesture.translation_m[1] - 0.2).abs() < 1e-5);
        assert!((gesture.rotation_angle_rad - std::f32::consts::FRAC_PI_2).abs() < 1e-5);

        let hit_pos = [
            scene[0].positions[0],
            scene[0].positions[1],
            scene[0].positions[2],
        ];
        assert!(check_hand_collision(hit_pos, 0.05, &scene));
        assert!(!check_hand_collision([5.0, 5.0, 5.0], 0.05, &scene));
    }
