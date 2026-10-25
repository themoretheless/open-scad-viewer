//! Preserve natural edge parameterization without mirroring stored knots.
//! A double floating reversal is not an exact identity for non-dyadic knots.
use super::*;
impl Builder {
    pub(super) fn coedge_parametric(
        &mut self,
        a: usize,
        b: usize,
        curve: Curve,
        pcurve: Curve,
        backward: bool,
    ) -> Coedge {
        let key = (a.min(b), a.max(b));
        let edge = *self.edges.entry(key).or_insert_with(|| {
            let id = self.model.edges.len();
            self.model.edges.push(Edge {
                degenerate: a == b,
                vertices: if backward { [b, a] } else { [a, b] },
                curve,
            });
            id
        });
        let reversed = self.model.edges[edge].vertices != [a, b];
        Coedge::new(
            edge,
            reversed,
            pcurve,
        )
    }
    /// The fourth input is the ORIGINAL increasing-V boundary, not a rounded
    /// reversed Curve. The coedge records its backwards traversal explicitly.
    pub(super) fn rectangular_patch_parametric(
        &mut self,
        surface: Surface,
        corners: [usize; 4],
        curves: [Curve; 4],
    ) {
        let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let uses = curves
            .into_iter()
            .enumerate()
            .map(|(k, curve)| {
                self.coedge_parametric(
                    corners[k],
                    corners[(k + 1) % 4],
                    curve,
                    line(uv[k].to_vec(), uv[(k + 1) % 4].to_vec()),
                    k == 3,
                )
            })
            .collect();
        let outer = self.wire(uses);
        self.face(surface, outer, vec![], false);
    }
}
