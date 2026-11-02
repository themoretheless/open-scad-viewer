use brep_core::{feature_ops::full_round_curved::build, Model};
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
fn exact_new_full_round_material_uses_canonical_source_and_original_rational_data() {
    for weight in [1., 0.7] {
        let (m, faces) = source(weight);
        let before = m.clone();
        let r = build(&m, faces).unwrap();
        let radius = r.exact_radius();
        assert_eq!(radius.wire().pieces(), r.exact_profile()[0].pieces());
        assert!(r
            .exact_smoothness()
            .junctions()
            .iter()
            .all(|j| j.sine_squared == ["0".to_string(), "1".to_string()]));
        assert_eq!(r.exact_profile()[0].line_contacts().len(), 1);
        radius.reverify().unwrap();
        assert!(radius
            .wire()
            .qualify_edited_radius(
                radius.center(),
                ["1".into(), "100".into()],
                ["0".into(), "1".into()]
            )
            .is_err());
        let relation = r.inspect_exact_source_material(Default::default()).unwrap();
        assert_eq!(relation.source().boundary().source(), &before);
        assert_eq!(relation.result().model().bodies.len(), 1);
        assert!(relation.relation().removed().next().is_some());
        eprintln!(
            "weight={weight} partitions={} added={} removed={} rays={}",
            relation.relation().partitions().len(),
            relation.relation().added().count(),
            relation.relation().removed().count(),
            relation.relation().ray_cells()
        );
        if let Some(witness) = relation.relation().added().next() {
            assert_eq!([witness.source_winding, witness.result_winding], [0, 1]);
            eprintln!(
                "added witness: selected={} parameter={:?} side={} point={:?}",
                witness.selected, witness.parameter, witness.side, witness.point
            );
            assert!(relation.clone().require_no_addition().is_err());
        } else {
            relation
                .clone()
                .require_no_addition()
                .unwrap()
                .reverify()
                .unwrap();
        }
        assert!(relation.relation().result_is_subset());
        if weight == 1. {
            use brep_core::profile_certificate::original_restriction::wire::{
                self, jordan, material, nesting,
            };
            let current = &r.exact_profile()[0];
            let old = wire::sew(
                current.sources().to_vec(),
                current.editable().to_vec(),
                current.tolerance(),
            )
            .unwrap();
            let old_region = nesting::qualify(
                &jordan::qualify(&[old], Default::default()).unwrap(),
                Default::default(),
            )
            .unwrap();
            let old_relation = material::inspect(
                relation.source().solid().nesting(),
                &old_region,
                Default::default(),
            )
            .unwrap();
            eprintln!(
                "unrepaired endpoint-only baseline added={}",
                old_relation.added().count()
            );
            assert!(!old_relation.result_is_subset());
            assert!(old_relation.require_no_addition().is_err());
        }
        relation.reverify().unwrap();
        assert_eq!(m, before);
    }
}
