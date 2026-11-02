//! Retained rational extrusion boundaries with explicit coedge incidence.
use crate::{Error, Result, check, curve::Curve, math::*, mesh::Mesh, surface::Surface};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    origin: Point,
    x: Point,
    y: Point,
    normal: Point,
}
impl Frame {
    pub fn xy() -> Self {
        Self {
            origin: [0.; 3],
            x: [1., 0., 0.],
            y: [0., 1., 0.],
            normal: [0., 0., 1.],
        }
    }
    pub fn new(origin: Point, x: Point, normal: Point) -> Result<Self> {
        check(finite(origin), "finite workplane origin required")?;
        let x = unit(x)?;
        let normal = unit(normal)?;
        check(
            dot(x, normal).abs() <= 1e-12,
            "orthogonal workplane axes required",
        )?;
        let y = unit(cross(normal, x))?;
        let x = cross(y, normal);
        Ok(Self {
            origin,
            x,
            y,
            normal,
        })
    }
    pub fn map(&self, p: Point) -> Point {
        add(
            self.origin,
            add(
                add(scale(self.x, p[0]), scale(self.y, p[1])),
                scale(self.normal, p[2]),
            ),
        )
    }
    fn affine(&self) -> Result<Affine> {
        Ok(Affine::new(
            std::array::from_fn(|i| [self.x[i], self.y[i], self.normal[i]]),
            self.origin,
        )?)
    }
}
#[derive(Clone, Debug, PartialEq)]
enum Shape {
    Polygon(Vec<[f64; 2]>),
    Circular {
        center: [f64; 2],
        outer: f64,
        inner: Option<f64>,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    shape: Shape,
    loops: Vec<Vec<Curve>>,
    area: f64,
    centroid: [f64; 2],
    bounds: [[f64; 2]; 2],
}
impl Profile {
    pub fn polygon(mut points: Vec<[f64; 2]>) -> Result<Self> {
        // The independent clean polygon validator handles crossings/contacts.
        Mesh::extrude(&points, 1.)?;
        let signed = (0..points.len())
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                a[0] * b[1] - a[1] * b[0]
            })
            .sum::<f64>()
            / 2.;
        if signed < 0. {
            points.reverse();
        }
        let area = signed.abs();
        let mut centroid = [0.; 2];
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            let cross = a[0] * b[1] - a[1] * b[0];
            for d in 0..2 {
                centroid[d] += (a[d] + b[d]) * cross / (6. * area);
            }
        }
        let mut bounds = [points[0]; 2];
        for p in &points {
            for d in 0..2 {
                bounds[0][d] = bounds[0][d].min(p[d]);
                bounds[1][d] = bounds[1][d].max(p[d]);
            }
        }
        let curves = (0..points.len())
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                Ok(Curve::line([a[0], a[1], 0.], [b[0], b[1], 0.])?)
            })
            .collect::<Result<_>>()?;
        check(
            area.is_finite() && centroid.iter().all(|x| x.is_finite()),
            "profile area/moment overflow",
        )?;
        Ok(Self {
            shape: Shape::Polygon(points),
            loops: vec![curves],
            area,
            centroid,
            bounds,
        })
    }
    pub fn disk(center: [f64; 2], radius: f64) -> Result<Self> {
        Self::circular(center, radius, None)
    }
    pub fn annulus(center: [f64; 2], outer: f64, inner: f64) -> Result<Self> {
        Self::circular(center, outer, Some(inner))
    }
    fn circular(center: [f64; 2], outer: f64, inner: Option<f64>) -> Result<Self> {
        check(
            center.iter().all(|x| x.is_finite())
                && outer.is_finite()
                && outer > 0.
                && inner.is_none_or(|r| r.is_finite() && r > 0. && r < outer),
            "invalid circular profile",
        )?;
        let make = |r: f64, sign: f64| {
            (0..4)
                .map(|i| {
                    Ok(Curve::ellipse_arc(
                        [center[0], center[1], 0.],
                        [r; 2],
                        sign * i as f64 * FRAC_PI_2,
                        sign * FRAC_PI_2,
                    )?)
                })
                .collect::<Result<Vec<_>>>()
        };
        let mut loops = vec![make(outer, 1.)?];
        if let Some(r) = inner {
            loops.push(make(r, -1.)?)
        }
        let area = PI * (outer - inner.unwrap_or(0.)) * (outer + inner.unwrap_or(0.));
        check(area.is_finite() && area > 0., "circular area overflow")?;
        let bounds = [
            [center[0] - outer, center[1] - outer],
            [center[0] + outer, center[1] + outer],
        ];
        check(
            bounds.iter().flatten().all(|x| x.is_finite()),
            "circular bounds overflow",
        )?;
        Ok(Self {
            shape: Shape::Circular {
                center,
                outer,
                inner,
            },
            loops,
            area,
            centroid: center,
            bounds,
        })
    }
    pub fn area(&self) -> f64 {
        self.area
    }
    pub fn boundary_loops(&self) -> &[Vec<Curve>] {
        &self.loops
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub point: Point,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub vertices: [usize; 2],
    pub curve: Curve,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeUse {
    pub edge: usize,
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub carrier: Surface,
    pub reversed: bool,
    pub loops: Vec<Vec<EdgeUse>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundaryTrack {
    pub profile_loop: usize,
    pub profile_edge: usize,
    pub bottom_edge: usize,
    pub top_edge: usize,
    pub side_face: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Audit {
    pub vertices: usize,
    pub edges: usize,
    pub faces: usize,
    pub max_endpoint_gap: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    faces: Vec<Face>,
    tracking: Vec<BoundaryTrack>,
    profile: Profile,
    frame: Frame,
    vector: Point,
    tolerance: f64,
}

fn plane_carrier(profile: &Profile, frame: &Frame, offset: Point) -> Result<Surface> {
    let [lo, hi] = profile.bounds;
    let controls = [lo[0], hi[0]]
        .iter()
        .map(|&x| {
            [lo[1], hi[1]]
                .iter()
                .map(|&y| add(frame.map([x, y, 0.]), offset))
                .collect()
        })
        .collect();
    Ok(Surface::new(
        [1, 1],
        [vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]],
        controls,
        vec![vec![1.; 2]; 2],
    )?)
}
impl Body {
    pub fn extrude(profile: Profile, frame: Frame, vector: Point, tolerance: f64) -> Result<Self> {
        check(
            finite(vector) && tolerance.is_finite() && tolerance > 0.,
            "finite extrusion and positive endpoint tolerance required",
        )?;
        let height = dot(vector, frame.normal);
        check(
            height.is_finite() && height.abs() > tolerance,
            "extrusion parallel to workplane or below tolerance",
        )?;
        let placement = frame.affine()?;
        let translation = Affine::translation(vector)?;
        let mut vertices = Vec::new();
        let mut edges = Vec::new();
        let mut faces = vec![
            Face {
                carrier: plane_carrier(&profile, &frame, [0.; 3])?,
                reversed: height > 0.,
                loops: vec![],
            },
            Face {
                carrier: plane_carrier(&profile, &frame, vector)?,
                reversed: height < 0.,
                loops: vec![],
            },
        ];
        let mut tracking = Vec::new();
        for (loop_id, curves) in profile.loops.iter().enumerate() {
            let world = curves
                .iter()
                .map(|c| c.transformed(&placement))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let n = world.len();
            let vertex_base = vertices.len();
            let edge_base = edges.len();
            for c in &world {
                vertices.push(Vertex {
                    point: c.evaluate(c.domain()[0])?,
                });
            }
            for i in 0..n {
                vertices.push(Vertex {
                    point: add(vertices[vertex_base + i].point, vector),
                });
            }
            for (i, c) in world.iter().enumerate() {
                edges.push(Edge {
                    vertices: [vertex_base + i, vertex_base + (i + 1) % n],
                    curve: c.clone(),
                });
            }
            for (i, c) in world.iter().enumerate() {
                edges.push(Edge {
                    vertices: [vertex_base + n + i, vertex_base + n + (i + 1) % n],
                    curve: c.transformed(&translation)?,
                });
            }
            for i in 0..n {
                let ids = [vertex_base + i, vertex_base + n + i];
                edges.push(Edge {
                    vertices: ids,
                    curve: Curve::line(vertices[ids[0]].point, vertices[ids[1]].point)?,
                });
            }
            faces[0].loops.push(
                (0..n)
                    .rev()
                    .map(|i| EdgeUse {
                        edge: edge_base + i,
                        reversed: true,
                    })
                    .collect(),
            );
            faces[1].loops.push(
                (0..n)
                    .map(|i| EdgeUse {
                        edge: edge_base + n + i,
                        reversed: false,
                    })
                    .collect(),
            );
            for (i, c) in world.iter().enumerate() {
                let side_face = faces.len();
                faces.push(Face {
                    carrier: Surface::extrude(c, vector)?,
                    reversed: height > 0.,
                    loops: vec![vec![
                        EdgeUse {
                            edge: edge_base + i,
                            reversed: false,
                        },
                        EdgeUse {
                            edge: edge_base + 2 * n + (i + 1) % n,
                            reversed: false,
                        },
                        EdgeUse {
                            edge: edge_base + n + i,
                            reversed: true,
                        },
                        EdgeUse {
                            edge: edge_base + 2 * n + i,
                            reversed: true,
                        },
                    ]],
                });
                tracking.push(BoundaryTrack {
                    profile_loop: loop_id,
                    profile_edge: i,
                    bottom_edge: edge_base + i,
                    top_edge: edge_base + n + i,
                    side_face,
                });
            }
        }
        if height < 0. {
            for f in &mut faces {
                for l in &mut f.loops {
                    l.reverse();
                    for edge_use in l {
                        edge_use.reversed = !edge_use.reversed;
                    }
                }
            }
        }
        let body = Self {
            vertices,
            edges,
            faces,
            tracking,
            profile,
            frame,
            vector,
            tolerance,
        };
        body.audit()?;
        Ok(body)
    }
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }
    pub fn tracking(&self) -> &[BoundaryTrack] {
        &self.tracking
    }
    pub fn volume(&self) -> f64 {
        self.profile.area * dot(self.vector, self.frame.normal).abs()
    }
    pub fn centroid(&self) -> Point {
        add(
            self.frame
                .map([self.profile.centroid[0], self.profile.centroid[1], 0.]),
            scale(self.vector, 0.5),
        )
    }
    /// Construction-domain audit: incidence and endpoint gaps. Not a general
    /// imported-B-rep geometric or self-intersection certificate.
    pub fn audit(&self) -> Result<Audit> {
        let mut uses = vec![Vec::new(); self.edges.len()];
        let mut max_gap: f64 = 0.;
        for e in &self.edges {
            for (end, t) in e.curve.domain().into_iter().enumerate() {
                let gap = norm(sub(
                    e.curve.evaluate(t)?,
                    self.vertices[e.vertices[end]].point,
                ));
                max_gap = max_gap.max(gap);
            }
            check(e.vertices[0] != e.vertices[1], "collapsed extrusion edge")?;
        }
        check(
            max_gap <= self.tolerance,
            "boundary endpoint gap exceeds tolerance",
        )?;
        for face in &self.faces {
            for wire in &face.loops {
                check(wire.len() >= 3, "face wire requires at least three edges")?;
                for (i, u) in wire.iter().enumerate() {
                    uses[u.edge].push(u.reversed);
                    let edge = &self.edges[u.edge];
                    let next = &wire[(i + 1) % wire.len()];
                    let next_edge = &self.edges[next.edge];
                    check(
                        edge.vertices[usize::from(!u.reversed)]
                            == next_edge.vertices[usize::from(next.reversed)],
                        "open face wire",
                    )?;
                }
            }
        }
        check(
            uses.iter().all(|u| u.len() == 2 && u[0] != u[1]),
            "nonmanifold extrusion edge incidence",
        )?;
        check(
            self.volume().is_finite() && self.volume() > 0. && finite(self.centroid()),
            "nonfinite extrusion measure",
        )?;
        Ok(Audit {
            vertices: self.vertices.len(),
            edges: self.edges.len(),
            faces: self.faces.len(),
            max_endpoint_gap: max_gap,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Faceting {
    pub mesh: Mesh,
    pub triangle_faces: Vec<usize>,
    pub boundary_chord_deviation: f64,
}
impl Body {
    /// Chord tolerance is an analytic circle sagitta estimate in real arithmetic,
    /// not a validated binary64 Hausdorff certificate. Source topology is retained.
    pub fn tessellate(&self, chord_tolerance: f64, max_segments: usize) -> Result<Faceting> {
        check(
            chord_tolerance.is_finite()
                && chord_tolerance > 0.
                && (4..=65536).contains(&max_segments),
            "invalid faceting settings",
        )?;
        if let Shape::Polygon(points) = &self.profile.shape {
            let local = Mesh::extrude(points, 1.)?;
            let affine = Affine::new(
                std::array::from_fn(|i| [self.frame.x[i], self.frame.y[i], self.vector[i]]),
                self.frame.origin,
            )?;
            let mesh = local.transformed(&affine)?;
            let caps = 2 * (points.len() - 2);
            let triangle_faces = (0..mesh.triangles().len())
                .map(|i| if i < caps { i % 2 } else { 2 + (i - caps) / 2 })
                .collect();
            return Ok(Faceting {
                mesh,
                triangle_faces,
                boundary_chord_deviation: 0.,
            });
        }
        let Shape::Circular {
            center,
            outer,
            inner,
        } = self.profile.shape
        else {
            unreachable!()
        };
        let angle = (1. - (chord_tolerance / outer).min(1.)).acos();
        if angle == 0. {
            return Err(Error::Budget(
                "chord tolerance below resolvable circle partition",
            ));
        }
        let raw = (PI / angle).ceil().max(4.);
        if !raw.is_finite() || raw > max_segments as f64 {
            return Err(Error::Budget("circle faceting segment budget"));
        }
        let n = (raw as usize).div_ceil(4) * 4;
        if n > max_segments {
            return Err(Error::Budget("quadrant-aligned faceting budget"));
        }
        let mut vertices = Vec::new();
        let ring = |r: f64| {
            (0..n)
                .map(|j| {
                    let a = TAU * j as f64 / n as f64;
                    self.frame
                        .map([center[0] + r * a.cos(), center[1] + r * a.sin(), 0.])
                })
                .collect::<Vec<_>>()
        };
        let outer_ring = ring(outer);
        vertices.extend(outer_ring.iter().copied());
        vertices.extend(outer_ring.iter().map(|p| add(*p, self.vector)));
        if let Some(r) = inner {
            let inner_ring = ring(r);
            vertices.extend(inner_ring.iter().copied());
            vertices.extend(inner_ring.iter().map(|p| add(*p, self.vector)));
        } else {
            vertices.push(self.frame.map([center[0], center[1], 0.]));
            vertices.push(add(vertices[2 * n], self.vector));
        }
        let mut triangles = Vec::new();
        let mut triangle_faces = Vec::new();
        let mut emit = |triangle: [usize; 3], face: usize| {
            triangles.push(triangle);
            triangle_faces.push(face);
        };
        for i in 0..n {
            let j = (i + 1) % n;
            let outer_face = 2 + i / (n / 4);
            emit([i, j, j + n], outer_face);
            emit([i, j + n, i + n], outer_face);
            if inner.is_some() {
                let a = 2 * n + i;
                let b = 2 * n + j;
                let inner_face = 6 + 3 - i / (n / 4);
                emit([a, b + n, b], inner_face);
                emit([a, a + n, b + n], inner_face);
                emit([i, a, b], 0);
                emit([i, b, j], 0);
                emit([i + n, j + n, b + n], 1);
                emit([i + n, b + n, a + n], 1);
            } else {
                emit([2 * n, j, i], 0);
                emit([2 * n + 1, i + n, j + n], 1);
            }
        }
        if dot(self.vector, self.frame.normal) < 0. {
            for t in &mut triangles {
                t.swap(1, 2);
            }
        }
        let mesh = Mesh::new(vertices, triangles)?;
        check(
            mesh.topology().is_closed_manifold(),
            "faceting failed incidence audit",
        )?;
        Ok(Faceting {
            mesh,
            triangle_faces,
            boundary_chord_deviation: outer * (1. - (PI / n as f64).cos()),
        })
    }
}
