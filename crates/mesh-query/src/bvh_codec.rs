//! Versioned little-endian byte encoding of the packed picking BVH.
//!
//! Layout: the magic `OSVBVH`, a `u16` format version, then three `u64`
//! counts (nodes, node words, triangles) followed by the bounds as `f32`,
//! the node words as `u32` and the triangle IDs as `u32`. The tree only
//! refers to triangles by ID, so it is valid only together with the vertex
//! and index buffers it was built from; storing those is the caller's job.
use crate::bvh::{LEAF_BIT, MeshBvh};

const MAGIC: &[u8; 6] = b"OSVBVH";
/// Version written by [`encode`]. [`decode`] refuses every other version.
pub const FORMAT_VERSION: u16 = 1;
const HEADER: usize = 6 + 2 + 3 * 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The bytes do not start with the BVH magic.
    NotABvh,
    UnsupportedVersion(u16),
    /// The length disagrees with the counts in the header.
    Truncated,
    /// A child index or leaf range points outside the buffers, or a child
    /// does not follow its parent.
    InvalidTree,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotABvh => f.write_str("not an encoded BVH"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported BVH format version {v}"),
            Self::Truncated => f.write_str("encoded BVH length disagrees with its header"),
            Self::InvalidTree => f.write_str("encoded BVH has an invalid node"),
        }
    }
}
impl std::error::Error for DecodeError {}

pub fn encode(bvh: &MeshBvh) -> Vec<u8> {
    let node_count = bvh
        .node_count
        .min(bvh.bounds.len() / 6)
        .min(bvh.nodes.len() / 2);
    let mut bytes = Vec::with_capacity(HEADER + node_count * (6 + 2) * 4 + bvh.triangles.len() * 4);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    for count in [node_count, node_count * 2, bvh.triangles.len()] {
        bytes.extend_from_slice(&(count as u64).to_le_bytes());
    }
    for value in &bvh.bounds[..node_count * 6] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for word in bvh.nodes[..node_count * 2].iter().chain(&bvh.triangles) {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes
}

/// Decode and structurally validate a tree. A decoded tree cannot make a
/// query loop or index outside its own buffers; whether its triangle IDs fit
/// a given index buffer is checked by the queries themselves.
pub fn decode(bytes: &[u8]) -> Result<MeshBvh, DecodeError> {
    if bytes.len() < 8 || &bytes[..6] != MAGIC {
        return Err(DecodeError::NotABvh);
    }
    let version = u16::from_le_bytes([bytes[6], bytes[7]]);
    if version != FORMAT_VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    if bytes.len() < HEADER {
        return Err(DecodeError::Truncated);
    }
    let count = |at: usize| {
        let raw = u64::from_le_bytes(bytes[at..at + 8].try_into().expect("eight bytes"));
        usize::try_from(raw).map_err(|_| DecodeError::Truncated)
    };
    let (node_count, node_words, triangle_count) = (count(8)?, count(16)?, count(24)?);
    if node_words != node_count.checked_mul(2).ok_or(DecodeError::Truncated)? {
        return Err(DecodeError::Truncated);
    }
    let words = node_count
        .checked_mul(8)
        .and_then(|n| n.checked_add(triangle_count))
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| n.checked_add(HEADER))
        .ok_or(DecodeError::Truncated)?;
    if bytes.len() != words {
        return Err(DecodeError::Truncated);
    }
    let mut chunks = bytes[HEADER..].as_chunks::<4>().0.iter();
    let bounds: Vec<f32> = chunks
        .by_ref()
        .take(node_count * 6)
        .map(|c| f32::from_le_bytes(*c))
        .collect();
    let nodes: Vec<u32> = chunks
        .by_ref()
        .take(node_words)
        .map(|c| u32::from_le_bytes(*c))
        .collect();
    let triangles: Vec<u32> = chunks.map(|c| u32::from_le_bytes(*c)).collect();
    for node in 0..node_count {
        let first = nodes[node * 2] as usize;
        let metadata = nodes[node * 2 + 1];
        let valid = if metadata & LEAF_BIT != 0 {
            first
                .checked_add((metadata & !LEAF_BIT) as usize)
                .is_some_and(|end| end <= triangle_count)
        } else {
            [first, metadata as usize]
                .iter()
                .all(|&child| child > node && child < node_count)
        };
        if !valid {
            return Err(DecodeError::InvalidTree);
        }
    }
    Ok(MeshBvh {
        node_count,
        bounds,
        nodes,
        triangles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bvh::build_mesh_bvh;

    fn sample() -> (Vec<f32>, Vec<u32>, MeshBvh) {
        let mut vertices = Vec::new();
        for i in 0..40 {
            let x = i as f32 * 0.75;
            vertices.extend([
                x,
                0.,
                (i % 5) as f32,
                x + 1.,
                0.5,
                0.,
                x,
                1.,
                (i % 3) as f32,
            ]);
        }
        let indices: Vec<u32> = (0..120).collect();
        let tree = build_mesh_bvh(&vertices, &indices, 3, 2);
        (vertices, indices, tree)
    }

    #[test]
    fn round_trip_is_bit_exact_and_answers_the_same_queries() {
        let (vertices, indices, tree) = sample();
        let bytes = encode(&tree);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.node_count, tree.node_count);
        assert_eq!(decoded.nodes, tree.nodes);
        assert_eq!(decoded.triangles, tree.triangles);
        let bits = |b: &[f32]| b.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&decoded.bounds), bits(&tree.bounds));
        assert_eq!(encode(&decoded), bytes);
        let p = [7.3, 0.4, 1.1];
        assert_eq!(
            crate::nearest(&decoded, &vertices, &indices, 3, p, 4).unwrap(),
            crate::nearest(&tree, &vertices, &indices, 3, p, 4).unwrap()
        );
        let empty = build_mesh_bvh(&[], &[], 3, 2);
        assert_eq!(decode(&encode(&empty)).unwrap().node_count, 0);
    }

    #[test]
    fn damaged_input_is_refused_with_a_reason() {
        let (_, _, tree) = sample();
        let bytes = encode(&tree);
        assert_eq!(decode(b"nope").unwrap_err(), DecodeError::NotABvh);
        assert_eq!(decode(&bytes[..7]).unwrap_err(), DecodeError::NotABvh);
        let mut other = bytes.clone();
        other[6] = 9;
        assert_eq!(
            decode(&other).unwrap_err(),
            DecodeError::UnsupportedVersion(9)
        );
        assert_eq!(decode(&bytes[..20]).unwrap_err(), DecodeError::Truncated);
        assert_eq!(
            decode(&bytes[..bytes.len() - 4]).unwrap_err(),
            DecodeError::Truncated
        );
        let mut long = bytes.clone();
        long.extend([0; 4]);
        assert_eq!(decode(&long).unwrap_err(), DecodeError::Truncated);
        // A header claiming an absurd size must not allocate or overflow.
        let mut huge = bytes.clone();
        huge[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(decode(&huge).unwrap_err(), DecodeError::Truncated);
        // Point the root's left child back at the root.
        let mut cyclic = bytes.clone();
        let root = HEADER + tree.node_count * 6 * 4;
        cyclic[root..root + 4].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(decode(&cyclic).unwrap_err(), DecodeError::InvalidTree);
        // Every truncation is refused rather than panicking.
        for len in 0..bytes.len() {
            assert!(decode(&bytes[..len]).is_err());
        }
    }
}
