use super::{refuse, spend, visit_polyhedral_boundary};
use crate::{Error, Result, closed_solid::QualifiedClosedSolid};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSegmentClip {
    /// Closed material intervals in the input segment parameter [0, 1].
    pub intervals: Vec<[Q; 2]>,
    pub exact_work: u64,
}

struct FaceBoundary {
    origin: [Q; 3],
    normal: Option<[Q; 3]>,
    segments: Vec<[[Q; 3]; 2]>,
}

/// Clip a nonzero spatial segment against a qualified polyhedral solid.
/// The full supporting line is traversed from outside to classify segments
/// whose endpoints are already inside material. Lines through a boundary
/// edge or vertex, or running along a retained face, are outside this
/// transverse mode.
pub fn exact_polyhedral_segment_clip(
    solid: &QualifiedClosedSolid,
    endpoints_mm: &[[Q; 3]; 2],
    max_exact_work: u64,
) -> Result<ExactPolyhedralSegmentClip> {
    let [start, end] = endpoints_mm;
    if endpoints_mm
        .iter()
        .flatten()
        .any(|v| v.numer().bits().max(v.denom().bits()) > 8192)
        || start == end
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Expected a nonzero clip segment with bounded coordinates",
        ));
    }
    let direction: [Q; 3] = std::array::from_fn(|i| &end[i] - &start[i]);
    let mut remaining = max_exact_work;
    let mut faces = BTreeMap::<usize, FaceBoundary>::new();
    visit_polyhedral_boundary(solid, &mut remaining, |face, origin, a, b, _, remaining| {
        spend(remaining, 24)?;
        let boundary = faces.entry(face).or_insert_with(|| FaceBoundary {
            origin: origin.clone(),
            normal: None,
            segments: Vec::new(),
        });
        if boundary.normal.is_none() {
            let u: [Q; 3] = std::array::from_fn(|i| &a[i] - &origin[i]);
            let v: [Q; 3] = std::array::from_fn(|i| &b[i] - &origin[i]);
            let normal = [(1, 2), (2, 0), (0, 1)].map(|(j, k)| &u[j] * &v[k] - &u[k] * &v[j]);
            if normal.iter().any(|value| !value.is_zero()) {
                boundary.normal = Some(normal);
            }
        }
        boundary.segments.push([a.clone(), b.clone()]);
        Ok(())
    })?;

    let mut crossings = Vec::new();
    for boundary in faces.into_values() {
        spend(&mut remaining, 24)?;
        let normal = boundary
            .normal
            .ok_or_else(|| refuse("Clip face has no planar boundary normal"))?;
        let denominator = dot(&normal, &direction);
        let delta = std::array::from_fn(|i| &boundary.origin[i] - &start[i]);
        let numerator = dot(&normal, &delta);
        let omitted = normal.iter().position(|value| !value.is_zero()).unwrap();
        let axes = [(omitted + 1) % 3, (omitted + 2) % 3];
        if denominator.is_zero() {
            if numerator.is_zero()
                && coplanar_line_hits_boundary(
                    &boundary.segments,
                    start,
                    &direction,
                    axes,
                    &mut remaining,
                )?
            {
                return Err(refuse("Clip line contacts a retained coplanar face"));
            }
            continue;
        }
        let t = numerator / denominator;
        let point = std::array::from_fn(|i| &start[i] + &t * &direction[i]);
        if inside_face(&boundary.segments, &point, axes, &mut remaining)? {
            crossings.push(t);
        }
    }
    let count = crossings.len();
    if count > 0 {
        spend(
            &mut remaining,
            count as u64 * (4 * u64::from(count.ilog2() + 1) + 5),
        )?;
    }
    crossings.sort_unstable();
    if count % 2 != 0 || crossings.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(refuse("Clip line crossings must be distinct and paired"));
    }
    let mut intervals = Vec::new();
    let mut crossings = crossings.into_iter();
    while let Some(lo) = crossings.next() {
        let hi = crossings.next().expect("even number of crossings");
        let lo = lo.max(Q::zero());
        let hi = hi.min(Q::one());
        if lo <= hi {
            intervals.push([lo, hi]);
        }
    }
    Ok(ExactPolyhedralSegmentClip {
        intervals,
        exact_work: max_exact_work - remaining,
    })
}

fn coplanar_line_hits_boundary(
    segments: &[[[Q; 3]; 2]],
    start: &[Q; 3],
    direction: &[Q; 3],
    [u, v]: [usize; 2],
    remaining: &mut u64,
) -> Result<bool> {
    let side =
        |p: &[Q; 3]| &direction[u] * (&p[v] - &start[v]) - &direction[v] * (&p[u] - &start[u]);
    // A full line cannot enter a bounded face without meeting its boundary.
    for [a, b] in segments {
        spend(remaining, 12)?;
        let a = side(a);
        let b = side(b);
        if a.is_zero() || b.is_zero() || a.is_negative() != b.is_negative() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn dot(a: &[Q; 3], b: &[Q; 3]) -> Q {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

fn inside_face(
    segments: &[[[Q; 3]; 2]],
    point: &[Q; 3],
    [u, v]: [usize; 2],
    remaining: &mut u64,
) -> Result<bool> {
    let [x, y] = [&point[u], &point[v]];
    let mut inside = false;
    for [a, b] in segments {
        spend(remaining, 12)?;
        let side = (&b[u] - &a[u]) * (y - &a[v]) - (&b[v] - &a[v]) * (x - &a[u]);
        if side.is_zero()
            && std::cmp::min(&a[u], &b[u]) <= x
            && x <= std::cmp::max(&a[u], &b[u])
            && std::cmp::min(&a[v], &b[v]) <= y
            && y <= std::cmp::max(&a[v], &b[v])
        {
            return Err(refuse("Clip line intersects a face boundary"));
        }
        if (&a[v] > y) != (&b[v] > y) && side.is_positive() == (b[v] > a[v]) {
            inside = !inside;
        }
    }
    Ok(inside)
}
