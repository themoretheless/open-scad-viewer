use super::*;
pub fn gear_with_report(spec: &GearSpec) -> Result<(Model, GearGeometry)> {
    let (loops, mut geometry) = profile_with_report(spec)?;
    let beta = spec.helix_angle_deg.to_radians();
    let total_twist = spec.height * beta.tan() / geometry.pitch_radius;
    // Axial layers: (z0, z1, theta0, twist).
    let layers: Vec<(f64, f64, f64, f64)> = if spec.herringbone && beta != 0. {
        let half = spec.height / 2.;
        vec![
            (0., half, 0., total_twist / 2.),
            (half, spec.height, total_twist / 2., -total_twist / 2.),
        ]
    } else {
        vec![(0., spec.height, 0., total_twist)]
    };
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut loops_out: Vec<Loop> = Vec::new();
    let mut faces: Vec<Face> = Vec::new();
    let mut uses: Vec<FaceUse> = Vec::new();
    let mut sweep_deviation: f64 = 0.;
    let nominal_twist = nurbs_core::involute::derive_gear_twist_bounds(
        spec.module,
        spec.teeth,
        spec.height,
        spec.helix_angle_deg,
    )
    .ok();
    let mut sweep_rounding_certified = nominal_twist.is_some();
    // Per level (layer boundaries) and per loop: the level edges (one per
    // outline curve) and the level vertices (one per curve start).
    let level_z: Vec<f64> = std::iter::once(layers[0].0)
        .chain(layers.iter().map(|l| l.1))
        .collect();
    let level_theta: Vec<f64> = std::iter::once(layers[0].2)
        .chain(layers.iter().map(|l| l.2 + l.3))
        .collect();
    let mut level_vertices: Vec<Vec<Vec<usize>>> = Vec::new();
    let mut level_edges: Vec<Vec<Vec<usize>>> = Vec::new();
    for (li, &z) in level_z.iter().enumerate() {
        let theta = level_theta[li];
        let mut vl = Vec::new();
        let mut el = Vec::new();
        for outline_loop in &loops {
            let mut vs = Vec::new();
            for c in outline_loop {
                let rotated = lift(&rotate2(c, theta), z);
                let p = rotated.control_points[0].clone();
                vs.push(vertices.len());
                vertices.push(Vertex {
                    point: [p[0], p[1], p[2]],
                });
            }
            let n = outline_loop.len();
            let mut es = Vec::new();
            for (k, c) in outline_loop.iter().enumerate() {
                let rotated = lift(&rotate2(c, theta), z);
                es.push(edges.len());
                edges.push(Edge {
                    degenerate: false,
                    vertices: [vs[k], vs[(k + 1) % n]],
                    curve: rotated,
                });
            }
            vl.push(vs);
            el.push(es);
        }
        level_vertices.push(vl);
        level_edges.push(el);
    }
    // Walls.
    for (layer, &(z0, z1, theta0, twist)) in layers.iter().enumerate() {
        for (loop_index, outline_loop) in loops.iter().enumerate() {
            let n = outline_loop.len();
            // Vertical edges at every curve start of this layer.
            let mut vertical: Vec<usize> = Vec::with_capacity(n);
            let mut surfaces: Vec<Surface> = Vec::with_capacity(n);
            for c in outline_loop {
                let nominal = nominal_twist
                    .map(|total| -> Result<_> {
                        use nurbs_core::interval_eval::Interval as I;
                        let t = I::new(total[0], total[1])?;
                        if layers.len() == 2 {
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
                    .transpose()?;
                let (surface, dev, certified) = wall(c, z0, z1, theta0, twist, nominal)?;
                sweep_rounding_certified &= certified;
                sweep_deviation = sweep_deviation.max(dev);
                surfaces.push(surface);
            }
            for k in 0..n {
                let curve = column(&surfaces[k], false);
                vertical.push(edges.len());
                edges.push(Edge {
                    degenerate: false,
                    vertices: [
                        level_vertices[layer][loop_index][k],
                        level_vertices[layer + 1][loop_index][k],
                    ],
                    curve,
                });
            }
            for k in 0..n {
                let next = (k + 1) % n;
                let bottom = level_edges[layer][loop_index][k];
                let top = level_edges[layer + 1][loop_index][k];
                let coedges = vec![
                    Coedge::new(
                        bottom,
                        false,
                        unit_line([0., 0.], [1., 0.]),
                    ),
                    Coedge::new(
                        vertical[next],
                        false,
                        unit_line([1., 0.], [1., 1.]),
                    ),
                    Coedge::new(
                        top,
                        true,
                        unit_line([1., 1.], [0., 1.]),
                    ),
                    Coedge::new(
                        vertical[k],
                        true,
                        unit_line([0., 1.], [0., 0.]),
                    ),
                ];
                loops_out.push(Loop { coedges });
                faces.push(Face {
                    surface: surfaces[k].clone(),
                    outer: loops_out.len() - 1,
                    holes: vec![],
                });
                uses.push(FaceUse {
                    face: faces.len() - 1,
                    reversed: false,
                });
            }
        }
    }
    // Caps: bottom (reversed use, normal -z) and top.
    for (li, reversed) in [(0usize, true), (level_z.len() - 1, false)] {
        let z = level_z[li];
        let theta = level_theta[li];
        let rotated: Vec<Vec<Curve>> = loops
            .iter()
            .map(|l| l.iter().map(|c| rotate2(c, theta)).collect())
            .collect();
        let all: Vec<&Curve> = rotated.iter().flatten().collect();
        let (surface, map) = cap(&all, z);
        let mut loop_ids = Vec::new();
        for (loop_index, curves) in rotated.iter().enumerate() {
            let coedges = curves
                .iter()
                .enumerate()
                .map(|(k, c)| {
                    Coedge::new(level_edges[li][loop_index][k], false, cap_pcurve(c, map))
                })
                .collect();
            loops_out.push(Loop { coedges });
            loop_ids.push(loops_out.len() - 1);
        }
        faces.push(Face {
            surface,
            outer: loop_ids[0],
            holes: loop_ids[1..].to_vec(),
        });
        uses.push(FaceUse {
            face: faces.len() - 1,
            reversed,
        });
    }
    geometry.sweep_deviation = sweep_deviation;
    geometry.sweep_rounding_certified = sweep_rounding_certified;
    geometry.ideal_wall_error_upper = if sweep_rounding_certified {
        if let Some(profile) = geometry.profile_error_upper {
            use nurbs_core::interval_eval::Interval as I;
            Some(I::point(profile).add(I::point(sweep_deviation))?.hi)
        } else {
            None
        }
    } else {
        None
    };
    geometry.face_count = faces.len();
    let mut model = Model(
        brep_topology::Model {
            vertices,
            edges,
            loops: loops_out,
            faces,
            shells: vec![Shell {
                faces: uses,
                closed: true,
            }],
            bodies: vec![Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model.validate()?;
    Ok((model, geometry))
}

pub fn gear(spec: &GearSpec) -> Result<Model> {
    gear_with_report(spec).map(|(model, _)| model)
}

