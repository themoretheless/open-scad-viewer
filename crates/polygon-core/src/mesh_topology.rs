//! Exact indexed edge uses for mesh inspection and boundary construction.
//! This is not position welding or tolerance-based display edge extraction.

pub(crate) struct EdgeUses {
    uses: Vec<u64>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct EdgeCounts {
    pub boundary: usize,
    pub non_manifold: usize,
    pub orientation: usize,
}

#[cfg(test)]
fn key([a, b]: [usize; 2]) -> [usize; 2] {
    [a.min(b), a.max(b)]
}

// Validated indices fit the high 31-bit and low 32-bit endpoint fields.
// The final bit retains direction without changing canonical group order.
const _: () = assert!(crate::MAX_VERTICES as u64 <= 1_u64 << 31);
fn pack([a, b]: [usize; 2]) -> u64 {
    ((a.min(b) as u64) << 33) | ((a.max(b) as u64) << 1) | u64::from(a > b)
}

fn unpack(edge: u64) -> [usize; 2] {
    let low = (edge >> 33) as usize;
    let high = ((edge >> 1) & u32::MAX as u64) as usize;
    if edge & 1 == 0 {
        [low, high]
    } else {
        [high, low]
    }
}

impl EdgeUses {
    /// Call only after Mesh::validate has admitted the input size and indices.
    pub(crate) fn new(indices: &[usize]) -> Self {
        let mut uses = Vec::with_capacity(indices.len());
        for &[a, b, c] in indices.as_chunks::<3>().0 {
            uses.extend([pack([a, b]), pack([b, c]), pack([c, a])]);
        }
        // Only canonical edge order is observable. Counts/orientation do not
        // depend on tie order, and every boundary group contains just one use.
        uses.sort_unstable();
        Self { uses }
    }

    fn groups(&self) -> impl Iterator<Item = &[u64]> {
        self.uses.chunk_by(|a, b| a >> 1 == b >> 1)
    }

    pub(crate) fn counts(&self) -> EdgeCounts {
        let mut counts = EdgeCounts::default();
        for uses in self.groups() {
            match uses.len() {
                1 => counts.boundary += 1,
                2 if uses[0] & 1 == uses[1] & 1 => counts.orientation += 1,
                3.. => counts.non_manifold += 1,
                _ => {}
            }
        }
        counts
    }

    /// Directed boundary edges in the same canonical order as the former map.
    pub(crate) fn boundary(&self) -> impl Iterator<Item = [usize; 2]> + '_ {
        self.groups()
            .filter(|uses| uses.len() == 1)
            .map(|uses| unpack(uses[0]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn check(indices: &[usize]) {
        let mut reference = BTreeMap::<_, Vec<_>>::new();
        for tri in indices.as_chunks::<3>().0 {
            for i in 0..3 {
                let edge = [tri[i], tri[(i + 1) % 3]];
                reference.entry(key(edge)).or_default().push(edge);
            }
        }
        let actual = EdgeUses::new(indices);
        assert_eq!(
            actual.counts(),
            EdgeCounts {
                boundary: reference.values().filter(|uses| uses.len() == 1).count(),
                non_manifold: reference.values().filter(|uses| uses.len() > 2).count(),
                orientation: reference
                    .values()
                    .filter(|uses| uses.len() == 2 && uses[0][0] == uses[1][0])
                    .count(),
            }
        );
        assert_eq!(
            actual.boundary().collect::<Vec<_>>(),
            reference
                .values()
                .filter(|uses| uses.len() == 1)
                .map(|uses| uses[0])
                .collect::<Vec<_>>()
        );
        let canonical_uses = |uses: &[[usize; 2]]| {
            let mut uses = uses.to_vec();
            uses.sort_unstable();
            uses
        };
        assert_eq!(
            actual
                .groups()
                .map(|uses| canonical_uses(&uses.iter().copied().map(unpack).collect::<Vec<_>>()))
                .collect::<Vec<_>>(),
            reference
                .values()
                .map(|uses| canonical_uses(uses))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn matches_map_for_boundaries_orientation_degeneracy_and_nonmanifold_edges() {
        for indices in [
            vec![],
            vec![0, 1, 2],
            vec![0, 1, 2, 0, 2, 3],
            vec![0, 1, 2, 0, 3, 2],
            vec![0, 1, 2, 1, 0, 3, 0, 1, 4],
            vec![0, 0, 1, 1, 1, 1],
            vec![0, 1, 2, 2, 1, 0],
        ] {
            check(&indices);
            check(&indices.into_iter().rev().collect::<Vec<_>>());
        }
    }

    #[test]
    fn packed_endpoints_cover_the_full_admitted_index_range() {
        let max = crate::MAX_VERTICES - 1;
        for edge in [[0, 0], [0, max], [max, 0], [max - 1, max], [max, max]] {
            assert_eq!(unpack(pack(edge)), edge);
        }
        check(&[0, max - 1, max, max, max - 1, 0, max, max, 0]);
    }

    #[test]
    fn matches_map_for_dense_meshes_and_adversarial_orderings() {
        let sphere = crate::solid::primitives::sphere(30., 128).unwrap();
        check(&sphere.indices);
        let mut indices = sphere.indices;
        indices.reverse();
        check(&indices);
        check(&[0, 1, 2].repeat(crate::MAX_MESH_TRIANGLES));
    }

    #[test]
    fn matches_map_for_deterministic_random_triangle_soups() {
        let mut seed = 0x64b478cd_u64;
        for vertices in [1, 2, 3, 17, 127, 1024] {
            for triangles in [1, 7, 64, 513, 4096] {
                let indices: Vec<_> = (0..triangles * 3)
                    .map(|_| {
                        seed = seed
                            .wrapping_mul(6364136223846793005)
                            .wrapping_add(1442695040888963407);
                        (seed >> 32) as usize % vertices
                    })
                    .collect();
                check(&indices);
            }
        }
    }
}
