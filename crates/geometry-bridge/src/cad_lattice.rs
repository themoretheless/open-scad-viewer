//! Native spatial lattice utilities: graph generation, component counting,
//! topology-preserving decimation, and lightening cell construction.
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;
use std::collections::{BTreeMap, BTreeSet};

fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

pub fn components(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    encode(component_count(&mesh, field(&v, "positiveOnly")?)?)
}

fn component_count(mesh: &Mesh, positive_only: bool) -> Result<usize> {
    mesh.validate()?;
    let mut parent = (0..mesh.positions.len() / 3).collect::<Vec<_>>();
    for f in mesh.indices.chunks_exact(3) {
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
    for f in mesh.indices.chunks_exact(3) {
        let r = root(&mut parent, f[0]);
        let origin = &mesh.positions[r * 3..r * 3 + 3];
        let p = f
            .iter()
            .map(|&i| std::array::from_fn::<_, 3, _>(|k| mesh.positions[i * 3 + k] - origin[k]))
            .collect::<Vec<_>>();
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
    values.iter().any(|&v| v == x)
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

fn ordered_intersection(a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut out = Vec::new();
    for &x in a {
        if ordered_contains(b, x) {
            out.push(x);
        }
    }
    out
}

fn neighbors(v: usize, faces: &[[usize; 3]], incident: &[Vec<usize>]) -> Vec<usize> {
    let mut out = Vec::new();
    for &fi in &incident[v] {
        for &w in &faces[fi] {
            if w != v {
                ordered_push_unique(&mut out, w);
            }
        }
    }
    out
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

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] - b[k])
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn decimate(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    let tolerance: f64 = field(&v, "tolerance")?;
    if !tolerance.is_finite() || tolerance < 0. {
        return Err(input(
            "Lattice decimation tolerance must be a non-negative finite number.",
        ));
    }
    let target: usize = {
        let t: f64 = field(&v, "target")?;
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

        let shared = ordered_intersection(&incident[a], &incident[b]);
        if shared.len() != 2 {
            continue;
        }

        let na = neighbors(a, &faces, &incident);
        let nb = neighbors(b, &faces, &incident);
        let common = ordered_intersection(&na, &nb);
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
                for k in 0..3 {
                    if f[k] == b {
                        f[k] = a;
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
            for n in neighbors(v, &faces, &incident) {
                heap.push(v, n, &points, &version, &active);
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

    encode(Mesh {
        positions,
        indices,
        uv: None,
    })
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
    let mut result = poly;
    for i in 0..result.len() {
        let a = result[i];
        let b = result[(i + 1) % result.len()];
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

pub fn lightening_cells(v: Value) -> Result<Value> {
    let min: [f64; 2] = field(&v, "min")?;
    let max: [f64; 2] = field(&v, "max")?;
    let pattern: String = field(&v, "pattern")?;
    let cell: f64 = field(&v, "cell")?;
    let rib: f64 = field(&v, "rib")?;
    let jitter: f64 = field(&v, "jitter")?;
    let seed0: f64 = field(&v, "seed")?;
    encode(generate_lightening_cells(
        min, max, &pattern, cell, rib, seed0, jitter,
    )?)
}

fn generate_lightening_cells(
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
                let jitter_y = jitter;
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

pub fn graph(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    mesh.validate()?;
    let options: Value = field(&v, "options")?;
    let cell: f64 = field(&options, "cell")?;
    let jitter: f64 = field(&options, "jitter")?;
    let seed: f64 = field(&options, "seed")?;
    let pattern: String = field(&options, "pattern")?;
    let diagonals = if options.get("diagonals").is_some() {
        field::<bool>(&options, "diagonals")?
    } else {
        false
    };
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
    if !["spatial", "bone"].contains(&pattern.as_str()) {
        return Err(input("Invalid spatial lattice pattern."));
    }

    let (min, max) = polygon_core::scene_flatten::bounds(&[mesh.positions])?;
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

    encode(value_codec::json!({"nodes": nodes, "edges": edges}))
}

pub fn lightening(v: Value) -> Result<Value> {
    use polygon_core::solid::{
        boolean::{Operation, Options, boolean},
        modeling::{Profile, extrude},
    };
    use value_codec::json;

    let body: Value = field(&v, "body")?;
    let source_mesh: Mesh = field(&body, "mesh")?;
    let pattern: String = field(&v, "pattern")?;
    let axis: String = field(&v, "axis")?;
    let cell: f64 = field(&v, "cell")?;
    let rib: f64 = field(&v, "rib")?;
    let rim: f64 = field(&v, "rim")?;
    let bottom: f64 = field(&v, "bottom")?;
    let top: f64 = field(&v, "top")?;
    let seed0: f64 = field(&v, "seed")?;
    let jitter: f64 = field(&v, "jitter")?;
    let line_width: f64 = field(&v, "lineWidth")?;
    let perimeters: f64 = field(&v, "perimeters")?;

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
    if pattern == "bone" || pattern == "spatial" {
        let skin: f64 = if v.get("skin").is_some_and(|field| !field.is_null()) {
            field(&v, "skin")?
        } else {
            0.
        };
        let step: f64 = if v.get("step").is_some_and(|field| !field.is_null()) {
            field(&v, "step")?
        } else {
            (rib / 3.).min(if skin > 0. { skin / 2. } else { f64::INFINITY })
        };
        let open_top: bool = if v.get("openTop").is_some_and(|field| !field.is_null()) {
            field(&v, "openTop")?
        } else {
            false
        };
        let wall_depth: f64 = if v.get("wallDepth").is_some_and(|field| !field.is_null()) {
            field(&v, "wallDepth")?
        } else {
            0.
        };
        let keep_core: bool = if v.get("keepCore").is_some_and(|field| !field.is_null()) {
            field(&v, "keepCore")?
        } else {
            false
        };

        let graph = graph(
            json!({"mesh":source_mesh,"options":{"cell":cell,"jitter":jitter,"seed":seed0,"pattern":pattern,"diagonals":if v
                .get("diagonals")
                .is_some_and(|field| !field.is_null())
            {
                field(&v, "diagonals")?
            } else {
                false
            }}}),
        )
        .map_err(|e| input(format!("Cad lightening graph generation failed: {e}")))?;
        let nodes: Vec<[f64; 3]> = field(&graph, "nodes")?;
        let edges: Vec<[usize; 2]> = field(&graph, "edges")?;
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
        reduced_mesh = value_codec::from_value(
            decimate(value_codec::json!({
                "mesh": reduced_mesh,
                "tolerance": step * 1.5,
                "target": 2600f64
            }))?,
        )
        .map_err(|e| input(format!("Invalid mesh: {e}")))?;
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
        let mut output = body
            .as_object()
            .ok_or_else(|| input("Expected body record"))?
            .clone();
        output.insert("mesh".into(), encode(reduced_mesh.clone())?);
        if component_count(&reduced_mesh, true)? > component_count(&source_mesh, true)? {
            return Err(input(
                "Clipping the spatial graph creates disconnected pieces. Increase strut thickness or add a skin.",
            ));
        }
        Ok(Value::Object(output))
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
            polygon_core::scene_flatten::bounds(std::slice::from_ref(&source_mesh.positions))?;

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
        let mesh = polygon_core::Mesh {
            positions: compact_positions,
            indices: result.mesh.indices,
            uv: None,
        };
        let check = mesh.inspect()?;
        if !check.closed || check.degenerate_triangles != 0 {
            return Err(input("Pattern creates details below export precision."));
        }

        let mut output = body
            .as_object()
            .ok_or_else(|| input("Expected body record"))?
            .clone();
        output.insert("mesh".into(), encode(mesh)?);
        Ok(Value::Object(output))
    }
}

/// User-facing admission message, kept identical to the pre-migration TS text.
const PRINT_SETTINGS_MESSAGE: &str =
    "Проверьте сопло, высоту слоя, число линий и длину моста.";

fn print_setting(settings: &Value, key: &str) -> Result<f64> {
    settings[key]
        .as_f64()
        .ok_or_else(|| input(PRINT_SETTINGS_MESSAGE))
}

/// Optional lightening option: absent/null stays absent, present values must be
/// finite numbers (the pre-migration TS computed NaN silently).
fn lattice_option(options: &Value, key: &str) -> Result<Option<f64>> {
    let v = &options[key];
    if v.is_null() {
        return Ok(None);
    }
    match v.as_f64() {
        Some(x) if x.is_finite() => Ok(Some(x)),
        _ => Err(input("Lattice geometry options must be finite numbers.")),
    }
}

fn required_lattice_option(options: &Value, key: &str) -> Result<f64> {
    lattice_option(options, key)?
        .ok_or_else(|| input("Lattice geometry options must be finite numbers."))
}

/// Legacy rounding order: ceil toward the step with a 1e-8 epsilon, then
/// compact to 1e6 grid. Kept bit-identical to the pre-migration TS helper.
fn round_to_step(v: f64, s: f64) -> f64 {
    (((v - 1e-8) / s).ceil() * s * 1e6).round() / 1e6
}

pub fn print_fit(v: Value) -> Result<Value> {
    let options = field::<Value>(&v, "options")?;
    let settings = field::<Value>(&v, "settings")?;
    if !options.is_object() || !settings.is_object() {
        return Err(input(PRINT_SETTINGS_MESSAGE));
    }
    let nozzle = print_setting(&settings, "nozzle")?;
    let layer = print_setting(&settings, "layer")?;
    let lines = print_setting(&settings, "lines")?;
    let skin_layers = print_setting(&settings, "skinLayers")?;
    let max_bridge = print_setting(&settings, "maxBridge")?;
    let open_top = settings["openTop"]
        .as_bool()
        .ok_or_else(|| input(PRINT_SETTINGS_MESSAGE))?;
    if ![nozzle, layer, lines, skin_layers, max_bridge]
        .iter()
        .all(|v| v.is_finite())
        || nozzle <= 0.
        || layer <= 0.
        || layer > nozzle
        || lines.fract() != 0.
        || lines < 1.
        || lines > 8.
        || skin_layers.fract() != 0.
        || skin_layers < 0.
        || max_bridge <= 0.
    {
        return Err(input(PRINT_SETTINGS_MESSAGE));
    }

    let width = (nozzle * 1.125 * 1e6).round() / 1e6;
    let rib = round_to_step(
        required_lattice_option(&options, "rib")?.max(width * lines),
        width,
    );
    let cell = required_lattice_option(&options, "cell")?.max(rib * 2. + width);
    let spatial = matches!(options["pattern"].as_str(), Some("bone" | "spatial"));

    let mut result = options;
    result["lineWidth"] = Value::from(width);
    result["perimeters"] = settings["lines"].clone();
    result["rib"] = Value::from(rib);
    result["cell"] = Value::from(cell);
    if spatial {
        let skin = match lattice_option(&result, "skin")? {
            // JS truthiness: absent/zero skin stays open, anything else is fitted.
            Some(s) if s != 0. => round_to_step(s.max(width * lines), width),
            _ => 0.,
        };
        result["skin"] = Value::from(skin);
        result["openTop"] = Value::from(open_top);
        let mut step = lattice_option(&result, "step")?.unwrap_or(rib / 3.);
        step = step.min(rib / 3.);
        if skin != 0. {
            step = step.min(skin / 2.);
        }
        if let Some(wall) = lattice_option(&result, "wallDepth")? {
            if wall != 0. {
                step = step.min(wall / 2.);
            }
        }
        result["step"] = Value::from(step);
    } else {
        result["axis"] = Value::from("z");
        let floor = skin_layers * layer;
        let bottom = required_lattice_option(&result, "bottom")?;
        let top = required_lattice_option(&result, "top")?;
        let rim = required_lattice_option(&result, "rim")?;
        result["bottom"] = Value::from(round_to_step(bottom.max(floor), layer));
        result["top"] = Value::from(if open_top {
            0.
        } else {
            round_to_step(top.max(floor), layer)
        });
        result["rim"] = Value::from(round_to_step(rim.max(width * lines), width));
    }
    Ok(result)
}

pub fn bridge_warning(v: Value) -> Result<Value> {
    // Legacy comparison semantics: absent or non-finite inputs never warn.
    let warn = match (
        v["cell"].as_f64(),
        v["rib"].as_f64(),
        v["maxBridge"].as_f64(),
    ) {
        (Some(cell), Some(rib), Some(max_bridge))
            if cell.is_finite() && rib.is_finite() && max_bridge.is_finite() =>
        {
            cell - rib > max_bridge
        }
        _ => false,
    };
    Ok(Value::from(warn))
}
