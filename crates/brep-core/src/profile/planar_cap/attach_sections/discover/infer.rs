use super::{charge, collect_free_rings, invalid};
use crate::Model;
use num_rational::BigRational as Q;
use num_traits::Zero;
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
