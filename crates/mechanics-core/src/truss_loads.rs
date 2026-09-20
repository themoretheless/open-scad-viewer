//! Resultant-preserving nodal loads, not a stiffness or material model.
use crate::{Error, Result, truss::MAX_NODES};
use nalgebra::{Matrix3, SymmetricEigen, Vector3};

/// Moment is about the explicitly supplied origin, in the same global axes.
#[derive(Clone, Debug)]
pub struct Wrench {
    pub origin_mm: [f64; 3],
    pub force_n: [f64; 3],
    pub moment_n_mm: [f64; 3],
}

fn invalid(message: &str) -> Error {
    Error::new("TRUSS_LOAD_INVALID_INPUT", message)
}
fn numeric() -> Error {
    Error::new(
        "TRUSS_LOAD_NUMERIC_RANGE",
        "Load assembly exceeds finite numeric range",
    )
}
fn unrealizable() -> Error {
    Error::new(
        "TRUSS_LOAD_UNREALIZABLE",
        "Selected nodes cannot stably represent the requested moment",
    )
}
fn norm(v: &Vector3<f64>) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn finite(v: &Vector3<f64>) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Equal force shares plus the minimum-squared-norm force couple on the selected
/// points. No point selection, pressure area, fallback support or lever heuristic.
/// Rank deficiency is allowed only for representable loads, unlike a structural
/// stiffness solve: a two-node pair can carry a perpendicular couple, not torsion
/// about the line between those nodes.
pub fn distribute(points_mm: &[[f64; 3]], wrench: &Wrench) -> Result<Vec<[f64; 3]>> {
    if points_mm.is_empty() || points_mm.len() > MAX_NODES {
        return Err(invalid("Select between 1 and 125 load nodes"));
    }
    if points_mm
        .iter()
        .flatten()
        .chain(wrench.origin_mm.iter())
        .chain(wrench.force_n.iter())
        .chain(wrench.moment_n_mm.iter())
        .any(|v| !v.is_finite())
    {
        return Err(invalid(
            "Load coordinates, forces and moments must be finite",
        ));
    }
    let origin = Vector3::from(wrench.origin_mm);
    let force = Vector3::from(wrench.force_n);
    let moment = Vector3::from(wrench.moment_n_mm);
    let reference = Vector3::from(points_mm[0]);
    let n = points_mm.len() as f64;
    // Local offsets avoid summing large world coordinates to find the centroid.
    let offsets: Vec<_> = points_mm
        .iter()
        .map(|p| Vector3::from(*p) - reference)
        .collect();
    let centroid = offsets.iter().fold(Vector3::zeros(), |sum, p| sum + p / n);
    let centered: Vec<_> = offsets.iter().map(|p| p - centroid).collect();
    let radius = centered.iter().map(norm).fold(0., f64::max);
    let center_from_origin = (reference - origin) + centroid;
    let couple = moment - center_from_origin.cross(&force);
    let couple_norm = norm(&couple);
    if !finite(&centroid)
        || !finite(&center_from_origin)
        || !finite(&couple)
        || !radius.is_finite()
        || !couple_norm.is_finite()
        || centered.iter().any(|p| !finite(p))
    {
        return Err(numeric());
    }

    let mut forces = vec![force / n; points_mm.len()];
    if couple_norm > 0. {
        if radius == 0. {
            return Err(unrealizable());
        }
        let normalized: Vec<_> = centered.iter().map(|p| p / radius).collect();
        // Sum |r|^2 I - r r^T, with direct diagonal terms to avoid subtraction
        // cancellation. Scaling coordinates makes rank admission unit-invariant.
        let mut inertia = Matrix3::<f64>::zeros();
        for p in &normalized {
            for i in 0..3 {
                inertia[(i, i)] += p[(i + 1) % 3].powi(2) + p[(i + 2) % 3].powi(2);
                for j in 0..i {
                    let value = p[i] * p[j];
                    inertia[(i, j)] -= value;
                    inertia[(j, i)] -= value;
                }
            }
        }
        let eigen = SymmetricEigen::try_new(inertia, f64::EPSILON, 64).ok_or_else(numeric)?;
        let largest = eigen.eigenvalues.iter().copied().fold(0., f64::max);
        let mut alpha = Vector3::zeros();
        for i in 0..3 {
            let value = eigen.eigenvalues[i];
            if !value.is_finite() || value < -largest * 1e-12 {
                return Err(numeric());
            }
            if value > largest * 1e-12 {
                let axis = eigen.eigenvectors.column(i);
                alpha += axis * (axis.dot(&couple) / value);
            }
        }
        if !finite(&alpha) {
            return Err(numeric());
        }
        // Never silently discard an unsupported torque component.
        let unresolved = couple - inertia * alpha;
        let unresolved_norm = norm(&unresolved);
        if !finite(&unresolved) || !unresolved_norm.is_finite() {
            return Err(numeric());
        }
        if unresolved_norm / couple_norm > 1e-10 {
            return Err(unrealizable());
        }
        for (f, p) in forces.iter_mut().zip(&normalized) {
            *f += alpha.cross(p) / radius;
        }
    }

    let mut actual_force = Vector3::zeros();
    let mut actual_moment = Vector3::zeros();
    let mut force_scale = force.map(f64::abs);
    let mut moment_scale = moment.map(f64::abs);
    for (point, f) in points_mm.iter().zip(&forces) {
        if !finite(f) {
            return Err(numeric());
        }
        let torque = (Vector3::from(*point) - origin).cross(f);
        actual_force += f;
        actual_moment += torque;
        force_scale += f.map(f64::abs);
        moment_scale += torque.map(f64::abs);
    }
    for (actual, expected, scale) in [
        (actual_force, force, force_scale),
        (actual_moment, moment, moment_scale),
    ] {
        if !finite(&actual) || !finite(&scale) {
            return Err(numeric());
        }
        for i in 0..3 {
            let error = (actual[i] - expected[i]).abs();
            if !error.is_finite()
                || (scale[i] == 0. && error != 0.)
                || (scale[i] > 0. && error / scale[i] > 1e-9)
            {
                return Err(Error::new(
                    "TRUSS_LOAD_RESIDUAL",
                    "Assembled nodal loads do not preserve the requested resultants",
                ));
            }
        }
    }
    Ok(forces.into_iter().map(|v| [v.x, v.y, v.z]).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wrench(force_n: [f64; 3], moment_n_mm: [f64; 3]) -> Wrench {
        Wrench {
            origin_mm: [0.; 3],
            force_n,
            moment_n_mm,
        }
    }
    fn check(points: &[[f64; 3]], load: &Wrench, forces: &[[f64; 3]]) {
        assert_eq!(points.len(), forces.len());
        let mut f = [0.; 3];
        let mut m = [0.; 3];
        for (p, g) in points.iter().zip(forces) {
            let r: [f64; 3] = std::array::from_fn(|i| p[i] - load.origin_mm[i]);
            for i in 0..3 {
                f[i] += g[i];
            }
            m[0] += r[1] * g[2] - r[2] * g[1];
            m[1] += r[2] * g[0] - r[0] * g[2];
            m[2] += r[0] * g[1] - r[1] * g[0];
        }
        for i in 0..3 {
            assert!((f[i] - load.force_n[i]).abs() < 1e-8, "force {f:?}");
            assert!((m[i] - load.moment_n_mm[i]).abs() < 1e-8, "moment {m:?}");
        }
    }
    #[test]
    fn cube_face_preserves_each_legacy_broken_moment_axis() {
        let points = [
            [0., 0., 10.],
            [10., 0., 10.],
            [10., 10., 10.],
            [0., 10., 10.],
        ];
        for moment in [[100., 0., 0.], [0., 100., 0.], [0., 0., 100.]] {
            let load = wrench([0.; 3], moment);
            check(&points, &load, &distribute(&points, &load).unwrap());
        }
    }
    #[test]
    fn two_node_couple_is_analytical_and_axial_torsion_is_refused() {
        let points = [[-5., 0., 0.], [5., 0., 0.]];
        let load = wrench([0.; 3], [0., 0., 100.]);
        assert_eq!(
            distribute(&points, &load).unwrap(),
            vec![[0., -10., 0.], [0., 10., 0.]]
        );
        assert_eq!(
            distribute(&points, &wrench([0.; 3], [100., 0., 0.]))
                .unwrap_err()
                .code,
            "TRUSS_LOAD_UNREALIZABLE"
        );
    }
    #[test]
    fn single_node_cannot_turn_a_moment_into_a_force() {
        let points = [[10., 0., 10.]];
        assert_eq!(
            distribute(&points, &wrench([0.; 3], [0., 0., 100.]))
                .unwrap_err()
                .code,
            "TRUSS_LOAD_UNREALIZABLE"
        );
        let load = wrench([0., 0., 40.], [0., -400., 0.]);
        assert_eq!(distribute(&points, &load).unwrap(), vec![[0., 0., 40.]]);
    }
    #[test]
    fn combined_force_and_moment_are_about_the_declared_origin() {
        let points = [[2., 3., 4.], [12., 3., 4.], [2., 13., 4.], [12., 13., 4.]];
        let load = Wrench {
            origin_mm: [-7., 1., 2.],
            force_n: [3., -5., 11.],
            moment_n_mm: [100., 200., -300.],
        };
        check(&points, &load, &distribute(&points, &load).unwrap());
        let translated: Vec<_> = points
            .iter()
            .map(|p| std::array::from_fn(|i| p[i] + [1e9, -1e9, 1e9][i]))
            .collect();
        let moved = Wrench {
            origin_mm: std::array::from_fn(|i| load.origin_mm[i] + [1e9, -1e9, 1e9][i]),
            ..load.clone()
        };
        assert_eq!(
            distribute(&points, &load).unwrap(),
            distribute(&translated, &moved).unwrap()
        );
    }
    #[test]
    fn admits_coincident_force_shares_but_not_a_couple() {
        let points = [[0.; 3]; 125];
        assert_eq!(
            distribute(&points, &wrench([125., 0., 0.], [0.; 3])).unwrap(),
            vec![[1., 0., 0.]; 125]
        );
        assert_eq!(
            distribute(&points, &wrench([0.; 3], [0., 1., 0.]))
                .unwrap_err()
                .code,
            "TRUSS_LOAD_UNREALIZABLE"
        );
    }
    #[test]
    fn rotations_and_uniform_length_scales_preserve_force_distribution() {
        let step = Matrix3::new(0.36, -0.48, 0.8, 0.8, 0.6, 0., -0.48, 0.64, 0.6);
        let mut rotation = Matrix3::identity();
        for _ in 0..10 {
            for points in [
                vec![[-5., 0., 0.], [5., 0., 0.]],
                vec![[-5., -5., 0.], [5., -5., 0.], [5., 5., 0.], [-5., 5., 0.]],
            ] {
                let load = wrench([3., -5., 11.], [0., 20., -100.]);
                let expected = distribute(&points, &load).unwrap();
                for scale in [1e-250, 1e-100, 1e-10, 1., 1e10, 1e100, 1e250] {
                    let rotated: Vec<[f64; 3]> = points
                        .iter()
                        .map(|p| (rotation * Vector3::from(*p) * scale).into())
                        .collect();
                    let transformed = wrench(
                        (rotation * Vector3::from(load.force_n)).into(),
                        (rotation * Vector3::from(load.moment_n_mm) * scale).into(),
                    );
                    let actual = distribute(&rotated, &transformed).unwrap();
                    for (a, e) in actual.iter().zip(&expected) {
                        let target = rotation * Vector3::from(*e);
                        assert!(
                            norm(&(Vector3::from(*a) - target)) < 1e-8,
                            "scale {scale}: {a:?} vs {target:?}"
                        );
                    }
                    let axial_moment: [f64; 3] =
                        (rotation * Vector3::new(100. * scale, 0., 0.)).into();
                    if points.len() == 2 {
                        assert_eq!(
                            distribute(&rotated, &wrench([0.; 3], axial_moment))
                                .unwrap_err()
                                .code,
                            "TRUSS_LOAD_UNREALIZABLE"
                        );
                    }
                }
            }
            rotation = step * rotation;
        }
    }
    #[test]
    fn input_order_does_not_change_the_resulting_forces() {
        let points = [[1., 2., 3.], [7., 0., 9.], [-4., 6., 1.], [2., -3., 5.]];
        let load = wrench([3., -5., 11.], [100., 20., -100.]);
        let expected = distribute(&points, &load).unwrap();
        let reversed: Vec<_> = points.iter().rev().copied().collect();
        let actual = distribute(&reversed, &load).unwrap();
        for (a, e) in actual.iter().rev().zip(&expected) {
            assert!(norm(&(Vector3::from(*a) - Vector3::from(*e))) < 1e-10);
        }
        check(&points, &load, &expected);
    }
    #[test]
    fn assembled_loads_and_solver_reactions_balance_on_the_node_limit() {
        use crate::truss::{Member, Model, solve};
        let selected: Vec<[f64; 3]> = (0..122).map(|i| [1., 2., 10. + i as f64 / 10.]).collect();
        let load = wrench([3., -5., 11.], [100., 200., -11.]);
        let applied = distribute(&selected, &load).unwrap();
        check(&selected, &load, &applied);
        let mut model = Model {
            nodes_mm: vec![[10., 0., 0.], [0., 10., 0.], [0., 0., 0.]],
            members: Vec::new(),
            restrained: vec![[true; 3]; 3],
            forces_n: vec![[0.; 3]; 3],
        };
        for (i, (point, force)) in selected.iter().zip(applied).enumerate() {
            model.nodes_mm.push(*point);
            model.restrained.push([false; 3]);
            model.forces_n.push(force);
            for anchor in 0..3 {
                model.members.push(Member {
                    nodes: [anchor, i + 3],
                    young_mpa: 2000.,
                    area_mm2: 2.,
                });
            }
        }
        let response = solve(&model).unwrap();
        assert_eq!(response.free_dofs, 366);
        let reaction_load = wrench(load.force_n.map(|v| -v), load.moment_n_mm.map(|v| -v));
        check(&model.nodes_mm, &reaction_load, &response.reactions_n);
    }
    #[test]
    fn rejects_invalid_budgets_and_numeric_ranges() {
        let load = wrench([1., 0., 0.], [0.; 3]);
        for points in [vec![], vec![[0.; 3]; 126], vec![[f64::NAN, 0., 0.]]] {
            assert_eq!(
                distribute(&points, &load).unwrap_err().code,
                "TRUSS_LOAD_INVALID_INPUT"
            );
        }
        assert_eq!(
            distribute(&[[f64::MAX, 0., 0.], [-f64::MAX, 0., 0.]], &load)
                .unwrap_err()
                .code,
            "TRUSS_LOAD_NUMERIC_RANGE"
        );
        let invalid = Wrench {
            force_n: [f64::INFINITY, 0., 0.],
            ..load
        };
        assert_eq!(
            distribute(&[[0.; 3]], &invalid).unwrap_err().code,
            "TRUSS_LOAD_INVALID_INPUT"
        );
    }
}
