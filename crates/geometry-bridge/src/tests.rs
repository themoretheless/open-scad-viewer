use super::*;
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 10., 0.]],
            vec![vec![10., 0., 0.], vec![10., 10., 0.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn nurbs_to_polygons_and_back_to_exact_boundary_curves() {
    let source = plane();
    let sampled = tessellate_nurbs(
        &source,
        &Options {
            segments_u: 4,
            segments_v: 3,
            trim: None,
            max_triangles: None,
        },
    )
    .unwrap();
    let loops = sampled.mesh.boundary_loops().unwrap();
    let curves = boundary_curves(&sampled.mesh).unwrap();
    assert_eq!(curves.len(), 1);
    assert_eq!(curves[0].degree, 1);
    assert!(!curves[0].periodic);
    for (i, index) in loops[0].iter().enumerate() {
        assert_eq!(
            curves[0].evaluate(i as f64).unwrap().point,
            sampled.mesh.point(*index).unwrap().to_vec()
        );
    }
    let solid = sampled.mesh.thicken([0., 0., 2.]).unwrap();
    assert!(solid.report.closed);
    assert!((solid.report.signed_volume_mm3 - 200.).abs() < 1e-9);
    assert!(solid.mesh.uv.is_none());
    assert_eq!(source.control_points[0][0], vec![0., 0., 0.]);
}
#[test]
fn mesh_boundary_can_construct_a_new_nurbs_surface() {
    let source = plane();
    let mesh = tessellate_nurbs(
        &source,
        &Options {
            segments_u: 2,
            segments_v: 2,
            trim: None,
            max_triangles: None,
        },
    )
    .unwrap();
    let curves = boundary_curves(&mesh.mesh).unwrap();
    let walls = nurbs_core::surface::extrude(&curves[0], [0., 0., 3.]).unwrap();
    let result = tessellate_nurbs(
        &walls,
        &Options {
            segments_u: 8,
            segments_v: 2,
            trim: None,
            max_triangles: None,
        },
    )
    .unwrap();
    assert_eq!(result.report.non_manifold_edges, 0);
    assert!(result.report.parameter_seams_welded.unwrap().u);
}
#[test]
fn invalid_surface_is_rejected_before_polygons_are_created() {
    let mut source = plane();
    source.weights[0][0] = 0.;
    assert!(tessellate_nurbs(
        &source,
        &Options {
            segments_u: 2,
            segments_v: 2,
            trim: None,
            max_triangles: None
        }
    )
    .is_err());
}

#[test]
fn brep_tessellation_preserves_faces_and_interchanges_both_kernels() {
    let model = nurbs_core::brep::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    for segments in [1, 2, 4] {
        let mesh = crate::brep::nurbs(&model, segments).unwrap();
        assert!(mesh.built.report.closed);
        assert!((mesh.built.report.signed_volume_mm3 - 24.).abs() < 1e-8);
        assert_eq!(
            mesh.face_ids
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            6
        );
        let polygon =
            polygon_core::solid::brep::from_mesh(&mesh.built.mesh, Some(&mesh.face_ids)).unwrap();
        assert_eq!(polygon.faces.len(), 6);
        assert_eq!(polygon.bodies.len(), 1);
        let back = crate::brep::polygons(&polygon).unwrap();
        assert!(back.built.report.closed);
        assert!((back.built.report.signed_volume_mm3 - 24.).abs() < 1e-8);
    }
}
