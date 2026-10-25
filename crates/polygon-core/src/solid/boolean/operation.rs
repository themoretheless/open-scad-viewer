use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Union,
    Intersection,
    Difference,
}


#[derive(Clone, Debug)]
pub struct Options {
    /// Relative to the longest side of the combined input bounds.
    pub relative_tolerance: f64,
    pub max_work: usize,
    pub max_fragments: usize,
    /// Cap on clipped (BSP) output. Separated, nested, identical or empty
    /// operands return whole input surfaces, which are bounded by the inputs
    /// and by `Mesh::validate` instead.
    pub max_output_triangles: usize,
}


/// Largest combined input the BSP clipping path admits. Exact fast paths for
/// separated, nested, identical or empty operands are not limited by it.
pub const BSP_INPUT_TRIANGLES: usize = 10_000;
pub(super) fn default_tolerance() -> f64 {
    1e-9
}
pub(super) fn default_work() -> usize {
    8_000_000
}
pub(super) fn default_fragments() -> usize {
    100_000
}
pub(super) fn default_output() -> usize {
    MAX_TRIANGLES
}
impl Default for Options {
    fn default() -> Self {
        Self {
            relative_tolerance: default_tolerance(),
            max_work: default_work(),
            max_fragments: default_fragments(),
            max_output_triangles: default_output(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct BooleanReport {
    pub operation: Operation,
    pub tolerance_mm: f64,
    pub work: usize,
    pub fragments: usize,
    pub input_triangles: [usize; 2],
}


pub(super) fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error {
        code,
        message: message.into(),
    }
}
pub(super) fn numeric(message: &str) -> Error {
    error("POLYGON_BOOLEAN_NUMERIC_ERROR", message)
}
pub(super) fn invalid(message: &str) -> Error {
    error("POLYGON_BOOLEAN_INVALID_SOLID", message)
}
pub(super) fn limit() -> Error {
    error(
        "POLYGON_BOOLEAN_RESOURCE_LIMIT",
        "Boolean work/fragment/output budget exceeded; reduce mesh complexity",
    )
}
pub(super) struct Budget {
    pub(super) stage: &'static str,
    pub(super) work: usize,
    pub(super) fragments: usize,
    pub(super) options: Options,
}
impl Budget {
    pub(super) fn tick(&mut self, amount: usize) -> Result<()> {
        self.work = self.work.checked_add(amount).ok_or_else(limit)?;
        if self.work > self.options.max_work {
            Err(error(
                "POLYGON_BOOLEAN_RESOURCE_LIMIT",
                format!("Boolean work budget exceeded during {}", self.stage),
            ))
        } else {
            Ok(())
        }
    }
    pub(super) fn fragment(&mut self) -> Result<()> {
        self.fragments += 1;
        if self.fragments > self.options.max_fragments {
            Err(error(
                "POLYGON_BOOLEAN_RESOURCE_LIMIT",
                format!("Boolean work budget exceeded during {}", self.stage),
            ))
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Plane {
    pub(super) normal: Point,
    pub(super) origin: Point,
}
impl Plane {
    pub(super) fn distance(self, p: Point) -> f64 {
        dot(self.normal, sub(p, self.origin))
    }
    pub(super) fn flip(&mut self) {
        self.normal = self.normal.map(|v| -v);
    }
}
#[derive(Clone)]
pub(super) struct Polygon {
    pub(super) vertices: Vec<Point>,
    pub(super) plane: Plane,
}
impl Polygon {
    pub(super) fn new(vertices: Vec<Point>, eps: f64, budget: &mut Budget) -> Result<Option<Self>> {
        let mut clean = Vec::new();
        for p in vertices {
            if clean.last().is_none_or(|q| norm(sub(p, *q)) > eps) {
                clean.push(p);
            }
        }
        if clean.len() > 1 && norm(sub(clean[0], *clean.last().unwrap())) <= eps {
            clean.pop();
        }
        if clean.len() < 3 {
            return Ok(None);
        }
        let origin = clean[0];
        let mut area = [0.; 3];
        for i in 1..clean.len() - 1 {
            let n = cross(sub(clean[i], origin), sub(clean[i + 1], origin));
            for a in 0..3 {
                area[a] += n[a];
            }
        }
        let magnitude = norm(area);
        if magnitude <= eps * eps {
            return Ok(None);
        }
        if !magnitude.is_finite() {
            return Err(numeric("Polygon plane exceeded finite precision"));
        }
        budget.fragment()?;
        Ok(Some(Self {
            vertices: clean,
            plane: Plane {
                normal: scale(area, 1. / magnitude),
                origin,
            },
        }))
    }
    pub(super) fn flip(&mut self) {
        self.vertices.reverse();
        self.plane.flip();
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Coplanar,
    Front,
    Back,
    Spanning,
}
pub(super) fn classify(plane: Plane, p: &Polygon, eps: f64, budget: &mut Budget) -> Result<Side> {
    budget.tick(p.vertices.len())?;
    let mut front = false;
    let mut back = false;
    for v in &p.vertices {
        let d = plane.distance(*v);
        front |= d > eps;
        back |= d < -eps;
    }
    Ok(match (front, back) {
        (false, false) => Side::Coplanar,
        (true, false) => Side::Front,
        (false, true) => Side::Back,
        (true, true) => Side::Spanning,
    })
}
/// Coplanar orientation is retained separately for BSP construction and clipping.
pub(super) struct Split {
    pub(super) same: Vec<Polygon>,
    pub(super) opposite: Vec<Polygon>,
    pub(super) front: Vec<Polygon>,
    pub(super) back: Vec<Polygon>,
}
pub(super) fn split(plane: Plane, polygons: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Split> {
    let mut out = Split {
        same: vec![],
        opposite: vec![],
        front: vec![],
        back: vec![],
    };
    for p in polygons {
        match classify(plane, &p, eps, budget)? {
            Side::Coplanar => {
                if dot(plane.normal, p.plane.normal) > 0. {
                    out.same.push(p)
                } else {
                    out.opposite.push(p)
                }
            }
            Side::Front => out.front.push(p),
            Side::Back => out.back.push(p),
            Side::Spanning => {
                let mut f = Vec::new();
                let mut b = Vec::new();
                for i in 0..p.vertices.len() {
                    let a = p.vertices[i];
                    let next = p.vertices[(i + 1) % p.vertices.len()];
                    let da = plane.distance(a);
                    let db = plane.distance(next);
                    if da >= -eps {
                        f.push(a);
                    }
                    if da <= eps {
                        b.push(a);
                    }
                    if (da > eps && db < -eps) || (da < -eps && db > eps) {
                        let t = da / (da - db);
                        if !t.is_finite() || !(0. ..=1.).contains(&t) {
                            return Err(numeric("Unstable plane intersection"));
                        }
                        let point =
                            std::array::from_fn(|axis| a[axis] + t * (next[axis] - a[axis]));
                        f.push(point);
                        b.push(point);
                    }
                }
                if let Some(poly) = Polygon::new(f, eps, budget)? {
                    out.front.push(poly);
                }
                if let Some(poly) = Polygon::new(b, eps, budget)? {
                    out.back.push(poly);
                }
            }
        }
    }
    Ok(out)
}
pub(super) struct Node {
    pub(super) plane: Plane,
    pub(super) polygons: Vec<Polygon>,
    pub(super) front: Option<usize>,
    pub(super) back: Option<usize>,
}
pub(super) struct Bsp {
    pub(super) nodes: Vec<Node>,
}
impl Bsp {
    pub(super) fn build(polygons: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Self> {
        let mut tree = Self { nodes: vec![] };
        if polygons.is_empty() {
            return Ok(tree);
        }
        let mut stack = vec![(polygons, None)];
        while let Some((polys, parent)) = stack.pop() {
            // Sample candidate planes rather than always selecting input order.
            let samples = polys.len().min(4);
            let mut best = polys[0].plane;
            let mut score = usize::MAX;
            for k in 0..samples {
                let candidate = polys[k * polys.len() / samples].plane;
                let mut front = 0_usize;
                let mut back = 0_usize;
                let mut cuts = 0;
                for p in &polys {
                    match classify(candidate, p, eps, budget)? {
                        Side::Front => front += 1,
                        Side::Back => back += 1,
                        Side::Spanning => cuts += 1,
                        Side::Coplanar => {}
                    }
                }
                let next = cuts * 8 + front.abs_diff(back);
                if next < score {
                    score = next;
                    best = candidate;
                }
                // A supporting plane cannot fragment this set. In particular,
                // every plane of a convex solid has this property; sampling
                // more planes only repeats a quadratic scan of curved solids.
                if cuts == 0 && (front == 0 || back == 0) {
                    best = candidate;
                    break;
                }
            }
            let mut parts = split(best, polys, eps, budget)?;
            parts.same.append(&mut parts.opposite);
            let index = tree.nodes.len();
            if index >= 20_000 {
                return Err(limit());
            }
            tree.nodes.push(Node {
                plane: best,
                polygons: parts.same,
                front: None,
                back: None,
            });
            if let Some((p, front)) = parent {
                let node: &mut Node = &mut tree.nodes[p];
                if front {
                    node.front = Some(index)
                } else {
                    node.back = Some(index)
                }
            }
            if !parts.back.is_empty() {
                stack.push((parts.back, Some((index, false))));
            }
            if !parts.front.is_empty() {
                stack.push((parts.front, Some((index, true))));
            }
        }
        Ok(tree)
    }
    pub(super) fn invert(&mut self) {
        for n in &mut self.nodes {
            n.plane.flip();
            for p in &mut n.polygons {
                p.flip();
            }
            std::mem::swap(&mut n.front, &mut n.back);
        }
    }
    pub(super) fn clip(&self, polys: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Vec<Polygon>> {
        if self.nodes.is_empty() {
            return Ok(polys);
        }
        let mut output = Vec::new();
        let mut stack = vec![(0, polys)];
        while let Some((i, p)) = stack.pop() {
            let n = &self.nodes[i];
            let mut parts = split(n.plane, p, eps, budget)?;
            parts.front.append(&mut parts.same);
            parts.back.append(&mut parts.opposite);
            if let Some(front) = n.front {
                if !parts.front.is_empty() {
                    stack.push((front, parts.front));
                }
            } else {
                output.append(&mut parts.front);
            }
            // Absence of a back child denotes the solid interior.
            if let Some(back) = n.back
                && !parts.back.is_empty()
            {
                stack.push((back, parts.back));
            }
        }
        Ok(output)
    }
    pub(super) fn clip_to(&mut self, other: &Self, eps: f64, budget: &mut Budget) -> Result<()> {
        for n in &mut self.nodes {
            n.polygons = other.clip(std::mem::take(&mut n.polygons), eps, budget)?;
        }
        Ok(())
    }
    pub(super) fn polygons(self) -> Vec<Polygon> {
        self.nodes.into_iter().flat_map(|n| n.polygons).collect()
    }
}
/// Reject disconnected vertex fans (edge counts alone miss pinched vertices).
/// Canonical implementation lives in manifold-core (`validate_vertex_links`).
pub(super) fn vertex_manifold(mesh: &Mesh) -> Result<()> {
    manifold_core::validate_vertex_links(mesh.positions.len() / 3, &mesh.indices).map_err(|defect| {
        invalid(match defect {
            manifold_core::VertexLinkDefect::NonManifoldLink(_) => {
                "Boolean solid has a nonmanifold vertex link"
            }
            manifold_core::VertexLinkDefect::DisconnectedFan(_) => {
                "Boolean solid has a disconnected vertex fan"
            }
        })
    })
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
pub(super) fn merge_coplanar(polygons: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Vec<Polygon>> {
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

pub(super) struct Vertices {
    pub(super) points: Vec<Point>,
    // Only point lookups/insertions by cell key; iteration order never leaks
    // into the output, so hash ordering cannot affect determinism.
    pub(super) cells: FxHashMap<[i64; 3], Vec<usize>>,
    pub(super) eps: f64,
}
impl Vertices {
    pub(super) fn new(eps: f64) -> Self {
        Self {
            points: vec![],
            cells: FxHashMap::default(),
            eps,
        }
    }
    pub(super) fn insert(&mut self, p: Point) -> usize {
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
