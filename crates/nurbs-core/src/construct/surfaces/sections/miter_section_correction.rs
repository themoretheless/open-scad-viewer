//! Native correction order and displacement composition for retained miter
//! sections. Final B-rep correspondence/regularity/admission remain mandatory.
use super::{circle_sweep_repair, sweep_section_projection};
use crate::{Error, Result, curve::Curve, numerics::error_upper};
#[derive(Clone, Copy)]
pub struct Budget {
    pub quantum: f64,
    pub tolerance: f64,
    pub max_work: Option<f64>,
}
pub struct CapCorrection {
    pub budget: Budget,
    pub authored_frame: bool,
}
pub fn correct(
    sections: &[Vec<Curve>],
    points: &[[f64; 3]],
    closed: bool,
    circle: Option<Budget>,
    cap: Option<CapCorrection>,
    axis: Option<&Curve>,
) -> Result<Option<sweep_section_projection::Report>> {
    let invalid = |message| Error::new("NURBS_INVALID_INPUT", message);
    if circle.is_none() && cap.is_none() {
        return Ok(None);
    }
    let repaired = if let Some(b) = circle {
        let r = circle_sweep_repair::repair(sections, b.quantum, b.tolerance, b.max_work)?;
        if r.sections.is_none() || r.wall_displacement_upper.is_none() {
            return Err(invalid(format!(
                "Miter circle section correction unproved: {}",
                r.reason
            )));
        }
        Some(r)
    } else {
        None
    };
    let retained = repaired
        .as_ref()
        .and_then(|r| r.sections.as_deref())
        .unwrap_or(sections);
    let projected = if let Some(c) = cap {
        let b = c.budget;
        let r = sweep_section_projection::project_miter_caps(
            retained,
            points,
            closed,
            c.authored_frame,
            axis,
            b.quantum,
            b.tolerance,
            b.max_work,
        )?;
        if r.sections.is_none() || r.wall_displacement_upper.is_none() {
            return Err(invalid(format!(
                "Miter {}cap correction unproved: {}",
                if c.authored_frame { "authored " } else { "" },
                r.reason
            )));
        }
        Some(r)
    } else {
        None
    };
    let upper = error_upper::add(
        repaired
            .as_ref()
            .map_or(0., |r| r.wall_displacement_upper.unwrap()),
        projected
            .as_ref()
            .map_or(0., |r| r.wall_displacement_upper.unwrap()),
    )
    .ok_or_else(|| invalid("Miter combined section correction displacement unproved".to_owned()))?;
    let work = repaired.as_ref().map_or(0, |r| r.work) + projected.as_ref().map_or(0, |r| r.work);
    let exact = projected
        .as_ref()
        .map_or_else(Vec::new, |r| r.exact_planar_sections.clone());
    let sections = projected
        .and_then(|r| r.sections)
        .or_else(|| repaired.and_then(|r| r.sections));
    Ok(Some(sweep_section_projection::Report {
        sections,
        wall_displacement_upper: Some(upper),
        exact_planar_sections: exact,
        work,
        reason: if circle.is_some() {
            "bounded-circle-section-interpolation"
        } else {
            "bounded-section-interpolation"
        },
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn budget(tolerance: f64) -> Budget {
        Budget {
            quantum: 2_f64.powi(-40),
            tolerance,
            max_work: Some(1000000.),
        }
    }
    fn sections() -> Vec<Vec<Curve>> {
        [0.01, 10.01]
            .iter()
            .map(|&z| {
                [0.5, 0.25]
                    .iter()
                    .map(|&r| crate::primitives::circle([0., 0., z], [0., 0., 1.], r).unwrap())
                    .collect()
            })
            .collect()
    }
    #[test]
    fn native_pipeline_projects_last_and_composes_every_applied_displacement() {
        let sections = sections();
        let before = sections.clone();
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let r = correct(
            &sections,
            &points,
            false,
            Some(budget(1e-9)),
            Some(CapCorrection {
                budget: budget(0.02),
                authored_frame: false,
            }),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r.reason, "bounded-circle-section-interpolation");
        assert_eq!(r.exact_planar_sections, vec![0, 1]);
        assert!(r.wall_displacement_upper.unwrap() < 0.02);
        assert!(r.work > 144);
        let retained = r.sections.unwrap();
        assert!(
            retained[0]
                .iter()
                .flat_map(|c| &c.control_points)
                .all(|p| p[2] == 0.)
        );
        assert!(
            retained[1]
                .iter()
                .flat_map(|c| &c.control_points)
                .all(|p| p[2] == 10.)
        );
        assert_eq!(sections, before);
        assert!(
            correct(&sections, &points, false, None, None, None)
                .unwrap()
                .is_none()
        );
        let only_caps = correct(
            &sections,
            &points,
            false,
            None,
            Some(CapCorrection {
                budget: budget(0.02),
                authored_frame: false,
            }),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(only_caps.reason, "bounded-section-interpolation");
        assert_eq!(only_caps.exact_planar_sections, vec![0, 1]);
    }
    #[test]
    fn correction_failure_never_leaves_a_candidate_for_admission() {
        let sections = sections();
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let mut short = budget(1e-9);
        short.max_work = Some(0.);
        assert!(
            correct(&sections, &points, false, Some(short), None, None)
                .err()
                .unwrap()
                .to_string()
                .contains("Miter circle section correction unproved")
        );
        assert!(
            correct(
                &sections,
                &points,
                false,
                Some(budget(1e-9)),
                Some(CapCorrection {
                    budget: budget(0.),
                    authored_frame: false
                }),
                None
            )
            .is_err()
        );
        assert!(
            correct(
                &sections,
                &points,
                true,
                None,
                Some(CapCorrection {
                    budget: budget(0.02),
                    authored_frame: false
                }),
                None
            )
            .is_err()
        );
    }
    #[cfg(feature = "transport")]
    #[test]
    fn protocol_owns_correction_order_and_preserves_no_correction_mode() {
        use value_codec::{Value, json};
        let request = json!({"op":"sweep_correct_miter_sections","sections":sections(),"points":[[0.,0.,0.],[0.,0.,10.]],"closed":false,
            "circleCorrection":{"quantum":2_f64.powi(-40),"tolerance":1e-9,"maxWork":1000000},
            "capCorrection":{"quantum":2_f64.powi(-40),"tolerance":0.02,"maxWork":1000000,"authoredFrame":false}});
        let r = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(r["reason"], "bounded-circle-section-interpolation");
        assert_eq!(r["exactPlanarSections"], json!([0, 1]));
        assert!(r["wallDisplacementUpper"].as_f64().unwrap() < 0.02);
        let mut untouched = request.clone();
        untouched["circleCorrection"] = Value::Null;
        untouched["capCorrection"] = Value::Null;
        assert_eq!(crate::transport::dispatch(untouched).unwrap(), Value::Null);
        let mut short = request;
        short["circleCorrection"]["maxWork"] = json!(0);
        assert!(crate::transport::dispatch(short).is_err());
    }
}
