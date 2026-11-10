use super::*;
use crate::retained_annular_cap::replace;
use nurbs_core::{
    primitives::{circle, line},
    progressive_sweep::{Options, Orientation, Spacing, constant_vector_law},
};

fn limits() -> Limits {
    Limits {
        exact_work: 1_000_000,
        products: 100_000,
        trim_pairs: 10_000,
        trim_cells: 100_000,
        trim_domain_cells: 1_000_000,
        spans: 1024,
        linear_cells: 100_000,
        contacts: crate::face_contacts::Limits {
            pairs: 10_000,
            cells: 100_000,
            domain_cells: 1_000_000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10_000,
        },
    }
}

#[test]
fn smooth_hollow_filled_caps_cover_oblique_rmf_and_curved_frames() {
    let curved = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0.; 3], vec![0.5, 0., 0.], vec![1., 1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    for (kind, axis, path, orientation) in [
        (
            "oblique-rmf",
            [3., 4., 0.],
            line([0.; 3], [3., 4., 0.]).unwrap(),
            Orientation::RotationMinimizing,
        ),
        (
            "quadratic-frenet",
            [1., 0., 0.],
            curved.clone(),
            Orientation::Frenet,
        ),
        (
            "quadratic-rmf",
            [1., 0., 0.],
            curved,
            Orientation::RotationMinimizing,
        ),
    ] {
        let profiles = [
            circle([0.; 3], axis, 0.1).unwrap(),
            circle([0.; 3], axis, 0.05).unwrap().reverse().unwrap(),
        ];
        let scale = if kind == "oblique-rmf" {
            Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![vec![1., 0., 0.], vec![2., 0., 0.]],
                weights: vec![1.; 2],
                periodic: false,
            }
        } else {
            constant_vector_law([1., 0., 0.]).unwrap()
        };
        let twist = constant_vector_law([0.; 3]).unwrap();
        let source = MultiSweep::new(
            &profiles,
            &path,
            &scale,
            &twist,
            Options {
                normal: [0., 0., 1.],
                orientation,
                spacing: Spacing::Parameter,
                initial_sections: 3,
                max_sections: 3,
                max_deviation: 0.01,
            },
        )
        .unwrap();
        let profiles_at = source.sections_at(3).unwrap();
        let sections = (0..3)
            .map(|i| {
                vec![
                    vec![profiles_at[0][i].clone()],
                    vec![profiles_at[1][i].clone()],
                ]
            })
            .collect::<Vec<_>>();
        let original = crate::rational_section_loft(&sections).unwrap();
        let caps = [original.faces.len() - 2, original.faces.len() - 1];
        let first = replace(&original, caps[0], 1_000_000).unwrap();
        let last = replace(
            &first.model,
            first.source_face_map[caps[1]].unwrap(),
            1_000_000,
        )
        .unwrap();
        let groups = [
            first
                .cap_faces
                .iter()
                .map(|&f| last.source_face_map[f].unwrap())
                .collect(),
            last.cap_faces.clone(),
        ];
        for face in 0..caps[0] {
            let mapped = last.source_face_map[first.source_face_map[face].unwrap()].unwrap();
            assert_eq!(
                original.faces[face].surface.control_points,
                last.model.faces[mapped].surface.control_points
            );
            assert_eq!(
                original.faces[face].surface.weights,
                last.model.faces[mapped].surface.weights
            );
        }
        let proof =
            inspect_smooth(&source, &last.model, &groups, &[1, 1], 1e-8, 1e-8, limits()).unwrap();
        assert!(
            proof.source_filling_covered && proof.within_budget,
            "{kind}: {} {:?}",
            proof.reason,
            proof.filled_cap_error_upper
        );
        assert!(
            proof
                .filled_cap_error_upper
                .unwrap()
                .iter()
                .all(|&e| e <= 1e-8)
        );
        assert!(
            proof.endpoint_cells > 0
                && proof.products > 0
                && proof.exact_work <= limits().exact_work
        );
        // A cap bound is not a proof that retained wall interpolation is close
        // to the original path, nor an embedding/volume admission certificate.
        let mut short = limits();
        short.products = proof.products - 1;
        let denied =
            inspect_smooth(&source, &last.model, &groups, &[1, 1], 1e-8, 1e-8, short).unwrap();
        assert!(denied.filled_cap_error_upper.is_none() && !denied.within_budget);
        assert!(denied.products <= short.products);
        let mut omitted = groups.clone();
        omitted[1].pop();
        assert!(
            inspect_smooth(
                &source,
                &last.model,
                &omitted,
                &[1, 1],
                1e-8,
                1e-8,
                limits()
            )
            .is_err()
        );
        let mut changed = last.model.clone();
        let pole = &mut changed.faces[groups[1][0]].surface.control_points[0][0][0];
        *pole = pole.next_up();
        let denied =
            inspect_smooth(&source, &changed, &groups, &[1, 1], 1e-8, 1e-8, limits()).unwrap();
        assert!(denied.filled_cap_error_upper.is_none());
    }
}
