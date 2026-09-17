//! Exact rational surfaces of revolution. Quarter patches avoid periodic seams
//! and preserve ordinary manifold incidence without collapsed pole edges.
use super::*;
mod loft;
pub use loft::ruled_loft;
pub(crate) use loft::piecewise_ruled_loft;

const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
const WEIGHT: f64 = std::f64::consts::FRAC_1_SQRT_2;

fn size(value: f64, name: &str) -> Result<()> {
    if !value.is_finite() || !(1e-5..=1e6).contains(&value) {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            format!("{name} must be finite and in 0.00001..1000000 mm"),
        ));
    }
    Ok(())
}
fn arc(radius: f64, z: f64, quadrant: usize) -> Curve {
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
fn arc_span(radius: f64, z: f64, start: f64, end: f64) -> Curve {
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
fn reversed(mut curve: Curve) -> Curve {
    curve.control_points.reverse();
    curve.weights.reverse();
    curve
}
// Control-hull bounds define a common affine UV frame for planar caps.
fn profile_control_bounds(wire: &[Curve]) -> ([f64; 2], [f64; 2]) {
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
struct Builder {
    model: Model,
    edges: BTreeMap<(usize, usize), usize>,
    faces: Vec<FaceUse>,
}
impl Builder {
    fn new() -> Self {
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
    fn coedge(&mut self, a: usize, b: usize, curve: Curve, pcurve: Curve) -> Coedge {
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
    fn wire(&mut self, coedges: Vec<Coedge>) -> usize {
        let id = self.model.loops.len();
        self.model.loops.push(Loop { coedges });
        id
    }
    fn face(&mut self, surface: Surface, outer: usize, holes: Vec<usize>, reverse: bool) {
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
    fn rectangular_patch(&mut self, surface: Surface, corners: [usize; 4], curves: [Curve; 4]) {
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
    fn analytic_cap(
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
    fn finish(mut self) -> Result<Model> {
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
            let mut circular_trim = arc(1., 0., 0);
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
                    arc(radius, 0., quadrant),
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

/// Region revolution with cavity shells for full turns and holed caps for partial turns.
pub fn revolve_region_angle(loops: &[Vec<Curve>], tolerance: f64, angle: f64) -> Result<Model> {
    if !angle.is_finite() || !(1e-5..=360.).contains(&angle.abs()) {
        return Err(Error::new(
            "BREP_INVALID_ANGLE",
            "Revolve angle magnitude must be in 0.00001..360 degrees",
        ));
    }
    let full = angle.abs() == 360.;
    let spans = loops.iter().map(Vec::len).sum::<usize>();
    let components = planar_trim::components(loops, tolerance)?;
    let segments = (angle.abs() / 90.).ceil() as usize;
    if spans > 64 || spans * segments + if full { 0 } else { 2 * components.len() } > 256 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Region revolution exceeds span or face budget",
        ));
    }
    let mut result = Model::empty(tolerance)?;
    for (outer, holes) in components {
        let mut shell_ids: Vec<usize> = Vec::new();
        let mut cap_ids = [0; 2];
        let (outer_min, outer_max) = profile_control_bounds(&loops[outer]);
        for (i, index) in std::iter::once(outer).chain(holes).enumerate() {
            let wire = if i == 0 {
                loops[index].clone()
            } else {
                loops[index]
                    .iter()
                    .rev()
                    .map(Curve::reverse)
                    .collect::<Result<Vec<_>>>()?
            };
            let model = revolve_wire_angle(&wire, tolerance, angle)?;
            let (v, e, l, f) = (
                result.vertices.len(),
                result.edges.len(),
                result.loops.len(),
                result.faces.len(),
            );
            let mut source = model.0;
            if !full {
                let cap_start = source.faces.len() - 2;
                if i == 0 {
                    cap_ids = [f + cap_start, f + cap_start + 1];
                } else {
                    let (inner_min, inner_max) = profile_control_bounds(&wire);
                    for k in 0..2 {
                        let loop_id = source.faces[cap_start + k].outer;
                        let cap = &mut source.loops[loop_id];
                        cap.coedges.reverse();
                        for coedge in &mut cap.coedges {
                            coedge.reversed = !coedge.reversed;
                            coedge.pcurve = coedge.pcurve.reverse()?;
                            for p in &mut coedge.pcurve.control_points {
                                for axis in 0..2 {
                                    p[axis] = (inner_min[axis]
                                        + p[axis] * (inner_max[axis] - inner_min[axis])
                                        - outer_min[axis])
                                        / (outer_max[axis] - outer_min[axis]);
                                }
                            }
                        }
                        result.faces[cap_ids[k]].holes.push(l + loop_id);
                    }
                    source.faces.truncate(cap_start);
                    for shell in &mut source.shells {
                        shell.faces.retain(|use_| use_.face < cap_start);
                    }
                }
            }
            result.vertices.extend(source.vertices);
            result
                .edges
                .extend(source.edges.into_iter().map(|mut edge| {
                    for id in &mut edge.vertices {
                        *id += v;
                    }
                    edge
                }));
            result
                .loops
                .extend(source.loops.into_iter().map(|mut wire| {
                    for coedge in &mut wire.coedges {
                        coedge.edge += e;
                    }
                    wire
                }));
            result
                .faces
                .extend(source.faces.into_iter().map(|mut face| {
                    face.outer += l;
                    for id in &mut face.holes {
                        *id += l;
                    }
                    face
                }));
            for mut shell in source.shells {
                for use_ in &mut shell.faces {
                    use_.face += f;
                    if i != 0 {
                        use_.reversed = !use_.reversed;
                    }
                }
                if !full && i != 0 {
                    result.shells[shell_ids[0]].faces.extend(shell.faces);
                } else {
                    shell_ids.push(result.shells.len());
                    result.shells.push(shell);
                }
            }
        }
        result.bodies.push(Body {
            outer_shell: shell_ids[0],
            inner_shells: shell_ids[1..].to_vec(),
        });
    }
    result.rebuild_topology_ids();
    result.validate()?;
    Ok(result)
}

/// Full revolution of one material-left analytic wire in the radial half-plane.
/// Retains line/circular Bezier definitions as rational tensor-product patches.
/// Axis endpoints share pole vertices; multi-wire regions require shell grouping.
pub fn revolve_wire(wire: &[Curve], tolerance: f64) -> Result<Model> {
    revolve_wire_angle(wire, tolerance, 360.)
}

/// Signed partial analytic revolution, capped by retained planar trim curves.
pub fn revolve_wire_angle(wire: &[Curve], tolerance: f64, angle_degrees: f64) -> Result<Model> {
    if !angle_degrees.is_finite() || !(1e-5..=360.).contains(&angle_degrees.abs()) {
        return Err(Error::new(
            "BREP_INVALID_ANGLE",
            "Revolve angle magnitude must be in 0.00001..360 degrees",
        ));
    }
    let full = angle_degrees.abs() == 360.;
    let segments = if full {
        4
    } else {
        (angle_degrees.abs() / 90.).ceil() as usize
    };
    planar_trim::validate(&[wire.to_vec()], tolerance)?;
    if wire.len() > 64 || wire.len() < 3 || wire.len() * segments + if full { 0 } else { 2 } > 256 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Revolution requires 3..64 spans",
        ));
    }
    let mut spans = Vec::with_capacity(wire.len());
    for source in wire {
        let n = source.degree + 1;
        let domain = source.domain();
        if source.periodic
            || source.control_points.len() != n
            || source.knots.len() != 2 * n
            || source.knots[..n].iter().any(|k| *k != domain[0])
            || source.knots[n..].iter().any(|k| *k != domain[1])
        {
            return Err(Error::new(
                "BREP_UNSUPPORTED",
                "Revolution requires clamped single-span curves",
            ));
        }
        for p in &source.control_points {
            if p[0] != 0. {
                size(p[0], "Revolution radial control point")?;
            }
            if p[1].abs() > 1e6 {
                return Err(Error::new(
                    "BREP_INVALID_SIZE",
                    "Revolution height exceeds coordinate bounds",
                ));
            }
        }
        let mut curve = source.clone();
        curve.knots[..n].fill(0.);
        curve.knots[n..].fill(1.);
        spans.push(curve);
    }
    for (i, c) in spans.iter().enumerate() {
        let end = c.control_points.last().unwrap();
        let start = &spans[(i + 1) % spans.len()].control_points[0];
        // Keep both retained curves unchanged. The shared topological vertex
        // uses the next span's start; Model::validate checks geometric agreement.
        if (end[0] - start[0]).hypot(end[1] - start[1]) > tolerance {
            return Err(Error::new(
                "BREP_UNSUPPORTED",
                "Revolution endpoint gap exceeds tolerance",
            ));
        }
    }
    let mut build = Builder::new();
    build.model.tolerance_mm = tolerance;
    let angles: Vec<_> = (0..=segments)
        .map(|i| angle_degrees.to_radians() * i as f64 / segments as f64)
        .collect();
    let directions: Vec<_> = if full {
        QUADRANTS.to_vec()
    } else {
        angles.iter().map(|a| [a.cos(), a.sin()]).collect()
    };
    let on_axis: Vec<_> = spans
        .iter()
        .map(|c| c.control_points.iter().all(|p| p[0] == 0.))
        .collect();
    let rings: Vec<Vec<usize>> = spans
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let p = &c.control_points[0];
            if full && on_axis[i] && on_axis[(i + spans.len() - 1) % spans.len()] {
                return Vec::new(); // Interior point of an axis chain has no boundary face.
            }
            if p[0] == 0. {
                let id = build.model.vertices.len();
                build.model.vertices.push(Vertex {
                    point: [0., 0., p[1]],
                });
                return vec![id; directions.len()];
            }
            directions
                .iter()
                .map(|d| {
                    let id = build.model.vertices.len();
                    build.model.vertices.push(Vertex {
                        point: [p[0] * d[0], p[0] * d[1], p[1]],
                    });
                    id
                })
                .collect()
        })
        .collect();
    for (j, section) in spans.iter().enumerate() {
        if on_axis[j] {
            continue;
        }
        let next = (j + 1) % spans.len();
        let start = &section.control_points[0];
        let end = section.control_points.last().unwrap();
        let meridian = |quadrant: usize| {
            let mut c = section.clone();
            c.control_points = c
                .control_points
                .iter()
                .map(|p| {
                    vec![
                        p[0] * directions[quadrant][0],
                        p[0] * directions[quadrant][1],
                        p[1],
                    ]
                })
                .collect();
            c
        };
        for i in 0..segments {
            let next_i = if full { (i + 1) % 4 } else { i + 1 };
            let circular = |r, z| {
                if full {
                    arc(r, z, i)
                } else {
                    arc_span(r, z, angles[i], angles[i + 1])
                }
            };
            let rotation = circular(1., 0.);
            let surface = Surface {
                degree_u: 2,
                degree_v: section.degree,
                knots_u: rotation.knots.clone(),
                knots_v: section.knots.clone(),
                control_points: rotation
                    .control_points
                    .iter()
                    .map(|a| {
                        section
                            .control_points
                            .iter()
                            .map(|b| vec![b[0] * a[0], b[0] * a[1], b[1]])
                            .collect()
                    })
                    .collect(),
                weights: rotation
                    .weights
                    .iter()
                    .map(|a| section.weights.iter().map(|b| a * b).collect())
                    .collect(),
                periodic_u: false,
                periodic_v: false,
            };
            build.rectangular_patch(
                surface,
                [
                    rings[j][i],
                    rings[j][next_i],
                    rings[next][next_i],
                    rings[next][i],
                ],
                [
                    circular(start[0], start[1]),
                    meridian(next_i),
                    reversed(circular(end[0], end[1])),
                    reversed(meridian(i)),
                ],
            );
            build.faces.last_mut().unwrap().reversed = !full && angle_degrees < 0.;
        }
    }
    if !full {
        build.analytic_cap(
            &spans,
            &rings.iter().map(|r| r[0]).collect::<Vec<_>>(),
            directions[0],
            angle_degrees < 0.,
        );
        build.analytic_cap(
            &spans,
            &rings.iter().map(|r| r[segments]).collect::<Vec<_>>(),
            directions[segments],
            angle_degrees > 0.,
        );
    }
    build.finish()
}

/// Full revolution of a simple CCW (radius,z) profile in the nonnegative half-plane.
/// Every off-axis line span becomes four exact rational ruled patches (at most
/// 256 faces). Axis spans close the volume without artificial internal faces;
/// axis endpoints become explicitly certified poles.
pub fn revolve(profile: &[[f64; 2]]) -> Result<Model> {
    revolve_angle(profile, 360.)
}

/// Signed full or partial polygonal revolution through the analytic constructor.
pub fn revolve_angle(profile: &[[f64; 2]], angle_degrees: f64) -> Result<Model> {
    operations::validate_extrusion_ring(profile, true, "Revolve profile")?;
    let wire: Vec<_> = (0..profile.len())
        .map(|i| {
            line(
                profile[i].to_vec(),
                profile[(i + 1) % profile.len()].to_vec(),
            )
        })
        .collect();
    revolve_wire_angle(&wire, 1e-7, angle_degrees)
}

#[cfg(test)]
mod tests {
    #[test]
    fn partial_caps_reparameterize_multiple_offset_holes() {
        let mut loops = Vec::new();
        for (radius, x, y) in [(1., 4., 2.), (0.2, 4.2, 2.1), (0.1, 3.6, 1.8)] {
            let mut wire = crate::sketch::circle_wire(radius).unwrap();
            for c in &mut wire {
                for p in &mut c.control_points {
                    p[0] += x;
                    p[1] += y;
                }
            }
            loops.push(wire);
        }
        let loops = crate::planar_trim::orient_even_odd(&loops, 1e-7).unwrap();
        let model = super::revolve_region_angle(&loops, 1e-7, -120.).unwrap();
        assert_eq!(model.faces.iter().filter(|f| f.holes.len() == 2).count(), 2);
        let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
        let expected = 2. * std::f64::consts::PI.powi(2) * (4. - 4.2 * 0.04 - 3.6 * 0.01) / 3.;
        assert!((mass.signed_volume_mm3 / expected - 1.).abs() < 1e-6);
    }
    #[test]
    fn partial_region_revolution_sews_holes_into_common_caps() {
        let mut loops = Vec::new();
        for radius in [1., 0.5] {
            let mut wire = crate::sketch::circle_wire(radius).unwrap();
            for c in &mut wire {
                for p in &mut c.control_points {
                    p[0] += 3.;
                }
            }
            loops.push(wire);
        }
        let loops = crate::planar_trim::orient_even_odd(&loops, 1e-7).unwrap();
        for angle in [90., -90., 210., -210.] {
            let model = super::revolve_region_angle(&loops, 1e-7, angle).unwrap();
            assert_eq!(model.bodies.len(), 1);
            assert_eq!(model.shells.len(), 1);
            assert!(model.bodies[0].inner_shells.is_empty());
            assert_eq!(model.faces.iter().filter(|f| f.holes.len() == 1).count(), 2);
            let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
            let expected = 4.5 * std::f64::consts::PI.powi(2) * angle.abs() / 360.;
            assert!((mass.signed_volume_mm3 / expected - 1.).abs() < 1e-6);
        }
    }
    #[test]
    fn region_revolution_owns_cavity_shell_and_nested_island() {
        let mut loops = Vec::new();
        for radius in [1., 0.7, 0.3] {
            let mut wire = crate::sketch::circle_wire(radius).unwrap();
            for c in &mut wire {
                for p in &mut c.control_points {
                    p[0] += 3.;
                }
            }
            loops.push(wire);
        }
        let loops = crate::planar_trim::orient_even_odd(&loops, 1e-7).unwrap();
        let model = super::revolve_region(&loops, 1e-7).unwrap();
        assert_eq!(model.bodies.len(), 2);
        assert_eq!(model.shells.len(), 3);
        assert_eq!(model.bodies[0].inner_shells.len(), 1);
        let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
        let expected = 6. * std::f64::consts::PI.powi(2) * (1. - 0.7_f64.powi(2) + 0.3_f64.powi(2));
        assert!((mass.signed_volume_mm3 / expected - 1.).abs() < 1e-6);
    }
    #[test]
    fn curved_axis_endpoints_form_shared_sphere_poles() {
        let circle = crate::sketch::circle_wire(1.).unwrap();
        let wire = vec![
            circle[3].clone(),
            circle[0].clone(),
            super::line(vec![0., 1.], vec![0., -1. + f64::EPSILON / 2.]),
        ];
        for angle in [360., 90., -210.] {
            let model = super::revolve_wire_angle(&wire, 1e-7, angle).unwrap();
            model.validate().unwrap();
            assert_eq!(
                model
                    .vertices
                    .iter()
                    .filter(|v| v.point[0] == 0. && v.point[1] == 0.)
                    .count(),
                2
            );
            let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
            let expected = 4. * std::f64::consts::PI / 3. * angle.abs() / 360.;
            assert!((mass.signed_volume_mm3 / expected - 1.).abs() < 1e-6);
        }
        let mut horn = circle;
        for c in &mut horn {
            for p in &mut c.control_points {
                p[0] += 1.;
            }
        }
        assert!(super::revolve_wire(&horn, 1e-7).is_err());
    }
    #[test]
    fn partial_curved_revolution_caps_and_signed_volume() {
        let mut wire = crate::sketch::circle_wire(1.).unwrap();
        for c in &mut wire {
            for p in &mut c.control_points {
                p[0] += 3.;
            }
        }
        for angle in [90., -90., 210., -210.] {
            let model = super::revolve_wire_angle(&wire, 1e-7, angle).unwrap();
            model.validate().unwrap();
            assert_eq!(
                model.faces.len(),
                4 * (angle.abs() / 90.).ceil() as usize + 2
            );
            let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
            let expected = 6. * std::f64::consts::PI.powi(2) * angle.abs() / 360.;
            assert!((mass.signed_volume_mm3 / expected - 1.).abs() < 1e-6);
            assert_eq!(
                model
                    .faces
                    .iter()
                    .filter(|f| f.surface.degree_u == 1)
                    .count(),
                2
            );
        }
    }
    #[test]
    fn retained_circle_revolution_is_a_closed_rational_torus() {
        let mut wire = crate::sketch::circle_wire(1.).unwrap();
        for c in &mut wire {
            for p in &mut c.control_points {
                p[0] += 3.;
                p[1] += 2.;
            }
        }
        let model = super::revolve_wire(&wire, 1e-7).unwrap();
        model.validate().unwrap();
        assert_eq!(model.faces.len(), 16);
        assert_eq!(model.edges.len(), 32);
        assert_eq!(model.vertices.len(), 16);
        for face in &model.faces {
            for u in [0., 0.13, 0.5, 0.87, 1.] {
                for v in [0., 0.17, 0.5, 0.83, 1.] {
                    let p = face.surface.evaluate(u, v).unwrap().point;
                    let implicit = (p[0].hypot(p[1]) - 3.).powi(2) + (p[2] - 2.).powi(2);
                    assert!((implicit - 1.).abs() < 1e-12);
                }
            }
        }
        assert!(super::revolve_wire(&crate::sketch::circle_wire(1.).unwrap(), 1e-7).is_err());
        wire[0].control_points[0][0] += 1e-9;
        assert!(super::revolve_wire(&wire, 1e-7).is_err());
    }
    use super::*;
    #[test]
    fn round_solids_have_exact_rational_boundaries_and_shared_topology() {
        for (model, expected_union_volume) in [
            (
                cylinder(3., 5.).unwrap(),
                Some(45. * std::f64::consts::PI + 4.),
            ),
            (frustum(3., 1., 5.).unwrap(), None),
            (tube(3., 1., 5.).unwrap(), None),
        ] {
            let report = model.validate().unwrap();
            assert_eq!(report.boundary_edge_count, 0);
            for face in model.faces.iter().filter(|f| f.surface.degree_u == 2) {
                for i in 0..=20 {
                    let v = i as f64 / 20.;
                    let expected = face.surface.control_points[0][0][0]
                        .hypot(face.surface.control_points[0][0][1])
                        * (1. - v)
                        + face.surface.control_points[0][1][0]
                            .hypot(face.surface.control_points[0][1][1])
                            * v;
                    for j in 0..=20 {
                        let p = face.surface.evaluate(j as f64 / 20., v).unwrap().point;
                        assert!((p[0].hypot(p[1]) - expected).abs() < 1e-12);
                    }
                }
            }
            let restored: Model =
                value_codec::from_str(&value_codec::to_string(&model).unwrap()).unwrap();
            restored.validate().unwrap();
            assert_eq!(model.1.faces, restored.1.faces);
            let union = boolean(
                &model,
                &cuboid([-1., -1., -1.], [1., 1., 1.]).unwrap(),
                "union",
            );
            if let Some(expected_volume) = expected_union_volume {
                let union = union.unwrap();
                assert_eq!(union.bodies.len(), 1);
                assert_eq!(union.validate().unwrap().boundary_edge_count, 0);
                let properties = crate::analysis::mass_properties(&union, 1e-7, 800_000).unwrap();
                assert!((properties.signed_volume_mm3 - expected_volume).abs() < 1e-6);
            } else {
                assert!(union.is_err());
            }
        }
    }
    fn verify_round_trip(model: &Model) {
        let report = model.validate().unwrap();
        assert_eq!(report.boundary_edge_count, 0);
        assert!(model.edges.iter().all(|e| e.vertices[0] != e.vertices[1]));
        let encoded = value_codec::to_string(model).unwrap();
        let restored: Model = value_codec::from_str(&encoded).unwrap();
        restored.validate().unwrap();
        assert_eq!(encoded, value_codec::to_string(&restored).unwrap());
    }
    fn normal(surface: &Surface, u: f64, v: f64) -> [f64; 3] {
        let e = surface.evaluate(u, v).unwrap();
        let value = value_codec::Serialize::to_value(&e);
        value_codec::Deserialize::from_value(value["normal"].clone()).unwrap()
    }
    #[test]
    fn sphere_is_exact_regular_oriented_and_has_ordinary_poles() {
        for radius in [1e-5, 3., 1e6] {
            let model = sphere(radius).unwrap();
            verify_round_trip(&model);
            assert_eq!(
                (model.vertices.len(), model.edges.len(), model.faces.len()),
                (6, 12, 8)
            );
            for usage in &model.shells[0].faces {
                let surface = &model.faces[usage.face].surface;
                for i in 0..=10 {
                    for j in 0..=10 {
                        let (u, v) = (i as f64 / 10., j as f64 / 10.);
                        let p = surface.evaluate(u, v).unwrap().point;
                        assert!(
                            (p.iter().map(|x| x * x).sum::<f64>().sqrt() - radius).abs()
                                <= radius * 1e-14
                        );
                        let n = normal(surface, u, v);
                        let dot = (0..3).map(|k| n[k] * p[k]).sum::<f64>()
                            * if usage.reversed { -1. } else { 1. };
                        assert!(dot > 0., "Sphere normal must point outward including poles");
                    }
                }
            }
        }
    }
    #[test]
    fn torus_is_exact_regular_oriented_and_genus_one() {
        let (major, minor) = (5., 2.);
        let model = torus(major, minor).unwrap();
        verify_round_trip(&model);
        assert_eq!(
            (model.vertices.len(), model.edges.len(), model.faces.len()),
            (16, 32, 16)
        );
        for usage in &model.shells[0].faces {
            let surface = &model.faces[usage.face].surface;
            for i in 0..=12 {
                for j in 0..=12 {
                    let (u, v) = (i as f64 / 12., j as f64 / 12.);
                    let p = surface.evaluate(u, v).unwrap().point;
                    let radial = p[0].hypot(p[1]);
                    assert!(
                        ((radial - major).powi(2) + p[2].powi(2) - minor.powi(2)).abs() < 1e-12
                    );
                    let outward = [
                        p[0] * (1. - major / radial),
                        p[1] * (1. - major / radial),
                        p[2],
                    ];
                    let n = normal(surface, u, v);
                    assert!((0..3).map(|k| outward[k] * n[k]).sum::<f64>() > 0.);
                }
            }
        }
        assert!(torus(2., 2.).is_err());
        assert!(torus(1., 2.).is_err());
        assert!(torus(1e6, 2.).is_err());
        assert!(sphere(0.).is_err());
        assert!(sphere(f64::NAN).is_err());
    }
    #[test]
    fn cones_have_one_certified_pole_without_artificial_caps() {
        for (bottom, top) in [(3., 0.), (0., 3.)] {
            let model = frustum(bottom, top, 5.).unwrap();
            assert_eq!((model.vertices.len(), model.faces.len()), (5, 5));
            assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
            assert_eq!(model.edges.iter().filter(|e| e.degenerate).count(), 1);
            for usage in &model.shells[0].faces {
                let surface = &model.faces[usage.face].surface;
                if surface.degree_u == 1 {
                    continue;
                }
                for i in 0..=16 {
                    for j in 1..16 {
                        let (u, v) = (i as f64 / 16., j as f64 / 16.);
                        let p = surface.evaluate(u, v).unwrap().point;
                        assert!((p[0].hypot(p[1]) - (bottom * (1. - v) + top * v)).abs() < 1e-12);
                        let n = normal(surface, u, v);
                        assert!(p[0] * n[0] + p[1] * n[1] > 0.);
                    }
                }
            }
            let encoded = value_codec::to_string(&model).unwrap();
            let restored: Model = value_codec::from_str(&encoded).unwrap();
            restored.validate().unwrap();
            assert_eq!(encoded, value_codec::to_string(&restored).unwrap());
            let pole_edge = model.edges.iter().position(|e| e.degenerate).unwrap();
            let mut stray_pole = model.clone();
            for face in 1..4 {
                let outer = stray_pole.faces[face].outer;
                stray_pole.loops[outer]
                    .coedges
                    .retain(|c| c.edge != pole_edge);
            }
            assert!(stray_pole.validate().is_err());
            for mutation in 0..6 {
                let mut bad = model.clone();
                match mutation {
                    0 => bad.edges[pole_edge].degenerate = false,
                    1 => bad.edges[pole_edge].curve.control_points[1][0] += 1e-12,
                    2 => bad.edges[pole_edge].vertices[1] = 1,
                    3 => bad.faces[0].surface.control_points[1][usize::from(top == 0.)][0] += 1e-12,
                    4 => {
                        let mut e = bad.edges[pole_edge].clone();
                        e.degenerate = true;
                        bad.edges.push(e);
                    }
                    _ => bad.vertices.push(Vertex {
                        point: [0., 0., 99.],
                    }),
                }
                bad.rebuild_topology_ids();
                assert!(bad.validate().is_err(), "Invalid cone mutation {mutation}");
            }
        }
        let mut marked_finite = cylinder(3., 5.).unwrap();
        marked_finite.edges[0].degenerate = true;
        assert!(marked_finite.validate().is_err());
    }
    #[test]
    fn collapsed_boundary_cannot_join_disconnected_face_fans() {
        let mut model = frustum(3., 0., 5.).unwrap();
        let other = frustum(4., 0., 5.).unwrap();
        let pole_edge = model.edges.iter().position(|e| e.degenerate).unwrap();
        let pole = model.edges[pole_edge].vertices[0];
        let other_pole_edge = other.edges.iter().position(|e| e.degenerate).unwrap();
        let other_pole = other.edges[other_pole_edge].vertices[0];
        let vertices: Vec<_> = other
            .vertices
            .iter()
            .enumerate()
            .map(|(i, vertex)| {
                if i == other_pole {
                    pole
                } else {
                    let id = model.vertices.len();
                    model.vertices.push(vertex.clone());
                    id
                }
            })
            .collect();
        let edges: Vec<_> = other
            .edges
            .iter()
            .enumerate()
            .map(|(i, edge)| {
                if i == other_pole_edge {
                    pole_edge
                } else {
                    let id = model.edges.len();
                    let mut edge = edge.clone();
                    edge.vertices = edge.vertices.map(|v| vertices[v]);
                    model.edges.push(edge);
                    id
                }
            })
            .collect();
        let loop_offset = model.loops.len();
        for wire in &other.loops {
            let mut wire = wire.clone();
            for c in &mut wire.coedges {
                c.edge = edges[c.edge];
            }
            model.loops.push(wire);
        }
        let face_offset = model.faces.len();
        for face in &other.faces {
            let mut face = face.clone();
            face.outer += loop_offset;
            face.holes.iter_mut().for_each(|l| *l += loop_offset);
            model.faces.push(face);
        }
        for usage in &other.shells[0].faces {
            model.shells[0].faces.push(FaceUse {
                face: usage.face + face_offset,
                reversed: usage.reversed,
            });
        }
        assert!(
            model
                .0
                .validate_topology()
                .unwrap_err()
                .message
                .contains("disconnected")
        );
    }
    #[test]
    fn axis_profiles_revolve_to_closed_solids_with_explicit_poles() {
        for profile in [
            vec![[0., 0.], [3., 0.], [0., 5.]],
            vec![[0., 0.], [3., 0.], [3., 5.], [0., 5.], [0., 3.], [0., 2.]],
            vec![[0., 0.], [3., 0.], [3., 5.], [2., 5.], [2., 2.], [0., 2.]],
        ] {
            let model = revolve(&profile).unwrap();
            assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
            assert!(model.edges.iter().any(|e| e.degenerate));
            let restored: Model =
                value_codec::from_str(&value_codec::to_string(&model).unwrap()).unwrap();
            restored.validate().unwrap();
        }
    }
    fn rectangular_surface_volume(model: &Model) -> f64 {
        let quadrature = [
            (0.06943184420297371, 0.17392742256872692),
            (0.33000947820757187, 0.32607257743127305),
            (0.6699905217924281, 0.32607257743127305),
            (0.9305681557970262, 0.17392742256872692),
        ];
        let mut volume = 0.;
        for usage in &model.shells[0].faces {
            for (u, wu) in quadrature {
                for (v, wv) in quadrature {
                    let evaluated = model.faces[usage.face].surface.evaluate(u, v).unwrap();
                    let data = value_codec::Serialize::to_value(&evaluated);
                    let du: [f64; 3] =
                        value_codec::Deserialize::from_value(data["du"].clone()).unwrap();
                    let dv: [f64; 3] =
                        value_codec::Deserialize::from_value(data["dv"].clone()).unwrap();
                    let n = [
                        du[1] * dv[2] - du[2] * dv[1],
                        du[2] * dv[0] - du[0] * dv[2],
                        du[0] * dv[1] - du[1] * dv[0],
                    ];
                    let dot = (0..3).map(|i| evaluated.point[i] * n[i]).sum::<f64>();
                    volume += dot * wu * wv / 3. * if usage.reversed { -1. } else { 1. };
                }
            }
        }
        volume
    }
    #[test]
    fn signed_partial_revolutions_have_exact_caps_and_correct_oriented_volume() {
        for angle in [30., 90., 137., 270., -30., -90., -137., -270., 360., -360.] {
            for inner in [0., 1.] {
                let profile = [[inner, 0.], [3., 0.], [3., 4.], [inner, 4.]];
                let model = revolve_angle(&profile, angle).unwrap();
                assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
                let expected = angle.abs().to_radians() * (9. - inner * inner) * 4. / 2.;
                assert!((rectangular_surface_volume(&model) / expected - 1.).abs() < 5e-5);
                let restored: Model =
                    value_codec::from_str(&value_codec::to_string(&model).unwrap()).unwrap();
                restored.validate().unwrap();
            }
        }
        for angle in [15., 120., -280.] {
            let profile = [[0., 0.], [3., 0.], [3., 1.], [1., 1.], [1., 3.], [0., 3.]];
            let model = revolve_angle(&profile, angle).unwrap();
            assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
            for face in model.faces.iter().rev().take(2) {
                assert_eq!(model.loops[face.outer].coedges.len(), profile.len());
            }
        }
        for invalid in [0., 361., -361., f64::NAN, f64::INFINITY] {
            assert!(revolve_angle(&[[0., 0.], [3., 0.], [0., 4.]], invalid).is_err());
        }
    }
    #[test]
    fn rejects_singular_and_invalid_round_solids() {
        for radius in [0., -1., f64::NAN, f64::INFINITY, 1e7] {
            assert!(cylinder(radius, 2.).is_err());
        }
        assert!(frustum(0., 0., 3.).is_err());
        assert!(tube(2., 2., 3.).is_err());
        assert!(tube(2., 3., 3.).is_err());
    }
}
