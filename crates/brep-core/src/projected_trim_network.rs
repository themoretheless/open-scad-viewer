//! Exact affine projections plus simple-polygon owned boundary networks.
use super::{projected_trim_contact, same_support_contact as common};
use crate::Model;
use std::collections::BTreeSet;
#[derive(Clone, Debug)]
pub struct Certificate {
    pub faces: [usize; 2],
    pub axes: [usize; 2],
    pub determinant_signs: [i8; 2],
    pub edges: Vec<usize>,
    pub vertices: Vec<usize>,
    pub cells: usize,
}
pub struct Attempt {
    pub certificate: Option<Certificate>,
    pub cells: usize,
}
pub fn multiple_shared_edges(m: &Model, faces: [usize; 2]) -> bool {
    if faces
        .iter()
        .any(|&f| !m.faces[f].holes.is_empty() || m.loops[m.faces[f].outer].coedges.len() > 64)
    {
        return false;
    }
    let edges = |f: usize| {
        std::iter::once(m.faces[f].outer)
            .chain(m.faces[f].holes.iter().copied())
            .flat_map(|w| m.loops[w].coedges.iter().map(|c| c.edge))
            .collect::<BTreeSet<_>>()
    };
    edges(faces[0])
        .intersection(&edges(faces[1]))
        .take(2)
        .count()
        == 2
}
/// Only called under fresh joint exact-boundary, valid-trim and injectivity
/// prerequisites. Projection injectivity is also proved on each whole carrier.
pub fn certify(m: &Model, faces: [usize; 2], max_cells: usize) -> nurbs_core::Result<Attempt> {
    let mut out = Attempt {
        certificate: None,
        cells: 0,
    };
    if max_cells == 0 || max_cells > 100000 {
        return Ok(out);
    }
    let (Some(a), Some(b)) = (common::polygon(m, faces[0]), common::polygon(m, faces[1])) else {
        return Ok(out);
    };
    let mut work = common::Work {
        used: 0,
        limit: max_cells,
    };
    for axes in [[0, 1], [0, 2], [1, 2]] {
        let Some((pa, sa)) =
            projected_trim_contact::project(&m.faces[faces[0]].surface, &a, axes, &mut work)
        else {
            continue;
        };
        let Some((pb, sb)) =
            projected_trim_contact::project(&m.faces[faces[1]].surface, &b, axes, &mut work)
        else {
            continue;
        };
        if let Some((edges, vertices)) = common::contact_network(&pa, &pb, &mut work) {
            if edges.len() < 2 {
                continue;
            }
            out.certificate = Some(Certificate {
                faces,
                axes,
                determinant_signs: [sa, sb],
                edges,
                vertices,
                cells: work.used,
            });
            break;
        }
    }
    out.cells = work.used;
    Ok(out)
}
