//! Local nonnegative control corrections with whole-support trim ownership.
//! Candidate construction only: no inherited G2, material or solid receipt.
use crate::{Model, face_domain::FaceDomain};
use nurbs_core::{Error, Result, surface::Surface, trim_domain::Location};
#[derive(Clone, Copy, Debug)]
pub struct InteriorLift {
    pub pole: [usize; 2],
    pub displacement: [f64; 3],
}
pub struct InteriorLiftProposal {
    pub surface: Surface,
    pub support_rectangles: Vec<[[f64; 2]; 2]>,
    pub domain_cells: usize,
}
fn check(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new("BREP_INTERIOR_LIFT_UNPROVEN", message))
    }
}
/// A B-spline pole contributes only on its complete tensor basis support.
/// Require that whole CLOSED rectangle to be strictly Inside the actual
/// trimmed face. Together with C2 internal knots this gives a compact C2
/// correction, identically zero at all trimmed boundary curves. Positive
/// stored coordinate increments and positive rational weights cannot lower
/// any coordinate. This does NOT bound the upper displacement or certify
/// injectivity, thickness, G2 receipts, or material containment of the result.
/// The immutable source is never changed; refusals yield no partial proposal.
pub fn propose_interior_lift(
    model: &Model,
    face: usize,
    changes: &[InteriorLift],
    tolerance_uv: f64,
    max_domain_cells: usize,
) -> Result<InteriorLiftProposal> {
    check(
        !changes.is_empty() && changes.len() <= 64 && max_domain_cells <= 1_000_000,
        "Local lift requires 1..64 changes and bounded shared UV work",
    )?;
    let domain = FaceDomain::new(model, face, tolerance_uv)?;
    let source = &model.faces[face].surface;
    check(
        !source.periodic_u
            && !source.periodic_v
            && source.degree_u >= 3
            && source.degree_v >= 3
            && source.degree_u <= 16
            && source.degree_v <= 16
            && source
                .control_points
                .len()
                .saturating_mul(source.control_points[0].len())
                <= 4096
            && source.control_points[0][0].len() == 3
            && source.weights.iter().flatten().all(|w| *w > 0.),
        "Local lift requires a bounded nonperiodic 3D C2 support",
    )?;
    let counts = [source.control_points.len(), source.control_points[0].len()];
    let degrees = [source.degree_u, source.degree_v];
    let knots = [&source.knots_u, &source.knots_v];
    for axis in 0..2 {
        let low = knots[axis][degrees[axis]];
        let high = knots[axis][counts[axis]];
        let mut i = 0;
        while i < knots[axis].len() {
            let value = knots[axis][i];
            let mut j = i + 1;
            while j < knots[axis].len() && knots[axis][j] == value {
                j += 1;
            }
            check(
                value <= low || value >= high || j - i <= degrees[axis] - 2,
                "Local lift refuses knots with less than C2 continuity",
            )?;
            i = j;
        }
    }
    let mut rectangles = Vec::new();
    let mut used = 0;
    for change in changes {
        check(
            (0..2).all(|k| change.pole[k] < counts[k])
                && change
                    .displacement
                    .iter()
                    .all(|v| v.is_finite() && *v >= 0.)
                && change.displacement.iter().any(|v| *v > 0.),
            "Invalid local lift pole or nonnegative displacement",
        )?;
        let rectangle = std::array::from_fn(|axis| {
            [
                knots[axis][change.pole[axis]],
                knots[axis][change.pole[axis] + degrees[axis] + 1],
            ]
        });
        check(
            (0..2).all(|k| {
                rectangle[k][0] > knots[k][degrees[k]] && rectangle[k][1] < knots[k][counts[k]]
            }),
            "Local lift support reaches the natural boundary",
        )?;
        let membership = domain.classify(rectangle, max_domain_cells.saturating_sub(used))?;
        used += membership.cells;
        check(
            membership.location == Location::Inside,
            "Whole local lift basis support is not certified Inside the trim",
        )?;
        rectangles.push(rectangle);
    }
    let mut surface = source.clone();
    for change in changes {
        let p = &mut surface.control_points[change.pole[0]][change.pole[1]];
        for k in 0..3 {
            let updated = p[k] + change.displacement[k];
            check(
                updated.is_finite() && updated >= p[k],
                "Local lift exceeded finite control range",
            )?;
            p[k] = updated;
        }
    }
    surface.validate()?;
    Ok(InteriorLiftProposal {
        surface,
        support_rectangles: rectangles,
        domain_cells: used,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Model {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let old = m.faces[0].surface.clone();
        let mut knots = vec![0.; 4];
        knots.extend((1..6).map(|i| i as f64 / 6.));
        knots.extend([1.; 4]);
        let g = (0..9)
            .map(|i| knots[i + 1..i + 4].iter().sum::<f64>() / 3.)
            .collect::<Vec<_>>();
        m.faces[0].surface = Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: knots.clone(),
            knots_v: knots,
            control_points: g
                .iter()
                .map(|&u| {
                    g.iter()
                        .map(|&v| old.evaluate(u, v).unwrap().point.to_vec())
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 9]; 9],
            periodic_u: false,
            periodic_v: false,
        };
        m
    }
    #[test]
    fn interior_correction_is_nonnegative_and_boundary_jets_are_unchanged() {
        let m = fixture();
        let old = m.faces[0].surface.clone();
        let r = propose_interior_lift(
            &m,
            0,
            &[InteriorLift {
                pole: [4, 4],
                displacement: [0.01; 3],
            }],
            1e-8,
            10000,
        )
        .unwrap();
        assert_eq!(m.faces[0].surface, old);
        for i in 0..=20 {
            for j in 0..=20 {
                let (u, v) = (i as f64 / 20., j as f64 / 20.);
                let a = old.evaluate(u, v).unwrap();
                let b = r.surface.evaluate(u, v).unwrap();
                assert!((0..3).all(|k| b.point[k] >= a.point[k] - 1e-15));
                if i == 0 || j == 0 || i == 20 || j == 20 {
                    assert_eq!(a.point, b.point);
                    assert!(a.first_derivatives().is_some());
                    assert!(a.second_derivatives().is_some());
                    assert_eq!(a.first_derivatives(), b.first_derivatives());
                    assert_eq!(a.second_derivatives(), b.second_derivatives());
                }
            }
        }
        assert!(
            r.surface.evaluate(0.5, 0.5).unwrap().point[0]
                > old.evaluate(0.5, 0.5).unwrap().point[0]
        );
    }
    #[test]
    fn touching_boundary_zero_work_and_negative_displacement_refuse() {
        let m = fixture();
        let old = m.faces[0].surface.clone();
        for (pole, displacement, budget) in [
            ([0, 4], [0.01; 3], 10000),
            ([4, 4], [0.01; 3], 0),
            ([4, 4], [-0.01; 3], 10000),
        ] {
            assert!(
                propose_interior_lift(&m, 0, &[InteriorLift { pole, displacement }], 1e-8, budget)
                    .is_err()
            );
            assert_eq!(m.faces[0].surface, old);
        }
    }
    #[test]
    fn an_inside_midpoint_does_not_authorize_a_support_covering_a_hole() {
        let mut m = fixture();
        let vertices = [[0.22, 0.22], [0.22, 0.28], [0.28, 0.28], [0.28, 0.22]];
        let coedges = (0..4)
            .map(|i| {
                crate::Coedge::new(
                    0,
                    false,
                    nurbs_core::curve::Curve::from_polyline(vec![
                        vertices[i].to_vec(),
                        vertices[(i + 1) % 4].to_vec(),
                    ])
                    .unwrap(),
                )
            })
            .collect();
        let hole = m.loops.len();
        m.loops.push(crate::Loop { coedges });
        m.faces[0].holes.push(hole);
        let domain = FaceDomain::new(&m, 0, 1e-8).unwrap();
        assert_eq!(
            domain.classify([[0.5, 0.5]; 2], 10000).unwrap().location,
            Location::Inside
        );
        assert!(
            propose_interior_lift(
                &m,
                0,
                &[InteriorLift {
                    pole: [4, 4],
                    displacement: [0.01; 3]
                }],
                1e-8,
                10000
            )
            .is_err()
        );
    }
    #[test]
    fn insufficient_knot_continuity_refuses_even_for_an_interior_pole() {
        let mut m = fixture();
        m.faces[0].surface.knots_u[5] = m.faces[0].surface.knots_u[4];
        m.faces[0].surface.validate().unwrap();
        assert!(
            propose_interior_lift(
                &m,
                0,
                &[InteriorLift {
                    pole: [4, 4],
                    displacement: [0.01; 3]
                }],
                1e-8,
                10000
            )
            .is_err()
        );
    }
    #[test]
    fn positive_rational_weights_are_retained_and_scale_the_actual_lift() {
        let mut m = fixture();
        m.faces[0].surface.weights[4][4] = 2.;
        let old = &m.faces[0].surface;
        let b = nurbs_core::curve::basis(3, &old.knots_u, 9, 0.5, false)
            .unwrap()
            .basis[4];
        let expected = 0.01 * (2. * b * b) / (1. + b * b);
        let result = propose_interior_lift(
            &m,
            0,
            &[InteriorLift {
                pole: [4, 4],
                displacement: [0.01; 3],
            }],
            1e-8,
            10000,
        )
        .unwrap();
        assert_eq!(result.surface.weights, old.weights);
        let a = old.evaluate(0.5, 0.5).unwrap().point;
        let q = result.surface.evaluate(0.5, 0.5).unwrap().point;
        for k in 0..3 {
            assert!((q[k] - a[k] - expected).abs() < 1e-14);
        }
    }
}
