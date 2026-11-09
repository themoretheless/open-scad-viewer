//! Atomic Euler operators over the existing indexed B-rep representation.
//!
//! These operators edit incidence only; callers supply the curve geometry for
//! newly created or merged edges. Every operation revalidates the complete
//! candidate, so invalid incidence, non-manifold links, NaN coordinates and
//! resource-limit violations never publish a partial edit.
use super::*;

pub const EULER_INVALID_OPERATION: &str = "BREP_EULER_INVALID_OPERATION";
pub const EULER_UNSUPPORTED: &str = "BREP_EULER_UNSUPPORTED";

fn euler_invalid(message: impl Into<String>) -> Error {
    Error::new(EULER_INVALID_OPERATION, message)
}

fn euler_unsupported(message: impl Into<String>) -> Error {
    Error::new(EULER_UNSUPPORTED, message)
}

fn require_euler(ok: bool, message: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(euler_invalid(message))
    }
}

fn coedge_start<V>(
    model: &Model<impl Sized, impl Sized, impl Sized, V>,
    edge: usize,
    reversed: bool,
) -> usize {
    model.edges[edge].vertices[usize::from(reversed)]
}

fn path_between<P: Clone>(coedges: &[Coedge<P>], from: usize, to: usize) -> Vec<Coedge<P>> {
    let mut path = Vec::new();
    let mut index = from;
    while index != to {
        path.push(coedges[index].clone());
        index = (index + 1) % coedges.len();
    }
    path
}

fn without_at<P: Clone>(coedges: &[Coedge<P>], removed: usize) -> Vec<Coedge<P>> {
    (1..coedges.len())
        .map(|offset| coedges[(removed + offset) % coedges.len()].clone())
        .collect()
}

impl<C: Clone, S: Clone, P: Clone> Model<C, S, P> {
    /// Split an edge at a new vertex (MEV), duplicating every face-local use.
    ///
    /// `first_curve` replaces the original edge curve and `second_curve` is the
    /// appended second half. Trimmed coedges are refused because their 3D and
    /// UV intervals require owner-specific splitting evidence.
    pub fn make_edge_vertex(
        &self,
        edge: usize,
        vertex: [f64; 3],
        first_curve: C,
        second_curve: C,
    ) -> Result<Self> {
        self.validate_topology()?;
        let original = self
            .edges
            .get(edge)
            .ok_or_else(|| euler_invalid("MEV references an unknown edge"))?;
        require_euler(!original.degenerate, "MEV cannot split a pole edge")?;
        require_euler(
            original.vertices[0] != original.vertices[1],
            "MEV requires an edge with distinct endpoints",
        )?;
        if self
            .loops
            .iter()
            .flat_map(|loop_| &loop_.coedges)
            .any(|coedge| coedge.edge == edge && coedge.trim().is_some())
        {
            return Err(euler_unsupported(
                "MEV of trimmed coedges requires explicit UV split parameters",
            ));
        }

        let mut next = self.clone();
        let new_vertex = next.vertices.len();
        let new_edge = next.edges.len();
        let end = next.edges[edge].vertices[1];
        next.vertices.push(Vertex { point: vertex });
        next.edges[edge].vertices[1] = new_vertex;
        next.edges[edge].curve = first_curve;
        next.edges.push(Edge {
            degenerate: false,
            vertices: [new_vertex, end],
            curve: second_curve,
        });
        for loop_ in &mut next.loops {
            let mut coedges = Vec::with_capacity(loop_.coedges.len() + 1);
            for coedge in loop_.coedges.drain(..) {
                if coedge.edge != edge {
                    coedges.push(coedge);
                    continue;
                }
                let second = Coedge::new(new_edge, coedge.reversed, coedge.pcurve.clone());
                if coedge.reversed {
                    coedges.push(second);
                    coedges.push(coedge);
                } else {
                    coedges.push(coedge);
                    coedges.push(second);
                }
            }
            loop_.coedges = coedges;
        }
        next.validate_topology()?;
        Ok(next)
    }

    /// Merge the two edges at a degree-two vertex and remove that vertex (KEV).
    /// This is the inverse of MEV when `merged_curve` restores the original curve.
    pub fn kill_edge_vertex(&self, edge: usize, vertex: usize, merged_curve: C) -> Result<Self> {
        self.validate_topology()?;
        let removed = self
            .edges
            .get(edge)
            .ok_or_else(|| euler_invalid("KEV references an unknown edge"))?;
        require_euler(!removed.degenerate, "KEV cannot remove a pole edge")?;
        require_euler(
            removed.vertices.contains(&vertex),
            "KEV vertex is not incident to the removed edge",
        )?;
        let incident: Vec<usize> = self
            .edges
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                (!candidate.degenerate && candidate.vertices.contains(&vertex)).then_some(index)
            })
            .collect();
        require_euler(
            incident.len() == 2 && incident.contains(&edge),
            "KEV requires a vertex of degree two",
        )?;
        let survivor_edge = *incident.iter().find(|&&index| index != edge).unwrap();
        let survivor_vertex = self.edges[survivor_edge].vertices
            [usize::from(self.edges[survivor_edge].vertices[0] == vertex)];
        let removed_other = removed.vertices[usize::from(removed.vertices[0] == vertex)];
        require_euler(
            survivor_vertex != removed_other
                && survivor_vertex != vertex
                && removed_other != vertex,
            "KEV cannot merge a closed or repeated edge",
        )?;
        if self
            .loops
            .iter()
            .flat_map(|loop_| &loop_.coedges)
            .any(|coedge| {
                matches!(coedge.edge, index if index == edge || index == survivor_edge)
                    && coedge.trim().is_some()
            })
        {
            return Err(euler_unsupported(
                "KEV of trimmed coedges requires explicit merged UV parameters",
            ));
        }

        let mut next = self.clone();
        let mut merged_vertices = None;
        for loop_ in &mut next.loops {
            let positions: Vec<usize> = loop_
                .coedges
                .iter()
                .enumerate()
                .filter_map(|(index, coedge)| (coedge.edge == edge).then_some(index))
                .collect();
            require_euler(
                positions.len() <= 1,
                "KEV cannot merge repeated uses in one loop",
            )?;
            let Some(&position) = positions.first() else {
                continue;
            };
            let coedge = loop_.coedges[position].clone();
            let len = loop_.coedges.len();
            let previous = (position + len - 1) % len;
            let following = (position + 1) % len;
            let candidate = if !coedge.reversed {
                previous
            } else {
                following
            };
            require_euler(
                candidate != position && loop_.coedges[candidate].edge == survivor_edge,
                "KEV edge pair is not a mergeable directed chain",
            )?;
            let retained = loop_.coedges[candidate].clone();
            let candidate_before = candidate == previous;
            let (directed_start, directed_end) = if candidate_before {
                (survivor_vertex, removed_other)
            } else {
                (removed_other, survivor_vertex)
            };
            let vertices = if retained.reversed {
                [directed_end, directed_start]
            } else {
                [directed_start, directed_end]
            };
            if let Some(expected) = merged_vertices {
                require_euler(
                    expected == vertices,
                    "KEV uses disagree on merged direction",
                )?;
            } else {
                merged_vertices = Some(vertices);
            }
            let first = position.min(candidate);
            let last = position.max(candidate);
            let mut coedges = Vec::with_capacity(len - 1);
            for (index, existing) in loop_.coedges.iter().cloned().enumerate() {
                if index == first {
                    coedges.push(retained.clone());
                }
                if index != first && index != last {
                    coedges.push(existing);
                }
            }
            loop_.coedges = coedges;
        }
        next.edges[survivor_edge].vertices =
            merged_vertices.ok_or_else(|| euler_invalid("KEV edge has no face use"))?;
        next.edges[survivor_edge].curve = merged_curve;
        next.edges.remove(edge);
        next.vertices.remove(vertex);
        for edge_ in &mut next.edges {
            for endpoint in &mut edge_.vertices {
                if *endpoint > vertex {
                    *endpoint -= 1;
                }
            }
        }
        for coedge in next.loops.iter_mut().flat_map(|loop_| &mut loop_.coedges) {
            if coedge.edge > edge {
                coedge.edge -= 1;
            }
        }
        next.validate_topology()?;
        Ok(next)
    }

    /// Split one hole-free face by an edge between two loop vertices (MEF).
    /// The new face is appended to the same shell with the same face reversal.
    pub fn make_edge_face(
        &self,
        face: usize,
        first_coedge: usize,
        second_coedge: usize,
        curve: C,
        pcurve: P,
    ) -> Result<Self> {
        self.validate_topology()?;
        let original = self
            .faces
            .get(face)
            .ok_or_else(|| euler_invalid("MEF references an unknown face"))?;
        if !original.holes.is_empty() {
            return Err(euler_unsupported(
                "MEF hole redistribution requires face-domain ownership evidence",
            ));
        }
        let loop_ = &self.loops[original.outer];
        require_euler(
            first_coedge < loop_.coedges.len()
                && second_coedge < loop_.coedges.len()
                && first_coedge != second_coedge,
            "MEF requires two distinct coedge positions",
        )?;
        let first = &loop_.coedges[first_coedge];
        let second = &loop_.coedges[second_coedge];
        let start = coedge_start(self, first.edge, first.reversed);
        let end = coedge_start(self, second.edge, second.reversed);
        require_euler(start != end, "MEF requires distinct vertices")?;
        let first_path = path_between(&loop_.coedges, first_coedge, second_coedge);
        let second_path = path_between(&loop_.coedges, second_coedge, first_coedge);
        require_euler(
            !first_path.is_empty() && !second_path.is_empty(),
            "MEF produced an empty face boundary",
        )?;

        let mut next = self.clone();
        let new_edge = next.edges.len();
        let new_loop = next.loops.len();
        let new_face = next.faces.len();
        next.edges.push(Edge {
            degenerate: false,
            vertices: [start, end],
            curve,
        });
        let mut first_loop = first_path;
        first_loop.push(Coedge::new(new_edge, true, pcurve.clone()));
        let mut second_loop = vec![Coedge::new(new_edge, false, pcurve)];
        second_loop.extend(second_path);
        next.loops[original.outer].coedges = first_loop;
        next.loops.push(Loop {
            coedges: second_loop,
        });
        next.faces.push(Face {
            surface: original.surface.clone(),
            outer: new_loop,
            holes: Vec::new(),
        });
        for shell in &mut next.shells {
            if let Some(position) = shell.faces.iter().position(|use_| use_.face == face) {
                let reversed = shell.faces[position].reversed;
                shell.faces.insert(
                    position + 1,
                    FaceUse {
                        face: new_face,
                        reversed,
                    },
                );
            }
        }
        next.validate_topology()?;
        Ok(next)
    }

    /// Remove an edge shared by two hole-free faces and join their outer loops
    /// (KEF), the inverse of MEF.
    pub fn kill_edge_face(&self, edge: usize) -> Result<Self> {
        self.validate_topology()?;
        require_euler(edge < self.edges.len(), "KEF references an unknown edge")?;
        require_euler(
            !self.edges[edge].degenerate,
            "KEF cannot remove a pole edge",
        )?;
        let uses: Vec<(usize, usize, usize)> = self
            .faces
            .iter()
            .enumerate()
            .flat_map(|(face_index, face)| {
                std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .map(move |loop_index| (face_index, loop_index))
            })
            .flat_map(|(face_index, loop_index)| {
                self.loops[loop_index]
                    .coedges
                    .iter()
                    .enumerate()
                    .filter_map(move |(position, coedge)| {
                        (coedge.edge == edge).then_some((face_index, loop_index, position))
                    })
            })
            .collect();
        require_euler(
            uses.len() == 2 && uses[0].0 != uses[1].0,
            "KEF requires one edge use in each of two distinct faces",
        )?;
        if uses.iter().any(|&(face, loop_index, position)| {
            !self.faces[face].holes.is_empty()
                || self.loops[loop_index].coedges[position].trim().is_some()
        }) {
            return Err(euler_unsupported(
                "KEF requires hole-free faces and untrimmed removed coedges",
            ));
        }
        let (keep_face, keep_loop, keep_position) = uses[0];
        let (kill_face, kill_loop, kill_position) = uses[1];
        require_euler(
            keep_loop == self.faces[keep_face].outer && kill_loop == self.faces[kill_face].outer,
            "KEF only merges outer loops",
        )?;
        let mut merged = without_at(&self.loops[keep_loop].coedges, keep_position);
        merged.extend(without_at(&self.loops[kill_loop].coedges, kill_position));
        require_euler(merged.len() >= 2, "KEF produced an empty face boundary")?;

        let mut next = self.clone();
        next.loops[keep_loop].coedges = merged;
        next.loops.remove(kill_loop);
        next.faces.remove(kill_face);
        next.edges.remove(edge);
        for face_ in &mut next.faces {
            for loop_index in std::iter::once(&mut face_.outer).chain(&mut face_.holes) {
                if *loop_index > kill_loop {
                    *loop_index -= 1;
                }
            }
        }
        for shell in &mut next.shells {
            shell.faces.retain_mut(|use_| {
                if use_.face == kill_face {
                    return false;
                }
                if use_.face > kill_face {
                    use_.face -= 1;
                }
                true
            });
        }
        for coedge in next.loops.iter_mut().flat_map(|loop_| &mut loop_.coedges) {
            if coedge.edge > edge {
                coedge.edge -= 1;
            }
        }
        next.validate_topology()?;
        Ok(next)
    }
}
