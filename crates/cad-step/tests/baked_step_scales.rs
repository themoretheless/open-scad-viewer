use cad_step::{StepAssemblyOccurrence, StepSharedAssembly, import_step_shared_assembly};

#[test]
fn scaled_and_mirrored_variants_keep_world_geometry_and_share_translated_copies() {
    let source = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let placed = |scale, x| StepAssemblyOccurrence {
        definition: 0,
        transform: [
            [0., -3., 0., x],
            [2., 0., 0., 0.],
            [0., 0., scale, 0.],
            [0., 0., 0., 1.],
        ],
    };
    let assembly = StepSharedAssembly::from_placed_solids(
        vec![source],
        vec![placed(4., 10.), placed(-4., 20.), placed(4., 40.)],
    )
    .unwrap();
    let step = assembly.export_step_with_baked_scales().unwrap();
    assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 2);
    assert_eq!(step.matches("ITEM_DEFINED_TRANSFORMATION(").count(), 3);
    assert!(!step.contains("CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM("));
    if let Some(path) = std::env::var_os("CAD_BAKED_SCALE_FIXTURE") {
        std::fs::write(path, &step).unwrap();
    }
    let restored = import_step_shared_assembly(&step).unwrap();
    assert_eq!(restored.occurrences().len(), 3);
    for i in 0..3 {
        let expected = assembly.materialize(i).unwrap();
        let actual = restored.materialize(i).unwrap();
        assert_eq!(actual.validate().unwrap().boundary_edge_count, 0);
        for vertex in &expected.vertices {
            assert!(
                actual
                    .vertices
                    .iter()
                    .any(|candidate| candidate.point == vertex.point)
            );
        }
    }
}

#[test]
fn reflected_elliptic_cylinder_retains_rational_surface_weights() {
    let cylinder = brep_core::cylinder(2., 3.).unwrap();
    let transform = [
        [2., 0., 0., 7.],
        [0., 3., 0., 11.],
        [0., 0., -4., 13.],
        [0., 0., 0., 1.],
    ];
    let assembly = StepSharedAssembly::from_placed_solids(
        vec![cylinder],
        vec![StepAssemblyOccurrence {
            definition: 0,
            transform,
        }],
    )
    .unwrap();
    let step = assembly.export_step_with_baked_scales().unwrap();
    if let Some(path) = std::env::var_os("CAD_ELLIPTIC_SCALE_FIXTURE") {
        std::fs::write(path, &step).unwrap();
    }
    let restored = import_step_shared_assembly(&step).unwrap();
    let body = restored.materialize(0).unwrap();
    assert!(
        body.faces
            .iter()
            .any(|face| face.surface.weights.iter().flatten().any(|w| *w != 1.))
    );
    assert_eq!(body.validate().unwrap().boundary_edge_count, 0);
}
