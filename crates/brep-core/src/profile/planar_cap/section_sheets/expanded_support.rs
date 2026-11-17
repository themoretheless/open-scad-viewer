use super::unresolved;
use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
use nurbs_core::{Result, analysis::exact_affine_hull::PlanarSurfaceProjection};

/// Each coordinate changes by a dyadic multiple of the odd denominator
/// of its slope. Its contribution to the omitted coordinate is then dyadic.
pub(super) fn expand(
    projection: &PlanarSurfaceProjection,
    anchor: [f64; 2],
    refinement: Option<usize>,
) -> Result<PlanarSurfaceProjection> {
    let [u, v] = [&projection.surface.knots_u, &projection.surface.knots_v];
    let [u0, u1] = enclose([u[0], u[3]], anchor[0], &projection.slopes[0], refinement)?;
    let [v0, v1] = enclose([v[0], v[3]], anchor[1], &projection.slopes[1], refinement)?;
    let mut expanded = projection.clone();
    expanded.surface.knots_u = vec![u0, u0, u1, u1];
    expanded.surface.knots_v = vec![v0, v0, v1, v1];
    expanded.surface.control_points = vec![
        vec![vec![u0, v0, 0.], vec![u0, v1, 0.]],
        vec![vec![u1, v0, 0.], vec![u1, v1, 0.]],
    ];
    Ok(expanded)
}

fn enclose(
    [lo, hi]: [f64; 2],
    anchor: f64,
    slope: &Q,
    refinement: Option<usize>,
) -> Result<[f64; 2]> {
    let denominator = slope.denom();
    let mut step = Q::from_integer(denominator >> denominator.trailing_zeros().unwrap() as usize);
    let lo = Q::from_float(lo).unwrap();
    let hi = Q::from_float(hi).unwrap();
    let width = &hi - &lo;
    if refinement.is_some() && width < step {
        let ratio = &step / width;
        let bits = (ratio.numer().bits() - ratio.denom().bits()) as usize;
        let mut divisor = BigInt::from(1) << bits;
        if Q::from_integer(divisor.clone()) < ratio {
            divisor <<= 1;
        }
        step /= divisor;
    }
    step /= BigInt::from(1) << refinement.unwrap_or(0);
    let anchor = Q::from_float(anchor).unwrap();
    let lower = ((lo - &anchor) / &step).floor();
    let upper = ((hi - &anchor) / &step).ceil();
    let store = |index: Q| {
        let bound = &anchor + &step * index;
        bound
            .to_f64()
            .filter(|value| Q::from_float(*value).as_ref() == Some(&bound))
            .ok_or_else(|| unresolved("Expanded cap chart endpoints cannot be stored exactly"))
    };
    Ok([store(lower)?, store(upper)?])
}
