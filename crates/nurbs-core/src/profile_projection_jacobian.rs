//! Exact orientation of the proposed separable profile, not its source weights.
//! Orientation alone does not prove boundary simplicity or global injectivity.
use crate::{Result, check, numeric_err, surface_weight_profile::WeightProfile};
use cad_predicates::{
    AuthoredScalar, Limits, PredicateContext, Sign, SourceArena, ToleranceContext,
};

pub struct Certificate<'p, 's> {
    profile: &'p WeightProfile<'s>,
    axes: [usize; 2],
    orientation: Sign,
}
impl<'p, 's> Certificate<'p, 's> {
    pub fn profile(&self) -> &'p WeightProfile<'s> {
        self.profile
    }
    pub fn axes(&self) -> [usize; 2] {
        self.axes
    }
    pub fn orientation(&self) -> Sign {
        self.orientation
    }
}
pub struct Report<'p, 's> {
    pub certificate: Option<Certificate<'p, 's>>,
    pub signs: Option<Vec<Vec<Sign>>>,
    pub exact_work: u64,
    pub exact_reason: Option<cad_predicates::Reason>,
    /// First coefficient not computed; no partial sign grid is admitted.
    pub pending_coefficient: Option<[usize; 2]>,
    pub reason: &'static str,
}
pub fn certify<'p, 's>(
    profile: &'p WeightProfile<'s>,
    axes: [usize; 2],
    max_work: u64,
) -> Result<Report<'p, 's>> {
    let surface = profile.source();
    surface.validate()?;
    check(
        (1..=100_000_000).contains(&max_work),
        "Profile projection needs 1..100000000 exact work",
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
    let (rows, columns, pivot) = profile.factors();
    check(
        !surface.periodic_u
            && !surface.periodic_v
            && clamped(surface.degree_u, &surface.knots_u, rows.len())
            && clamped(surface.degree_v, &surface.knots_v, columns.len())
            && surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p.len() == 3)
            && pivot.is_finite()
            && pivot > 0.
            && rows.iter().chain(columns).all(|w| w.is_finite() && *w > 0.),
        "Profile projection requires a positive clamped 3D Bezier chart of degrees 1..8",
    )?;
    let count = rows.len() * columns.len() * 3;
    let values = surface
        .control_points
        .iter()
        .flatten()
        .flatten()
        .copied()
        .chain(rows.iter().copied())
        .chain(columns.iter().copied())
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let arena = SourceArena::authored("factorized-profile-projection", 1, values)
        .map_err(|_| numeric_err("Profile projection source admission failed"))?;
    let points: Vec<Vec<_>> = surface
        .control_points
        .iter()
        .enumerate()
        .map(|(u, row)| {
            row.iter()
                .enumerate()
                .map(|(v, _)| {
                    std::array::from_fn(|a| arena.leaf((u * columns.len() + v) * 3 + a).unwrap())
                })
                .collect()
        })
        .collect();
    let rows: Vec<_> = (0..rows.len())
        .map(|u| arena.leaf(count + u).unwrap())
        .collect();
    let columns: Vec<_> = (0..columns.len())
        .map(|v| arena.leaf(count + rows.len() + v).unwrap())
        .collect();
    let tolerance = ToleranceContext::default_valid();
    let mut work = 0;
    let mut exact_reason = None;
    let mut pending_coefficient = None;
    let mut signs = Vec::new();
    for row in 0..3 * surface.degree_u {
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: max_work.saturating_sub(work).min(cad_predicates::MAX_WORK),
                ..Limits::default()
            },
            None,
        );
        let d = cad_predicates::factorized_surface_projected_jacobian_row(
            &mut ctx, &points, &rows, &columns, axes, row,
        )
        .map_err(|_| numeric_err("Profile projection predicate admission failed"))?;
        work += d.work_used;
        match d.signs {
            Some(mut net) => signs.push(net.remove(0)),
            None => {
                exact_reason = d.reason;
                if exact_reason != Some(cad_predicates::Reason::ResourceLimit) || work >= max_work {
                    break;
                }
                let mut coefficients = Vec::new();
                for column in 0..3 * surface.degree_v {
                    let mut ctx = PredicateContext::new(
                        &arena,
                        &tolerance,
                        Limits {
                            max_work: max_work.saturating_sub(work).min(cad_predicates::MAX_WORK),
                            ..Limits::default()
                        },
                        None,
                    );
                    let d = cad_predicates::factorized_surface_projected_jacobian(
                        &mut ctx,
                        &points,
                        &rows,
                        &columns,
                        axes,
                        Some([row, column]),
                    )
                    .map_err(|_| numeric_err("Profile projection coefficient admission failed"))?;
                    work += d.work_used;
                    match d.signs {
                        Some(net) => coefficients.push(net[0][0]),
                        None => {
                            exact_reason = d.reason;
                            pending_coefficient = Some([row, column]);
                            break;
                        }
                    }
                }
                if coefficients.len() != 3 * surface.degree_v {
                    break;
                }
                exact_reason = None;
                signs.push(coefficients);
            }
        }
    }
    let signs = (signs.len() == 3 * surface.degree_u).then_some(signs);
    let orientation = signs.as_ref().and_then(|net| {
        let first = net.iter().flatten().find(|s| **s != Sign::Zero).copied()?;
        net.iter()
            .flatten()
            .all(|s| *s == Sign::Zero || *s == first)
            .then_some(first)
    });
    Ok(Report {
        certificate: orientation.map(|orientation| Certificate {
            profile,
            axes,
            orientation,
        }),
        signs,
        exact_work: work,
        exact_reason,
        pending_coefficient,
        reason: if orientation.is_some() {
            "profile-strict-interior-projection-orientation"
        } else {
            "profile-projection-orientation-unproven"
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{surface::Surface, surface_weight_profile::qualify_weight_profile};
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
            weights: vec![vec![1., 1.1], vec![1.1, 1.21]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn exact_profile_owner_orientation_and_budget() {
        let s = plane();
        let (profile, _) = qualify_weight_profile(&s, [0, 0], 4).unwrap();
        let profile = profile.unwrap();
        let r = certify(&profile, [0, 1], 1_000_000).unwrap();
        let c = r.certificate.unwrap();
        assert!(std::ptr::eq(c.profile(), &profile));
        assert_eq!(c.axes(), [0, 1]);
        assert_eq!(c.orientation(), Sign::Positive);
        assert_eq!(r.signs.unwrap().len(), 3);
        assert!(r.exact_work > 0 && r.exact_work <= 1_000_000);
        assert_eq!(
            certify(&profile, [1, 0], 1_000_000)
                .unwrap()
                .certificate
                .unwrap()
                .orientation(),
            Sign::Negative
        );
        for limit in [1, 16, 100] {
            let r = certify(&profile, [0, 1], limit).unwrap();
            assert!(r.certificate.is_none() && r.signs.is_none());
            assert!(r.exact_work <= limit);
        }
        assert!(certify(&profile, [0, 0], 1_000_000).is_err());
        assert!(certify(&profile, [0, 1], 0).is_err());
    }
    #[test]
    fn profile_fold_and_rank_deficiency_never_admit() {
        let mut s = plane();
        s.control_points[0][1][0] = 1.;
        s.control_points[1][1][0] = 0.;
        let p = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        let r = certify(&p, [0, 1], 1_000_000).unwrap();
        assert!(r.certificate.is_none());
        let signs = r.signs.unwrap();
        assert!(signs.iter().flatten().any(|s| *s == Sign::Positive));
        assert!(signs.iter().flatten().any(|s| *s == Sign::Negative));
        for row in &mut s.control_points {
            for p in row {
                p[1] = 0.;
            }
        }
        let p = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        let r = certify(&p, [0, 1], 1_000_000).unwrap();
        assert!(r.certificate.is_none());
        assert!(r.signs.unwrap().iter().flatten().all(|s| *s == Sign::Zero));
    }
}
