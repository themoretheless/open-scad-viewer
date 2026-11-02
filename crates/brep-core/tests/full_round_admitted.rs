use brep_core::{feature_ops::full_round_exact, Model};
use nurbs_core::curve::Curve;
fn source(weight: f64) -> (Model, [usize; 3]) {
    let curve = |points: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: points.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., weight, 1.],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    let profile = vec![
        line([0., 0.], [4., 0.]),
        curve([[4., 0.], [5., 4.], [4., 8.]]),
        line([4., 8.], [0., 8.]),
        curve([[0., 8.], [-1., 4.], [0., 0.]]),
    ];
    let m = brep_core::prism::extrude(&[profile], 0., 6.).unwrap();
    let side = |left: bool| {
        m.faces
            .iter()
            .position(|f| {
                f.surface.degree_u == 2
                    && f.surface.control_points.iter().flatten().all(|p| {
                        if left {
                            p[0] <= 0.
                        } else {
                            p[0] >= 4.
                        }
                    })
            })
            .unwrap()
    };
    let middle = m
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[1] == 8.)
        })
        .unwrap();
    let faces = [side(true), middle, side(false)];
    (m, faces)
}

#[test]
fn public_exact_entry_returns_the_material_owned_rational_topology() {
    let (m, faces) = source(0.7);
    let before = m.clone();
    let admitted = full_round_exact(&m, faces, Default::default()).unwrap();
    assert_eq!(admitted.source(), &before);
    assert_eq!(admitted.selected_faces(), faces);
    assert_eq!(
        admitted.lineage().unwrap().identities.faces[0],
        m.1.faces[0]
    );
    let mut forged = admitted.lineage().unwrap();
    forged.faces[0].id = m.1.faces[1];
    assert!(forged.reverify(&admitted).is_err());
    assert!(std::ptr::eq(
        admitted.model(),
        admitted.material().result().model()
    ));
    assert_eq!(admitted.model().bodies.len(), 1);
    assert!(admitted
        .material()
        .certificate()
        .relation()
        .result_is_subset());
    admitted.reverify().unwrap();
    let lineage = admitted.lineage().unwrap();
    lineage.change_set.validate().unwrap();
    assert_eq!(lineage.faces.len(), admitted.model().faces.len());
    lineage.reverify(&admitted).unwrap();
    assert_eq!(m, before);
}
#[test]
fn missing_material_budget_and_wrong_selection_do_not_return_a_legacy_fallback() {
    use brep_core::profile_certificate::original_restriction::wire::material::Limits;
    let (m, faces) = source(1.);
    let before = m.clone();
    let limits = Limits {
        max_pair_projections: 0,
        ..Default::default()
    };
    assert!(full_round_exact(&m, faces, limits).is_err());
    assert!(full_round_exact(&m, [0, 2, 1], Default::default()).is_err());
    assert_eq!(m, before);
}

#[test]
fn exact_entry_preserves_a_through_hole_in_the_admitted_material() {
    // Normalized recognition is used only to author a fresh test input.
    let (base, _) = source(0.7);
    let mut loops = brep_core::prism::recognize(&base).unwrap().unwrap().loops;
    let p = [[1., 1.], [1., 2.], [2., 2.], [2., 1.], [1., 1.]];
    loops.push(
        p.windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect(),
    );
    let model = brep_core::prism::extrude(&loops, 0., 6.).unwrap();
    let before = model.clone();
    let admitted = full_round_exact(&model, [3, 2, 1], Default::default()).unwrap();
    assert_eq!(
        admitted.material().result().nesting().containment().len(),
        2
    );
    assert!(admitted
        .material()
        .certificate()
        .relation()
        .result_is_subset());
    assert_eq!(
        admitted
            .model()
            .faces
            .iter()
            .filter(|f| !f.holes.is_empty())
            .count(),
        2
    );
    assert!(admitted.model().bodies[0].inner_shells.is_empty());
    admitted.reverify().unwrap();
    let lineage = admitted.lineage().unwrap();
    lineage.change_set.validate().unwrap();
    assert_eq!(lineage.faces.len(), admitted.model().faces.len());
    lineage.reverify(&admitted).unwrap();
    assert_eq!(model, before);
}

#[test]
fn authored_source_identity_cannot_alias_a_new_output_child() {
    let (mut source, faces) = source(0.7);
    let initial = full_round_exact(&source, faces, Default::default()).unwrap();
    let initial = initial.lineage().unwrap();
    let old = source.1.faces[0];
    let chosen = initial.identities.faces[1];
    source.1.faces[0] = chosen;
    let kind = source.1.change_set.nodes.remove(&old).unwrap();
    source.1.change_set.nodes.insert(chosen, kind);
    for change in &mut source.1.change_set.changes {
        for id in change.parents.iter_mut().chain(&mut change.children) {
            if *id == old {
                *id = chosen;
            }
        }
    }
    for change in &mut source.1.lineage {
        for id in change.parents.iter_mut().chain(&mut change.children) {
            if *id == old {
                *id = chosen;
            }
        }
    }
    source.validate().unwrap();
    let result = full_round_exact(&source, faces, Default::default()).unwrap();
    let lineage = result.lineage().unwrap();
    assert_eq!(lineage.identities.faces[0], chosen);
    assert_ne!(lineage.identities.faces[1], chosen);
    lineage.reverify(&result).unwrap();
}
