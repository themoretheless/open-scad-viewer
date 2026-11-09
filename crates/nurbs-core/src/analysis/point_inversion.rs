//! Newton point inversion (The NURBS Book, A6.1) with both stopping tests.
//!
//! This is a local solver, complementary to the certified global projection and
//! trust-region APIs. A coarse bounded sampling pass chooses the seed. Newton
//! solves `f(u)=C'(u)·(C(u)-Q)=0` using
//! `f'(u)=C''(u)·(C(u)-Q)+C'(u)·C'(u)` and clamps every trial to the active
//! domain. Interior convergence requires both A6.1 criteria:
//!
//! 1. `distance = |C(u)-Q| <= epsilon1`;
//! 2. `orthogonality = |C'·(C-Q)|/(|C'| |C-Q|) <= epsilon2`.
//!
//! A zero-distance point is returned immediately because the cosine expression
//! is undefined at 0/0. The same holds when the distance satisfies criterion 1
//! and does not exceed the rounding error of the evaluated point and of the
//! query coordinates: such a residual carries no direction. A domain endpoint is a valid constrained minimum only
//! when the residual/tangent cone points outwards; the absolute cosine test is
//! not imposed at such an endpoint. Failure to converge within the explicit
//! budget is reported, not disguised as success.

use crate::{Result, check, curve::Curve, numeric, numeric_err};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointInversionOptions {
    /// Absolute Euclidean distance tolerance ε₁, in model units.
    pub epsilon1: f64,
    /// Dimensionless absolute cosine tolerance ε₂.
    pub epsilon2: f64,
    /// Number of equal parameter intervals in the initial sampling pass.
    pub samples: usize,
    pub max_iterations: usize,
}

impl Default for PointInversionOptions {
    fn default() -> Self {
        Self {
            epsilon1: 1e-9,
            epsilon2: 1e-9,
            samples: 64,
            max_iterations: 32,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InversionCriteria {
    pub distance: f64,
    pub orthogonality: Option<f64>,
    pub distance_satisfied: bool,
    pub orthogonality_satisfied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointInversionStop {
    CoincidentPoint,
    DistanceAndOrthogonality,
    BoundaryMinimum,
    IterationLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointInversionReport {
    pub parameter: f64,
    pub point: Vec<f64>,
    pub iterations: usize,
    pub criteria: InversionCriteria,
    pub stop: PointInversionStop,
    pub converged: bool,
}

fn criteria(evaluation: &crate::curve::Evaluation, query: &[f64]) -> Result<InversionCriteria> {
    let derivative = evaluation
        .d1
        .as_ref()
        .ok_or_else(|| numeric_err("Point inversion requires a first derivative"))?;
    let residual: Vec<f64> = evaluation
        .point
        .iter()
        .zip(query)
        .map(|(a, b)| a - b)
        .collect();
    let distance = residual.iter().map(|x| x * x).sum::<f64>().sqrt();
    let speed = derivative.iter().map(|x| x * x).sum::<f64>().sqrt();
    let dot = derivative
        .iter()
        .zip(&residual)
        .map(|(a, b)| a * b)
        .sum::<f64>();
    numeric(
        distance.is_finite() && speed.is_finite() && dot.is_finite(),
        "Point inversion criteria exhausted finite precision",
    )?;
    let orthogonality = if distance == 0. {
        None
    } else {
        numeric(
            speed > 0.,
            "Point inversion encountered a stationary tangent",
        )?;
        Some((dot / (speed * distance)).abs())
    };
    Ok(InversionCriteria {
        distance,
        orthogonality,
        distance_satisfied: false,
        orthogonality_satisfied: false,
    })
}

fn boundary_minimum(u: f64, domain: [f64; 2], e: &crate::curve::Evaluation, q: &[f64]) -> bool {
    let Some(d1) = &e.d1 else { return false };
    let directional = d1
        .iter()
        .zip(&e.point)
        .zip(q)
        .map(|((d, c), q)| d * (c - q))
        .sum::<f64>();
    (u == domain[0] && directional >= 0.) || (u == domain[1] && directional <= 0.)
}

/// Invert `query` near its closest sampled point. Both tolerances, the sampling
/// work and the Newton iteration budget are explicit and checked for NaN/Inf.
pub fn invert_point(
    curve: &Curve,
    query: &[f64],
    options: PointInversionOptions,
) -> Result<PointInversionReport> {
    curve.validate()?;
    check(
        query.len() == curve.control_points[0].len() && query.iter().all(|x| x.is_finite()),
        "Inversion query must match curve dimension and be finite",
    )?;
    check(
        options.epsilon1.is_finite() && options.epsilon1 > 0.,
        "Point inversion epsilon1 must be positive and finite",
    )?;
    check(
        options.epsilon2.is_finite() && options.epsilon2 > 0. && options.epsilon2 <= 1.,
        "Point inversion epsilon2 must be in (0, 1]",
    )?;
    check(
        (1..=4096).contains(&options.samples),
        "Point inversion samples must be in [1, 4096]",
    )?;
    check(
        (1..=256).contains(&options.max_iterations),
        "Point inversion iterations must be in [1, 256]",
    )?;
    let domain = curve.domain();
    let mut best_u = domain[0];
    let mut best_distance = f64::INFINITY;
    for i in 0..=options.samples {
        let u = domain[0] + (domain[1] - domain[0]) * i as f64 / options.samples as f64;
        let point = curve.evaluate_validated(u)?.point;
        let squared = point
            .iter()
            .zip(query)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>();
        numeric(
            squared.is_finite(),
            "Point inversion seed distance is not finite",
        )?;
        if squared < best_distance {
            best_distance = squared;
            best_u = u;
        }
    }
    let mut u = best_u;
    for iteration in 0..=options.max_iterations {
        let e = curve.evaluate_validated(u)?;
        let mut c = criteria(&e, query)?;
        c.distance_satisfied = c.distance <= options.epsilon1;
        c.orthogonality_satisfied = c.orthogonality.is_some_and(|v| v <= options.epsilon2);
        // Below the rounding error of the evaluated point and of the query
        // coordinates the residual has no significant direction, so its
        // cosine with the tangent is as undefined as at an exact zero.
        let noise = (query.len() as f64).sqrt()
            * (2. * e.rounding_bound
                + f64::EPSILON * query.iter().map(|x| x.abs()).fold(0., f64::max));
        let (stop, converged) = if c.distance == 0.
            || (c.distance_satisfied && c.distance <= noise)
        {
            (PointInversionStop::CoincidentPoint, true)
        } else if c.distance_satisfied && c.orthogonality_satisfied {
            (PointInversionStop::DistanceAndOrthogonality, true)
        } else if boundary_minimum(u, domain, &e, query) {
            (PointInversionStop::BoundaryMinimum, true)
        } else if iteration == options.max_iterations {
            (PointInversionStop::IterationLimit, false)
        } else {
            let d1 =
                e.d1.as_ref()
                    .ok_or_else(|| numeric_err("Point inversion requires a first derivative"))?;
            let d2 =
                e.d2.as_ref()
                    .ok_or_else(|| numeric_err("Point inversion requires a second derivative"))?;
            let residual: Vec<f64> = e.point.iter().zip(query).map(|(a, b)| a - b).collect();
            let f = d1.iter().zip(&residual).map(|(a, b)| a * b).sum::<f64>();
            let df = d2.iter().zip(&residual).map(|(a, b)| a * b).sum::<f64>()
                + d1.iter().map(|x| x * x).sum::<f64>();
            numeric(
                f.is_finite() && df.is_finite() && df.abs() > f64::MIN_POSITIVE,
                "Point inversion Newton derivative vanished or overflowed",
            )?;
            let next = (u - f / df).clamp(domain[0], domain[1]);
            numeric(
                next.is_finite(),
                "Point inversion Newton step is not finite",
            )?;
            u = next;
            continue;
        };
        return Ok(PointInversionReport {
            parameter: u,
            point: e.point,
            iterations: iteration,
            criteria: c,
            stop,
            converged,
        });
    }
    // The bounded loop always returns above; retain a defensive typed failure
    // rather than an unreachable panic if its structure changes later.
    Err(numeric_err("Point inversion exhausted its bounded loop"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quadratic() -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 2., 0.], vec![2., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        }
    }

    #[test]
    fn off_curve_minimum_requires_distance_and_orthogonality() {
        let curve = quadratic();
        let report = invert_point(
            &curve,
            &[1., 2.5, 0.],
            PointInversionOptions {
                epsilon1: 10., // Deliberately loose: distance alone must not stop Newton.
                epsilon2: 1e-12,
                samples: 1, // Deliberately poor seed at an endpoint.
                ..Default::default()
            },
        )
        .unwrap();
        assert!(report.converged, "{report:?}");
        assert_eq!(report.stop, PointInversionStop::DistanceAndOrthogonality);
        assert!(report.criteria.distance_satisfied);
        assert!(report.criteria.orthogonality_satisfied);
        assert!(report.criteria.orthogonality.unwrap() <= 1e-12);
        assert!(report.iterations > 0);
        assert!((report.parameter - 0.5).abs() < 1e-10, "{report:?}");
    }

    #[test]
    fn exact_curve_point_uses_distance_without_zero_over_zero_cosine() {
        let curve = quadratic();
        let q = curve.evaluate(0.3).unwrap().point;
        let report = invert_point(
            &curve,
            &q,
            PointInversionOptions {
                epsilon1: 1e-12,
                epsilon2: 1e-15,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(report.converged);
        assert_eq!(report.stop, PointInversionStop::CoincidentPoint);
        assert_eq!(report.criteria.orthogonality, None);
        assert_eq!(report.criteria.distance, 0.);
    }

    #[test]
    fn budgets_endpoints_and_nonfinite_inputs_are_explicit() {
        let curve = quadratic();
        let unfinished = invert_point(
            &curve,
            &[1., 2.5, 0.],
            PointInversionOptions {
                max_iterations: 1,
                samples: 1,
                epsilon1: 1e-15,
                epsilon2: 1e-15,
            },
        )
        .unwrap();
        assert!(!unfinished.converged);
        assert_eq!(unfinished.stop, PointInversionStop::IterationLimit);
        let boundary = invert_point(&curve, &[-1., 0., 0.], Default::default()).unwrap();
        assert_eq!(boundary.stop, PointInversionStop::BoundaryMinimum);
        assert_eq!(boundary.parameter, 0.);
        assert!(invert_point(&curve, &[f64::NAN, 0., 0.], Default::default()).is_err());
        assert!(
            invert_point(
                &curve,
                &[0., 0., 0.],
                PointInversionOptions {
                    epsilon2: f64::NAN,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
