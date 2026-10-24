//! Split both uses of an interior mesh edge in one operation.
use super::*;
use nurbs_core::trim_domain::Location;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
pub struct AdaptiveInteriorLimits {
    pub max_splits: usize,
    pub controls: usize,
    pub spans: usize,
    pub spans_per_query: usize,
    pub target_mm: f64,
    pub trim_cells: usize,
    pub trim_cells_per_query: usize,
    pub tolerance_uv: f64,
}
pub struct InteriorBisectionReport {
    pub candidates: FaceCandidateReport,
    pub splits: usize,
    pub spans: usize,
    pub refused: usize,
    pub deviation_queries: usize,
    pub deviation_unproven: usize,
    pub trim_queries: usize,
    pub trim_cells: usize,
    pub trim_unproven: usize,
}
fn key(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}
fn children(
    t: &FaceTriangleCandidate,
    edge: (usize, usize),
    middle: usize,
    uv: [f64; 2],
) -> Option<[FaceTriangleCandidate; 2]> {
    for i in 0..3 {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        if key(t.vertices[i], t.vertices[j]) == edge {
            return Some([
                FaceTriangleCandidate {
                    face: t.face,
                    vertices: [t.vertices[i], middle, t.vertices[k]],
                    uv: [t.uv[i], uv, t.uv[k]],
                },
                FaceTriangleCandidate {
                    face: t.face,
                    vertices: [middle, t.vertices[j], t.vertices[k]],
                    uv: [uv, t.uv[j], t.uv[k]],
                },
            ]);
        }
    }
    None
}
fn orientation(uv: [[f64; 2]; 3]) -> Result<i8> {
    use nurbs_core::interval_eval::Interval as I;
    let delta = |a: f64, b: f64| I::point(a).sub(I::point(b));
    let d = delta(uv[1][0], uv[0][0])?
        .mul(delta(uv[2][1], uv[0][1])?)?
        .sub(delta(uv[1][1], uv[0][1])?.mul(delta(uv[2][0], uv[0][0])?)?)?;
    Ok(if d.lo > 0. {
        1
    } else if d.hi < 0. {
        -1
    } else {
        0
    })
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn bisect_candidate_interior_edges(
        &self,
        graph: &mut BoundaryGraphReport,
        candidates: FaceCandidateReport,
        max_splits: usize,
        controls: usize,
        max_spans: usize,
    ) -> Result<InteriorBisectionReport> {
        self.bisect_interior(
            graph, candidates, max_splits, controls, max_spans, None, None,
        )
    }
    /// Numerical approximation gate only; material trim remains independent.
    pub fn bisect_candidate_deviation_edges(
        &self,
        graph: &mut BoundaryGraphReport,
        candidates: FaceCandidateReport,
        max_splits: usize,
        controls: usize,
        max_spans: usize,
        target_mm: f64,
        spans_per_query: usize,
    ) -> Result<InteriorBisectionReport> {
        if !target_mm.is_finite() || target_mm <= 0. || spans_per_query > 100000 {
            return Err(Error::new(
                "BREP_INTERIOR_BISECTION",
                "Bound adaptive tolerance and query work",
            ));
        }
        self.bisect_interior(
            graph,
            candidates,
            max_splits,
            controls,
            max_spans,
            Some((target_mm, spans_per_query)),
            None,
        )
    }
    /// Refine when either approximation or exact material classification is
    /// unproven. Outside triangles are retained; this never drops a trim band.
    pub fn bisect_candidate_trimmed_edges(
        &self,
        graph: &mut BoundaryGraphReport,
        candidates: FaceCandidateReport,
        limits: AdaptiveInteriorLimits,
    ) -> Result<InteriorBisectionReport> {
        if !limits.target_mm.is_finite()
            || limits.target_mm <= 0.
            || limits.spans_per_query > 100000
            || limits.trim_cells > 1000000
            || limits.trim_cells_per_query > 100000
            || !limits.tolerance_uv.is_finite()
            || limits.tolerance_uv <= 0.
        {
            return Err(Error::new(
                "BREP_INTERIOR_BISECTION",
                "Bound trim and approximation work",
            ));
        }
        self.bisect_interior(
            graph,
            candidates,
            limits.max_splits,
            limits.controls,
            limits.spans,
            Some((limits.target_mm, limits.spans_per_query)),
            Some((
                limits.trim_cells,
                limits.trim_cells_per_query,
                limits.tolerance_uv,
            )),
        )
    }
    fn bisect_interior(
        &self,
        graph: &mut BoundaryGraphReport,
        mut candidates: FaceCandidateReport,
        max_splits: usize,
        controls: usize,
        max_spans: usize,
        deviation_gate: Option<(f64, usize)>,
        trim_gate: Option<(usize, usize, f64)>,
    ) -> Result<InteriorBisectionReport> {
        if max_splits > 100000 || controls > 1000000 || max_spans > 100000 {
            return Err(Error::new(
                "BREP_INTERIOR_BISECTION",
                "Bound interior subdivision work",
            ));
        }
        let boundary = graph
            .segments
            .iter()
            .map(|s| key(s.vertices[0], s.vertices[1]))
            .collect::<BTreeSet<_>>();
        let mut refused_edges: BTreeSet<(usize, usize)> = BTreeSet::new();
        let mut splits = 0;
        let mut spans = 0;
        let mut deviation_queries = 0;
        let mut deviation_unproven = 0;
        let mut trim_queries = 0;
        let mut trim_cells = 0;
        let mut trim_unproven = 0;
        let mut trim_cache = vec![None; candidates.triangles.len()];
        // Outer None means unexamined; inner None means numerically unproven.
        let mut error_cache = vec![None; candidates.triangles.len()];
        let mut incidence: BTreeMap<(usize, usize), BTreeSet<usize>> = BTreeMap::new();
        for (id, t) in candidates.triangles.iter().enumerate() {
            for i in 0..3 {
                incidence
                    .entry(key(t.vertices[i], t.vertices[(i + 1) % 3]))
                    .or_default()
                    .insert(id);
            }
        }
        // Nonnegative IEEE bits have the same order as their values. This is
        // only a subdivision priority, never a geometric error certificate.
        let priority = |g: &BoundaryGraphReport, e: (usize, usize)| {
            let a = g.vertices.get(e.0)?.point?;
            let b = g.vertices.get(e.1)?.point?;
            let length = (0..3).map(|k| (a[k] - b[k]).abs()).fold(0_f64, f64::max);
            Some((length.to_bits(), e))
        };
        let mut heap: BinaryHeap<_> = incidence
            .keys()
            .filter(|e| !boundary.contains(e))
            .filter_map(|&e| priority(graph, e))
            .collect();
        while splits < max_splits && spans < max_spans {
            let Some((_, edge)) = heap.pop() else {
                break;
            };
            if boundary.contains(&edge) || refused_edges.contains(&edge) {
                continue;
            }
            let Some(neighbors) = incidence.get(&edge) else {
                continue;
            };
            if neighbors.len() != 2 {
                continue;
            }
            let mut ids = neighbors.iter().copied();
            let uses = [ids.next().unwrap(), ids.next().unwrap()];
            if candidates.triangles[uses[0]].face != candidates.triangles[uses[1]].face {
                continue;
            }
            if let Some((target, per_query)) = deviation_gate {
                if per_query == 0 {
                    break;
                }
                for &id in &uses {
                    if error_cache[id].is_some() {
                        continue;
                    }
                    let allowance = per_query.min(max_spans - spans);
                    if allowance == 0 {
                        break;
                    }
                    let (error, used) = match self.bound_candidate_triangle(
                        &candidates.triangles[id],
                        graph,
                        controls,
                        allowance,
                    ) {
                        Ok(value) => value,
                        Err(error) if error.code == "NURBS_NUMERIC_ERROR" => (None, allowance),
                        Err(error) => return Err(error),
                    };
                    spans += used;
                    deviation_queries += 1;
                    deviation_unproven += usize::from(error.is_none());
                    error_cache[id] = Some(error);
                }
                if spans == max_spans {
                    break;
                }
                if let Some((total, per_query, tolerance)) = trim_gate {
                    for &id in &uses {
                        if trim_cache[id].is_some() {
                            continue;
                        }
                        let t = &candidates.triangles[id];
                        let domain = std::array::from_fn(|k| {
                            [
                                t.uv.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min),
                                t.uv.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max),
                            ]
                        });
                        let allowance = per_query.min(total - trim_cells);
                        let (location, used) =
                            match self.display_location(t.face, domain, tolerance, allowance) {
                                Ok(value) => value,
                                Err(error) if error.code == "NURBS_NUMERIC_ERROR" => {
                                    (Location::Unresolved, allowance)
                                }
                                Err(error) => return Err(error),
                            };
                        trim_cells += used;
                        trim_queries += 1;
                        trim_unproven += usize::from(location == Location::Unresolved);
                        trim_cache[id] = Some(location);
                    }
                }
                if uses.iter().all(|&id| {
                    error_cache[id].flatten().is_some_and(|e| e <= target)
                        && (trim_gate.is_none() || trim_cache[id] == Some(Location::Inside))
                }) {
                    continue;
                }
            }
            let t = &candidates.triangles[uses[0]];
            let coordinate = |t: &FaceTriangleCandidate, id: usize| {
                t.vertices.iter().position(|v| *v == id).map(|i| t.uv[i])
            };
            let a = coordinate(t, edge.0).unwrap();
            let b = coordinate(t, edge.1).unwrap();
            let other = &candidates.triangles[uses[1]];
            if coordinate(other, edge.0) != Some(a) || coordinate(other, edge.1) != Some(b) {
                refused_edges.insert(edge);
                continue;
            }
            let uv = std::array::from_fn(|k| a[k] * 0.5 + b[k] * 0.5);
            if uv == a || uv == b {
                refused_edges.insert(edge);
                continue;
            }
            let middle = graph.vertices.len();
            let first = children(t, edge, middle, uv).unwrap();
            let second = children(other, edge, middle, uv).unwrap();
            let parent_signs = [orientation(t.uv)?, orientation(other.uv)?];
            if parent_signs.contains(&0)
                || first
                    .iter()
                    .any(|c| orientation(c.uv).ok() != Some(parent_signs[0]))
                || second
                    .iter()
                    .any(|c| orientation(c.uv).ok() != Some(parent_signs[1]))
            {
                refused_edges.insert(edge);
                continue;
            }
            let allowance =
                deviation_gate.map_or(max_spans - spans, |(_, q)| q.min(max_spans - spans));
            let (bounds, spent) =
                match self.enclose_support(t.face, uv.map(|x| [x, x]), controls, allowance) {
                    Ok(value) => value,
                    Err(error) if error.code == "NURBS_NUMERIC_ERROR" => (None, allowance),
                    Err(error) => return Err(error),
                };
            spans += spent;
            let Some(bounds) = bounds else {
                refused_edges.insert(edge);
                continue;
            };
            graph.vertices.push(BoundaryGraphVertex {
                bounds: Some(bounds),
                point: Some(std::array::from_fn(|k| {
                    bounds[k][0] * 0.5 + bounds[k][1] * 0.5
                })),
            });
            // Remove exactly the replaced triangles before installing their
            // children. Unaffected adjacency and every CAD boundary ID survive.
            for &id in &uses {
                let old = &candidates.triangles[id];
                for i in 0..3 {
                    let e = key(old.vertices[i], old.vertices[(i + 1) % 3]);
                    if let Some(set) = incidence.get_mut(&e) {
                        set.remove(&id);
                    }
                }
            }
            let [a, b] = first;
            let [c, d] = second;
            candidates.triangles[uses[0]] = a;
            candidates.triangles[uses[1]] = c;
            let n = candidates.triangles.len();
            candidates.triangles.push(b);
            candidates.triangles.push(d);
            error_cache[uses[0]] = None;
            error_cache[uses[1]] = None;
            error_cache.extend([None, None]);
            trim_cache[uses[0]] = None;
            trim_cache[uses[1]] = None;
            trim_cache.extend([None, None]);
            for id in [uses[0], uses[1], n, n + 1] {
                let t = &candidates.triangles[id];
                for i in 0..3 {
                    let e = key(t.vertices[i], t.vertices[(i + 1) % 3]);
                    incidence.entry(e).or_default().insert(id);
                    if !boundary.contains(&e) {
                        if let Some(item) = priority(graph, e) {
                            heap.push(item);
                        }
                    }
                }
            }
            splits += 1;
        }
        Ok(InteriorBisectionReport {
            candidates,
            splits,
            spans,
            refused: refused_edges.len(),
            deviation_queries,
            deviation_unproven,
            trim_queries,
            trim_cells,
            trim_unproven,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_neighbors_reuse_one_midpoint_without_dropping_boundary_edges() {
        let a = FaceTriangleCandidate {
            face: 0,
            vertices: [0, 1, 2],
            uv: [[0., 0.], [1., 0.], [1., 1.]],
        };
        let b = FaceTriangleCandidate {
            face: 0,
            vertices: [0, 2, 3],
            uv: [[0., 0.], [1., 1.], [0., 1.]],
        };
        let mut out = children(&a, (0, 2), 4, [0.5, 0.5])
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        out.extend(children(&b, (0, 2), 4, [0.5, 0.5]).unwrap());
        let mut edges = BTreeMap::new();
        for t in &out {
            for i in 0..3 {
                *edges
                    .entry(key(t.vertices[i], t.vertices[(i + 1) % 3]))
                    .or_insert(0) += 1;
            }
        }
        for e in [(0, 1), (1, 2), (2, 3), (0, 3)] {
            assert_eq!(edges[&e], 1)
        }
        for e in [(0, 4), (2, 4)] {
            assert_eq!(edges[&e], 2)
        }
        assert!(!edges.contains_key(&(0, 2)));
        assert!(out.iter().all(|t| orientation(t.uv).unwrap() == 1));
        assert_eq!(orientation([[0., 0.], [1., 1.], [2., 2.]]).unwrap(), 0);
        assert_eq!(orientation([[0., 0.], [0., 1.], [1., 0.]]).unwrap(), -1);
    }
}
