use brep_core::{
    boundary_embedding, face_contacts, feature_ops::full_round_curved::build, volume_validity,
};
use nurbs_core::curve::Curve;

#[test]
fn curved_full_round_requires_whole_embedding_roles_and_outward_volume() {
    for (weight, right, left) in [
        (1., 5., -1.),
        (0.7, 5., -1.),
        (1., 3., 1.),
        (0.7, 3., 1.),
        (1., 3., -1.),
        (0.7, 5., 1.),
    ] {
        let c = |p: [[f64; 2]; 3]| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: p.map(|p| p.to_vec()).to_vec(),
            weights: vec![1., weight, 1.],
            periodic: false,
        };
        let line =
            |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
        let source = brep_core::prism::extrude(
            &[vec![
                line([0., 0.], [4., 0.]),
                c([[4., 0.], [right, 4.], [4., 8.]]),
                line([4., 8.], [0., 8.]),
                c([[0., 8.], [left, 4.], [0., 0.]]),
            ]],
            0.,
            6.,
        )
        .unwrap();
        // Native constructor stores side faces in the original profile order.
        let r = build(&source, [3, 2, 1]).unwrap();
        let exact = r.qualify_exact_solid(
            brep_core::profile_certificate::original_restriction::wire::jordan::Limits::default(),
            brep_core::profile_certificate::original_restriction::wire::nesting::Limits::default(),
        ).unwrap();
        assert_eq!(exact.model().bodies.len(), 1);
        exact.reverify().unwrap();
        let q = &r.construction().tangency;
        assert_eq!(q.all_edges().len(), q.envelopes().len() + 1);
        assert_eq!(q.seams().len(), q.all_edges().len());
        let first = if right < 4. { 2 } else { 1 };
        let second = if left > 0. { 2 } else { 1 };
        assert_eq!(q.envelopes().len(), first + second);
        assert_eq!(q.all_edges().len(), first + second + 1);
        assert_eq!(
            q.edges(),
            [
                q.all_edges()[0],
                q.all_edges()[first],
                q.all_edges()[first + second]
            ]
        );
        let cover = r
            .qualify_source_arc_cover(nurbs_core::trim_domain::curve_cover::Limits {
                cells: 100_000,
                classification_work: 1_000_000,
                witness_work: 1_000_000,
            })
            .unwrap();
        assert_eq!(cover.source(), &source);
        assert_eq!(cover.result(), r.model());
        assert_eq!(cover.arcs().len(), first + second);
        assert!(
            cover
                .arcs()
                .iter()
                .all(|(_, c)| c.tolerance() == source.tolerance_mm)
        );
        eprintln!(
            "source-cover weight={weight} right={right} left={left}: {:?}",
            cover.work()
        );
        let limits = volume_validity::Limits {
            boundary: boundary_embedding::Limits {
                exact_work: 1_000_000,
                trim_pairs: 10_000,
                trim_cells: 100_000,
                trim_domain_cells: 1_000_000,
                spans: 1024,
                contacts: face_contacts::Limits {
                    pairs: 10_000,
                    cells: 100_000,
                    domain_cells: 1_000_000,
                    cells_per_pair: 1000,
                    domain_cells_per_pair: 10_000,
                },
            },
            nesting_pairs: 100,
            nesting_cells: 100_000,
            nesting_domain_cells: 1_000_000,
            orientation_cells: 100_000,
            orientation_domain_cells: 1_000_000,
            orientation_spans: 100,
        };
        let material = r
            .construction()
            .tangency
            .clone()
            .qualify_volume(1e-8, limits, 100_000)
            .unwrap();
        assert_eq!(material.model(), r.model());
        assert!(material.report().proven);
        assert!(material.report().boundary.intersections.absence_proven);
        assert_eq!(
            material.report().boundary.intersections.pairs.next_pair,
            None
        );
        assert_eq!(
            material.report().nesting.as_ref().unwrap().roles_consistent,
            Some(true)
        );
        assert!(
            material
                .report()
                .orientations
                .iter()
                .all(|o| o.outward == Some(o.expected_outward))
        );
    }
}
