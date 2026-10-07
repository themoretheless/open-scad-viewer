//! Read-only, bounded sweep seam qualification. A jet tolerance is explicit:
//! a nonzero tolerance is not a claim of exact G1/G2 continuity.
use crate::{Result, check, continuity, surface::Surface};
pub struct Seam<'a> {
    pub patches: [usize; 2],
    pub boundaries: [&'a str; 2],
    pub order: usize,
    pub normal_scale: f64,
    pub jet_tolerance: f64,
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane(y: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..3)
                        .map(|v| vec![u as f64, y + v as f64 * 0.5, 0.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn seam(order: usize) -> Seam<'static> {
        Seam {
            patches: [0, 1],
            boundaries: ["vMax", "vMin"],
            order,
            normal_scale: 1.,
            jet_tolerance: 1e-8,
        }
    }
    #[test]
    fn ruled_second_jets_keep_rational_weight_terms() {
        let ruled=|y| {
            let mut s=plane(y);
            s.degree_v=1;s.knots_v=vec![0.,0.,1.,1.];
            for row in &mut s.control_points {row.remove(1);}
            s.weights=vec![vec![1.;2];2];s
        };
        let a=ruled(0.);let mut b=ruled(1.);
        assert!(inspect(&[a.clone(),b.clone()],&[seam(2)],1).unwrap().all_within_jet_budget);
        for row in &mut b.weights {row[1]=2.;}
        let r=inspect(&[a,b],&[seam(2)],1).unwrap();
        assert!(!r.all_within_jet_budget);
        assert!(r.seams[0].error_upper.is_some());
    }
    #[test]
    fn separates_first_second_jets_and_exhausted_seams_without_editing() {
        let a = plane(0.);
        let mut b = plane(1.);
        let r = inspect(&[a.clone(), b.clone()], &[seam(1), seam(2)], 2).unwrap();
        assert!(r.all_within_jet_budget, "{:?}", r.seams);
        for row in &mut b.control_points {
            row[2][2] = 0.1;
        }
        let before = b.control_points.clone();
        let r = inspect(&[a.clone(), b.clone()], &[seam(1), seam(2)], 2).unwrap();
        assert!(r.seams[0].within_jet_budget);
        assert!(!r.seams[1].within_jet_budget);
        assert_eq!(r.seams[1].reason, "jet-deviation-exceeds-budget");
        assert_eq!(b.control_points, before);
        let r = inspect(&[a, b], &[seam(1), seam(2)], 1).unwrap();
        assert!(!r.all_within_jet_budget && r.inspected_seams == 1);
        assert_eq!(r.seams[1].reason, "seam-budget-exhausted");
        assert!(r.seams[1].error_upper.is_none());
    }
    #[test]
    fn sharp_or_singular_seam_cannot_promote_to_smooth() {
        let a = plane(0.);
        let mut b = plane(1.);
        for row in &mut b.control_points {
            row[1][2] = 0.25;
        }
        let r = inspect(&[a.clone(), b.clone()], &[seam(1)], 1).unwrap();
        assert!(!r.all_within_jet_budget);
        assert_eq!(r.seams[0].reason, "jet-deviation-exceeds-budget");
        b.control_points[1] = b.control_points[0].clone();
        let r = inspect(&[a, b], &[seam(1)], 1).unwrap();
        assert!(!r.all_within_jet_budget && !r.seams[0].regularity_certified);
        assert_eq!(r.seams[0].reason, "seam-certificate-unresolved");
        assert!(r.seams[0].error_upper.is_none());
    }
    #[test]
    fn actual_corner_translation_sweeps_qualify_declared_orders() {
        let sites = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.]];
        let spatial = [[0., 0., 0.], [10., 0., 2.], [10., 10., 5.], [0., 10., 1.]];
        let planar = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]];
        let profile = crate::primitives::line([0., 0., 0.], [0., 0., 1.]).unwrap();
        for (path, order, closed) in [
            (crate::paths::round_polyline(&sites, 2.).unwrap(), 1, false),
            (
                crate::paths::transition_polyline(&sites, 2.).unwrap(),
                2,
                false,
            ),
            (
                crate::paths::round_polyline(&spatial, 2.).unwrap(),
                1,
                false,
            ),
            (
                crate::paths::transition_polyline(&spatial, 2.).unwrap(),
                2,
                false,
            ),
            (
                crate::paths::closed_round_polyline(&planar, 2.).unwrap(),
                1,
                true,
            ),
            (
                crate::paths::closed_transition_polyline(&planar, 2.).unwrap(),
                2,
                true,
            ),
            (
                crate::paths::closed_round_polyline(&spatial, 2.).unwrap(),
                1,
                true,
            ),
            (
                crate::paths::closed_transition_polyline(&spatial, 2.).unwrap(),
                2,
                true,
            ),
        ] {
            let pieces = path.decompose().unwrap();
            let walls: Vec<_> = if path.control_points.len() <= 32 {
                let base = crate::surface::sweep(&profile, &path).unwrap();
                pieces
                    .iter()
                    .map(|piece| {
                        let d = piece.domain();
                        base.trim([0., 1., d[0], d[1]]).unwrap()
                    })
                    .collect()
            } else {
                assert!(crate::surface::sweep(&profile, &path).is_err());
                // Qualify retained coefficient composition per rational path
                // piece when the whole-surface constructor refuses its budget.
                // All fixture paths start at zero, so translation is P(u)+Q(v).
                pieces
                    .iter()
                    .map(|piece| {
                        let curve = piece.definition();
                        let wall = Surface {
                            degree_u: profile.degree,
                            degree_v: curve.degree,
                            knots_u: profile.knots.clone(),
                            knots_v: curve.knots.clone(),
                            control_points: profile
                                .control_points
                                .iter()
                                .map(|p| {
                                    curve
                                        .control_points
                                        .iter()
                                        .map(|q| (0..3).map(|k| p[k] + q[k]).collect())
                                        .collect()
                                })
                                .collect(),
                            weights: profile
                                .weights
                                .iter()
                                .map(|p| curve.weights.iter().map(|q| p * q).collect())
                                .collect(),
                            periodic_u: false,
                            periodic_v: false,
                        };
                        wall.validate().unwrap();
                        wall
                    })
                    .collect()
            };
            let mut joins: Vec<_> = (0..walls.len() - 1)
                .map(|i| {
                    let a = pieces[i].domain();
                    let b = pieces[i + 1].domain();
                    Seam {
                        patches: [i, i + 1],
                        boundaries: ["vMax", "vMin"],
                        order,
                        normal_scale: (b[1] - b[0]) / (a[1] - a[0]),
                        jet_tolerance: 1e-8,
                    }
                })
                .collect();
            if closed {
                let last = pieces.len() - 1;
                let a = pieces[last].domain();
                let b = pieces[0].domain();
                joins.push(Seam {
                    patches: [last, 0],
                    boundaries: ["vMax", "vMin"],
                    order,
                    normal_scale: (b[1] - b[0]) / (a[1] - a[0]),
                    jet_tolerance: 1e-8,
                });
            }
            let report = inspect(&walls, &joins, joins.len()).unwrap();
            assert!(
                report.all_within_jet_budget,
                "order {order}, closed {closed}: {:?}",
                report.seams
            );
            assert!(report.seams.iter().all(|s| s.regularity_certified));
            let exhausted = inspect(&walls, &joins, 0).unwrap();
            assert!(!exhausted.all_within_jet_budget && exhausted.inspected_seams == 0);
            assert!(
                exhausted
                    .seams
                    .iter()
                    .all(|s| s.error_upper.is_none() && s.reason == "seam-budget-exhausted")
            );
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn transport_retains_unproved_budget_and_explicit_tolerance_contract() {
        use value_codec::json;
        let request = json!({"op":"sweep_seam_audit","patches":[plane(0.),plane(1.)],
            "seams":[{"patches":[0,1],"boundaries":["vMax","vMin"],"order":2,
                "normalScale":1.,"jetTolerance":1e-8}],"maxSeams":1});
        let r = crate::dispatch(request.clone()).unwrap();
        assert_eq!(r["allWithinJetBudget"], true);
        assert_eq!(r["exactG1G2Certified"], false);
        let mut exhausted = request.clone();
        exhausted["maxSeams"] = json!(0);
        let r = crate::dispatch(exhausted).unwrap();
        assert_eq!(r["allWithinJetBudget"], false);
        assert!(r["seams"][0]["errorUpper"].is_null());
        assert_eq!(r["seams"][0]["reason"], "seam-budget-exhausted");
        let mut invalid = request;
        invalid["seams"][0]["normalScale"] = json!(-1.);
        assert!(crate::dispatch(invalid).is_err());
    }
}
#[derive(Clone, Debug)]
pub struct Evidence {
    pub within_jet_budget: bool,
    pub regularity_certified: bool,
    pub tangential_smoothness_certified: bool,
    pub error_upper: Option<f64>,
    pub reason: &'static str,
}
pub struct Report {
    pub all_within_jet_budget: bool,
    pub inspected_seams: usize,
    /// Same order as the declarations, including uninspected suffix entries.
    pub seams: Vec<Evidence>,
}
pub fn inspect(patches: &[Surface], seams: &[Seam<'_>], max_seams: usize) -> Result<Report> {
    check(
        patches.len() <= 1024 && seams.len() <= 4096 && max_seams <= 4096,
        "Invalid seam audit size",
    )?;
    for patch in patches {
        patch.validate()?;
    }
    for seam in seams {
        check(
            seam.patches.iter().all(|i| *i < patches.len())
                && matches!(seam.order, 1 | 2)
                && seam.normal_scale.is_finite()
                && seam.normal_scale > 0.
                && seam.jet_tolerance.is_finite()
                && seam.jet_tolerance >= 0.
                && seam
                    .boundaries
                    .iter()
                    .all(|name| matches!(*name, "uMin" | "uMax" | "vMin" | "vMax")),
            "Invalid seam declaration",
        )?;
    }
    let mut evidence = Vec::with_capacity(seams.len());
    for (index, seam) in seams.iter().enumerate() {
        let mut item = Evidence {
            within_jet_budget: false,
            regularity_certified: false,
            tangential_smoothness_certified: false,
            error_upper: None,
            reason: if index >= max_seams {
                "seam-budget-exhausted"
            } else {
                "seam-certificate-unresolved"
            },
        };
        if index < max_seams {
            if let Ok(report) = continuity::inspect_surface_jets_checked_report(
                &patches[seam.patches[0]],
                &patches[seam.patches[1]],
                seam.boundaries[0],
                seam.boundaries[1],
                seam.order,
                seam.normal_scale,
                seam.jet_tolerance,
            ) {
                item.regularity_certified = report.regularity_certified();
                if let Some(decision) = report.decision {
                    item.within_jet_budget = decision.accepted;
                    item.tangential_smoothness_certified = decision.tangential_smoothness_certified;
                    item.error_upper = Some(decision.error_upper);
                    item.reason = match decision.reason {
                        continuity::SurfaceJetDecisionReason::Accepted => "jets-within-budget",
                        continuity::SurfaceJetDecisionReason::UnprovenRegularity => {
                            "seam-regularity-unproved"
                        }
                        continuity::SurfaceJetDecisionReason::UnprovenTangentialSmoothness => {
                            "tangential-smoothness-unproved"
                        }
                        continuity::SurfaceJetDecisionReason::DeviationExceedsBudget => {
                            "jet-deviation-exceeds-budget"
                        }
                    };
                }
            }
        }
        evidence.push(item);
    }
    Ok(Report {
        all_within_jet_budget: evidence.iter().all(|item| item.within_jet_budget),
        inspected_seams: seams.len().min(max_seams),
        seams: evidence,
    })
}
