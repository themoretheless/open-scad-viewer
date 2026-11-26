use cad_step::{
    StepAssemblyOccurrence, StepSharedAssembly, export_named_step_assembly,
    import_named_step_assembly,
};

#[test]
fn compact_definition_survives_named_step_round_trip() {
    let mut box_model = brep_core::cuboid([0.; 3], [8., 4., 2.]).unwrap();
    let chart = &mut box_model.faces[0].surface;
    let middle = chart.control_points[0]
        .iter()
        .zip(&chart.control_points[1])
        .map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x + y) * 0.5).collect())
        .collect();
    chart.control_points.insert(1, middle);
    chart.weights.insert(1, chart.weights[0].clone());
    chart.knots_u.insert(2, 0.5);
    let placement = |definition, x| StepAssemblyOccurrence {
        definition,
        transform: [
            [1., 0., 0., x],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    };
    let mut rotated = placement(0, 40.);
    rotated.transform[0][0] = 0.;
    rotated.transform[0][1] = -1.;
    rotated.transform[1][0] = 1.;
    rotated.transform[1][1] = 0.;
    let assembly = StepSharedAssembly::from_placed_solids(
        vec![box_model, brep_core::cuboid([0.; 3], [1.; 3]).unwrap()],
        vec![placement(0, 0.), placement(1, 20.), rotated],
    )
    .unwrap();
    let names = ["Bracket", "Pin"];
    let initial = export_named_step_assembly(&assembly, "Mount", &names).unwrap();
    let mut imported = import_named_step_assembly(&initial).unwrap();
    let repeated = imported
        .definition_names
        .iter()
        .position(|name| name == "Bracket")
        .unwrap();
    let baseline = imported.assembly.definitions().to_vec();
    let i = baseline[repeated]
        .faces
        .iter()
        .position(|face| face.surface.control_points.len() == 3)
        .unwrap();
    assert!(
        !imported
            .assembly
            .compress_definition_faces(repeated, &[i], 1)
            .unwrap()
    );
    assert!(
        imported
            .assembly
            .compress_definition_faces(repeated, &[usize::MAX], 100)
            .is_err()
    );
    assert_eq!(baseline, imported.assembly.definitions());
    assert!(
        imported
            .assembly
            .compress_definition_faces(repeated, &[i], 1_000_000)
            .unwrap()
    );
    let labels: Vec<_> = imported
        .definition_names
        .iter()
        .map(String::as_str)
        .collect();
    let edited = export_named_step_assembly(&imported.assembly, &imported.name, &labels).unwrap();
    if let Some(directory) = std::env::var_os("CAD_SHARED_COMPRESSION_FIXTURES") {
        let directory = std::path::Path::new(&directory);
        std::fs::write(directory.join("original.step"), &initial).unwrap();
        std::fs::write(directory.join("compressed.step"), &edited).unwrap();
        let mut edge_model = baseline[repeated].clone();
        let edge = 0;
        let refine = |curve: &mut nurbs_core::curve::Curve| {
            *curve = curve.insert((curve.domain()[0] + curve.domain()[1]) / 2., 1).unwrap();
        };
        refine(&mut edge_model.edges[edge].curve);
        for coedge in edge_model.loops.iter_mut()
            .flat_map(|wire| &mut wire.coedges).filter(|coedge| coedge.edge == edge)
        {
            refine(&mut coedge.pcurve);
        }
        let (step, _, _) = cad_step::export_step_v9(&edge_model).unwrap();
        std::fs::write(directory.join("refined-edge.step"), step).unwrap();
    }
    assert!(edited.len() < initial.len());
    let restored = import_named_step_assembly(&edited).unwrap();
    assert_eq!(restored.name, "Mount");
    for (a, b) in imported
        .assembly
        .occurrences()
        .iter()
        .zip(restored.assembly.occurrences())
    {
        assert_eq!(
            imported.definition_names[a.definition],
            restored.definition_names[b.definition]
        );
        assert_eq!(a.transform, b.transform);
    }
    let usage = restored.surface_usage().unwrap();
    assert_eq!(usage.len(), 2);
    assert_eq!(
        (
            usage[0].part.name,
            usage[0].part.quantity,
            usage[0].placed_poles
        ),
        ("Bracket", 2, 48)
    );
    assert_eq!(
        (
            usage[1].part.name,
            usage[1].part.quantity,
            usage[1].placed_poles
        ),
        ("Pin", 1, 24)
    );
    for body in restored.assembly.definitions() {
        assert_eq!(body.validate().unwrap().boundary_edge_count, 0);
    }
    eprintln!(
        "Shared assembly bytes {} -> {}",
        initial.len(),
        edited.len()
    );
}
