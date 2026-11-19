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

/// Version of the check/repair/metrics report contract. Bump it whenever a
/// report gains, loses, renames or reorders a field; the geometry bridge
/// republishes it in its Manifold report schema so the TypeScript wrappers
/// cannot drift silently.
pub const REPORT_SCHEMA_VERSION: u32 = 3;

/// Undirected edge key with canonical ordering: always `(min, max)`.
pub type EdgeKey = (usize, usize);

pub(crate) fn edge_key(a: usize, b: usize) -> EdgeKey {
    if a < b { (a, b) } else { (b, a) }
}

pub mod diagnostics;
mod metrics;
pub mod repair;
mod validation;
pub mod vertex_link;

pub use diagnostics::{
    ClearanceOptions, ClearancePair, ClearanceReport, DiagnosticsError, MassProperties,
    SelfIntersectionOptions, SelfIntersectionReport, ThicknessOptions, ThicknessReport, clearance,
    mass_properties, self_intersections, wall_thickness,
};
pub use diagnostics::predicates::{orient2d, orient3d};
pub use metrics::{ComponentMetrics, MeshMetrics, metrics};
pub use repair::{
    HoleFillLimits, RepairBudget, RepairBudgetExhausted, RepairBudgetKind, RepairMode,
    RepairOptions, RepairOutcome, RepairReport, repair, repair_with_mode, repair_with_options,
};
pub use validation::{IndexedMeshError, validate_indexed_mesh};
pub use vertex_link::{VertexLinkDefect, validate_vertex_links};

/// Structured manifoldness report for an indexed triangle mesh.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
    /// Triangles with three distinct corners whose area is exactly zero
    /// (collinear or coincident positions). Ascending.
    pub zero_area_triangles: Vec<usize>,
    /// Triangles with three distinct corners and `0 < area <=
    /// CheckOptions::sliver_area_threshold` (slivers and needles). Ascending;
    /// empty with the default threshold of `0.0`.
    pub sliver_triangles: Vec<usize>,
    /// Vertices whose incident triangle fan is disconnected
    /// (pinched / bowtie vertices). Edge counts alone miss these.
    pub non_manifold_vertices: Vec<usize>,
    /// Vertices referenced by no triangle.
    pub isolated_vertices: Vec<usize>,
    /// Connected components of the face adjacency graph (dual graph).
    /// 0 when the mesh has no triangles. Triangles touching only at a vertex
    /// are already separate components here.
    pub component_count: usize,
    /// Components when triangles are joined only across manifold edges
    /// (exactly two incident triangles), so shells meeting at a
    /// non-manifold edge or vertex are counted separately. 0 when empty.
    pub manifold_component_count: usize,
    /// Manifold components (see `manifold_component_count`) that cannot be
    /// consistently oriented: propagating orientation across their manifold
    /// edges reaches a triangle with both orientations (Möbius strip, Klein
    /// bottle). Orientation repair cannot fix these.
    pub non_orientable_components: usize,
}

/// Tunables for [`check_with_options`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckOptions {
    /// Triangles with distinct corners and area at or below this value (but
    /// above zero) are reported as `sliver_triangles`. Negative or NaN is
    /// treated as `0.0`.
    pub sliver_area_threshold: f64,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            sliver_area_threshold: 0.0,
        }
    }
}

/// Result of [`remove_isolated_vertices`].
#[derive(Debug, Clone, PartialEq)]
pub struct IsolatedVertexRemoval {
    /// Compacted positions (referenced vertices only, original order).
    pub positions: Vec<f64>,
    /// Indices rewritten into the compacted vertex numbering.
    pub indices: Vec<usize>,
    /// Per original vertex: its new index, or `None` when it was removed.
    pub remap: Vec<Option<usize>>,
    /// Number of removed vertices.
    pub removed: usize,
}

/// Drop vertices referenced by no triangle, keeping the order of the rest,
/// and return the old-to-new index remap. Indices must be in range.
pub fn remove_isolated_vertices(positions: &[f64], indices: &[usize]) -> IsolatedVertexRemoval {
    let vertex_count = positions.len() / 3;
    let mut used = vec![false; vertex_count];
    for &i in indices {
        used[i] = true;
    }
    let mut remap = vec![None; vertex_count];
    let mut out = Vec::with_capacity(positions.len());
    for v in 0..vertex_count {
        if used[v] {
            remap[v] = Some(out.len() / 3);
            out.extend_from_slice(&positions[v * 3..v * 3 + 3]);
        }
    }
    let removed = vertex_count - out.len() / 3;
    IsolatedVertexRemoval {
        indices: indices.iter().map(|&i| remap[i].expect("every referenced vertex was marked used and remapped")).collect(),
        positions: out,
        remap,
        removed,
    }
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
///
/// Invariants:
/// - memory is O(V + F); time is O(V + F) plus the edge sort inside
///   `mesh_topology::EdgeUses`;
/// - the report is a pure function of the input and every defect list has a
///   deterministic order: edges as canonical `(min, max)` keys in ascending
///   order, triangle and vertex indices ascending;
/// - an empty mesh yields an empty report with `component_count == 0`.
pub fn check(positions: &[f64], indices: &[usize]) -> ManifoldReport {
    check_with_options(positions, indices, &CheckOptions::default())
}

/// [`check`] with explicit [`CheckOptions`].
pub fn check_with_options(
    positions: &[f64],
    indices: &[usize],
    options: &CheckOptions,
) -> ManifoldReport {
    let sliver_threshold = if options.sliver_area_threshold > 0.0 {
        options.sliver_area_threshold
    } else {
        0.0
    };
    let vertex_count = positions.len() / 3;
    let triangles: Vec<[usize; 3]> = indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| [t[0], t[1], t[2]])
        .collect();

    let mut degenerate_triangles = Vec::new();
    let mut zero_area_triangles = Vec::new();
    let mut sliver_triangles = Vec::new();
    // Degenerate corners pollute edge statistics; edge incidence is computed
    // by the canonical packed-edge kernel in mesh-topology over the surviving
    // triangles only.
    let mut edge_indices: Vec<usize> = Vec::with_capacity(indices.len());
    for (ti, t) in triangles.iter().enumerate() {
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
            degenerate_triangles.push(ti);
            continue;
        }
        let area = triangle_area(positions, t);
        if area == 0.0 {
            zero_area_triangles.push(ti);
        } else if area <= sliver_threshold {
            sliver_triangles.push(ti);
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
    // One sorted edge-use list serves both passes (it replaced two hash maps
    // that dominated `check` on 1M+ triangle meshes).
    let uses = sorted_edge_uses(&triangles);
    let component_count = count_roots(&labels_from_uses(triangles.len(), &uses));
    let (manifold_component_count, non_orientable_components) =
        manifold_components(&triangles, &uses);

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
        zero_area_triangles,
        sliver_triangles,
        manifold_component_count,
        non_orientable_components,
    }
}

fn triangle_area(positions: &[f64], t: &[usize; 3]) -> f64 {
    let p = |i: usize| [positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]];
    let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    0.5 * (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()
}

/// `(components joined across manifold edges, how many of them are
/// non-orientable)`. Uses a parity union-find: the parity of a triangle is
/// whether it must be flipped relative to its root to agree in orientation.
fn manifold_components(triangles: &[[usize; 3]], uses: &[EdgeUse]) -> (usize, usize) {
    let live: Vec<bool> = triangles
        .iter()
        .map(|t| !(t[0] == t[1] || t[1] == t[2] || t[0] == t[2]))
        .collect();
    let mut parent: Vec<usize> = (0..triangles.len()).collect();
    let mut parity = vec![false; triangles.len()];
    let mut conflict = vec![false; triangles.len()];
    let mut path = Vec::new();
    fn find(
        parent: &mut [usize],
        parity: &mut [bool],
        path: &mut Vec<usize>,
        x: usize,
    ) -> (usize, bool) {
        path.clear();
        let mut r = x;
        while parent[r] != r {
            path.push(r);
            r = parent[r];
        }
        // Compress from the node nearest the root outward.
        for &n in path.iter().rev() {
            let p = parent[n];
            if p != r {
                parity[n] ^= parity[p];
            }
            parent[n] = r;
        }
        (r, if x == r { false } else { parity[x] })
    }
    // Edges used exactly twice, in ascending key order; within a group the
    // uses are sorted by triangle, so `t1 < t2` is first-seen order.
    for group in uses.chunk_by(|x, y| x.key == y.key) {
        let [first, second] = group else {
            continue;
        };
        let (t1, d1, t2, d2) = (
            first.triangle(),
            first.forward(),
            second.triangle(),
            second.forward(),
        );
        if t1 == t2 {
            continue;
        }
        // Same traversal direction means one of the two must be flipped.
        let need = d1 == d2;
        let (r1, p1) = find(&mut parent, &mut parity, &mut path, t1);
        let (r2, p2) = find(&mut parent, &mut parity, &mut path, t2);
        if r1 == r2 {
            if (p1 ^ p2) != need {
                conflict[r1] = true;
            }
        } else {
            parent[r1] = r2;
            parity[r1] = p1 ^ p2 ^ need;
            conflict[r2] |= conflict[r1];
        }
    }
    let mut count = 0;
    let mut bad = 0;
    for ti in 0..triangles.len() {
        if live[ti] && parent[ti] == ti {
            count += 1;
            bad += conflict[ti] as usize;
        }
    }
    (count, bad)
}

/// Convenience predicate: strict closed 2-manifold.
pub fn is_manifold(positions: &[f64], indices: &[usize]) -> bool {
    check(positions, indices).is_manifold()
}

#[cfg(test)]
fn count_components(triangles: &[[usize; 3]]) -> usize {
    count_roots(&component_labels(triangles))
}

fn count_roots(labels: &[usize]) -> usize {
    // Each union-find component has exactly one representative, and that
    // triangle labels itself. Count those fixed points without another hash
    // table or an allocation proportional to the number of components.
    labels
        .iter()
        .enumerate()
        .filter(|(i, root)| *i == **root)
        .count()
}

/// One directed use of an undirected edge by a non-degenerate triangle.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EdgeUse {
    /// Canonical `(min, max)` endpoints.
    key: EdgeKey,
    /// `triangle << 1 | (traversed min -> max)`.
    tagged: usize,
}

impl EdgeUse {
    fn triangle(self) -> usize {
        self.tagged >> 1
    }
    fn forward(self) -> bool {
        self.tagged & 1 == 1
    }
}

/// All edge uses of the non-degenerate triangles, sorted by edge key and
/// then by triangle index. A flat sort is several times faster and smaller
/// than a hash map keyed by edge on million-triangle meshes.
pub(crate) fn sorted_edge_uses(triangles: &[[usize; 3]]) -> Vec<EdgeUse> {
    let mut uses = Vec::with_capacity(triangles.len() * 3);
    for (ti, t) in triangles.iter().enumerate() {
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
            continue;
        }
        for i in 0..3 {
            let (a, b) = (t[i], t[(i + 1) % 3]);
            uses.push(EdgeUse {
                key: edge_key(a, b),
                tagged: (ti << 1) | usize::from(a < b),
            });
        }
    }
    uses.sort_unstable();
    uses
}

/// Union-find label (component root) per triangle; empty mesh yields an
/// empty vector. Labels are not dense — compare, don't index by them.
pub(crate) fn component_labels(triangles: &[[usize; 3]]) -> Vec<usize> {
    labels_from_uses(triangles.len(), &sorted_edge_uses(triangles))
}

fn labels_from_uses(triangle_count: usize, uses: &[EdgeUse]) -> Vec<usize> {
    let mut parent: Vec<usize> = (0..triangle_count).collect();
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
    // Every further use of an edge joins its triangle to the previous user
    // (boundary edges have one use and therefore never join anything).
    for pair in uses.windows(2) {
        if pair[0].key == pair[1].key {
            let (ra, rb) = (
                find(&mut parent, pair[0].triangle()),
                find(&mut parent, pair[1].triangle()),
            );
            if ra != rb {
                parent[ra] = rb;
            }
        }
    }
    (0..triangle_count).map(|i| find(&mut parent, i)).collect()
}

#[cfg(test)]
mod component_count_regression;

#[cfg(test)]
mod tests;
