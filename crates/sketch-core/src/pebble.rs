//! (2,3)-pebble game (Jacobs and Hendrickson) for generic 2D bar-joint rigidity.
//!
//! Every vertex starts with two pebbles (its two translational degrees of
//! freedom). An edge is independent when four pebbles can be gathered on its
//! endpoints; one of them is then spent to cover the edge, oriented away from
//! the vertex that paid. A graph is generically rigid when exactly three free
//! pebbles (the rigid motions of the plane) remain. This is a combinatorial
//! count for generic positions: it does not see special geometry such as
//! collinear triangles, which the numeric rank in [`crate::plan_dr`] handles.

use crate::{Result, error};

/// Result of playing the (2,3)-pebble game on a graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PebbleAnalysis {
    /// Indices into the input edge list, in insertion order.
    pub independent_edges: Vec<usize>,
    /// Edges whose endpoints were already rigidly connected (over-constraint).
    pub redundant_edges: Vec<usize>,
    /// Maximal rigid components as sorted vertex sets of at least two vertices.
    /// Every edge lies in exactly one; isolated vertices belong to none.
    pub components: Vec<Vec<usize>>,
    /// Remaining degrees of freedom, `2 * vertices - independent edges`,
    /// including the three rigid motions of a rigid graph.
    pub free_pebbles: usize,
    /// The whole graph is generically rigid (at most one vertex, or three free pebbles).
    pub rigid: bool,
}

/// Upper bounds keep the quadratic component pass bounded on hostile input.
pub const MAX_PEBBLE_VERTICES: usize = 4_096;
pub const MAX_PEBBLE_EDGES: usize = 16_384;

struct Game {
    pebbles: Vec<u8>,
    /// Heads of the independent edges each vertex covers.
    out: Vec<Vec<usize>>,
}

impl Game {
    /// Move one free pebble to `start` along a directed path that avoids
    /// `blocked`, reversing the path's edges. Returns whether one was found.
    fn fetch(&mut self, start: usize, blocked: &[usize]) -> bool {
        let n = self.pebbles.len();
        let mut parent = vec![usize::MAX; n];
        let mut seen = vec![false; n];
        seen[start] = true;
        for &b in blocked {
            seen[b] = true;
        }
        let mut stack = vec![start];
        while let Some(v) = stack.pop() {
            for k in 0..self.out[v].len() {
                let w = self.out[v][k];
                if seen[w] {
                    continue;
                }
                seen[w] = true;
                parent[w] = v;
                if self.pebbles[w] > 0 {
                    self.pebbles[w] -= 1;
                    let mut head = w;
                    while head != start {
                        let tail = parent[head];
                        let edges = &mut self.out[tail];
                        let at = edges.iter().position(|&x| x == head).expect("path edge");
                        edges.swap_remove(at);
                        self.out[head].push(tail);
                        head = tail;
                    }
                    self.pebbles[start] += 1;
                    return true;
                }
                stack.push(w);
            }
        }
        false
    }

    /// Gather as many pebbles as possible on `u` and `v` (at most two each).
    fn gather(&mut self, u: usize, v: usize) -> u8 {
        while self.pebbles[u] < 2 && self.fetch(u, &[v]) {}
        while self.pebbles[v] < 2 && self.fetch(v, &[u]) {}
        self.pebbles[u] + self.pebbles[v]
    }

    /// Whether a free pebble is reachable from `w` without touching `pinned`.
    fn can_reach_pebble(&self, w: usize, pinned: &[usize]) -> bool {
        let n = self.pebbles.len();
        let mut seen = vec![false; n];
        for &p in pinned {
            seen[p] = true;
        }
        seen[w] = true;
        let mut stack = vec![w];
        while let Some(v) = stack.pop() {
            if self.pebbles[v] > 0 {
                return true;
            }
            for &x in &self.out[v] {
                if !seen[x] {
                    seen[x] = true;
                    stack.push(x);
                }
            }
        }
        false
    }
}

/// Play the (2,3)-pebble game on an undirected multigraph. Self-loops are
/// rejected; parallel edges are allowed and all but one become redundant.
pub fn pebble_game_2d(vertex_count: usize, edges: &[(usize, usize)]) -> Result<PebbleAnalysis> {
    if vertex_count > MAX_PEBBLE_VERTICES || edges.len() > MAX_PEBBLE_EDGES {
        return Err(error("Pebble game graph exceeds its size limit"));
    }
    if edges
        .iter()
        .any(|&(a, b)| a >= vertex_count || b >= vertex_count || a == b)
    {
        return Err(error("Pebble game edges must join two distinct vertices"));
    }
    let mut game = Game {
        pebbles: vec![2; vertex_count],
        out: vec![Vec::new(); vertex_count],
    };
    let mut independent_edges = Vec::new();
    let mut redundant_edges = Vec::new();
    for (index, &(u, v)) in edges.iter().enumerate() {
        if game.gather(u, v) == 4 {
            game.pebbles[u] -= 1;
            game.out[u].push(v);
            independent_edges.push(index);
        } else {
            redundant_edges.push(index);
        }
    }

    // Component pass: pin three pebbles on an unassigned edge; every vertex
    // that cannot then reach a free pebble is rigid with it.
    let mut assigned = vec![false; edges.len()];
    let mut components: Vec<Vec<usize>> = Vec::new();
    for &index in &independent_edges {
        if assigned[index] {
            continue;
        }
        let (u, v) = edges[index];
        let pinned = game.gather(u, v);
        debug_assert!(
            pinned >= 3,
            "an independent edge always frees three pebbles"
        );
        let mut member = vec![false; vertex_count];
        member[u] = true;
        member[v] = true;
        for w in 0..vertex_count {
            if w != u && w != v && !game.can_reach_pebble(w, &[u, v]) {
                member[w] = true;
            }
        }
        let component: Vec<usize> = (0..vertex_count).filter(|&w| member[w]).collect();
        for (k, &(a, b)) in edges.iter().enumerate() {
            if member[a] && member[b] {
                assigned[k] = true;
            }
        }
        components.push(component);
    }
    // Rigid components overlap in at most one vertex, so any redundant edge
    // already lies inside the component that made it redundant.
    debug_assert!(redundant_edges.iter().all(|&k| assigned[k]));
    components.sort();
    let free_pebbles = game.pebbles.iter().map(|&p| p as usize).sum();
    Ok(PebbleAnalysis {
        rigid: vertex_count <= 1 || free_pebbles == 3,
        independent_edges,
        redundant_edges,
        components,
        free_pebbles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: [(usize, usize); 4] = [(0, 1), (1, 2), (2, 3), (3, 0)];

    #[test]
    fn triangle_is_rigid() {
        let result = pebble_game_2d(3, &[(0, 1), (1, 2), (2, 0)]).unwrap();
        assert!(result.rigid);
        assert_eq!(result.free_pebbles, 3);
        assert!(result.redundant_edges.is_empty());
        assert_eq!(result.components, vec![vec![0, 1, 2]]);
    }

    #[test]
    fn square_is_flexible_with_one_component_per_bar() {
        let result = pebble_game_2d(4, &SQUARE).unwrap();
        assert!(!result.rigid);
        assert_eq!(result.free_pebbles, 4);
        assert!(result.redundant_edges.is_empty());
        assert_eq!(
            result.components,
            vec![vec![0, 1], vec![0, 3], vec![1, 2], vec![2, 3]]
        );
    }

    #[test]
    fn square_with_diagonal_is_rigid() {
        let mut edges = SQUARE.to_vec();
        edges.push((0, 2));
        let result = pebble_game_2d(4, &edges).unwrap();
        assert!(result.rigid);
        assert_eq!(result.free_pebbles, 3);
        assert!(result.redundant_edges.is_empty());
        assert_eq!(result.components, vec![vec![0, 1, 2, 3]]);
    }

    #[test]
    fn k4_is_over_constrained_by_one_edge() {
        let edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        let result = pebble_game_2d(4, &edges).unwrap();
        assert!(result.rigid);
        assert_eq!(result.free_pebbles, 3);
        assert_eq!(result.independent_edges.len(), 5);
        assert_eq!(result.redundant_edges, vec![5]);
        assert_eq!(result.components, vec![vec![0, 1, 2, 3]]);
    }

    #[test]
    fn hinged_triangles_are_two_components_sharing_a_vertex() {
        let edges = [(0, 1), (1, 2), (2, 0), (2, 3), (3, 4), (4, 2)];
        let result = pebble_game_2d(5, &edges).unwrap();
        assert!(!result.rigid);
        assert_eq!(result.free_pebbles, 4);
        assert_eq!(result.components, vec![vec![0, 1, 2], vec![2, 3, 4]]);
    }

    #[test]
    fn redundant_edge_in_one_part_does_not_rigidify_the_rest() {
        // K4 (one redundant bar) hinged to a dangling bar: counts alone would
        // see 2n - 3 = 7 edges, but the pebble game finds the extra freedom.
        let edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3), (3, 4)];
        let result = pebble_game_2d(5, &edges).unwrap();
        assert_eq!(edges.len(), 2 * 5 - 3);
        assert!(!result.rigid);
        assert_eq!(result.free_pebbles, 4);
        assert_eq!(result.redundant_edges, vec![5]);
        assert_eq!(result.components, vec![vec![0, 1, 2, 3], vec![3, 4]]);
    }

    #[test]
    fn parallel_bars_and_isolated_vertices() {
        let result = pebble_game_2d(3, &[(0, 1), (1, 0)]).unwrap();
        assert_eq!(result.redundant_edges, vec![1]);
        assert_eq!(result.components, vec![vec![0, 1]]);
        assert_eq!(result.free_pebbles, 5);
        assert!(pebble_game_2d(1, &[]).unwrap().rigid);
        assert!(pebble_game_2d(2, &[(0, 0)]).is_err());
        assert!(pebble_game_2d(2, &[(0, 2)]).is_err());
    }
}
