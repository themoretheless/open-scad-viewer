//! C2 function approximation with a supplied mathematical enclosure contract.
use crate::{
    Result, check,
    distance_bounds::{Interval as I, box_distance},
    foundation::guards::{Budget, require_finite_point},
    surface::Surface,
};
#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub domain: [[f64; 2]; 2],
    /// Global Euclidean bounds ||S_uu|| and ||S_vv|| on the entire rectangle.
    pub second_derivative_bounds: [f64; 2],
    pub tolerance: f64,
    pub max_cells: usize,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub surface: Surface,
    pub interpolation_error_upper: f64,
    pub sample_error_upper: f64,
    pub error_upper: f64,
    pub within_tolerance: bool,
    pub cells: usize,
    pub evaluations: usize,
}
/// Why interpolation-grid refinement stopped. This is independent of the
/// final certificate, which also includes oracle enclosure and control rounding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridStop {
    InterpolationBoundReached,
    CellBudget,
    AxisResolutionLimit,
}
/// Why the final bound does or does not satisfy the requested tolerance.
/// These describe bounds, not proof that the true geometric error is too large.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CertificateOutcome {
    WithinTolerance,
    InterpolationBoundTooLarge,
    SampleBoundTooLarge,
    BothBoundsTooLarge,
    CombinedBoundTooLarge,
}
/// Additive diagnostic API; the original Report remains source-compatible.
#[derive(Clone, Debug)]
pub struct DetailedReport {
    pub report: Report,
    pub grid_stop: GridStop,
    pub certificate: CertificateOutcome,
}
fn grid(range: [f64; 2], n: usize) -> Result<Vec<f64>> {
    let points: Vec<_> = (0..=n)
        .map(|i| {
            if i == 0 {
                range[0]
            } else if i == n {
                range[1]
            } else {
                range[0] * (1. - i as f64 / n as f64) + range[1] * (i as f64 / n as f64)
            }
        })
        .collect();
    check(
        points.windows(2).all(|p| p[0] < p[1]),
        "Function surface grid reached binary64 precision limit",
    )?;
    Ok(points)
}
fn axis_error(points: &[f64], bound: f64) -> Result<f64> {
    let mut result = 0f64;
    for p in points.windows(2) {
        let h = I::point(p[1]).sub(I::point(p[0]))?;
        result = result.max(h.mul(h)?.mul(I::point(bound))?.div(I::point(8.))?.hi);
    }
    Ok(result.max(0.))
}
/// Approximate any C2 spatial function as a conforming piecewise bilinear NURBS.
/// Oracle intervals must enclose exact S at the requested binary64 parameters;
/// derivative bounds must hold globally. The certificate is conditional on
/// these explicit caller-supplied mathematical contracts, not inferred from
/// samples. Includes interval arithmetic and authored control rounding.
/// A mixed-derivative bound is unnecessary for tensor linear interpolation.
/// Budget exhaustion returns a valid surface with within_tolerance=false.
pub fn approximate<F>(request: Request, oracle: F) -> Result<Report>
where
    F: FnMut(f64, f64) -> Result<[[f64; 2]; 3]>,
{
    Ok(approximate_detailed(request, oracle)?.report)
}
/// Same geometry and bounds as `approximate`, with explicit refinement and
/// certificate outcomes. Invalid inputs/oracles remain errors, not outcomes.
pub fn approximate_detailed<F>(request: Request, mut oracle: F) -> Result<DetailedReport>
where
    F: FnMut(f64, f64) -> Result<[[f64; 2]; 3]>,
{
    require_finite_point(&request.domain[0], "function_surface_domain_u")?;
    require_finite_point(&request.domain[1], "function_surface_domain_v")?;
    require_finite_point(
        &request.second_derivative_bounds,
        "function_surface_second_derivative_bounds",
    )?;
    check(
        request.domain.iter().all(|r| r[0] < r[1])
            && request
                .second_derivative_bounds
                .iter()
                .all(|x| *x >= 0.)
            && request.tolerance.is_finite()
            && request.tolerance > 0.
            && (1..=256).contains(&request.max_cells),
        "Function surface requires increasing domain, nonnegative derivative bounds, positive tolerance and 1..256 cells",
    )?;
    let mut counts = [1usize; 2];
    // The grid-refinement loop strictly grows one axis per pass (doubling to
    // at most 16 per axis), so the unified budget mirrors the geometric cap.
    let mut refinement =
        Budget::with_iterations(request.max_cells.max(16))?.guard("function-surface-grid-refinement");
    let (u, v, interpolation_error_upper, grid_stop) = loop {
        refinement.tick()?;
        let u = grid(request.domain[0], counts[0])?;
        let v = grid(request.domain[1], counts[1])?;
        let errors = [
            axis_error(&u, request.second_derivative_bounds[0])?,
            axis_error(&v, request.second_derivative_bounds[1])?,
        ];
        let error = I::point(errors[0]).add(I::point(errors[1]))?.hi.max(0.);
        if error <= request.tolerance {
            break (u, v, error, GridStop::InterpolationBoundReached);
        }
        if counts[0] * counts[1] * 2 > request.max_cells {
            break (u, v, error, GridStop::CellBudget);
        }
        let mut axis = usize::from(errors[1] > errors[0]);
        if counts[axis] == 16 {
            axis = 1 - axis;
        }
        if counts[axis] == 16 {
            break (u, v, error, GridStop::AxisResolutionLimit);
        }
        counts[axis] *= 2;
    };
    let mut control_points = Vec::new();
    let mut sample_error_upper = 0f64;
    for &a in &u {
        let mut row = Vec::new();
        for &b in &v {
            let sample = oracle(a, b)?;
            let mut intervals = Vec::new();
            let mut point = Vec::new();
            for range in sample {
                let i = I::new(range[0], range[1])?;
                let center = (range[0] * 0.5 + range[1] * 0.5).clamp(range[0], range[1]);
                intervals.push(i);
                point.push(center);
            }
            let represented: Vec<_> = point.iter().copied().map(I::point).collect();
            sample_error_upper = sample_error_upper.max(box_distance(&intervals, &represented)?.1);
            row.push(point.try_into().unwrap());
        }
        control_points.push(row);
    }
    fn knots(points: &[f64]) -> Vec<f64> {
        std::iter::once(points[0])
            .chain(points.iter().copied())
            .chain(std::iter::once(*points.last().unwrap()))
            .collect()
    }
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: knots(&u),
        knots_v: knots(&v),
        control_points,
        weights: vec![vec![1.; v.len()]; u.len()],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let error_upper = I::point(interpolation_error_upper)
        .add(I::point(sample_error_upper))?
        .hi;
    let certificate = if error_upper <= request.tolerance {
        CertificateOutcome::WithinTolerance
    } else {
        match (
            interpolation_error_upper > request.tolerance,
            sample_error_upper > request.tolerance,
        ) {
            (true, true) => CertificateOutcome::BothBoundsTooLarge,
            (true, false) => CertificateOutcome::InterpolationBoundTooLarge,
            (false, true) => CertificateOutcome::SampleBoundTooLarge,
            (false, false) => CertificateOutcome::CombinedBoundTooLarge,
        }
    };
    Ok(DetailedReport {
        grid_stop,
        certificate,
        report: Report {
            surface,
            interpolation_error_upper,
            sample_error_upper,
            error_upper,
            within_tolerance: error_upper <= request.tolerance,
            cells: counts[0] * counts[1],
            evaluations: u.len() * v.len(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Request {
        Request {
            domain: [[0., 1.], [0., 1.]],
            second_derivative_bounds: [2., 2.],
            tolerance: 1e-2,
            max_cells: 256,
        }
    }

    fn paraboloid(_u: f64, _v: f64) -> Result<[[f64; 2]; 3]> {
        Ok([[0.; 2]; 3])
    }

    #[test]
    fn non_finite_domain_is_a_typed_boundary_error() {
        let mut invalid_domain = request();
        invalid_domain.domain[0][1] = f64::NAN;
        let err = approximate_detailed(invalid_domain, paraboloid).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("function_surface_domain_u[1]"), "{err}");
        let mut request = request();
        request.second_derivative_bounds[0] = f64::INFINITY;
        let err = approximate_detailed(request, paraboloid).unwrap_err();
        assert!(err.contains("function_surface_second_derivative_bounds[0]"), "{err}");
    }

    #[test]
    fn valid_request_refines_under_budget() {
        let report = approximate(request(), paraboloid).unwrap();
        assert!(report.within_tolerance);
        assert!(report.cells <= 256);
        report.surface.validate().unwrap();
    }
}
