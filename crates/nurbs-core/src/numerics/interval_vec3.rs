//! Shared outward-rounded interval vec3 helpers over
//! [`crate::distance_bounds::Interval`].
//!
//! Every operation propagates the interval arithmetic `Result` errors
//! unchanged — this layer adds no error messages of its own, so call-site
//! behavior is identical to the former per-file copies. Rounding-sensitive
//! variants are kept distinct (`dot` vs `dot_tight`); normalization variants
//! whose enclosures differ (`frame_certificate::length`) stay local to their
//! files.
use crate::Result;
use crate::distance_bounds::{Interval, box_distance};

/// Interval 3-vector.
pub(crate) type Vec3 = [Interval; 3];

/// Component-wise sum.
pub(crate) fn add(a: Vec3, b: Vec3) -> Result<Vec3> {
    Ok([a[0].add(b[0])?, a[1].add(b[1])?, a[2].add(b[2])?])
}

/// Component-wise difference.
pub(crate) fn sub(a: Vec3, b: Vec3) -> Result<Vec3> {
    Ok([a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?])
}

/// Component-wise product with a scalar interval.
pub(crate) fn scale(a: Vec3, b: Interval) -> Result<Vec3> {
    Ok([a[0].mul(b)?, a[1].mul(b)?, a[2].mul(b)?])
}

/// Component-wise quotient by a scalar interval separated above zero.
pub(crate) fn div(a: Vec3, b: Interval) -> Result<Vec3> {
    Ok([a[0].div(b)?, a[1].div(b)?, a[2].div(b)?])
}

/// Dot product accumulated from a zero interval; the zero addend widens the
/// enclosure by one outward rounding step per component.
pub(crate) fn dot(a: Vec3, b: Vec3) -> Result<Interval> {
    let mut out = Interval::point(0.);
    for k in 0..3 {
        out = out.add(a[k].mul(b[k])?)?;
    }
    Ok(out)
}

/// Dot product without the initial zero addend — a tighter enclosure than
/// [`dot`], matching the former certificate-file formulation.
pub(crate) fn dot_tight(a: Vec3, b: Vec3) -> Result<Interval> {
    a[0].mul(b[0])?.add(a[1].mul(b[1])?)?.add(a[2].mul(b[2])?)
}

/// Cross product.
pub(crate) fn cross(a: Vec3, b: Vec3) -> Result<Vec3> {
    let mut out = [Interval::point(0.); 3];
    for k in 0..3 {
        let i = (k + 1) % 3;
        let j = (k + 2) % 3;
        out[k] = a[i].mul(b[j])?.sub(a[j].mul(b[i])?)?;
    }
    Ok(out)
}

/// Euclidean norm enclosure via `box_distance` against the origin.
pub(crate) fn norm(v: Vec3) -> Result<Interval> {
    let (lo, hi) = box_distance(&v, &[Interval::point(0.); 3])?;
    Interval::new(lo, hi)
}
