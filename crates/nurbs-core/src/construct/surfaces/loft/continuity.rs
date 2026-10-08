//! Checked end-seam matching with complete section and guide retention bounds.
use crate::{
    Result, check,
    continuity::{self, SurfaceJetReport},
    curve::Curve,
    surface::{Axis, Surface},
};

pub struct EndConstraint<'a> {
    pub reference: &'a Surface,
    pub boundary: &'a str,
    pub order: usize,
    pub scale: f64,
    pub reverse: bool,
}
pub struct MatchedLoft {
    pub surface: Surface,
    pub seams: Vec<SurfaceJetReport>,
    pub section_error_upper: Vec<f64>,
    pub guide_error_upper: Vec<f64>,
}
fn curve_error(a: &Curve, b: &Curve) -> Result<f64> {
    let a = crate::gordon::normalized(a)?;
    let b = crate::gordon::normalized(b)?;
    continuity::deviation::positional_upper(
        &crate::surface::loft(&[a.clone(), a])?,
        &crate::surface::loft(&[b.clone(), b])?,
    )
}
/// Match one or both ends of an existing normalized loft. New boundary strips
/// terminate before the nearest interior section; checked whole-curve bounds
/// reject any lost section/guide constraints. G1/G2 are provided by the stronger
/// scaled homogeneous C1/C2 seam conditions and certified regularity.
pub fn match_ends(
    base: &Surface,
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    guide_parameters: &[f64],
    ends: [Option<EndConstraint<'_>>; 2],
    budget: f64,
) -> Result<MatchedLoft> {
    check(
        budget.is_finite() && budget >= 0.,
        "Loft matching budget must be finite and nonnegative",
    )?;
    check(
        guides.len() == guide_parameters.len()
            && guide_parameters
                .iter()
                .all(|u| u.is_finite() && (0. ..=1.).contains(u)),
        "Loft matching guide stations invalid",
    )?;
    check(
        sections.len() >= 2,
        "Loft matching needs at least two sections",
    )?;
    let stations = crate::gordon::stations(parameters, sections.len())?;
    base.validate()?;
    check(
        base.knots_v[base.degree_v] == 0. && base.knots_v[base.control_points[0].len()] == 1.,
        "Loft matching needs normalized V",
    )?;
    check(
        base.degree_v >= 3,
        "Loft end matching requires at least cubic V degree",
    )?;
    check(
        base.knots_u[base.degree_u] == 0. && base.knots_u[base.control_points.len()] == 1.,
        "Loft matching needs normalized U",
    )?;
    let mut surface = base.clone();
    // Disjoint support keeps a second end match from undoing the first one.
    for (end, constraint) in ends.iter().enumerate() {
        if constraint.is_none() {
            continue;
        }
        // Simple distinct knots retain cross-direction C2. Repeated knots
        // would isolate the strip but introduce an artificial interior kink.
        for i in 1..=3 {
            let fraction = i as f64 / 8.;
            let station = if end == 0 {
                stations[1] * fraction
            } else {
                1. - (1. - stations[stations.len() - 2]) * fraction
            };
            surface = surface.edit_axis(Axis::V, |c| c.insert(station, 1))?;
        }
    }
    let mut seams = Vec::new();
    for (end, constraint) in ends.iter().enumerate() {
        if let Some(c) = constraint {
            let edge = if end == 0 { "vMin" } else { "vMax" };
            let prepared = continuity::preparation::checked_report(
                c.reference,
                &surface,
                c.boundary,
                edge,
                c.reverse,
                budget,
            )?;
            check(
                prepared.report.accepted,
                "Loft seam basis preparation exceeds budget",
            )?;
            let matched = continuity::match_surface_jets_checked_report(
                &prepared.reference,
                &prepared.edited,
                c.boundary,
                edge,
                c.order,
                c.scale,
                false,
                budget,
            )?;
            check(
                matched.report.decision.as_ref().is_some_and(|d| d.accepted),
                "Loft seam continuity is not certified within budget",
            )?;
            surface = if c.reverse {
                matched
                    .surface
                    .edit_axis(Axis::U, |curve| curve.reverse())?
            } else {
                matched.surface
            };
            seams.push(matched.report);
        }
    }
    // Reinspect the final surface: a later seam basis preparation must not
    // silently invalidate a report from an earlier match.
    seams.clear();
    for (end, constraint) in ends.iter().enumerate() {
        if let Some(c) = constraint {
            let edge = if end == 0 { "vMin" } else { "vMax" };
            let prepared = continuity::preparation::checked_report(
                c.reference,
                &surface,
                c.boundary,
                edge,
                c.reverse,
                budget,
            )?;
            check(
                prepared.report.accepted,
                "Final seam preparation exceeds budget",
            )?;
            let expected = if c.reverse {
                surface.edit_axis(Axis::U, |curve| curve.reverse())?
            } else {
                surface.clone()
            };
            check(
                prepared.edited.degree_u == expected.degree_u
                    && prepared.edited.degree_v == expected.degree_v
                    && prepared.edited.knots_u == expected.knots_u
                    && prepared.edited.knots_v == expected.knots_v
                    && prepared.edited.control_points == expected.control_points
                    && prepared.edited.weights == expected.weights,
                "Final seam inspection would require changing the loft basis",
            )?;
            let mut report = continuity::inspect_surface_jets_checked_report(
                &prepared.reference,
                &prepared.edited,
                c.boundary,
                edge,
                c.order,
                c.scale,
                budget,
            )?;
            check(
                report.decision.as_ref().is_some_and(|d| d.accepted),
                "Final loft seam jets are outside budget",
            )?;
            report.reversed = c.reverse;
            seams.push(report);
        }
    }
    let section_error_upper = sections
        .iter()
        .zip(stations)
        .map(|(c, v)| curve_error(c, &surface.iso(Axis::V, v)?))
        .collect::<Result<Vec<_>>>()?;
    let guide_error_upper = guides
        .iter()
        .zip(guide_parameters)
        .map(|(c, &u)| curve_error(c, &surface.iso(Axis::U, u)?))
        .collect::<Result<Vec<_>>>()?;
    check(
        section_error_upper
            .iter()
            .chain(&guide_error_upper)
            .all(|e| *e <= budget),
        "Loft continuity conflicts with retained section or guide constraints",
    )?;
    Ok(MatchedLoft {
        surface,
        seams,
        section_error_upper,
        guide_error_upper,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn section(z: f64) -> Curve {
        crate::primitives::line([0., 0., z], [1., 0., z]).unwrap()
    }
    fn plane(a: f64, b: f64) -> Surface {
        crate::surface::loft(&[section(a), section(b)])
            .unwrap()
            .edit_axis(Axis::V, |c| c.elevate(3))
            .unwrap()
    }
    #[test]
    fn two_end_g2_retains_complete_sections() {
        let sections = vec![section(0.), section(1.), section(2.)];
        let base = crate::natural_loft::interpolate(&sections, &[0., 0.5, 1.]).unwrap();
        let a = plane(-2., 0.);
        let b = plane(2., 4.);
        let matched = match_ends(
            &base,
            &sections,
            &[0., 0.5, 1.],
            &[],
            &[],
            [
                Some(EndConstraint {
                    reference: &a,
                    boundary: "vMax",
                    order: 2,
                    scale: 1.,
                    reverse: false,
                }),
                Some(EndConstraint {
                    reference: &b,
                    boundary: "vMin",
                    order: 2,
                    scale: 1.,
                    reverse: false,
                }),
            ],
            1e-9,
        )
        .unwrap();
        assert_eq!(matched.seams.len(), 2);
        for seam in matched.seams {
            assert!(seam.decision.unwrap().accepted);
        }
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            for v in [0., 0.13, 0.5, 0.87, 1.] {
                let p = matched.surface.evaluate(u, v).unwrap().point;
                assert!(
                    (p[0] - u).abs() < 1e-12 && p[1].abs() < 1e-12 && (p[2] - 2. * v).abs() < 1e-12
                );
            }
        }
    }
    #[test]
    fn conflicting_guide_is_refused() {
        let sections = vec![section(0.), section(2.)];
        let base = plane(0., 2.);
        let guide = crate::primitives::line([0.5, 1., 0.], [0.5, 1., 2.]).unwrap();
        assert!(
            match_ends(
                &base,
                &sections,
                &[0., 1.],
                &[guide],
                &[0.5],
                [None, None],
                1e-9
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod curved_tests {
    use super::*;
    #[test]
    fn nonzero_curvature_is_matched_without_artificial_internal_c0_knots() {
        let section = |z| crate::primitives::line([0., 0., z], [1., 0., z]).unwrap();
        let sections = [section(0.), section(1.), section(2.)];
        let base = crate::natural_loft::interpolate(&sections, &[0., 0.5, 1.]).unwrap();
        let reference = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![0., 1., -2.], vec![0., 0., -1.], vec![0., 0., 0.]],
                vec![vec![1., 1., -2.], vec![1., 0., -1.], vec![1., 0., 0.]],
            ],
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let matched = match_ends(
            &base,
            &sections,
            &[0., 0.5, 1.],
            &[],
            &[],
            [
                Some(EndConstraint {
                    reference: &reference,
                    boundary: "vMax",
                    order: 2,
                    scale: 1.,
                    reverse: false,
                }),
                None,
            ],
            1e-8,
        )
        .unwrap();
        for u in [0., 0.23, 0.71, 1.] {
            let value = matched.surface.evaluate(u, 0.).unwrap();
            let (_, dv) = value.first_derivatives().unwrap();
            let (_, _, dvv) = value.second_derivatives().unwrap();
            assert!(dv[1].abs() < 1e-12 && (dv[2] - 2.).abs() < 1e-12);
            assert!((dvv[1] - 2.).abs() < 1e-10 && dvv[2].abs() < 1e-10);
            assert!(matched.surface.evaluate(u, 0.5).unwrap().point[1].abs() < 1e-12);
        }
        for &k in matched
            .surface
            .knots_v
            .iter()
            .filter(|k| **k > 0. && **k < 0.5)
        {
            assert_eq!(
                matched.surface.knots_v.iter().filter(|x| **x == k).count(),
                1
            );
            assert!(
                matched
                    .surface
                    .evaluate(0.5, k)
                    .unwrap()
                    .second_derivatives()
                    .is_some()
            );
        }
    }
}

#[cfg(test)]
mod orientation_tests {
 use super::*;
 #[test]
 fn reversed_reference_and_scaled_normal_keep_authored_u_orientation(){
  let section=|z|crate::primitives::line([0.,0.,z],[1.,0.,z]).unwrap();
  let sections=[section(0.),section(2.)];
  let base=crate::natural_loft::interpolate(&sections,&[0.,1.]).unwrap();
  let reference=crate::natural_loft::interpolate(&[section(-1.),section(0.)],&[0.,1.]).unwrap().edit_axis(Axis::U,|c|c.reverse()).unwrap();
  let matched=match_ends(&base,&sections,&[0.,1.],&[],&[],[Some(EndConstraint{reference:&reference,boundary:"vMax",order:2,scale:2.,reverse:true}),None],1e-8).unwrap();
  assert!(matched.seams[0].reversed);
  assert!(matched.seams[0].decision.as_ref().unwrap().accepted);
  for u in [0.,0.27,0.63,1.] {
   let evaluated=matched.surface.evaluate(u,0.).unwrap();
   assert!((evaluated.point[0]-u).abs()<1e-12);
   assert!((evaluated.first_derivatives().unwrap().1[2]-2.).abs()<1e-12);
  }
 }
}
