//! Continuous world/UV identity on an exactly affine support.
use super::*;
use cad_predicates::{
    AuthoredScalar, BezierIdentityDecision, Limits, PredicateContext, SourceArena,
    ToleranceContext, rational_bspline_trimmed_boundary_identity,
};
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;

pub(super) fn verify(
    world: &Curve,
    uv: &Curve,
    surface: &Surface,
    reversed: bool,
    max_work: u64,
) -> Result<Option<BezierIdentityDecision>> {
    verify_ranges(
        world,
        uv,
        surface,
        reversed,
        world.domain(),
        uv.domain(),
        max_work,
    )
}

pub(super) fn verify_ranges(
    world: &Curve,
    uv: &Curve,
    surface: &Surface,
    reversed: bool,
    world_range: [f64; 2],
    uv_range: [f64; 2],
    max_work: u64,
) -> Result<Option<BezierIdentityDecision>> {
    if [world, uv]
        .iter()
        .any(|c| c.degree == 0 || c.degree > 32 || c.control_points.len() > 33)
        || (world.control_points.len() == world.degree + 1
            && uv.control_points.len() == uv.degree + 1)
        || world.control_points[0].len() != 3
        || uv.control_points[0].len() != 2
    {
        return Ok(None);
    }
    if (world.periodic || uv.periodic)
        && (!world.periodic
            || !uv.periodic
            || !crate::curve_partition_agreement::verify_periodic_source_closure(world)?
            || !crate::curve_partition_agreement::verify_periodic_source_closure(uv)?)
    {
        return Ok(None);
    }
    let Some(affine) = crate::surface_injectivity::certify_affine_parameterization(surface) else {
        return Ok(None);
    };
    let domain = [
        [affine.knots_u[1], affine.knots_u[2]],
        [affine.knots_v[1], affine.knots_v[2]],
    ];
    if uv
        .control_points
        .iter()
        .any(|p| (0..2).any(|k| p[k] < domain[k][0] || p[k] > domain[k][1]))
    {
        return Ok(None);
    }
    let q = |v| Q::from_float(v).unwrap();
    let bits = |v: f64| AuthoredScalar::Binary64Bits(v.to_bits());
    let encoded = |v: Q| {
        if let Some(f) = v
            .to_f64()
            .filter(|&f| Q::from_float(f).as_ref() == Some(&v))
        {
            return Some(bits(f));
        }
        Some(AuthoredScalar::RationalConstant {
            numerator: v.numer().to_i64()?,
            denominator: v.denom().to_u64()?,
        })
    };
    let origin = &affine.control_points[0][0];
    // Zero-width spans at a periodic domain end leave inactive exterior
    // controls. Keep only the controls and knots supporting the natural
    // interval, whose exact endpoint limits were checked above.
    let active = |curve: &Curve| {
        let n = curve.control_points.len();
        if !curve.periodic {
            return 0..n;
        }
        let spans = (curve.degree..n).filter(|&i| curve.knots[i] < curve.knots[i + 1]);
        let first = spans.clone().next().unwrap();
        let last = spans.last().unwrap();
        first - curve.degree..last + 1
    };
    let wc = active(world);
    let uc = active(uv);
    let mut values = Vec::new();
    for (p, &w) in world.control_points[wc.clone()].iter().zip(&world.weights[wc.clone()]) {
        values.extend([p[0], p[1], p[2], w].map(bits));
    }
    let lifted_start = values.len();
    for (p, &w) in uv.control_points[uc.clone()].iter().zip(&uv.weights[uc.clone()]) {
        let t = [0, 1].map(|k| (q(p[k]) - q(domain[k][0])) / (q(domain[k][1]) - q(domain[k][0])));
        for k in 0..3 {
            let o = q(origin[k]);
            let u = q(affine.control_points[1][0][k]) - &o;
            let v = q(affine.control_points[0][1][k]) - &o;
            let Some(coordinate) = encoded(o + &t[0] * u + &t[1] * v) else {
                return Ok(None);
            };
            values.push(coordinate);
        }
        values.push(bits(w));
    }
    let world_knots = values.len();
    values.extend(world.knots[wc.start..wc.end + world.degree + 1].iter().copied().map(bits));
    let uv_knots = values.len();
    values.extend(uv.knots[uc.start..uc.end + uv.degree + 1].iter().copied().map(bits));
    let clock = values.len();
    // Supply the UV curve's original clock as the affine free coordinate.
    // The trimmed predicate then compares the two declared source intervals.
    values.extend(uv.domain().map(bits));
    values.extend(uv.domain().map(bits));
    values.extend(uv_range.map(bits));
    values.extend(world_range.map(bits));
    // Preserve the original chart and pcurve alongside the derived controls
    // in the predicate identity. Their coefficients have not been rounded.
    values.extend(uv.control_points.iter().flatten().copied().map(bits));
    values.extend(
        surface
            .control_points
            .iter()
            .flatten()
            .flatten()
            .copied()
            .map(bits),
    );
    values.extend(surface.weights.iter().flatten().copied().map(bits));
    values.extend(
        surface
            .knots_u
            .iter()
            .chain(&surface.knots_v)
            .copied()
            .map(bits),
    );
    let arena = SourceArena::authored("affine-bspline-lift", 1, values)
        .map_err(|e| crate::input(e.to_string()))?;
    let leaf = |at| arena.leaf(at).unwrap();
    let refs = |at, count| (at..at + count).map(leaf).collect::<Vec<_>>();
    let rows = |at, count| {
        (0..count)
            .map(|i| std::array::from_fn(|k| leaf(at + 4 * i + k)))
            .collect::<Vec<_>>()
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    rational_bspline_trimmed_boundary_identity(
        &mut ctx,
        [
            cad_predicates::SplineLeafRefs {
                controls: &rows(0, wc.len()),
                knots: &refs(world_knots, wc.len() + world.degree + 1),
                degree: world.degree,
            },
            cad_predicates::SplineLeafRefs {
                controls: &rows(lifted_start, uc.len()),
                knots: &refs(uv_knots, uc.len() + uv.degree + 1),
                degree: uv.degree,
            },
        ],
        [leaf(clock), leaf(clock + 1)],
        std::array::from_fn(|i| leaf(clock + 2 + i)),
        [leaf(clock + 6), leaf(clock + 7)],
        reversed,
    )
    .map(Some)
    .map_err(|e| crate::input(e.to_string()))
}
