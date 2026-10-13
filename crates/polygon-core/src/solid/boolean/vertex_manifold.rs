use super::*;

/// Reject disconnected vertex fans (edge counts alone miss pinched vertices).
pub(super) fn vertex_manifold(mesh: &Mesh) -> Result<()> {
    let vertex_count = mesh.positions.len() / 3;
    let triangles = mesh.indices.as_chunks::<3>().0;
    // CSR fan edges: fan[offsets[v]..offsets[v + 1]] holds the link edges
    // (next, previous) of every triangle corner at v, in triangle order
    // (same layout style as `build_face_adjacency` in mesh_render).
    let mut offsets = vec![0usize; vertex_count + 1];
    for t in triangles {
        for &id in t {
            offsets[id + 1] += 1;
        }
    }
    for v in 0..vertex_count {
        offsets[v + 1] += offsets[v];
    }
    let mut fan = vec![(0usize, 0usize); offsets[vertex_count]];
    let mut cursors = offsets[..vertex_count].to_vec();
    for t in triangles {
        for i in 0..3 {
            let cursor = &mut cursors[t[i]];
            fan[*cursor] = (t[(i + 1) % 3], t[(i + 2) % 3]);
            *cursor += 1;
        }
    }
    // Reused scratch buffers: generation-stamped membership, degree/cursor per
    // link node and a DFS stack, so dense meshes do not allocate per vertex.
    // The result is only Ok/Err with fixed messages, so traversal order
    // cannot affect the output.
    let mut link_mark = vec![usize::MAX; vertex_count];
    let mut seen_mark = vec![usize::MAX; vertex_count];
    let mut degree = vec![0usize; vertex_count];
    let mut slot = vec![0usize; vertex_count];
    let mut members: Vec<usize> = Vec::new();
    let mut link_offsets: Vec<usize> = Vec::new();
    let mut adjacent: Vec<usize> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for v in 0..vertex_count {
        let range = offsets[v]..offsets[v + 1];
        if range.is_empty() {
            continue;
        }
        let edges = &fan[range];
        members.clear();
        for &(a, b) in edges {
            for id in [a, b] {
                if link_mark[id] != v {
                    link_mark[id] = v;
                    degree[id] = 0;
                    slot[id] = members.len();
                    members.push(id);
                }
                degree[id] += 1;
            }
        }
        // Repeated incidences count, as in the previous map-based version.
        if members.iter().any(|&id| degree[id] != 2) {
            return Err(invalid("Boolean solid has a nonmanifold vertex link"));
        }
        // Flat per-link adjacency (CSR over `members`) for the DFS below.
        link_offsets.clear();
        link_offsets.push(0);
        for &id in &members {
            link_offsets.push(link_offsets.last().unwrap() + degree[id]);
        }
        adjacent.clear();
        adjacent.resize(*link_offsets.last().unwrap(), 0);
        for &id in &members {
            degree[id] = link_offsets[slot[id]];
        }
        for &(a, b) in edges {
            adjacent[degree[a]] = b;
            degree[a] += 1;
            adjacent[degree[b]] = a;
            degree[b] += 1;
        }
        stack.clear();
        stack.push(members[0]);
        seen_mark[members[0]] = v;
        let mut seen = 1usize;
        while let Some(w) = stack.pop() {
            for &n in &adjacent[link_offsets[slot[w]]..link_offsets[slot[w] + 1]] {
                if seen_mark[n] != v {
                    seen_mark[n] = v;
                    seen += 1;
                    stack.push(n);
                }
            }
        }
        if seen != members.len() {
            return Err(invalid("Boolean solid has a disconnected vertex fan"));
        }
    }
    Ok(())
}
pub(super) fn validate_solid(mesh: &Mesh) -> Result<()> {
    if mesh.indices.is_empty() {
        return Ok(());
    }
    let r = mesh.inspect()?;
    if !r.closed || r.signed_volume_mm3 <= 0. {
        return Err(invalid(
            "Booleans require closed, consistently outward-oriented solid meshes",
        ));
    }
    vertex_manifold(mesh)
}
pub(super) fn normalized(mesh: &Mesh, origin: Point, scale: f64) -> Result<Mesh> {
    let mut map = BTreeMap::new();
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for old in &mesh.indices {
        let new = if let Some(&i) = map.get(old) {
            i
        } else {
            let i = positions.len() / 3;
            positions.extend(
                mesh.point(*old)?
                    .iter()
                    .enumerate()
                    .map(|(a, x)| (x - origin[a]) / scale),
            );
            map.insert(*old, i);
            i
        };
        indices.push(new);
    }
    let result = Mesh {
        positions,
        indices,
        uv: None,
    };
    result.validate()?;
    Ok(result)
}
pub(super) fn polygons(mesh: &Mesh, eps: f64, budget: &mut Budget) -> Result<Vec<Polygon>> {
    let polygons = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            Polygon::new(
                vec![mesh.point(t[0])?, mesh.point(t[1])?, mesh.point(t[2])?],
                eps,
                budget,
            )?
            .ok_or_else(|| numeric("Input triangle is smaller than the Boolean tolerance"))
        })
        .collect::<Result<Vec<_>>>()?;
    merge_coplanar(polygons, eps, budget)
}
// Reuse planar faces instead of repeatedly splitting their triangulation edges.
// Only adjacent, coplanar polygons whose union is convex may be merged.
fn merge_coplanar(polygons: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Vec<Polygon>> {
    let key = |a: Point, b: Point| {
        (
            a.map(|x| (x / eps).round() as i64),
            b.map(|x| (x / eps).round() as i64),
        )
    };
    let mut edges = BTreeMap::new();
    let mut slots: Vec<_> = polygons.into_iter().map(Some).collect();
    for (i, p) in slots.iter().enumerate() {
        let p = p.as_ref().unwrap();
        for k in 0..p.vertices.len() {
            edges.insert(
                key(p.vertices[k], p.vertices[(k + 1) % p.vertices.len()]),
                (i, k),
            );
        }
    }
    for i in 0..slots.len() {
        while let Some(p) = slots[i].as_ref() {
            let mut pair = None;
            for k in 0..p.vertices.len() {
                budget.tick(1)?;
                let Some(&(j, l)) =
                    edges.get(&key(p.vertices[(k + 1) % p.vertices.len()], p.vertices[k]))
                else {
                    continue;
                };
                if i == j {
                    continue;
                }
                let Some(q) = slots[j].as_ref() else { continue };
                if dot(p.plane.normal, q.plane.normal) <= 1. - 1e-10
                    || p.plane.distance(q.plane.origin).abs() >= eps
                {
                    continue;
                }
                let mut v = Vec::new();
                for offset in 1..=p.vertices.len() {
                    v.push(p.vertices[(k + offset) % p.vertices.len()]);
                }
                for offset in 2..q.vertices.len() {
                    v.push(q.vertices[(l + offset) % q.vertices.len()]);
                }
                // Distinctness via the same quantization as `key` above:
                // O(v log v) instead of the O(v^2) pairwise distance check.
                budget.tick(v.len())?;
                let mut seen = BTreeSet::new();
                if v.iter().all(|p| seen.insert(p.map(|x| (x / eps).round() as i64)))
                    && (0..v.len()).all(|t| {
                        dot(
                            cross(
                                sub(v[(t + 1) % v.len()], v[t]),
                                sub(v[(t + 2) % v.len()], v[(t + 1) % v.len()]),
                            ),
                            p.plane.normal,
                        ) >= -eps * eps
                    })
                {
                    pair = Some((j, v));
                    break;
                }
            }
            let Some((j, v)) = pair else { break };
            let Some(merged) = Polygon::new(v, eps, budget)? else {
                break;
            };
            for index in [i, j] {
                let old = slots[index].take().unwrap();
                for k in 0..old.vertices.len() {
                    edges.remove(&key(
                        old.vertices[k],
                        old.vertices[(k + 1) % old.vertices.len()],
                    ));
                }
            }
            for k in 0..merged.vertices.len() {
                edges.insert(
                    key(
                        merged.vertices[k],
                        merged.vertices[(k + 1) % merged.vertices.len()],
                    ),
                    (i, k),
                );
            }
            slots[i] = Some(merged);
        }
    }
    Ok(slots.into_iter().flatten().collect())
}

struct Vertices {
    points: Vec<Point>,
    // Only point lookups/insertions by cell key; iteration order never leaks
    // into the output, so hash ordering cannot affect determinism.
    cells: FxHashMap<[i64; 3], Vec<usize>>,
    eps: f64,
}
impl Vertices {
    pub(super) fn new(eps: f64) -> Self {
        Self {
            points: vec![],
            cells: FxHashMap::default(),
            eps,
        }
    }
    fn insert(&mut self, p: Point) -> usize {
        let cell = p.map(|v| (v / self.eps).floor() as i64);
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(indices) = self.cells.get(&[cell[0] + x, cell[1] + y, cell[2] + z])
                    {
                        for &i in indices {
                            if norm(sub(self.points[i], p)) <= self.eps {
                                return i;
                            }
                        }
                    }
                }
            }
        }
        let id = self.points.len();
        self.points.push(p);
        self.cells.entry(cell).or_default().push(id);
        id
    }
}
pub(super) fn stitch(polys: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Mesh> {
    let mut vertices = Vertices::new(eps);
    let mut faces = Vec::new();
    for p in polys {
        let mut face: Vec<usize> = p.vertices.into_iter().map(|v| vertices.insert(v)).collect();
        face.dedup();
        if face.len() > 1 && face.first() == face.last() {
            face.pop();
        }
        if face.len() >= 3 {
            faces.push(face);
        }
    }
    // All fragment vertices participate in edge splitting, including those
    // introduced by an adjacent face. This removes BSP T junctions.
    // Split candidates must lie within the eps-expanded bbox of the edge
    // (projection inside the segment plus perpendicular distance <= eps
    // implies per-axis containment), so a uniform grid with cells no smaller
    // than the longest edge span finds them in O(1) cell lookups per edge.
    // This replaces three sorted sweeps over all vertices (3 * O(n log n)
    // plus three index arrays). The eps-sized `Vertices.cells` cannot serve
    // here: walking it along an edge costs O((length / eps)^3) cell visits.
    let mut span = eps;
    for face in &faces {
        for i in 0..face.len() {
            let (a, b) = (face[i], face[(i + 1) % face.len()]);
            if a == b {
                continue;
            }
            let (p, q) = (vertices.points[a], vertices.points[b]);
            for axis in 0..3 {
                span = span.max((p[axis] - q[axis]).abs());
            }
        }
    }
    // Buckets are filled in ascending vertex order, but the per-edge sequence
    // is fixed by the explicit sort below, so neither hash ordering nor cell
    // traversal order can affect the output.
    let mut grid = FxHashMap::<[i64; 3], Vec<usize>>::default();
    for (id, p) in vertices.points.iter().enumerate() {
        grid.entry(p.map(|v| (v / span).floor() as i64))
            .or_default()
            .push(id);
    }
    // Lookup-only map: output order is fixed by face/vertex traversal, so the
    // hasher cannot affect determinism.
    let mut edges = FxHashMap::<(usize, usize), Vec<usize>>::default();
    let mut indices = Vec::new();
    for face in faces {
        let mut boundary = Vec::new();
        for i in 0..face.len() {
            let a = face[i];
            let b = face[(i + 1) % face.len()];
            if a == b {
                continue;
            }
            let key = (a.min(b), a.max(b));
            let sequence = match edges.entry(key) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let p = vertices.points[key.0];
                    let q = vertices.points[key.1];
                    let direction = sub(q, p);
                    let length = norm(direction);
                    if length <= eps {
                        return Err(numeric("Collapsed Boolean edge"));
                    }
                    let mut along = vec![(0., key.0), (1., key.1)];
                    // Cells overlapping the eps-expanded edge bbox: with a
                    // cell size of at least the longest edge span this is a
                    // constant number of cells per axis. Every point passing
                    // the precise filter below lies in one of these cells.
                    let low: [f64; 3] =
                        std::array::from_fn(|axis| (p[axis].min(q[axis]) - eps) / span);
                    let high: [f64; 3] =
                        std::array::from_fn(|axis| (p[axis].max(q[axis]) + eps) / span);
                    for cx in low[0].floor() as i64..=high[0].floor() as i64 {
                        for cy in low[1].floor() as i64..=high[1].floor() as i64 {
                            for cz in low[2].floor() as i64..=high[2].floor() as i64 {
                                let Some(cell) = grid.get(&[cx, cy, cz]) else {
                                    continue;
                                };
                                for &id in cell {
                                    budget.tick(1)?;
                                    if id == key.0 || id == key.1 {
                                        continue;
                                    }
                                    let delta = sub(vertices.points[id], p);
                                    let t = dot(delta, direction) / (length * length);
                                    if t > eps / length
                                        && t < 1. - eps / length
                                        && norm(sub(delta, scale(direction, t))) <= eps
                                    {
                                        along.push((t, id));
                                    }
                                }
                            }
                        }
                    }
                    // Parameter order along the edge; the index tiebreak keeps
                    // the sequence deterministic when two candidates share a
                    // bitwise-equal parameter.
                    along.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                    entry.insert(along.into_iter().map(|(_, i)| i).collect())
                }
            };
            if a == key.0 {
                boundary.extend_from_slice(&sequence[..sequence.len() - 1]);
            } else {
                boundary.extend(sequence.iter().rev().take(sequence.len() - 1));
            }
        }
        if boundary.len() < 3 {
            continue;
        }
        if boundary.len() == 3 {
            indices.extend(&boundary);
        } else {
            let center = std::array::from_fn(|a| {
                boundary.iter().map(|i| vertices.points[*i][a]).sum::<f64>() / boundary.len() as f64
            });
            // Do not re-use a boundary vertex as a centroid of a thin polygon.
            let id = vertices.points.len();
            vertices.points.push(center);
            for i in 0..boundary.len() {
                indices.extend([id, boundary[i], boundary[(i + 1) % boundary.len()]]);
            }
        }
        if indices.len() / 3 > budget.options.max_output_triangles {
            return Err(limit());
        }
    }
    // Canonical triangle accounting removes coincident zero-thickness faces.
    let mut unique = BTreeMap::<[usize; 3], ([usize; 3], i32)>::new();
    for t in indices.as_chunks::<3>().0 {
        let t = [t[0], t[1], t[2]];
        let mut key = t;
        key.sort();
        let inversions =
            usize::from(t[0] > t[1]) + usize::from(t[0] > t[2]) + usize::from(t[1] > t[2]);
        let sign = if inversions % 2 == 0 { 1 } else { -1 };
        let entry = unique.entry(key).or_insert((t, 0));
        entry.1 += sign;
    }
    let mut out = Mesh {
        positions: vec![],
        indices: vec![],
        uv: None,
    };
    let mut remap = BTreeMap::new();
    for (_, (mut t, count)) in unique {
        if count == 0 {
            continue;
        }
        let inversions =
            usize::from(t[0] > t[1]) + usize::from(t[0] > t[2]) + usize::from(t[1] > t[2]);
        if (inversions % 2 == 0) != (count > 0) {
            t.swap(1, 2);
        }
        for old in t {
            let id = *remap.entry(old).or_insert_with(|| {
                let id = out.positions.len() / 3;
                out.positions.extend(vertices.points[old]);
                id
            });
            out.indices.push(id);
        }
    }
    Ok(out)
}
