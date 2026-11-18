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

#[test]
fn coloured_mirrored_cylinders_share_geometry_with_distinct_occurrence_styles() {
    use cad_step::{
        StepOccurrenceColour, export_styled_step_assembly, import_step_occurrence_colours,
    };
    let placements = [0., 20.].map(|x| StepAssemblyOccurrence {
        definition: 0,
        transform: [
            [2., 0., 0., x],
            [0., 3., 0., 0.],
            [0., 0., -4., 0.],
            [0., 0., 0., 1.],
        ],
    });
    let scene = StepSharedAssembly::from_placed_solids(
        vec![brep_core::cylinder(2., 3.).unwrap()],
        placements.to_vec(),
    )
    .unwrap();
    let rgb = [[1., 0., 0.], [0., 0., 1.]];
    let styles = rgb.map(|rgb| {
        Some(StepOccurrenceColour {
            name: "Finish",
            rgb,
        })
    });
    let text = export_styled_step_assembly(&scene, &[], &[None, None], &styles).unwrap();
    assert_eq!(text.matches("=MANIFOLD_SOLID_BREP(").count(), 1);
    assert_eq!(
        import_step_occurrence_colours(&text).unwrap(),
        rgb.map(Some)
    );
    let grouped = export_styled_step_assembly(
        &scene,
        &["Drive"],
        &[Some(0), None],
        &[
            None,
            Some(StepOccurrenceColour {
                name: "Blue",
                rgb: rgb[1],
            }),
        ],
    )
    .unwrap();
    let reopened = import_step_shared_assembly(&grouped).unwrap();
    let colours = import_step_occurrence_colours(&grouped).unwrap();
    for (place, colour) in reopened.occurrences().iter().zip(colours) {
        assert_eq!(colour, (place.transform[0][3] == 20.).then_some(rgb[1]));
    }
    if let Some(path) = std::env::var_os("CAD_STYLED_SCALE_FIXTURE") {
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn named_variants_retain_source_labels_and_quantities_after_export() {
    let place = |definition, s, x| StepAssemblyOccurrence {
        definition,
        transform: [
            [s, 0., 0., x],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    };
    let assembly = StepSharedAssembly::from_placed_solids(
        vec![
            brep_core::cuboid([0.; 3], [1.; 3]).unwrap(),
            brep_core::cylinder(2., 3.).unwrap(),
        ],
        vec![
            place(1, -2., 0.),
            place(0, 1., 20.),
            place(1, 1., 40.),
            place(1, -2., 60.),
        ],
    )
    .unwrap();
    let mut named = cad_step::NamedStepAssembly {
        assembly,
        name: "Drive".into(),
        definition_names: vec!["Pin".into(), "Roller".into()],
    };
    let step = named.export_step_with_baked_scales().unwrap();
    let back = cad_step::import_named_step_assembly(&step).unwrap();
    assert_eq!(back.name, "Drive");
    let mut rows: Vec<_> = back
        .parts_list()
        .unwrap()
        .into_iter()
        .map(|row| (row.name, row.quantity))
        .collect();
    rows.sort();
    assert_eq!(rows, [("Pin", 1), ("Roller", 1), ("Roller", 2)]);
    for o in back.assembly.occurrences() {
        assert_eq!(
            back.definition_names[o.definition],
            if o.transform[0][3] == 20. {
                "Pin"
            } else {
                "Roller"
            }
        );
    }
    if let Some(path) = std::env::var_os("CAD_NAMED_SCALE_FIXTURE") {
        std::fs::write(path, step).unwrap();
    }
    named.definition_names.pop();
    assert!(named.export_step_with_baked_scales().is_err());
}

#[test]
fn named_coloured_parts_share_a_scaled_definition_across_group_and_root() {
    let placements = [0., 20.].map(|x| StepAssemblyOccurrence {
        definition: 0,
        transform: [
            [2., 0., 0., x],
            [0., 3., 0., 0.],
            [0., 0., -4., 0.],
            [0., 0., 0., 1.],
        ],
    });
    let scene = cad_step::NamedStepAssembly {
        name: "Drive".into(),
        definition_names: vec!["Roller".into()],
        assembly: StepSharedAssembly::from_placed_solids(
            vec![brep_core::cylinder(2., 3.).unwrap()],
            placements.to_vec(),
        )
        .unwrap(),
    };
    let rgb = [[1., 0., 0.], [0., 0., 1.]];
    let styles = rgb.map(|rgb| {
        Some(cad_step::StepOccurrenceColour {
            name: "Finish",
            rgb,
        })
    });
    let text =
        cad_step::export_named_styled_step_assembly(&scene, &["Feed"], &[Some(0), None], &styles)
            .unwrap();
    if let Some(path) = std::env::var_os("CAD_NAMED_STYLED_FIXTURE") {
        std::fs::write(path, &text).unwrap();
    }
    let reopened = cad_step::import_step_shared_assembly(&text).unwrap();
    assert_eq!(reopened.definitions().len(), 1);
    assert_eq!(reopened.occurrences().len(), 2);
    for (part, colour) in reopened
        .occurrences()
        .iter()
        .zip(cad_step::import_step_occurrence_colours(&text).unwrap())
    {
        assert_eq!(colour, Some(rgb[usize::from(part.transform[0][3] == 20.)]));
    }
}
