//! Original natural-corner and tangent-control binding.
use crate::{source_boundary_fragment::Fragment, source_vertex_links::Node};
use nurbs_core::surface::Surface;

pub(super) struct Corner<'s> {
    pub face: usize,
    pub wire: usize,
    pub edges: [usize; 2],
    pub uv: [f64; 2],
    pub point: [f64; 3],
    pub rays: [(Node, [f64; 3]); 2],
    pub source: &'s Surface,
    pub rectangle: [[f64; 2]; 2],
}

fn natural_end(f: &Fragment, end: usize) -> Option<([f64; 2], [f64; 2])> {
    let c = f.curve();
    let d = c.domain();
    if c.periodic
        || c.degree != 1
        || c.control_points.len() != 2
        || c.knots.len() != 4
        || c.knots[..2].iter().any(|v| *v != d[0])
        || c.knots[2..].iter().any(|v| *v != d[1])
    {
        return None;
    }
    // Singleton ORIGINAL endpoint enclosures also admit a privately proved
    // algebraic endpoint exactly at a natural end. No rounded root is used.
    let bounds = f.parameter_bounds();
    let i = d.iter().position(|v| bounds[end] == [*v; 2])?;
    if bounds[1 - end] != [d[1 - i]; 2] {
        return None;
    }
    Some((
        c.control_points[i].as_slice().try_into().ok()?,
        c.control_points[1 - i].as_slice().try_into().ok()?,
    ))
}

/// Only immutable original controls become tangent-ray leaves. Positive
/// endpoint weights and clamped knot widths multiply these ray directions by
/// positive factors; no rounded derivative is promoted to authored geometry.
pub(super) fn prepare<'s>(
    face: usize,
    wire: usize,
    edges: [usize; 2],
    source: &'s Surface,
    current: &Fragment,
    previous: &Fragment,
    nodes: [Node; 2],
) -> Option<Corner<'s>> {
    if source != current.surface() || source != previous.surface() {
        return None;
    }
    let (uv, next) = natural_end(current, 0)?;
    let (other_uv, prev) = natural_end(previous, 1)?;
    if uv != other_uv {
        return None;
    }
    let counts = [source.control_points.len(), source.control_points[0].len()];
    let knots = [&source.knots_u, &source.knots_v];
    let degrees = [source.degree_u, source.degree_v];
    let mut indexes = [0; 2];
    let mut domains = [[0.; 2]; 2];
    let mut rectangle = domains;
    for axis in 0..2 {
        let (p, n, k) = (degrees[axis], counts[axis], knots[axis]);
        if p == 0
            || k[..=p].iter().any(|v| *v != k[p])
            || k[n..].iter().any(|v| *v != k[n])
            || k[p + 1] <= k[p]
            || k[n - 1] >= k[n]
        {
            return None;
        }
        let d = [k[p], k[n]];
        let end = d.iter().position(|v| *v == uv[axis])?;
        indexes[axis] = if end == 0 { 0 } else { n - 1 };
        domains[axis] = d;
        // A domain proposal, not replacement source data. Strictly positive
        // width gives a relative neighborhood of the exact natural corner.
        let cut = if end == 0 {
            d[0] * 0.75 + d[1] * 0.25
        } else {
            d[0] * 0.25 + d[1] * 0.75
        };
        if !(d[0] < cut && cut < d[1]) {
            return None;
        }
        rectangle[axis] = if end == 0 { [d[0], cut] } else { [cut, d[1]] };
    }
    let point = source.control_points[indexes[0]][indexes[1]]
        .as_slice()
        .try_into()
        .ok()?;
    let mut rays = [(nodes[0], [0.; 3]), (nodes[1], [0.; 3])];
    let mut axes = [0; 2];
    for (i, away) in [next, prev].into_iter().enumerate() {
        let mut changes = Vec::new();
        for axis in 0..2 {
            if away[axis] != uv[axis] {
                if !domains[axis].contains(&away[axis]) {
                    return None;
                }
                changes.push(axis);
            }
        }
        if changes.len() != 1 {
            return None;
        }
        let axis = changes[0];
        axes[i] = axis;
        let mut neighbor = indexes;
        neighbor[axis] = if indexes[axis] == 0 {
            1
        } else {
            counts[axis] - 2
        };
        rays[i].1 = source.control_points[neighbor[0]][neighbor[1]]
            .as_slice()
            .try_into()
            .ok()?;
    }
    if axes[0] == axes[1] {
        return None;
    }
    Some(Corner {
        face,
        wire,
        edges,
        uv,
        point,
        rays,
        source,
        rectangle,
    })
}
