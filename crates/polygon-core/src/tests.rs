use super::*;
use crate::solid::tessellation;
fn square() -> Mesh {
    Mesh {
        positions: vec![0., 0., 0., 2., 0., 0., 2., 3., 0., 0., 3., 0.],
        indices: vec![0, 1, 2, 0, 2, 3],
        uv: None,
    }
}
#[test]
fn polygon_library_works_without_a_surface_definition() {
    let mesh = square();
    let report = mesh.inspect().unwrap();
    assert_eq!(report.boundary_edges, 4);
    assert!(!report.closed);
    assert_eq!(mesh.boundary_loops().unwrap(), vec![vec![0, 1, 2, 3, 0]]);
    let solid = mesh.thicken([0., 0., 4.]).unwrap();
    assert!(solid.report.closed);
    assert_eq!(solid.report.signed_volume_mm3, 24.);
    assert!(solid.mesh.boundary_loops().unwrap().is_empty());
    assert!(solid.mesh.export_stl().unwrap().contains("facet normal"));
    assert!(solid.mesh.uv.is_none());
}
#[test]
fn affine_reflection_preserves_outward_orientation() {
    let solid = square().thicken([0., 0., 4.]).unwrap().mesh;
    let transformed = solid
        .transform([
            [-2., 0., 0., 100.],
            [0., 1., 0., 200.],
            [0., 0., 1., 300.],
            [0., 0., 0., 1.],
        ])
        .unwrap();
    let report = transformed.inspect().unwrap();
    assert!(report.closed);
    assert_eq!(report.signed_volume_mm3, 48.);
    assert_eq!(solid.point(0).unwrap(), [0., 0., 0.]);
}
#[test]
fn rejects_bad_indices_bad_uv_and_ambiguous_boundaries() {
    let mut mesh = square();
    mesh.indices.push(99);
    assert!(mesh.inspect().is_err());
    mesh = square();
    mesh.uv = Some(vec![0.; 3]);
    assert!(mesh.inspect().is_err());
    mesh = square();
    mesh.indices.extend([0, 1, 3]);
    assert!(mesh.boundary_loops().is_err());
    assert!(square().thicken([1., 0., 0.]).is_err());
}

#[test]
fn inspection_preserves_topology_diagnostics_without_coordinate_welding() {
    let mut mesh = square();
    mesh.indices = vec![0, 1, 2, 0, 3, 2];
    let report = mesh.inspect().unwrap();
    assert_eq!(report.boundary_edges, 4);
    assert_eq!(report.orientation_conflicts, 1);
    assert!(!report.closed);
    mesh.indices.extend([0, 2, 1]);
    let report = mesh.inspect().unwrap();
    assert_eq!(report.non_manifold_edges, 1);
    assert!(mesh.boundary_loops().is_err());
    assert!(mesh.thicken([0., 0., 1.]).is_err());

    mesh = square();
    mesh.positions.extend([0., 0., 0.]);
    mesh.indices = vec![0, 1, 2, 4, 2, 3];
    assert_eq!(mesh.inspect().unwrap().boundary_edges, 6);
    assert!(mesh.boundary_loops().is_err());
    mesh.indices = vec![0, 0, 1];
    assert_eq!(mesh.inspect().unwrap().degenerate_triangles, 1);
    assert!(!mesh.inspect().unwrap().closed);
}

#[test]
fn boundary_order_and_thickened_indices_are_unchanged() {
    let mesh = square();
    let solid = mesh.thicken([0., 0., 4.]).unwrap();
    assert_eq!(
        solid.mesh.indices,
        vec![
            0, 2, 1, 4, 5, 6, 0, 3, 2, 4, 6, 7, 0, 1, 5, 0, 5, 4, 3, 0, 4, 3, 4, 7, 1, 2, 6, 1, 6,
            5, 2, 3, 7, 2, 7, 6,
        ]
    );
    let reversed = Mesh {
        indices: vec![2, 1, 0, 3, 2, 0],
        ..mesh.clone()
    };
    assert_eq!(
        reversed.boundary_loops().unwrap(),
        vec![vec![0, 3, 2, 1, 0]]
    );
    let mut reordered = mesh.clone();
    reordered.indices = vec![0, 2, 3, 0, 1, 2];
    assert_eq!(
        reordered.boundary_loops().unwrap(),
        mesh.boundary_loops().unwrap()
    );
    let malformed = Mesh {
        positions: vec![f64::NAN, 0., 0.],
        indices: vec![0, 0, 0],
        uv: None,
    };
    assert_eq!(
        malformed.inspect().unwrap_err().message,
        "Malformed triangle mesh."
    );
}
#[test]
fn mesher_accepts_an_unrelated_analytic_surface() {
    struct Plane;
    impl tessellation::ParametricSurface for Plane {
        fn domain(&self) -> [f64; 4] {
            [0., 1., 0., 1.]
        }
        fn point(&self, u: f64, v: f64) -> Result<[f64; 3]> {
            Ok([u * 2., v * 3., 0.])
        }
    }
    let options = tessellation::Options {
        segments_u: 3,
        segments_v: 3,
        trim: None,
        max_triangles: None,
    };
    let sampled = tessellation::tessellate(&Plane, &options).unwrap();
    assert_eq!(sampled.report.boundary_edges, 12);
    assert!(sampled.report.sampled_deviation_mm.unwrap() < 1e-12);
    assert!(
        (sampled
            .mesh
            .thicken([0., 0., 4.])
            .unwrap()
            .report
            .signed_volume_mm3
            - 24.)
            .abs()
            < 1e-10
    );
}

#[test]
fn imported_mesh_budget_is_separate_from_tessellation_budget() {
    let mesh = Mesh {
        positions: vec![0., 0., 0., 1., 0., 0., 0., 1., 0.],
        indices: [0, 1, 2].repeat(21_000),
        uv: None,
    };
    assert_eq!(mesh.inspect().unwrap().triangle_count, 21_000);
    let oversized = Mesh {
        indices: [0, 1, 2].repeat(100_001),
        ..mesh
    };
    assert!(oversized.inspect().is_err());
}
