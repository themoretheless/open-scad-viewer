//! Exact sufficient orientation of an original rational Bezier projection.
//! Interior orientation does not establish global injectivity or contacts.
use crate::{Result, check, numeric_err, surface::Surface};
use cad_predicates::{
    AuthoredScalar, Limits, PredicateContext, Sign, SourceArena, ToleranceContext,
};

#[derive(Clone, Debug)]
pub struct Certificate {
    surface: Surface,
    axes: [usize; 2],
    orientation: Sign,
}
impl Certificate {
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn axes(&self) -> [usize; 2] {
        self.axes
    }
    pub fn orientation(&self) -> Sign {
        self.orientation
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub signs: Option<Vec<Vec<Sign>>>,
    pub exact_work: u64,
    pub exact_reason: Option<cad_predicates::Reason>,
    /// Opposite strict signs on v=0 and v=1 imply an interior zero of
    /// the projected Jacobian for every interior u, by continuity.
    /// This is not a certificate of a 3D self-intersection.
    pub opposite_v_boundary_signs: Option<[Sign; 2]>,
    pub reason: &'static str,
}
/// Only single, clamped, nonperiodic Bezier charts are admitted here.
pub fn certify(surface: &Surface, axes: [usize; 2], max_work: u64) -> Result<Report> {
    surface.validate()?;
    check(
        (1..=100_000_000).contains(&max_work),
        "Projection needs 1..100000000 exact work",
    )?;
    check(
        axes[0] < 3 && axes[1] < 3 && axes[0] != axes[1],
        "Projection axes must be distinct XYZ axes",
    )?;
    let clamped = |degree: usize, knots: &[f64], count: usize| {
        (1..=8).contains(&degree)
            && count == degree + 1
            && knots.len() == 2 * (degree + 1)
            && knots[0] < knots[degree + 1]
            && knots[..=degree].iter().all(|k| *k == knots[0])
            && knots[degree + 1..].iter().all(|k| *k == knots[degree + 1])
    };
    check(
        !surface.periodic_u
            && !surface.periodic_v
            && clamped(
                surface.degree_u,
                &surface.knots_u,
                surface.control_points.len(),
            )
            && clamped(
                surface.degree_v,
                &surface.knots_v,
                surface.control_points[0].len(),
            )
            && surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p.len() == 3)
            && surface.weights.iter().flatten().all(|w| *w > 0.),
        "Projection requires a positive-weight clamped 3D Bezier chart of degrees 1..8",
    )?;
    let values = surface
        .control_points
        .iter()
        .enumerate()
        .flat_map(|(u, row)| {
            row.iter()
                .enumerate()
                .flat_map(move |(v, p)| [p[0], p[1], p[2], surface.weights[u][v]])
        })
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let arena = SourceArena::authored("original-projection-jacobian", 1, values)
        .map_err(|_| numeric_err("Projection source admission failed"))?;
    let leaves = surface
        .control_points
        .iter()
        .enumerate()
        .map(|(u, row)| {
            row.iter()
                .enumerate()
                .map(|(v, _)| {
                    std::array::from_fn(|a| arena.leaf((u * row.len() + v) * 4 + a).unwrap())
                })
                .collect()
        })
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut signs = Vec::new();
    let mut work = 0u64;
    let mut exact_reason = None;
    for row in 0..3 * surface.degree_u {
        let remaining = max_work.saturating_sub(work).min(cad_predicates::MAX_WORK);
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: remaining,
                ..Limits::default()
            },
            None,
        );
        let decision =
            cad_predicates::rational_surface_projected_jacobian_row(&mut ctx, &leaves, axes, row)
                .map_err(|_| numeric_err("Projection predicate admission failed"))?;
        work = work
            .checked_add(decision.work_used)
            .ok_or_else(|| numeric_err("Projection work overflow"))?;
        match decision.signs {
            Some(mut rows) => signs.push(rows.remove(0)),
            None => {
                if decision.reason != Some(cad_predicates::Reason::ResourceLimit)
                    || work >= max_work
                {
                    exact_reason = decision.reason;
                    break;
                }
                let mut coefficients = Vec::new();
                for column in 0..3 * surface.degree_v {
                    let remaining = max_work.saturating_sub(work).min(cad_predicates::MAX_WORK);
                    let mut ctx = PredicateContext::new(
                        &arena,
                        &tolerance,
                        Limits {
                            max_work: remaining,
                            ..Limits::default()
                        },
                        None,
                    );
                    let d = cad_predicates::rational_surface_projected_jacobian_coefficient(
                        &mut ctx,
                        &leaves,
                        axes,
                        [row, column],
                    )
                    .map_err(|_| numeric_err("Projection coefficient admission failed"))?;
                    work = work
                        .checked_add(d.work_used)
                        .ok_or_else(|| numeric_err("Projection work overflow"))?;
                    match d.signs {
                        Some(s) => coefficients.push(s[0][0]),
                        None => {
                            exact_reason = d.reason;
                            break;
                        }
                    }
                }
                if coefficients.len() != 3 * surface.degree_v {
                    break;
                }
                signs.push(coefficients);
            }
        }
    }
    let signs = (signs.len() == 3 * surface.degree_u).then_some(signs);
    let orientation = signs.as_ref().and_then(|s| {
        let first = s.iter().flatten().find(|s| **s != Sign::Zero).copied()?;
        s.iter()
            .flatten()
            .all(|s| *s == Sign::Zero || *s == first)
            .then_some(first)
    });
    let opposite_v_boundary_signs = signs.as_ref().and_then(|rows| {
        let edge_sign = |column: usize| {
            let first = rows
                .iter()
                .map(|row| row[column])
                .find(|s| *s != Sign::Zero)?;
            rows.iter()
                .all(|row| row[column] == Sign::Zero || row[column] == first)
                .then_some(first)
        };
        let first = edge_sign(0)?;
        let last = edge_sign(rows[0].len() - 1)?;
        (first != last).then_some([first, last])
    });
    Ok(Report {
        certificate: orientation.map(|orientation| Certificate {
            surface: surface.clone(),
            axes,
            orientation,
        }),
        signs,
        opposite_v_boundary_signs,
        exact_work: work,
        exact_reason: exact_reason.clone(),
        reason: if orientation.is_some() {
            "strict-interior-projection-orientation"
        } else if exact_reason.is_some() {
            "exact-projection-computation-unproven"
        } else {
            "projection-sign-unproven"
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![2., 2., 5., 5.],
            knots_v: vec![-3., -3., 4., 4.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 2.], vec![1., 2.]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn original_chart_certificate_owns_geometry_and_budget_refuses() {
        let mut s = plane();
        let report = certify(&s, [0, 1], 1_000_000).unwrap();
        let certificate = report.certificate.unwrap();
        assert_eq!(certificate.surface(), &s);
        assert_eq!(certificate.axes(), [0, 1]);
        assert_eq!(certificate.orientation(), Sign::Positive);
        s.control_points[1][1][1] = -1.;
        assert_ne!(certificate.surface(), &s);
        assert!(
            certify(&s, [0, 1], 1_000_000)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(certify(&plane(), [0, 1], 1).unwrap().certificate.is_none());
    }
    #[test]
    fn unsupported_axes_and_periodic_charts_are_errors() {
        assert!(certify(&plane(), [1, 1], 1000).is_err());
        let mut p = plane();
        p.periodic_u = true;
        assert!(certify(&p, [0, 1], 1000).is_err());
        assert!(certify(&plane(), [0, 1], 0).is_err());
    }
}
