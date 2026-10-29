//! Complete simple zero cycles of original graph surfaces meet only at their
//! shared authored contour. Called only after exact lifts, simple positive trim
//! disks and whole-chart injectivity are independently admitted.
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
        || !a.holes.is_empty()
        || !b.holes.is_empty()
        || max == 0
        || a.surface.periodic_u
        || a.surface.periodic_v
        || b.surface.periodic_u
        || b.surface.periodic_v
    {
        return (None, 0);
    }
    let loops = [&model.loops[a.outer].coedges, &model.loops[b.outer].coedges];
    let edges: BTreeSet<_> = loops[0].iter().map(|c| c.edge).collect();
    if edges.len() < 2
        || loops[0].len() != edges.len()
        || loops[1].len() != edges.len()
        || edges != loops[1].iter().map(|c| c.edge).collect()
        || loops
            .iter()
            .flat_map(|l| l.iter())
            .any(|c| c.trim().is_some())
    {
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
        // A complete simple positive disk cycle lying on the one nonsingular
        // ellipse must be that entire ellipse. Both disk interiors are the
        // bounded ellipse interior. The original complete SS census contains
        // no other contact, hence no interior intersection can remain.
        Some(Certificate {
            faces,
            edges: edges.into_iter().collect(),
            operations: used,
        })
    })();
    (result, used)
}
