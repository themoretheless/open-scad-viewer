//! Bounded body- and face-centered graphs for CAD lightening.
use super::{Result, Value, encode, input};

pub(super) fn graph(
    min: [f64; 3],
    max: [f64; 3],
    cells: [usize; 3],
    pattern: &str,
) -> Result<Value> {
    let [nx, ny, nz] = cells;
    let [cx, cy, cz] = cells.map(|n| n + 1);
    let corners = cx * cy * cz;
    let volumes = nx * ny * nz;
    let faces = nx * ny * cz + nx * nz * cy + ny * nz * cx;
    let (node_count, edge_count) = if pattern == "bcc" {
        (corners + volumes, 8 * volumes)
    } else {
        (corners + faces, 4 * faces + 12 * volumes)
    };
    // The caller admits at most 125 corners; these products cannot overflow.
    // Count the complete topology before allocating or iterating over cells.
    if node_count > 125 || edge_count > 400 {
        return Err(input(
            "Spatial graph exceeds 125 nodes or 400 edges. Increase cell size.",
        ));
    }
    let at = |x: f64, y: f64, z: f64| {
        let xyz = [x, y, z];
        std::array::from_fn::<_, 3, _>(|k| min[k] + xyz[k] * (max[k] - min[k]) / cells[k] as f64)
    };
    let corner = |x, y, z| (z * cy + y) * cx + x;
    let mut nodes = Vec::with_capacity(node_count);
    let mut edges = Vec::with_capacity(edge_count);
    for z in 0..cz {
        for y in 0..cy {
            for x in 0..cx {
                nodes.push(at(x as f64, y as f64, z as f64));
            }
        }
    }
    let mut link = |a: usize, b: usize| edges.push([a.min(b), a.max(b)]);
    if pattern == "bcc" {
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let center = nodes.len();
                    nodes.push(at(x as f64 + 0.5, y as f64 + 0.5, z as f64 + 0.5));
                    for dz in 0..2 {
                        for dy in 0..2 {
                            for dx in 0..2 {
                                link(center, corner(x + dx, y + dy, z + dz));
                            }
                        }
                    }
                }
            }
        }
    } else {
        let xy_base = nodes.len();
        for z in 0..cz {
            for y in 0..ny {
                for x in 0..nx {
                    nodes.push(at(x as f64 + 0.5, y as f64 + 0.5, z as f64));
                }
            }
        }
        let xz_base = nodes.len();
        for y in 0..cy {
            for z in 0..nz {
                for x in 0..nx {
                    nodes.push(at(x as f64 + 0.5, y as f64, z as f64 + 0.5));
                }
            }
        }
        let yz_base = nodes.len();
        for x in 0..cx {
            for z in 0..nz {
                for y in 0..ny {
                    nodes.push(at(x as f64, y as f64 + 0.5, z as f64 + 0.5));
                }
            }
        }
        let xy = |x, y, z| xy_base + (z * ny + y) * nx + x;
        let xz = |x, y, z| xz_base + (y * nz + z) * nx + x;
        let yz = |x, y, z| yz_base + (x * nz + z) * ny + y;
        for z in 0..cz {
            for y in 0..ny {
                for x in 0..nx {
                    for dy in 0..2 {
                        for dx in 0..2 {
                            link(xy(x, y, z), corner(x + dx, y + dy, z));
                        }
                    }
                }
            }
        }
        for y in 0..cy {
            for z in 0..nz {
                for x in 0..nx {
                    for dz in 0..2 {
                        for dx in 0..2 {
                            link(xz(x, y, z), corner(x + dx, y, z + dz));
                        }
                    }
                }
            }
        }
        for x in 0..cx {
            for z in 0..nz {
                for y in 0..ny {
                    for dz in 0..2 {
                        for dy in 0..2 {
                            link(yz(x, y, z), corner(x, y + dy, z + dz));
                        }
                    }
                }
            }
        }
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let faces = [
                        xy(x, y, z),
                        xy(x, y, z + 1),
                        xz(x, y, z),
                        xz(x, y + 1, z),
                        yz(x, y, z),
                        yz(x + 1, y, z),
                    ];
                    for [a, b] in [
                        [0, 2],
                        [0, 3],
                        [0, 4],
                        [0, 5],
                        [1, 2],
                        [1, 3],
                        [1, 4],
                        [1, 5],
                        [2, 4],
                        [2, 5],
                        [3, 4],
                        [3, 5],
                    ] {
                        link(faces[a], faces[b]);
                    }
                }
            }
        }
    }
    if nodes.iter().flatten().any(|v| !v.is_finite()) {
        return Err(input("Spatial graph exceeds finite numeric range."));
    }
    debug_assert_eq!(nodes.len(), node_count);
    debug_assert_eq!(edges.len(), edge_count);
    encode(value_codec::json!({"nodes": nodes, "edges": edges}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn bounded_centered_topologies() {
        for (pattern, cells, nodes_expected, edges_expected) in [
            ("bcc", [1, 1, 1], 9, 8),
            ("bcc", [3, 3, 3], 91, 216),
            ("octet", [1, 1, 1], 14, 36),
            ("octet", [2, 2, 2], 63, 240),
        ] {
            let v = graph([0.; 3], [12.; 3], cells, pattern).unwrap();
            let nodes: Vec<[f64; 3]> = crate::field(&v, "nodes").unwrap();
            let edges: Vec<[usize; 2]> = crate::field(&v, "edges").unwrap();
            assert_eq!(nodes.len(), nodes_expected);
            assert_eq!(edges.len(), edges_expected);
            assert_eq!(edges.iter().collect::<BTreeSet<_>>().len(), edges.len());
            for [a, b] in edges {
                assert!(a < b && b < nodes.len());
                assert_ne!(nodes[a], nodes[b]);
            }
        }
        assert!(graph([0.; 3], [12.; 3], [4, 4, 4], "bcc").is_err());
        // 113 nodes fit, but 464 edges exceed the independent edge budget.
        assert!(graph([0.; 3], [12.; 3], [4, 2, 2], "octet").is_err());
    }
}
