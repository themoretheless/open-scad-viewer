use super::*;

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
pub(super) fn default_tolerance() -> f64 {
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
pub(super) struct Bsp {
    nodes: Vec<Node>,
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
