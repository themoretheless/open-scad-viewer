use super::{
    PolyhedralSectionPointLocation, classify_section_loops, exact_polyhedral_section_loops,
    projected_axes, spend, validate_query_point,
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
    let [start, end] = endpoints_mm;
    validate_query_point(start)?;
    validate_query_point(end)?;
    let section = exact_polyhedral_section_loops(solid, normal, offset, max_exact_work)?;
    let mut remaining = max_exact_work - section.exact_work;
    spend(&mut remaining, 32)?;
    let n = super::super::lift(&normal);
    let offset = Q::from_float(offset).unwrap();
    for point in endpoints_mm {
        if n.iter().zip(point).map(|(n, p)| n * p).sum::<Q>() != offset {
            return Err(Error::new(
                "BREP_INVALID_INPUT",
                "Clip segment must lie in the section plane",
            ));
        }
    }

    let axes = projected_axes(normal);
    if start == end {
        let location = classify_section_loops(&section.loops, axes, start, &mut remaining)?;
        return Ok(ExactPolyhedralSectionClip {
            intervals: if is_material(location) {
                vec![[Q::zero(), Q::one()]]
            } else {
                Vec::new()
            },
            exact_work: max_exact_work - remaining,
        });
    }
    let origin = axes.map(|i| start[i].clone());
    let direction = axes.map(|i| &end[i] - &start[i]);
    let axis = usize::from(direction[0].is_zero());
    let zero = Q::zero();
    let one = Q::one();
    let mut boundary = BTreeSet::new();
    for ring in &section.loops {
        let points = &ring.points_mm;
        for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
            spend(
                &mut remaining,
                32 + 8 * u64::from((boundary.len() + 2).ilog2()),
            )?;
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
        spend(&mut remaining, 16)?;
        if boundary.contains(lo) {
            append_interval(&mut intervals, lo, lo);
        }
        let t = (lo + hi) / Q::from_integer(2.into());
        let sample = std::array::from_fn(|i| &start[i] + &t * (&end[i] - &start[i]));
        let location = classify_section_loops(&section.loops, axes, &sample, &mut remaining)?;
        if is_material(location) {
            append_interval(&mut intervals, lo, hi);
        }
    }
    if let Some(last) = partitions.last().filter(|t| boundary.contains(*t)) {
        append_interval(&mut intervals, last, last);
    }
    Ok(ExactPolyhedralSectionClip {
        intervals,
        exact_work: max_exact_work - remaining,
    })
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
