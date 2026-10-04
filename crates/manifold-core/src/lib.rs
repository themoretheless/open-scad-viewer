//! Manifoldness analysis for indexed triangle meshes.
//!
//! A triangle soup is a 2-manifold ("watertight") surface when:
//! - every undirected edge is shared by exactly two triangles;
//! - the two triangles sharing an edge traverse it in opposite directions
//!   (consistent orientation);
//! - the link of every vertex is a single connected cycle (no pinched fans);
//! - no degenerate (zero-area or repeated-index) triangles exist.
//!
//! Boundary edges (used once) are reported separately: they break
//! watertightness but are not "non-manifold" in the strict sense.
//!
//! This crate is the single public home for manifold checks that were
//! previously scattered across `polygon-core` (private `vertex_manifold`),
//! `brep-core` (`close_topology`) and the TypeScript mesh services.

/// Undirected edge key with canonical ordering.
pub type EdgeKey = (usize, usize);

pub(crate) fn edge_key(a: usize, b: usize) -> EdgeKey {
    if a < b { (a, b) } else { (b, a) }
}

pub mod repair;
pub mod vertex_link;

pub use repair::{RepairOutcome, RepairReport, repair};
pub use vertex_link::{VertexLinkDefect, validate_vertex_links};

/// Structured manifoldness report for an indexed triangle mesh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifoldReport {
    /// `positions.len() / 3`.
    pub vertex_count: usize,
    /// `indices.len() / 3`.
    pub triangle_count: usize,
    /// Undirected edges used by exactly one triangle.
    pub boundary_edges: Vec<EdgeKey>,
    /// Undirected edges used by three or more triangles.
    pub non_manifold_edges: Vec<EdgeKey>,
    /// Directed half-edges traversed in the same direction by both
    /// incident triangles (orientation mismatch). Canonicalised, deduped.
    pub orientation_edges: Vec<EdgeKey>,
    /// Triangle indices with a repeated corner (a == b, b == c, or a == c).
    pub degenerate_triangles: Vec<usize>,
    /// Vertices whose incident triangle fan is disconnected
    /// (pinched / bowtie vertices). Edge counts alone miss these.
    pub non_manifold_vertices: Vec<usize>,
    /// Vertices referenced by no triangle.
    pub isolated_vertices: Vec<usize>,
    /// Connected components of the face adjacency graph (dual graph).
    /// 0 when the mesh has no triangles.
    pub component_count: usize,
}

impl ManifoldReport {
    /// Strict 2-manifold: every edge has exactly two consistently oriented
    /// triangles, every vertex link is one cycle, no degenerates.
    pub fn is_manifold(&self) -> bool {
        self.boundary_edges.is_empty()
            && self.non_manifold_edges.is_empty()
            && self.orientation_edges.is_empty()
            && self.degenerate_triangles.is_empty()
            && self.non_manifold_vertices.is_empty()
    }

    /// Manifold except for open boundaries (a valid surface with holes).
    pub fn is_manifold_with_boundary(&self) -> bool {
        self.non_manifold_edges.is_empty()
            && self.orientation_edges.is_empty()
            && self.degenerate_triangles.is_empty()
            && self.non_manifold_vertices.is_empty()
    }
}

/// One corner fan adjacency built in CSR layout over triangle corners.
struct VertexFans {
    offsets: Vec<usize>,
    /// Per corner: (next, previous) link edge of its triangle at the vertex.
    link: Vec<(usize, usize)>,
}

fn build_vertex_fans(vertex_count: usize, triangles: &[[usize; 3]]) -> VertexFans {
    let mut offsets = vec![0usize; vertex_count + 1];
    for t in triangles {
        for &id in t {
            offsets[id + 1] += 1;
        }
    }
    for v in 0..vertex_count {
        offsets[v + 1] += offsets[v];
    }
    let mut link = vec![(0usize, 0usize); offsets[vertex_count]];
    let mut cursors = offsets[..vertex_count].to_vec();
    for t in triangles {
        for i in 0..3 {
            let cursor = &mut cursors[t[i]];
            link[*cursor] = (t[(i + 1) % 3], t[(i + 2) % 3]);
            *cursor += 1;
        }
    }
    VertexFans { offsets, link }
}

/// Vertices whose link graph (fan) is not a single connected component.
/// Generation-stamped scratch buffers avoid per-vertex allocation on
/// dense meshes.
fn disconnected_fans(fans: &VertexFans, vertex_count: usize) -> Vec<usize> {
    let mut link_mark = vec![usize::MAX; vertex_count];
    let mut seen_mark = vec![usize::MAX; vertex_count];
    let mut degree = vec![0usize; vertex_count];
    let mut slot = vec![0usize; vertex_count];
    let mut member_index = vec![usize::MAX; vertex_count];
    let mut members: Vec<usize> = Vec::new();
    let mut link_offsets: Vec<usize> = Vec::new();
    let mut adjacent: Vec<usize> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut bad = Vec::new();

    for v in 0..vertex_count {
        let range = fans.offsets[v]..fans.offsets[v + 1];
        if range.is_empty() {
            continue;
        }
        // Deduplicate link nodes of this fan (both link-edge endpoints).
        members.clear();
        for &(next, prev) in &fans.link[range.clone()] {
            for u in [next, prev] {
                if link_mark[u] != v {
                    link_mark[u] = v;
                    member_index[u] = members.len();
                    degree[u] = 0;
                    members.push(u);
                }
            }
        }
        // Build adjacency over link nodes via link edges.
        for &(next, prev) in &fans.link[range.clone()] {
            degree[next] += 1;
            degree[prev] += 1;
        }
        link_offsets.clear();
        link_offsets.push(0);
        for &u in &members {
            link_offsets.push(link_offsets.last().unwrap() + degree[u]);
            degree[u] = 0;
            slot[u] = 0;
        }
        adjacent.clear();
        adjacent.resize(*link_offsets.last().unwrap(), 0usize);
        // Recompute slot bases: link_offsets[u_index] is the base of u.
        for &(next, prev) in &fans.link[range.clone()] {
            let ni = member_index[next];
            let pi = member_index[prev];
            let bn = link_offsets[ni];
            let bp = link_offsets[pi];
            adjacent[bn + slot[next]] = prev;
            slot[next] += 1;
            adjacent[bp + slot[prev]] = next;
            slot[prev] += 1;
        }
        // Count connected components of the link graph.
        let mut components = 0usize;
        stack.clear();
        for (i, &u) in members.iter().enumerate() {
            if seen_mark[u] == v {
                continue;
            }
            components += 1;
            seen_mark[u] = v;
            stack.push(i);
            while let Some(cur) = stack.pop() {
                let cu = members[cur];
                let base = link_offsets[cur];
                for k in 0..slot[cu] {
                    let w = adjacent[base + k];
                    if seen_mark[w] != v {
                        seen_mark[w] = v;
                        stack.push(member_index[w]);
                    }
                }
            }
        }
        if components > 1 {
            bad.push(v);
        }
    }
    bad
}

/// Analyse an indexed triangle mesh (`positions` as xyz triples,
/// `indices` as triangle corners) and return a full manifoldness report.
pub fn check(positions: &[f64], indices: &[usize]) -> ManifoldReport {
    let vertex_count = positions.len() / 3;
    let triangles: Vec<[usize; 3]> = indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| [t[0], t[1], t[2]])
        .collect();

    let mut degenerate_triangles = Vec::new();
    // Degenerate corners pollute edge statistics; edge incidence is computed
    // by the canonical packed-edge kernel in mesh-topology over the surviving
    // triangles only.
    let mut edge_indices: Vec<usize> = Vec::with_capacity(indices.len());
    for (ti, t) in triangles.iter().enumerate() {
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
            degenerate_triangles.push(ti);
            continue;
        }
        edge_indices.extend_from_slice(t);
    }
    let edge_uses = mesh_topology::EdgeUses::new(&edge_indices);
    let boundary_edges: Vec<EdgeKey> = edge_uses.boundary().map(|[a, b]| edge_key(a, b)).collect();
    let (non_manifold_edges, orientation_edges) = edge_uses.defects();
    let non_manifold_edges: Vec<EdgeKey> = non_manifold_edges
        .into_iter()
        .map(|[a, b]| (a, b))
        .collect();
    let orientation_edges: Vec<EdgeKey> =
        orientation_edges.into_iter().map(|[a, b]| (a, b)).collect();

    let fans = build_vertex_fans(vertex_count, &triangles);
    let non_manifold_vertices = disconnected_fans(&fans, vertex_count);
    let isolated_vertices = (0..vertex_count)
        .filter(|&v| fans.offsets[v] == fans.offsets[v + 1])
        .collect();

    // Connected components via face adjacency over shared edges.
    let component_count = count_components(&triangles);

    ManifoldReport {
        vertex_count,
        triangle_count: triangles.len(),
        boundary_edges,
        non_manifold_edges,
        orientation_edges,
        degenerate_triangles,
        non_manifold_vertices,
        isolated_vertices,
        component_count,
    }
}

/// Convenience predicate: strict closed 2-manifold.
pub fn is_manifold(positions: &[f64], indices: &[usize]) -> bool {
    check(positions, indices).is_manifold()
}

fn count_components(triangles: &[[usize; 3]]) -> usize {
    if triangles.is_empty() {
        return 0;
    }
    // Union-find over triangles connected by shared edges: an edge seen for
    // the second time joins its two incident triangles (boundary edges never
    // repeat and therefore never join anything).
    let mut parent: Vec<usize> = (0..triangles.len()).collect();
    fn find(parent: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while parent[r] != r {
            r = parent[r];
        }
        let mut c = x;
        while parent[c] != r {
            let n = parent[c];
            parent[c] = r;
            c = n;
        }
        r
    }
    let mut owner: std::collections::HashMap<EdgeKey, usize> = std::collections::HashMap::new();
    for (ti, t) in triangles.iter().enumerate() {
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
            continue;
        }
        for i in 0..3 {
            let key = edge_key(t[i], t[(i + 1) % 3]);
            if let Some(other) = owner.insert(key, ti) {
                let (ra, rb) = (find(&mut parent, other), find(&mut parent, ti));
                if ra != rb {
                    parent[ra] = rb;
                }
            }
        }
    }
    let mut roots = std::collections::HashSet::new();
    for i in 0..triangles.len() {
        roots.insert(find(&mut parent, i));
    }
    roots.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit cube: 8 vertices, 12 triangles, consistently oriented (CCW
    /// seen from outside).
    fn cube() -> (Vec<f64>, Vec<usize>) {
        let positions = vec![
            0.0, 0.0, 0.0, // 0
            1.0, 0.0, 0.0, // 1
            1.0, 1.0, 0.0, // 2
            0.0, 1.0, 0.0, // 3
            0.0, 0.0, 1.0, // 4
            1.0, 0.0, 1.0, // 5
            1.0, 1.0, 1.0, // 6
            0.0, 1.0, 1.0, // 7
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, // bottom (z = 0, outward -z)
            4, 5, 6, 4, 6, 7, // top (z = 1, outward +z)
            0, 1, 5, 0, 5, 4, // front (y = 0)
            2, 3, 7, 2, 7, 6, // back (y = 1)
            1, 2, 6, 1, 6, 5, // right (x = 1)
            3, 0, 4, 3, 4, 7, // left (x = 0)
        ];
        (positions, indices)
    }

    #[test]
    fn closed_cube_is_manifold() {
        let (p, i) = cube();
        let report = check(&p, &i);
        assert!(report.is_manifold(), "{report:?}");
        assert!(is_manifold(&p, &i));
        assert_eq!(report.component_count, 1);
        assert_eq!(report.triangle_count, 12);
        assert!(report.isolated_vertices.is_empty());
    }

    #[test]
    fn open_mesh_reports_boundary_only() {
        // Single quad (two triangles): manifold with boundary.
        let p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        let i = vec![0, 1, 2, 0, 2, 3];
        let report = check(&p, &i);
        assert!(!report.is_manifold());
        assert!(report.is_manifold_with_boundary(), "{report:?}");
        assert_eq!(report.boundary_edges.len(), 4);
    }

    #[test]
    fn triple_edge_is_non_manifold() {
        // Three triangles share edge (0,1).
        let p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0,
        ];
        let i = vec![0, 1, 2, 0, 4, 1, 0, 1, 3];
        let report = check(&p, &i);
        assert!(
            report.non_manifold_edges.contains(&edge_key(0, 1)),
            "{report:?}"
        );
        assert!(!report.is_manifold());
    }

    #[test]
    fn flipped_triangle_is_orientation_mismatch() {
        // Quad where the second triangle is flipped.
        let p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        let i = vec![0, 1, 2, 0, 3, 2]; // (0,2) traversed 2->0 and 0->2? second: 0->3,3->2,2->0
        let report = check(&p, &i);
        assert_eq!(report.orientation_edges, vec![edge_key(0, 2)]);
        assert!(!report.is_manifold_with_boundary());
    }

    #[test]
    fn pinched_vertex_is_detected() {
        // Two tetra-like fans touching only at vertex 0 (bowtie).
        let p = vec![
            0.0, 0.0, 0.0, // 0 shared tip
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, // fan A ring
            0.0, 0.0, 1.0, -1.0, 0.0, 0.0, // fan B ring
        ];
        let i = vec![0, 1, 2, 0, 3, 4];
        let report = check(&p, &i);
        assert_eq!(report.non_manifold_vertices, vec![0]);
        assert!(!report.is_manifold());
    }

    #[test]
    fn degenerate_triangle_is_reported() {
        let p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let i = vec![0, 1, 1];
        let report = check(&p, &i);
        assert_eq!(report.degenerate_triangles, vec![0]);
        assert_eq!(report.isolated_vertices, vec![2]);
    }

    #[test]
    fn disconnected_components_are_counted() {
        let (p, i) = cube();
        // Two separate quads: two components, four boundary edges each.
        let report = check(&p[..12], &i[..6]); // bottom face only -> 1 component
        assert_eq!(report.component_count, 1);
        let both = check(&p, &[i[..6].to_vec(), i[6..12].to_vec()].concat()); // bottom + top
        assert_eq!(both.component_count, 2);
    }
}
