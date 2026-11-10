//! Exact original contact-vertex quotient across periodic chart shores.
use super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use std::{cmp::Ordering, sync::Arc};
#[derive(Clone, Copy, Debug)]
pub struct OriginalChartVertexLimits {
    pub algebraic: ParameterOrderLimits,
    pub comparisons: usize,
}
impl Default for OriginalChartVertexLimits {
    fn default() -> Self {
        Self {
            algebraic: ParameterOrderLimits::default(),
            comparisons: 32768,
        }
    }
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalChartVertices {
    graph: Arc<OwnedOriginalCarrierGraph>,
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    limits: OriginalChartVertexLimits,
    groups: Vec<Vec<usize>>,
    vertex_groups: Vec<usize>,
    upper_shifts: Vec<[bool; 2]>,
    comparisons: usize,
}
#[derive(Clone)]
struct Position {
    primitive: Vec<Q>,
    root: [Q; 2],
    coordinate: [[Vec<Q>; 2]; 2],
}
fn rational(point: [Q; 2]) -> Position {
    Position {
        primitive: vec![Q::zero(), Q::one()],
        root: [Q::zero(), Q::zero()],
        coordinate: point.map(|q| [vec![q], vec![Q::one()]]),
    }
}
fn position(graph: &OwnedOriginalCarrierGraph, vertex: &OriginalCarrierVertex) -> Result<Position> {
    match vertex {
        OriginalCarrierVertex::RationalEndpoint(p) => {
            Ok(rational([decode_ratio(&p[0])?, decode_ratio(&p[1])?]))
        }
        OriginalCarrierVertex::Contact(group) => {
            let a = graph.owner().atlas();
            let o = a.occurrences()[a.groups()[*group][0]];
            let e = &a.owners()[o.pair].nodes()[o.node].exact;
            Ok(Position {
                primitive: decode_polynomial(&e.primitive)?,
                root: [
                    decode_ratio(&e.isolating_interval[0])?,
                    decode_ratio(&e.isolating_interval[1])?,
                ],
                coordinate: [
                    [
                        decode_polynomial(&e.position[0][0])?,
                        decode_polynomial(&e.position[0][1])?,
                    ],
                    [
                        decode_polynomial(&e.position[1][0])?,
                        decode_polynomial(&e.position[1][1])?,
                    ],
                ],
            })
        }
    }
}
fn compare(
    a: &Position,
    b: &Position,
    axis: usize,
    limits: OriginalChartVertexLimits,
    used: &mut usize,
) -> Result<Ordering> {
    if *used >= limits.comparisons {
        return Err(refused(
            "Original chart vertex comparison allowance exhausted",
        ));
    }
    *used += 1;
    arithmetic::parameter_order::compare_rational(
        &a.primitive,
        [&a.coordinate[axis][0], &a.coordinate[axis][1]],
        &a.root,
        &b.primitive,
        [&b.coordinate[axis][0], &b.coordinate[axis][1]],
        &b.root,
        limits.algebraic.max_refinements,
        limits.algebraic.max_resultant_size,
    )
}
type Quotient = (Vec<Vec<usize>>, Vec<usize>, Vec<[bool; 2]>, usize);
fn derive(
    graph: &OwnedOriginalCarrierGraph,
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    limits: OriginalChartVertexLimits,
) -> Result<Quotient> {
    graph.reverify_source()?;
    if chart
        .iter()
        .any(|a| a.iter().any(|q| !q.is_finite()) || a[0] >= a[1])
        || limits.comparisons > 65536
        || limits.algebraic.max_refinements > 512
        || !(1..=128).contains(&limits.algebraic.max_resultant_size)
    {
        return Err(refused("Invalid original chart vertex domain or allowance"));
    }
    let domain = chart.map(|a| a.map(|q| Q::from_float(q).unwrap()));
    let mut positions = Vec::new();
    let mut shifts = Vec::new();
    let mut comparisons = 0;
    for vertex in graph.vertices() {
        let mut p = position(graph, vertex)?;
        let mut shift = [false; 2];
        for axis in 0..2 {
            let lo = rational([domain[axis][0].clone(), domain[axis][0].clone()]);
            let hi = rational([domain[axis][1].clone(), domain[axis][1].clone()]);
            let lower = compare(&p, &lo, axis, limits, &mut comparisons)?;
            let upper = compare(&p, &hi, axis, limits, &mut comparisons)?;
            if lower == Ordering::Less || upper == Ordering::Greater {
                return Err(refused(
                    "Original contact vertex lies outside the closed chart",
                ));
            }
            if wrapped[axis] && upper == Ordering::Equal {
                let period = &domain[axis][1] - &domain[axis][0];
                let d = p.coordinate[axis][1].clone();
                let n = &mut p.coordinate[axis][0];
                n.resize(n.len().max(d.len()), Q::zero());
                for (i, q) in d.iter().enumerate() {
                    n[i] -= &period * q;
                }
                shift[axis] = true;
            }
        }
        positions.push(p);
        shifts.push(shift);
    }
    let mut groups = Vec::<Vec<usize>>::new();
    for vertex in 0..positions.len() {
        let mut lo = 0;
        let mut hi = groups.len();
        let mut equal = None;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let representative = groups[mid][0];
            let first = compare(
                &positions[vertex],
                &positions[representative],
                0,
                limits,
                &mut comparisons,
            )?;
            let order = if first == Ordering::Equal {
                compare(
                    &positions[vertex],
                    &positions[representative],
                    1,
                    limits,
                    &mut comparisons,
                )?
            } else {
                first
            };
            match order {
                Ordering::Less => hi = mid,
                Ordering::Greater => lo = mid + 1,
                Ordering::Equal => {
                    equal = Some(mid);
                    break;
                }
            }
        }
        if let Some(i) = equal {
            groups[i].push(vertex);
        } else {
            groups.insert(lo, vec![vertex]);
        }
    }
    let mut vertex_groups = vec![0; positions.len()];
    for (group, vertices) in groups.iter().enumerate() {
        for &vertex in vertices {
            vertex_groups[vertex] = group;
        }
    }
    Ok((groups, vertex_groups, shifts, comparisons))
}
impl OwnedOriginalChartVertices {
    pub(super) fn chart(&self)->[[f64;2];2]{self.chart}
    pub(super) fn wrapped(&self)->[bool;2]{self.wrapped}
    pub fn graph(&self) -> &Arc<OwnedOriginalCarrierGraph> {
        &self.graph
    }
    pub fn groups(&self) -> &[Vec<usize>] {
        &self.groups
    }
    pub fn vertex_groups(&self) -> &[usize] {
        &self.vertex_groups
    }
    /// Actual original upper-shore coordinate translated by one exact period.
    pub fn upper_shifts(&self) -> &[[bool; 2]] {
        &self.upper_shifts
    }
    pub fn comparisons_used(&self) -> usize {
        self.comparisons
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (groups, vertex_groups, upper_shifts, comparisons) =
            derive(&self.graph, self.chart, self.wrapped, self.limits)?;
        if groups != self.groups
            || vertex_groups != self.vertex_groups
            || upper_shifts != self.upper_shifts
            || comparisons != self.comparisons
        {
            return Err(refused(
                "Original periodic vertex quotient or shore provenance changed",
            ));
        }
        Ok(())
    }
}
/// Exact original vertex identity modulo explicitly wrapped chart axes. Both
/// coordinates must agree algebraically after a proved upper-shore translation.
/// This proves vertex quotient only, not seam edge pairing, curve chart coverage,
/// surface agreement, manifold links or physical face admission.
pub fn own_original_chart_vertices(
    graph: Arc<OwnedOriginalCarrierGraph>,
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    limits: OriginalChartVertexLimits,
) -> Result<OwnedOriginalChartVertices> {
    let (groups, vertex_groups, upper_shifts, comparisons) =
        derive(&graph, chart, wrapped, limits)?;
    Ok(OwnedOriginalChartVertices {
        graph,
        chart,
        wrapped,
        limits,
        groups,
        vertex_groups,
        upper_shifts,
        comparisons,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_chart_vertex_replay_rejects_partition_shore_and_work_tampering() {
        let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        let sources = points
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect::<Vec<_>>();
        let atlas =
            Arc::new(assemble_original_contact_atlas(&sources, 1e-12, Default::default()).unwrap());
        let graph = Arc::new(
            own_original_carrier_graph(Arc::new(own_original_carriers(atlas).unwrap())).unwrap(),
        );
        let q = own_original_chart_vertices(
            graph,
            [[0., 1.], [0., 1.]],
            [true, false],
            Default::default(),
        )
        .unwrap();
        q.reverify_source().unwrap();
        let mut bad = q.clone();
        bad.groups[0].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = q.clone();
        bad.vertex_groups[0] = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = q.clone();
        bad.upper_shifts[0][0] = !bad.upper_shifts[0][0];
        assert!(bad.reverify_source().is_err());
        let mut bad = q.clone();
        bad.comparisons += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = q;
        bad.wrapped[0] = false;
        assert!(bad.reverify_source().is_err());
    }
}
