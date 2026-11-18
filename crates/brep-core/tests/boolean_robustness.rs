use brep_core::{analysis::mass_properties, boolean, boolean_robustness::{
        boolean_with_sheet_semantics, classify_boolean_operand, preclassify_coincident_faces,
        BooleanOperandKind, CoincidentFaceClass, SheetBooleanSemantics,
    }, cuboid, Model};

fn volume(model: &Model) -> f64 {
    if model.is_empty() {
        0.
    } else {
        mass_properties(model, 1e-8, 100_000)
            .unwrap()
            .signed_volume_mm3
    }
}

fn face_at_x(model: &Model, x: f64) -> usize {
    model
        .faces
        .iter()
        .position(|face| {
            face.surface
                .control_points
                .iter()
                .flatten()
                .all(|point| (point[0] - x).abs() < 1e-12)
        })
        .unwrap()
}

fn open_sheet(mut model: Model) -> Model {
    model.bodies.clear();
    for shell in &mut model.shells {
        shell.closed = false;
    }
    model.rebuild_topology_ids();
    model
}

#[test]
fn coincident_faces_are_classified_before_curved_marching() {
    let a = cuboid([0.; 3], [1.; 3]).unwrap();
    let b = cuboid([1., 0., 0.], [2., 1., 1.]).unwrap();
    assert_eq!(
        preclassify_coincident_faces(&a, 0, &a, 0).unwrap(),
        CoincidentFaceClass::IdenticalCarrier
    );
    let class = preclassify_coincident_faces(&a, face_at_x(&a, 1.), &b, face_at_x(&b, 1.)).unwrap();
    assert!(
        matches!(
            class,
            CoincidentFaceClass::CoincidentPlane { .. } | CoincidentFaceClass::EquivalentNurbs(_)
        ),
        "{class:?}"
    );
    let delta = a.tolerance_mm * 2.;
    let near = cuboid([1. + delta, 0., 0.], [2. + delta, 1., 1.]).unwrap();
    assert_eq!(
        preclassify_coincident_faces(&a, face_at_x(&a, 1.), &near, face_at_x(&near, 1. + delta),)
            .unwrap(),
        CoincidentFaceClass::NearCoincident
    );
    assert!(preclassify_coincident_faces(&a, usize::MAX, &b, 0).is_err());
}

#[test]
fn analytic_cylinders_require_coaxial_bounded_control_net_evidence() {
    let a = brep_core::analytic::cylinder(1., 2.).unwrap();
    let b = brep_core::analytic::cylinder(1., 2.).unwrap();
    let mut equivalent = b.clone();
    equivalent.faces[0].surface.knots_u = equivalent.faces[0]
        .surface
        .knots_u
        .iter()
        .map(|k| 2. + 3. * k)
        .collect();
    equivalent.faces[0].surface.weights = equivalent.faces[0]
        .surface
        .weights
        .iter()
        .map(|row| row.iter().map(|w| 4. * w).collect())
        .collect();
    assert!(matches!(
        preclassify_coincident_faces(&a, 0, &equivalent, 0).unwrap(),
        CoincidentFaceClass::EquivalentNurbs(_)
    ));
    let mut approximate = b.clone();
    for point in approximate.faces[0]
        .surface
        .control_points
        .iter_mut()
        .flatten()
    {
        point[2] += 1e-9;
    }
    let class = preclassify_coincident_faces(&a, 0, &approximate, 0).unwrap();
    assert!(
        matches!(
            class,
            CoincidentFaceClass::CoincidentCylinder { max_deviation_mm, .. } if max_deviation_mm <= a.tolerance_mm
        ),
        "{class:?}"
    );
    let different = brep_core::analytic::cylinder(2., 2.).unwrap();
    assert_eq!(
        preclassify_coincident_faces(&a, 0, &different, 0).unwrap(),
        CoincidentFaceClass::NonCoincident
    );
}

#[test]
fn sheets_never_become_implicit_solid_operands() {
    let solid = cuboid([0.; 3], [1.; 3]).unwrap();
    let sheet = open_sheet(solid.clone());
    assert_eq!(
        classify_boolean_operand(&solid).unwrap(),
        BooleanOperandKind::Solid
    );
    assert_eq!(
        classify_boolean_operand(&sheet).unwrap(),
        BooleanOperandKind::Sheet
    );
    let error = boolean(&solid, &sheet, "intersection").unwrap_err();
    assert_eq!(error.code, "BREP_SHEET_BOOLEAN_REQUIRES_SEMANTICS");
    let error =
        boolean_with_sheet_semantics(&solid, &sheet, SheetBooleanSemantics::TrimSolidBySheet)
            .unwrap_err();
    assert_eq!(error.code, "BREP_SHEET_BOOLEAN_UNIMPLEMENTED");
    let error =
        boolean_with_sheet_semantics(&solid, &solid, SheetBooleanSemantics::TrimSolidBySheet)
            .unwrap_err();
    assert_eq!(error.code, "BREP_SHEET_BOOLEAN_OPERAND_MISMATCH");
}

#[test]
fn deterministic_cuboid_corpus_obeys_boolean_volume_and_symmetry_invariants() {
    // A tiny deterministic xorshift corpus: no random-state or test dependency.
    let mut state = 0x4d595df4d0f33173_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % 7) as f64 / 2.
    };
    for _ in 0..12 {
        let a = cuboid([0.; 3], [2., 3., 4.]).unwrap();
        let lo = [next() - 1., next() - 1., next() - 1.];
        let hi = [
            lo[0] + 1. + next() / 3.,
            lo[1] + 1. + next() / 3.,
            lo[2] + 1. + next() / 3.,
        ];
        let b = cuboid(lo, hi).unwrap();
        let union = boolean(&a, &b, "union").unwrap();
        let intersection = boolean(&a, &b, "intersection").unwrap();
        let difference = boolean(&a, &b, "difference").unwrap();
        for result in [&union, &intersection, &difference] {
            result.validate().unwrap();
            let used = result.loops.iter().flat_map(|wire| &wire.coedges).fold(
                vec![0_usize; result.edges.len()],
                |mut counts, coedge| {
                    counts[coedge.edge] += 1;
                    counts
                },
            );
            assert!(used.into_iter().all(|count| count == 2));
        }
        let va = volume(&a);
        let vb = volume(&b);
        let vu = volume(&union);
        let vi = volume(&intersection);
        let tolerance = (va + vb).max(1.) * 2e-7;
        assert!((vu + vi - va - vb).abs() <= tolerance);
        assert!((volume(&difference) + vi - va).abs() <= tolerance);
        assert!((volume(&boolean(&b, &a, "union").unwrap()) - vu).abs() <= tolerance);
        assert!((volume(&boolean(&b, &a, "intersection").unwrap()) - vi).abs() <= tolerance);
    }
}
