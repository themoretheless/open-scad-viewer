//! Interior refinement preserves every shared boundary constraint. Exact trim
//! bands that cannot be admitted remain pending, with their triangles intact.
use super::*;
use nurbs_core::{interval_eval::Interval as I, trim_domain::Location};
use std::collections::{BTreeMap, VecDeque};
#[derive(Clone, Copy)]
pub struct TriangleRefinementLimits {
    pub max_deviation_mm: f64,
    pub cells: usize,
    pub spans: usize,
    /// Per native query allowance; exceptions charge only this issued budget.
    pub spans_per_query: usize,
    pub controls: usize,
    pub domain_cells: usize,
    pub domain_cells_per_triangle: usize,
    pub tolerance_uv: f64,
    pub max_depth: usize,
}
pub struct RefinedTriangle {
    pub triangle: FaceTriangleCandidate,
    pub deviation_upper_mm: f64,
}
pub struct TriangleRefinementPending {
    pub triangle: FaceTriangleCandidate,
    pub reason: &'static str,
}
pub struct TriangleNumericFailure {
    pub face: usize,
    pub vertices: [usize; 3],
    pub uv: [[f64; 2]; 3],
    pub stage: &'static str,
    pub code: &'static str,
    pub message: String,
}
pub struct TriangleRefinementReport {
    pub limits: TriangleRefinementLimits,
    pub accepted: Vec<RefinedTriangle>,
    pub pending: Vec<TriangleRefinementPending>,
    pub pending_faces: Vec<FaceCandidatePending>,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub initial_triangles: usize,
    pub initial_checked: usize,
    pub initial_domain_cells: usize,
    pub trim_inherited: usize,
    /// Exact same-face UV rectangle proofs reused within this immutable call.
    pub trim_cache_hits: usize,
    pub trim_triangle_queries: usize,
    pub trim_triangle_proven: usize,
    pub contour_enclosure_queries: usize,
    pub contour_enclosure_cache_hits: usize,
    pub numeric_failures: Vec<TriangleNumericFailure>,
    pub split_triangles: usize,
}
/// Report each exhausted resource explicitly; simultaneous limits remain visible.
fn exhausted_work_reason(cells: bool, spans: bool, trim: bool) -> &'static str {
    match (cells, spans, trim) {
        (true, true, true) => "triangle-cell-span-trim-work-limit",
        (true, true, false) => "triangle-cell-span-work-limit",
        (true, false, true) => "triangle-cell-trim-work-limit",
        (false, true, true) => "triangle-span-trim-work-limit",
        (true, false, false) => "triangle-cell-work-limit",
        (false, true, false) => "triangle-span-work-limit",
        (false, false, true) => "triangle-trim-work-limit",
        (false, false, false) => "triangle-query-disabled",
    }
}
pub struct RegionalTransitionReport {
    pub max_spans: usize,
    pub spans: usize,
    pub queries: usize,
    pub accepted: usize,
}
fn domain(t: &FaceTriangleCandidate) -> [[f64; 2]; 2] {
    std::array::from_fn(|k| {
        [
            t.uv.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min),
            t.uv.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max),
        ]
    })
}
/// Visit every face before returning to its next candidate. No triangle is
/// discarded; the immutable source order within each face is retained.
fn fair_face_order(triangles: &[FaceTriangleCandidate]) -> Vec<usize> {
    let mut faces: BTreeMap<usize, VecDeque<usize>> = BTreeMap::new();
    for (index, triangle) in triangles.iter().enumerate() {
        faces.entry(triangle.face).or_default().push_back(index);
    }
    let mut order = Vec::with_capacity(triangles.len());
    while order.len() < triangles.len() {
        for queue in faces.values_mut() {
            if let Some(index) = queue.pop_front() {
                order.push(index);
            }
        }
    }
    order
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Additional explicit work, retaining every existing leaf and mesh vertex.
    /// Admission changes are committed only after all issued queries succeed.
    pub fn qualify_pending_transition_regions(
        &self,
        graph: &BoundaryGraphReport,
        report: &mut TriangleRefinementReport,
        max_spans: usize,
    ) -> Result<RegionalTransitionReport> {
        if max_spans > 100000 {
            return Err(Error::new(
                "BREP_REGIONAL_TRANSITION",
                "Bound additional native span work",
            ));
        }
        let mut work = RegionalTransitionReport {
            max_spans,
            spans: 0,
            queries: 0,
            accepted: 0,
        };
        let mut admitted = BTreeMap::new();
        for (index, pending) in report.pending.iter().enumerate() {
            if work.spans == max_spans {
                break;
            }
            let triangle = &pending.triangle;
            let Some(face) = self.faces.get(triangle.face) else {
                return Err(Error::new(
                    "BREP_REGIONAL_TRANSITION",
                    "Use owned expression faces",
                ));
            };
            if face.kind() != ShellFaceKind::Transition {
                continue;
            }
            // Current transition construction owns its complete chart. Ask its
            // exact trim classifier independently rather than inferring material
            // from a successful support-error bound.
            let (location, _, _, _, _) = self.display_triangle_location(
                triangle.face,
                triangle.uv,
                report.limits.tolerance_uv,
                0,
                None,
            )?;
            if location != Location::Inside {
                continue;
            }
            work.queries += 1;
            let (bound, spent) = self.bound_candidate_triangle_to_target(
                triangle,
                graph,
                report.limits.controls,
                2048.min(max_spans - work.spans),
                2,
                report.limits.max_deviation_mm,
            )?;
            work.spans += spent;
            if let Some(bound) = bound.filter(|bound| *bound <= report.limits.max_deviation_mm) {
                admitted.insert(index, bound);
            }
        }
        let mut index = 0;
        report.pending.retain(|pending| {
            let keep = if let Some(&bound) = admitted.get(&index) {
                report.accepted.push(RefinedTriangle {
                    triangle: FaceTriangleCandidate {
                        face: pending.triangle.face,
                        vertices: pending.triangle.vertices,
                        uv: pending.triangle.uv,
                    },
                    deviation_upper_mm: bound,
                });
                false
            } else {
                true
            };
            index += 1;
            keep
        });
        work.accepted = admitted.len();
        Ok(work)
    }
    /// Mutates only the diagnostic graph by adding face-interior vertices.
    pub fn refine_face_candidates(
        &self,
        graph: &mut BoundaryGraphReport,
        candidates: &FaceCandidateReport,
        limits: TriangleRefinementLimits,
    ) -> Result<TriangleRefinementReport> {
        if !limits.max_deviation_mm.is_finite()
            || limits.max_deviation_mm <= 0.
            || !limits.tolerance_uv.is_finite()
            || limits.tolerance_uv <= 0.
            || limits.cells > 100000
            || limits.spans > 100000
            || limits.spans_per_query > 100000
            || limits.controls > 1000000
            || limits.domain_cells > 1000000
            || limits.domain_cells_per_triangle > 100000
            || limits.max_depth > 32
        {
            return Err(Error::new(
                "BREP_TRIANGLE_REFINEMENT",
                "Choose finite tolerance and bounded native work",
            ));
        }
        let mut report = TriangleRefinementReport {
            limits,
            accepted: vec![],
            pending: vec![],
            pending_faces: candidates
                .pending
                .iter()
                .map(|p| FaceCandidatePending {
                    face: p.face,
                    reason: p.reason,
                })
                .collect(),
            cells: 0,
            spans: 0,
            domain_cells: 0,
            initial_triangles: candidates.triangles.len(),
            initial_checked: 0,
            initial_domain_cells: 0,
            trim_inherited: 0,
            trim_cache_hits: 0,
            trim_triangle_queries: 0,
            trim_triangle_proven: 0,
            contour_enclosure_queries: 0,
            contour_enclosure_cache_hits: 0,
            numeric_failures: vec![],
            split_triangles: 0,
        };
        let mut queue: VecDeque<_> = fair_face_order(&candidates.triangles)
            .into_iter()
            .map(|index| {
                let t = &candidates.triangles[index];
                (
                    FaceTriangleCandidate {
                        face: t.face,
                        vertices: t.vertices,
                        uv: t.uv,
                    },
                    0,
                    false,
                )
            })
            .collect();
        let mut deferred = VecDeque::new();
        let mut trim_proofs = BTreeMap::new();
        let mut prepared_contours = BTreeMap::new();
        loop {
            if let Some((triangle, depth, inherited_inside)) = queue.pop_front() {
                if report.cells == limits.cells
                    || report.spans == limits.spans
                    || limits.spans_per_query == 0
                {
                    report.pending.push(TriangleRefinementPending {
                        triangle,
                        reason: exhausted_work_reason(
                            report.cells == limits.cells,
                            report.spans == limits.spans,
                            false,
                        ),
                    });
                    continue;
                }
                report.cells += 1;
                report.initial_checked += usize::from(depth == 0);
                let uv_domain = domain(&triangle);
                let trim_key = (triangle.face, uv_domain.map(|axis| axis.map(f64::to_bits)));
                let (location, used, rectangle_proven) = if inherited_inside {
                    report.trim_inherited += 1;
                    (Location::Inside, 0, true)
                } else if let Some(location) = trim_proofs.get(&trim_key) {
                    report.trim_cache_hits += 1;
                    (*location, 0, true)
                } else {
                    report.trim_triangle_queries += 1;
                    if !prepared_contours.contains_key(&triangle.face) {
                        let loops: Option<Vec<Vec<_>>> = self.faces[triangle.face]
                            .loops
                            .iter()
                            .map(|wire| {
                                wire.iter()
                                    .map(|edge| match edge {
                                        ShellUse::Source(fragment) => Some(fragment),
                                        _ => None,
                                    })
                                    .collect()
                            })
                            .collect();
                        if let Some(loops) = loops {
                            let capacity = 100000usize / self.faces.len().max(1);
                            prepared_contours.insert(
                                triangle.face,
                                crate::source_contour_winding::Prepared::from_references(
                                    loops, capacity,
                                )?,
                            );
                        }
                    }

                    // Reserve a fair share for every original candidate before
                    // spending domain work on descendants. Zero-cost whole-chart
                    // or cached proofs leave their share available to later leaves.
                    let remaining_domain = limits.domain_cells - report.domain_cells;
                    let remaining_original =
                        candidates.triangles.len() - report.initial_checked + 1;
                    let domain_allowance = if depth == 0 {
                        remaining_domain / remaining_original
                    } else {
                        remaining_domain
                    };
                    let answer = self.display_triangle_location(
                        triangle.face,
                        triangle.uv,
                        limits.tolerance_uv,
                        limits.domain_cells_per_triangle.min(domain_allowance),
                        prepared_contours.get_mut(&triangle.face),
                    )?;
                    report.trim_triangle_proven += usize::from(answer.0 != Location::Unresolved);
                    report.contour_enclosure_queries += answer.3;
                    report.contour_enclosure_cache_hits += answer.4;
                    (answer.0, answer.1, answer.2)
                };
                report.domain_cells += used;
                report.initial_domain_cells += if depth == 0 { used } else { 0 };
                // Unresolved is a budget-dependent observation, never a proof.
                if rectangle_proven && location != Location::Unresolved {
                    trim_proofs.insert(trim_key, location);
                }
                let rectangle_inside = rectangle_proven && location == Location::Inside;
                if location == Location::Outside {
                    report.pending.push(TriangleRefinementPending {
                        triangle,
                        reason: "triangle-outside-material",
                    });
                    continue;
                }
                if location == Location::Unresolved && report.domain_cells == limits.domain_cells {
                    report.pending.push(TriangleRefinementPending {
                        triangle,
                        reason: "triangle-trim-work-limit",
                    });
                    continue;
                }
                let remaining = limits.spans_per_query.min(limits.spans - report.spans);
                let (error, spent) = match self.bound_candidate_triangle(
                    &triangle,
                    graph,
                    limits.controls,
                    remaining,
                ) {
                    Ok(value) => value,
                    Err(error) if error.code == "NURBS_NUMERIC_ERROR" => {
                        // The lower-level error carries no spent-work counter.
                        // Conservatively charge the entire issued allowance.
                        report.spans += remaining;
                        report.numeric_failures.push(TriangleNumericFailure {
                            face: triangle.face,
                            vertices: triangle.vertices,
                            uv: triangle.uv,
                            stage: "support-bound",
                            code: error.code,
                            message: error.message,
                        });
                        report.pending.push(TriangleRefinementPending {
                            triangle,
                            reason: "triangle-support-numeric-unproven",
                        });
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                report.spans += spent;
                if location == Location::Inside
                    && error.is_some_and(|e| e <= limits.max_deviation_mm)
                {
                    report.accepted.push(RefinedTriangle {
                        triangle,
                        deviation_upper_mm: error.unwrap(),
                    });
                    continue;
                }
                if depth == limits.max_depth {
                    report.pending.push(TriangleRefinementPending {
                        triangle,
                        reason: if location != Location::Inside {
                            "triangle-exact-trim-unproven"
                        } else {
                            "triangle-deviation-limit"
                        },
                    });
                    continue;
                }
                // Examine the entire current queue before issuing midpoint
                // queries. Original candidates cannot be starved by new vertices.
                deferred.push_back((triangle, depth, rectangle_inside));
                continue;
            }
            let Some((triangle, depth, inside)) = deferred.pop_front() else {
                break;
            };
            if report.cells == limits.cells
                || report.spans == limits.spans
                || limits.spans_per_query == 0
                || (!inside && report.domain_cells == limits.domain_cells)
            {
                report.pending.push(TriangleRefinementPending {
                    triangle,
                    reason: exhausted_work_reason(
                        report.cells == limits.cells,
                        report.spans == limits.spans,
                        !inside && report.domain_cells == limits.domain_cells,
                    ),
                });
                continue;
            }
            // The centroid lies strictly within a nondegenerate triangle. Splitting
            // three ways keeps all existing edges, so adjacent faces cannot gain a crack.
            let parent_domain = domain(&triangle);
            let center = std::array::from_fn(|k| {
                (triangle.uv[0][k] / 3. + triangle.uv[1][k] / 3. + triangle.uv[2][k] / 3.)
                    .clamp(parent_domain[k][0], parent_domain[k][1])
            });
            let a = triangle.uv[0];
            let b = triangle.uv[1];
            let c = triangle.uv[2];
            let orient = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| -> Result<I> {
                let sub = |x: f64, y: f64| I::point(x).sub(I::point(y));
                sub(b[0], a[0])?
                    .mul(sub(c[1], a[1])?)?
                    .sub(sub(b[1], a[1])?.mul(sub(c[0], a[0])?)?)
            };
            let determinants = [
                orient(a, b, center)?,
                orient(b, c, center)?,
                orient(c, a, center)?,
            ];
            if !(determinants.iter().all(|d| d.lo > 0.) || determinants.iter().all(|d| d.hi < 0.)) {
                report.pending.push(TriangleRefinementPending {
                    triangle,
                    reason: "triangle-parameter-resolution",
                });
                continue;
            }
            if report.spans == limits.spans {
                report.pending.push(TriangleRefinementPending {
                    triangle,
                    reason: "triangle-refinement-work-limit",
                });
                continue;
            }
            let allowance = limits.spans_per_query.min(limits.spans - report.spans);
            let (bounds, spent) = match self.enclose_support(
                triangle.face,
                center.map(|t| [t, t]),
                limits.controls,
                allowance,
            ) {
                Ok(value) => value,
                Err(error) if error.code == "NURBS_NUMERIC_ERROR" => {
                    report.spans += allowance;
                    report.numeric_failures.push(TriangleNumericFailure {
                        face: triangle.face,
                        vertices: triangle.vertices,
                        uv: triangle.uv,
                        stage: "centroid",
                        code: error.code,
                        message: error.message,
                    });
                    report.pending.push(TriangleRefinementPending {
                        triangle,
                        reason: "triangle-midpoint-numeric-unproven",
                    });
                    continue;
                }
                Err(error) => return Err(error),
            };
            report.spans += spent;
            let Some(bounds) = bounds else {
                report.pending.push(TriangleRefinementPending {
                    triangle,
                    reason: "triangle-refinement-work-limit",
                });
                continue;
            };
            let id = graph.vertices.len();
            graph.vertices.push(BoundaryGraphVertex {
                bounds: Some(bounds),
                point: Some(std::array::from_fn(|k| {
                    bounds[k][0] * 0.5 + bounds[k][1] * 0.5
                })),
            });
            report.split_triangles += 1;
            for i in 0..3 {
                let j = (i + 1) % 3;
                let child = FaceTriangleCandidate {
                    face: triangle.face,
                    vertices: [triangle.vertices[i], triangle.vertices[j], id],
                    uv: [triangle.uv[i], triangle.uv[j], center],
                };
                let child_domain = domain(&child);
                let inherits = inside
                    && (0..2).all(|k| {
                        child_domain[k][0] >= parent_domain[k][0]
                            && child_domain[k][1] <= parent_domain[k][1]
                    });
                queue.push_back((child, depth + 1, inherits));
            }
        }
        if report.accepted.len() + report.pending.len()
            != report.initial_triangles + 2 * report.split_triangles
        {
            return Err(Error::new(
                "BREP_TRIANGLE_REFINEMENT",
                "Refinement coverage accounting conflict",
            ));
        }
        Ok(report)
    }
}

impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn refine_boundary_face_triangles(
        &self,
        graph: &mut BoundaryGraphReport,
        limits: TriangleRefinementLimits,
    ) -> Result<TriangleRefinementReport> {
        let candidates = self.propose_boundary_face_triangles(graph);
        self.refine_face_candidates(graph, &candidates, limits)
    }
}

#[cfg(test)]
mod scheduling_tests {
    use super::*;
    #[test]
    fn budget_diagnostics_preserve_simultaneous_exhaustion() {
        let mut reasons = std::collections::BTreeSet::new();
        for cells in [false, true] {
            for spans in [false, true] {
                for trim in [false, true] {
                    assert!(reasons.insert(exhausted_work_reason(cells, spans, trim)));
                }
            }
        }
        assert_eq!(reasons.len(), 8);
        assert_eq!(
            exhausted_work_reason(false, true, true),
            "triangle-span-trim-work-limit"
        );
    }
    #[test]
    fn interleaves_faces_without_losing_or_reordering_their_candidates() {
        let faces = [3, 3, 3, 8, 8, 1];
        let triangles: Vec<_> = faces
            .iter()
            .map(|&face| FaceTriangleCandidate {
                face,
                vertices: [0, 1, 2],
                uv: [[0., 0.], [1., 0.], [0., 1.]],
            })
            .collect();
        assert_eq!(fair_face_order(&triangles), vec![5, 0, 3, 1, 4, 2]);
        assert!(fair_face_order(&[]).is_empty());
    }
}
