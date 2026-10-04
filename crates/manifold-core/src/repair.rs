//! Best-effort mesh repair toward a manifold surface.
//!
//! Repair is deliberately conservative: it only performs operations that
//! cannot silently destroy geometry intent.
//!
//! 1. **Weld** — merge vertices closer than `epsilon` (exact weld with
//!    `epsilon = 0.0`).
//! 2. **Drop degenerates** — remove triangles that collapsed to a point or
//!    segment (after welding, or already degenerate in the input).
//! 3. **Unify orientation** — BFS over the face adjacency graph, flipping
//!    triangles so shared edges are traversed in opposite directions.
//!
//! What repair does *not* do: fill boundary loops, split non-manifold
//! edges (3+ faces), or separate pinched vertices. Those need semantic
//! decisions; they are reported in [`RepairReport::residual`].

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

/// What repair changed and what it could not fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairReport {
    /// Vertices merged by welding.
    pub welded_vertices: usize,
    /// Triangles removed as degenerate (input or post-weld).
    pub removed_degenerate_triangles: usize,
    /// Triangles flipped to unify orientation.
    pub flipped_triangles: usize,
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

/// Weld + de-degenerate + orientation-unify `positions`/`indices`.
///
/// `epsilon` is the weld tolerance in mesh units; use `0.0` for an exact
/// bitwise weld. Negative epsilon is treated as `0.0`.
pub fn repair(positions: &[f64], indices: &[usize], epsilon: f64) -> RepairOutcome {
    let epsilon = epsilon.max(0.0);

    // --- 1. Weld vertices -------------------------------------------------
    // Sort vertices by quantized coordinates; equal keys merge. Quantizing
    // by epsilon/2 keeps same-cell points within epsilon of each other and
    // avoids transitivity chains of a plain sort-by-x sweep.
    let (welded_positions, remap, welded_count) = weld(positions, epsilon);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::is_manifold;

    /// Cube with each face's vertices duplicated (unwelded soup): 24
    /// positions, same 12 triangles as the welded cube.
    fn unwelded_cube() -> (Vec<f64>, Vec<usize>) {
        // Face-local vertices; every corner appears 3 times.
        let faces: [[(f64, f64, f64); 4]; 6] = [
            // bottom (z=0, outward -z): CCW from below
            [
                (0.0, 0.0, 0.0),
                (0.0, 1.0, 0.0),
                (1.0, 1.0, 0.0),
                (1.0, 0.0, 0.0),
            ],
            // top (z=1, outward +z)
            [
                (0.0, 0.0, 1.0),
                (1.0, 0.0, 1.0),
                (1.0, 1.0, 1.0),
                (0.0, 1.0, 1.0),
            ],
            // front (y=0, outward -y)
            [
                (0.0, 0.0, 0.0),
                (1.0, 0.0, 0.0),
                (1.0, 0.0, 1.0),
                (0.0, 0.0, 1.0),
            ],
            // back (y=1, outward +y)
            [
                (1.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 1.0),
                (1.0, 1.0, 1.0),
            ],
            // right (x=1, outward +x)
            [
                (1.0, 0.0, 0.0),
                (1.0, 1.0, 0.0),
                (1.0, 1.0, 1.0),
                (1.0, 0.0, 1.0),
            ],
            // left (x=0, outward -x)
            [
                (0.0, 1.0, 0.0),
                (0.0, 0.0, 0.0),
                (0.0, 0.0, 1.0),
                (0.0, 1.0, 1.0),
            ],
        ];
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        for face in faces {
            let base = positions.len() / 3;
            for (x, y, z) in face {
                positions.extend_from_slice(&[x, y, z]);
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        (positions, indices)
    }

    #[test]
    fn weld_makes_unwelded_cube_manifold() {
        let (p, i) = unwelded_cube();
        assert!(!is_manifold(&p, &i));
        let out = repair(&p, &i, 0.0);
        assert_eq!(out.report.welded_vertices, 16); // 24 -> 8
        assert!(out.is_manifold(), "{:?}", out.report.residual);
    }

    #[test]
    fn epsilon_weld_merges_nearby_vertices() {
        let (mut p, i) = unwelded_cube();
        // Perturb duplicates slightly.
        for k in (0..p.len()).step_by(3) {
            p[k] += (k as f64 % 7.0) * 1e-7;
        }
        let out = repair(&p, &i, 1e-5);
        assert_eq!(out.positions.len() / 3, 8);
        assert!(out.is_manifold(), "{:?}", out.report.residual);
    }

    #[test]
    fn flipped_triangle_is_fixed() {
        let (p, mut i) = unwelded_cube();
        // Flip one triangle of the top face (indices 6..12 region: second
        // triangle of top face is at positions 9..12 of indices).
        i.swap(10, 11);
        let before = crate::check(&p, &i);
        assert!(!before.orientation_edges.is_empty() || !before.is_manifold());
        let out = repair(&p, &i, 0.0);
        assert!(out.is_manifold(), "{:?}", out.report.residual);
        assert!(out.report.flipped_triangles >= 1);
    }

    #[test]
    fn degenerate_triangles_are_removed() {
        let p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let i = vec![0, 1, 2, 0, 1, 1];
        let out = repair(&p, &i, 0.0);
        assert_eq!(out.report.removed_degenerate_triangles, 1);
        assert_eq!(out.indices.len(), 3);
    }

    #[test]
    fn triple_edge_is_reported_not_fixed() {
        // Repair must not silently drop faces of a 3-use edge.
        let p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0,
        ];
        let i = vec![0, 1, 2, 0, 4, 1, 0, 1, 3];
        let out = repair(&p, &i, 0.0);
        assert!(!out.is_manifold());
        assert!(out.report.residual.non_manifold_edges.contains(&(0, 1)));
        assert_eq!(out.indices.len(), 9); // nothing dropped
    }
}
