//! Exact affine dimension of a positive-weight tensor surface.
use crate::{
    Result, check,
    core::bounded_rational::{Error, Work},
    surface::Surface,
};
use num_rational::BigRational as Q;
use num_traits::Zero;

#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceAffineHull {
    Point([f64; 3]),
    Line([[f64; 3]; 2]),
    Plane([[f64; 3]; 3]),
    Spatial([[f64; 3]; 4]),
    ResourceLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceAffineHullReport {
    pub hull: SurfaceAffineHull,
    pub work_used: u64,
}

/// Classify the affine hull of the entire active surface. Witnesses are
/// authored active poles which span that hull, with no rounded normal or
/// fitted plane. Positive weights and local independence of the active
/// tensor basis make affine constraints on the image equivalent to those
/// on the active poles. Exterior poles whose support never reaches the
/// open active domain are excluded.
pub fn classify_surface_affine_hull(
    surface: &Surface,
    max_work: u64,
) -> Result<SurfaceAffineHullReport> {
    surface.validate()?;
    validate_budget(max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let hull = classify(surface, &mut work).unwrap_or(SurfaceAffineHull::ResourceLimit);
    Ok(SurfaceAffineHullReport {
        hull,
        work_used: work.used,
    })
}

fn validate_budget(max_work: u64) -> Result<()> {
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Affine hull work must be in [1, 1000000]",
    )
}

fn active(knots: &[f64], degree: usize, count: usize) -> impl Iterator<Item = usize> + Clone + '_ {
    let [a, b] = [knots[degree], knots[count]];
    (0..count).filter(move |&i| knots[i] < b && a < knots[i + degree + 1])
}

type Proof<T> = std::result::Result<T, Error>;

fn classify(surface: &Surface, work: &mut Work) -> Proof<SurfaceAffineHull> {
    let u = active(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
    );
    let v = active(
        &surface.knots_v,
        surface.degree_v,
        surface.control_points[0].len(),
    );
    let mut points = u.flat_map(|i| {
        v.clone().map(move |j| {
            let p = &surface.control_points[i][j];
            [p[0], p[1], p[2]]
        })
    });
    let origin = points.next().expect("validated surface has active poles");
    let origin_q = lift(origin, work)?;
    let mut line = None;
    let mut plane = None;
    for point in points {
        if point == origin {
            work.charge(1)?;
            continue;
        }
        let direction = delta(&lift(point, work)?, &origin_q, work)?;
        let Some((second, base)) = &line else {
            line = Some((point, direction));
            continue;
        };
        if let Some((third, normal)) = &plane {
            if !dot(normal, &direction, work)?.is_zero() {
                return Ok(SurfaceAffineHull::Spatial([origin, *second, *third, point]));
            }
        } else {
            let normal = cross(base, &direction, work)?;
            if normal.iter().any(|q| !q.is_zero()) {
                plane = Some((point, normal));
            }
        }
    }
    Ok(match (line, plane) {
        (Some((b, _)), Some((c, _))) => SurfaceAffineHull::Plane([origin, b, c]),
        (Some((b, _)), None) => SurfaceAffineHull::Line([origin, b]),
        _ => SurfaceAffineHull::Point(origin),
    })
}

fn lift(p: [f64; 3], w: &mut Work) -> Proof<[Q; 3]> {
    Ok([w.lift(p[0])?, w.lift(p[1])?, w.lift(p[2])?])
}

fn delta(a: &[Q; 3], b: &[Q; 3], w: &mut Work) -> Proof<[Q; 3]> {
    let mut result = std::array::from_fn(|_| Q::zero());
    for i in 0..3 {
        result[i] = w.op(&a[i], &b[i], |a, b| a - b)?;
    }
    Ok(result)
}

fn cross(a: &[Q; 3], b: &[Q; 3], w: &mut Work) -> Proof<[Q; 3]> {
    let mut result = std::array::from_fn(|_| Q::zero());
    for (k, (i, j)) in [(1, 2), (2, 0), (0, 1)].into_iter().enumerate() {
        let first = w.op(&a[i], &b[j], |a, b| a * b)?;
        let second = w.op(&a[j], &b[i], |a, b| a * b)?;
        result[k] = w.op(&first, &second, |a, b| a - b)?;
    }
    Ok(result)
}

fn dot(a: &[Q; 3], b: &[Q; 3], w: &mut Work) -> Proof<Q> {
    a.iter().zip(b).try_fold(Q::zero(), |sum, (a, b)| {
        let term = w.op(a, b, |a, b| a * b)?;
        w.op(&sum, &term, |a, b| a + b)
    })
}

/// A planar NURBS surface expressed in a Cartesian coordinate projection.
/// The projected surface uses XY with zero Z, with the original rational
/// basis. The omitted original coordinate is exactly
/// `offset + slopes[0]*x + slopes[1]*y`.
#[derive(Clone, Debug)]
pub struct PlanarSurfaceProjection {
    pub surface: Surface,
    pub retained_axes: [usize; 2],
    pub omitted_axis: usize,
    pub offset: Q,
    pub slopes: [Q; 2],
}

#[derive(Clone, Debug)]
pub enum PlanarProjectionOutcome {
    Projected(PlanarSurfaceProjection),
    /// The active affine hull has dimension other than two.
    NotPlane(SurfaceAffineHull),
    ResourceLimit,
}

#[derive(Clone, Debug)]
pub struct PlanarProjectionReport {
    pub outcome: PlanarProjectionOutcome,
    pub work_used: u64,
}

/// Drop the coordinate with largest exact absolute plane normal component.
/// This gives an invertible coordinate projection on the support plane
/// without rounding its inverse or changing the surface parameters.
/// Inactive exterior poles can be projected as-is; the inverse recovers
/// the entire active image and all active poles.
pub fn project_planar_surface(surface: &Surface, max_work: u64) -> Result<PlanarProjectionReport> {
    surface.validate()?;
    validate_budget(max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = project(surface, &mut work).unwrap_or(PlanarProjectionOutcome::ResourceLimit);
    Ok(PlanarProjectionReport {
        outcome,
        work_used: work.used,
    })
}

fn project(surface: &Surface, work: &mut Work) -> Proof<PlanarProjectionOutcome> {
    use num_traits::Signed;
    let hull = classify(surface, work)?;
    let SurfaceAffineHull::Plane([a, b, c]) = hull else {
        return Ok(PlanarProjectionOutcome::NotPlane(hull));
    };
    let origin = lift(a, work)?;
    let u = delta(&lift(b, work)?, &origin, work)?;
    let v = delta(&lift(c, work)?, &origin, work)?;
    let normal = cross(&u, &v, work)?;
    let mut omitted_axis = 0;
    for i in 1..3 {
        if normal[i].abs() > normal[omitted_axis].abs() {
            omitted_axis = i;
        }
    }
    let retained_axes = match omitted_axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    };
    let [i, j] = retained_axes;
    let n = &normal[omitted_axis];
    let plane_constant = dot(&normal, &origin, work)?;
    let offset = work.op(&plane_constant, n, |a, b| a / b)?;
    let slopes = [
        work.op(&normal[i], n, |a, b| -a / b)?,
        work.op(&normal[j], n, |a, b| -a / b)?,
    ];
    let mut projected = surface.clone();
    for pole in projected.control_points.iter_mut().flatten() {
        work.charge(1)?;
        let xy = [pole[i], pole[j]];
        pole.copy_from_slice(&[xy[0], xy[1], 0.]);
    }
    Ok(PlanarProjectionOutcome::Projected(
        PlanarSurfaceProjection {
            surface: projected,
            retained_axes,
            omitted_axis,
            offset,
            slopes,
        },
    ))
}
