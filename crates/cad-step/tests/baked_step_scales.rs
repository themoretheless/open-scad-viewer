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
    let props = brep_core::analysis::mass_properties(&cylinder, 1e-7, 200_000).unwrap();
    let measured = brep_core::analysis::AssemblyMass {
        mass: props.signed_volume_mm3,
        centroid: props.centroid,
        inertia: props.inertia_mm5,
    };
    for shear in [0., 0.5] {
        let mut map = transform;
        map[0][1] = shear;
        let placed = measured.affine_at_constant_density(map).unwrap();
        let body = brep_core::transform::affine(&cylinder, map).unwrap();
        let direct = brep_core::analysis::mass_properties(&body, 1e-7, 200_000).unwrap();
        assert!((placed.mass - direct.signed_volume_mm3).abs() < 1e-5);
        for i in 0..3 {
            assert!((placed.centroid[i] - direct.centroid[i]).abs() < 1e-8);
            for j in 0..3 {
                assert!((placed.inertia[i][j] - direct.inertia_mm5[i][j]).abs() < 1e-4);
            }
        }
    }
    assert!(measured.affine_at_constant_density([[0.; 4]; 4]).is_err());
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
    let (header, data) = text.split_once("DATA;").unwrap();
    let (rows, tail) = data.split_once("ENDSEC;").unwrap();
    let reordered = format!(
        "{header}DATA;\n{}\nENDSEC;{tail}",
        rows.lines().rev().collect::<Vec<_>>().join("\n")
    );
    let (named, paths) = cad_step::import_step_product_paths(&reordered).unwrap();
    assert_eq!(named.name, "Drive");
    assert_eq!(named.definition_names, ["Roller"]);
    assert_eq!(paths.len(), 2);
    for (part, path) in named.assembly.occurrences().iter().zip(paths) {
        let expected = if part.transform[0][3] == 0. {
            vec!["Drive", "Feed", "Roller"]
        } else {
            vec!["Drive", "Roller"]
        };
        assert_eq!(path, expected);
    }
    let subset = cad_step::export_step_product_subassembly(&reordered, &["Drive", "Feed"]).unwrap();
    let (feed, feed_paths) = cad_step::import_step_product_paths(&subset).unwrap();
    assert_eq!(feed.name, "Feed");
    assert_eq!(feed_paths, [vec!["Feed", "Roller"]]);
    assert_eq!(
        cad_step::import_step_occurrence_colours(&subset).unwrap(),
        [Some(rgb[0])]
    );
    assert_eq!(feed.assembly.occurrences()[0].transform[0][3], 0.);
    assert!(cad_step::export_step_product_subassembly(&text, &["Drive", "Missing"]).is_err());
    assert!(cad_step::export_step_product_subassembly(&text, &[]).is_err());
    if let Some(path) = std::env::var_os("CAD_SUBASSEMBLY_FIXTURE") {
        std::fs::write(path, &subset).unwrap();
    }
    let revised = cad_step::replace_step_product_at_path(
        &reordered,
        &["Drive", "Feed", "Roller"],
        brep_core::cuboid([0.; 3], [2., 4., 8.]).unwrap(),
        "Insert",
    )
    .unwrap();
    let (edited, paths) = cad_step::import_step_product_paths(&revised).unwrap();
    assert_eq!(edited.assembly.definitions().len(), 2);
    for ((occ, path), colour) in edited
        .assembly
        .occurrences()
        .iter()
        .zip(paths)
        .zip(cad_step::import_step_occurrence_colours(&revised).unwrap())
    {
        let direct = occ.transform[0][3] == 20.;
        assert_eq!(
            path,
            if direct {
                vec!["Drive", "Roller"]
            } else {
                vec!["Drive", "Feed", "Insert"]
            }
        );
        assert_eq!(colour, Some(rgb[usize::from(direct)]));
    }
    if let Some(path) = std::env::var_os("CAD_BRANCH_VARIANT_FIXTURE") {
        std::fs::write(path, &revised).unwrap();
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

#[test]
fn branch_motion_moves_two_linked_parts_and_preserves_external_sibling() {
    let placements = [2., 5., 20.].map(|x| StepAssemblyOccurrence {
        definition: 0,
        transform: [
            [1., 0., 0., x],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    });
    let scene = cad_step::NamedStepAssembly {
        name: "Drive".into(),
        definition_names: vec!["Bracket".into()],
        assembly: StepSharedAssembly::from_placed_solids(
            vec![brep_core::cuboid([0.; 3], [2., 4., 6.]).unwrap()],
            placements.to_vec(),
        )
        .unwrap(),
    };
    let colours = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
        .map(|rgb| Some(cad_step::StepOccurrenceColour { name: "", rgb }));
    let original = cad_step::export_named_styled_step_assembly(
        &scene,
        &["Feed"],
        &[Some(0), Some(0), None],
        &colours,
    )
    .unwrap();
    let motion = [
        [0., -1., 0., 10.],
        [1., 0., 0., 3.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    let result = cad_step::move_step_product_branch(&original, &["Drive", "Feed"], motion).unwrap();
    let (back, paths) = cad_step::import_step_product_paths(&result).unwrap();
    assert_eq!(back.assembly.definitions().len(), 1);
    assert_eq!(paths.len(), 3);
    for ((place, path), colour) in back
        .assembly
        .occurrences()
        .iter()
        .zip(paths)
        .zip(cad_step::import_step_occurrence_colours(&result).unwrap())
    {
        let (x, y) = (place.transform[0][3], place.transform[1][3]);
        if path.len() == 3 {
            assert_eq!(path[1], "Feed");
            assert_eq!(x, 10.);
            assert_eq!(place.transform[1][0], 1.);
            assert!(matches!(
                (y, colour),
                (5., Some([1., 0., 0.])) | (8., Some([0., 1., 0.]))
            ));
        } else {
            assert_eq!((x, y, colour), (20., 0., Some([0., 0., 1.])));
        }
    }
    assert!(cad_step::move_step_product_branch(&original, &[], motion).is_err());
    assert!(cad_step::move_step_product_branch(&original, &["Missing"], motion).is_err());
    assert!(cad_step::move_step_product_branch(&original, &["Drive"], [[0.; 4]; 4]).is_err());
    for (path, upper_x) in [(vec!["Drive", "Feed"], 10.), (vec!["Drive"], 22.)] {
        let [lo, hi] = cad_step::step_branch_bounds(&result, &path).unwrap();
        let lower = [6., if path.len() == 1 { 0. } else { 5. }, 0.];
        let upper = [upper_x, 10., 6.];
        for axis in 0..3 {
            assert!(lo[axis] <= lower[axis] && lower[axis] - lo[axis] < 1e-10);
            assert!(hi[axis] >= upper[axis] && hi[axis] - upper[axis] < 1e-10);
        }
    }
    assert!(cad_step::step_branch_bounds(&result, &["Drive", "Missing"]).is_err());
    let mass = cad_step::step_branch_mass_properties(
        &result,
        &["Drive", "Feed"],
        1e-8,
        100_000,
        |path, part| {
            assert_eq!(path, ["Drive", "Feed", "Bracket"]);
            Ok(if part.transform[1][3] == 5. {
                0.001
            } else {
                0.002
            })
        },
    )
    .unwrap();
    assert!((mass.mass - 0.144).abs() < 1e-12);
    for (v, w) in mass.centroid.iter().zip([8., 8., 3.]) {
        assert!((*v - w).abs() < 1e-9);
    }
    for (point, axis, expected) in [
        ([5., 4., 0.], [0., 0., 2.], [0.432, 0.576, 0.]),
        ([5., 4., 1e15], [0., 0., -1e-300], [0.432, 0.576, 0.]),
        ([8., 8., 3.], [2., -3., 4.], [0.; 3]),
        ([5., 4., 0.], [1., 1., 0.], [-0.072, 0.072, 0.432]),
    ] {
        let imbalance = mass.static_unbalance(point, axis).unwrap();
        for (actual, target) in imbalance.iter().zip(expected) {
            assert!((actual - target).abs() < 1e-10);
        }
    }
    assert!(mass.static_unbalance([0.; 3], [0.; 3]).is_err());
    let sites = [
        [[-12., 0., 0.], [0., -6., 0.]],
        [[0., -12., 0.], [-6., 0., 0.]],
    ];
    let loads = mass
        .balance_at_fixed_sites([5., 4., 0.], [0., 0., 1.], [0., 12.], sites)
        .unwrap();
    for (pair, expected) in loads.iter().zip([[0.027, 0.072], [0.012, 0.018]]) {
        for (actual, wanted) in pair.iter().zip(expected) {
            assert!((actual - wanted).abs() < 1e-10);
        }
    }
    for (rounded, expected) in [
        (false, [[0.; 3]; 2]),
        (true, [[0.036, -0.012, 0.], [0.012, -0.024, 0.]]),
    ] {
        let mut fitted = mass.clone();
        for (plane, pair) in loads.iter().enumerate() {
            for (index, &value) in pair.iter().enumerate() {
                let [x, y, z] = sites[plane][index];
                let kg = if rounded {
                    (value * 100.).round() / 100.
                } else {
                    value
                };
                fitted = fitted
                    .with_point_mass([5. + x, 4. + y, 12. * plane as f64 + z], kg)
                    .unwrap();
            }
        }
        let residual = fitted
            .two_plane_balance_corrections([5., 4., 0.], [0., 0., 1.], [0., 12.])
            .unwrap();
        for (row, want) in residual.iter().zip(expected) {
            for (a, b) in row.iter().zip(want) {
                assert!((a - b).abs() < 1e-10);
            }
        }
    }
    assert_eq!(mass.with_point_mass([0.; 3], 0.).unwrap(), mass);
    assert!(mass.with_point_mass([0.; 3], -1.).is_err());
    let oblique = [[[-6., -3., 0.], [0., -8., 0.]]; 2];
    let split = mass
        .balance_at_fixed_sites([5., 4., 0.], [0., 0., 1.], [0., 12.], oblique)
        .unwrap();
    for (pair, expected) in split.iter().zip([[0.054, 0.03375], [0.018, 0.01125]]) {
        for (actual, wanted) in pair.iter().zip(expected) {
            assert!((actual - wanted).abs() < 1e-10);
        }
    }
    for invalid in [[12., 0., 0.], [0., -2., 0.], [0., 0., 1.]] {
        let mut bad = sites;
        bad[0][0] = invalid;
        assert!(
            mass.balance_at_fixed_sites([5., 4., 0.], [0., 0., 1.], [0., 12.], bad)
                .is_err()
        );
    }
    let corrections = mass
        .two_plane_balance_corrections([5., 4., 0.], [0., 0., 1.], [0., 12.])
        .unwrap();
    for (vector, expected) in corrections
        .iter()
        .zip([[-0.324, -0.432, 0.], [-0.108, -0.144, 0.]])
    {
        for (v, w) in vector.iter().zip(expected) {
            assert!((v - w).abs() < 1e-10);
        }
    }
    // A tilted axis through the centre has zero static moment and a nonzero couple.
    let couple = mass
        .two_plane_balance_corrections([8., 8., 3.], [1., 1., 0.], [-2., 2.])
        .unwrap();
    let a = 0.018 / 2_f64.sqrt();
    for (row, expected) in couple.iter().zip([[-a, a, 0.], [a, -a, 0.]]) {
        for (v, w) in row.iter().zip(expected) {
            assert!((v - w).abs() < 1e-10);
        }
    }
    assert!(
        mass.two_plane_balance_corrections([0.; 3], [1., 0., 0.], [1., 1.])
            .is_err()
    );
    let counterweight = cad_step::step_branch_mass_properties(
        &result,
        &["Drive", "Bracket"],
        1e-8,
        100_000,
        |_, _| Ok(0.003),
    )
    .unwrap();
    let complete =
        cad_step::step_branch_mass_properties(&result, &["Drive"], 1e-8, 100_000, |path, part| {
            Ok(if path.len() == 2 {
                0.003
            } else if part.transform[1][3] == 5. {
                0.001
            } else {
                0.002
            })
        })
        .unwrap();
    for (remove, expected) in [(&counterweight, &mass), (&mass, &counterweight)] {
        let remaining = complete.without_component(remove).unwrap();
        assert!((remaining.mass - expected.mass).abs() < 1e-12);
        for i in 0..3 {
            assert!((remaining.centroid[i] - expected.centroid[i]).abs() < 1e-9);
            for j in 0..3 {
                assert!((remaining.inertia[i][j] - expected.inertia[i][j]).abs() < 1e-9);
            }
        }
    }
    assert!(complete.without_component(&complete).is_err());
    let mut distant = counterweight.clone();
    distant.centroid[0] += 1000.;
    assert!(complete.without_component(&distant).is_err());
    for fitted in [
        mass.combined_with(&counterweight).unwrap(),
        counterweight.combined_with(&mass).unwrap(),
    ] {
        assert!((fitted.mass - complete.mass).abs() < 1e-12);
        for i in 0..3 {
            assert!((fitted.centroid[i] - complete.centroid[i]).abs() < 1e-9);
            for j in 0..3 {
                assert!((fitted.inertia[i][j] - complete.inertia[i][j]).abs() < 1e-9);
            }
        }
        assert!(
            (fitted
                .moment_about_axis([5., 4., 0.], [0., 0., 1.])
                .unwrap()
                - 41.808)
                .abs()
                < 1e-9
        );
    }
    let initial_mass = cad_step::step_branch_mass_properties(
        &original,
        &["Drive", "Feed"],
        1e-8,
        100_000,
        |_, part| {
            Ok(if part.transform[0][3] == 2. {
                0.001
            } else {
                0.002
            })
        },
    )
    .unwrap();
    let placed = initial_mass.rigidly_placed(motion).unwrap();
    assert_eq!(placed.mass, initial_mass.mass);
    for i in 0..3 {
        assert!((placed.centroid[i] - mass.centroid[i]).abs() < 1e-10);
        for j in 0..3 {
            assert!((placed.inertia[i][j] - mass.inertia[i][j]).abs() < 1e-10);
        }
    }
    let oblique_pose = [
        [0.6, -0.8, 0., 3.],
        [0.8, 0.6, 0., 4.],
        [0., 0., 1., 5.],
        [0., 0., 0., 1.],
    ];
    let tilted = mass.rigidly_placed(oblique_pose).unwrap();
    assert!(
        (tilted.inertia[0][1] - 0.48 * (mass.inertia[0][0] - mass.inertia[1][1])).abs() < 1e-10
    );
    assert!((tilted.inertia[1][0] - tilted.inertia[0][1]).abs() < 1e-10);
    let mut scaled = motion;
    scaled[0][1] = -2.;
    assert!(initial_mass.rigidly_placed(scaled).is_err());
    let bearing = mass.moment_about_axis([5., 4., 0.], [0., 0., -2.]).unwrap();
    assert!((bearing - 4.128).abs() < 1e-9);
    // Rotated box sides in world coordinates are 4, 2, 6 mm. The two
    // centres are separated by 3 mm along Y.
    let shift = 0.048 * 0.096 / 0.144 * 9.;
    let diagonal = [
        0.144 * 40. / 12. + shift,
        0.144 * 52. / 12.,
        0.144 * 20. / 12. + shift,
    ];
    for (i, row) in mass.inertia.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            assert!((value - if i == j { diagonal[i] } else { 0. }).abs() < 1e-9);
        }
    }
    assert!(
        cad_step::step_branch_mass_properties(
            &result,
            &["Drive", "Feed"],
            1e-8,
            100_000,
            |_, _| Ok(0.)
        )
        .is_err()
    );
    if let Some(path) = std::env::var_os("CAD_BRANCH_MOTION_FIXTURE") {
        std::fs::write(path, result).unwrap();
    }
}

#[test]
fn branch_bill_counts_coincident_instances_by_definition() {
    let item = |definition, x| StepAssemblyOccurrence {
        definition,
        transform: [
            [1., 0., 0., x],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    };
    let scene = cad_step::NamedStepAssembly {
        name: "Drive".into(),
        definition_names: vec!["Pin".into(), "Pin".into(), "Plate".into()],
        assembly: StepSharedAssembly::from_placed_solids(
            vec![
                brep_core::cylinder(1., 2.).unwrap(),
                brep_core::cylinder(2., 3.).unwrap(),
                brep_core::cuboid([0.; 3], [1.; 3]).unwrap(),
            ],
            vec![
                item(0, 0.),
                item(0, 0.),
                item(1, 5.),
                item(0, 20.),
                item(2, 30.),
            ],
        )
        .unwrap(),
    };
    let step = cad_step::export_named_styled_step_assembly(
        &scene,
        &["Feed"],
        &[Some(0), Some(0), Some(0), None, None],
        &[None, None, None, None, None],
    )
    .unwrap();
    let rows = cad_step::step_branch_parts_list(&step, &["Drive", "Feed"]).unwrap();
    let mut counts: Vec<_> = rows
        .iter()
        .map(|r| (r.name.as_str(), r.inside, r.outside))
        .collect();
    counts.sort();
    assert_eq!(counts, [("Pin", 1, 0), ("Pin", 2, 1)]);
    assert_ne!(rows[0].definition, rows[1].definition);
    assert_eq!(
        cad_step::step_branch_parts_list(&step, &["Drive"])
            .unwrap()
            .iter()
            .map(|r| r.inside)
            .sum::<usize>(),
        5
    );
    assert!(cad_step::step_branch_parts_list(&step, &["Feed"]).is_err());
    assert!(cad_step::step_branch_parts_list(&step, &[]).is_err());
    if let Some(path) = std::env::var_os("CAD_BRANCH_BOM_FIXTURE") {
        std::fs::write(path, step).unwrap();
    }
}
