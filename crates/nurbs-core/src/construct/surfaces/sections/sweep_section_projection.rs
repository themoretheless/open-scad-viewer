//! Complete same-basis section correction with one shared exact-work budget.
//! Wall interpolation error is conditional on the owning sweep's retained
//! section family. Source interpolation and filled regions remain separate.
use super::section_projection;
use crate::{Error, Result, curve::Curve};

pub enum Target {
    Plane {
        axis: usize,
        coefficients: [f64; 2],
        offset: f64,
    },
    AuthoredAxis {
        axis: Curve,
        traversal: f64,
    },
}
pub struct Correction {
    pub section: usize,
    pub quantum: f64,
    pub tolerance: f64,
    pub target: Target,
}
pub struct Report {
    pub sections: Option<Vec<Vec<Curve>>>,
    pub wall_displacement_upper: Option<f64>,
    pub exact_planar_sections: Vec<usize>,
    pub work: u64,
    pub reason: &'static str,
}
/// Native endpoint-plane selection for an explicitly requested cap repair.
/// The authored axis has its independent source domain; the ordinary mode
/// uses the first/last actual path edge, with deterministic dominant-axis ties.
pub fn project_miter_caps(
    sections: &[Vec<Curve>],
    points: &[[f64; 3]],
    closed: bool,
    authored: bool,
    axis: Option<&Curve>,
    quantum: f64,
    tolerance: f64,
    max_work: Option<f64>,
) -> Result<Report> {
    let invalid = |message| Error::new("NURBS_INVALID_INPUT", message);
    if closed {
        return Err(invalid("Closed miter has no caps to correct"));
    }
    if !(2..=1025).contains(&sections.len())
        || !(2..=17).contains(&points.len())
        || points.iter().flatten().any(|x| !x.is_finite())
    {
        return Err(invalid(
            "Cap correction requires finite endpoint sites and sections",
        ));
    }
    let mut corrections = Vec::with_capacity(2);
    for end in [false, true] {
        let target = if authored {
            Target::AuthoredAxis {
                axis: axis
                    .ok_or_else(|| invalid("Authored cap correction requires frame_axis"))?
                    .clone(),
                traversal: if end { 1. } else { 0. },
            }
        } else {
            let at = if end { points.len() - 1 } else { 0 };
            let from = if end { points.len() - 2 } else { 0 };
            let to = if end { points.len() - 1 } else { 1 };
            let direction: [f64; 3] = std::array::from_fn(|k| points[to][k] - points[from][k]);
            let axis = (1..3).fold(0, |a, b| {
                if direction[a].abs() >= direction[b].abs() {
                    a
                } else {
                    b
                }
            });
            if !direction[axis].is_finite() || direction[axis] == 0. {
                return Err(invalid("Cap correction needs a nonzero endpoint direction"));
            }
            let free = (0..3).filter(|&k| k != axis).collect::<Vec<_>>();
            let coefficients = [
                -direction[free[0]] / direction[axis],
                -direction[free[1]] / direction[axis],
            ];
            let offset = points[at][axis]
                - coefficients[0] * points[at][free[0]]
                - coefficients[1] * points[at][free[1]];
            Target::Plane {
                axis,
                coefficients,
                offset,
            }
        };
        corrections.push(Correction {
            section: if end { sections.len() - 1 } else { 0 },
            quantum,
            tolerance,
            target,
        });
    }
    project(sections, &corrections, max_work)
}
pub fn project(
    sections: &[Vec<Curve>],
    corrections: &[Correction],
    max_work: Option<f64>,
) -> Result<Report> {
    let mut out = Report {
        sections: None,
        wall_displacement_upper: None,
        exact_planar_sections: vec![],
        work: 0,
        reason: "work-limit",
    };
    let Some(max_work) =
        max_work.filter(|x| x.is_finite() && *x >= 1. && *x <= 1000000. && x.fract() == 0.)
    else {
        return Ok(out);
    };
    let max_work = max_work as u64;
    out.reason = "incompatible-section-basis";
    if !(2..=1025).contains(&sections.len())
        || sections[0].is_empty()
        || sections[0].len() > 64
        || corrections.is_empty()
        || corrections.len() > sections.len()
    {
        return Ok(out);
    }
    let mut seen = vec![false; sections.len()];
    for c in corrections {
        let Some(slot) = seen.get_mut(c.section) else {
            return Ok(out);
        };
        if *slot {
            return Ok(out);
        }
        *slot = true;
    }
    let basis = &sections[0];
    for station in sections {
        if station.len() != basis.len() {
            return Ok(out);
        }
        for (c, a) in station.iter().zip(basis) {
            if c.validate().is_err()
                || c.control_points.iter().any(|p| p.len() != 3)
                || c.degree != a.degree
                || c.periodic != a.periodic
                || c.knots != a.knots
                || c.weights != a.weights
                || c.control_points.len() != a.control_points.len()
            {
                return Ok(out);
            }
        }
    }
    let mut corrected = sections.to_vec();
    let mut exact = vec![];
    let mut upper = 0_f64;
    for c in corrections {
        if out.work == max_work {
            out.reason = "work-limit";
            return Ok(out);
        }
        let r = match &c.target {
            Target::Plane {
                axis,
                coefficients,
                offset,
            } => section_projection::project(
                &sections[c.section],
                *axis,
                *coefficients,
                *offset,
                c.quantum,
                c.tolerance,
                max_work - out.work,
            )?,
            Target::AuthoredAxis { axis, traversal } => section_projection::project_authored_axis(
                &sections[c.section],
                axis,
                *traversal,
                c.quantum,
                c.tolerance,
                max_work - out.work,
            )?,
        };
        out.work += r.work;
        if !r.exact_planar || r.curves.is_none() || r.displacement_upper.is_none() {
            out.reason = if r.reason == "work-limit" {
                "work-limit"
            } else {
                "projection-unproved"
            };
            return Ok(out);
        }
        corrected[c.section] = r.curves.unwrap();
        upper = upper.max(r.displacement_upper.unwrap());
        exact.push(c.section);
    }
    out.sections = Some(corrected);
    out.wall_displacement_upper = Some(upper);
    out.exact_planar_sections = exact;
    out.reason = "bounded-section-interpolation";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn periodic(z: f64) -> Curve {
        Curve {
            degree: 2,
            knots: (0..9).map(|i| i as f64).collect(),
            control_points: [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.], [0., 1.]]
                .iter()
                .map(|p| vec![p[0], p[1], z])
                .collect(),
            weights: vec![1.; 6],
            periodic: true,
        }
    }
    fn correction(section: usize, offset: f64) -> Correction {
        Correction {
            section,
            quantum: 2_f64.powi(-40),
            tolerance: 0.02,
            target: Target::Plane {
                axis: 2,
                coefficients: [0., 0.],
                offset,
            },
        }
    }
    #[test]
    fn whole_family_projection_preserves_periodic_basis_and_shared_work() {
        let sections = vec![
            vec![periodic(0.01)],
            vec![periodic(0.02)],
            vec![periodic(0.01)],
        ];
        let before = sections.clone();
        let c = correction(0, 0.);
        let first = project(&sections, &[c], Some(1000000.)).unwrap();
        assert!(first.wall_displacement_upper.unwrap() < 0.02);
        assert!(first.work > 0);
        let whole = project(
            &sections,
            &[correction(0, 0.), correction(2, 0.)],
            Some(1000000.),
        )
        .unwrap();
        assert_eq!(whole.exact_planar_sections, vec![0, 2]);
        assert_eq!(whole.work, first.work * 2);
        assert_eq!(whole.wall_displacement_upper, first.wall_displacement_upper);
        let kept = whole.sections.unwrap();
        assert_eq!(kept[1], sections[1]);
        assert_eq!(kept[2][0].weights, sections[2][0].weights);
        assert!(kept[0][0].periodic);
        assert!(kept[2][0].control_points.iter().all(|p| p[2] == 0.));
        let cut = project(
            &sections,
            &[correction(0, 0.), correction(2, 0.)],
            Some(first.work as f64),
        )
        .unwrap();
        assert_eq!(cut.reason, "work-limit");
        assert_eq!(cut.work, first.work);
        assert!(cut.sections.is_none());
        assert!(cut.wall_displacement_upper.is_none());
        assert!(cut.exact_planar_sections.is_empty());
        assert_eq!(sections, before);
    }
    #[test]
    fn declaration_and_basis_refusals_cannot_leave_partial_correction() {
        let sections = vec![vec![periodic(0.01)], vec![periodic(0.01)]];
        for budget in [
            None,
            Some(0.),
            Some(-1.),
            Some(0.5),
            Some(1000001.),
            Some(f64::NAN),
        ] {
            let r = project(&sections, &[correction(1, 0.)], budget).unwrap();
            assert_eq!(r.reason, "work-limit");
            assert_eq!(r.work, 0);
        }
        for c in [
            vec![],
            vec![correction(2, 0.)],
            vec![correction(0, 0.), correction(0, 0.)],
        ] {
            let r = project(&sections, &c, Some(1000000.)).unwrap();
            assert_eq!(r.reason, "incompatible-section-basis");
            assert!(r.sections.is_none());
        }
        let mut bad = sections.clone();
        bad[1][0].weights[0] = 2.;
        assert_eq!(
            project(&bad, &[correction(1, 0.)], Some(1000000.))
                .unwrap()
                .reason,
            "incompatible-section-basis"
        );
        let mut bad = sections.clone();
        bad[1][0].periodic = false;
        assert_eq!(
            project(&bad, &[correction(1, 0.)], Some(1000000.))
                .unwrap()
                .reason,
            "incompatible-section-basis"
        );
        let r = project(
            &sections,
            &[correction(0, 0.), correction(1, 10.)],
            Some(1000000.),
        )
        .unwrap();
        assert_eq!(r.reason, "projection-unproved");
        assert!(r.sections.is_none());
        assert!(r.exact_planar_sections.is_empty());
    }
    #[test]
    fn authored_axis_and_plane_targets_share_the_same_complete_family() {
        let z = 10. + 0.25 * 1.25_f64.sqrt();
        let mut c = periodic(z);
        for p in &mut c.control_points {
            p[2] -= 0.5 * p[1];
        }
        let axis = Curve {
            degree: 1,
            knots: vec![2., 2., 5., 5.],
            control_points: vec![vec![0., 0., 1.], vec![0., 0.5, 1.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let r = project(
            &[vec![periodic(0.)], vec![c]],
            &[
                correction(0, 0.),
                Correction {
                    section: 1,
                    quantum: 2_f64.powi(-40),
                    tolerance: 1e-9,
                    target: Target::AuthoredAxis {
                        axis,
                        traversal: 1.,
                    },
                },
            ],
            Some(1000000.),
        )
        .unwrap();
        assert_eq!(r.reason, "bounded-section-interpolation");
        assert_eq!(r.exact_planar_sections, vec![0, 1]);
        assert!(r.wall_displacement_upper.unwrap() < 1e-9);
    }
    #[test]
    fn endpoint_selection_is_native_and_uses_one_budget_for_both_caps() {
        let sections = vec![vec![periodic(0.01)], vec![periodic(10.01)]];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let r = project_miter_caps(
            &sections,
            &points,
            false,
            false,
            None,
            2_f64.powi(-40),
            0.02,
            Some(1000000.),
        )
        .unwrap();
        assert_eq!(r.exact_planar_sections, vec![0, 1]);
        assert!(r.wall_displacement_upper.unwrap() < 0.02);
        assert!(
            r.sections.unwrap()[1][0]
                .control_points
                .iter()
                .all(|p| p[2] == 10.)
        );
        assert!(
            project_miter_caps(
                &sections,
                &points,
                true,
                false,
                None,
                1.,
                1.,
                Some(1000000.)
            )
            .is_err()
        );
        assert!(
            project_miter_caps(
                &sections,
                &points,
                false,
                true,
                None,
                1.,
                1.,
                Some(1000000.)
            )
            .is_err()
        );
        assert!(
            project_miter_caps(
                &sections,
                &[[0.; 3], [0.; 3]],
                false,
                false,
                None,
                1.,
                1.,
                Some(1000000.)
            )
            .is_err()
        );
        let first_work = project(&sections, &[correction(0, 0.)], Some(1000000.))
            .unwrap()
            .work;
        let short = project_miter_caps(
            &sections,
            &points,
            false,
            false,
            None,
            2_f64.powi(-40),
            0.02,
            Some(first_work as f64),
        )
        .unwrap();
        assert_eq!(short.reason, "work-limit");
        assert!(short.sections.is_none());
    }
    #[cfg(feature = "transport")]
    #[test]
    fn protocol_carries_complete_union_and_shared_budget_refusal() {
        use value_codec::{Value, json};
        let c = periodic(0.01);
        let request = json!({"op":"sweep_project_sections","sections":[[c],[c]],
            "corrections":[{"section":0,"plane":{"axis":2,"coefficients":[0.,0.],"offset":0.},"quantum":2_f64.powi(-40),"tolerance":0.02},
                {"section":1,"plane":{"axis":2,"coefficients":[0.,0.],"offset":0.},"quantum":2_f64.powi(-40),"tolerance":0.02}],"maxWork":1000000});
        let report = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(report["reason"], "bounded-section-interpolation");
        assert_eq!(report["exactPlanarSections"], json!([0, 1]));
        let work = report["work"].as_u64().unwrap();
        let mut cut = request.clone();
        cut["maxWork"] = json!(work / 2);
        let report = crate::transport::dispatch(cut).unwrap();
        assert_eq!(report["reason"], "work-limit");
        assert_eq!(report["sections"], Value::Null);
        assert_eq!(report["wallDisplacementUpper"], Value::Null);
        assert_eq!(report["exactPlanarSections"], json!([]));
        let mut invalid = request;
        invalid["maxWork"] = Value::Null;
        assert_eq!(
            crate::transport::dispatch(invalid).unwrap()["reason"],
            "work-limit"
        );
        let caps = json!({"op":"sweep_project_miter_caps","sections":[[periodic(0.01)],[periodic(10.01)]],
            "points":[[0.,0.,0.],[0.,0.,10.]],"closed":false,"authoredFrame":false,
            "quantum":2_f64.powi(-40),"tolerance":0.02,"maxWork":1000000});
        let report = crate::transport::dispatch(caps.clone()).unwrap();
        assert_eq!(report["reason"], "bounded-section-interpolation");
        assert_eq!(report["exactPlanarSections"], json!([0, 1]));
        let mut closed = caps;
        closed["closed"] = json!(true);
        assert!(crate::transport::dispatch(closed).is_err());
    }
}
