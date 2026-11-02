//! Indexed triangle geometry. Faceting resolution is explicit; no B-rep claims.
use crate::{Error, Result, math::*, require};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    vertices: Vec<Point>,
    triangles: Vec<[usize; 3]>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Topology {
    pub boundary_edges: usize,
    pub nonmanifold_edges: usize,
    pub nonmanifold_vertices: usize,
    pub orientation_conflicts: usize,
    pub components: usize,
}
impl Topology {
    pub fn is_closed_manifold(&self) -> bool {
        self.boundary_edges == 0
            && self.nonmanifold_edges == 0
            && self.nonmanifold_vertices == 0
            && self.orientation_conflicts == 0
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub triangle: usize,
    pub distance: f64,
    pub barycentric: [f64; 3],
    pub point: Point,
}

impl Mesh {
    pub fn new(vertices: Vec<Point>, triangles: Vec<[usize; 3]>) -> Result<Self> {
        require(
            vertices.len() <= 2_000_000 && triangles.len() <= 2_000_000,
            "mesh budget exceeded",
        )?;
        require(
            vertices.iter().copied().all(finite),
            "finite vertices required",
        )?;
        require(
            triangles.iter().flatten().all(|i| *i < vertices.len()),
            "triangle index out of range",
        )?;
        require(
            triangles.iter().all(|t| {
                let n = cross(
                    sub(vertices[t[1]], vertices[t[0]]),
                    sub(vertices[t[2]], vertices[t[0]]),
                );
                finite(n) && norm(n) > 0.
            }),
            "degenerate or overflowing triangle",
        )?;
        Ok(Self {
            vertices,
            triangles,
        })
    }
    pub fn empty() -> Self {
        Self {
            vertices: vec![],
            triangles: vec![],
        }
    }
    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }
    pub fn triangles(&self) -> &[[usize; 3]] {
        &self.triangles
    }
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }
    pub fn triangle_normal(&self, index: usize) -> Result<Point> {
        let t = self
            .triangles
            .get(index)
            .ok_or(Error::InvalidInput("unknown triangle"))?;
        unit(cross(
            sub(self.vertices[t[1]], self.vertices[t[0]]),
            sub(self.vertices[t[2]], self.vertices[t[0]]),
        ))
    }
    pub fn surface_area(&self) -> f64 {
        self.triangles
            .iter()
            .map(|t| {
                norm(cross(
                    sub(self.vertices[t[1]], self.vertices[t[0]]),
                    sub(self.vertices[t[2]], self.vertices[t[0]]),
                )) / 2.
            })
            .sum()
    }
    /// Oriented tetrahedron sum. Meaningful as volume only for a closed boundary.
    pub fn signed_volume(&self) -> f64 {
        let Some(&origin) = self.vertices.first() else {
            return 0.;
        };
        self.triangles
            .iter()
            .map(|t| {
                dot(
                    sub(self.vertices[t[0]], origin),
                    cross(
                        sub(self.vertices[t[1]], origin),
                        sub(self.vertices[t[2]], origin),
                    ),
                ) / 6.
            })
            .sum()
    }
    pub fn bounds(&self) -> Option<(Point, Point)> {
        let &p = self.vertices.first()?;
        let mut lo = p;
        let mut hi = p;
        for v in &self.vertices {
            for i in 0..3 {
                lo[i] = lo[i].min(v[i]);
                hi[i] = hi[i].max(v[i]);
            }
        }
        Some((lo, hi))
    }
    pub fn transformed(&self, t: &Affine) -> Result<Self> {
        let mut triangles = self.triangles.clone();
        if t.determinant() < 0. {
            for f in &mut triangles {
                f.swap(1, 2)
            }
        }
        Self::new(
            self.vertices.iter().map(|p| t.apply(*p)).collect(),
            triangles,
        )
    }
    pub fn topology(&self) -> Topology {
        let mut edges: BTreeMap<(usize, usize), Vec<(usize, bool)>> = BTreeMap::new();
        for (f, t) in self.triangles.iter().enumerate() {
            for i in 0..3 {
                let a = t[i];
                let b = t[(i + 1) % 3];
                edges
                    .entry((a.min(b), a.max(b)))
                    .or_default()
                    .push((f, a < b));
            }
        }
        let mut neighbors = vec![Vec::new(); self.triangles.len()];
        let mut out = Topology {
            boundary_edges: 0,
            nonmanifold_edges: 0,
            nonmanifold_vertices: 0,
            orientation_conflicts: 0,
            components: 0,
        };
        for uses in edges.values() {
            if uses.len() == 1 {
                out.boundary_edges += 1
            }
            if uses.len() > 2 {
                out.nonmanifold_edges += 1
            }
            if uses.len() == 2 && uses[0].1 == uses[1].1 {
                out.orientation_conflicts += 1
            }
            for u in uses.iter().skip(1) {
                neighbors[uses[0].0].push(u.0);
                neighbors[u.0].push(uses[0].0);
            }
        }
        // Edge incidence is necessary but not sufficient: reject disconnected
        // incident face fans at a shared vertex (pinched/bow-tie vertices).
        let mut incident = vec![Vec::new(); self.vertices.len()];
        for (f, t) in self.triangles.iter().enumerate() {
            for &v in t {
                incident[v].push(f);
            }
        }
        for (v, faces) in incident.iter().enumerate() {
            if faces.is_empty() {
                continue;
            }
            let allowed = faces.iter().copied().collect::<BTreeSet<_>>();
            let mut seen = BTreeSet::new();
            let mut stack = vec![faces[0]];
            while let Some(f) = stack.pop() {
                if !seen.insert(f) {
                    continue;
                }
                for &g in &neighbors[f] {
                    if allowed.contains(&g) && self.triangles[g].contains(&v) {
                        stack.push(g);
                    }
                }
            }
            if seen.len() != faces.len() {
                out.nonmanifold_vertices += 1;
            }
        }
        let mut visited = vec![false; self.triangles.len()];
        for i in 0..visited.len() {
            if visited[i] {
                continue;
            }
            out.components += 1;
            let mut stack = vec![i];
            while let Some(j) = stack.pop() {
                if visited[j] {
                    continue;
                }
                visited[j] = true;
                stack.extend(neighbors[j].iter().copied());
            }
        }
        out
    }
    /// Adjacent triangle IDs across indexed edges; geometry is not welded.
    pub fn adjacent_triangles(&self, index: usize) -> Result<Vec<usize>> {
        let t = self
            .triangles
            .get(index)
            .ok_or(Error::InvalidInput("unknown triangle"))?;
        Ok(self
            .triangles
            .iter()
            .enumerate()
            .filter(|(i, u)| *i != index && u.iter().filter(|v| t.contains(v)).count() >= 2)
            .map(|(i, _)| i)
            .collect())
    }
    /// Two-sided ray intersection; distance is in caller length units.
    /// Floating point, no watertight/grazing-hit certificate.
    pub fn raycast(&self, origin: Point, direction: Point) -> Result<Option<RayHit>> {
        require(finite(origin), "finite ray origin required")?;
        let direction = unit(direction)?;
        let mut best: Option<RayHit> = None;
        for (i, t) in self.triangles.iter().enumerate() {
            let a = self.vertices[t[0]];
            let e1 = sub(self.vertices[t[1]], a);
            let e2 = sub(self.vertices[t[2]], a);
            let p = cross(direction, e2);
            let det = dot(e1, p);
            if det == 0. || !det.is_finite() {
                continue;
            }
            let s = sub(origin, a);
            let u = dot(s, p) / det;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let q = cross(s, e1);
            let v = dot(direction, q) / det;
            if v < 0. || u + v > 1. {
                continue;
            }
            let distance = dot(e2, q) / det;
            if distance < 0. || !distance.is_finite() {
                continue;
            }
            if best.is_none_or(|b| distance < b.distance) {
                best = Some(RayHit {
                    triangle: i,
                    distance,
                    barycentric: [1. - u - v, u, v],
                    point: add(origin, scale(direction, distance)),
                });
            }
        }
        Ok(best)
    }
    pub fn cuboid(bounds: Bounds) -> Result<Self> {
        Bounds::new(bounds.min, bounds.max)?;
        let a = bounds.min;
        let b = bounds.max;
        let vertices = vec![
            [a[0], a[1], a[2]],
            [b[0], a[1], a[2]],
            [b[0], b[1], a[2]],
            [a[0], b[1], a[2]],
            [a[0], a[1], b[2]],
            [b[0], a[1], b[2]],
            [b[0], b[1], b[2]],
            [a[0], b[1], b[2]],
        ];
        Self::new(
            vertices,
            vec![
                [0, 2, 1],
                [0, 3, 2],
                [4, 5, 6],
                [4, 6, 7],
                [0, 1, 5],
                [0, 5, 4],
                [1, 2, 6],
                [1, 6, 5],
                [2, 3, 7],
                [2, 7, 6],
                [3, 0, 4],
                [3, 4, 7],
            ],
        )
    }
    /// Closed consistently oriented triangle boundary only. Numerical integration
    /// does not certify that the embedded surface has no self-intersections.
    pub fn mass_properties(&self) -> Result<(f64, Point)> {
        require(
            !self.is_empty() && self.topology().is_closed_manifold(),
            "closed nonempty manifold required",
        )?;
        let origin = self.vertices[0];
        let mut volume = 0.;
        let mut moment = [0.; 3];
        for t in &self.triangles {
            let a = sub(self.vertices[t[0]], origin);
            let b = sub(self.vertices[t[1]], origin);
            let c = sub(self.vertices[t[2]], origin);
            let v = dot(a, cross(b, c)) / 6.;
            volume += v;
            moment = add(moment, scale(add(add(a, b), c), v / 4.));
        }
        require(
            volume > 0. && volume.is_finite(),
            "positive finite oriented volume required",
        )?;
        let center = add(origin, scale(moment, 1. / volume));
        require(finite(center), "mass moment overflow")?;
        Ok((volume, center))
    }
    /// Unordered triangle/plane crossing segments. Coplanar triangles are refused;
    /// endpoints are not welded and tangential point contacts are omitted.
    pub fn section_segments(&self, normal: Point, offset: f64) -> Result<Vec<[Point; 2]>> {
        let n = unit(normal)?;
        let len = norm(normal);
        require(offset.is_finite(), "finite plane offset required")?;
        let d = offset / len;
        let mut out = Vec::new();
        for t in &self.triangles {
            let points = t.map(|i| self.vertices[i]);
            let dist = points.map(|p| dot(n, p) - d);
            if dist.iter().all(|x| *x == 0.) {
                return Err(Error::Unsupported("coplanar triangle section"));
            }
            let mut hits = Vec::new();
            for i in 0..3 {
                let j = (i + 1) % 3;
                if dist[i] == 0. && !hits.contains(&points[i]) {
                    hits.push(points[i]);
                }
                if (dist[i] < 0. && dist[j] > 0.) || (dist[i] > 0. && dist[j] < 0.) {
                    let hit = lerp(points[i], points[j], dist[i] / (dist[i] - dist[j]));
                    if !hits.contains(&hit) {
                        hits.push(hit);
                    }
                }
            }
            if hits.len() == 2 {
                out.push([hits[0], hits[1]]);
            }
        }
        Ok(out)
    }
    /// Simple polygon extrusion; planar XY profile, no holes.
    pub fn extrude(profile: &[[f64; 2]], height: f64) -> Result<Self> {
        require(
            height.is_finite() && height > 0.,
            "positive extrusion height required",
        )?;
        let caps = triangulate(profile)?;
        let n = profile.len();
        let mut vertices = Vec::new();
        for z in [0., height] {
            vertices.extend(profile.iter().map(|p| [p[0], p[1], z]));
        }
        let mut triangles = Vec::new();
        let ccw = signed_area(profile) > 0.;
        for mut t in caps {
            if !ccw {
                t.swap(1, 2)
            }
            triangles.push([t[0], t[2], t[1]]);
            triangles.push([t[0] + n, t[1] + n, t[2] + n]);
        }
        for i in 0..n {
            let j = (i + 1) % n;
            let (a, b) = if ccw { (i, j) } else { (j, i) };
            triangles.extend([[a, b, b + n], [a, b + n, a + n]]);
        }
        Self::new(vertices, triangles)
    }
    pub fn cylinder(radius: f64, height: f64, segments: usize) -> Result<Self> {
        require(
            radius.is_finite() && radius > 0. && (3..=4096).contains(&segments),
            "invalid cylinder radius/segments",
        )?;
        Self::extrude(
            &(0..segments)
                .map(|i| {
                    let a = std::f64::consts::TAU * i as f64 / segments as f64;
                    [radius * a.cos(), radius * a.sin()]
                })
                .collect::<Vec<_>>(),
            height,
        )
    }
    pub fn sphere(radius: f64, latitude: usize, longitude: usize) -> Result<Self> {
        require(
            radius.is_finite()
                && radius > 0.
                && (2..=512).contains(&latitude)
                && (3..=1024).contains(&longitude),
            "invalid sphere arguments",
        )?;
        let mut v = vec![[0., 0., radius]];
        for i in 1..latitude {
            let a = std::f64::consts::PI * i as f64 / latitude as f64;
            for j in 0..longitude {
                let b = std::f64::consts::TAU * j as f64 / longitude as f64;
                v.push([
                    radius * a.sin() * b.cos(),
                    radius * a.sin() * b.sin(),
                    radius * a.cos(),
                ]);
            }
        }
        let bottom = v.len();
        v.push([0., 0., -radius]);
        let mut t = Vec::new();
        for j in 0..longitude {
            let k = (j + 1) % longitude;
            t.push([0, 1 + j, 1 + k]);
            t.push([
                bottom,
                1 + (latitude - 2) * longitude + k,
                1 + (latitude - 2) * longitude + j,
            ]);
        }
        for i in 0..latitude - 2 {
            for j in 0..longitude {
                let a = 1 + i * longitude + j;
                let b = 1 + i * longitude + (j + 1) % longitude;
                t.extend([[a, a + longitude, b + longitude], [a, b + longitude, b]]);
            }
        }
        let mut mesh = Self::new(v, t)?;
        mesh.orient_positive();
        Ok(mesh)
    }
    pub fn torus(major: f64, minor: f64, segments: [usize; 2]) -> Result<Self> {
        let [n, m] = segments;
        require(
            major.is_finite()
                && minor.is_finite()
                && major > minor
                && minor > 0.
                && (3..=1024).contains(&n)
                && (3..=1024).contains(&m),
            "invalid ring torus",
        )?;
        let mut v = Vec::new();
        for i in 0..n {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            for j in 0..m {
                let b = std::f64::consts::TAU * j as f64 / m as f64;
                v.push([
                    (major + minor * b.cos()) * a.cos(),
                    (major + minor * b.cos()) * a.sin(),
                    minor * b.sin(),
                ]);
            }
        }
        let mut t = Vec::new();
        for i in 0..n {
            for j in 0..m {
                let a = i * m + j;
                let b = ((i + 1) % n) * m + j;
                let c = ((i + 1) % n) * m + (j + 1) % m;
                let d = i * m + (j + 1) % m;
                t.extend([[a, b, c], [a, c, d]]);
            }
        }
        Self::new(v, t)
    }
    /// Closed faceted solid between corresponding simple XY section polygons.
    /// Caps are horizontal. Side self-intersections are not certified.
    pub fn loft(sections: &[Vec<[f64; 2]>], heights: &[f64]) -> Result<Self> {
        require(
            (2..=1024).contains(&sections.len())
                && sections.len() == heights.len()
                && heights.iter().all(|x| x.is_finite())
                && heights.windows(2).all(|s| s[0] < s[1]),
            "increasing finite loft heights required",
        )?;
        let n = sections[0].len();
        require(
            n >= 3 && sections.iter().all(|s| s.len() == n && signed_area(s) > 0.),
            "matching CCW section vertex counts required",
        )?;
        for s in sections {
            triangulate(s)?;
        }
        let v = sections
            .iter()
            .zip(heights)
            .flat_map(|(s, &z)| s.iter().map(move |p| [p[0], p[1], z]))
            .collect();
        let mut t = Vec::new();
        for f in triangulate(&sections[0])? {
            t.push([f[0], f[2], f[1]]);
        }
        let offset = (sections.len() - 1) * n;
        for f in triangulate(sections.last().unwrap())? {
            t.push(f.map(|i| i + offset));
        }
        for k in 0..sections.len() - 1 {
            for i in 0..n {
                let a = k * n + i;
                let b = k * n + (i + 1) % n;
                t.extend([[a, b, b + n], [a, b + n, a + n]]);
            }
        }
        Self::new(v, t)
    }
    fn orient_positive(&mut self) {
        if self.signed_volume() < 0. {
            for t in &mut self.triangles {
                t.swap(1, 2)
            }
        }
    }
    pub fn to_obj(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        for p in &self.vertices {
            writeln!(&mut out, "v {} {} {}", p[0], p[1], p[2]).unwrap();
        }
        for t in &self.triangles {
            writeln!(&mut out, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1).unwrap();
        }
        out
    }
    /// Only vertex and triangular face records; ignores comments, refuses others.
    pub fn from_obj(text: &str) -> Result<Self> {
        require(text.len() <= 64 * 1024 * 1024, "OBJ byte budget exceeded")?;
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("");
            let words = line.split_whitespace().collect::<Vec<_>>();
            if words.is_empty() {
                continue;
            }
            match words[0] {
                "v" => {
                    require(words.len() == 4, "OBJ vertex needs XYZ")?;
                    let p = words[1..]
                        .iter()
                        .map(|x| {
                            x.parse::<f64>()
                                .map_err(|_| Error::InvalidInput("invalid OBJ number"))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    vertices.push([p[0], p[1], p[2]]);
                }
                "f" => {
                    require(words.len() == 4, "triangular OBJ faces required")?;
                    let mut t = [0; 3];
                    for i in 0..3 {
                        let k = words[i + 1]
                            .parse::<i64>()
                            .map_err(|_| Error::Unsupported("plain OBJ indices only"))?;
                        let index = if k > 0 {
                            k - 1
                        } else {
                            vertices.len() as i64 + k
                        };
                        require(
                            k != 0 && index >= 0 && (index as usize) < vertices.len(),
                            "OBJ index outside current vertices",
                        )?;
                        t[i] = index as usize;
                    }
                    triangles.push(t);
                }
                _ => return Err(Error::Unsupported("OBJ supports only v/f records")),
            }
            require(
                vertices.len() <= 2_000_000 && triangles.len() <= 2_000_000,
                "OBJ geometry budget exceeded",
            )?;
        }
        Self::new(vertices, triangles)
    }
    pub fn to_stl(&self) -> Result<String> {
        use std::fmt::Write;
        let mut out = String::from("solid independent\n");
        for (i, t) in self.triangles.iter().enumerate() {
            let n = self.triangle_normal(i)?;
            writeln!(
                &mut out,
                " facet normal {} {} {}\n  outer loop",
                n[0], n[1], n[2]
            )
            .unwrap();
            for &v in t {
                let p = self.vertices[v];
                writeln!(&mut out, "   vertex {} {} {}", p[0], p[1], p[2]).unwrap();
            }
            out.push_str("  endloop\n endfacet\n");
        }
        out.push_str("endsolid independent\n");
        Ok(out)
    }
}

fn cross2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn signed_area(p: &[[f64; 2]]) -> f64 {
    (0..p.len())
        .map(|i| {
            let q = p[(i + 1) % p.len()];
            p[i][0] * q[1] - p[i][1] * q[0]
        })
        .sum::<f64>()
        / 2.
}
/// Ear clipping on a simple polygon; rejects touches/crossings and collinear corners.
fn triangulate(p: &[[f64; 2]]) -> Result<Vec<[usize; 3]>> {
    require(
        (3..=4096).contains(&p.len()) && p.iter().flatten().all(|x| x.is_finite()),
        "finite simple polygon required",
    )?;
    let area = signed_area(p);
    require(
        area.is_finite() && area != 0.,
        "polygon has zero or overflowing area",
    )?;
    let sign = area.signum();
    let n = p.len();
    for i in 0..n {
        require(
            cross2(p[(i + n - 1) % n], p[i], p[(i + 1) % n]) != 0.,
            "collinear or duplicate polygon corner",
        )?;
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let a = p[i];
            let b = p[(i + 1) % n];
            let c = p[j];
            let d = p[(j + 1) % n];
            let f = [
                cross2(a, b, c),
                cross2(a, b, d),
                cross2(c, d, a),
                cross2(c, d, b),
            ];
            let overlap = (0..2)
                .all(|k| a[k].min(b[k]) <= c[k].max(d[k]) && c[k].min(d[k]) <= a[k].max(b[k]));
            require(
                !(overlap && f[0].signum() != f[1].signum() && f[2].signum() != f[3].signum())
                    && !(overlap && f.contains(&0.)),
                "self-contacting polygon unsupported",
            )?;
        }
    }
    let mut ids = (0..n).collect::<Vec<_>>();
    let mut result = Vec::new();
    while ids.len() > 3 {
        let mut found = None;
        for i in 0..ids.len() {
            let a = ids[(i + ids.len() - 1) % ids.len()];
            let b = ids[i];
            let c = ids[(i + 1) % ids.len()];
            if cross2(p[a], p[b], p[c]) * sign <= 0. {
                continue;
            }
            let contains = ids
                .iter()
                .filter(|&&k| k != a && k != b && k != c)
                .any(|&k| {
                    cross2(p[a], p[b], p[k]) * sign >= 0.
                        && cross2(p[b], p[c], p[k]) * sign >= 0.
                        && cross2(p[c], p[a], p[k]) * sign >= 0.
                });
            if !contains {
                found = Some((i, [a, b, c]));
                break;
            }
        }
        let (i, t) = found.ok_or(Error::Degenerate("polygon triangulation unresolved"))?;
        result.push(t);
        ids.remove(i);
    }
    result.push([ids[0], ids[1], ids[2]]);
    Ok(result)
}
