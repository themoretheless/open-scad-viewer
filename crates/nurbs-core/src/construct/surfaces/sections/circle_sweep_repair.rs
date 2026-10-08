//! Whole same-basis circle section repair, with one shared work budget.
//! The maximum pole displacement bounds ruled interpolation only; the source
//! sweep, retained topology, filled caps and embedding need their own proofs.
use super::circle_repair;
use crate::{Result, curve::Curve};
pub struct Report {
    pub sections: Option<Vec<Vec<Curve>>>,
    pub wall_displacement_upper: Option<f64>,
    pub work: u64,
    pub reason: &'static str,
}
pub fn repair(
    sections: &[Vec<Curve>],
    quantum: f64,
    tolerance: f64,
    max_work: Option<f64>,
) -> Result<Report> {
    let mut out = Report {
        sections: None,
        wall_displacement_upper: None,
        work: 0,
        reason: "invalid-budget-or-sections",
    };
    let Some(max_work) =
        max_work.filter(|x| x.is_finite() && *x >= 0. && *x <= 1000000. && x.fract() == 0.)
    else {
        return Ok(out);
    };
    let max_work = max_work as u64;
    if sections.is_empty()
        || sections.len() > 1025
        || sections[0].is_empty()
        || sections[0].len() > 64
    {
        return Ok(out);
    }
    out.reason = "incompatible-section-basis";
    let basis = &sections[0];
    for station in sections {
        if station.len() != basis.len() {
            return Ok(out);
        }
        for (c, a) in station.iter().zip(basis) {
            if c.validate().is_err()
                || c.degree != a.degree
                || c.knots != a.knots
                || c.weights != a.weights
                || c.periodic != a.periodic
                || c.control_points.len() != a.control_points.len()
            {
                return Ok(out);
            }
        }
    }
    let mut corrected = Vec::with_capacity(sections.len());
    let mut upper = 0_f64;
    for station in sections {
        let r = circle_repair::repair(station, quantum, tolerance, max_work - out.work)?;
        out.work += r.work;
        let (Some(curves), Some(bound)) = (r.curves, r.displacement_upper) else {
            out.reason = r.reason;
            return Ok(out);
        };
        if !bound.is_finite() || bound < 0. || bound > tolerance {
            out.reason = "invalid-native-displacement";
            return Ok(out);
        }
        upper = upper.max(bound);
        corrected.push(curves);
    }
    out.sections = Some(corrected);
    out.wall_displacement_upper = Some(upper);
    out.reason = "bounded-circle-section-interpolation";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sections() -> Vec<Vec<Curve>> {
        (0..3)
            .map(|z| {
                [0.5, 0.25]
                    .iter()
                    .map(|&r| {
                        crate::primitives::circle([0., 0., z as f64], [0., 0., 1.], r).unwrap()
                    })
                    .collect()
            })
            .collect()
    }
    #[test]
    fn complete_hollow_family_bounds_every_station_and_preserves_input() {
        let sections = sections();
        let before = sections.clone();
        let r = repair(&sections, 2_f64.powi(-40), 1e-9, Some(1000000.)).unwrap();
        assert_eq!(r.reason, "bounded-circle-section-interpolation");
        assert_eq!(r.work, 216);
        assert!(r.wall_displacement_upper.unwrap() < 1e-9);
        let corrected = r.sections.unwrap();
        assert_eq!(corrected.len(), 3);
        for (a, b) in corrected.iter().flatten().zip(sections.iter().flatten()) {
            assert_eq!(a.weights, b.weights);
            assert_eq!(a.knots, b.knots);
        }
        assert_eq!(sections, before);
        let short = repair(&sections, 2_f64.powi(-40), 1e-9, Some(215.)).unwrap();
        assert_eq!(short.work, 215);
        assert_eq!(short.reason, "work-limit");
        assert!(short.sections.is_none());
        assert!(short.wall_displacement_upper.is_none());
    }
    #[test]
    fn basis_budget_and_displacement_failures_never_return_partial_geometry() {
        let sections = sections();
        for budget in [None, Some(-1.), Some(0.5), Some(1000001.)] {
            let r = repair(&sections, 2_f64.powi(-40), 1e-9, budget).unwrap();
            assert_eq!(r.reason, "invalid-budget-or-sections");
            assert_eq!(r.work, 0);
        }
        let r = repair(&sections, 2_f64.powi(-40), 1e-9, Some(0.)).unwrap();
        assert_eq!(r.reason, "work-limit");
        assert!(r.sections.is_none());
        let mut bad = sections.clone();
        bad[1][1].weights[0] = 2.;
        assert_eq!(
            repair(&bad, 2_f64.powi(-40), 1e-9, Some(1000000.))
                .unwrap()
                .reason,
            "incompatible-section-basis"
        );
        let mut bad = sections.clone();
        bad[1].pop();
        assert!(
            repair(&bad, 2_f64.powi(-40), 1e-9, Some(1000000.))
                .unwrap()
                .sections
                .is_none()
        );
        let mut displaced = sections.clone();
        displaced[2][1].control_points[1][0] += 0.01;
        let r = repair(&displaced, 0.125, 1e-30, Some(1000000.)).unwrap();
        assert_eq!(r.reason, "displacement-budget");
        assert!(r.wall_displacement_upper.is_none());
    }
    #[cfg(feature = "transport")]
    #[test]
    fn transport_keeps_shared_work_and_union_maximum_native() {
        use value_codec::{Value, json};
        let request = json!({"op":"sweep_repair_circle_sections","sections":sections(),"quantum":2_f64.powi(-40),"tolerance":1e-9,"maxWork":1000000});
        let r = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(r["work"], 216);
        assert_eq!(r["reason"], "bounded-circle-section-interpolation");
        let mut cut = request;
        cut["maxWork"] = json!(215);
        let r = crate::transport::dispatch(cut).unwrap();
        assert_eq!(r["sections"], Value::Null);
        assert_eq!(r["wallDisplacementUpper"], Value::Null);
        assert_eq!(r["reason"], "work-limit");
    }
}
