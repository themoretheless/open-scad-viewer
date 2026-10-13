use super::*;

pub(super) fn size(value: f64, name: &str) -> Result<()> {
    if !value.is_finite() || !(1e-5..=1e6).contains(&value) {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            format!("{name} must be finite and in 0.00001..1000000 mm"),
        ));
    }
    Ok(())
}
pub(super) fn arc(radius: f64, z: f64, quadrant: usize) -> Curve {
    let [x, y] = QUADRANTS[quadrant];
    let [nx, ny] = QUADRANTS[(quadrant + 1) % 4];
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![radius * x, radius * y, z],
            vec![radius * (x + nx), radius * (y + ny), z],
            vec![radius * nx, radius * ny, z],
        ],
        weights: vec![1., WEIGHT, 1.],
        periodic: false,
    }
}
// Rational t -> ((1-t*t)/(1+t*t), 2*t/(1+t*t)). Unlike the
// symmetric sqrt(1/2) weight, these binary coefficients satisfy the circle
// identity exactly. Sphere trims and their equator edges share this traversal.
fn rational_quarter_arc(radius:f64,z:f64,quadrant:usize)->Curve{
    let mut curve=arc(radius,z,quadrant);
    curve.weights=vec![1.,1.,2.];
    curve
}
pub(super) fn arc_span(radius: f64, z: f64, start: f64, end: f64) -> Curve {
    let middle = (start + end) * 0.5;
    let weight = ((end - start) * 0.5).cos();
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![radius * start.cos(), radius * start.sin(), z],
            vec![
                radius * middle.cos() / weight,
                radius * middle.sin() / weight,
                z,
            ],
            vec![radius * end.cos(), radius * end.sin(), z],
        ],
        weights: vec![1., weight, 1.],
        periodic: false,
    }
}
pub(super) fn reversed(mut curve: Curve) -> Curve {
    let [a, b] = curve.domain();
    curve.knots = curve.knots.iter().rev().map(|k| a + b - k).collect();
    curve.control_points.reverse();
    curve.weights.reverse();
    curve
}
// Control-hull bounds define a common affine UV frame for planar caps.
pub(super) fn profile_control_bounds(wire: &[Curve]) -> ([f64; 2], [f64; 2]) {
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for curve in wire {
        for point in &curve.control_points {
            for axis in 0..2 {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }
    }
    (min, max)
}
pub(super) struct Builder {
    pub(super) model: Model,
    pub(super) edges: BTreeMap<(usize, usize), usize>,
    pub(super) faces: Vec<FaceUse>,
}
impl Builder {
    pub(super) fn new() -> Self {
        Self {
            model: Model(
                brep_topology::Model {
                    vertices: vec![],
                    edges: vec![],
                    loops: vec![],
                    faces: vec![],
                    shells: vec![],
                    bodies: vec![],
                    tolerance_mm: 1e-7,
                },
                TopologyIds::default(),
            ),
            edges: BTreeMap::new(),
            faces: vec![],
        }
    }
    fn ring(&mut self, radius: f64, z: f64) -> [usize; 4] {
        if radius == 0. {
            let id = self.model.vertices.len();
            self.model.vertices.push(Vertex { point: [0., 0., z] });
            return [id; 4];
        }
        std::array::from_fn(|i| {
            let id = self.model.vertices.len();
            self.model.vertices.push(Vertex {
                point: [radius * QUADRANTS[i][0], radius * QUADRANTS[i][1], z],
            });
            id
        })
    }
    pub(super) fn coedge(&mut self, a: usize, b: usize, curve: Curve, pcurve: Curve) -> Coedge {
        let key = (a.min(b), a.max(b));
        let edge = *self.edges.entry(key).or_insert_with(|| {
            let id = self.model.0.edges.len();
            self.model.0.edges.push(Edge {
                degenerate: a == b,
                vertices: [key.0, key.1],
                curve: if a > b { reversed(curve) } else { curve },
            });
            id
        });
        Coedge {
            edge,
            reversed: a > b,
            pcurve,
        }
    }
    pub(super) fn wire(&mut self, coedges: Vec<Coedge>) -> usize {
        let id = self.model.loops.len();
        self.model.loops.push(Loop { coedges });
        id
    }
    pub(super) fn face(&mut self, surface: Surface, outer: usize, holes: Vec<usize>, reverse: bool) {
        let face = self.model.faces.len();
        self.model.faces.push(Face {
            surface,
            outer,
            holes,
        });
        self.faces.push(FaceUse {
            face,
            reversed: reverse,
        });
    }
    fn band(&mut self, a: [usize; 4], b: [usize; 4], p: [f64; 2], q: [f64; 2], reverse: bool) {
        for i in 0..4 {
            let j = (i + 1) % 4;
            let ca = arc(p[0], p[1], i);
            let cb = arc(q[0], q[1], i);
            let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
            let corners = [a[i], a[j], b[j], b[i]];
            let curves = [
                ca.clone(),
                line(
                    self.model.vertices[a[j]].point.to_vec(),
                    self.model.vertices[b[j]].point.to_vec(),
                ),
                reversed(cb.clone()),
                line(
                    self.model.vertices[b[i]].point.to_vec(),
                    self.model.vertices[a[i]].point.to_vec(),
                ),
            ];
            let coedges = curves
                .into_iter()
                .enumerate()
                .map(|(k, curve)| {
                    self.coedge(
                        corners[k],
                        corners[(k + 1) % 4],
                        curve,
                        line(uv[k].to_vec(), uv[(k + 1) % 4].to_vec()),
                    )
                })
                .collect();
            let outer = self.wire(coedges);
            self.face(
                Surface {
                    degree_u: 2,
                    degree_v: 1,
                    knots_u: ca.knots.clone(),
                    knots_v: vec![0., 0., 1., 1.],
                    control_points: (0..3)
                        .map(|k| vec![ca.control_points[k].clone(), cb.control_points[k].clone()])
                        .collect(),
                    weights: vec![vec![1., 1.], vec![WEIGHT, WEIGHT], vec![1., 1.]],
                    periodic_u: false,
                    periodic_v: false,
                },
                outer,
                vec![],
                reverse,
            );
        }
    }
    pub(super) fn rectangular_patch(&mut self, surface: Surface, corners: [usize; 4], curves: [Curve; 4]) {
        let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let coedges = curves
            .into_iter()
            .enumerate()
            .map(|(k, curve)| {
                self.coedge(
                    corners[k],
                    corners[(k + 1) % 4],
                    curve,
                    line(uv[k].to_vec(), uv[(k + 1) % 4].to_vec()),
                )
            })
            .collect();
        let outer = self.wire(coedges);
        self.face(surface, outer, vec![], false);
    }
    pub(super) fn analytic_cap(
        &mut self,
        wire: &[Curve],
        vertices: &[usize],
        direction: [f64; 2],
        reverse: bool,
    ) {
        let (min, max) = profile_control_bounds(wire);
        let point = |p: &[f64]| vec![p[0] * direction[0], p[0] * direction[1], p[1]];
        let mut coedges = Vec::with_capacity(wire.len());
        for (i, source) in wire.iter().enumerate() {
            let mut curve = source.clone();
            curve.control_points = source.control_points.iter().map(|p| point(p)).collect();
            let mut pcurve = source.clone();
            for p in &mut pcurve.control_points {
                for k in 0..2 {
                    p[k] = (p[k] - min[k]) / (max[k] - min[k]);
                }
            }
            coedges.push(self.coedge(vertices[i], vertices[(i + 1) % wire.len()], curve, pcurve));
        }
        let outer = self.wire(coedges);
        self.face(
            Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: vec![0., 0., 1., 1.],
                control_points: vec![
                    vec![point(&min), point(&[min[0], max[1]])],
                    vec![point(&[max[0], min[1]]), point(&max)],
                ],
                weights: vec![vec![1., 1.], vec![1., 1.]],
                periodic_u: false,
                periodic_v: false,
            },
            outer,
            vec![],
            reverse,
        );
    }
    fn cap_wire(
        &mut self,
        ring: [usize; 4],
        radius: f64,
        z: f64,
        scale: f64,
        reverse: bool,
    ) -> usize {
        let mut coedges = vec![];
        for index in 0..4 {
            let i = if reverse { 3 - index } else { index };
            let j = (i + 1) % 4;
            let mut curve = arc(radius, z, i);
            if reverse {
                curve = reversed(curve);
            }
            let mut pcurve = curve.clone();
            pcurve.control_points = curve
                .control_points
                .iter()
                .map(|p| vec![0.5 + p[0] / (2. * scale), 0.5 + p[1] / (2. * scale)])
                .collect();
            let (a, b) = if reverse {
                (ring[j], ring[i])
            } else {
                (ring[i], ring[j])
            };
            coedges.push(self.coedge(a, b, curve, pcurve));
        }
        self.wire(coedges)
    }
    fn cap(
        &mut self,
        ring: [usize; 4],
        radius: f64,
        z: f64,
        hole: Option<([usize; 4], f64)>,
        reverse: bool,
    ) {
        let outer = self.cap_wire(ring, radius, z, radius, false);
        let holes = hole
            .map(|(ring, r)| vec![self.cap_wire(ring, r, z, radius, true)])
            .unwrap_or_default();
        self.face(
            Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: vec![0., 0., 1., 1.],
                control_points: vec![
                    vec![vec![-radius, -radius, z], vec![-radius, radius, z]],
                    vec![vec![radius, -radius, z], vec![radius, radius, z]],
                ],
                weights: vec![vec![1., 1.], vec![1., 1.]],
                periodic_u: false,
                periodic_v: false,
            },
            outer,
            holes,
            reverse,
        );
    }
    pub(super) fn finish(mut self) -> Result<Model> {
        self.model.shells.push(Shell {
            faces: self.faces,
            closed: true,
        });
        self.model.bodies.push(Body {
            outer_shell: 0,
            inner_shells: vec![],
        });
        self.model.rebuild_topology_ids();
        self.model.validate()?;
        Ok(self.model)
    }
}

/// Exact cylindrical solid, z=0..height. Display detail never changes the 6 faces.
pub fn cylinder(radius: f64, height: f64) -> Result<Model> {
    frustum(radius, radius, height)
}

/// Exact conical frustum, including a true apex when exactly one radius is zero.
/// A collapsed surface boundary explicitly references the one shared pole vertex.
pub fn frustum(bottom_radius: f64, top_radius: f64, height: f64) -> Result<Model> {
    if bottom_radius != 0. {
        size(bottom_radius, "Bottom radius")?;
    }
    if top_radius != 0. {
        size(top_radius, "Top radius")?;
    }
    if bottom_radius == 0. && top_radius == 0. {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "At least one cone radius must be positive",
        ));
    }
    size(height, "Height")?;
    let mut build = Builder::new();
    let a = build.ring(bottom_radius, 0.);
    let b = build.ring(top_radius, height);
    build.band(a, b, [bottom_radius, 0.], [top_radius, height], false);
    if bottom_radius > 0. {
        build.cap(a, bottom_radius, 0., None, true);
    }
    if top_radius > 0. {
        build.cap(b, top_radius, height, None, false);
    }
    build.finish()
}

/// Exact hollow cylinder with two annular trimmed planar faces and 8 curved patches.
pub fn tube(outer_radius: f64, inner_radius: f64, height: f64) -> Result<Model> {
    size(outer_radius, "Outer radius")?;
    size(inner_radius, "Inner radius")?;
    size(height, "Height")?;
    if outer_radius - inner_radius < 1e-5 {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Tube wall must be at least 0.00001 mm thick",
        ));
    }
    let mut build = Builder::new();
    let a = build.ring(outer_radius, 0.);
    let b = build.ring(outer_radius, height);
    let c = build.ring(inner_radius, 0.);
    let d = build.ring(inner_radius, height);
    build.band(a, b, [outer_radius, 0.], [outer_radius, height], false);
    build.band(c, d, [inner_radius, 0.], [inner_radius, height], true);
    build.cap(a, outer_radius, 0., Some((c, inner_radius)), true);
    build.cap(b, outer_radius, height, Some((d, inner_radius)), false);
    build.finish()
}

/// Exact sphere centered at the origin, represented by eight regular trimmed
/// stereographic patches. Its poles are ordinary vertices: no zero-length
/// edges, collapsed surface boundaries, or tolerance-sized artificial caps.
pub fn sphere(radius: f64) -> Result<Model> {
    size(radius, "Sphere radius")?;
    let mut build = Builder::new();
    let equator = build.ring(radius, 0.);
    for sign in [1., -1.] {
        let pole = build.model.vertices.len();
        build.model.vertices.push(Vertex {
            point: [0., 0., sign * radius],
        });
        for quadrant in 0..4 {
            let next = (quadrant + 1) % 4;
            let a = QUADRANTS[quadrant];
            let b = QUADRANTS[next];
            // Bernstein coefficients of t and t^2 in degree 2. Homogeneous
            // numerator is (2u, 2v, +/- (1-u^2-v^2)); denominator 1+u^2+v^2.
            let linear = [0., 0.5, 1.];
            let square = [0., 0., 1.];
            let weights: Vec<Vec<f64>> = (0..3)
                .map(|i| (0..3).map(|j| 1. + square[i] + square[j]).collect())
                .collect();
            let surface = Surface {
                degree_u: 2,
                degree_v: 2,
                knots_u: vec![0., 0., 0., 1., 1., 1.],
                knots_v: vec![0., 0., 0., 1., 1., 1.],
                control_points: (0..3)
                    .map(|i| {
                        (0..3)
                            .map(|j| {
                                let w = weights[i][j];
                                vec![
                                    2. * radius * (linear[i] * a[0] + linear[j] * b[0]) / w,
                                    2. * radius * (linear[i] * a[1] + linear[j] * b[1]) / w,
                                    sign * radius * (1. - square[i] - square[j]) / w,
                                ]
                            })
                            .collect()
                    })
                    .collect(),
                weights,
                periodic_u: false,
                periodic_v: false,
            };
            let meridian = |direction: [f64; 2]| Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![
                    vec![0., 0., sign * radius],
                    vec![radius * direction[0], radius * direction[1], sign * radius],
                    vec![radius * direction[0], radius * direction[1], 0.],
                ],
                weights: vec![1., 1., 2.],
                periodic: false,
            };
            let mut circular_trim = rational_quarter_arc(1., 0., 0);
            circular_trim.control_points.iter_mut().for_each(|p| {
                p.pop();
            });
            let coedges = vec![
                build.coedge(
                    pole,
                    equator[quadrant],
                    meridian(a),
                    line(vec![0., 0.], vec![1., 0.]),
                ),
                build.coedge(
                    equator[quadrant],
                    equator[next],
                    rational_quarter_arc(radius, 0., quadrant),
                    circular_trim,
                ),
                build.coedge(
                    equator[next],
                    pole,
                    reversed(meridian(b)),
                    line(vec![0., 1.], vec![0., 0.]),
                ),
            ];
            let outer = build.wire(coedges);
            build.face(surface, outer, vec![], sign < 0.);
        }
    }
    build.finish()
}

/// Exact ring torus, centered at the origin with its axis along Z.
/// The 16 regular biquadratic patches share 32 circular edges; horn and spindle
/// tori are excluded because their boundaries are not embedded regular solids.
pub fn torus(major_radius: f64, minor_radius: f64) -> Result<Model> {
    size(major_radius, "Major radius")?;
    size(minor_radius, "Minor radius")?;
    if major_radius - minor_radius < 1e-5 || major_radius + minor_radius > 1e6 {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Ring torus requires major radius minus minor radius at least 0.00001 mm and total radius at most 1000000 mm",
        ));
    }
    let mut wire = sketch::circle_wire(minor_radius)?;
    for curve in &mut wire {
        for point in &mut curve.control_points {
            point[0] += major_radius;
        }
    }
    revolve_wire(&wire, 1e-7)
}

/// Full revolution of a material-left region, including cavities and islands.
pub fn revolve_region(loops: &[Vec<Curve>], tolerance: f64) -> Result<Model> {
    revolve_region_angle(loops, tolerance, 360.)
}
