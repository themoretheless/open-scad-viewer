use super::*;
use nurbs_core::interval_eval::Interval as I;

/// Model tolerance of the assembled gear body, in millimetres.
const GEAR_MODEL_TOLERANCE_MM: f64 = 1e-7;

/// Axial layer: (z0, z1, theta0, twist).
type Layer = (f64, f64, f64, f64);
/// Nominal start and twist intervals of one layer.
type NominalTwist = ([f64; 2], [f64; 2]);

/// Per outline loop at one layer boundary: the level vertices (one per curve
/// start) and level edges (one per outline curve).
struct Level {
    z: f64,
    theta: f64,
    vertices: Vec<Vec<usize>>,
    edges: Vec<Vec<usize>>,
}

/// Topology arrays of the gear body under construction plus sweep tallies.
struct GearBodyBuilder<'a> {
    loops: &'a [Vec<Curve>],
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    loops_out: Vec<Loop>,
    faces: Vec<Face>,
    uses: Vec<FaceUse>,
    sweep_deviation: f64,
    sweep_rounding_certified: bool,
}

fn axial_layers(spec: &GearSpec, beta: f64, total_twist: f64) -> Vec<Layer> {
    if spec.herringbone && beta != 0. {
        let half = spec.height / 2.;
        vec![
            (0., half, 0., total_twist / 2.),
            (half, spec.height, total_twist / 2., -total_twist / 2.),
        ]
    } else {
        vec![(0., spec.height, 0., total_twist)]
    }
}

fn max_radius<'p>(points: impl Iterator<Item = &'p Vec<f64>>) -> f64 {
    points.map(|p| p[0].hypot(p[1])).fold(0., f64::max)
}

/// Sweep budget of one curve, scaled by its radius relative to its loop.
fn curve_budget(c: &Curve, loop_max_radius: f64) -> f64 {
    let curve_radius = max_radius(c.control_points.iter());
    if loop_max_radius > 0. && curve_radius > 0. {
        (SWEEP_FIT * (curve_radius / loop_max_radius)).min(SWEEP_FIT)
    } else {
        SWEEP_FIT
    }
}

fn layer_nominal_twist(
    nominal_twist: Option<[f64; 2]>,
    layer_count: usize,
    layer: usize,
) -> Result<Option<NominalTwist>> {
    nominal_twist
        .map(|total| -> Result<_> {
            let t = I::new(total[0], total[1])?;
            if layer_count == 2 {
                let half = t.div(I::point(2.))?;
                if layer == 0 {
                    Ok(([0.; 2], [half.lo, half.hi]))
                } else {
                    Ok(([half.lo, half.hi], [-half.hi, -half.lo]))
                }
            } else {
                Ok(([0.; 2], [t.lo, t.hi]))
            }
        })
        .transpose()
}

impl<'a> GearBodyBuilder<'a> {
    fn push_level(&mut self, z: f64, theta: f64) -> Level {
        let mut level = Level {
            z,
            theta,
            vertices: Vec::new(),
            edges: Vec::new(),
        };
        for outline_loop in self.loops {
            let mut vs = Vec::new();
            for c in outline_loop {
                let rotated = lift(&rotate2(c, theta), z);
                let p = rotated.control_points[0].clone();
                vs.push(self.vertices.len());
                self.vertices.push(Vertex {
                    point: [p[0], p[1], p[2]],
                });
            }
            let n = outline_loop.len();
            let mut es = Vec::new();
            for (k, c) in outline_loop.iter().enumerate() {
                let rotated = lift(&rotate2(c, theta), z);
                es.push(self.edges.len());
                self.edges.push(Edge {
                    degenerate: false,
                    vertices: [vs[k], vs[(k + 1) % n]],
                    curve: rotated,
                });
            }
            level.vertices.push(vs);
            level.edges.push(es);
        }
        level
    }

    fn wall_surfaces(
        &mut self,
        outline_loop: &[Curve],
        layer: Layer,
        nominal: Option<NominalTwist>,
    ) -> Result<Vec<Surface>> {
        let (z0, z1, theta0, twist) = layer;
        let loop_max_radius = max_radius(outline_loop.iter().flat_map(|c| &c.control_points));
        let mut surfaces = Vec::with_capacity(outline_loop.len());
        for c in outline_loop {
            let loop_budget = curve_budget(c, loop_max_radius);
            let (surface, dev, certified) = wall(c, z0, z1, theta0, twist, loop_budget, nominal)?;
            self.sweep_rounding_certified &= certified;
            self.sweep_deviation = self.sweep_deviation.max(dev);
            surfaces.push(surface);
        }
        Ok(surfaces)
    }

    /// Vertical edges at every curve start, then one wall face per curve.
    fn push_wall_faces(
        &mut self,
        surfaces: &[Surface],
        bottom: (&[usize], &[usize]),
        top: (&[usize], &[usize]),
    ) {
        let n = surfaces.len();
        let mut vertical: Vec<usize> = Vec::with_capacity(n);
        for (k, surface) in surfaces.iter().enumerate() {
            let curve = column(surface, false);
            vertical.push(self.edges.len());
            self.edges.push(Edge {
                degenerate: false,
                vertices: [bottom.0[k], top.0[k]],
                curve,
            });
        }
        for (k, surface) in surfaces.iter().enumerate() {
            let next = (k + 1) % n;
            let coedges = vec![
                Coedge::new(bottom.1[k], false, unit_line([0., 0.], [1., 0.])),
                Coedge::new(vertical[next], false, unit_line([1., 0.], [1., 1.])),
                Coedge::new(top.1[k], true, unit_line([1., 1.], [0., 1.])),
                Coedge::new(vertical[k], true, unit_line([0., 1.], [0., 0.])),
            ];
            self.loops_out.push(Loop { coedges });
            self.faces.push(Face {
                surface: surface.clone(),
                outer: self.loops_out.len() - 1,
                holes: vec![],
            });
            self.uses.push(FaceUse {
                face: self.faces.len() - 1,
                reversed: false,
            });
        }
    }

    fn push_walls(
        &mut self,
        layers: &[Layer],
        levels: &[Level],
        nominal_twist: Option<[f64; 2]>,
    ) -> Result<()> {
        let loops = self.loops;
        for (layer, (&bounds, pair)) in layers.iter().zip(levels.windows(2)).enumerate() {
            let (low, high) = (&pair[0], &pair[1]);
            for (loop_index, outline_loop) in loops.iter().enumerate() {
                let nominal = layer_nominal_twist(nominal_twist, layers.len(), layer)?;
                let surfaces = self.wall_surfaces(outline_loop, bounds, nominal)?;
                self.push_wall_faces(
                    &surfaces,
                    (&low.vertices[loop_index], &low.edges[loop_index]),
                    (&high.vertices[loop_index], &high.edges[loop_index]),
                );
            }
        }
        Ok(())
    }

    fn push_cap(&mut self, level: &Level, reversed: bool) {
        let rotated: Vec<Vec<Curve>> = self
            .loops
            .iter()
            .map(|l| l.iter().map(|c| rotate2(c, level.theta)).collect())
            .collect();
        let all: Vec<&Curve> = rotated.iter().flatten().collect();
        let (surface, map) = cap(&all, level.z);
        let mut loop_ids = Vec::new();
        for (curves, level_edges) in rotated.iter().zip(&level.edges) {
            let coedges = curves
                .iter()
                .zip(level_edges)
                .map(|(c, &edge)| Coedge::new(edge, false, cap_pcurve(c, map)))
                .collect();
            self.loops_out.push(Loop { coedges });
            loop_ids.push(self.loops_out.len() - 1);
        }
        self.faces.push(Face {
            surface,
            outer: loop_ids[0],
            holes: loop_ids[1..].to_vec(),
        });
        self.uses.push(FaceUse {
            face: self.faces.len() - 1,
            reversed,
        });
    }

    fn into_model(self) -> Result<Model> {
        let mut model = Model(
            brep_topology::Model {
                vertices: self.vertices,
                edges: self.edges,
                loops: self.loops_out,
                faces: self.faces,
                shells: vec![Shell {
                    faces: self.uses,
                    closed: true,
                }],
                bodies: vec![Body {
                    outer_shell: 0,
                    inner_shells: vec![],
                }],
                tolerance_mm: GEAR_MODEL_TOLERANCE_MM,
            },
            TopologyIds::default(),
        );
        model.rebuild_topology_ids();
        model.validate()?;
        Ok(model)
    }
}

fn ideal_wall_error_upper(
    sweep_rounding_certified: bool,
    profile_error_upper: Option<f64>,
    sweep_deviation: f64,
) -> Result<Option<f64>> {
    if !sweep_rounding_certified {
        return Ok(None);
    }
    match profile_error_upper {
        Some(profile) => Ok(Some(I::point(profile).add(I::point(sweep_deviation))?.hi)),
        None => Ok(None),
    }
}

pub fn gear_with_report(spec: &GearSpec) -> Result<(Model, GearGeometry)> {
    let (loops, mut geometry) = profile_with_report(spec)?;
    let beta = spec.helix_angle_deg.to_radians();
    let total_twist = spec.height * beta.tan() / geometry.pitch_radius;
    let layers = axial_layers(spec, beta, total_twist);
    let nominal_twist = nurbs_core::involute::derive_gear_twist_bounds(
        spec.module,
        spec.teeth,
        spec.height,
        spec.helix_angle_deg,
    )
    .ok();
    let mut builder = GearBodyBuilder {
        loops: &loops,
        vertices: Vec::new(),
        edges: Vec::new(),
        loops_out: Vec::new(),
        faces: Vec::new(),
        uses: Vec::new(),
        sweep_deviation: 0.,
        sweep_rounding_certified: nominal_twist.is_some(),
    };
    let level_z_theta: Vec<(f64, f64)> = layers
        .iter()
        .take(1)
        .map(|l| (l.0, l.2))
        .chain(layers.iter().map(|l| (l.1, l.2 + l.3)))
        .collect();
    let levels: Vec<Level> = level_z_theta
        .into_iter()
        .map(|(z, theta)| builder.push_level(z, theta))
        .collect();
    builder.push_walls(&layers, &levels, nominal_twist)?;
    // Caps: bottom (reversed use, normal -z) and top.
    if let (Some(bottom), Some(top)) = (levels.first(), levels.last()) {
        builder.push_cap(bottom, true);
        builder.push_cap(top, false);
    }
    geometry.sweep_deviation = builder.sweep_deviation;
    geometry.sweep_rounding_certified = builder.sweep_rounding_certified;
    geometry.ideal_wall_error_upper = ideal_wall_error_upper(
        builder.sweep_rounding_certified,
        geometry.profile_error_upper,
        builder.sweep_deviation,
    )?;
    geometry.face_count = builder.faces.len();
    let model = builder.into_model()?;
    Ok((model, geometry))
}

pub fn gear(spec: &GearSpec) -> Result<Model> {
    gear_with_report(spec).map(|(model, _)| model)
}
