//! Coherent display endpoints on exact source-shell vertex identities.
//! Anchors are display representatives, never exact root coordinates or mesh proof.
use crate::{
    source_boundary_network, source_edge_restriction::Restriction, source_shell_incidence::Shell,
};
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq)]
pub struct Edge {
    pub vertices: [usize; 2],
    pub segments: Vec<([f64; 2], [[f64; 3]; 2], [[f64; 2]; 3])>,
}
#[derive(Debug, PartialEq)]
pub struct Preview {
    pub network: source_boundary_network::Network,
    pub edges: Vec<Edge>,
}
/// Keep original parameter intervals and source enclosures. Replace only display
/// endpoints by the common vertex representative; widen display segment boxes
/// to include its complete uncertainty enclosure. No proximity joins are used.
pub fn prepare(
    shell: &Shell,
    segments: usize,
    max_spans: usize,
    max_endpoints: usize,
) -> Result<Preview> {
    if !(1..=4096).contains(&segments)
        || shell
            .edges()
            .len()
            .checked_mul(segments)
            .is_none_or(|n| n > 65536)
    {
        return Err(Error::new(
            "BREP_SOURCE_BOUNDARY_DISPLAY",
            "Bound source boundary display segments",
        ));
    }
    let network = source_boundary_network::inspect(shell, max_spans, max_endpoints)?;
    let anchors: BTreeMap<_, _> = network
        .vertices
        .iter()
        .map(|v| {
            // This form cannot overflow even for opposite extreme finite bounds.
            let point = v.bounds.map(anchor);
            (v.id, (point, v.bounds))
        })
        .collect();
    let mut edges = Vec::with_capacity(shell.edges().len());
    for (index, edge) in shell.edges().iter().enumerate() {
        let use_ = shell.uses()[index][0];
        let mut vertices = shell.vertices()[use_.face][use_.wire][use_.edge];
        if edge.reversed()[0] {
            vertices.reverse();
        }
        let mut display = Restriction::from_edge(edge).display_segments(segments)?;
        for end in 0..2 {
            let (point, bounds) = anchors[&vertices[end]];
            let segment = if end == 0 {
                &mut display[0]
            } else {
                &mut display[segments - 1]
            };
            segment.1[end] = point;
            for axis in 0..3 {
                segment.2[axis][0] = segment.2[axis][0].min(bounds[axis][0]);
                segment.2[axis][1] = segment.2[axis][1].max(bounds[axis][1]);
            }
        }
        edges.push(Edge {
            vertices,
            segments: display,
        });
    }
    Ok(Preview { network, edges })
}

fn anchor(b: [f64; 2]) -> f64 {
    (b[0] * 0.5 + b[1] * 0.5).clamp(b[0], b[1])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_anchor_stays_inside_extreme_and_subnormal_bounds() {
        let tiny = f64::from_bits(1);
        for bounds in [
            [tiny, tiny],
            [-tiny, -tiny],
            [-f64::MAX, f64::MAX],
            [f64::MAX, f64::MAX],
        ] {
            let p = anchor(bounds);
            assert!(p.is_finite() && p >= bounds[0] && p <= bounds[1]);
        }
    }
}
