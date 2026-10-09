//! Best-effort mesh repair toward a manifold surface.
//!
//! Two modes:
//!
//! **Conservative** (default, [`repair`]) — only operations that cannot
//! silently destroy geometry intent:
//! 1. **Weld** — merge vertices closer than `epsilon` (exact weld with
//!    `epsilon = 0.0`).
//! 2. **Drop degenerates** — remove triangles that collapsed to a point or
//!    segment (after welding, or already degenerate in the input).
//! 3. **Unify orientation** — BFS over the face adjacency graph, flipping
//!    triangles so shared edges are traversed in opposite directions.
//!
//! **Full** ([`repair_with_mode`] with [`RepairMode::Full`]) — additionally:
//! 4. **Split non-manifold edges** — triangles around a 3+-use edge are
//!    grouped into manifold shells; extra shells get duplicated endpoints.
//! 5. **Split pinched vertices** — each disconnected fan component gets its
//!    own copy of the shared vertex.
//! 6. **Fill boundary loops** — simple boundary loops are triangulated
//!    (ear clipping on the best-fit plane projection, fan fallback) with
//!    orientation opposite to the surface boundary direction.
//!
//! What even Full does not do: remove self-intersections, or fill non-simple
//! (self-touching) boundary loops. Leftovers are reported in
//! [`RepairReport::residual`].

use crate::{EdgeKey, ManifoldReport, check, edge_key};

/// Result of a repair pass.
#[derive(Debug, Clone)]
pub struct RepairOutcome {
    /// Repaired mesh positions (welded; length shrinks when duplicates merge).
    pub positions: Vec<f64>,
    /// Repaired mesh indices (degenerates removed, orientation unified).
    pub indices: Vec<usize>,
    pub report: RepairReport,
}

/// Repair aggressiveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepairMode {
    /// Weld + de-degenerate + orientation unify only.
    #[default]
    Conservative,
    /// Additionally split non-manifold edges, split pinched vertices and
    /// fill simple boundary loops.
    Full,
}

/// What repair changed and what it could not fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairReport {
    /// Vertices merged by welding.
    pub welded_vertices: usize,
    /// Triangles removed as degenerate (input or post-weld).
    pub removed_degenerate_triangles: usize,
    /// Triangles flipped to unify orientation.
    pub flipped_triangles: usize,
    /// Vertices duplicated when splitting non-manifold edge shells and
    /// pinched fans (Full mode; 0 in Conservative).
    pub split_vertices: usize,
    /// Boundary loops triangulated (Full mode; 0 in Conservative).
    pub filled_holes: usize,
    /// Triangles added by hole filling (Full mode; 0 in Conservative).
    pub filled_triangles: usize,
    /// Manifoldness issues that repair does not attempt: residual report
    /// computed on the repaired mesh.
    pub residual: ManifoldReport,
}

impl RepairOutcome {
    /// Strict 2-manifold after repair.
    pub fn is_manifold(&self) -> bool {
        self.report.residual.is_manifold()
    }
}

/// Weld + de-degenerate + orientation-unify `positions`/`indices`
/// (Conservative mode; see [`repair_with_mode`]).
///
/// `epsilon` is the weld tolerance in mesh units; use `0.0` for an exact
/// bitwise weld. Negative epsilon is treated as `0.0`.
pub fn repair(positions: &[f64], indices: &[usize], epsilon: f64) -> RepairOutcome {
    repair_with_mode(positions, indices, epsilon, RepairMode::Conservative)
}

/// Repair `positions`/`indices` with the given aggressiveness.
///
/// `epsilon` is the weld tolerance in mesh units; use `0.0` for an exact
/// bitwise weld. Negative epsilon is treated as `0.0`.
pub fn repair_with_mode(
    positions: &[f64],
    indices: &[usize],
    epsilon: f64,
    mode: RepairMode,
) -> RepairOutcome {
    let epsilon = epsilon.max(0.0);

    // --- 1. Weld vertices -------------------------------------------------
    let (mut welded_positions, remap, welded_count) = weld(positions, epsilon);

    // --- 2. Remap indices, drop degenerates -------------------------------
    let mut triangles: Vec<[usize; 3]> = Vec::with_capacity(indices.len() / 3);
    let mut removed = 0usize;
    for t in indices.as_chunks::<3>().0 {
        let tri = [remap[t[0]], remap[t[1]], remap[t[2]]];
        if tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2] {
            removed += 1;
            continue;
        }
        triangles.push(tri);
    }

    // --- 3. Unify orientation ---------------------------------------------
    let flipped = unify_orientation(&mut triangles);

    // --- 4-6. Full mode: splits + hole filling ----------------------------
    let mut split_vertices = 0usize;
    let mut filled_holes = 0usize;
    let mut filled_triangles = 0usize;
    if mode == RepairMode::Full {
        split_vertices += split_non_manifold_edges(&mut welded_positions, &mut triangles);
        split_vertices += split_pinched_vertices(&mut welded_positions, &mut triangles);
        let (loops, added) = fill_boundary_loops(&welded_positions, &mut triangles);
        filled_holes = loops;
        filled_triangles = added;
    }

    let mut flat_indices = Vec::with_capacity(triangles.len() * 3);
    for t in &triangles {
        flat_indices.extend_from_slice(t);
    }
    let residual = check(&welded_positions, &flat_indices);

    RepairOutcome {
        positions: welded_positions,
        indices: flat_indices,
        report: RepairReport {
            welded_vertices: welded_count,
            removed_degenerate_triangles: removed,
            flipped_triangles: flipped,
            split_vertices,
            filled_holes,
            filled_triangles,
            residual,
        },
    }
}

/// Weld vertices closer than `epsilon`. Returns (positions, remap, merged).
fn weld(positions: &[f64], epsilon: f64) -> (Vec<f64>, Vec<usize>, usize) {
    use std::collections::HashMap;
    let vertex_count = positions.len() / 3;
    let mut remap: Vec<usize> = (0..vertex_count).collect();
    let mut out: Vec<f64> = Vec::with_capacity(positions.len());

    if epsilon == 0.0 {
        // Exact weld on bit patterns.
        let mut canonical: HashMap<[u64; 3], usize> = HashMap::new();
        for v in 0..vertex_count {
            let key = [
                positions[v * 3].to_bits(),
                positions[v * 3 + 1].to_bits(),
                positions[v * 3 + 2].to_bits(),
            ];
            let next = out.len() / 3;
            let id = *canonical.entry(key).or_insert_with(|| {
                out.extend_from_slice(&positions[v * 3..v * 3 + 3]);
                next
            });
            remap[v] = id;
        }
    } else {
        // Grid hash on cells of side epsilon: merge within same cell only
        // (conservative; points straddling a boundary stay separate unless
        // identical). Deterministic and O(n).
        let inv = 1.0 / epsilon;
        let mut canonical: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
        for v in 0..vertex_count {
            let p = &positions[v * 3..v * 3 + 3];
            let cell = (
                (p[0] * inv).floor() as i64,
                (p[1] * inv).floor() as i64,
                (p[2] * inv).floor() as i64,
            );
            let bucket = canonical.entry(cell).or_default();
            let mut found = None;
            for &u in bucket.iter() {
                let q = &out[u * 3..u * 3 + 3];
                let d2 = (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2);
                if d2 <= epsilon * epsilon {
                    found = Some(u);
                    break;
                }
            }
            match found {
                Some(u) => remap[v] = u,
                None => {
                    let id = out.len() / 3;
                    out.extend_from_slice(p);
                    bucket.push(id);
                    remap[v] = id;
                }
            }
        }
    }
    let merged = vertex_count - out.len() / 3;
    (out, remap, merged)
}

/// Flip triangles so every edge shared by exactly two triangles is
/// traversed in opposite directions. Returns the flip count.
///
/// BFS over the dual graph: an edge used twice imposes a constraint
/// between its two triangles; single-use (boundary) and 3+-use edges
/// impose none. Each connected component is oriented from an arbitrary
/// seed triangle; conflicting components cannot exist because constraints
/// are only checked, never forced.
fn unify_orientation(triangles: &mut [[usize; 3]]) -> usize {
    use std::collections::HashMap;
    // Edge -> incident (triangle, direction of traversal: true if a->b
    // with a < b... store actual (from, to) of the directed half-edge).
    let mut half_edges: HashMap<EdgeKey, Vec<(usize, usize, usize)>> = HashMap::new();
    for (ti, t) in triangles.iter().enumerate() {
        for i in 0..3 {
            let (a, b) = (t[i], t[(i + 1) % 3]);
            half_edges
                .entry(edge_key(a, b))
                .or_default()
                .push((ti, a, b));
        }
    }
    // Dual adjacency with the constraint: for edge used by triangles
    // (t1, a, b) and (t2, c, d), orientations agree iff the directed
    // half-edges are opposite, i.e. a == d && b == c.
    let mut dual: HashMap<usize, Vec<(usize, bool)>> = HashMap::new();
    for uses in half_edges.values() {
        if uses.len() != 2 {
            continue;
        }
        let (t1, a, b) = uses[0];
        let (t2, c, d) = uses[1];
        let agree = a == d && b == c;
        dual.entry(t1).or_default().push((t2, agree));
        dual.entry(t2).or_default().push((t1, agree));
    }

    // BFS assigning desired flip parity per component.
    let mut flip = vec![false; triangles.len()];
    let mut visited = vec![false; triangles.len()];
    let mut stack = Vec::new();
    let mut flipped = 0usize;
    for seed in 0..triangles.len() {
        if visited[seed] {
            continue;
        }
        visited[seed] = true;
        stack.push((seed, false));
        while let Some((t, parity)) = stack.pop() {
            if flip[t] != parity && visited[t] && t != seed {
                // Constraint conflict within a component (e.g. mobius-like
                // configuration). Keep first assignment; residual report
                // will flag the edge.
                continue;
            }
            flip[t] = parity;
            visited[t] = true;
            if let Some(neighbors) = dual.get(&t) {
                for &(n, agree) in neighbors {
                    // If we flip t, neighbor must flip too to keep the
                    // (dis)agreement.
                    let n_parity = if agree { parity } else { !parity };
                    if !visited[n] {
                        stack.push((n, n_parity));
                    }
                }
            }
        }
    }
    for (t, tri) in triangles.iter_mut().enumerate() {
        if flip[t] {
            tri.swap(1, 2);
            flipped += 1;
        }
    }
    flipped
}

// ---------------------------------------------------------------------------
// Full mode stages
// ---------------------------------------------------------------------------

/// Append a copy of vertex `v` to `positions`; returns the new vertex id.
fn duplicate_vertex(positions: &mut Vec<f64>, v: usize) -> usize {
    let point = [positions[v * 3], positions[v * 3 + 1], positions[v * 3 + 2]];
    positions.extend_from_slice(&point);
    positions.len() / 3 - 1
}

/// Group the triangles incident to a non-manifold edge (3+ uses) into
/// manifold shells: triangles connected through any *other* shared edge
/// belong to the same shell. Every shell beyond the first receives its own
/// copies of the edge endpoints. Returns the number of duplicated vertices.
fn split_non_manifold_edges(positions: &mut Vec<f64>, triangles: &mut [[usize; 3]]) -> usize {
    use std::collections::HashMap;
    // Edge -> incident triangle ids.
    let mut edge_tris: HashMap<EdgeKey, Vec<usize>> = HashMap::new();
    for (ti, t) in triangles.iter().enumerate() {
        for i in 0..3 {
            edge_tris
                .entry(edge_key(t[i], t[(i + 1) % 3]))
                .or_default()
                .push(ti);
        }
    }
    let mut duplicated = 0usize;
    for (&edge, tris) in &edge_tris {
        if tris.len() <= 2 {
            continue;
        }
        // Union-find over incident triangles, joined by shared edges other
        // than `edge` itself.
        let mut parent: HashMap<usize, usize> = tris.iter().map(|&t| (t, t)).collect();
        fn find(parent: &mut HashMap<usize, usize>, x: usize) -> usize {
            let mut r = x;
            while parent[&r] != r {
                r = parent[&r];
            }
            let mut c = x;
            while parent[&c] != r {
                let n = parent[&c];
                parent.insert(c, r);
                c = n;
            }
            r
        }
        let tri_set: std::collections::HashSet<usize> = tris.iter().copied().collect();
        let mut link_owner: HashMap<EdgeKey, usize> = HashMap::new();
        for &ti in &tri_set {
            for i in 0..3 {
                let key = edge_key(triangles[ti][i], triangles[ti][(i + 1) % 3]);
                if key == edge {
                    continue;
                }
                if let Some(&other) = link_owner.get(&key) {
                    if tri_set.contains(&other) {
                        let (ra, rb) = (find(&mut parent, other), find(&mut parent, ti));
                        if ra != rb {
                            parent.insert(ra, rb);
                        }
                    }
                } else {
                    link_owner.insert(key, ti);
                }
            }
        }
        // Group triangle ids by shell root, preserving first-shell-wins order.
        let mut shells: Vec<(usize, Vec<usize>)> = Vec::new();
        for &ti in tris {
            let root = find(&mut parent, ti);
            match shells.iter_mut().find(|(r, _)| *r == root) {
                Some((_, group)) => group.push(ti),
                None => shells.push((root, vec![ti])),
            }
        }
        // Shells beyond the first get private copies of both endpoints.
        for (_, group) in shells.iter().skip(1) {
            let (new_a, new_b) = (
                duplicate_vertex(positions, edge.0),
                duplicate_vertex(positions, edge.1),
            );
            duplicated += 2;
            for &ti in group {
                for corner in triangles[ti].iter_mut() {
                    if *corner == edge.0 {
                        *corner = new_a;
                    } else if *corner == edge.1 {
                        *corner = new_b;
                    }
                }
            }
        }
    }
    duplicated
}

/// Give every disconnected fan component its own copy of the shared vertex.
/// Returns the number of duplicated vertices.
fn split_pinched_vertices(positions: &mut Vec<f64>, triangles: &mut [[usize; 3]]) -> usize {
    use std::collections::HashMap;
    // Corners per vertex: (triangle, slot, next, prev) link edge endpoints.
    let vertex_count = positions.len() / 3;
    let mut corners: Vec<Vec<(usize, usize, usize, usize)>> = vec![Vec::new(); vertex_count];
    for (ti, t) in triangles.iter().enumerate() {
        for slot in 0..3 {
            corners[t[slot]].push((ti, slot, t[(slot + 1) % 3], t[(slot + 2) % 3]));
        }
    }
    let mut duplicated = 0usize;
    for (v, list) in corners.iter().enumerate() {
        if list.len() < 2 {
            continue;
        }
        // Union corners whose link edges share a node.
        let mut parent: Vec<usize> = (0..list.len()).collect();
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
        let mut node_owner: HashMap<usize, usize> = HashMap::new();
        for (ci, &(_, _, next, prev)) in list.iter().enumerate() {
            for node in [next, prev] {
                if let Some(&other) = node_owner.get(&node) {
                    let (ra, rb) = (find(&mut parent, other), find(&mut parent, ci));
                    if ra != rb {
                        parent[ra] = rb;
                    }
                } else {
                    node_owner.insert(node, ci);
                }
            }
        }
        // Components beyond the first get a private vertex copy.
        let mut seen_roots: HashMap<usize, usize> = HashMap::new();
        for (ci, &(ti, slot, _, _)) in list.iter().enumerate() {
            let root = find(&mut parent, ci);
            if seen_roots.is_empty() {
                seen_roots.insert(root, v);
            }
            let target = *seen_roots.entry(root).or_insert_with(|| {
                let copy = duplicate_vertex(positions, v);
                duplicated += 1;
                copy
            });
            if target != v {
                triangles[ti][slot] = target;
            }
        }
    }
    duplicated
}

/// Triangulate simple boundary loops and append the fill triangles.
/// Returns (filled loops, added triangles). Non-simple loops and ambiguous
/// (branching) boundaries are left for the residual report.
fn fill_boundary_loops(positions: &[f64], triangles: &mut Vec<[usize; 3]>) -> (usize, usize) {
    use std::collections::HashMap;
    let flat: Vec<usize> = triangles.iter().flatten().copied().collect();
    let uses = mesh_topology::EdgeUses::new(&flat);
    // Directed boundary edges as traversed by the surface.
    let mut next_edge: HashMap<usize, usize> = HashMap::new();
    let mut ambiguous: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for [a, b] in uses.boundary() {
        if next_edge.insert(a, b).is_some() {
            ambiguous.insert(a);
        }
    }
    if next_edge.is_empty() {
        return (0, 0);
    }
    // Walk all simple boundary loops first.
    let mut loops: Vec<Vec<usize>> = Vec::new();
    let mut visited: std::collections::HashSet<EdgeKey> = std::collections::HashSet::new();
    let starts: Vec<usize> = next_edge.keys().copied().collect();
    for start in starts {
        let mut loop_vs = vec![start];
        let mut cur = start;
        let mut simple = !ambiguous.contains(&start);
        loop {
            let Some(&nxt) = next_edge.get(&cur) else {
                simple = false;
                break;
            };
            if !visited.insert(edge_key(cur, nxt)) {
                simple = false;
                break;
            }
            if nxt == start {
                break;
            }
            if ambiguous.contains(&nxt) || loop_vs.contains(&nxt) {
                simple = false;
                cur = nxt;
                continue;
            }
            loop_vs.push(nxt);
            cur = nxt;
        }
        if simple && loop_vs.len() >= 3 {
            loops.push(loop_vs);
        }
    }
    if loops.is_empty() {
        return (0, 0);
    }
    // Hole loops wind opposite to the exterior contour on a consistently
    // oriented surface. Filling the exterior contour would double-cover the
    // sheet (and clash with interior diagonals), so with several loops only
    // the ones wound against the largest-area loop are filled. A single
    // loop is always filled: it is the hole by definition.
    let areas: Vec<f64> = loops
        .iter()
        .map(|l| loop_signed_area(positions, l))
        .collect();
    let exterior = areas
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map(|(i, _)| i)
        .unwrap();
    let mut filled_loops = 0usize;
    let mut added = 0usize;
    for (li, loop_vs) in loops.iter().enumerate() {
        if loops.len() > 1 && li != exterior && areas[li] * areas[exterior] > 0.0 {
            continue; // same winding as the exterior contour: another sheet's outside
        }
        if loops.len() > 1 && li == exterior {
            continue;
        }
        // Triangulate, then orient fill triangles so boundary edges are
        // traversed against the surface direction (ear clipping yields a
        // consistently wound strip, so one probe orients them all).
        let Some(tris) = triangulate_loop(positions, loop_vs) else {
            continue;
        };
        let boundary_dir: std::collections::HashSet<(usize, usize)> = (0..loop_vs.len())
            .map(|k| (loop_vs[k], loop_vs[(k + 1) % loop_vs.len()]))
            .collect();
        // Determine flip need with a single probe: if any fill triangle
        // traverses a boundary edge in surface direction, flip everything.
        let needs_flip = tris.iter().any(|t| {
            (0..3).any(|e| boundary_dir.contains(&(loop_vs[t[e]], loop_vs[t[(e + 1) % 3]])))
        });
        filled_loops += 1;
        added += tris.len();
        for mut t in tris {
            if needs_flip {
                t.swap(1, 2);
            }
            triangles.push([loop_vs[t[0]], loop_vs[t[1]], loop_vs[t[2]]]);
        }
    }
    (filled_loops, added)
}

/// Signed projected loop area along its dominant normal axis (half the
/// Newell normal component). Sign distinguishes hole loops from the
/// exterior contour on a consistently oriented surface.
fn loop_signed_area(positions: &[f64], loop_vs: &[usize]) -> f64 {
    let mut n = [0.0f64; 3];
    for i in 0..loop_vs.len() {
        let p = &positions[loop_vs[i] * 3..loop_vs[i] * 3 + 3];
        let q = &positions
            [loop_vs[(i + 1) % loop_vs.len()] * 3..loop_vs[(i + 1) % loop_vs.len()] * 3 + 3];
        n[0] += (p[1] - q[1]) * (p[2] + q[2]);
        n[1] += (p[2] - q[2]) * (p[0] + q[0]);
        n[2] += (p[0] - q[0]) * (p[1] + q[1]);
    }
    let axis = if n[0].abs() >= n[1].abs() && n[0].abs() >= n[2].abs() {
        0
    } else if n[1].abs() >= n[2].abs() {
        1
    } else {
        2
    };
    n[axis] / 2.0
}

/// Triangulate a simple 3D loop, returning triangle indices into `loop_vs`
/// with the same winding as the polygon orientation on its best-fit plane.
/// `None` when the loop is degenerate (zero-area or unclippable).
fn triangulate_loop(positions: &[f64], loop_vs: &[usize]) -> Option<Vec<[usize; 3]>> {
    // Newell normal of the loop.
    let mut n = [0.0f64; 3];
    for i in 0..loop_vs.len() {
        let p = &positions[loop_vs[i] * 3..loop_vs[i] * 3 + 3];
        let q = &positions
            [loop_vs[(i + 1) % loop_vs.len()] * 3..loop_vs[(i + 1) % loop_vs.len()] * 3 + 3];
        n[0] += (p[1] - q[1]) * (p[2] + q[2]);
        n[1] += (p[2] - q[2]) * (p[0] + q[0]);
        n[2] += (p[0] - q[0]) * (p[1] + q[1]);
    }
    let len2 = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
    if !len2.is_finite() || len2 <= 1e-24 {
        return None;
    }
    // Project onto the plane perpendicular to the dominant axis.
    let axis = if n[0].abs() >= n[1].abs() && n[0].abs() >= n[2].abs() {
        0
    } else if n[1].abs() >= n[2].abs() {
        1
    } else {
        2
    };
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    let pts: Vec<(f64, f64)> = loop_vs
        .iter()
        .map(|&i| (positions[i * 3 + u], positions[i * 3 + v]))
        .collect();
    let signed_area: f64 = (0..pts.len())
        .map(|i| {
            let (x0, y0) = pts[i];
            let (x1, y1) = pts[(i + 1) % pts.len()];
            x0 * y1 - x1 * y0
        })
        .sum::<f64>()
        / 2.0;
    if signed_area.abs() <= 1e-24 || !signed_area.is_finite() {
        return None;
    }
    // Ear clipping on a CCW-normalised index list.
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    if signed_area < 0.0 {
        idx.reverse();
    }
    let cross = |a: usize, b: usize, c: usize| -> f64 {
        (pts[b].0 - pts[a].0) * (pts[c].1 - pts[a].1)
            - (pts[b].1 - pts[a].1) * (pts[c].0 - pts[a].0)
    };
    let inside = |p: usize, a: usize, b: usize, c: usize| -> bool {
        cross(a, b, p) >= 0.0 && cross(b, c, p) >= 0.0 && cross(c, a, p) >= 0.0
    };
    let mut out = Vec::with_capacity(idx.len() - 2);
    let mut guard = idx.len() * idx.len();
    while idx.len() > 3 && guard > 0 {
        guard -= 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            if cross(a, b, c) <= 0.0 {
                continue; // reflex vertex
            }
            if idx
                .iter()
                .any(|&p| p != a && p != b && p != c && inside(p, a, b, c))
            {
                continue;
            }
            out.push([a, b, c]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            return None; // numerically unclippable; left for residual
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    // Triangles are in polygon (loop) winding modulo the reverse above; the
    // caller flips them against the surface boundary direction anyway.
    Some(out)
}

#[cfg(test)]
#[path = "tests/repair.rs"]
mod tests;
