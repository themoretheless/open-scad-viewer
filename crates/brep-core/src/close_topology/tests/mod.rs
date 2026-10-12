use super::*;
use crate::cuboid;

fn solid(model: Model) -> ComplexPart {
    ComplexPart {
        role: BodyRole::Solid,
        model,
    }
}

#[test]
fn solid_typestate_is_not_weakened() {
    let one = MixedDimensionalBrep::new(
        vec![solid(cuboid([0.; 3], [1.; 3]).unwrap())],
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .audit()
    .unwrap();
    one.try_into_globally_audited_solid_set().unwrap();

    let a = cuboid([0.; 3], [1.; 3]).unwrap();
    let b = a.clone();
    let relation = SharedFace {
        uses: vec![
            FaceRef {
                part: 0,
                face: 0,
                reversed: false,
            },
            FaceRef {
                part: 1,
                face: 0,
                reversed: true,
            },
        ],
    };
    let mixed = MixedDimensionalBrep::new(
        vec![solid(a), solid(b)],
        vec![relation],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .audit()
    .unwrap();
    assert_eq!(mixed.certificate().solid_cell_count, 2);
    assert_eq!(mixed.certificate().shared_face_count, 1);
    assert_eq!(mixed.boundary_faces().len(), 10);
    assert_eq!(mixed.manifold_decomposition().len(), 2);
    assert_eq!(
        mixed
            .try_into_globally_audited_solid_set()
            .unwrap_err()
            .code,
        "BREP_SOLID_AUDIT_REFUSED"
    );
}

#[test]
fn radial_fans_and_orientation_are_checked() {
    let model = cuboid([0.; 3], [1.; 3]).unwrap();
    let edge = model.1.edges[0];
    let face = model.1.faces[0];
    let parts = vec![
        solid(model.clone()),
        solid(model.clone()),
        solid(model.clone()),
    ];
    let ring = EdgeRadialRing {
        uses: (0..3)
            .map(|part| EdgeUseRef {
                part,
                face: 0,
                edge: 0,
                reversed: part % 2 == 0,
            })
            .collect(),
    };
    let fan = VertexFan {
        vertex: (0, model.edges[0].vertices[0]),
        uses: vec![VertexUseRef {
            part: 0,
            face: 0,
            vertex: model.edges[0].vertices[0],
        }],
        closed: false,
    };
    let audited = MixedDimensionalBrep::new(
        parts,
        vec![],
        vec![ring],
        vec![fan],
        vec![TopologyLineageRecord {
            operation: "share".into(),
            entity_kind: "edge".into(),
            parents: vec![edge],
            children: vec![edge, face],
        }],
    )
    .unwrap()
    .audit()
    .unwrap();
    assert_eq!(audited.certificate().max_radial_valence, 3);
    assert_eq!(audited.certificate().vertex_fan_count, 1);

    let bad = SharedFace {
        uses: vec![
            FaceRef {
                part: 0,
                face: 0,
                reversed: false,
            },
            FaceRef {
                part: 1,
                face: 0,
                reversed: false,
            },
        ],
    };
    assert_eq!(
        MixedDimensionalBrep::new(
            vec![solid(model.clone()), solid(model)],
            vec![bad],
            vec![],
            vec![],
            vec![]
        )
        .unwrap()
        .audit()
        .unwrap_err()
        .code,
        "BREP_COMPLEX_ORIENTATION_REFUSED"
    );
}

#[test]
fn multi_cell_heal_budgets_partitions_and_atomic_refusal() {
    let model = cuboid([0.; 3], [1.; 3]).unwrap();
    let audited = MixedDimensionalBrep::new(
        vec![solid(model.clone()), solid(model.clone())],
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .audit()
    .unwrap();
    let plan = ComplexHealPlan {
        context_canonical: "v1:test".into(),
        correspondences: vec![LocalCorrespondence {
            kind: CorrespondenceKind::Vertex,
            entities: vec![(0, model.1.vertices[0]), (1, model.1.vertices[0])],
            displacement_mm: 1e-8,
            context_path: vec!["compound".into(), "cell-pair".into(), "vertex".into()],
            provenance: "qualified-test-correspondence".into(),
        }],
        per_entity_limit_mm: 2e-8,
        cumulative_limit_mm: 3e-8,
        partitions: vec![ExactParameterPartition {
            carrier_definition: "exact-line".into(),
            parent_domain_bits: [0f64.to_bits(), 1f64.to_bits()],
            child_domain_bits: vec![
                [0f64.to_bits(), 0.5f64.to_bits()],
                [0.5f64.to_bits(), 1f64.to_bits()],
            ],
        }],
    };
    let cert = certify_complex_heal(&audited, &plan).unwrap();
    assert_eq!(cert.touched_entity_count, 2);
    assert_eq!(cert.cumulative_displacement_mm, 2e-8);
    assert!(cert.rollback_safe && cert.idempotent && cert.no_tolerance_growth);
    let mut over = plan.clone();
    over.cumulative_limit_mm = 1e-8;
    assert_eq!(
        certify_complex_heal(&audited, &over).unwrap_err().code,
        "BREP_HEAL_BUDGET_EXCEEDED"
    );
}

#[test]
fn sheet_solid_compound_transform_permutation_and_resources() {
    let source = cuboid([0.; 3], [1.; 3]).unwrap();
    let placed = crate::transform::affine(
        &source,
        [
            [0., -1., 0., 4.],
            [1., 0., 0., 5.],
            [0., 0., 1., 6.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let mut sheet = placed.clone();
    sheet.0.bodies.clear();
    sheet.1.bodies.clear();
    sheet.validate().unwrap();
    let audited = MixedDimensionalBrep::new(
        vec![
            ComplexPart {
                role: BodyRole::SheetShell,
                model: sheet,
            },
            solid(placed),
        ],
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .audit()
    .unwrap();
    assert_eq!(
        (
            audited.certificate().sheet_count,
            audited.certificate().solid_cell_count
        ),
        (1, 1)
    );
    assert_eq!(audited.boundary_faces().len(), 12);

    let over = (0..=MAX_PARTS).map(|_| solid(source.clone())).collect();
    assert_eq!(
        MixedDimensionalBrep::new(over, vec![], vec![], vec![], vec![])
            .unwrap_err()
            .code,
        "BREP_COMPLEX_RESOURCE_LIMIT"
    );
}
