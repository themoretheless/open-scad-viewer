use super::{
    PolyhedralSectionLoop, PolyhedralSectionPointLocation, classify_section_loops,
    exact_polyhedral_section_loops, projected_axes, spend, validate_query_point,
};
use crate::{Error, Result, closed_solid::QualifiedClosedSolid};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSectionClip {
    /// Closed intervals of the input segment parameter in [0, 1].
    /// Equal endpoints represent isolated contact.
    pub intervals: Vec<[Q; 2]>,
    pub exact_work: u64,
}

/// Clip a coplanar segment to the closed material of a strict transverse
/// section. Holes split intervals. Boundary overlaps and isolated touches
/// are included. The segment is parameterized from its first endpoint.
pub fn exact_polyhedral_section_clip(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    endpoints_mm: &[[Q; 3]; 2],
    max_exact_work: u64,
) -> Result<ExactPolyhedralSectionClip> {
    let (section, mut remaining) =
        prepare_clip(solid, normal, offset, endpoints_mm, max_exact_work)?;
    let [start, end] = endpoints_mm;
    let intervals = clip_segment(
        &section.loops,
        projected_axes(normal),
        start,
        end,
        &mut remaining,
    )?;
    Ok(ExactPolyhedralSectionClip {
        intervals,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSectionPolylineClip {
    /// One interval list per input span, in traversal order.
    /// Each interval uses the span's local parameter in [0, 1].
    pub span_intervals: Vec<Vec<[Q; 2]>>,
    pub exact_work: u64,
}

/// Clip a coplanar polyline against one shared transverse section.
/// Repeated vertices retain their zero-length span and local parameter.
pub fn exact_polyhedral_section_clip_polyline(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralSectionPolylineClip> {
    let (section, mut remaining) = prepare_clip(solid, normal, offset, points_mm, max_exact_work)?;
    let axes = projected_axes(normal);
    let span_intervals = points_mm
        .windows(2)
        .map(|pair| clip_segment(&section.loops, axes, &pair[0], &pair[1], &mut remaining))
        .collect::<Result<_>>()?;
    Ok(ExactPolyhedralSectionPolylineClip {
        span_intervals,
        exact_work: max_exact_work - remaining,
    })
}

fn prepare_clip(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    points: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<(super::ExactPolyhedralSectionLoops, u64)> {
    if points.len() < 2 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Clip path requires at least two vertices",
        ));
    }
    let section = exact_polyhedral_section_loops(solid, normal, offset, max_exact_work)?;
    let mut remaining = max_exact_work - section.exact_work;
    let n = super::super::lift(&normal);
    let offset = Q::from_float(offset).unwrap();
    for point in points {
        spend(&mut remaining, 16)?;
        validate_query_point(point)?;
        if n.iter().zip(point).map(|(n, p)| n * p).sum::<Q>() != offset {
            return Err(Error::new(
                "BREP_INVALID_INPUT",
                "Clip path must lie in the section plane",
            ));
        }
    }
    Ok((section, remaining))
}

fn clip_segment(
    loops: &[PolyhedralSectionLoop],
    axes: [usize; 2],
    start: &[Q; 3],
    end: &[Q; 3],
    remaining: &mut u64,
) -> Result<Vec<[Q; 2]>> {
    if start == end {
        let location = classify_section_loops(loops, axes, start, remaining)?;
        return Ok(if is_material(location) {
            vec![[Q::zero(), Q::one()]]
        } else {
            Vec::new()
        });
    }
    let origin = axes.map(|i| start[i].clone());
    let direction = axes.map(|i| &end[i] - &start[i]);
    let axis = usize::from(direction[0].is_zero());
    let zero = Q::zero();
    let one = Q::one();
    let mut boundary = BTreeSet::new();
    for ring in loops {
        let points = &ring.points_mm;
        for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
            spend(remaining, 32 + 8 * u64::from((boundary.len() + 2).ilog2()))?;
            let relative = axes.map(|i| &a[i] - &start[i]);
            let edge = axes.map(|i| &b[i] - &a[i]);
            let denominator = cross(&direction, &edge);
            if !denominator.is_zero() {
                let t = cross(&relative, &edge) / &denominator;
                let u = cross(&relative, &direction) / denominator;
                if zero <= t && t <= one && zero <= u && u <= one {
                    boundary.insert(t);
                }
            } else if cross(&relative, &direction).is_zero() {
                let t = (&a[axes[axis]] - &origin[axis]) / &direction[axis];
                let u = (&b[axes[axis]] - &origin[axis]) / &direction[axis];
                let (lo, hi) = if t <= u { (t, u) } else { (u, t) };
                let lo = lo.max(zero.clone());
                let hi = hi.min(one.clone());
                if lo <= hi {
                    boundary.insert(lo);
                    boundary.insert(hi);
                }
            }
        }
    }
    let mut partitions: BTreeSet<_> = boundary.clone();
    partitions.insert(zero);
    partitions.insert(one);
    let mut intervals: Vec<[Q; 2]> = Vec::new();
    for (lo, hi) in partitions.iter().zip(partitions.iter().skip(1)) {
        spend(remaining, 16)?;
        if boundary.contains(lo) {
            append_interval(&mut intervals, lo, lo);
        }
        let t = (lo + hi) / Q::from_integer(2.into());
        let sample = std::array::from_fn(|i| &start[i] + &t * (&end[i] - &start[i]));
        let location = classify_section_loops(loops, axes, &sample, remaining)?;
        if is_material(location) {
            append_interval(&mut intervals, lo, hi);
        }
    }
    if let Some(last) = partitions.last().filter(|t| boundary.contains(*t)) {
        append_interval(&mut intervals, last, last);
    }
    Ok(intervals)
}

fn append_interval(intervals: &mut Vec<[Q; 2]>, lo: &Q, hi: &Q) {
    if let Some(last) = intervals.last_mut().filter(|last| last[1] == *lo) {
        last[1] = hi.clone();
    } else {
        intervals.push([lo.clone(), hi.clone()]);
    }
}

fn cross(a: &[Q; 2], b: &[Q; 2]) -> Q {
    &a[0] * &b[1] - &a[1] * &b[0]
}

fn is_material(location: PolyhedralSectionPointLocation) -> bool {
    matches!(
        location,
        PolyhedralSectionPointLocation::Material | PolyhedralSectionPointLocation::Boundary { .. }
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedPolyhedralSectionPathLength {
    /// Outward rational bounds on the total length inside closed material.
    pub bounds_mm: [Q; 2],
    pub exact_work: u64,
}

/// Measure the length of a planar polyline inside a strict transverse section.
/// Boundary spans have length, isolated touches have zero length, and repeated
/// traversals contribute once per traversal.
pub fn bounded_polyhedral_section_path_length(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    points_mm: &[[Q; 3]],
    max_width_mm: f64,
    max_exact_work: u64,
) -> Result<BoundedPolyhedralSectionPathLength> {
    let width = super::super::enclosure_width(max_width_mm)?;
    let clip =
        exact_polyhedral_section_clip_polyline(solid, normal, offset, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - clip.exact_work;
    let mut squares = Vec::new();
    for (span, intervals) in points_mm.windows(2).zip(clip.span_intervals) {
        spend(&mut remaining, 1 + 2 * intervals.len() as u64)?;
        let covered: Q = intervals.into_iter().map(|[lo, hi]| hi - lo).sum();
        if covered.is_zero() || span[0] == span[1] {
            continue;
        }
        spend(&mut remaining, 12)?;
        let span_squared: Q = span[0]
            .iter()
            .zip(&span[1])
            .map(|(a, b)| (b - a).pow(2))
            .sum();
        squares.push(covered.pow(2) * span_squared);
    }
    let bounds_mm = super::super::bound_root_sum(squares, width, &mut remaining)?;
    Ok(BoundedPolyhedralSectionPathLength {
        bounds_mm,
        exact_work: max_exact_work - remaining,
    })
}
