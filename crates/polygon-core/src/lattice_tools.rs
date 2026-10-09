//! Typed spatial graph generation, component counts and conservative decimation.
use crate::{Error, Mesh, Result};
use math_core::{cross, dot, sub};
use std::collections::{BTreeMap, BTreeSet};
fn input(message: impl Into<String>) -> Error {
    Error::new("GEOMETRY_INVALID_INPUT", message)
}
#[derive(Clone, Debug)]
pub struct SpatialGraph {
    pub nodes: Vec<[f64; 3]>,
    pub edges: Vec<[usize; 2]>,
}
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

pub fn component_count(mesh: &Mesh, positive_only: bool) -> Result<usize> {
    mesh.validate()?;
    let mut parent = (0..mesh.positions.len() / 3).collect::<Vec<_>>();
    for f in mesh.indices.as_chunks::<3>().0 {
        let a = root(&mut parent, f[0]);
        let b = root(&mut parent, f[1]);
        let c = root(&mut parent, f[2]);
        parent[b] = a;
        parent[c] = a;
    }

    if !positive_only {
        return Ok(mesh
            .indices
            .iter()
            .map(|&i| root(&mut parent, i))
            .collect::<BTreeSet<_>>()
            .len());
    }

    // Give each indexed component its own origin to avoid cancellation between
    // distant disconnected pieces. The threshold retains the editor contract.
    let mut sums = BTreeMap::<usize, (f64, f64)>::new();
    for f in mesh.indices.as_chunks::<3>().0 {
        let r = root(&mut parent, f[0]);
        let origin = &mesh.positions[r * 3..r * 3 + 3];
        let p: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|k| mesh.positions[f[i] * 3 + k] - origin[k])
        });
        let volume = (p[0][0] * (p[1][1] * p[2][2] - p[1][2] * p[2][1])
            + p[0][1] * (p[1][2] * p[2][0] - p[1][0] * p[2][2])
            + p[0][2] * (p[1][0] * p[2][1] - p[1][1] * p[2][0]))
            / 6.;
        if !volume.is_finite() {
            return Err(input("Component volume exceeds finite numeric range."));
        }
        let (sum, correction) = sums.entry(r).or_default();
        let y = volume - *correction;
        let next = *sum + y;
        *correction = (next - *sum) - y;
        *sum = next;
        if !sum.is_finite() {
            return Err(input("Component volume exceeds finite numeric range."));
        }
    }
    Ok(sums.values().filter(|&&(volume, _)| volume > 1e-8).count())
}

mod decimation;
pub use decimation::{decimate};


pub fn graph(
    mesh: Mesh,
    cell: f64,
    jitter: f64,
    seed: f64,
    pattern: &str,
    diagonals: bool,
) -> Result<SpatialGraph> {
    mesh.validate()?;
    if !cell.is_finite()
        || cell <= 0.
        || !jitter.is_finite()
        || !(0. ..=1.).contains(&jitter)
        || !seed.is_finite()
        || seed.fract() != 0.
    {
        return Err(input(
            "Spatial graph requires positive cell size, jitter between 0 and 1, and an integer seed.",
        ));
    }
    if !["spatial", "bone", "bcc", "octet"].contains(&pattern) {
        return Err(input("Invalid spatial lattice pattern."));
    }

    let (min, max) = crate::scene_flatten::bounds(&[mesh.positions])?;
    let mut cells = [0usize; 3];
    let mut counts = [0usize; 3];
    let mut total = 1usize;
    for k in 0..3 {
        let n = ((max[k] - min[k]) / cell).ceil().max(1.);
        if !n.is_finite() || n > 124. {
            return Err(input(
                "Spatial graph exceeds 125 nodes. Increase cell size.",
            ));
        }
        cells[k] = n as usize;
        counts[k] = cells[k] + 1;
        total = total
            .checked_mul(counts[k])
            .ok_or_else(|| input("Spatial graph exceeds 125 nodes."))?;
    }
    if total > 125 {
        return Err(input(
            "Spatial graph exceeds 125 nodes. Increase cell size.",
        ));
    }

    if matches!(pattern, "bcc" | "octet") {
        return centered_graph(min, max, cells, pattern);
    }

    let mut seed = seed.trunc().rem_euclid(4294967296.) as u32;
    let mut random = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed as f64 / 4294967296.
    };

    let mut nodes = Vec::<[f64; 3]>::with_capacity(total);
    let mut edges = Vec::<[usize; 2]>::new();
    let id = |x: usize, y: usize, z: usize| (z * counts[1] + y) * counts[0] + x;

    for z in 0..counts[2] {
        for y in 0..counts[1] {
            for x in 0..counts[0] {
                let xyz = [x, y, z];
                let point = std::array::from_fn(|k| {
                    let v = xyz[k];
                    min[k]
                        + (v as f64
                            + if v > 0 && v < cells[k] {
                                (random() - 0.5) * jitter
                            } else {
                                0.
                            })
                            * (max[k] - min[k])
                            / cells[k] as f64
                });
                if !point.iter().all(|x| x.is_finite()) {
                    return Err(input("Spatial graph exceeds finite numeric range."));
                }
                nodes.push(point);
            }
        }
    }

    for z in 0..counts[2] {
        for y in 0..counts[1] {
            for x in 0..counts[0] {
                let a = id(x, y, z);
                if x < cells[0] {
                    edges.push([a, id(x + 1, y, z)]);
                }
                if y < cells[1] {
                    edges.push([a, id(x, y + 1, z)]);
                }
                if z < cells[2] {
                    edges.push([a, id(x, y, z + 1)]);
                }
                if (diagonals || pattern == "bone") && x < cells[0] && y < cells[1] && z < cells[2]
                {
                    if random() < 0.5 {
                        edges.push([a, id(x + 1, y + 1, z + 1)]);
                    } else {
                        edges.push([id(x + 1, y, z), id(x, y + 1, z + 1)]);
                    }
                }
            }
        }
    }

    Ok(SpatialGraph { nodes, edges })
}

pub fn centered_graph(
    min: [f64; 3],
    max: [f64; 3],
    cells: [usize; 3],
    pattern: &str,
) -> Result<SpatialGraph> {
    if cells.iter().any(|&n| n == 0 || n > 124)
        || min.into_iter().chain(max).any(|x| !x.is_finite())
        || !matches!(pattern, "bcc" | "octet")
    {
        return Err(input(
            "Invalid centered spatial graph bounds, cells or pattern.",
        ));
    }
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
    Ok(SpatialGraph { nodes, edges })
}

mod lightening;
pub use lightening::{generate_lightening_cells, LighteningOptions, lighten};
