use super::*;
use math_core::{cross, dot};

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

pub(super) fn component_count(mesh: &Mesh, positive_only: bool) -> Result<usize> {
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
        let p: [[f64; 3]; 3] =
            std::array::from_fn(|i| std::array::from_fn(|k| mesh.positions[f[i] * 3 + k] - origin[k]));
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

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] - b[k])
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
            for i in 0..nv.len() {
                heap.push(v, nv[i], &points, &version, &active);
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

pub(super) fn clip_polygon(poly: Vec<[f64; 2]>, n: [f64; 2], d: f64) -> Vec<[f64; 2]> {
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

pub(super) fn inset_polygon(poly: Vec<[f64; 2]>, distance: f64) -> Vec<[f64; 2]> {
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

pub(super) fn area(poly: &[[f64; 2]]) -> f64 {
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
