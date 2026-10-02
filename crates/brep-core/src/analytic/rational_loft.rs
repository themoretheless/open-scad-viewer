//! Retained rational section loft with authored incidence and audited cap trims.
//! A closed manifold model is not a proof of global embedding/self-separation.
use super::*;
type P = [f64; 3];
fn err(message: impl Into<String>) -> Error {
    Error::new("BREP_RATIONAL_SWEEP_REFUSED", message)
}
fn minus(a: P, b: P) -> P {
    std::array::from_fn(|k| a[k] - b[k])
}
fn dot(a: P, b: P) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(p: P) -> Result<P> {
    let n = p[0].hypot(p[1]).hypot(p[2]);
    if !n.is_finite() || n <= 1e-10 {
        return Err(err("Singular cap frame"));
    }
    Ok(p.map(|x| x / n))
}
fn point(c: &Curve, start: bool) -> P {
    let p = if start {
        &c.control_points[0]
    } else {
        c.control_points.last().unwrap()
    };
    [p[0], p[1], p[2]]
}
struct Cap {
    normal: P,
    surface: Surface,
    loops: Vec<Vec<Curve>>,
}
fn cap(loops: &[Vec<Curve>]) -> Result<Cap> {
    let origin = point(&loops[0][0], true);
    let u = unit(minus(point(&loops[0][0], false), origin))?;
    let candidate = loops[0]
        .iter()
        .flat_map(|c| &c.control_points)
        .map(|p| cross(u, minus([p[0], p[1], p[2]], origin)))
        .max_by(|a, b| dot(*a, *a).total_cmp(&dot(*b, *b)))
        .unwrap();
    let n = unit(candidate)?;
    let v = cross(n, u);
    let mut projected = loops.to_vec();
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in projected
        .iter_mut()
        .flatten()
        .flat_map(|c| &mut c.control_points)
    {
        let delta = minus([p[0], p[1], p[2]], origin);
        if dot(delta, n).abs() > 1e-9 {
            return Err(err("Cap section controls must be coplanar"));
        }
        *p = vec![dot(delta, u), dot(delta, v)];
        for k in 0..2 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    let width = [max[0] - min[0], max[1] - min[1]];
    if width.iter().any(|x| !x.is_finite() || *x <= 1e-8) {
        return Err(err("Collapsed cap bounds"));
    }
    for p in projected
        .iter_mut()
        .flatten()
        .flat_map(|c| &mut c.control_points)
    {
        for k in 0..2 {
            p[k] = (p[k] - min[k]) / width[k];
        }
    }
    let audit = nurbs_core::trim_region_audit::inspect(&projected, 1e-10, 100000, 100000, 1000000)?;
    if audit.valid != Some(true) {
        return Err(err(format!(
            "Cap trim region unresolved or invalid: {}",
            audit.reason
        )));
    }
    let world = |a: f64, b: f64| {
        (0..3)
            .map(|k| origin[k] + a * u[k] + b * v[k])
            .collect::<Vec<_>>()
    };
    let mut result = Cap {
        normal: n,
        surface: Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![world(min[0], min[1]), world(min[0], max[1])],
                vec![world(max[0], min[1]), world(max[0], max[1])],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        },
        loops: projected,
    };
    if audit.winding[0] == Some(-1) {
        result.normal = result.normal.map(|x| -x);
        for row in &mut result.surface.control_points {
            row.reverse();
        }
        for p in result
            .loops
            .iter_mut()
            .flatten()
            .flat_map(|c| &mut c.control_points)
        {
            p[1] = 1. - p[1];
        }
    }
    Ok(result)
}

/// Sections contain one outer loop followed by clockwise holes in a common
/// authored correspondence. Curves are decomposed exactly into Bezier spans;
/// corresponding spans retain degree and weights. Planar end caps must pass
/// interval trim-region audit. Side regularity/global self-intersections remain
/// unproven and are reported by the model's separate solid audit.
pub fn rational_section_loft(sections: &[Vec<Vec<Curve>>]) -> Result<Model> {
    section_loft(sections, None, None, false)
}

/// Cubic section interpolation with audited planar caps and shared side edges.
/// Requires authored span correspondence; global embedding is audited separately.
pub fn natural_section_loft(sections: &[Vec<Vec<Curve>>], parameters: &[f64]) -> Result<Model> {
    if !(2..=11).contains(&sections.len()) || parameters.len() != sections.len() {
        return Err(err(
            "Natural capped loft needs 2..11 sections and matching stations",
        ));
    }
    section_loft(sections, Some(parameters), None, false)
}

/// Caps complete authored side patches. Every patch corresponds to one Bezier
/// span of the endpoint loops; shared isocurves are checked by BRep validation.
pub fn capped_loft_surfaces(
    start: &[Vec<Curve>],
    end: &[Vec<Curve>],
    sides: &[Vec<Surface>],
) -> Result<Model> {
    section_loft(&[start.to_vec(), end.to_vec()], None, Some(sides), false)
}

/// Uncapped periodic topology: final section must be an exact repeat of the
/// first. Each authored contour owns a connected shell; holes become inner
/// shells of one body. Embedding and global shell containment remain unproven.
pub fn periodic_section_loft(sections: &[Vec<Vec<Curve>>]) -> Result<Model> {
    if sections.len() < 4 {
        return Err(err(
            "Periodic loft needs at least four sections including repeated seam",
        ));
    }
    section_loft(sections, None, None, true)
}

fn section_loft(
    sections: &[Vec<Vec<Curve>>],
    parameters: Option<&[f64]>,
    sides: Option<&[Vec<Surface>]>,
    periodic: bool,
) -> Result<Model> {
    if !(2..=1025).contains(&sections.len()) {
        return Err(err("Need 2..1025 rational sections"));
    }
    let loop_count = sections[0].len();
    if !(1..=16).contains(&loop_count) || sections.iter().any(|s| s.len() != loop_count) {
        return Err(err("Need matching 1..16 loops per section"));
    }
    let mut prepared = Vec::new();
    for section in sections {
        let mut loops = Vec::new();
        let mut count = 0;
        for ring in section {
            if ring.is_empty() || ring.len() > 64 {
                return Err(err("Section loop needs 1..64 curves"));
            }
            let mut pieces = Vec::new();
            for curve in ring {
                curve.validate()?;
                if curve.periodic
                    || curve
                        .control_points
                        .iter()
                        .any(|p| p.len() != 3 || p.iter().any(|x| x.abs() > 1e6))
                {
                    return Err(err("Need bounded nonperiodic 3D section curves"));
                }
                for span in curve.decompose()? {
                    let mut c = span.definition().clone();
                    c.knots = std::iter::repeat_n(0., c.degree + 1)
                        .chain(std::iter::repeat_n(1., c.degree + 1))
                        .collect();
                    pieces.push(c);
                    count += 1;
                    if count > 64 {
                        return Err(err("Section exceeds 64 Bezier spans"));
                    }
                }
            }
            if pieces.len() < 3 {
                return Err(err(
                    "Closed loop needs at least three distinct boundary spans",
                ));
            }
            for i in 0..pieces.len() {
                if point(&pieces[i], false) != point(&pieces[(i + 1) % pieces.len()], true) {
                    return Err(err("Section loops require exact authored endpoint joins"));
                }
            }
            loops.push(pieces);
        }
        if count * (sections.len() - 1) + if periodic { 0 } else { 2 } > MAX_FACES {
            return Err(err("Rational section loft exceeds B-rep face budget"));
        }
        prepared.push(loops);
    }
    for section in &prepared[1..] {
        for (a, b) in prepared[0].iter().zip(section) {
            if a.len() != b.len()
                || a.iter()
                    .zip(b)
                    .any(|(a, b)| a.degree != b.degree || a.weights != b.weights)
            {
                return Err(err(
                    "Rational section span correspondence/weights must match",
                ));
            }
        }
    }
    if periodic && prepared.first() != prepared.last() {
        return Err(err(
            "Periodic section loft requires identical rational seam definitions",
        ));
    }
    if let Some(sides) = sides {
        if sides.len() != loop_count
            || sides
                .iter()
                .zip(&prepared[0])
                .any(|(s, r)| s.len() != r.len())
        {
            return Err(err(
                "Capped loft side patches must match endpoint Bezier spans",
            ));
        }
        for surface in sides.iter().flatten() {
            surface.validate()?;
            if surface.knots_u[surface.degree_u] != 0.
                || surface.knots_u[surface.control_points.len()] != 1.
                || surface.knots_v[surface.degree_v] != 0.
                || surface.knots_v[surface.control_points[0].len()] != 1.
            {
                return Err(err("Capped loft patches require normalized U/V domains"));
            }
        }
    }
    let lower = cap(&prepared[0])?;
    let upper = if periodic {
        None
    } else {
        Some(cap(prepared.last().unwrap())?)
    };
    let travel = minus(
        point(&prepared[1][0][0], true),
        point(&prepared[0][0][0], true),
    );
    let direction = dot(travel, lower.normal);
    if !direction.is_finite() || direction.abs() <= 1e-10 {
        return Err(err("Initial loft ruling must cross the cap plane"));
    }

    let mut build = Builder::new();
    let mut ids = prepared
        .iter()
        .enumerate()
        .map(|(station, s)| {
            if periodic && station == prepared.len() - 1 {
                return Vec::new();
            }
            if parameters.is_some() && station != 0 && station != prepared.len() - 1 {
                return Vec::new();
            }
            s.iter()
                .map(|ring| {
                    ring.iter()
                        .map(|c| {
                            let id = build.model.vertices.len();
                            build.model.vertices.push(Vertex {
                                point: point(c, true),
                            });
                            id
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if periodic {
        let last = ids.len() - 1;
        ids[last] = ids[0].clone();
    }
    let mut shell_faces = vec![Vec::new(); loop_count];
    let layers = if parameters.is_some() {
        1
    } else {
        prepared.len() - 1
    };
    for layer in 0..layers {
        let next = if parameters.is_some() {
            prepared.len() - 1
        } else {
            layer + 1
        };
        for ring in 0..loop_count {
            let count = prepared[layer][ring].len();
            for i in 0..count {
                let j = (i + 1) % count;
                let a = &prepared[layer][ring][i];
                let b = &prepared[next][ring][i];
                let side = if let Some(s) = sides {
                    s[ring][i].clone()
                } else if let Some(p) = parameters {
                    let profiles = prepared
                        .iter()
                        .map(|s| s[ring][i].clone())
                        .collect::<Vec<_>>();
                    nurbs_core::natural_loft::interpolate(&profiles, p)?
                } else {
                    nurbs_core::surface::loft(&[a.clone(), b.clone()])?
                };
                let end_edge = side.iso(nurbs_core::surface::Axis::U, 1.)?;
                let start_edge = side.iso(nurbs_core::surface::Axis::U, 0.)?.reverse()?;
                build.rectangular_patch(
                    side,
                    [
                        ids[layer][ring][i],
                        ids[layer][ring][j],
                        ids[next][ring][j],
                        ids[next][ring][i],
                    ],
                    [a.clone(), end_edge, b.reverse()?, start_edge],
                );
                shell_faces[ring].push(FaceUse {
                    face: build.model.faces.len() - 1,
                    reversed: false,
                });
            }
        }
    }
    let caps = if let Some(upper) = upper {
        vec![(0, lower), (prepared.len() - 1, upper)]
    } else {
        Vec::new()
    };
    for (layer, cap) in caps {
        let mut wires = Vec::new();
        for (ring, uv) in cap.loops.iter().enumerate() {
            let mut coedges = Vec::new();
            for i in 0..uv.len() {
                let j = (i + 1) % uv.len();
                coedges.push(build.coedge(
                    ids[layer][ring][i],
                    ids[layer][ring][j],
                    prepared[layer][ring][i].clone(),
                    uv[i].clone(),
                ));
            }
            wires.push(build.wire(coedges));
        }
        build.face(cap.surface, wires[0], wires[1..].to_vec(), layer == 0);
    }
    let mut model = if periodic {
        let mut model = build.model;
        model.shells = shell_faces
            .into_iter()
            .map(|faces| Shell {
                faces,
                closed: true,
            })
            .collect();
        model.bodies.push(Body {
            outer_shell: 0,
            inner_shells: (1..loop_count).collect(),
        });
        model.rebuild_topology_ids();
        model.validate()?;
        model
    } else {
        build.finish()?
    };
    if direction < 0. {
        for face in model.shells.iter_mut().flat_map(|s| &mut s.faces) {
            face.reversed = !face.reversed;
        }
        model.validate()?;
    }
    Ok(model)
}
