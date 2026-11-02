//! Convex planar boundary clipping in world coordinates. No mesh voxelization,
//! axis-alignment restriction, general nonconvex Boolean or curved clipping.
use crate::{Error, Result, check, math::*, mesh::Mesh};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    normal: Point,
    offset: f64,
}
impl Plane {
    /// The retained side is normal.dot(point) <= offset. Normal need not be unit.
    pub fn new(normal: Point, offset: f64) -> Result<Self> {
        let length = norm(normal);
        let normal = unit(normal)?;
        check(
            offset.is_finite() && (offset / length).is_finite(),
            "finite plane offset required",
        )?;
        Ok(Self {
            normal,
            offset: offset / length,
        })
    }
    pub fn normal(&self) -> Point {
        self.normal
    }
    pub fn offset(&self) -> f64 {
        self.offset
    }
    pub fn signed_distance(&self, p: Point) -> f64 {
        dot(self.normal, p) - self.offset
    }
    pub fn flipped(&self) -> Self {
        Self {
            normal: scale(self.normal, -1.),
            offset: -self.offset,
        }
    }
    fn project(&self, p: Point) -> Point {
        sub(p, scale(self.normal, self.signed_distance(p)))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Authored(usize),
    ClipPlane(usize),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub points: Vec<Point>,
    pub origin: Origin,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Convex {
    faces: Vec<Face>,
    planes: Vec<Plane>,
    tolerance: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Intersection {
    pub body: Option<Convex>,
    pub planes_processed: usize,
    /// All retained clipping-origin faces, including those from earlier cuts.
    pub retained_clip_faces: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Partition {
    pub outside: Vec<Convex>,
    pub overlap: Option<Convex>,
}

fn center(points: &[Point]) -> Point {
    let base = points[0];
    add(
        base,
        scale(
            points.iter().map(|p| sub(*p, base)).fold([0.; 3], add),
            1. / points.len() as f64,
        ),
    )
}
fn support(points: &[Point], tol: f64) -> Result<Plane> {
    check(points.len() >= 3, "face requires three vertices")?;
    let origin = points[0];
    for i in 1..points.len() - 1 {
        let n = cross(sub(points[i], origin), sub(points[i + 1], origin));
        if norm(n) > tol * tol {
            let normal = unit(n)?;
            return Plane::new(normal, dot(normal, origin));
        }
    }
    Err(Error::Unresolved("face area below tolerance"))
}
fn dedup_ring(points: Vec<Point>, tol: f64) -> Vec<Point> {
    let mut result = Vec::new();
    for p in points {
        if result.last().is_none_or(|q| norm(sub(p, *q)) > tol) {
            result.push(p);
        }
    }
    if result.len() > 1 && norm(sub(result[0], *result.last().unwrap())) <= tol {
        result.pop();
    }
    result
}
fn vertex(points: &mut Vec<Point>, p: Point, tol: f64) -> Result<usize> {
    let matches = points
        .iter()
        .enumerate()
        .filter(|(_, q)| norm(sub(p, **q)) <= tol)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    check(
        matches.len() <= 1,
        "ambiguous tolerance vertex identification",
    )?;
    if let Some(i) = matches.first() {
        Ok(*i)
    } else {
        let i = points.len();
        points.push(p);
        Ok(i)
    }
}
impl Convex {
    /// Outward CCW polygons. Validation checks convex halfspaces and indexed
    /// manifold incidence after bounded endpoint identification.
    pub fn new(polygons: Vec<Vec<Point>>, tolerance: f64) -> Result<Self> {
        let faces = polygons
            .into_iter()
            .enumerate()
            .map(|(i, points)| Face {
                points,
                origin: Origin::Authored(i),
            })
            .collect();
        Self::from_faces(faces, tolerance)
    }
    fn from_faces(faces: Vec<Face>, tolerance: f64) -> Result<Self> {
        check(
            tolerance.is_finite() && tolerance > 0.,
            "positive finite clipping tolerance required",
        )?;
        check(
            (4..=256).contains(&faces.len())
                && faces.iter().all(|f| (3..=512).contains(&f.points.len()))
                && faces.iter().map(|f| f.points.len()).sum::<usize>() <= 16384,
            "convex face budget",
        )?;
        check(
            faces.iter().flat_map(|f| &f.points).copied().all(finite),
            "finite convex vertices required",
        )?;
        let planes = faces
            .iter()
            .map(|f| support(&f.points, tolerance))
            .collect::<Result<Vec<_>>>()?;
        for (face, plane) in faces.iter().zip(&planes) {
            check(
                face.points
                    .iter()
                    .all(|p| plane.signed_distance(*p).abs() <= tolerance),
                "nonplanar face",
            )?;
            for i in 0..face.points.len() {
                let a = face.points[i];
                let b = face.points[(i + 1) % face.points.len()];
                let tangent = sub(b, a);
                let length = norm(tangent);
                check(length > tolerance, "face edge below tolerance")?;
                let inward = cross(plane.normal, tangent);
                check(
                    face.points
                        .iter()
                        .all(|p| dot(inward, sub(*p, a)) >= -tolerance * length),
                    "face polygon is not convex CCW",
                )?;
            }
        }
        check(
            planes.iter().all(|plane| {
                faces
                    .iter()
                    .flat_map(|f| &f.points)
                    .all(|p| plane.signed_distance(*p) <= tolerance)
            }),
            "faces do not define a convex outward boundary",
        )?;
        let value = Self {
            faces,
            planes,
            tolerance,
        };
        let mesh = value.mesh()?;
        check(
            mesh.topology().is_closed_manifold(),
            "clipped boundary is not manifold",
        )?;
        check(
            mesh.signed_volume().is_finite() && mesh.signed_volume() > tolerance.powi(3),
            "convex volume below resolution",
        )?;
        Ok(value)
    }
    pub fn cuboid(bounds: Bounds, pose: &Affine, tolerance: f64) -> Result<Self> {
        Bounds::new(bounds.min, bounds.max)?;
        let a = bounds.min;
        let b = bounds.max;
        let points = [
            [a[0], a[1], a[2]],
            [b[0], a[1], a[2]],
            [b[0], b[1], a[2]],
            [a[0], b[1], a[2]],
            [a[0], a[1], b[2]],
            [b[0], a[1], b[2]],
            [b[0], b[1], b[2]],
            [a[0], b[1], b[2]],
        ]
        .map(|p| pose.apply(p));
        let faces = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ]
        .map(|f| {
            let mut polygon = f.map(|i| points[i]).to_vec();
            if pose.determinant() < 0. {
                polygon.reverse();
            }
            polygon
        })
        .to_vec();
        Self::new(faces, tolerance)
    }
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }
    /// A simple convex polygon extruded along a nonparallel vector in a workplane.
    /// Nonconvex inputs fail the same support-plane validation as imported faces.
    pub fn prism(
        profile: &[[f64; 2]],
        frame: crate::brep::Frame,
        vector: Point,
        tolerance: f64,
    ) -> Result<Self> {
        let validation = crate::brep::Profile::polygon(profile.to_vec())?;
        let body = crate::brep::Body::extrude(validation, frame, vector, tolerance)?;
        let points = body.vertices().iter().map(|v| v.point).collect::<Vec<_>>();
        let polygons = body
            .faces()
            .iter()
            .map(|face| {
                face.loops[0]
                    .iter()
                    .map(|u| {
                        let e = &body.edges()[u.edge];
                        points[e.vertices[usize::from(u.reversed)]]
                    })
                    .collect()
            })
            .collect();
        Self::new(polygons, tolerance)
    }
    pub fn planes(&self) -> &[Plane] {
        &self.planes
    }
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }
    pub fn contains(&self, p: Point) -> bool {
        finite(p)
            && self
                .planes
                .iter()
                .all(|plane| plane.signed_distance(p) <= self.tolerance)
    }
    /// A separate face-center fan preserves collinear cap boundary vertices,
    /// avoiding T-junctions caused by discarding cut-edge subdivision points.
    pub fn mesh(&self) -> Result<Mesh> {
        let mut points = Vec::new();
        let mut triangles = Vec::new();
        for f in &self.faces {
            let ids = f
                .points
                .iter()
                .map(|p| vertex(&mut points, *p, self.tolerance))
                .collect::<Result<Vec<_>>>()?;
            let c = vertex(&mut points, center(&f.points), self.tolerance)?;
            for i in 0..ids.len() {
                triangles.push([c, ids[i], ids[(i + 1) % ids.len()]]);
            }
        }
        Ok(Mesh::new(points, triangles)?)
    }
    pub fn volume(&self) -> Result<f64> {
        Ok(self.mesh()?.signed_volume())
    }
    pub fn mass_properties(&self) -> Result<(f64, Point)> {
        Ok(self.mesh()?.mass_properties()?)
    }
    pub fn transformed(&self, pose: &Affine) -> Result<Self> {
        let mut faces = self.faces.clone();
        for f in &mut faces {
            for p in &mut f.points {
                *p = pose.apply(*p)
            }
            if pose.determinant() < 0. {
                f.points.reverse();
            }
        }
        Self::from_faces(faces, self.tolerance)
    }
    /// Tolerance-band vertices are projected only when a true cut occurs.
    /// This is floating-point clipping with explicit resolution, not exact predicates.
    pub fn clip(&self, plane: Plane, tag: usize) -> Result<Option<Self>> {
        let distances = self
            .faces
            .iter()
            .flat_map(|f| &f.points)
            .map(|p| plane.signed_distance(*p))
            .collect::<Vec<_>>();
        check(
            distances.iter().all(|d| d.is_finite()),
            "plane classification overflow",
        )?;
        let inside = distances.iter().any(|d| *d < -self.tolerance);
        let outside = distances.iter().any(|d| *d > self.tolerance);
        if !inside {
            return Ok(None);
        }
        if !outside {
            return Ok(Some(self.clone()));
        }
        let mut faces = Vec::new();
        let mut cap = Vec::new();
        for face in &self.faces {
            let mut result = Vec::new();
            for i in 0..face.points.len() {
                let a = face.points[i];
                let b = face.points[(i + 1) % face.points.len()];
                let da = plane.signed_distance(a);
                let db = plane.signed_distance(b);
                let ain = da <= self.tolerance;
                let bin = db <= self.tolerance;
                if ain != bin {
                    let denominator = da - db;
                    check(
                        denominator.is_finite() && denominator != 0.,
                        "cut denominator unresolved",
                    )?;
                    let t = (da / denominator).clamp(0., 1.);
                    let p = plane.project(lerp(a, b, t));
                    result.push(p);
                    vertex(&mut cap, p, self.tolerance)?;
                }
                if bin {
                    let p = if db.abs() <= self.tolerance {
                        let p = plane.project(b);
                        vertex(&mut cap, p, self.tolerance)?;
                        p
                    } else {
                        b
                    };
                    result.push(p);
                }
            }
            let result = dedup_ring(result, self.tolerance);
            if result.len() >= 3 && support(&result, self.tolerance).is_ok() {
                faces.push(Face {
                    points: result,
                    origin: face.origin,
                });
            }
        }
        if cap.len() < 3 {
            return Err(Error::Unresolved("cut did not recover a cap polygon"));
        }
        let origin = center(&cap);
        let (u, v) = frame(plane.normal)?;
        cap.sort_by(|a, b| {
            let a = sub(*a, origin);
            let b = sub(*b, origin);
            dot(a, v)
                .atan2(dot(a, u))
                .total_cmp(&dot(b, v).atan2(dot(b, u)))
        });
        faces.push(Face {
            points: cap,
            origin: Origin::ClipPlane(tag),
        });
        Ok(Some(Self::from_faces(faces, self.tolerance)?))
    }
    /// Halfspace intersection. Face/edge/point-only contacts regularize to empty.
    pub fn intersection(&self, other: &Self) -> Result<Intersection> {
        check(
            self.tolerance == other.tolerance,
            "operands require the same geometric tolerance",
        )?;
        let mut body = Some(self.clone());
        let mut processed = 0;
        for (i, &plane) in other.planes.iter().enumerate() {
            let Some(current) = body else { break };
            body = current.clip(plane, i)?;
            processed += 1;
        }
        let caps = body.as_ref().map_or(0, |b| {
            b.faces
                .iter()
                .filter(|f| matches!(f.origin, Origin::ClipPlane(_)))
                .count()
        });
        Ok(Intersection {
            body,
            planes_processed: processed,
            retained_clip_faces: caps,
        })
    }
    /// Disjoint-interior convex decomposition of self minus other. Returned
    /// pieces retain internal interfaces; they are not one sewn Boolean shell.
    pub fn subtract_partition(&self, other: &Self) -> Result<Partition> {
        check(
            self.tolerance == other.tolerance,
            "operands require the same geometric tolerance",
        )?;
        let mut remainder = Some(self.clone());
        let mut outside = Vec::new();
        for (i, &plane) in other.planes.iter().enumerate() {
            let Some(current) = remainder else { break };
            if let Some(piece) = current.clip(plane.flipped(), i)? {
                outside.push(piece);
            }
            remainder = current.clip(plane, i)?;
        }
        Ok(Partition {
            outside,
            overlap: remainder,
        })
    }
}
