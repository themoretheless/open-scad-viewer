//! Boundary-preserving triangulation proposals. Surface and trim error admission
//! remains separate; these triangles must not be treated as qualified material.
use super::*;
#[path = "source_profile_triangle_error.rs"]
mod triangle_error;
use std::collections::BTreeMap;
pub struct FaceTriangleCandidate {
    pub face: usize,
    pub vertices: [usize; 3],
    pub uv: [[f64; 2]; 3],
}
pub struct FaceCandidatePending {
    pub face: usize,
    pub reason: &'static str,
}
pub struct FaceCandidateReport {
    pub triangles: Vec<FaceTriangleCandidate>,
    pub pending: Vec<FaceCandidatePending>,
}
fn edge(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}
fn rectangle_partition(domain: [[f64; 2]; 2]) -> [[[f64; 2]; 2]; 4] {
    let middle = domain.map(|r| (r[0] * 0.5 + r[1] * 0.5).clamp(r[0], r[1]));
    std::array::from_fn(|index| {
        std::array::from_fn(|axis| {
            if (index >> axis) & 1 == 0 {
                [domain[axis][0], middle[axis]]
            } else {
                [middle[axis], domain[axis][1]]
            }
        })
    })
}
/// Strict interval separation only. Touching or unresolved rectangles remain
/// in the support evaluation, so no triangle edge or thin region is discarded.
fn rectangle_outside_triangle(uv: [[f64; 2]; 3], rectangle: [[f64; 2]; 2]) -> Result<bool> {
    use nurbs_core::interval_eval::Interval as I;
    let difference = |a: f64, b: f64| I::point(a).sub(I::point(b));
    let cross = |a: [f64; 2], b: [f64; 2], point: [f64; 2]| {
        difference(b[0], a[0])?
            .mul(difference(point[1], a[1])?)?
            .sub(difference(b[1], a[1])?.mul(difference(point[0], a[0])?)?)
    };
    let orientation = cross(uv[0], uv[1], uv[2])?;
    let sign = if orientation.lo > 0. {
        1
    } else if orientation.hi < 0. {
        -1
    } else {
        return Ok(false);
    };
    for index in 0..3 {
        let a = uv[index];
        let b = uv[(index + 1) % 3];
        let offset = [
            I::new(rectangle[0][0], rectangle[0][1])?.sub(I::point(a[0]))?,
            I::new(rectangle[1][0], rectangle[1][1])?.sub(I::point(a[1]))?,
        ];
        let side = difference(b[0], a[0])?
            .mul(offset[1])?
            .sub(difference(b[1], a[1])?.mul(offset[0])?)?;
        if (sign == 1 && side.hi < 0.) || (sign == -1 && side.lo > 0.) {
            return Ok(true);
        }
    }
    Ok(false)
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn propose_boundary_face_triangles(
        &self,
        graph: &BoundaryGraphReport,
    ) -> FaceCandidateReport {
        let loops = self.order_boundary_loops(graph);
        let mut result = FaceCandidateReport {
            triangles: vec![],
            pending: vec![],
        };
        for face in 0..self.faces.len() {
            let wires = loops.iter().filter(|w| w.face == face).collect::<Vec<_>>();
            if wires.is_empty() || wires.iter().any(|w| !w.pending.is_empty()) {
                result.pending.push(FaceCandidatePending {
                    face,
                    reason: "face-boundary-constraints-incomplete",
                });
                continue;
            }
            let mut rings = Vec::new();
            let mut source = Vec::new();
            let mut expected = BTreeMap::new();
            let mut boundary_directions = BTreeMap::new();
            for wire in wires {
                let mut ring = Vec::new();
                for owned in &wire.segments {
                    let segment = &graph.segments[owned.segment];
                    let first = usize::from(owned.reversed);
                    let last = 1 - first;
                    let id = segment.vertices[first];
                    let uv = segment.uv[first][owned.side];
                    let point = [
                        uv[0][0] * 0.5 + uv[0][1] * 0.5,
                        uv[1][0] * 0.5 + uv[1][1] * 0.5,
                    ];
                    boundary_directions.insert(
                        edge(id, segment.vertices[last]),
                        if id < segment.vertices[last] {
                            1i32
                        } else {
                            -1
                        },
                    );
                    ring.push(point);
                    source.push((point, id));
                    *expected
                        .entry(edge(id, segment.vertices[last]))
                        .or_insert(0usize) += 1;
                }
                rings.push(ring);
            }
            if source.len() > 4096 {
                result.pending.push(FaceCandidatePending {
                    face,
                    reason: "face-triangulation-work-limit",
                });
                continue;
            }
            let fill =
                match planar_geometry::triangulation::triangulate_profile(&rings[0], &rings[1..]) {
                    Ok(fill) => fill,
                    Err(error) => {
                        eprintln!(
                            "face {face} rings={:?} triangulation refused: {error}",
                            rings.iter().map(Vec::len).collect::<Vec<_>>()
                        );
                        result.pending.push(FaceCandidatePending {
                            face,
                            reason: "face-triangulation-proposal-refused",
                        });
                        continue;
                    }
                };
            let mut ids = Vec::new();
            let mut valid = true;
            for point in &fill.positions {
                let matches = source
                    .iter()
                    .filter(|(p, _)| p == point)
                    .map(|(_, id)| *id)
                    .collect::<std::collections::BTreeSet<_>>();
                if matches.len() != 1 {
                    valid = false;
                    break;
                }
                ids.push(*matches.iter().next().unwrap());
            }
            if !valid {
                result.pending.push(FaceCandidatePending {
                    face,
                    reason: "face-triangulation-vertex-identity-unproven",
                });
                continue;
            }
            let mut incidence = BTreeMap::new();
            let mut directions = BTreeMap::new();
            let mut triangles = Vec::new();
            for t in fill.indices.chunks_exact(3) {
                let vertices = [ids[t[0] as usize], ids[t[1] as usize], ids[t[2] as usize]];
                if vertices[0] == vertices[1]
                    || vertices[1] == vertices[2]
                    || vertices[2] == vertices[0]
                {
                    valid = false;
                    break;
                }
                for i in 0..3 {
                    *directions
                        .entry(edge(vertices[i], vertices[(i + 1) % 3]))
                        .or_insert(0i32) += if vertices[i] < vertices[(i + 1) % 3] {
                        1
                    } else {
                        -1
                    };
                    *incidence
                        .entry(edge(vertices[i], vertices[(i + 1) % 3]))
                        .or_insert(0usize) += 1;
                }
                triangles.push(FaceTriangleCandidate {
                    face,
                    vertices,
                    uv: [
                        fill.positions[t[0] as usize],
                        fill.positions[t[1] as usize],
                        fill.positions[t[2] as usize],
                    ],
                });
            }
            valid &= expected
                .iter()
                .all(|(e, n)| *n == 1 && incidence.get(e) == Some(&1));
            valid &= incidence
                .iter()
                .all(|(e, n)| *n == if expected.contains_key(e) { 1 } else { 2 });
            if !valid {
                result.pending.push(FaceCandidatePending {
                    face,
                    reason: "face-triangulation-boundary-incidence-unproven",
                });
                continue;
            }
            let aligned = boundary_directions
                .iter()
                .all(|(e, d)| directions.get(e) == Some(d));
            let reversed = boundary_directions
                .iter()
                .all(|(e, d)| directions.get(e) == Some(&(-d)));
            let interior_oriented = directions
                .iter()
                .filter(|(e, _)| !expected.contains_key(e))
                .all(|(_, d)| *d == 0);
            if (!aligned && !reversed) || !interior_oriented {
                result.pending.push(FaceCandidatePending {
                    face,
                    reason: "face-triangulation-orientation-unproven",
                });
                continue;
            }
            if reversed {
                for triangle in &mut triangles {
                    triangle.vertices.swap(1, 2);
                    triangle.uv.swap(1, 2);
                }
            }
            result.triangles.extend(triangles);
        }
        result
    }
}

impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Numerical error gate only. Exact trim coverage remains independent.
    pub fn bound_candidate_triangle(
        &self,
        candidate: &FaceTriangleCandidate,
        graph: &BoundaryGraphReport,
        controls: usize,
        spans: usize,
    ) -> Result<(Option<f64>, usize)> {
        self.bound_candidate_triangle_impl(candidate, graph, controls, spans, false, 0, None)
    }
    /// Diagnostic correspondence deviation at a specified native support point.
    /// The caller must separately prove membership in the material triangle;
    /// a small lower bound never admits a surface or establishes proximity.
    pub fn candidate_point_deviation_lower(
        &self,
        candidate: &FaceTriangleCandidate,
        graph: &BoundaryGraphReport,
        query: [f64; 2],
        controls: usize,
        spans: usize,
    ) -> Result<(Option<f64>, usize)> {
        if candidate.face >= self.faces.len()
            || controls > 1000000
            || spans > 100000
            || candidate
                .uv
                .iter()
                .flatten()
                .chain(query.iter())
                .any(|v| !v.is_finite())
        {
            return Err(Error::new(
                "BREP_FACE_TRIANGLE",
                "Use finite owned support coordinates and bounded work",
            ));
        }
        let natural = self.support_domain(candidate.face)?;
        if (0..2).any(|k| query[k] < natural[k][0] || query[k] > natural[k][1]) {
            return Err(Error::new(
                "BREP_FACE_TRIANGLE",
                "Keep the witness inside the owned support",
            ));
        }
        let mut points = [[0.; 3]; 3];
        for (index, vertex) in candidate.vertices.iter().enumerate() {
            let Some(point) = graph.vertices.get(*vertex).and_then(|v| v.point) else {
                return Ok((None, 0));
            };
            points[index] = point;
        }
        if spans == 0 {
            return Ok((None, 0));
        }
        let (support, spent) =
            self.enclose_support(candidate.face, query.map(|q| [q, q]), controls, spans)?;
        let Some(support) = support else {
            return Ok((None, spent));
        };
        Ok((
            triangle_error::point_deviation_lower(candidate.uv, points, query, support)?,
            spent,
        ))
    }
    /// Diagnostic alternative anchor; this does not change production admission.
    pub fn bound_candidate_triangle_rectangle_center(
        &self,
        candidate: &FaceTriangleCandidate,
        graph: &BoundaryGraphReport,
        controls: usize,
        spans: usize,
    ) -> Result<(Option<f64>, usize)> {
        self.bound_candidate_triangle_impl(candidate, graph, controls, spans, true, 0, None)
    }
    /// Diagnostic triangle coverage by rectangle partitions; retains the original mesh plane.
    pub fn bound_candidate_triangle_partitioned(
        &self,
        candidate: &FaceTriangleCandidate,
        graph: &BoundaryGraphReport,
        controls: usize,
        spans: usize,
    ) -> Result<(Option<f64>, usize)> {
        self.bound_candidate_triangle_impl(candidate, graph, controls, spans, true, 1, None)
    }
    /// Bounded diagnostic rectangle refinement; every cell retains the original
    /// affine plane. Shared split coordinates tile the UV box; only cells proved
    /// strictly separated from the original triangle are excluded.
    pub fn bound_candidate_triangle_partition_depth(
        &self,
        candidate: &FaceTriangleCandidate,
        graph: &BoundaryGraphReport,
        controls: usize,
        spans: usize,
        depth: usize,
    ) -> Result<(Option<f64>, usize)> {
        self.bound_candidate_triangle_impl(candidate, graph, controls, spans, true, depth, None)
    }
    /// Complete upper bound only when the requested tolerance is met. A region
    /// whose upper bound exceeds the target ends this attempt as unproven;
    /// this is not evidence that the actual surface violates the tolerance.
    pub fn bound_candidate_triangle_to_target(
        &self,
        candidate: &FaceTriangleCandidate,
        graph: &BoundaryGraphReport,
        controls: usize,
        spans: usize,
        depth: usize,
        target_mm: f64,
    ) -> Result<(Option<f64>, usize)> {
        if !target_mm.is_finite() || target_mm <= 0. || depth == 0 {
            return Err(Error::new(
                "BREP_FACE_TRIANGLE",
                "Use a positive finite tolerance and partition depth",
            ));
        }
        self.bound_candidate_triangle_impl(
            candidate,
            graph,
            controls,
            spans,
            true,
            depth,
            Some(target_mm),
        )
    }
    fn bound_candidate_triangle_impl(
        &self,
        candidate: &FaceTriangleCandidate,
        graph: &BoundaryGraphReport,
        controls: usize,
        spans: usize,
        rectangle_center: bool,
        partition_depth: usize,
        target_mm: Option<f64>,
    ) -> Result<(Option<f64>, usize)> {
        if candidate.face >= self.faces.len()
            || controls > 1000000
            || spans > 100000
            || partition_depth > 4
            || candidate.uv.iter().flatten().any(|v| !v.is_finite())
        {
            return Err(Error::new(
                "BREP_FACE_TRIANGLE",
                "Choose an owned face, finite UV coordinates and bounded work",
            ));
        }
        if spans == 0 {
            let natural = self.support_domain(candidate.face)?;
            if candidate
                .uv
                .iter()
                .any(|p| (0..2).any(|k| p[k] < natural[k][0] || p[k] > natural[k][1]))
            {
                return Err(Error::new(
                    "BREP_FACE_TRIANGLE",
                    "Keep triangle UV coordinates inside the owned support",
                ));
            }
            return Ok((None, 0));
        }
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let source = match self.faces[candidate.face].kind {
            ShellFaceKind::Original(id) => {
                root.restricted.proposal.body.geometry().shell().faces()[id][0].edges()[0].surface()
            }
            _ => root
                .restricted
                .proposal
                .profile(root.face)
                .unwrap()
                .source(),
        };
        if source.periodic_u || source.periodic_v {
            return Ok((None, 0));
        }
        source.validate()?;
        let points = candidate
            .vertices
            .map(|id| graph.vertices.get(id).and_then(|v| v.point));
        let [Some(a), Some(b), Some(c)] = points else {
            return Ok((None, 0));
        };
        let domain = std::array::from_fn(|axis| {
            [
                candidate
                    .uv
                    .iter()
                    .map(|p| p[axis])
                    .fold(f64::INFINITY, f64::min),
                candidate
                    .uv
                    .iter()
                    .map(|p| p[axis])
                    .fold(f64::NEG_INFINITY, f64::max),
            ]
        });
        if partition_depth > 0 {
            let mut spent = 0;
            let mut upper = 0_f64;
            let mut evaluated = false;
            let mut parts = vec![domain];
            for _ in 0..partition_depth {
                parts = parts.into_iter().flat_map(rectangle_partition).collect();
            }
            for part in parts {
                if rectangle_outside_triangle(candidate.uv, part)? {
                    continue;
                }
                evaluated = true;
                let anchor_uv = part.map(|r| r[0] * 0.5 + r[1] * 0.5);
                let (anchor, used) = self.enclose_support(
                    candidate.face,
                    anchor_uv.map(|x| [x, x]),
                    controls,
                    spans - spent,
                )?;
                spent += used;
                let Some(anchor) = anchor else {
                    return Ok((None, spent));
                };
                let (derivatives, used) = self.enclose_support_derivatives(
                    candidate.face,
                    part,
                    controls,
                    spans - spent,
                )?;
                spent += used;
                let Some(derivatives) = derivatives else {
                    return Ok((None, spent));
                };
                // These two triangles cover the complete rectangle, including
                // any region outside the original triangle. This is conservative.
                let [u, v] = part;
                let corners = [[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]];
                for region in [
                    [corners[0], corners[1], corners[2]],
                    [corners[0], corners[2], corners[3]],
                ] {
                    let Some(bound) = triangle_error::bound_region(
                        candidate.uv,
                        [a, b, c],
                        region,
                        anchor_uv,
                        anchor,
                        derivatives,
                    )?
                    else {
                        return Ok((None, spent));
                    };
                    if target_mm.is_some_and(|target| bound > target) {
                        return Ok((None, spent));
                    }
                    upper = upper.max(bound);
                }
            }
            return Ok((evaluated.then_some(upper), spent));
        }
        // Every segment from this anchor to the triangle remains within the
        // derivative rectangle, even when its center lies outside the triangle.
        let anchor_uv = if rectangle_center {
            domain.map(|range| range[0] * 0.5 + range[1] * 0.5)
        } else {
            candidate.uv[0]
        };
        let (anchor, spent) =
            self.enclose_support(candidate.face, anchor_uv.map(|t| [t, t]), controls, spans)?;
        let Some(anchor) = anchor else {
            return Ok((None, spent));
        };
        let (derivatives, used) =
            self.enclose_support_derivatives(candidate.face, domain, controls, spans - spent)?;
        let Some(derivatives) = derivatives else {
            return Ok((None, spent + used));
        };
        Ok((
            if rectangle_center {
                triangle_error::bound_at(candidate.uv, [a, b, c], anchor_uv, anchor, derivatives)?
            } else {
                triangle_error::bound(candidate.uv, [a, b, c], anchor, derivatives)?
            },
            spent + used,
        ))
    }
}

#[cfg(test)]
mod partition_tests {
    use super::*;
    #[test]
    fn separated_cells_are_skipped_but_touching_and_degenerate_regions_remain() {
        for scale in [1e-150, 1., 1e150] {
            let uv = [[0., 0.], [scale, 0.], [0., scale]];
            let outside = [[0.75 * scale, scale], [0.75 * scale, scale]];
            assert!(rectangle_outside_triangle(uv, outside).unwrap());
            assert!(rectangle_outside_triangle([uv[0], uv[2], uv[1]], outside).unwrap());
            for rectangle in [
                [[0., 0.25 * scale], [0., 0.25 * scale]],
                [[0.5 * scale, 0.75 * scale], [0.5 * scale, 0.75 * scale]],
                [[0., scale], [0., scale]],
            ] {
                assert!(!rectangle_outside_triangle(uv, rectangle).unwrap());
            }
        }
        assert!(!rectangle_outside_triangle([[0.; 2]; 3], [[1., 2.], [1., 2.]]).unwrap());
    }
    #[test]
    fn separation_retains_thin_contact_and_refuses_overflow() {
        let uv = [[0., 0.], [1., 0.], [0., 1.]];
        // One ULP beyond the diagonal is not enough to establish separation
        // through outward interval subtraction and multiplication.
        let near = f64::from_bits(0.5_f64.to_bits() + 1);
        assert!(!rectangle_outside_triangle(uv, [[near, near], [0.5, 0.5]]).unwrap());
        for scale in [1e-160, 1e-150, 1., 1e150] {
            let triangle = uv.map(|p| p.map(|v| v * scale));
            for i in 0..=8 {
                for j in 0..=8-i {
                    let point = [i as f64 * scale / 8., j as f64 * scale / 8.];
                    let cell = point.map(|v| [v, v]);
                    assert!(!rectangle_outside_triangle(triangle, cell).unwrap());
                    assert!(!rectangle_outside_triangle(
                        [triangle[0], triangle[2], triangle[1]], cell).unwrap());
                }
            }
        }
        assert!(rectangle_outside_triangle(
            [[-f64::MAX, 0.], [f64::MAX, 0.], [0., 1.]],
            [[1., 2.], [2., 3.]],
        ).is_err());
    }
    #[test]
    fn recursive_partition_keeps_shared_seams_and_parent_extents() {
        for domain in [
            [[0., 1.], [-2., 3.]],
            [[-1e308, 1e308], [0., f64::from_bits(1)]],
        ] {
            let mut parts = vec![domain];
            for depth in 1..=4 {
                parts = parts.into_iter().flat_map(rectangle_partition).collect();
                assert_eq!(parts.len(), 4_usize.pow(depth));
                for axis in 0..2 {
                    let mut intervals = parts.iter().map(|p| p[axis]).collect::<Vec<_>>();
                    intervals.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
                    intervals.dedup();
                    assert_eq!(intervals.first().unwrap()[0], domain[axis][0]);
                    assert_eq!(intervals.last().unwrap()[1], domain[axis][1]);
                    for pair in intervals.windows(2) {
                        assert_eq!(pair[0][1], pair[1][0]);
                    }
                }
            }
        }
    }
    #[test]
    fn rectangle_parts_cover_extreme_and_subnormal_domains_without_gaps() {
        for domain in [
            [[0., 1.], [-2., 3.]],
            [[-1e308, 1e308], [0., f64::from_bits(1)]],
        ] {
            let parts = rectangle_partition(domain);
            for part in parts {
                for axis in 0..2 {
                    assert!(
                        part[axis][0] >= domain[axis][0]
                            && part[axis][1] <= domain[axis][1]
                            && part[axis][0] <= part[axis][1]
                    );
                }
            }
            for i in 0..=20 {
                for j in 0..=20 {
                    let point = [
                        domain[0][0] * (1. - i as f64 / 20.) + domain[0][1] * (i as f64 / 20.),
                        domain[1][0] * (1. - j as f64 / 20.) + domain[1][1] * (j as f64 / 20.),
                    ];
                    assert!(
                        parts
                            .iter()
                            .any(|p| (0..2).all(|a| p[a][0] <= point[a] && point[a] <= p[a][1]))
                    );
                }
            }
        }
    }
}
