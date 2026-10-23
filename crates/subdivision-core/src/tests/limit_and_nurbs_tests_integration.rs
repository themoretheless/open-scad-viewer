    use super::*;

    #[test]
    fn evaluate_limit_points_matches_converged_catmull_clark_refinement() {
        let cage = Cage::extrude(
            &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
            [0., 0., 2.],
        )
        .unwrap();
        let exact_limit = cage.evaluate_limit_points().unwrap();
        let refined = cage.subdivide(5).unwrap();
        for (i, &p_inf) in exact_limit.iter().enumerate() {
            let p_sub = refined.cage.vertices[i];
            for k in 0..3 {
                assert!(
                    (p_inf[k] - p_sub[k]).abs() < 1e-3,
                    "vertex {i} axis {k}: exact={} vs refined={}",
                    p_inf[k],
                    p_sub[k]
                );
            }
        }
        let normals = cage.evaluate_limit_normals().unwrap();
        assert_eq!(normals.len(), 8);
        for (p, n) in exact_limit.iter().zip(&normals) {
            let radial = math_core::sub(*p, [1., 1., 1.]);
            assert!(math_core::dot(*n, radial) > 0.8);
            assert!((math_core::norm(*n) - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn to_nurbs_patches_extracts_bicubic_patch_matching_limit_points() {
        // 3x3 grid of quads (16 vertices, 9 quad faces); center face (index 4) is regular interior valence-4.
        let mut vertices = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                let z = if (x == 1 || x == 2) && (y == 1 || y == 2) {
                    1.0
                } else {
                    0.0
                };
                vertices.push([x as f64, y as f64, z]);
            }
        }
        let mut faces = Vec::new();
        for y in 0..3 {
            for x in 0..3 {
                let i0 = y * 4 + x;
                faces.push(vec![i0, i0 + 1, i0 + 5, i0 + 4]);
            }
        }
        let cage = Cage { vertices, faces };
        let patches = cage.to_nurbs_patches().unwrap();
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].face_index, 4);

        let limit_pts = cage.evaluate_limit_points().unwrap();
        // Corners of the center face (face 4 has vertices [5, 6, 10, 9]) must match patch(u, v) at {0,1}x{0,1}
        let corners = [
            (0.0, 0.0, 5),
            (1.0, 0.0, 6),
            (1.0, 1.0, 10),
            (0.0, 1.0, 9),
        ];
        for (u, v, vi) in corners {
            let eval = patches[0].evaluate(u, v);
            let lim = limit_pts[vi];
            for k in 0..3 {
                assert!(
                    (eval[k] - lim[k]).abs() < 1e-12,
                    "corner ({u},{v}) mismatch: {:?} vs {:?}",
                    eval,
                    lim
                );
            }
        }
    }

    #[test]
    fn adaptive_subdivide_and_quality_report_work() {
        let cube = CatmullClarkMesh::extrude(
            &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
            [0., 0., 2.],
        )
        .unwrap();
        let report = cube.mesh_quality_report().unwrap();
        assert_eq!(report.vertex_count, 8);
        assert_eq!(report.face_count, 6);
        assert_eq!(report.extraordinary_vertex_count, 8); // 8 corners of valence 3
        assert_eq!(report.valence_histogram, vec![(3, 8)]);
        assert!((report.min_aspect_ratio - 1.0).abs() < 1e-12);
        assert!((report.surface_area - 24.0).abs() < 1e-12);

        // Flat grid has 0 dihedral angle -> adaptive_subdivide stops at level 0
        let flat = CatmullClarkMesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
            faces: vec![vec![0, 1, 2, 3]],
        };
        let flat_ref = flat.adaptive_subdivide(0.1, 4).unwrap();
        assert_eq!(flat_ref.cage.faces.len(), 1);

        // Cube has 90 deg (1.57 rad) dihedral angles -> refines until angle <= tol
        let cube_ref = cube.adaptive_subdivide(0.45, 4).unwrap();
        assert!(cube_ref.cage.faces.len() >= 96);
    }
