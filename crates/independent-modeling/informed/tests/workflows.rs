use modeling_informed::{
    Display, Document, Error, Geometry, GeometryKind, Operation, Resolution,
    geometry::{
        boolean::Boolean,
        curve::Curve,
        math::{Affine, Bounds},
    },
};
use std::sync::Arc;
fn cube() -> Operation {
    Operation::Box {
        bounds: Bounds::new([0.; 3], [2.; 3]).unwrap(),
    }
}
fn id(doc: &mut Document, op: Operation) -> modeling_informed::EntityId {
    doc.apply(op).unwrap().entities[0]
}
fn volume(doc: &mut Document, id: modeling_informed::EntityId) -> f64 {
    match &*doc.display(id, Resolution::default()).unwrap() {
        Display::Triangles(m) => m.signed_volume(),
        _ => panic!("mesh expected"),
    }
}

#[test]
fn transaction_failure_and_drop_never_publish_partial_geometry() {
    let mut doc = Document::new().unwrap();
    let initial = doc.revision();
    let aborted;
    {
        let mut tx = doc.transaction();
        aborted = tx.apply(cube()).unwrap()[0];
        assert!(tx.preview().entity(aborted).is_some());
        assert!(
            tx.apply(Operation::Sphere {
                radius: -1.,
                latitude: 12,
                longitude: 24
            })
            .is_err()
        );
        assert_eq!(tx.commit().unwrap_err(), Error::FailedTransaction);
    }
    assert_eq!(doc.revision(), initial);
    assert!(doc.entities().next().is_none());
    assert!(doc.entity(aborted).is_none());
    {
        let mut tx = doc.transaction();
        tx.apply(cube()).unwrap();
    }
    assert_eq!(doc.revision(), initial);
    assert_eq!(doc.entities().count(), 0);
    let committed = id(&mut doc, cube());
    assert_ne!(committed, aborted);
}
#[test]
fn grouped_commit_undo_redo_and_branching_have_monotonic_revisions() {
    let mut doc = Document::new().unwrap();
    let (a, b);
    {
        let mut tx = doc.transaction();
        a = tx.apply(cube()).unwrap()[0];
        b = tx.apply(cube()).unwrap()[0];
        assert_eq!(tx.commit().unwrap().revision, 1);
    }
    assert_eq!(doc.entities().count(), 2);
    assert!(doc.undo().unwrap());
    assert_eq!(doc.revision(), 2);
    assert_eq!(doc.entities().count(), 0);
    assert!(doc.redo().unwrap());
    assert_eq!(doc.revision(), 3);
    assert!(doc.entity(a).is_some() && doc.entity(b).is_some());
    doc.undo().unwrap();
    let new = id(&mut doc, cube());
    assert_ne!(new, a);
    assert_ne!(new, b);
    assert!(!doc.redo().unwrap());
}
#[test]
fn background_job_conflict_retains_computed_result() {
    let mut doc = Document::new().unwrap();
    let job = doc.prepare(cube());
    let computed = std::thread::spawn(move || job.run())
        .join()
        .unwrap()
        .unwrap();
    let pending = computed.entities()[0];
    id(&mut doc, cube());
    let revision = doc.revision();
    let error = doc.publish(computed).unwrap_err();
    assert_eq!(
        error.reason,
        Error::StaleRevision {
            expected: 0,
            actual: 1
        }
    );
    assert_eq!(doc.revision(), revision);
    assert!(doc.entity(pending).is_none());
    assert!(error.computed.preview().entity(pending).is_some());
}
#[test]
fn successful_job_is_one_atomic_edit_and_cannot_cross_documents() {
    let mut a = Document::new().unwrap();
    let mut b = Document::new().unwrap();
    let computed = a.prepare(cube()).run().unwrap();
    let entity = computed.entities()[0];
    let rejected = b.publish(computed).unwrap_err();
    assert_eq!(rejected.reason, Error::WrongDocument);
    assert_eq!(b.entities().count(), 0);
    a.publish(rejected.computed).unwrap();
    assert!(a.entity(entity).is_some());
    assert_eq!(a.revision(), 1);
    a.undo().unwrap();
    assert!(a.entity(entity).is_none());
}
#[test]
fn cancellation_before_work_and_before_publication_does_not_change_document() {
    let mut d = Document::new().unwrap();
    let job = d.prepare(cube());
    job.cancellation().cancel();
    assert_eq!(job.run().unwrap_err(), Error::Cancelled);
    let job = d.prepare(cube());
    let cancel = job.cancellation();
    let result = job.run().unwrap();
    cancel.cancel();
    assert_eq!(d.publish(result).unwrap_err().reason, Error::Cancelled);
    assert_eq!(d.revision(), 0);
    assert_eq!(d.entities().count(), 0);
}
#[test]
fn derived_mesh_cache_reuses_work_and_does_not_replace_geometry() {
    let mut doc = Document::new().unwrap();
    let c = id(
        &mut doc,
        Operation::AddCurve(Curve::circle([0.; 3], 3.).unwrap()),
    );
    let surface = id(
        &mut doc,
        Operation::ExtrudeSurface {
            profile: c,
            vector: [0., 0., 5.],
        },
    );
    let settings = Resolution::default();
    let a = doc.display(surface, settings).unwrap();
    let b = doc.display(surface, settings).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    assert_eq!(
        doc.entity(surface).unwrap().geometry().kind(),
        GeometryKind::Surface
    );
    let snapshot = doc.snapshot();
    let before = doc.entity(surface).unwrap().content_id();
    doc.apply(Operation::Transform {
        target: surface,
        transform: Affine::translation([1., 0., 0.]).unwrap(),
    })
    .unwrap();
    assert_ne!(before, doc.entity(surface).unwrap().content_id());
    assert_eq!(snapshot.entity(surface).unwrap().content_id(), before);
    let moved = doc.display(surface, settings).unwrap();
    assert!(!Arc::ptr_eq(&a, &moved));
    doc.undo().unwrap();
    assert_eq!(doc.entity(surface).unwrap().content_id(), before);
}
#[test]
fn booleans_keep_reusable_cells_and_refuse_transformed_mesh_authority() {
    let mut d = Document::new().unwrap();
    let a = id(&mut d, cube());
    let b = id(
        &mut d,
        Operation::Box {
            bounds: Bounds::new([1.; 3], [3.; 3]).unwrap(),
        },
    );
    let cut = id(
        &mut d,
        Operation::BoxBoolean {
            left: a,
            right: b,
            kind: Boolean::Difference,
        },
    );
    assert!((volume(&mut d, cut) - 7.).abs() < 1e-12);
    let empty = id(
        &mut d,
        Operation::BoxBoolean {
            left: cut,
            right: a,
            kind: Boolean::Difference,
        },
    );
    assert_eq!(volume(&mut d, empty), 0.);
    d.apply(Operation::Transform {
        target: a,
        transform: Affine::rotation([0., 0., 1.], 0.3).unwrap(),
    })
    .unwrap();
    assert_eq!(
        d.entity(a).unwrap().geometry().kind(),
        GeometryKind::FacetedMesh
    );
    let revision = d.revision();
    let err = d
        .apply(Operation::BoxBoolean {
            left: a,
            right: b,
            kind: Boolean::Union,
        })
        .unwrap_err();
    assert!(matches!(err, Error::WrongGeometry { .. }));
    assert_eq!(d.revision(), revision);
}
#[test]
fn curve_edits_and_import_preserve_the_document_workflow() {
    let mut d = Document::new().unwrap();
    let c = id(
        &mut d,
        Operation::AddCurve(Curve::circle([0.; 3], 2.).unwrap()),
    );
    d.apply(Operation::InsertKnot {
        target: c,
        parameter: 0.13,
    })
    .unwrap();
    let split = d
        .apply(Operation::SplitCurve {
            target: c,
            parameter: 0.4,
        })
        .unwrap()
        .entities;
    assert_eq!(split.len(), 2);
    for id in split {
        assert!(matches!(
            d.entity(id).unwrap().geometry(),
            Geometry::Curve(_)
        ));
    }
    let solid = id(
        &mut d,
        Operation::ExtrudePolygon {
            profile: vec![[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
            height: 3.,
        },
    );
    let display = d.display(solid, Resolution::default()).unwrap();
    let Display::Triangles(mesh) = &*display else {
        panic!()
    };
    let imported = id(&mut d, Operation::ImportObj(mesh.to_obj()));
    assert!((volume(&mut d, imported) - 12.).abs() < 1e-12);
    d.apply(Operation::Delete(imported)).unwrap();
    assert!(d.entity(imported).is_none());
    d.undo().unwrap();
    assert!(d.entity(imported).is_some());
}
#[test]
fn history_limit_and_empty_transaction_are_explicit() {
    let mut d = Document::new().unwrap();
    assert_eq!(d.transaction().commit().unwrap().revision, 0);
    d.set_history_limit(1).unwrap();
    id(&mut d, cube());
    id(&mut d, cube());
    assert!(d.undo().unwrap());
    assert!(!d.undo().unwrap());
    d.set_history_limit(0).unwrap();
    id(&mut d, cube());
    assert!(!d.undo().unwrap());
    assert!(d.set_history_limit(1025).is_err());
}

#[test]
fn grouped_receipt_contains_only_surviving_unique_outputs() {
    let mut d = Document::new().unwrap();
    let mut tx = d.transaction();
    let deleted = tx.apply(cube()).unwrap()[0];
    let retained = tx.apply(cube()).unwrap()[0];
    tx.apply(Operation::Delete(deleted)).unwrap();
    tx.apply(Operation::Transform {
        target: retained,
        transform: Affine::identity(),
    })
    .unwrap();
    let receipt = tx.commit().unwrap();
    assert_eq!(receipt.entities, vec![retained]);
    assert_eq!(d.entities().count(), 1);
}
