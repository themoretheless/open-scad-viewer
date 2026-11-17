use super::unresolved;
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
use nurbs_core::{Result, analysis::exact_affine_hull::PlanarSurfaceProjection};

/// Each coordinate changes by an integer multiple of the odd denominator
/// of its slope. Its contribution to the omitted coordinate is then dyadic.
pub(super) fn expand(
    projection: &PlanarSurfaceProjection,
    anchor: [f64; 2],
) -> Result<PlanarSurfaceProjection> {
    let [u, v] = [&projection.surface.knots_u, &projection.surface.knots_v];
    let [u0, u1] = enclose([u[0], u[3]], anchor[0], &projection.slopes[0])?;
    let [v0, v1] = enclose([v[0], v[3]], anchor[1], &projection.slopes[1])?;
    let mut expanded = projection.clone();
    expanded.surface.knots_u = vec![u0, u0, u1, u1];
    expanded.surface.knots_v = vec![v0, v0, v1, v1];
    expanded.surface.control_points = vec![
        vec![vec![u0, v0, 0.], vec![u0, v1, 0.]],
        vec![vec![u1, v0, 0.], vec![u1, v1, 0.]],
    ];
    Ok(expanded)
}

fn enclose([lo, hi]: [f64; 2], anchor: f64, slope: &Q) -> Result<[f64; 2]> {
    let denominator = slope.denom();
    let step = Q::from_integer(denominator >> denominator.trailing_zeros().unwrap() as usize);
    let anchor = Q::from_float(anchor).unwrap();
    let lower = ((Q::from_float(lo).unwrap() - &anchor) / &step).floor();
    let upper = ((Q::from_float(hi).unwrap() - &anchor) / &step).ceil();
    let store = |index: Q| {
        let bound = &anchor + &step * index;
        bound
            .to_f64()
            .filter(|value| Q::from_float(*value).as_ref() == Some(&bound))
            .ok_or_else(|| unresolved("Expanded cap chart endpoints cannot be stored exactly"))
    };
    Ok([store(lower)?, store(upper)?])
}
