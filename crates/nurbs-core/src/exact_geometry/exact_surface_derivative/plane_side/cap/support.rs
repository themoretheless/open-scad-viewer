//! Binary64 affine cap supports with exact residual bounds.
use super::{PResult, PlanarSurfaceProjection, Q, Refusal, Result, Surface, Work, check};
use num_traits::{Signed, ToPrimitive, Zero};

#[derive(Clone, Debug)]
pub struct BoundedPlanarCapSupport {
    pub surface: Surface,
    /// Continuous XYZ distance bound at the same UV point on the exact plane.
    pub error_upper: Q,
}

#[derive(Clone, Debug)]
pub struct BoundedPlanarCapSupportReport {
    pub outcome: std::result::Result<BoundedPlanarCapSupport, Refusal>,
    pub work_used: u64,
}

/// Lift an affine XY cap rectangle to a binary64 plane within tolerance.
/// Joint rounding enforces exact affine corner closure. The maximum corner
/// error bounds the entire stored surface in the original chart.
pub fn lift_planar_cap_support_bounded(
    projection: &PlanarSurfaceProjection,
    tolerance: f64,
    max_work: u64,
) -> Result<BoundedPlanarCapSupportReport> {
    projection.surface.validate()?;
    let [u, v] = projection.retained_axes;
    let k = projection.omitted_axis;
    check(
        u < 3 && v < 3 && k < 3 && u != v && u != k && v != k,
        "Invalid cap projection axes",
    )?;
    check(
        tolerance.is_finite()
            && tolerance >= 0.
            && (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Cap lift needs a finite nonnegative tolerance and bounded work",
    )?;
    let source = &projection.surface;
    let [u0, u1] = [
        source.knots_u[source.degree_u],
        source.knots_u[source.control_points.len()],
    ];
    let [v0, v1] = [
        source.knots_v[source.degree_v],
        source.knots_v[source.control_points[0].len()],
    ];
    check(
        source.degree_u == 1
            && source.degree_v == 1
            && source.control_points
                == vec![
                    vec![vec![u0, v0, 0.], vec![u0, v1, 0.]],
                    vec![vec![u1, v0, 0.], vec![u1, v1, 0.]],
                ]
            && source
                .weights
                .iter()
                .flatten()
                .all(|w| *w == source.weights[0][0]),
        "Bounded cap lift requires an identity affine XY rectangle",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = lift(projection, tolerance, &mut work);
    Ok(BoundedPlanarCapSupportReport {
        outcome,
        work_used: work.used,
    })
}

fn lift(
    projection: &PlanarSurfaceProjection,
    tolerance: f64,
    work: &mut Work,
) -> PResult<BoundedPlanarCapSupport> {
    let limit = work.lift(tolerance)?;
    let mut surface = projection.surface.clone();
    let mut exact = Vec::with_capacity(4);
    let mut rounded = Vec::with_capacity(4);
    let mut peak = 0_f64;
    for point in surface.control_points.iter().flatten() {
        let x = work.lift(point[0])?;
        let y = work.lift(point[1])?;
        let a = work.op(&x, &projection.slopes[0], |a, b| a * b)?;
        let b = work.op(&y, &projection.slopes[1], |a, b| a * b)?;
        let sum = work.op(&a, &b, |a, b| a + b)?;
        let z = work.op(&sum, &projection.offset, |a, b| a + b)?;
        let value = z
            .to_f64()
            .filter(|v| v.is_finite() && v.abs() <= 1e9)
            .ok_or(Refusal::NotRepresentable)?;
        peak = peak.max(value.abs());
        rounded.push(work.lift(value)?);
        exact.push(z);
    }
    if rounded[3] != complete(&rounded, work)? {
        // A common dyadic grid keeps all four selected coordinates storable
        // even when the nearest binary64 choices fail affine closure.
        let spacing = work.lift(4. * (peak.next_up() - peak))?;
        for i in 0..3 {
            let index = work.op(&exact[i], &spacing, |a, b| (a / b).round())?;
            rounded[i] = work.op(&index, &spacing, |a, b| a * b)?;
        }
        rounded[3] = complete(&rounded, work)?;
    }
    let mut error_upper = Q::zero();
    for (i, point) in surface.control_points.iter_mut().flatten().enumerate() {
        let error = work.op(&rounded[i], &exact[i], |a, b| (a - b).abs())?;
        error_upper = error_upper.max(error);
        let [a, b] = [point[0], point[1]];
        point[projection.retained_axes[0]] = a;
        point[projection.retained_axes[1]] = b;
        point[projection.omitted_axis] = work.float(&rounded[i])?;
    }
    if error_upper > limit {
        return Err(Refusal::NotRepresentable);
    }
    surface.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(BoundedPlanarCapSupport {
        surface,
        error_upper,
    })
}

fn complete(p: &[Q], work: &mut Work) -> PResult<Q> {
    let sum = work.op(&p[1], &p[2], |a, b| a + b)?;
    Ok(work.op(&sum, &p[0], |a, b| a - b)?)
}
