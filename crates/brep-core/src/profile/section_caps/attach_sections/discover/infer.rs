use super::super::attach_section_caps;
use super::{
    AttachedSectionCaps, BoundedSectionAttachment, SectionAttachmentOptions,
    attach_section_caps_with_support_tolerance, charge, close_shell, collect_free_rings, invalid,
};
use crate::Model;
use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
use nurbs_core::Result;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct InferredSectionPlane {
    /// Exact equation `normal · point = offset`; first nonzero component is one.
    pub normal: [Q; 3],
    pub offset: Q,
    /// Entire source boundary cycles, as [loop, coedge] addresses.
    pub rings: Vec<Vec<[usize; 2]>>,
}

#[derive(Clone, Debug)]
pub struct InferredSectionPlanes {
    pub planes: Vec<InferredSectionPlane>,
    pub exact_work: u64,
}

/// Infer the exact plane of each free boundary cycle of an open shell.
/// Coplanar cycles are grouped for cap construction, including hole cycles.
/// Reject ambiguous line/point boundaries and nonplanar loops.
pub fn infer_section_planes(
    source: &Model,
    shell: usize,
    max_exact_work: u64,
) -> Result<InferredSectionPlanes> {
    let mut remaining = max_exact_work;
    let rings = collect_free_rings(source, shell, &mut remaining, |_, _| Ok(true))?;
    let mut groups = BTreeMap::<([Q; 3], Q), Vec<_>>::new();
    for ring in rings {
        let mut points = ring.iter().flat_map(|&[wire, index]| {
            let coedge = &source.loops[wire].coedges[index];
            source.edges[coedge.edge].curve.control_points.iter()
        });
        let origin = lift(points.next().ok_or_else(|| invalid("Empty section ring"))?);
        let mut line: Option<[Q; 3]> = None;
        let mut normal: Option<[Q; 3]> = None;
        for point in points {
            charge(&mut remaining, 16)?;
            let point = lift(point);
            let delta = std::array::from_fn(|i| &point[i] - &origin[i]);
            if let Some(n) = &normal {
                if !dot(n, &delta).is_zero() {
                    return Err(invalid("Free section cycle has nonplanar controls"));
                }
            } else if let Some(direction) = &line {
                let cross = [
                    &direction[1] * &delta[2] - &direction[2] * &delta[1],
                    &direction[2] * &delta[0] - &direction[0] * &delta[2],
                    &direction[0] * &delta[1] - &direction[1] * &delta[0],
                ];
                if cross.iter().any(|q| !q.is_zero()) {
                    normal = Some(cross);
                }
            } else if delta.iter().any(|q| !q.is_zero()) {
                line = Some(delta);
            }
        }
        let normal =
            normal.ok_or_else(|| invalid("Free section cycle does not determine a plane"))?;
        let pivot = normal.iter().find(|q| !q.is_zero()).unwrap().clone();
        charge(&mut remaining, 10)?;
        let normal = normal.map(|q| q / &pivot);
        let offset = dot(&normal, &origin);
        groups.entry((normal, offset)).or_default().push(ring);
    }
    Ok(InferredSectionPlanes {
        planes: groups
            .into_iter()
            .map(|((normal, offset), rings)| InferredSectionPlane {
                normal,
                offset,
                rings,
            })
            .collect(),
        exact_work: max_exact_work - remaining,
    })
}

fn lift(point: &[f64]) -> [Q; 3] {
    std::array::from_fn(|i| Q::from_float(point[i]).unwrap())
}

fn dot(a: &[Q; 3], b: &[Q; 3]) -> Q {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

/// Detect and cap all planar free rims under shared work limits. Refuses
/// any plane equation or support that cannot be stored exactly in binary64.
pub fn close_shell_with_inferred_section_caps(
    source: &Model,
    shell: usize,
    max_exact_work: u64,
    max_verification_cells: usize,
) -> Result<AttachedSectionCaps> {
    close_inferred(
        source,
        shell,
        max_exact_work,
        max_verification_cells,
        |current, rings, normal, offset, work, cells| {
            attach_section_caps(current, shell, rings, normal, offset, work, cells)
        },
    )
}

/// Infer exact rim planes, then cap with certified, exactly planar stored
/// supports. The support error limit is applied separately to each cap.
pub fn close_shell_with_bounded_inferred_section_caps(
    source: &Model,
    shell: usize,
    options: SectionAttachmentOptions,
) -> Result<BoundedSectionAttachment> {
    let mut support_error_upper = Vec::new();
    let attachment = close_inferred(
        source,
        shell,
        options.max_exact_work,
        options.max_verification_cells,
        |current, rings, normal, offset, work, cells| {
            let report = attach_section_caps_with_support_tolerance(
                current,
                shell,
                rings,
                normal,
                offset,
                SectionAttachmentOptions {
                    max_exact_work: work,
                    max_verification_cells: cells,
                    ..options
                },
            )?;
            support_error_upper.extend(report.support_error_upper);
            Ok(report.attachment)
        },
    )?;
    Ok(BoundedSectionAttachment {
        attachment,
        support_error_upper,
    })
}

pub(crate) fn close_inferred(
    source: &Model,
    shell: usize,
    max_exact_work: u64,
    max_verification_cells: usize,
    mut attach: impl FnMut(
        &Model,
        &[&[[usize; 2]]],
        [f64; 3],
        f64,
        u64,
        usize,
    ) -> Result<AttachedSectionCaps>,
) -> Result<AttachedSectionCaps> {
    let inferred = infer_section_planes(source, shell, max_exact_work)?;
    let mut result = close_shell(
        source,
        shell,
        &inferred.planes,
        max_exact_work - inferred.exact_work,
        max_verification_cells,
        |current, plane, mut work, cells| {
            let budget = work;
            let [x, y, z, offset] = store_equation(plane, &mut work)?;
            let rings: Vec<_> = plane.rings.iter().map(Vec::as_slice).collect();
            let attachment = attach(current, &rings, [x, y, z], offset, work, cells)?;
            Ok((attachment, budget - work))
        },
    )?;
    result.exact_work += inferred.exact_work;
    Ok(result)
}

fn store_equation(plane: &InferredSectionPlane, work: &mut u64) -> Result<[f64; 4]> {
    let coefficients = [
        &plane.normal[0],
        &plane.normal[1],
        &plane.normal[2],
        &plane.offset,
    ];
    let mut denominator = BigInt::from(1);
    for value in coefficients {
        let mut a = denominator.clone();
        let mut b = value.denom().clone();
        while !b.is_zero() {
            charge(work, 1)?;
            let rem = a % &b;
            a = b;
            b = rem;
        }
        charge(work, 2)?;
        denominator = (denominator / a) * value.denom();
    }
    charge(work, 8)?;
    let integers = coefficients.map(|q| q.numer() * (&denominator / q.denom()));
    let normal_bits = integers[..3].iter().map(BigInt::bits).max().unwrap();
    let scale = BigInt::from(1) << normal_bits.saturating_sub(1) as usize;
    let mut stored = [0.; 4];
    for (out, integer) in stored.iter_mut().zip(integers) {
        charge(work, 2)?;
        let q = Q::new(integer, scale.clone());
        *out = q
            .to_f64()
            .filter(|value| Q::from_float(*value).as_ref() == Some(&q))
            .ok_or_else(|| invalid("Inferred section plane cannot be stored exactly"))?;
    }
    Ok(stored)
}
