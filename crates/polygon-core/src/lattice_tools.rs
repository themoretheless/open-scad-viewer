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

fn ordered_contains(values: &[usize], x: usize) -> bool {
    values.contains(&x)
}

fn ordered_push_unique(values: &mut Vec<usize>, x: usize) {
    if !ordered_contains(values, x) {
        values.push(x);
    }
}

fn ordered_remove(values: &mut Vec<usize>, x: usize) {
    if let Some(pos) = values.iter().position(|&v| v == x) {
        values.remove(pos);
    }
}

fn ordered_intersection_into(a: &[usize], b: &[usize], out: &mut Vec<usize>) {
    out.clear();
    for &x in a {
        if ordered_contains(b, x) {
            out.push(x);
        }
    }
}

fn neighbors_into(v: usize, faces: &[[usize; 3]], incident: &[Vec<usize>], out: &mut Vec<usize>) {
    out.clear();
    for &fi in &incident[v] {
        for &w in &faces[fi] {
            if w != v {
                ordered_push_unique(out, w);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct HeapEdge {
    a: usize,
    b: usize,
    cost: f64,
    va: usize,
    vb: usize,
    seq: u64,
}

struct EdgeHeap {
    data: Vec<HeapEdge>,
    seq: u64,
}

impl EdgeHeap {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            seq: 0,
        }
    }
    fn push(
        &mut self,
        a: usize,
        b: usize,
        points: &[[f64; 3]],
        versions: &[usize],
        active: &[bool],
    ) {
        if a == b || !active[a] || !active[b] {
            return;
        }
        let dx = points[a][0] - points[b][0];
        let dy = points[a][1] - points[b][1];
        let dz = points[a][2] - points[b][2];
        let cost = (dx * dx + dy * dy + dz * dz).sqrt();
        let item = HeapEdge {
            a,
            b,
            cost,
            va: versions[a],
            vb: versions[b],
            seq: self.seq,
        };
        self.seq += 1;

        let mut i = self.data.len();
        self.data.push(item);
        while i > 0 {
            let parent = (i - 1) >> 1;
            let p = self.data[parent];
            if p.cost < item.cost || (p.cost == item.cost && p.seq <= item.seq) {
                break;
            }
            self.data[i] = p;
            i = parent;
        }
        self.data[i] = item;
    }
    fn pop(&mut self) -> Option<HeapEdge> {
        if self.data.is_empty() {
            return None;
        }
        let first = self.data[0];
        let last = self.data.pop().unwrap();
        if !self.data.is_empty() {
            let mut i = 0;
            while i * 2 + 1 < self.data.len() {
                let left = i * 2 + 1;
                let right = left + 1;
                let mut child = left;
                if right < self.data.len() && self.data[right].cost < self.data[left].cost {
                    child = right;
                }
                if self.data[child].cost >= last.cost {
                    break;
                }
                self.data[i] = self.data[child];
                i = child;
            }
            self.data[i] = last;
        }
        Some(first)
    }
}

pub fn decimate(mesh: Mesh, tolerance: f64, target: f64) -> Result<Mesh> {
    if !tolerance.is_finite() || tolerance < 0. {
        return Err(input(
            "Lattice decimation tolerance must be a non-negative finite number.",
        ));
    }
    let target: usize = {
        let t = target;
        if !t.is_finite() {
            return Err(input("Lattice decimation target must be finite."));
        }
        if t < 0. {
            return Err(input("Lattice decimation target must be non-negative."));
        }
        t.floor() as usize
    };
    mesh.validate()?;

    let mut points = (0..mesh.positions.len() / 3)
        .map(|i| std::array::from_fn(|k| mesh.positions[i * 3 + k]))
        .collect::<Vec<_>>();
    let mut faces = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|f| [f[0], f[1], f[2]])
        .collect::<Vec<_>>();

    let mut alive = vec![true; faces.len()];
    let mut active = vec![true; points.len()];
    let mut version = vec![0usize; points.len()];
    let mut radius = vec![0f64; points.len()];
    let mut incident = vec![Vec::<usize>::new(); points.len()];

    for (i, face) in faces.iter().enumerate() {
        for &v in face {
            ordered_push_unique(&mut incident[v], i);
        }
    }

    let mut heap = EdgeHeap::new();
    for face in &faces {
        for i in 0..3 {
            let a = face[i];
            let b = face[(i + 1) % 3];
            if a < b {
                heap.push(a, b, &points, &version, &active);
            } else {
                heap.push(b, a, &points, &version, &active);
            }
        }
    }

    let mut count = faces.len();
    let mut attempts = 0usize;

    // Scratch buffers reused across decimation attempts to avoid a fresh
    // allocation for every edge popped from the heap.
    let mut shared = Vec::new();
    let mut na = Vec::new();
    let mut nb = Vec::new();
    let mut common = Vec::new();
    let mut nv = Vec::new();

    while count > target && !heap.data.is_empty() && attempts < 1_000_000 {
        attempts += 1;
        let e = match heap.pop() {
            Some(edge) => edge,
            None => break,
        };
        let a = e.a;
        let b = e.b;
        if !active[a] || !active[b] || version[a] != e.va || version[b] != e.vb {
            continue;
        }

        ordered_intersection_into(&incident[a], &incident[b], &mut shared);
        if shared.len() != 2 {
            continue;
        }

        neighbors_into(a, &faces, &incident, &mut na);
        neighbors_into(b, &faces, &incident, &mut nb);
        ordered_intersection_into(&na, &nb, &mut common);
        if common.len() != 2 {
            continue;
        }

        let point = std::array::from_fn(|k| (points[a][k] + points[b][k]) / 2.);
        let ra = (point[0] - points[a][0])
            .hypot(point[1] - points[a][1])
            .hypot(point[2] - points[a][2]);
        let rb = (point[0] - points[b][0])
            .hypot(point[1] - points[b][1])
            .hypot(point[2] - points[b][2]);
        let radius_new = (radius[a] + ra).max(radius[b] + rb);
        if radius_new > tolerance {
            continue;
        }

        let mut affected = Vec::new();
        for &i in &incident[a] {
            ordered_push_unique(&mut affected, i);
        }
        for &i in &incident[b] {
            ordered_push_unique(&mut affected, i);
        }

        let mut valid = true;
        for &i in &affected {
            if ordered_contains(&shared, i) {
                continue;
            }
            let f = faces[i];
            let old = [points[f[0]], points[f[1]], points[f[2]]];
            let next = [
                if f[0] == b { point } else { points[f[0]] },
                if f[1] == b { point } else { points[f[1]] },
                if f[2] == b { point } else { points[f[2]] },
            ];
            let n = cross(sub(old[1], old[0]), sub(old[2], old[0]));
            let m = cross(sub(next[1], next[0]), sub(next[2], next[0]));
            let nn = dot(n, n);
            let nm = dot(m, m);
            if nm < 1e-16 || dot(n, m) < 0.2 * nn.sqrt() * nm.sqrt() {
                valid = false;
                break;
            }
        }
        if !valid {
            continue;
        }

        let mut touched = na.clone();
        for &x in &nb {
            ordered_push_unique(&mut touched, x);
        }
        ordered_push_unique(&mut touched, a);
        ordered_push_unique(&mut touched, b);

        for &i in &affected {
            for &w in &faces[i] {
                ordered_remove(&mut incident[w], i);
            }
        }

        points[a] = point;
        radius[a] = radius_new;
        active[b] = false;

        for &i in &affected {
            if ordered_contains(&shared, i) {
                if alive[i] {
                    alive[i] = false;
                    count -= 1;
                }
            } else {
                let f = &mut faces[i];
                for value in f.iter_mut().take(3) {
                    if *value == b {
                        *value = a;
                    }
                }
                for &w in f.iter() {
                    ordered_push_unique(&mut incident[w], i);
                }
            }
        }

        for &v in &touched {
            version[v] += 1;
        }
        for &v in &touched {
            if !active[v] {
                continue;
            }
            neighbors_into(v, &faces, &incident, &mut nv);
            for &neighbor in &nv {
                heap.push(v, neighbor, &points, &version, &active);
            }
        }
    }

    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let mut mapping = BTreeMap::<usize, usize>::new();

    for i in 0..faces.len() {
        if !alive[i] {
            continue;
        }
        for &v in &faces[i] {
            if !active[v] {
                continue;
            }
            let id = match mapping.get(&v) {
                Some(&id) => id,
                None => {
                    let id = positions.len() / 3;
                    mapping.insert(v, id);
                    let p = points[v];
                    positions.extend([
                        (p[0] * 1_000_000.).round() / 1_000_000.,
                        (p[1] * 1_000_000.).round() / 1_000_000.,
                        (p[2] * 1_000_000.).round() / 1_000_000.,
                    ]);
                    id
                }
            };
            indices.push(id);
        }
    }

    Ok(Mesh {
        positions,
        indices,
        uv: None,
    })
}

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

fn clip_polygon(poly: Vec<[f64; 2]>, n: [f64; 2], d: f64) -> Vec<[f64; 2]> {
    let mut result = Vec::new();
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let da = a[0] * n[0] + a[1] * n[1] - d;
        let db = b[0] * n[0] + b[1] * n[1] - d;
        if da <= 1e-9 {
            result.push(a);
        }
        if (da < 0.) != (db < 0.) {
            let t = da / (da - db);
            result.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
        }
    }
    result
}

fn inset_polygon(poly: Vec<[f64; 2]>, distance: f64) -> Vec<[f64; 2]> {
    let mut result = poly.clone();
    // Clipping may remove vertices; offset the original edges, not the result.
    for i in 0..poly.len() {
        if result.is_empty() {
            break;
        }
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy);
        if len < 1e-8 {
            continue;
        }
        let n = [dy / len, -dx / len];
        let boundary = a[0] * n[0] + a[1] * n[1] - distance;
        result = clip_polygon(result, n, boundary);
    }
    result
}

fn area(poly: &[[f64; 2]]) -> f64 {
    let s = poly.iter().enumerate().fold(0.0, |acc, (i, a)| {
        let b = poly[(i + 1) % poly.len()];
        acc + a[0] * b[1] - a[1] * b[0]
    });
    s.abs() * 0.5
}

#[cfg(test)]
mod cell_tests {
    use super::*;

    #[test]
    fn triangle_inset_offsets_each_original_edge_once() {
        let polygon = vec![[0., 0.], [8., 0.], [4., 4. * 3f64.sqrt()]];
        let inset = inset_polygon(polygon, 0.5);
        assert_eq!(inset.len(), 3);
        for i in 0..3 {
            let a = inset[i];
            let b = inset[(i + 1) % 3];
            assert!(((a[0] - b[0]).hypot(a[1] - b[1]) - (8. - 3f64.sqrt())).abs() < 1e-10);
        }
    }

    #[test]
    fn oversized_inset_returns_empty_without_indexing_removed_vertices() {
        assert!(inset_polygon(vec![[0., 0.], [1., 0.], [0., 1.]], 2.).is_empty());
    }

    #[test]
    fn honeycomb_does_not_apply_web_jitter() {
        let regular =
            generate_lightening_cells([0., 0.], [1., 9.], "honeycomb", 6., 0.2, 42., 0.).unwrap();
        let candidate =
            generate_lightening_cells([0., 0.], [1., 9.], "honeycomb", 6., 0.2, 99., 1.).unwrap();
        assert_eq!(candidate, regular);
    }
}

pub fn generate_lightening_cells(
    min: [f64; 2],
    max: [f64; 2],
    pattern: &str,
    cell: f64,
    rib: f64,
    seed0: f64,
    jitter: f64,
) -> Result<Vec<Vec<[f64; 2]>>> {
    if !cell.is_finite()
        || cell <= 0.
        || !rib.is_finite()
        || rib <= 0.
        || !seed0.is_finite()
        || seed0.fract() != 0.
    {
        return Err(input(
            "Lightening cells must use positive finite parameters.",
        ));
    }
    if !jitter.is_finite() || !(0. ..=1.).contains(&jitter) {
        return Err(input("Lightening jitter must be between 0 and 1."));
    }

    let mut cells = Vec::<Vec<[f64; 2]>>::new();
    let width = max[0] - min[0];
    let height = max[1] - min[1];
    let nx = (width / cell).ceil().max(1.);
    let ny = (height / cell).ceil().max(1.);
    if !nx.is_finite() || !ny.is_finite() {
        return Err(input("Lightening domain has non-finite size."));
    }
    let nx = nx as usize;
    let ny = ny as usize;
    let dx = width / nx as f64;
    let dy = height / ny as f64;
    let rect = |x: f64, y: f64, w: f64, h: f64| -> Vec<[f64; 2]> {
        vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
    };
    let boundary = rect(min[0], min[1], width, height);

    if pattern == "grid" || pattern == "triangles" {
        if nx.saturating_mul(ny) > 144 {
            return Err(input("More than 144 cells. Increase cell size."));
        }
        for y in 0..ny {
            for x in 0..nx {
                let p = rect(min[0] + x as f64 * dx, min[1] + y as f64 * dy, dx, dy);
                if pattern == "grid" {
                    cells.push(p);
                } else {
                    cells.push(vec![p[0], p[1], p[2]]);
                    cells.push(vec![p[0], p[2], p[3]]);
                }
            }
        }
    } else if pattern == "isogrid" {
        let h = cell * 3f64.sqrt() / 2.;
        if !min.iter().chain(max.iter()).all(|v| v.is_finite())
            || width <= 0.
            || height <= 0.
            || !h.is_finite()
            || h <= 0.
        {
            return Err(input("Isogrid requires finite increasing bounds."));
        }
        let cols = (width / cell).ceil() + 2.;
        let rows = (height / h).ceil() + 2.;
        if !cols.is_finite() || !rows.is_finite() || cols * rows * 2. > 144. {
            return Err(input("More than 144 cells. Increase cell size."));
        }
        let at = |i: i32, j: i32| {
            [
                min[0] + (i as f64 + j as f64 * 0.5) * cell,
                min[1] + j as f64 * h,
            ]
        };
        for j in -1i32..rows as i32 {
            for i in -j.div_euclid(2) - 2..cols as i32 {
                for tri in [
                    vec![at(i, j), at(i + 1, j), at(i, j + 1)],
                    vec![at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)],
                ] {
                    if tri.iter().flatten().any(|v| !v.is_finite()) {
                        return Err(input("Isogrid exceeds finite numeric range."));
                    }
                    let mut polygon = tri;
                    for (normal, bound) in [
                        ([1., 0.], max[0]),
                        ([-1., 0.], -min[0]),
                        ([0., 1.], max[1]),
                        ([0., -1.], -min[1]),
                    ] {
                        polygon = clip_polygon(polygon, normal, bound);
                    }
                    if polygon.len() >= 3 && area(&polygon) > rib * rib / 4. {
                        cells.push(polygon);
                    }
                }
            }
        }
    } else if pattern == "honeycomb" || pattern == "web" {
        let mut seed = seed0.rem_euclid(4294967296.) as u32;
        let mut random = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            seed as f64 / 4294967296.
        };

        let rows = (height / (cell * 3f64.sqrt() / 2.)).ceil().max(1.) as usize;
        if nx.saturating_mul(rows) > 144 {
            return Err(input("More than 144 sites. Increase cell size."));
        }

        let mut sites = Vec::<[f64; 2]>::new();
        for y in 0..rows {
            for x in 0..nx {
                let jitter_x = if pattern == "web" { jitter } else { 0. };
                let jitter_y = jitter_x;
                let xx = min[0]
                    + (x as f64 + 0.25 + (y % 2) as f64 * 0.5 + (random() - 0.5) * jitter_x) * dx;
                let yy = min[1]
                    + (y as f64 + 0.5 + (random() - 0.5) * jitter_y) * (height / rows as f64);
                sites.push([xx, yy]);
            }
        }

        for a in &sites {
            let mut polygon = boundary.clone();
            for b in &sites {
                if a == b {
                    continue;
                }
                let n = [b[0] - a[0], b[1] - a[1]];
                let d = (b[0] * b[0] + b[1] * b[1] - a[0] * a[0] - a[1] * a[1]) / 2.;
                polygon = clip_polygon(polygon, n, d);
                if polygon.len() < 3 {
                    break;
                }
            }
            cells.push(polygon);
        }
    } else {
        return Err(input("Unknown lightening pattern."));
    }

    Ok(cells
        .into_iter()
        .map(|p| inset_polygon(p, rib / 2.))
        .filter(|p| p.len() >= 3 && area(p) > rib * rib / 8.)
        .collect::<Vec<_>>())
}

#[derive(Clone, Debug)]
pub struct LighteningOptions {
    pub pattern: String,
    pub axis: String,
    pub cell: f64,
    pub rib: f64,
    pub rim: f64,
    pub bottom: f64,
    pub top: f64,
    pub seed: f64,
    pub jitter: f64,
    pub line_width: f64,
    pub perimeters: f64,
    pub skin: f64,
    pub step: f64,
    pub wall_depth: f64,
    pub open_top: bool,
    pub keep_core: bool,
    pub diagonals: bool,
}
pub fn lighten(source_mesh: Mesh, options: LighteningOptions) -> Result<Mesh> {
    use crate::solid::{
        boolean::{Operation, Options, boolean},
        modeling::{Profile, extrude},
    };
    let LighteningOptions {
        pattern,
        axis,
        cell,
        rib,
        rim,
        bottom,
        top,
        seed: seed0,
        jitter,
        line_width,
        perimeters,
        skin,
        step,
        wall_depth,
        open_top,
        keep_core,
        diagonals,
    } = options;
    if ![
        cell, rib, rim, bottom, top, seed0, jitter, line_width, perimeters,
    ]
    .iter()
    .all(|x| x.is_finite())
        || cell <= 0.
        || rib <= 0.
        || rib >= cell / 2.
        || rim < 0.
        || bottom < 0.
        || top < 0.
        || !(0. ..=1.).contains(&jitter)
        || perimeters.fract() != 0.
        || perimeters < 1.
        || perimeters > 8.
        || seed0.fract() != 0.
        || line_width <= 0.
    {
        return Err(input(
            "Check cell size, rib width, borders, seed and print settings. Rib must be smaller than half a cell.",
        ));
    }
    if rib + 1e-6 < line_width * perimeters {
        return Err(input(
            "Rib is thinner than the requested number of extrusion lines. Increase rib width or change the print settings.",
        ));
    }
    if matches!(pattern.as_str(), "bone" | "spatial" | "bcc" | "octet") {
        let g = graph(
            source_mesh.clone(),
            cell,
            jitter,
            seed0,
            &pattern,
            diagonals,
        )
        .map_err(|e| input(format!("Cad lightening graph generation failed: {e}")))?;
        let nodes = g.nodes;
        let edges = g.edges;
        let mut reduced_mesh = crate::mesh_shell::lattice(
            &source_mesh,
            nodes,
            edges,
            rib / 2.,
            skin,
            step,
            pattern == "bone",
            open_top,
            wall_depth,
            keep_core,
        )?
        .mesh;
        reduced_mesh = decimate(reduced_mesh, step * 1.5, 2600.)?;
        let compact_positions = reduced_mesh
            .positions
            .iter()
            .map(|v| (v * 1e6).round() / 1e6)
            .collect::<Vec<_>>();
        reduced_mesh = Mesh {
            positions: compact_positions,
            indices: reduced_mesh.indices,
            uv: reduced_mesh.uv,
        };
        let reduced = reduced_mesh.inspect()?;
        let source = source_mesh.inspect()?;
        if !reduced.closed
            || reduced.degenerate_triangles > 0
            || reduced.signed_volume_mm3 <= 0.
            || reduced.signed_volume_mm3 >= source.signed_volume_mm3
        {
            return Err(input(
                "Lattice simplification failed topology checks. Increase grid resolution.",
            ));
        }
        if component_count(&reduced_mesh, true)? > component_count(&source_mesh, true)? {
            return Err(input(
                "Clipping the spatial graph creates disconnected pieces. Increase strut thickness or add a skin.",
            ));
        }
        Ok(reduced_mesh)
    } else {
        let axis_index = match axis.as_str() {
            "x" => 0,
            "y" => 1,
            "z" => 2,
            _ => return Err(input("Choose X, Y or Z channel direction.")),
        };
        let u = (axis_index + 1) % 3;
        let v_index = (axis_index + 2) % 3;
        let (min, max) =
            crate::scene_flatten::bounds(std::slice::from_ref(&source_mesh.positions))?;

        if max[axis_index] - min[axis_index] <= bottom + top
            || max[u] - min[u] <= 2. * rim + rib
            || max[v_index] - min[v_index] <= 2. * rim + rib
        {
            return Err(input("Skins or frame consume the available body."));
        }

        let before = source_mesh.inspect()?;
        if !before.closed || before.signed_volume_mm3 <= 0. {
            return Err(input("Select a closed outward-oriented solid."));
        }

        let cells = generate_lightening_cells(
            [min[u] + rim, min[v_index] + rim],
            [max[u] - rim, max[v_index] - rim],
            &pattern,
            cell,
            rib,
            seed0,
            jitter,
        )?;
        if cells.is_empty() {
            return Err(input("No openings fit. Reduce rib or frame width."));
        }

        let extension = (max
            .iter()
            .zip(min.iter())
            .map(|(a, b)| a - b)
            .fold(0.0, f64::max))
            * 1e-5
            + 1e-5;
        let start = min[axis_index] + bottom - if bottom == 0. { extension } else { 0. };
        let end = max[axis_index] - top + if top == 0. { extension } else { 0. };
        let height = end - start;
        if !height.is_finite() || height <= 0. {
            return Err(input("Skins or frame consume the available body."));
        }

        let mut cutters = Mesh {
            positions: Vec::new(),
            indices: Vec::new(),
            uv: None,
        };

        let transform = match axis_index {
            0 => [
                [0., 0., 1., start],
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 0., 1.],
            ],
            1 => [
                [1., 0., 0., 0.],
                [0., 0., 1., start],
                [0., 1., 0., 0.],
                [0., 0., 0., 1.],
            ],
            2 => [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., start],
                [0., 0., 0., 1.],
            ],
            _ => unreachable!(),
        };

        for cell in cells {
            let mut cutter = extrude(
                &Profile {
                    outer: cell,
                    holes: vec![],
                },
                [0., 0., height],
            )?
            .mesh;
            cutter = cutter.transform(transform)?;
            let offset = cutters.positions.len() / 3;
            cutters.positions.extend(cutter.positions);
            cutters
                .indices
                .extend(cutter.indices.into_iter().map(|i| i + offset));
        }

        let result = boolean(
            &source_mesh,
            &cutters,
            Operation::Difference,
            &Options::default(),
        )?;
        if result.mesh.indices.is_empty() {
            return Err(input("Lightening removed the entire body."));
        }
        if !result.mesh.inspect()?.closed || result.mesh.inspect()?.signed_volume_mm3 <= 0. {
            return Err(input("Lightening did not produce a closed solid."));
        }
        if result.mesh.inspect()?.signed_volume_mm3 >= before.signed_volume_mm3 - 1e-6 {
            return Err(input(
                "No material was removed. Change channel direction or cell size.",
            ));
        }
        if component_count(&result.mesh, false)? > component_count(&source_mesh, false)? {
            return Err(input(
                "Pattern creates disconnected pieces. Increase the frame/rib width or keep a bottom skin.",
            ));
        }

        let compact_positions = result
            .mesh
            .positions
            .iter()
            .map(|v| (v * 1e6).round() / 1e6)
            .collect::<Vec<_>>();
        let mesh = crate::Mesh {
            positions: compact_positions,
            indices: result.mesh.indices,
            uv: None,
        };
        let check = mesh.inspect()?;
        if !check.closed || check.degenerate_triangles != 0 {
            return Err(input("Pattern creates details below export precision."));
        }

        Ok(mesh)
    }
}
