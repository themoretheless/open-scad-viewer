//! Exact contact classification for the analytic surface/surface matrix.
//!
//! Every quadric pair handled here reduces tangency to the sign of a
//! polynomial in the binary64 inputs (plane normal and offset, centres,
//! axis directions, radii). The polynomial is evaluated as an exact
//! floating-point expansion, so "tangent" means the discriminant of the input
//! data is exactly zero and "near tangent" is settled by its exact nonzero
//! sign: no distance tolerance participates in the decision. When an
//! expansion overflows or could lose bits to underflow the helpers return
//! `None` and the caller refuses with a typed reason.
//!
//! Only the contact class is certified. Points, radii and line positions of
//! the returned components are still rounded binary64 geometry.

use crate::exact_expansion::{self as ex, Expansion};

fn e(x: f64) -> Expansion {
    ex::grow(&[], x)
}
fn diff(a: f64, b: f64) -> Expansion {
    let (s, h) = ex::two_sum(a, -b);
    ex::grow(&e(h), s)
}
fn dot(a: &[Expansion; 3], b: &[Expansion; 3]) -> Expansion {
    (0..3).fold(Vec::new(), |acc, i| ex::add(&acc, &ex::mul(&a[i], &b[i])))
}
fn lift(a: [f64; 3]) -> [Expansion; 3] {
    a.map(e)
}
fn checked(x: Expansion) -> Option<Expansion> {
    ex::finite(&x).then_some(x)
}

/// Exact sign of the contact discriminant: positive separates, zero touches,
/// negative cuts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Apart,
    Touching,
    Cutting,
}
fn side(x: &[f64]) -> Side {
    match ex::sign(x) {
        1 => Side::Apart,
        0 => Side::Touching,
        _ => Side::Cutting,
    }
}

/// Round section of a plane `n.x = offset` (unnormalised) with a sphere or a
/// cylinder whose axis is exactly parallel to the plane: the sign of
/// `(n.c - offset)^2 - r^2 |n|^2` and, when cutting, the squared half-chord
/// `r^2 - dist^2` rounded from the exact value.
pub(super) fn plane_round(
    normal: [f64; 3],
    offset: f64,
    center: [f64; 3],
    radius: f64,
) -> Option<(Side, f64)> {
    let n = lift(normal);
    let s = checked(ex::sub(&dot(&n, &lift(center)), &e(offset)))?;
    let nn = checked(dot(&n, &n))?;
    let rr = checked(ex::scale(&e(radius), radius))?;
    let disc = checked(ex::sub(&ex::mul(&s, &s), &ex::mul(&rr, &nn)))?;
    let chord2 = -ex::approximate(&disc) / ex::approximate(&nn);
    Some((side(&disc), chord2))
}

/// Exactly zero `a . b`.
pub(super) fn exactly_orthogonal(a: [f64; 3], b: [f64; 3]) -> Option<bool> {
    Some(ex::sign(&checked(dot(&lift(a), &lift(b)))?) == 0)
}

/// Exactly zero `a x b`.
pub(super) fn exactly_parallel(a: [f64; 3], b: [f64; 3]) -> Option<bool> {
    for (i, j) in [(1, 2), (2, 0), (0, 1)] {
        let c = checked(ex::sub(&ex::scale(&e(a[i]), b[j]), &ex::scale(&e(a[j]), b[i])))?;
        if ex::sign(&c) != 0 {
            return Some(false);
        }
    }
    Some(true)
}

/// Two round sections at squared centre distance `q` (times a positive
/// factor `k`): whether the centres coincide exactly, and the exact sides
/// for external contact `q - (ra + rb)^2 k` and internal contact
/// `q - (ra - rb)^2 k`. Internal `Apart` means the circles cut or are
/// separated externally; internal `Cutting` means one strictly nests.
#[derive(Clone, Copy, Debug)]
pub(super) struct PairSides {
    pub concentric: bool,
    pub external: Side,
    pub internal: Side,
}
fn pair_sides(q: Expansion, k: Expansion, ra: f64, rb: f64) -> Option<PairSides> {
    let ra2 = ex::scale(&e(ra), ra);
    let rb2 = ex::scale(&e(rb), rb);
    let cross2 = ex::scale(&ex::scale(&e(ra), rb), 2.);
    let base = checked(ex::sub(&q, &ex::mul(&ex::add(&ra2, &rb2), &k)))?;
    let mixed = checked(ex::mul(&cross2, &k))?;
    let external = checked(ex::sub(&base, &mixed))?;
    let internal = checked(ex::add(&base, &mixed))?;
    Some(PairSides {
        concentric: ex::sign(&q) == 0,
        external: side(&external),
        internal: side(&internal),
    })
}

/// Sphere/sphere contact sides from the binary64 centres and radii.
pub(super) fn sphere_pair(a: [f64; 3], ra: f64, b: [f64; 3], rb: f64) -> Option<PairSides> {
    let delta: [Expansion; 3] = [0, 1, 2].map(|i| diff(b[i], a[i]));
    let q = checked(dot(&delta, &delta))?;
    pair_sides(q, e(1.), ra, rb)
}

/// Contact sides of two cylinders whose axis directions are exactly
/// parallel: the squared axis separation is `|d|^2 - (d.u)^2 / |u|^2`, so
/// everything is scaled by `|u|^2`.
pub(super) fn parallel_cylinder_pair(
    a_origin: [f64; 3],
    direction: [f64; 3],
    ra: f64,
    b_origin: [f64; 3],
    rb: f64,
) -> Option<PairSides> {
    let delta: [Expansion; 3] = [0, 1, 2].map(|i| diff(b_origin[i], a_origin[i]));
    let u = lift(direction);
    let uu = checked(dot(&u, &u))?;
    let du = checked(dot(&delta, &u))?;
    let dd = checked(dot(&delta, &delta))?;
    let q = checked(ex::sub(&ex::mul(&dd, &uu), &ex::mul(&du, &du)))?;
    pair_sides(q, uu, ra, rb)
}
