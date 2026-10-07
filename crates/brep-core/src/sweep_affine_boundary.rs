//! Conditional complete-boundary transport through exact lattice placement.
//! Constructor provenance and geometric Solid admission remain separate.
use crate::{Model, Result, affine_lattice};
use nurbs_core::{
    numerics::error_upper,
    sweeps::filled_cap_error::{self, BoundaryCertificate},
};

pub struct Premises {
    pub wall: Option<f64>,
    pub caps: Option<[f64; 2]>,
    pub closed: bool,
    pub source_budget: Option<f64>,
}
pub struct Report {
    pub placement: Option<affine_lattice::Report>,
    pub boundary: Option<BoundaryCertificate>,
    pub reason: &'static str,
}
/// Never publishes transformed geometry when its complete transported bound
/// is missing, outside tolerance, or requires unsupported arithmetic.
pub fn place(
    source: &Model,
    premises: &Premises,
    matrix: [[f64; 4]; 4],
    quantum: f64,
    max_work: u64,
    budget: Option<f64>,
) -> Result<Report> {
    let mut out = Report {
        placement: None,
        boundary: None,
        reason: "source-bound-unproved",
    };
    let before = filled_cap_error::compose_boundary(
        premises.wall,
        premises.caps,
        premises.closed,
        premises.source_budget,
    );
    if !before.continuous_bound || before.within_budget != Some(true) {
        return Ok(out);
    }
    let mut placement = affine_lattice::place_bounded(source, matrix, quantum, max_work)?;
    let Some(norm) = placement.operator_norm_upper else {
        out.reason = placement.reason;
        return Ok(out);
    };
    let Some(arithmetic) = placement.arithmetic_error_upper else {
        out.reason = placement.reason;
        return Ok(out);
    };
    if placement.model.is_none() {
        out.reason = placement.reason;
        return Ok(out);
    }
    let wall = before.wall_error_upper.and_then(|upper| {
        error_upper::multiply(norm, upper).and_then(|x| error_upper::add(x, arithmetic))
    });
    let caps = before.filled_cap_error_upper.and_then(|[a, b]| {
        Some([
            error_upper::add(error_upper::multiply(norm, a)?, arithmetic)?,
            error_upper::add(error_upper::multiply(norm, b)?, arithmetic)?,
        ])
    });
    let boundary = filled_cap_error::compose_boundary(wall, caps, premises.closed, budget);
    if !boundary.continuous_bound || boundary.within_budget != Some(true) {
        placement.model = None;
        out.reason = "boundary-budget-unproved";
        out.boundary = Some(boundary);
        out.placement = Some(placement);
        return Ok(out);
    }
    out.reason = if arithmetic == 0. {
        "exact-affine-complete-boundary"
    } else {
        "bounded-affine-complete-boundary"
    };
    out.boundary = Some(boundary);
    out.placement = Some(placement);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scales_complete_caps_and_refuses_source_budget_work_and_overflow_atomically() {
        let model = crate::cuboid([0., 0., 0.], [1., 1., 10.]).unwrap();
        let matrix = [
            [2., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];
        let p = Premises {
            wall: Some(0.1),
            caps: Some([0.2, 0.3]),
            closed: false,
            source_budget: Some(0.4),
        };
        let r = place(&model, &p, matrix, 0.125, 100000, Some(1.)).unwrap();
        let boundary = r.boundary.unwrap();
        assert!(r.placement.unwrap().model.is_some());
        assert!(boundary.error_upper.unwrap() >= 6_f64.sqrt() * 0.3);
        assert!(boundary.within_budget == Some(true));
        let denied = place(&model, &p, matrix, 0.125, 100000, Some(0.5)).unwrap();
        assert!(denied.placement.unwrap().model.is_none());
        assert!(denied.boundary.unwrap().within_budget == Some(false));
        let denied = place(&model, &p, matrix, 0.125, 0, Some(1.)).unwrap();
        assert!(denied.placement.is_none());
        let p = Premises { caps: None, ..p };
        assert!(
            place(&model, &p, matrix, 0.125, 100000, Some(1.))
                .unwrap()
                .placement
                .is_none()
        );
        let p = Premises {
            wall: Some(f64::MAX),
            caps: Some([0., 0.]),
            source_budget: Some(f64::MAX),
            closed: false,
        };
        let denied = place(&model, &p, matrix, 0.125, 100000, Some(f64::MAX)).unwrap();
        assert!(denied.placement.unwrap().model.is_none());
        assert!(!denied.boundary.unwrap().continuous_bound);
    }
}
