//! Bounded regularized solid CSG using oriented polygon BSPs.
//! Implemented here in Rust; no external geometry engine. Binary64 predicates
//! use a common normalized frame. Results are stitched and topology-checked;
//! this is not an exact-arithmetic or globally certified geometry algorithm.
use crate::{
    BuiltMesh, Construction, Error, MAX_TRIANGLES, Mesh, Result, cross, dot, norm, scale, sub,
};
use std::collections::{BTreeMap, BTreeSet};
mod validation;
type Point = math_core::V3;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Union,
    Intersection,
    Difference,
}
impl value_codec::Serialize for Operation {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Union => value_codec::Value::String("union".into()),
            Self::Intersection => value_codec::Value::String("intersection".into()),
            Self::Difference => value_codec::Value::String("difference".into()),
        }
    }
}
impl<'de> value_codec::Deserialize<'de> for Operation {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        match value.as_str().unwrap_or("") {
            "union" => Ok(Self::Union),
            "intersection" => Ok(Self::Intersection),
            "difference" => Ok(Self::Difference),
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
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
impl value_codec::Serialize for Options {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "relativeTolerance".into(),
            value_codec::Serialize::to_value(&self.relative_tolerance),
        );
        object.insert(
            "maxWork".into(),
            value_codec::Serialize::to_value(&self.max_work),
        );
        object.insert(
            "maxFragments".into(),
            value_codec::Serialize::to_value(&self.max_fragments),
        );
        object.insert(
            "maxOutputTriangles".into(),
            value_codec::Serialize::to_value(&self.max_output_triangles),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Options {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let relative_tolerance: f64 = if let Some(v) = object.remove("relativeTolerance") {
            value_codec::Deserialize::from_value(v)?
        } else {
            default_tolerance()
        };
        let max_work: usize = if let Some(v) = object.remove("maxWork") {
            value_codec::Deserialize::from_value(v)?
        } else {
            default_work()
        };
        let max_fragments: usize = if let Some(v) = object.remove("maxFragments") {
            value_codec::Deserialize::from_value(v)?
        } else {
            default_fragments()
        };
        let max_output_triangles: usize = if let Some(v) = object.remove("maxOutputTriangles") {
            value_codec::Deserialize::from_value(v)?
        } else {
            default_output()
        };
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            relative_tolerance,
            max_work,
            max_fragments,
            max_output_triangles,
        })
    }
}
/// Largest combined input the BSP clipping path admits. Exact fast paths for
/// separated, nested, identical or empty operands are not limited by it.
pub const BSP_INPUT_TRIANGLES: usize = 10_000;
fn default_tolerance() -> f64 {
    1e-9
}
fn default_work() -> usize {
    8_000_000
}
fn default_fragments() -> usize {
    100_000
}
fn default_output() -> usize {
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
impl value_codec::Serialize for BooleanReport {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "operation".into(),
            value_codec::Serialize::to_value(&self.operation),
        );
        object.insert(
            "toleranceMm".into(),
            value_codec::Serialize::to_value(&self.tolerance_mm),
        );
        object.insert("work".into(), value_codec::Serialize::to_value(&self.work));
        object.insert(
            "fragments".into(),
            value_codec::Serialize::to_value(&self.fragments),
        );
        object.insert(
            "inputTriangles".into(),
            value_codec::Serialize::to_value(&self.input_triangles),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for BooleanReport {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let operation: Operation = value_codec::Deserialize::from_value(
            object
                .remove("operation")
                .ok_or_else(|| value_codec::error("Missing field operation"))?,
        )?;
        let tolerance_mm: f64 = value_codec::Deserialize::from_value(
            object
                .remove("toleranceMm")
                .ok_or_else(|| value_codec::error("Missing field toleranceMm"))?,
        )?;
        let work: usize = value_codec::Deserialize::from_value(
            object
                .remove("work")
                .ok_or_else(|| value_codec::error("Missing field work"))?,
        )?;
        let fragments: usize = value_codec::Deserialize::from_value(
            object
                .remove("fragments")
                .ok_or_else(|| value_codec::error("Missing field fragments"))?,
        )?;
        let input_triangles: [usize; 2] = value_codec::Deserialize::from_value(
            object
                .remove("inputTriangles")
                .ok_or_else(|| value_codec::error("Missing field inputTriangles"))?,
        )?;
        Ok(Self {
            operation,
            tolerance_mm,
            work,
            fragments,
            input_triangles,
        })
    }
}
fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error {
        code,
        message: message.into(),
    }
}
fn numeric(message: &str) -> Error {
    error("POLYGON_BOOLEAN_NUMERIC_ERROR", message)
}
fn invalid(message: &str) -> Error {
    error("POLYGON_BOOLEAN_INVALID_SOLID", message)
}
fn limit() -> Error {
    error(
        "POLYGON_BOOLEAN_RESOURCE_LIMIT",
        "Boolean work/fragment/output budget exceeded; reduce mesh complexity",
    )
}
struct Budget {
    stage: &'static str,
    work: usize,
    fragments: usize,
    options: Options,
}
impl Budget {
    fn tick(&mut self, amount: usize) -> Result<()> {
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
    fn fragment(&mut self) -> Result<()> {
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
struct Plane {
    normal: Point,
    origin: Point,
}
impl Plane {
    fn distance(self, p: Point) -> f64 {
        dot(self.normal, sub(p, self.origin))
    }
    fn flip(&mut self) {
        self.normal = self.normal.map(|v| -v);
    }
}
#[derive(Clone)]
struct Polygon {
    vertices: Vec<Point>,
    plane: Plane,
}
impl Polygon {
    fn new(vertices: Vec<Point>, eps: f64, budget: &mut Budget) -> Result<Option<Self>> {
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
    fn flip(&mut self) {
        self.vertices.reverse();
        self.plane.flip();
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Coplanar,
    Front,
    Back,
    Spanning,
}
fn classify(plane: Plane, p: &Polygon, eps: f64, budget: &mut Budget) -> Result<Side> {
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
struct Split {
    same: Vec<Polygon>,
    opposite: Vec<Polygon>,
    front: Vec<Polygon>,
    back: Vec<Polygon>,
}
fn split(plane: Plane, polygons: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Split> {
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
struct Node {
    plane: Plane,
    polygons: Vec<Polygon>,
    front: Option<usize>,
    back: Option<usize>,
}
struct Bsp {
    nodes: Vec<Node>,
}
impl Bsp {
    fn build(polygons: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Self> {
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
    fn invert(&mut self) {
        for n in &mut self.nodes {
            n.plane.flip();
            for p in &mut n.polygons {
                p.flip();
            }
            std::mem::swap(&mut n.front, &mut n.back);
        }
    }
    fn clip(&self, polys: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Vec<Polygon>> {
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
    fn clip_to(&mut self, other: &Self, eps: f64, budget: &mut Budget) -> Result<()> {
        for n in &mut self.nodes {
            n.polygons = other.clip(std::mem::take(&mut n.polygons), eps, budget)?;
        }
        Ok(())
    }
    fn polygons(self) -> Vec<Polygon> {
        self.nodes.into_iter().flat_map(|n| n.polygons).collect()
    }
}
/// Reject disconnected vertex fans (edge counts alone miss pinched vertices).
fn vertex_manifold(mesh: &Mesh) -> Result<()> {
    let mut links = BTreeMap::<usize, Vec<(usize, usize)>>::new();
    for t in mesh.indices.as_chunks::<3>().0 {
        for i in 0..3 {
            links
                .entry(t[i])
                .or_default()
                .push((t[(i + 1) % 3], t[(i + 2) % 3]));
        }
    }
    for edges in links.values() {
        let mut adjacent = BTreeMap::<usize, Vec<usize>>::new();
        for &(a, b) in edges {
            adjacent.entry(a).or_default().push(b);
            adjacent.entry(b).or_default().push(a);
        }
        if adjacent.values().any(|v| v.len() != 2) {
            return Err(invalid("Boolean solid has a nonmanifold vertex link"));
        }
        let mut seen = BTreeSet::new();
        let mut pending = vec![*adjacent.first_key_value().unwrap().0];
        while let Some(v) = pending.pop() {
            if seen.insert(v) {
                pending.extend(&adjacent[&v]);
            }
        }
        if seen.len() != adjacent.len() {
            return Err(invalid("Boolean solid has a disconnected vertex fan"));
        }
    }
    Ok(())
}
fn validate_solid(mesh: &Mesh) -> Result<()> {
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
fn normalized(mesh: &Mesh, origin: Point, scale: f64) -> Result<Mesh> {
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
fn polygons(mesh: &Mesh, eps: f64, budget: &mut Budget) -> Result<Vec<Polygon>> {
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
                budget.tick(v.len().saturating_mul(v.len()))?;
                if (0..v.len()).all(|a| (a + 1..v.len()).all(|b| norm(sub(v[a], v[b])) > eps))
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
    cells: BTreeMap<[i64; 3], Vec<usize>>,
    eps: f64,
}
impl Vertices {
    fn new(eps: f64) -> Self {
        Self {
            points: vec![],
            cells: BTreeMap::new(),
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
fn stitch(polys: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Mesh> {
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
    let sorted: [Vec<usize>; 3] = std::array::from_fn(|axis| {
        let mut v: Vec<_> = (0..vertices.points.len()).collect();
        v.sort_by(|a, b| vertices.points[*a][axis].total_cmp(&vertices.points[*b][axis]));
        v
    });
    let mut edges = BTreeMap::<(usize, usize), Vec<usize>>::new();
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
            if let std::collections::btree_map::Entry::Vacant(entry) = edges.entry(key) {
                let p = vertices.points[key.0];
                let q = vertices.points[key.1];
                let direction = sub(q, p);
                let length = norm(direction);
                if length <= eps {
                    return Err(numeric("Collapsed Boolean edge"));
                }
                let axis = (0..3)
                    .min_by_key(|&axis| {
                        let low = p[axis].min(q[axis]) - eps;
                        let high = p[axis].max(q[axis]) + eps;
                        sorted[axis].partition_point(|i| vertices.points[*i][axis] <= high)
                            - sorted[axis].partition_point(|i| vertices.points[*i][axis] < low)
                    })
                    .unwrap();
                let sorted = &sorted[axis];
                let low = p[axis].min(q[axis]) - eps;
                let high = p[axis].max(q[axis]) + eps;
                let start = sorted.partition_point(|i| vertices.points[*i][axis] < low);
                let end = sorted.partition_point(|i| vertices.points[*i][axis] <= high);
                let mut along = vec![(0., key.0), (1., key.1)];
                for &id in &sorted[start..end] {
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
                along.sort_by(|a, b| a.0.total_cmp(&b.0));
                entry.insert(along.into_iter().map(|(_, i)| i).collect());
            }
            let sequence = &edges[&key];
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
/// Union, intersection or A-minus-B of closed oriented solid meshes. Empty
/// meshes are valid identities. UVs are dropped because CSG creates new faces.
pub fn boolean(a: &Mesh, b: &Mesh, operation: Operation, options: &Options) -> Result<BuiltMesh> {
    a.validate()?;
    b.validate()?;
    if !(1e-12..=1e-5).contains(&options.relative_tolerance)
        || options.max_work == 0
        || options.max_work > 8_000_000
        || options.max_fragments == 0
        || options.max_fragments > 100_000
        || options.max_output_triangles == 0
        || options.max_output_triangles > MAX_TRIANGLES
    {
        return Err(error(
            "POLYGON_BOOLEAN_INVALID_OPTIONS",
            "Invalid Boolean tolerance or work/fragment/output limits",
        ));
    }
    let input_triangles = [a.indices.len() / 3, b.indices.len() / 3];
    // Inputs above the BSP admission cap can still take an exact fast path
    // (separated, nested, identical or empty operands). Those results are
    // bounded by their inputs, so only the BSP path keeps the cap; the
    // tolerance-budgeted intersection audits are skipped above it and the
    // report says so through `self_intersection_status`.
    let audited = input_triangles.iter().sum::<usize>() <= BSP_INPUT_TRIANGLES;
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for mesh in [a, b] {
        for i in &mesh.indices {
            let p = mesh.point(*i)?;
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
    }
    let empty = a.indices.is_empty() && b.indices.is_empty();
    let origin = if empty {
        [0.; 3]
    } else {
        std::array::from_fn(|i| min[i] / 2. + max[i] / 2.)
    };
    let extent = if empty {
        1.
    } else {
        (0..3).map(|i| max[i] - min[i]).fold(0., f64::max)
    };
    if !extent.is_finite() || extent <= 0. {
        return Err(numeric("Solid bounds cannot be normalized"));
    }
    let a = normalized(a, origin, extent)?;
    let b = normalized(b, origin, extent)?;
    validate_solid(&a)?;
    validate_solid(&b)?;
    let eps = options.relative_tolerance;
    let mut budget = Budget {
        stage: "input validation",
        work: 0,
        fragments: 0,
        options: options.clone(),
    };
    if audited {
        for (index, mesh) in [&a, &b].into_iter().enumerate() {
            validation::geometry(mesh, eps, &mut budget)
                .map_err(|e| error(e.code, format!("Input {index}: {}", e.message)))?;
            validation::orientation(mesh, eps, &mut budget)?;
        }
    }
    // Exact mesh identity avoids unnecessary splitting of densely sampled surfaces.
    // Validation above still applies: identity cannot admit malformed solids.
    let contains = |outer: &Mesh, inner: &Mesh| -> Result<bool> {
        if outer.indices.is_empty() || inner.indices.is_empty() || outer.indices.len() / 3 > 128 {
            return Ok(false);
        }
        for t in outer.indices.as_chunks::<3>().0 {
            let p = outer.point(t[0])?;
            let n = cross(sub(outer.point(t[1])?, p), sub(outer.point(t[2])?, p));
            let tolerance = eps * norm(n);
            if outer
                .positions
                .as_chunks::<3>()
                .0
                .iter()
                .any(|v| dot(n, sub([v[0], v[1], v[2]], p)) > tolerance)
                || inner
                    .positions
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .any(|v| dot(n, sub([v[0], v[1], v[2]], p)) >= -tolerance)
            {
                return Ok(false);
            }
        }
        Ok(true)
    };
    let center = |m: &Mesh| {
        std::array::from_fn::<_, 3, _>(|k| {
            (m.positions
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min)
                + m.positions
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max))
                / 2.
        })
    };
    let axis = sub(center(&b), center(&a));
    let separated = !a.indices.is_empty()
        && !b.indices.is_empty()
        && a.positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| dot(axis, [p[0], p[1], p[2]]))
            .fold(f64::NEG_INFINITY, f64::max)
            + eps * norm(axis)
            < b.positions
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| dot(axis, [p[0], p[1], p[2]]))
                .fold(f64::INFINITY, f64::min);
    // `fast` marks results assembled from whole input surfaces without BSP
    // clipping; their size is bounded by the inputs and they carry no new
    // seams, so the output cap and coplanar simplification do not apply.
    let mut fast = true;
    let mut result = if separated {
        match operation {
            Operation::Union => crate::solid::primitives::join(&[a, b])?,
            Operation::Difference => a,
            Operation::Intersection => crate::solid::primitives::empty(),
        }
    } else if contains(&a, &b)? {
        match operation {
            Operation::Union => a,
            Operation::Intersection => b,
            Operation::Difference => {
                let mut cavity = b;
                cavity.reverse_winding();
                crate::solid::primitives::join(&[a, cavity])?
            }
        }
    } else if contains(&b, &a)? {
        match operation {
            Operation::Union => b,
            Operation::Intersection => a,
            Operation::Difference => crate::solid::primitives::empty(),
        }
    } else if a.positions == b.positions && a.indices == b.indices {
        match operation {
            Operation::Union | Operation::Intersection => a,
            Operation::Difference => Mesh {
                positions: vec![],
                indices: vec![],
                uv: None,
            },
        }
    } else if a.indices.is_empty() || b.indices.is_empty() {
        budget.stage = "clipping";
        match operation {
            Operation::Union => {
                if a.indices.is_empty() {
                    b
                } else {
                    a
                }
            }
            Operation::Difference => a,
            Operation::Intersection => Mesh {
                positions: vec![],
                indices: vec![],
                uv: None,
            },
        }
    } else {
        if !audited {
            return Err(limit());
        }
        fast = false;
        budget.stage = "BSP construction";
        let mut a = Bsp::build(polygons(&a, eps, &mut budget)?, eps, &mut budget)
            .map_err(|e| error(e.code, format!("Build A: {}", e.message)))?;
        let mut b = Bsp::build(polygons(&b, eps, &mut budget)?, eps, &mut budget)
            .map_err(|e| error(e.code, format!("Build B: {}", e.message)))?;
        budget.stage = "clipping";
        match operation {
            Operation::Union => {
                a.clip_to(&b, eps, &mut budget)?;
                b.clip_to(&a, eps, &mut budget)?;
                b.invert();
                b.clip_to(&a, eps, &mut budget)?;
                b.invert();
            }
            Operation::Difference => {
                a.invert();
                a.clip_to(&b, eps, &mut budget)?;
                b.clip_to(&a, eps, &mut budget)?;
                b.invert();
                b.clip_to(&a, eps, &mut budget)?;
                b.invert();
            }
            Operation::Intersection => {
                a.invert();
                b.clip_to(&a, eps, &mut budget)?;
                b.invert();
                a.clip_to(&b, eps, &mut budget)?;
                b.clip_to(&a, eps, &mut budget)?;
            }
        }
        let mut polys = a.polygons();
        polys.extend(b.polygons());
        if operation != Operation::Union {
            for p in &mut polys {
                p.flip();
            }
        }
        budget.stage = "stitch";
        stitch(polys, eps, &mut budget)
            .map_err(|e| error(e.code, format!("Stitch: {}", e.message)))?
    };
    if !fast && result.indices.len() / 3 > options.max_output_triangles {
        return Err(limit());
    }
    // Remove internal triangulation seams before the expensive intersection
    // audit. The audit still checks the final surface, including its topology.
    // Audited fast-path results keep the simplification so their bytes match
    // the pre-fast-path output; above the cap it is skipped because it groups
    // faces per plane and a large curved surface has one plane per triangle.
    if (!fast || audited) && !result.indices.is_empty() {
        // Coplanar retriangulation is an optimization; difficult multi-hole caps
        // may fail its polygon bridge heuristic. Keep the stitched triangles in
        // that case and run the same topology/intersection/orientation audits.
        if let Ok(simplified) = crate::solid::primitives::simplify(&result) {
            result = simplified;
        }
    }
    validate_solid(&result).map_err(|e| {
        error(
            "POLYGON_BOOLEAN_NON_MANIFOLD_RESULT",
            format!(
                "Boolean result is not a valid solid at this tolerance: {}",
                e.message
            ),
        )
    })?;
    if audited {
        budget.stage = "result validation";
        validation::geometry(&result, eps, &mut budget).map_err(|e| {
            if e.code == "POLYGON_BOOLEAN_RESOURCE_LIMIT" {
                e
            } else {
                error("POLYGON_BOOLEAN_NON_MANIFOLD_RESULT", e.message)
            }
        })?;
        validation::orientation(&result, eps, &mut budget)?;
    }
    for p in result.positions.as_chunks_mut::<3>().0 {
        for axis in 0..3 {
            p[axis] = origin[axis] + p[axis] * extent;
        }
    }
    let mut report = result.inspect()?;
    if !result.indices.is_empty() && !report.closed {
        return Err(numeric(
            "World-coordinate precision cannot represent the Boolean result",
        ));
    }
    report.construction = Construction::Boolean;
    report.self_intersection_status = if audited {
        "checked_with_tolerance".into()
    } else {
        "not_checked".into()
    };
    report.boolean = Some(BooleanReport {
        operation,
        tolerance_mm: eps * extent,
        work: budget.work,
        fragments: budget.fragments,
        input_triangles,
    });
    Ok(BuiltMesh {
        mesh: result,
        report,
    })
}
#[cfg(test)]
mod tests;

/// Axis-aligned bounds of the vertices a mesh's triangles reference.
pub fn bounds(mesh: &Mesh) -> Option<(Point, Point)> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for &i in &mesh.indices {
        let p = mesh.point(i).ok()?;
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    (!mesh.indices.is_empty()).then_some((min, max))
}
/// Bounds that touch or overlap within `pad` are treated as connected, so face
/// contact is never mistaken for separation.
fn bounds_touch(a: Option<(Point, Point)>, b: Option<(Point, Point)>, pad: f64) -> bool {
    match (a, b) {
        (Some((alo, ahi)), Some((blo, bhi))) => {
            (0..3).all(|k| alo[k] <= bhi[k] + pad && blo[k] <= ahi[k] + pad)
        }
        _ => false,
    }
}
/// Connectivity tolerance for operand bounds: relative to the overall extent
/// like the BSP tolerance, so a scaled model behaves like the original.
fn bounds_pad(boxes: &[Option<(Point, Point)>]) -> f64 {
    let extent = boxes
        .iter()
        .flatten()
        .flat_map(|(lo, hi)| (0..3).map(move |k| hi[k] - lo[k]))
        .fold(0., f64::max);
    default_tolerance() * 8. * extent.max(1.)
}
/// Groups of operands whose bounds are transitively connected, in first-index
/// order; every operand appears in exactly one group.
fn connected_groups(boxes: &[Option<(Point, Point)>], pad: f64) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..boxes.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            if bounds_touch(boxes[i], boxes[j], pad) {
                let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                if ri != rj {
                    parent[rj.max(ri)] = rj.min(ri);
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut slot = vec![usize::MAX; boxes.len()];
    for i in 0..boxes.len() {
        let r = root(&mut parent, i);
        if slot[r] == usize::MAX {
            slot[r] = groups.len();
            groups.push(Vec::new());
        }
        groups[slot[r]].push(i);
    }
    groups
}
/// Union of many solids. Operands whose bounds are separated never meet in
/// CSG: each bound-connected group is folded through `pairwise` in operand
/// order and the group results are joined, which is exact for disjoint
/// surfaces. A single operand is returned unchanged.
pub fn union_many(
    meshes: &[Mesh],
    pairwise: &mut dyn FnMut(&Mesh, &Mesh) -> Result<Mesh>,
) -> Result<Mesh> {
    let boxes: Vec<_> = meshes.iter().map(bounds).collect();
    let pad = bounds_pad(&boxes);
    let mut parts = Vec::new();
    for group in connected_groups(&boxes, pad) {
        let mut folded: Option<Mesh> = None;
        for i in group {
            folded = Some(match folded {
                None => meshes[i].clone(),
                Some(m) => pairwise(&m, &meshes[i])?,
            });
        }
        if let Some(m) = folded {
            parts.push(m);
        }
    }
    match parts.len() {
        0 => Ok(crate::solid::primitives::empty()),
        1 => Ok(parts.pop().unwrap()),
        _ => crate::solid::primitives::join(&parts),
    }
}
/// `base` minus every cutter, as A \ (B1 ∪ B2 …) = ((A \ B1) \ B2) …. Cutters
/// whose bounds miss the base are skipped; the rest are subtracted in batches
/// of at most `batch` mutually separated cutters, joined without CSG. Small
/// batches keep every BSP step local instead of clipping the whole base
/// against one large joined cutter.
pub fn difference_many(
    base: &Mesh,
    cutters: &[Mesh],
    pairwise: &mut dyn FnMut(&Mesh, &Mesh) -> Result<Mesh>,
    batch: usize,
) -> Result<Mesh> {
    let batch = batch.max(1);
    let base_box = bounds(base);
    if base_box.is_none() {
        return Ok(crate::solid::primitives::empty());
    }
    let boxes: Vec<_> = cutters.iter().map(bounds).collect();
    let pad = bounds_pad(&[&[base_box][..], &boxes[..]].concat());
    let mut batches: Vec<Vec<usize>> = Vec::new();
    for (i, cutter_box) in boxes.iter().enumerate() {
        if !bounds_touch(base_box, *cutter_box, pad) {
            continue;
        }
        let open = batches.iter_mut().find(|members| {
            members.len() < batch
                && members
                    .iter()
                    .all(|&j| !bounds_touch(boxes[j], *cutter_box, pad))
        });
        match open {
            Some(members) => members.push(i),
            None => batches.push(vec![i]),
        }
    }
    let mut result = base.clone();
    for members in batches {
        let cutter = if members.len() == 1 {
            cutters[members[0]].clone()
        } else {
            let parts: Vec<Mesh> = members.iter().map(|&j| cutters[j].clone()).collect();
            crate::solid::primitives::join(&parts)?
        };
        result = pairwise(&result, &cutter)?;
        if result.indices.is_empty() {
            break;
        }
    }
    Ok(result)
}

/// Reconnect triangulated planar face boundaries after exact planar remeshing.
pub(crate) fn stitch_mesh(mesh: &Mesh, eps: f64) -> Result<Mesh> {
    let mut budget = Budget {
        stage: "stitch",
        work: 0,
        fragments: 0,
        options: Options::default(),
    };
    let p = polygons(mesh, eps, &mut budget)?;
    stitch(p, eps, &mut budget)
}
