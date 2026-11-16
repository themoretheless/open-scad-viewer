//! Shared exact stages of the axis-aligned hull contact certificates.
//! Callers restrict the hull intersection first, then resolve the remaining
//! free coordinates to a vertex or a covered segment of common edges.
use super::{edge_ends, propose_vertex_plane};
use crate::Model;
use cad_predicates::Sign;
use std::collections::BTreeSet;

/// Coordinate interval per world axis.
pub(super) type Hull = [[f64; 2]; 3];
/// Fixed relaxation passes for the projected separator proposal.
const PROPOSAL_ITERATIONS: usize = 256;

/// Contact proven by one of the stages.
pub(super) struct Contact {
    pub hull: Hull,
    pub edges: Vec<usize>,
    pub vertex: Option<usize>,
}

/// Caller-specific bounds on candidate generation.
pub(super) struct Limits {
    /// Largest net that still receives generated/axis plane proposals.
    pub proposal_poles: usize,
    /// Largest net whose own poles are also enumerated as plane anchors.
    pub anchor_poles: usize,
}

pub(super) fn hull_of<'p>(points: impl IntoIterator<Item = &'p [f64]>) -> Hull {
    let mut h = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for p in points {
        for k in 0..3 {
            h[k][0] = h[k][0].min(p[k]);
            h[k][1] = h[k][1].max(p[k]);
        }
    }
    h
}

pub(super) fn is_empty(hull: &Hull) -> bool {
    hull.iter().any(|r| r[0] > r[1])
}

pub(super) fn face_edges(model: &Model, faces: [usize; 2]) -> [BTreeSet<usize>; 2] {
    faces.map(|f| {
        std::iter::once(model.faces[f].outer)
            .chain(model.faces[f].holes.iter().copied())
            .flat_map(|l| model.loops[l].coedges.iter().map(|c| c.edge))
            .collect::<BTreeSet<_>>()
    })
}

// Extrema are authored binary64 values: comparisons introduce no rounding.
pub(super) fn intersect(hulls: &[Hull; 2]) -> Hull {
    std::array::from_fn(|k| {
        [
            hulls[0][k][0].max(hulls[1][k][0]),
            hulls[0][k][1].min(hulls[1][k][1]),
        ]
    })
}

fn axis_candidates() -> Vec<Vec<f64>> {
    vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 1.],
    ]
}

// One net lies strictly on one side, touching the plane only at `point`.
fn side_sign(
    poles: &[&Vec<f64>],
    point: [f64; 3],
    sign_of: impl Fn(&Vec<f64>) -> Option<Sign>,
) -> Option<Sign> {
    let mut side = None;
    for p in poles {
        let sign = sign_of(p)?;
        if sign == Sign::Zero {
            if p.as_slice() != point {
                return None;
            }
        } else {
            if side.is_some_and(|s| s != sign) {
                return None;
            }
            side = Some(sign);
        }
    }
    side
}

fn strictly_separated(sides: [Option<Sign>; 2]) -> bool {
    matches!(
        sides,
        [Some(Sign::Positive), Some(Sign::Negative)] | [Some(Sign::Negative), Some(Sign::Positive)]
    )
}

// Bounded floating-point proposal of a projected separator through `point`.
fn propose_projected_anchor(
    point: [f64; 3],
    poles: &[Vec<&Vec<f64>>; 2],
    axes: [usize; 2],
) -> Option<Vec<f64>> {
    let mut projected = [0.; 2];
    let mut directions = Vec::new();
    for (face, ps) in poles.iter().enumerate() {
        for p in ps {
            let mut d = [p[axes[0]] - point[axes[0]], p[axes[1]] - point[axes[1]]];
            let length = d[0].hypot(d[1]);
            if length > 0. && length.is_finite() {
                for x in &mut d {
                    *x /= length;
                    if face == 1 {
                        *x = -*x;
                    }
                }
                directions.push(d);
            }
        }
    }
    for _ in 0..PROPOSAL_ITERATIONS {
        for d in &directions {
            let dot = projected[0] * d[0] + projected[1] * d[1];
            if dot < 1. {
                for k in 0..2 {
                    projected[k] += (1. - dot) * d[k];
                }
            }
        }
    }
    let length = projected[0].hypot(projected[1]);
    (length > 0. && length.is_finite()).then(|| {
        let mut anchor = point.to_vec();
        anchor[axes[0]] -= projected[1] / length;
        anchor[axes[1]] += projected[0] / length;
        anchor
    })
}

/// Restricted pair of faces: their edges and current coordinate hulls.
pub(super) struct Stages<'a> {
    pub model: &'a Model,
    pub faces: [usize; 2],
    pub edges: &'a [BTreeSet<usize>; 2],
    pub hulls: &'a [Hull; 2],
    pub limits: Limits,
}

impl Stages<'_> {
    /// Resolves a restricted hull intersection; `None` refuses the contact.
    pub(super) fn resolve(&self, intersection: Hull) -> Option<Contact> {
        let free = (0..3)
            .filter(|&k| intersection[k][0] < intersection[k][1])
            .collect::<Vec<_>>();
        if free.len() == 3
            && let Some(found) = self.spatial_vertex()
        {
            return Some(found);
        }
        if !free.is_empty()
            && free.len() <= 2
            && let Some(found) = self.projected_vertex(&intersection, &free)
        {
            return Some(found);
        }
        match free.as_slice() {
            [] => self.point_contact(intersection),
            [axis] => self.segment_contact(intersection, *axis),
            _ => None,
        }
    }

    fn vertex_contact(vertex: usize, point: [f64; 3]) -> Contact {
        Contact {
            hull: point.map(|x| [x, x]),
            edges: Vec::new(),
            vertex: Some(vertex),
        }
    }

    // Axes where the intersection is degenerate and hull `side` touches it.
    fn support_axes(&self, side: usize, intersection: &Hull) -> Vec<usize> {
        let hull = &self.hulls[side];
        (0..3)
            .filter(|&k| {
                intersection[k][0] == intersection[k][1]
                    && (hull[k][0] == intersection[k][0] || hull[k][1] == intersection[k][0])
            })
            .collect()
    }

    fn common_vertices(&self) -> Vec<usize> {
        let vertices = self.edges.each_ref().map(|set| {
            set.iter()
                .flat_map(|&e| self.model.edges[e].vertices)
                .collect::<BTreeSet<_>>()
        });
        vertices[0].intersection(&vertices[1]).copied().collect()
    }

    // Each face must actually own an exact endpoint at this vertex.
    fn owns_exact_endpoint(&self, vertex: usize, point: [f64; 3]) -> bool {
        self.edges.iter().all(|set| {
            set.iter().any(|&e| {
                edge_ends(self.model, e).is_some_and(|ends| {
                    (0..2).any(|i| self.model.edges[e].vertices[i] == vertex && ends[i] == point)
                })
            })
        })
    }

    // A 3D supporting plane may expose a single common topological vertex
    // even when all coordinate hull intervals overlap with positive width.
    fn spatial_vertex(&self) -> Option<Contact> {
        let poles = self.faces.map(|f| {
            self.model.faces[f]
                .surface
                .control_points
                .iter()
                .flatten()
                .collect::<Vec<_>>()
        });
        // Generated/axis proposals keep the same fixed iteration and predicate limits.
        // Larger supported nets are checked against every original pole.
        if !poles
            .iter()
            .all(|ps| ps.len() <= self.limits.proposal_poles)
        {
            return None;
        }
        for vertex in self.common_vertices() {
            let point = self.model.vertices[vertex].point;
            if !self.owns_exact_endpoint(vertex, point) {
                continue;
            }
            let mut candidates = axis_candidates();
            if let Some(anchors) = propose_vertex_plane(point, &poles) {
                candidates.extend(anchors);
            }
            // Quintic station walls have eighteen poles. For larger nets
            // retain only the bounded generated/axis proposals: enumerating
            // every pole pair would multiply exact predicate work.
            if poles.iter().all(|ps| ps.len() <= self.limits.anchor_poles) {
                candidates.extend(poles.iter().flat_map(|ps| ps.iter().map(|p| (*p).clone())));
            }
            for a in 0..candidates.len() {
                for b in a + 1..candidates.len() {
                    let sides = poles.each_ref().map(|ps| {
                        side_sign(ps, point, |p| {
                            crate::shared_boundary::orient(
                                &[&point, &candidates[a], &candidates[b], p],
                                None,
                            )
                        })
                    });
                    if strictly_separated(sides) {
                        return Some(Self::vertex_contact(vertex, point));
                    }
                }
            }
        }
        None
    }

    fn projected_vertex(&self, intersection: &Hull, free: &[usize]) -> Option<Contact> {
        let projections = if free.len() == 2 {
            vec![[free[0], free[1]]]
        } else {
            (0..3)
                .filter(|&k| k != free[0])
                .map(|k| [free[0], k])
                .collect()
        };
        // Shared station support can leave two transverse coordinates free.
        // An exact projected separator may restrict that contact to one
        // authored vertex even when its coordinate AABBs overlap in area.
        for axes in projections {
            for vertex in self.common_vertices() {
                if let Some(found) = self.projected_vertex_at(intersection, axes, vertex) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn projected_vertex_at(
        &self,
        intersection: &Hull,
        axes: [usize; 2],
        vertex: usize,
    ) -> Option<Contact> {
        let point = self.model.vertices[vertex].point;
        if (0..3).any(|k| point[k] < intersection[k][0] || point[k] > intersection[k][1]) {
            return None;
        }
        let poles = self.faces.map(|f| {
            let support = self.support_axes(usize::from(f == self.faces[1]), intersection);
            self.model.faces[f]
                .surface
                .control_points
                .iter()
                .flatten()
                .filter(|p| support.iter().all(|&k| p[k] == intersection[k][0]))
                .collect::<Vec<_>>()
        });
        let mut candidates = axis_candidates();
        // Coordinate support already restricts the contact to this plane.
        // A rotated corner need not be separated by a world-axis proposal;
        // use bounded candidate generation, then verify all retained
        // source poles with the exact projected orientation predicate.
        if poles
            .iter()
            .all(|ps| ps.len() <= self.limits.proposal_poles)
        {
            candidates.extend(propose_projected_anchor(point, &poles, axes));
        }
        for candidate in candidates {
            let sides = poles.each_ref().map(|ps| {
                side_sign(ps, point, |p| {
                    crate::shared_boundary::orient(&[&point, &candidate, p], Some(axes))
                })
            });
            if strictly_separated(sides) && self.owns_exact_endpoint(vertex, point) {
                return Some(Self::vertex_contact(vertex, point));
            }
        }
        None
    }

    // Require the same topological vertex and an exact curve endpoint on
    // both faces. Merely nearby vertex coordinates do not authorize contact.
    fn point_contact(&self, intersection: Hull) -> Option<Contact> {
        let point = intersection.map(|r| r[0]);
        let model = self.model;
        let vertices = self.edges.each_ref().map(|set| {
            set.iter()
                .flat_map(|&e| {
                    let ends = edge_ends(model, e);
                    (0..2).filter_map(move |i| {
                        ends.filter(|p| p[i] == point)
                            .map(|_| model.edges[e].vertices[i])
                    })
                })
                .collect::<BTreeSet<_>>()
        });
        let vertex = vertices[0]
            .intersection(&vertices[1])
            .copied()
            .find(|&v| model.vertices[v].point == point)?;
        Some(Contact {
            hull: intersection,
            edges: Vec::new(),
            vertex: Some(vertex),
        })
    }

    fn segment_contact(&self, intersection: Hull, axis: usize) -> Option<Contact> {
        let mut segments = Vec::new();
        for &e in self.edges[0].intersection(&self.edges[1]) {
            let c = &self.model.edges[e].curve;
            let Some(ends) = edge_ends(self.model, e) else {
                continue;
            };
            if c.control_points
                .iter()
                .any(|p| (0..3).any(|k| k != axis && p[k] != intersection[k][0]))
            {
                continue;
            }
            // Continuity covers the whole interval between endpoints. Exact lift
            // agreement places that interval on both authored face boundaries.
            segments.push((
                ends[0][axis].min(ends[1][axis]),
                ends[0][axis].max(ends[1][axis]),
                e,
            ));
        }
        segments.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut out = Contact {
            hull: intersection,
            edges: Vec::new(),
            vertex: None,
        };
        let mut covered = intersection[axis][0];
        for (lo, hi, e) in segments {
            if lo > covered {
                break;
            }
            if hi > covered {
                covered = hi;
                out.edges.push(e);
            }
            if covered >= intersection[axis][1] {
                return Some(out);
            }
        }
        None
    }
}
