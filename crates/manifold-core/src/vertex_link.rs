//! Strict vertex-link validation for solid (closed, oriented) meshes.
//!
//! Stronger than the connectivity-only fan check in [`crate::check`]: at a
//! valid solid vertex every link node must have degree exactly two (a single
//! closed cycle), so boundary vertices and nonmanifold links are both
//! rejected. This is the canonical implementation used by the boolean
//! kernels in `polygon-core`.

/// Why a vertex link is invalid for a solid mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexLinkDefect {
    /// A link node has degree other than two (nonmanifold or boundary
    /// vertex; repeated incidences count).
    NonManifoldLink(usize),
    /// The link graph splits into several components (pinched/bowtie fan).
    DisconnectedFan(usize),
}

impl VertexLinkDefect {
    /// The offending vertex index.
    pub fn vertex(self) -> usize {
        match self {
            Self::NonManifoldLink(v) | Self::DisconnectedFan(v) => v,
        }
    }
}

impl std::fmt::Display for VertexLinkDefect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonManifoldLink(v) => write!(f, "vertex {v} has a nonmanifold link"),
            Self::DisconnectedFan(v) => write!(f, "vertex {v} has a disconnected fan"),
        }
    }
}

impl std::error::Error for VertexLinkDefect {}

/// Validate every vertex link of a solid mesh. `indices` are triangle
/// corners over `vertex_count` vertices.
///
/// Reused scratch buffers (generation-stamped membership, degree/cursor
/// per link node, DFS stack) avoid per-vertex allocation on dense meshes.
/// Returns the first defect in vertex order, so the result is
/// deterministic.
pub fn validate_vertex_links(
    vertex_count: usize,
    indices: &[usize],
) -> Result<(), VertexLinkDefect> {
    let triangles = indices.as_chunks::<3>().0;
    // CSR fan edges: fan[offsets[v]..offsets[v + 1]] holds the link edges
    // (next, previous) of every triangle corner at v, in triangle order.
    let mut offsets = vec![0usize; vertex_count + 1];
    for t in triangles {
        for &id in t {
            offsets[id + 1] += 1;
        }
    }
    for v in 0..vertex_count {
        offsets[v + 1] += offsets[v];
    }
    let mut fan = vec![(0usize, 0usize); offsets[vertex_count]];
    let mut cursors = offsets[..vertex_count].to_vec();
    for t in triangles {
        for i in 0..3 {
            let cursor = &mut cursors[t[i]];
            fan[*cursor] = (t[(i + 1) % 3], t[(i + 2) % 3]);
            *cursor += 1;
        }
    }
    let mut link_mark = vec![usize::MAX; vertex_count];
    let mut seen_mark = vec![usize::MAX; vertex_count];
    let mut degree = vec![0usize; vertex_count];
    let mut slot = vec![usize::MAX; vertex_count];
    let mut members: Vec<usize> = Vec::new();
    let mut link_offsets: Vec<usize> = Vec::new();
    let mut adjacent: Vec<usize> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for v in 0..vertex_count {
        let range = offsets[v]..offsets[v + 1];
        if range.is_empty() {
            continue;
        }
        let edges = &fan[range];
        members.clear();
        for &(a, b) in edges {
            for id in [a, b] {
                if link_mark[id] != v {
                    link_mark[id] = v;
                    degree[id] = 0;
                    slot[id] = members.len();
                    members.push(id);
                }
                degree[id] += 1;
            }
        }
        // Repeated incidences count, as in the previous map-based version.
        if members.iter().any(|&id| degree[id] != 2) {
            return Err(VertexLinkDefect::NonManifoldLink(v));
        }
        // Flat per-link adjacency (CSR over `members`) for the DFS below.
        link_offsets.clear();
        link_offsets.push(0);
        for &id in &members {
            link_offsets.push(link_offsets.last().unwrap() + degree[id]);
        }
        adjacent.clear();
        adjacent.resize(*link_offsets.last().unwrap(), 0);
        for &id in &members {
            degree[id] = link_offsets[slot[id]];
        }
        for &(a, b) in edges {
            adjacent[degree[a]] = b;
            degree[a] += 1;
            adjacent[degree[b]] = a;
            degree[b] += 1;
        }
        stack.clear();
        stack.push(members[0]);
        seen_mark[members[0]] = v;
        let mut seen = 1usize;
        while let Some(w) = stack.pop() {
            for &n in &adjacent[link_offsets[slot[w]]..link_offsets[slot[w] + 1]] {
                if seen_mark[n] != v {
                    seen_mark[n] = v;
                    seen += 1;
                    stack.push(n);
                }
            }
        }
        if seen != members.len() {
            return Err(VertexLinkDefect::DisconnectedFan(v));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_cube_passes() {
        let indices = vec![
            0, 2, 1, 0, 3, 2, // bottom
            4, 5, 6, 4, 6, 7, // top
            0, 1, 5, 0, 5, 4, // front
            2, 3, 7, 2, 7, 6, // back
            1, 2, 6, 1, 6, 5, // right
            3, 0, 4, 3, 4, 7, // left
        ];
        assert_eq!(validate_vertex_links(8, &indices), Ok(()));
    }

    #[test]
    fn boundary_vertex_fails_link_degree() {
        // Single open quad: corner vertices have degree-1 link nodes.
        let indices = vec![0, 1, 2, 0, 2, 3];
        assert!(matches!(
            validate_vertex_links(4, &indices),
            Err(VertexLinkDefect::NonManifoldLink(_))
        ));
    }

    #[test]
    fn pinched_vertex_fails_disconnected() {
        // Two fans touching only at vertex 0; every link node has degree 2,
        // but the fan at 0 splits in two.
        let indices = vec![0, 1, 2, 0, 2, 1, 0, 3, 4, 0, 4, 3];
        assert_eq!(
            validate_vertex_links(5, &indices),
            Err(VertexLinkDefect::DisconnectedFan(0))
        );
    }
}
