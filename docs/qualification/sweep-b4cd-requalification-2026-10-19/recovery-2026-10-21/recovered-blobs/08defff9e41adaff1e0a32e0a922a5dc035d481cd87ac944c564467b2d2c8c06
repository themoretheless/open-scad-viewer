//! Single-call adapters for renderer transparency preprocessing.
use super::{Result, Value, encode, field};
use geometry_ops::transparency::{Bsp, Fragment, Limits, Triangle};
pub fn build(v: Value) -> Result<Value> {
    let triangles: Vec<Triangle> = field(&v, "triangles")?;
    let fragments = triangles
        .into_iter()
        .enumerate()
        .map(|(owner, triangle)| Fragment { owner, triangle })
        .collect();
    let tree = Bsp::build(
        fragments,
        Limits {
            fragments: field(&v, "limit")?,
            operations: field(&v, "operationLimit")?,
            tolerance: field(&v, "tolerance")?,
        },
    )?;
    let nodes=tree.nodes.into_iter().map(|n|value_codec::json!({"plane":n.plane,"front":n.front,"back":n.back,"fragments":n.fragments.into_iter().map(|f|value_codec::json!({"owner":f.owner,"triangle":f.triangle})).collect::<Vec<_>>()})).collect::<Vec<_>>();
    encode(
        value_codec::json!({"root":tree.root,"nodes":nodes,"width":tree.width,"count":tree.fragment_count,"operations":tree.operation_count}),
    )
}
pub fn split(v: Value) -> Result<Value> {
    let parts = geometry_ops::transparency::split(
        &field(&v, "triangle")?,
        field(&v, "plane")?,
        field(&v, "tolerance")?,
    )?;
    encode(value_codec::json!({"front":parts.front,"back":parts.back,"coplanar":parts.coplanar}))
}

/// Detached numeric buffers for the renderer ABI; no per-vertex Value encoding.
pub struct Buffers {
    pub planes: Vec<f64>,
    pub links: Vec<u32>,
    pub owners: Vec<u32>,
    pub vertices: Vec<f64>,
    /// root+1 (0 means empty), width, fragment count, operation count.
    pub summary: [usize; 4],
}
pub fn buffers(width: usize, vertices: Vec<f64>, limits: Limits) -> Result<Buffers> {
    if width > 32 * 1024 * 1024 / 24
        || (width < 3 && !vertices.is_empty())
        || !vertices.len().is_multiple_of(
            width
                .max(3)
                .checked_mul(3)
                .ok_or_else(|| super::input("Invalid transparency vertex width"))?,
        )
    {
        return Err(super::input("Invalid transparency BSP vertex"));
    }
    let fragments = vertices
        .chunks_exact(width.max(3) * 3)
        .enumerate()
        .map(|(owner, row)| Fragment {
            owner,
            triangle: std::array::from_fn(|i| row[i * width..(i + 1) * width].to_vec()),
        })
        .collect();
    let per_fragment = width * 24 + 52;
    let transport_fragments = (32 * 1024 * 1024 - 256 * 48) / per_fragment;
    let bounded = Limits {
        fragments: limits.fragments.min(transport_fragments),
        ..limits
    };
    let tree = Bsp::build(fragments, bounded).map_err(|e| {
        if transport_fragments < limits.fragments
            && (e.message.contains("fragment limit") || e.message.contains("input limit"))
        {
            super::input("Transparency output exceeds transport limit")
        } else {
            e
        }
    })?;
    // Match the ordinary bridge's total response byte budget before allocation.
    let bytes = tree
        .fragment_count
        .checked_mul(width * 3 * 8 + 4)
        .and_then(|v| {
            tree.nodes
                .len()
                .checked_mul(48)
                .and_then(|n| v.checked_add(n))
        });
    if bytes.is_none_or(|n| n > 32 * 1024 * 1024) {
        return Err(super::input("Transparency output exceeds transport limit"));
    }
    let mut result = Buffers {
        planes: Vec::with_capacity(tree.nodes.len() * 4),
        links: Vec::with_capacity(tree.nodes.len() * 4),
        owners: Vec::with_capacity(tree.fragment_count),
        vertices: Vec::with_capacity(tree.fragment_count * width * 3),
        summary: [
            tree.root.map_or(0, |i| i + 1),
            width,
            tree.fragment_count,
            tree.operation_count,
        ],
    };
    for node in tree.nodes {
        result.planes.extend(node.plane);
        result.links.extend([
            node.front.map_or(0, |i| i as u32 + 1),
            node.back.map_or(0, |i| i as u32 + 1),
            result.owners.len() as u32,
            node.fragments.len() as u32,
        ]);
        for f in node.fragments {
            result.owners.push(f.owner as u32);
            for v in f.triangle {
                result.vertices.extend(v)
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_buffers_preserve_empty_tree_and_detached_attributes() {
        let empty = buffers(0, Vec::new(), Limits::default()).unwrap();
        assert_eq!(empty.summary, [0, 0, 0, 0]);
        let t = vec![-1., -1., 0., 0., 1., -1., 0., 1., 0., 1., 0., 0.5];
        let result = buffers(4, t.clone(), Limits::default()).unwrap();
        assert_eq!(result.vertices, t);
        assert_eq!(result.links, [0, 0, 0, 1]);
        assert_eq!(result.summary, [1, 4, 1, 1]);
        assert_eq!(result.owners, [0]);
        assert!(buffers(0, vec![1.], Limits::default()).is_err());
        assert!(buffers(usize::MAX, Vec::new(), Limits::default()).is_err());
    }
}
