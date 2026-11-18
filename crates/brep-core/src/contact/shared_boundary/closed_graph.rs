//! Complete simple zero cycles of original graph surfaces meet only at their
//! shared authored contour. Called only after exact lifts, simple positive trim
//! regions and whole-chart injectivity are independently admitted.
use crate::Model;
use nurbs_core::ss_intersection as ss;
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct Certificate {
    faces: [usize; 2],
    edges: Vec<usize>,
    operations: usize,
}
impl Certificate {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn edges(&self) -> &[usize] {
        &self.edges
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
}
pub(crate) fn certify(
    model: &Model,
    faces: [usize; 2],
    max: usize,
) -> (Option<Certificate>, usize) {
    let Some(a) = model.faces.get(faces[0]) else {
        return (None, 0);
    };
    let Some(b) = model.faces.get(faces[1]) else {
        return (None, 0);
    };
    if faces[0] == faces[1]
        || max == 0
        || a.surface.periodic_u
        || a.surface.periodic_v
        || b.surface.periodic_u
        || b.surface.periodic_v
    {
        return (None, 0);
    }
    let wires = |f: &crate::Face| {
        std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .collect::<Vec<_>>()
    };
    let wire_edges = |w: usize| {
        model.loops[w]
            .coedges
            .iter()
            .map(|c| c.edge)
            .collect::<BTreeSet<_>>()
    };
    let aw = wires(a);
    let bw = wires(b);
    let ae: BTreeSet<_> = aw.iter().flat_map(|&w| wire_edges(w)).collect();
    let be: BTreeSet<_> = bw.iter().flat_map(|&w| wire_edges(w)).collect();
    let edges: BTreeSet<_> = ae.intersection(&be).copied().collect();
    let complete = |ws: &[usize]| {
        ws.iter().any(|&w| {
            let coedges = &model.loops[w].coedges;
            coedges.len() == edges.len()
                && wire_edges(w) == edges
                && coedges.iter().all(|c| c.trim().is_none())
        })
    };
    if edges.len() < 2 || !complete(&aw) || !complete(&bw) {
        return (None, 0);
    }
    let mut used = 0;
    // Failed core queries reserve their complete remaining allowance. Each query
    // retains the existing 4096-operation core cap; no retry resets the budget.
    let result = (|| {
        let sources = ss::FiberSources::new(&a.surface, &b.surface).ok()?;
        let allowance = max.min(4096);
        let original = match ss::qualify_quadratic_graph_ss(
            &sources,
            ss::QuadraticGraphSSLimits {
                max_operations: allowance,
            },
        ) {
            Ok(p) => {
                used += p.operations();
                p
            }
            Err(_) => {
                used += allowance;
                return None;
            }
        };
        if original.kind() != ss::QuadraticGraphSSKind::ClosedEllipse
            || original.branches().len() != 1
            || !original.branches()[0].closed
        {
            return None;
        }
        // Joint exact lift identity already proves that every complete shared
        // world edge lies on BOTH original surfaces. Whole-chart injectivity
        // transfers the independently audited simple UV cycles to simple world
        // cycles. Repeating pcurve residual algebra here would re-prove the
        // same identity, not strengthen it. The complete original SS census
        // therefore places those actual cycles on its unique ellipse.
        // Any complete simple cycle lying on the one nonsingular ellipse is
        // that entire ellipse, including a negatively oriented hole cycle.
        // The complete original SS census contains no further contact. Both
        // regions therefore meet only on their actual common boundary, whether
        // the adjacent material occupies the inner or the outer component.
        Some(Certificate {
            faces,
            edges: edges.into_iter().collect(),
            operations: used,
        })
    })();
    (result, used)
}
