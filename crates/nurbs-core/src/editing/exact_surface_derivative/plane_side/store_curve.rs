//! Store a certified planar section as a degree-one binary64 NURBS curve.
use super::{
    PResult, Q, Refusal, Result, Surface, SurfacePlaneWorldPolyline,
    SurfacePlaneWorldPolylineOptions, Work, check, samples::cell_domain, validate,
    world_polyline::build,
};
use crate::curve::Curve;
use num_traits::{Signed, ToPrimitive, Zero};

#[derive(Clone, Debug)]
pub struct StoredSurfacePlaneCurve {
    pub domain: [[f64; 2]; 2],
    /// Degree-one curve with exact integer knots. Divide its parameter by the
    /// segment count to recover the normalized transverse parameter.
    pub curve: Curve,
    /// Continuous error at the corresponding normalized transverse parameter.
    pub xyz_error_upper: Q,
    /// Continuous absolute bound for dot(normal, XYZ) - offset.
    pub plane_residual_upper: Q,
}

#[derive(Clone, Debug)]
pub struct StoredSurfacePlaneCurveReport {
    pub outcome: std::result::Result<StoredSurfacePlaneCurve, Refusal>,
    pub work_used: u64,
}

/// Project the rational polyline onto the exact cutting plane, then store
/// binary64 control points. Orthogonal projection is nonexpansive relative
/// to the true plane section. Exact rounding bounds are added to the
/// continuous modelling error before admitting the curve.
pub fn store_surface_plane_section_curve(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldPolylineOptions,
) -> Result<StoredSurfacePlaneCurveReport> {
    validate(surface, normal, offset, options.max_work)?;
    check(
        options.max_xyz_error.is_finite()
            && options.max_xyz_error > 0.
            && (1..=255).contains(&options.max_segments),
        "Section curve storage needs a positive XYZ tolerance and 1..255 segments",
    )?;
    let domain = cell_domain(surface, options.span)?;
    let mut work = Work {
        used: 0,
        limit: options.max_work,
    };
    let outcome = store(surface, normal, offset, options, domain, &mut work);
    Ok(StoredSurfacePlaneCurveReport {
        outcome,
        work_used: work.used,
    })
}

fn store(
    surface: &Surface,
    [x, y, z]: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldPolylineOptions,
    domain: [[f64; 2]; 2],
    work: &mut Work,
) -> PResult<StoredSurfacePlaneCurve> {
    let tolerance = work.lift(options.max_xyz_error)?;
    let half = work.op(&tolerance, &Q::from_integer(2.into()), |a, b| a / b)?;
    let world = build(surface, [x, y, z], offset, options, domain, &half, work)?;
    store_world(&world, [x, y, z], offset, &tolerance, work)
}

pub(super) fn store_world(
    world: &SurfacePlaneWorldPolyline,
    normal: [f64; 3],
    offset: f64,
    tolerance: &Q,
    work: &mut Work,
) -> PResult<StoredSurfacePlaneCurve> {
    store_world_vertices(
        &world.xyz,
        world.domain,
        &world.xyz_error_upper,
        normal,
        offset,
        tolerance,
        work,
    )
}

pub(super) fn store_world_vertices(
    xyz: &[[Q; 3]],
    domain: [[f64; 2]; 2],
    model_error: &Q,
    [x, y, z]: [f64; 3],
    offset: f64,
    tolerance: &Q,
    work: &mut Work,
) -> PResult<StoredSurfacePlaneCurve> {
    let normal = [work.lift(x)?, work.lift(y)?, work.lift(z)?];
    let offset = work.lift(offset)?;
    let norm_squared = dot(&normal, &normal, work)?;
    let mut points = Vec::with_capacity(xyz.len());
    let mut rounding = Q::zero();
    let mut plane_residual_upper = Q::zero();
    for vertex in xyz {
        let product = dot(&normal, vertex, work)?;
        let residual = work.op(&product, &offset, |a, b| a - b)?;
        let scale = work.op(&residual, &norm_squared, |a, b| a / b)?;
        let mut stored = [Q::zero(), Q::zero(), Q::zero()];
        let mut point = Vec::with_capacity(3);
        let mut total = Q::zero();
        for axis in 0..3 {
            let shift = work.op(&normal[axis], &scale, |a, b| a * b)?;
            let projected = work.op(&vertex[axis], &shift, |a, b| a - b)?;
            work.charge(1 + (projected.numer().bits() + projected.denom().bits()) / 64)?;
            let value = projected
                .to_f64()
                .filter(|x| x.is_finite())
                .ok_or(Refusal::NotRepresentable)?;
            stored[axis] = work.lift(value)?;
            let delta = work.op(&stored[axis], &projected, |a, b| (a - b).abs())?;
            total = work.op(&total, &delta, |a, b| a + b)?;
            point.push(value);
        }
        if points.last() == Some(&point) {
            return Err(Refusal::NotRepresentable);
        }
        rounding = rounding.max(total);
        let product = dot(&normal, &stored, work)?;
        let residual = work.op(&product, &offset, |a, b| (a - b).abs())?;
        plane_residual_upper = plane_residual_upper.max(residual);
        points.push(point);
    }
    let xyz_error_upper = work.op(model_error, &rounding, |a, b| a + b)?;
    if xyz_error_upper > *tolerance {
        return Err(Refusal::NotRepresentable);
    }
    let curve = Curve::from_polyline(points).map_err(|_| Refusal::InvalidOutput)?;
    Ok(StoredSurfacePlaneCurve {
        domain,
        curve,
        xyz_error_upper,
        plane_residual_upper,
    })
}

fn dot(a: &[Q; 3], b: &[Q; 3], work: &mut Work) -> PResult<Q> {
    let mut total = Q::zero();
    for (x, y) in a.iter().zip(b) {
        let term = work.op(x, y, |a, b| a * b)?;
        total = work.op(&total, &term, |a, b| a + b)?;
    }
    Ok(total)
}
