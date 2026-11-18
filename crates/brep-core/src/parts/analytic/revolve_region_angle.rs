use super::*;

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
        return Err(Error::new("BREP_INVALID_ANGLE","Revolve angle magnitude must be in 0.00001..360 degrees"));
    }
    planar_trim::validate(&[wire.to_vec()],tolerance)?;
    revolve_wire_assembly(wire,tolerance,angle_degrees)
}
pub(super) fn revolve_g2_annular_meridian(source:&g2_annular::Meridian,tolerance:f64)->Result<Model> {
    revolve_wire_assembly(source.wire(),tolerance,360.)
}
fn revolve_wire_assembly(wire: &[Curve], tolerance: f64, angle_degrees: f64) -> Result<Model> {
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
