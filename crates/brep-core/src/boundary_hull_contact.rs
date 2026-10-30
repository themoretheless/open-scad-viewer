//! Exact hull contact restricted to common authored topology.
//! Axis bounds and exact oblique supporting planes provide sufficient proofs.
//! Only used after exact edge/lift agreement, simple trims and chart injectivity.
use crate::Model;
use std::collections::BTreeSet;
#[derive(Clone, Debug)]
pub struct Certificate {
    pub faces: [usize; 2],
    /// Encloses possible surface contact after all proof restrictions.
    /// Bezier support restrictions can be tighter than control-hull overlap.
    pub contact_enclosure: [[f64; 2]; 3],
    pub edges: Vec<usize>,
    pub vertex: Option<usize>,
    pub joined_proof: Option<JoinedProof>,
    pub polar_join: Option<nurbs_core::surface_injectivity::PolarPMatrix>,
    pub source_join: Option<SourceJoinedProof>,
}
#[derive(Clone, Debug)]
pub struct SourceJoinedProof {
    pub blend_face: usize,
    pub wall_face: usize,
    pub collapsed_end: usize,
    pub report: nurbs_core::surface_quotient_injectivity::JoinedReport,
}
#[derive(Clone, Debug)]
pub struct JoinedProof {
    pub blend_face: usize,
    pub wall_face: usize,
    pub collapsed_end: usize,
    pub projection: [[f64; 3]; 2],
    pub report: nurbs_core::surface_quotient_injectivity::JoinedReport,
}
fn edge_ends(model: &Model, edge: usize) -> Option<[[f64; 3]; 2]> {
    let c = &model.edges[edge].curve;
    let n = c.control_points.len();
    if c.periodic
        || c.control_points.iter().any(|p| p.len() != 3)
        || c.knots[..=c.degree].iter().any(|k| *k != c.knots[c.degree])
        || c.knots[n..].iter().any(|k| *k != c.knots[n])
    {
        return None;
    }
    Some([
        std::array::from_fn(|k| c.control_points[0][k]),
        std::array::from_fn(|k| c.control_points[n - 1][k]),
    ])
}
// Bounded floating-point proposals for a strict supporting plane. The caller
// verifies the resulting binary64 anchors exactly against every original pole.

// With exact boundary agreement, a simple positively wound planar outer
// loop bounds the trimmed face inside the convex hull of its boundary curves.
// Positive rational curve weights enclose each curve by its original controls.
// This avoids using the much larger untrimmed plane chart as a face enclosure.
fn contact_controls(model: &Model, face: usize) -> Vec<&[f64]> {
    let f = &model.faces[face];
    let surface = f
        .surface
        .control_points
        .iter()
        .flatten()
        .map(|p| p.as_slice())
        .collect::<Vec<_>>();
    for axis in 0..3 {
        let value = surface[0][axis];
        if !surface.iter().all(|p| p[axis] == value) {
            continue;
        }
        let outer = model.loops[f.outer]
            .coedges
            .iter()
            .flat_map(|c| model.edges[c.edge].curve.control_points.iter())
            .map(|p| p.as_slice())
            .collect::<Vec<_>>();
        if !outer.is_empty() && outer.iter().all(|p| p[axis] == value) {
            return outer;
        }
    }
    surface
}
/// Input must be structurally valid. Does not establish its prerequisites.
mod extended_support;
pub(crate) fn certify(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    certify_original(model, faces).or_else(|| {
        extended_support::certify(model, faces).map(|c| Certificate {
            faces: c.faces,
            contact_enclosure: c.hull_intersection,
            edges: c.edges,
            vertex: c.vertex,
            joined_proof: None,
            polar_join: None,
            source_join: None,
        })
    })
}
fn certify_original(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    certify_axis(model, faces)
        .or_else(|| certify_edge_plane(model, faces))
        .or_else(|| certify_vertex_plane(model, faces))
        .or_else(|| certify_joined_charts(model, faces))
        .or_else(|| certify_polar_charts(model, faces))
}


// The normalized chart union is globally injective under the projected
// weighted bounds. Exact shared natural-boundary lifts bind that union to
// the authored edge, including its owned collapsed endpoint.
fn certify_polar_charts(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    let uses = faces.map(|f| {
        std::iter::once(model.faces[f].outer)
            .chain(model.faces[f].holes.iter().copied())
            .flat_map(|l| model.loops[l].coedges.iter())
            .collect::<Vec<_>>()
    });
    let surfaces = faces.map(|f| &model.faces[f].surface);
    for ca in &uses[0] {
        for cb in &uses[1] {
            if ca.edge != cb.edge {
                continue;
            }
            let edge = &model.edges[ca.edge].curve;
            if crate::shared_boundary::boundary(surfaces[0], &ca.pcurve, edge)
                != Some((1, surfaces[0].degree_v))
                || crate::shared_boundary::boundary(surfaces[1], &cb.pcurve, edge)
                    != Some((1, surfaces[1].degree_v))
            {
                continue;
            }
            for z in [-1., 1.] {
                let coordinates = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., z, 0.]];
                let Some(proof) = nurbs_core::surface_injectivity::certify_polar_join(
                    surfaces[0],
                    surfaces[1],
                    coordinates,
                    16,
                    512,
                )
                .ok()
                .flatten() else {
                    continue;
                };
                let mut enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
                for p in &edge.control_points {
                    for k in 0..3 {
                        enclosure[k][0] = enclosure[k][0].min(p[k]);
                        enclosure[k][1] = enclosure[k][1].max(p[k]);
                    }
                }
                return Some(Certificate {
                    faces,
                    contact_enclosure: enclosure,
                    edges: vec![ca.edge],
                    vertex: None,
                    joined_proof: None,
                    source_join: None,
                    polar_join: Some(proof),
                });
            }
        }
    }
    None
}

fn certify_joined_charts(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    let uses = faces.map(|f| {
        std::iter::once(model.faces[f].outer)
            .chain(model.faces[f].holes.iter().copied())
            .flat_map(|l| model.loops[l].coedges.iter())
            .collect::<Vec<_>>()
    });
    for (blend_face, wall_face, blend_uses, wall_uses) in [
        (faces[0], faces[1], &uses[0], &uses[1]),
        (faces[1], faces[0], &uses[1], &uses[0]),
    ] {
        let blend = &model.faces[blend_face].surface;
        let wall = &model.faces[wall_face].surface;
        if wall.degree_v != 1 || wall.degree_u != blend.degree_u {
            continue;
        }
        for end in 0..2 {
            let row = if end == 0 {
                0
            } else {
                blend.control_points.len() - 1
            };
            if !blend.control_points[row]
                .iter()
                .all(|p| p == &blend.control_points[row][0])
            {
                continue;
            }
            for ca in blend_uses.iter() {
                for cb in wall_uses.iter() {
                    if ca.edge != cb.edge {
                        continue;
                    }
                    let edge = &model.edges[ca.edge].curve;
                    if crate::shared_boundary::boundary(blend, &ca.pcurve, edge)
                        != Some((1, blend.control_points[0].len() - 1))
                        || crate::shared_boundary::boundary(wall, &cb.pcurve, edge) != Some((1, 1))
                    {
                        continue;
                    }
                    // Try a bounded family of coordinate projections. Each admission still
                    // requires the full original-chart ruled-join certificate below.
                    let mut projections =
                        vec![[[0., 1., 0.], [1., 0., -1.]], [[1., 0., 0.], [0., 1., -1.]]];
                    for axis in 0..3 {
                        for first_sign in [-1., 1.] {
                            for a in [-1., 1.] {
                                for b in [-1., 1.] {
                                    let mut first = [0.; 3];
                                    first[axis] = first_sign;
                                    let other = (0..3).filter(|k| *k != axis).collect::<Vec<_>>();
                                    let mut second = [0.; 3];
                                    second[other[0]] = a;
                                    second[other[1]] = b;
                                    let projection = [first, second];
                                    if !projections.contains(&projection) {
                                        projections.push(projection);
                                    }
                                }
                            }
                        }
                    }
                    for projection in projections {
                        // Filter exact structural requirements before expensive bounds.
                        if (0..3).any(|k| {
                            projection[0][k] != 0.
                                && wall.control_points.iter().any(|r| r[0][k] != r[1][k])
                        }) {
                            continue;
                        }
                        let pole = if end == 0 {
                            0
                        } else {
                            wall.control_points.len() - 1
                        };
                        let neighbor = if end == 0 { 1 } else { pole - 1 };
                        if (0..2).any(|j| {
                            (0..3).any(|k| {
                                projection[1][k] != 0.
                                    && wall.control_points[pole][j][k]
                                        != wall.control_points[neighbor][j][k]
                            })
                        }) {
                            continue;
                        }

                        let report = nurbs_core::surface_quotient_injectivity::certify_ruled_join(
                            blend, wall, end, projection, 16, 512,
                        )
                        .ok()?;
                        if !report.proven {
                            continue;
                        }
                        let mut enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
                        for p in &edge.control_points {
                            for k in 0..3 {
                                enclosure[k][0] = enclosure[k][0].min(p[k]);
                                enclosure[k][1] = enclosure[k][1].max(p[k]);
                            }
                        }
                        return Some(Certificate {
                            faces,
                            contact_enclosure: enclosure,
                            edges: vec![ca.edge],
                            vertex: None,
                            source_join: None,
                            polar_join: None,
                            joined_proof: Some(JoinedProof {
                                blend_face,
                                wall_face,
                                collapsed_end: end,
                                projection,
                                report,
                            }),
                        });
                    }
                    let source =
                        nurbs_core::surface_quotient_injectivity::certify_ruled_join_source_frame(
                            blend, wall, end, 16, 512,
                        )
                        .ok()?;
                    if source.proven {
                        let mut enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
                        for p in &edge.control_points {
                            for k in 0..3 {
                                enclosure[k][0] = enclosure[k][0].min(p[k]);
                                enclosure[k][1] = enclosure[k][1].max(p[k]);
                            }
                        }
                        return Some(Certificate {
                            faces,
                            contact_enclosure: enclosure,
                            edges: vec![ca.edge],
                            vertex: None,
                            joined_proof: None,
                            polar_join: None,
                            source_join: Some(SourceJoinedProof {
                                blend_face,
                                wall_face,
                                collapsed_end: end,
                                report: source,
                            }),
                        });
                    }
                }
            }
        }
    }
    None
}

// A straight owned edge covers its endpoint segment. A separating plane
// can restrict the intersection of the enclosing hulls to this segment even
// when its world-axis bounding box has two or three nonzero dimensions.
fn certify_edge_plane(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    use cad_predicates::Sign;
    let nets = faces.map(|f| contact_controls(model, f));
    if nets.iter().any(|n| n.len() > 64) {
        return None;
    }
    let edges = faces.map(|f| {
        std::iter::once(model.faces[f].outer)
            .chain(model.faces[f].holes.iter().copied())
            .flat_map(|l| model.loops[l].coedges.iter().map(|c| c.edge))
            .collect::<BTreeSet<_>>()
    });
    for &edge in edges[0].intersection(&edges[1]) {
        let curve = &model.edges[edge].curve;
        if curve.degree != 1 || curve.control_points.len() != 2 {
            continue;
        }
        let Some([a, b]) = edge_ends(model, edge) else {
            continue;
        };
        if a == b
            || [a, b]
                .iter()
                .enumerate()
                .any(|(i, p)| *p != model.vertices[model.edges[edge].vertices[i]].point)
        {
            continue;
        }
        let on_segment = |p: &[f64]| {
            (0..3).all(|k| p[k] >= a[k].min(b[k]) && p[k] <= a[k].max(b[k]))
                && [[0, 1], [0, 2], [1, 2]].into_iter().all(|axes| {
                    crate::shared_boundary::orient(&[&a, &b, p], Some(axes)) == Some(Sign::Zero)
                })
        };
        // Coplanar trim hulls need a transverse candidate, since their own
        // controls only generate their common support plane. Arbitrary finite
        // candidate points are safe: exact signs on every original enclosure
        // control, rather than candidate generation, establish separation.
        let mut candidates = nets
            .iter()
            .flatten()
            .take(128)
            .map(|p| [p[0], p[1], p[2]])
            .collect::<Vec<_>>();
        for axis in 0..3 {
            let mut p = a;
            p[axis] += 1.;
            if p.iter().all(|v| v.is_finite()) && p != a {
                candidates.push(p);
            }
        }
        for candidate in &candidates {
            let mut signs = [None, None];
            let mut segment_only = [true, true];
            let mut valid = true;
            for side in 0..2 {
                for &p in &nets[side] {
                    let Some(sign) = crate::shared_boundary::orient(&[&a, &b, candidate, p], None)
                    else {
                        valid = false;
                        break;
                    };
                    if sign == Sign::Zero {
                        segment_only[side] &= on_segment(p);
                    } else if signs[side].is_some_and(|previous| previous != sign) {
                        valid = false;
                        break;
                    } else {
                        signs[side] = Some(sign);
                    }
                }
                if !valid {
                    break;
                }
            }
            if valid
                && (segment_only[0] || segment_only[1])
                && signs.iter().any(Option::is_some)
                && !(signs[0].is_some() && signs[0] == signs[1])
            {
                return Some(Certificate {
                    faces,
                    contact_enclosure: std::array::from_fn(|k| [a[k].min(b[k]), a[k].max(b[k])]),
                    edges: vec![edge],
                    vertex: None,
                    joined_proof: None,
                    source_join: None,
                    polar_join: None,
                });
            }
        }
    }
    None
}

// In a single clamped tensor Bezier chart every interior basis value is
// positive. A nonplanar one-sided net can reach its support plane only on
// complete zero-sign natural edges or zero-sign corners, not at an isolated
// intermediate control. Enumerate those strata before allowing a vertex.
fn bezier_plane_vertex_only(
    surface: &nurbs_core::surface::Surface,
    plane: [&[f64]; 3],
    point: &[f64],
) -> bool {
    use cad_predicates::Sign;
    let sizes = [
        surface.control_points.len(),
        surface.control_points[0].len(),
    ];
    let degrees = [surface.degree_u, surface.degree_v];
    let knots = [&surface.knots_u, &surface.knots_v];
    if (0..2).any(|k| {
        degrees[k] == 0
            || sizes[k] != degrees[k] + 1
            || knots[k][..=degrees[k]]
                .iter()
                .any(|v| *v != knots[k][degrees[k]])
            || knots[k][sizes[k]..]
                .iter()
                .any(|v| *v != knots[k][sizes[k]])
    }) {
        return false;
    }
    let mut sign = None;
    let mut zeros = vec![vec![false; sizes[1]]; sizes[0]];
    for (u, row) in surface.control_points.iter().enumerate() {
        for (v, p) in row.iter().enumerate() {
            let Some(s) = crate::shared_boundary::orient(&[plane[0], plane[1], plane[2], p], None)
            else {
                return false;
            };
            if s == Sign::Zero {
                zeros[u][v] = true;
            } else {
                if sign.is_some_and(|previous| previous != s) {
                    return false;
                }
                sign = Some(s);
            }
        }
    }
    if sign.is_none() {
        return false;
    }
    for u in [0, sizes[0] - 1] {
        for v in [0, sizes[1] - 1] {
            if zeros[u][v] && surface.control_points[u][v] != point {
                return false;
            }
        }
    }
    for u in [0, sizes[0] - 1] {
        if zeros[u].iter().all(|z| *z) && surface.control_points[u].iter().any(|p| p != point) {
            return false;
        }
    }
    for v in [0, sizes[1] - 1] {
        if (0..sizes[0]).all(|u| zeros[u][v])
            && (0..sizes[0]).any(|u| surface.control_points[u][v] != point)
        {
            return false;
        }
    }
    true
}

// A supporting plane need not align with world axes. Original binary64
// controls and an exact orientation predicate establish its two half spaces.
// If one net reaches the plane only at the shared vertex, every possible
// surface contact is that vertex. No tolerance or sampled normal is used.
fn certify_vertex_plane(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    use cad_predicates::Sign;
    let nets = faces.map(|f| contact_controls(model, f));
    if nets.iter().any(|n| n.len() > 64) {
        return None;
    }
    let vertices = faces.map(|f| {
        std::iter::once(model.faces[f].outer)
            .chain(model.faces[f].holes.iter().copied())
            .flat_map(|l| model.loops[l].coedges.iter())
            .flat_map(|c| {
                let ends = edge_ends(model, c.edge);
                (0..2).filter_map(move |i| {
                    ends.filter(|p| p[i] == model.vertices[model.edges[c.edge].vertices[i]].point)
                        .map(|_| model.edges[c.edge].vertices[i])
                })
            })
            .collect::<BTreeSet<_>>()
    });
    for &vertex in vertices[0].intersection(&vertices[1]) {
        let point = model.vertices[vertex].point;
        // Owned straight-edge endpoints supply an exact radial/vertical line.
        // Tilted finite anchors can separate a cap from the neighboring wall
        // while exposing only their common vertex on one of the two charts.
        // Proposals have no authority until every original control passes.
        let mut owned_candidates = Vec::<Vec<f64>>::new();
        for face in faces {
            for loop_index in std::iter::once(model.faces[face].outer)
                .chain(model.faces[face].holes.iter().copied())
            {
                for coedge in &model.loops[loop_index].coedges {
                    let edge = &model.edges[coedge.edge];
                    if edge.curve.degree != 1 || !edge.vertices.contains(&vertex) {
                        continue;
                    }
                    for v in edge.vertices {
                        let p = model.vertices[v].point;
                        if p != point
                            && !owned_candidates
                                .iter()
                                .any(|q| q.as_slice() == p.as_slice())
                        {
                            owned_candidates.push(p.to_vec());
                        }
                    }
                }
            }
        }
        for axis in 0..2 {
            for sign in [-1., 1.] {
                let mut anchor = point.to_vec();
                anchor[axis] += 1.;
                anchor[2] += sign;
                if anchor.iter().all(|v| v.is_finite()) {
                    owned_candidates.push(anchor);
                }
            }
        }
        owned_candidates.truncate(8);
        let mut candidates = owned_candidates
            .iter()
            .map(|p| p.as_slice())
            .collect::<Vec<_>>();
        let proposal_controls = nets
            .each_ref()
            .map(|ps| ps.iter().map(|p| p.to_vec()).collect::<Vec<_>>());
        let proposal_poles = proposal_controls
            .each_ref()
            .map(|ps| ps.iter().collect::<Vec<_>>());
        let proposed = propose_vertex_plane(point, &proposal_poles);
        for p in nets.iter().flatten().copied() {
            if p != point && !candidates.contains(&p) {
                candidates.push(p);
            }
            if candidates.len() == 24 {
                break;
            }
        }
        // Floating proposals only choose anchors; every original enclosure
        // control still passes the exact half-space and owned-stratum tests.
        if let Some(anchors) = &proposed {
            candidates.extend(anchors.iter().map(|p| p.as_slice()));
        }
        for a in 0..candidates.len() {
            for b in a + 1..candidates.len() {
                let mut signs = [None, None];
                let mut vertex_only = [true, true];
                let mut valid = true;
                for side in 0..2 {
                    for &p in &nets[side] {
                        let Some(sign) = crate::shared_boundary::orient(
                            &[&point, candidates[a], candidates[b], p],
                            None,
                        ) else {
                            valid = false;
                            break;
                        };
                        if sign == Sign::Zero {
                            vertex_only[side] &= p == point;
                        } else if signs[side].is_some_and(|previous| previous != sign) {
                            valid = false;
                            break;
                        } else {
                            signs[side] = Some(sign);
                        }
                    }
                    if !valid {
                        break;
                    }
                }
                if valid {
                    for side in 0..2 {
                        if !vertex_only[side] {
                            vertex_only[side] = bezier_plane_vertex_only(
                                &model.faces[faces[side]].surface,
                                [&point, candidates[a], candidates[b]],
                                &point,
                            );
                        }
                    }
                }
                if valid
                    && (vertex_only[0] || vertex_only[1])
                    && signs.iter().any(Option::is_some)
                    && !(signs[0].is_some() && signs[0] == signs[1])
                {
                    return Some(Certificate {
                        faces,
                        contact_enclosure: point.map(|v| [v, v]),
                        edges: Vec::new(),
                        vertex: Some(vertex),
                        joined_proof: None,
                        source_join: None,
                        polar_join: None,
                    });
                }
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "boundary_hull_contact/tests/mod.rs"]
mod tests;

#[path="boundary_hull_contact/algorithm_propose_vertex_plane.rs"]
mod algorithm_propose_vertex_plane;
use algorithm_propose_vertex_plane::propose_vertex_plane;
#[path="boundary_hull_contact/algorithm_certify_axis.rs"]
mod algorithm_certify_axis;
use algorithm_certify_axis::certify_axis;
