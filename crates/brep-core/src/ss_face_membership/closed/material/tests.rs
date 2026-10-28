use super::*;
use crate::graph_body::{
    self,
    tests::{column, limits},
};
use nurbs_core::ss_intersection as ss;
fn columns() -> [Model; 2] {
    std::array::from_fn(|side| {
        let mut roof = crate::ss_face_membership::tests::surface(side == 0, 0.0625);
        for p in roof.control_points.iter_mut().flatten() {
            p[2] += 2.;
        }
        column(roof)
    })
}
#[test]
fn original_roof_sectors_have_actual_other_body_material_roles() {
    let models = columns();
    let saved = models.clone();
    let budget = limits();
    let bodies = [
        graph_body::qualify(&models[0], 0, 1, &budget).unwrap(),
        graph_body::qualify(&models[1], 0, 1, &budget).unwrap(),
    ];
    let originals =
        ss::FiberSources::new(&models[0].faces[0].surface, &models[1].faces[0].surface).unwrap();
    let intersection = ss::qualify_quadratic_graph_ss(&originals, Default::default()).unwrap();
    let curves = intersection
        .construct_graph_curves_with_dyadic_proposals(
            ss::GraphCurveTolerances {
                world: 1e-7,
                uv: 1e-9,
            },
            Default::default(),
        )
        .unwrap();
    let membership = qualify(
        &curves,
        [(&models[0], 0), (&models[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let closed = partition_closed_branches(&membership, Default::default()).unwrap();
    let sides = qualify_closed_graph_sides(&closed, Default::default()).unwrap();
    let mut material =
        qualify_closed_graph_material(&sides, [&bodies[0], &bodies[1]], Default::default())
            .unwrap();
    assert_eq!(material.faces().len(), 4);
    for face in material.faces() {
        assert_eq!(
            face.inside_other,
            face.sector.inner ^ (face.sector.side == 1)
        );
        assert_eq!(
            face.sector.face,
            closed.regions()[face.sector.side].faces[usize::from(face.sector.inner)]
        );
    }
    material.reverify().unwrap();
    let exact = ClosedGraphSideLimits {
        operations: material.operations(),
    };
    assert!(qualify_closed_graph_material(&sides, [&bodies[0], &bodies[1]], exact).is_ok());
    assert!(
        qualify_closed_graph_material(
            &sides,
            [&bodies[0], &bodies[1]],
            ClosedGraphSideLimits {
                operations: exact.operations - 1
            }
        )
        .is_err()
    );
    assert!(
        qualify_closed_graph_material(&sides, [&bodies[1], &bodies[0]], Default::default())
            .is_err()
    );
    let copied = models[0].clone();
    let copy_body = graph_body::qualify(&copied, 0, 1, &budget).unwrap();
    assert!(
        qualify_closed_graph_material(&sides, [&copy_body, &bodies[1]], Default::default())
            .is_err()
    );
    material.faces[0].inside_other = !material.faces[0].inside_other;
    assert!(material.reverify().is_err());
    assert_eq!(models, saved);
}

#[test]
fn support_sides_do_not_imply_material_with_different_original_floors() {
    let mut models = columns();
    models[1] = crate::graph_body::tests::column_at_floor(models[1].faces[0].surface.clone(), -1.);
    let budget = limits();
    let bodies = [
        graph_body::qualify(&models[0], 0, 1, &budget).unwrap(),
        graph_body::qualify(&models[1], 0, 1, &budget).unwrap(),
    ];
    let originals =
        ss::FiberSources::new(&models[0].faces[0].surface, &models[1].faces[0].surface).unwrap();
    let intersection = ss::qualify_quadratic_graph_ss(&originals, Default::default()).unwrap();
    let curves = intersection
        .construct_graph_curves_with_dyadic_proposals(
            ss::GraphCurveTolerances {
                world: 1e-7,
                uv: 1e-9,
            },
            Default::default(),
        )
        .unwrap();
    let membership = qualify(
        &curves,
        [(&models[0], 0), (&models[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let closed = partition_closed_branches(&membership, Default::default()).unwrap();
    let sides = qualify_closed_graph_sides(&closed, Default::default()).unwrap();
    let error =
        match qualify_closed_graph_material(&sides, [&bodies[0], &bodies[1]], Default::default()) {
            Ok(_) => panic!("different floors acquired common-column authority"),
            Err(error) => error,
        };
    assert!(error.message.contains("common footprint and floor"));
}
