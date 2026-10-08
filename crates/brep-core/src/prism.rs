//! Exact extrusion of retained planar trim curves and conservative recognition
//! of XY prisms. Display meshes never participate in either operation.
use super::*;

pub const MAX_PROFILE_LOOPS: usize = 64;
pub const MAX_PROFILE_CURVES: usize = 254;
pub const MAX_PROFILE_CONTROLS: usize = 8192;
const TOLERANCE: f64 = 1e-7;

#[derive(Clone, Debug)]
pub struct ProfilePrism {
    pub loops: Vec<Vec<Curve>>,
    pub z_min: f64,
    pub z_max: f64,
}
fn unsupported(message: impl Into<String>) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}
fn normalized(curve: &Curve) -> Curve {
    let [a, b] = curve.domain();
    let mut result = curve.clone();
    result.knots = result.knots.iter().map(|k| (k - a) / (b - a)).collect();
    result
}
fn lifted(curve: &Curve, z: f64) -> Curve {
    let mut result = curve.clone();
    for p in &mut result.control_points {
        p.push(z);
    }
    result
}
fn empty_model() -> Model {
    Model(
        brep_topology::Model {
            vertices: vec![],
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: TOLERANCE,
        },
        TopologyIds::default(),
    )
}
fn push_edge(model: &mut Model, vertices: [usize; 2], curve: Curve) -> usize {
    let id = model.edges.len();
    model.edges.push(Edge {
        vertices,
        curve,
        degenerate: false,
    });
    id
}
fn push_loop(model: &mut Model, coedges: Vec<Coedge>) -> usize {
    let id = model.loops.len();
    model.loops.push(Loop { coedges });
    id
}
fn push_face(
    model: &mut Model,
    surface: Surface,
    outer: usize,
    holes: Vec<usize>,
    reversed: bool,
) -> FaceUse {
    let face = model.faces.len();
    model.faces.push(Face {
        surface,
        outer,
        holes,
    });
    FaceUse { face, reversed }
}
pub(crate) fn cap_surface(bounds: [[f64; 2]; 2], z: f64) -> Surface {
    let [a, b] = bounds;
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![a[0], a[1], z], vec![a[0], b[1], z]],
            vec![vec![b[0], a[1], z], vec![b[0], b[1], z]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}
pub(crate) fn cap_pcurve(curve: &Curve, bounds: [[f64; 2]; 2]) -> Curve {
    let mut result = curve.clone();
    for p in &mut result.control_points {
        for i in 0..2 {
            p[i] = (p[i] - bounds[0][i]) / (bounds[1][i] - bounds[0][i]);
        }
    }
    result
}
/// Each loop traverses with material on its left. Nested islands and disjoint
/// outer loops become separate bodies; holes remain cap trim loops. Curve
/// definitions are retained exactly up to affine knot-domain normalization.
/// General rational loops require bounded simplicity, separation and nesting
/// proofs. Unsupported or unproven containment is refused before construction.
pub fn extrude(loops: &[Vec<Curve>], z_min: f64, z_max: f64) -> Result<Model> {
    if !z_min.is_finite()
        || !z_max.is_finite()
        || z_min.abs() > 1e6
        || z_max.abs() > 1e6
        || z_max - z_min < 1e-5
    {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Prism Z limits must be finite within +/-1000000 mm with height at least 0.00001 mm",
        ));
    }
    let curve_count = loops.iter().map(Vec::len).sum::<usize>();
    let control_count = loops
        .iter()
        .flatten()
        .map(|c| c.control_points.len())
        .sum::<usize>();
    if loops.len() > MAX_PROFILE_LOOPS
        || curve_count > MAX_PROFILE_CURVES
        || control_count > MAX_PROFILE_CONTROLS
    {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Prism profile exceeds 64 loops, 254 curves or 8192 control points",
        ));
    }
    if loops.is_empty() {
        return Ok(empty_model());
    }
    let mut profile = Vec::with_capacity(loops.len());
    for wire in loops {
        if wire.is_empty() {
            return Err(unsupported("Empty prism profile loop"));
        }
        let mut normalized_wire = Vec::new();
        for curve in wire {
            curve.validate()?;
            if curve
                .control_points
                .iter()
                .any(|p| p.len() != 2 || p.iter().any(|x| x.abs() > 1e6))
            {
                return Err(Error::new(
                    "BREP_INVALID_SIZE",
                    "Prism profile requires 2D control points within +/-1000000 mm",
                ));
            }
            // Every active knot span owns a separate boundary edge. A closed
            // multi-span circle must not collapse to one display chord at LOD1.
            let [start, end] = curve.domain();
            let mut breaks: Vec<_> = curve
                .knots
                .iter()
                .copied()
                .filter(|&t| t >= start && t <= end)
                .collect();
            breaks.dedup();
            for pair in breaks.windows(2) {
                normalized_wire.push(normalized(&curve.trim(pair[0], pair[1])?));
            }
            if normalized_wire.len() > MAX_PROFILE_CURVES {
                return Err(Error::new(
                    "BREP_RESOURCE_LIMIT",
                    "Prism profile exceeds 254 active curve spans",
                ));
            }
        }
        profile.push(normalized_wire);
    }
    let curve_count = profile.iter().map(Vec::len).sum::<usize>();
    if curve_count > MAX_PROFILE_CURVES {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Prism profile exceeds 254 active curve spans",
        ));
    }
    let components = crate::profile_region::components(&profile, TOLERANCE)?;
    if curve_count + 2 * components.len() > 256 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Prism exceeds 256 faces including caps",
        ));
    }
    let mut model = empty_model();
    for (outer, holes) in components {
        let members: Vec<_> = std::iter::once(outer).chain(holes).collect();
        let mut bounds = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
        for p in members
            .iter()
            .flat_map(|&i| profile[i].iter())
            .flat_map(|c| &c.control_points)
        {
            for k in 0..2 {
                bounds[0][k] = bounds[0][k].min(p[k]);
                bounds[1][k] = bounds[1][k].max(p[k]);
            }
        }
        if (0..2).any(|i| bounds[1][i] <= bounds[0][i]) {
            return Err(unsupported("Prism profile has degenerate bounds"));
        }
        let mut face_uses = Vec::new();
        let mut bottom_loops = Vec::new();
        let mut top_loops = Vec::new();
        for member in members {
            let wire = &profile[member];
            let mut bottom = Vec::new();
            let mut top = Vec::new();
            for curve in wire {
                let p = curve.evaluate(0.)?.point;
                bottom.push(model.vertices.len());
                model.vertices.push(Vertex {
                    point: [p[0], p[1], z_min],
                });
                top.push(model.vertices.len());
                model.vertices.push(Vertex {
                    point: [p[0], p[1], z_max],
                });
            }
            let mut low_edges = Vec::new();
            let mut high_edges = Vec::new();
            let mut vertical = Vec::new();
            for (i, curve) in wire.iter().enumerate() {
                let j = (i + 1) % wire.len();
                low_edges.push(push_edge(
                    &mut model,
                    [bottom[i], bottom[j]],
                    lifted(curve, z_min),
                ));
                high_edges.push(push_edge(
                    &mut model,
                    [top[i], top[j]],
                    lifted(curve, z_max),
                ));
                let a = model.vertices[bottom[i]].point.to_vec();
                let b = model.vertices[top[i]].point.to_vec();
                vertical.push(push_edge(&mut model, [bottom[i], top[i]], line(a, b)));
            }
            let bottom_wire = wire
                .iter()
                .enumerate()
                .map(|(i, c)| Coedge {
                    edge: low_edges[i],
                    reversed: false,
                    pcurve: cap_pcurve(c, bounds),
                })
                .collect();
            let top_wire = wire
                .iter()
                .enumerate()
                .map(|(i, c)| Coedge {
                    edge: high_edges[i],
                    reversed: false,
                    pcurve: cap_pcurve(c, bounds),
                })
                .collect();
            bottom_loops.push(push_loop(&mut model, bottom_wire));
            top_loops.push(push_loop(&mut model, top_wire));
            for (i, curve) in wire.iter().enumerate() {
                let j = (i + 1) % wire.len();
                let outer = push_loop(
                    &mut model,
                    vec![
                        Coedge {
                            edge: low_edges[i],
                            reversed: false,
                            pcurve: line(vec![0., 0.], vec![1., 0.]),
                        },
                        Coedge {
                            edge: vertical[j],
                            reversed: false,
                            pcurve: line(vec![1., 0.], vec![1., 1.]),
                        },
                        Coedge {
                            edge: high_edges[i],
                            reversed: true,
                            pcurve: line(vec![1., 1.], vec![0., 1.]),
                        },
                        Coedge {
                            edge: vertical[i],
                            reversed: true,
                            pcurve: line(vec![0., 1.], vec![0., 0.]),
                        },
                    ],
                );
                let surface = Surface {
                    degree_u: curve.degree,
                    degree_v: 1,
                    knots_u: curve.knots.clone(),
                    knots_v: vec![0., 0., 1., 1.],
                    control_points: curve
                        .control_points
                        .iter()
                        .map(|p| vec![vec![p[0], p[1], z_min], vec![p[0], p[1], z_max]])
                        .collect(),
                    weights: curve.weights.iter().map(|w| vec![*w, *w]).collect(),
                    periodic_u: curve.periodic,
                    periodic_v: false,
                };
                face_uses.push(push_face(&mut model, surface, outer, vec![], false));
            }
        }
        face_uses.push(push_face(
            &mut model,
            cap_surface(bounds, z_min),
            bottom_loops[0],
            bottom_loops[1..].to_vec(),
            true,
        ));
        face_uses.push(push_face(
            &mut model,
            cap_surface(bounds, z_max),
            top_loops[0],
            top_loops[1..].to_vec(),
            false,
        ));
        let shell = model.shells.len();
        model.shells.push(Shell {
            faces: face_uses,
            closed: true,
        });
        model.bodies.push(Body {
            outer_shell: shell,
            inner_shells: vec![],
        });
    }
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}

/// Ruled loft from a retained profile to its uniformly scaled, translated copy.
/// Every intermediate section is an affine image with strictly positive scale,
/// so it preserves the admitted profile's components, holes and orientation.
/// This is not the general correspondence problem for independent profiles.
pub fn loft_scaled(
    loops: &[Vec<Curve>], z_min: f64, z_max: f64, scale: f64, offset: [f64; 2],
) -> Result<Model> {
    if !scale.is_finite() || scale <= 0. || !offset.iter().all(|x| x.is_finite()) {
        return Err(Error::new("BREP_INVALID_SIZE", "Loft requires a finite positive scale and finite offset"));
    }
    let mut model = extrude(loops, z_min, z_max)?;
    let map = |p: &mut [f64]| -> Result<()> {
        let t = (p[2]-z_min)/(z_max-z_min);
        let factor = 1.+t*(scale-1.);
        for i in 0..2 { p[i] = factor*p[i]+t*offset[i]; }
        if p.iter().any(|x| !x.is_finite() || x.abs()>1e6) {
            return Err(Error::new("BREP_INVALID_SIZE", "Loft control coordinates exceed +/-1000000 mm"));
        }
        Ok(())
    };
    for vertex in &mut model.vertices { map(&mut vertex.point)?; }
    for edge in &mut model.edges {
        for point in &mut edge.curve.control_points { map(point)?; }
    }
    for face in &mut model.faces {
        for row in &mut face.surface.control_points {
            for point in row { map(point)?; }
        }
    }
    model.validate()?;
    Ok(model)
}

fn roundoff_equal(a: f64, b: f64) -> bool {
    roundoff_equal_at(a, b, a.abs().max(b.abs()))
}
/// Roundoff-level equality where the error budget follows `scale`, the
/// magnitude of the quantities the compared values were derived from.
fn roundoff_equal_at(a: f64, b: f64, scale: f64) -> bool {
    (a - b).abs() <= 32. * f64::EPSILON * scale.max(1.)
}
fn same_curve(a: &Curve, b: &Curve) -> bool {
    let a = normalized(a);
    let b = normalized(b);
    // Control points are the outcome of rigid placements of the whole curve;
    // each coordinate carries roundoff proportional to the curve's extent,
    // not to its own possibly near-zero magnitude.
    let extent = a
        .control_points
        .iter()
        .chain(&b.control_points)
        .flatten()
        .fold(0., |m: f64, x| m.max(x.abs()));
    a.degree == b.degree
        && a.periodic == b.periodic
        && a.knots.len() == b.knots.len()
        && a.control_points.len() == b.control_points.len()
        && a.knots
            .iter()
            .zip(&b.knots)
            .all(|(x, y)| roundoff_equal(*x, *y))
        && a.weights
            .iter()
            .zip(&b.weights)
            .all(|(x, y)| roundoff_equal(*x / a.weights[0], *y / b.weights[0]))
        && a.control_points
            .iter()
            .zip(&b.control_points)
            .all(|(p, q)| {
                p.len() == q.len()
                    && p.iter()
                        .zip(q)
                        .all(|(x, y)| roundoff_equal_at(*x, *y, extent))
            })
}
pub(crate) fn directed_edge(model: &Model, c: &Coedge) -> Result<Curve> {
    let curve = &model.edges[c.edge].curve;
    Ok(normalized(&if c.reversed {
        curve.reverse()?
    } else {
        curve.clone()
    }))
}
fn surface_domain(surface: &Surface) -> [[f64; 2]; 2] {
    [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ]
}
pub(crate) fn planar_cap_z(surface: &Surface) -> Option<f64> {
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.control_points.len() != 2
        || surface.control_points[0].len() != 2
        || surface.periodic_u
        || surface.periodic_v
        || surface
            .weights
            .iter()
            .flatten()
            .any(|w| *w != surface.weights[0][0])
    {
        return None;
    }
    let z = surface.control_points[0][0][2];
    if surface.control_points.iter().flatten().any(|p| p[2] != z) {
        return None;
    }
    let a = &surface.control_points[0][0];
    let b = &surface.control_points[1][0];
    let c = &surface.control_points[0][1];
    let d = &surface.control_points[1][1];
    if (0..3).any(|i| !roundoff_equal(a[i] + d[i], b[i] + c[i])) {
        return None;
    }
    Some(z)
}
fn cap_is_proven(model: &Model, usage: &FaceUse, z_min: f64, z_max: f64) -> Result<bool> {
    let Some(z) = planar_cap_z(&model.faces[usage.face].surface) else {
        return Ok(false);
    };
    if z != z_min && z != z_max {
        return Ok(false);
    }
    cap_plane_is_proven(model, usage, z, z == z_max)
}
/// Full affine cap/carrier agreement with a specified physical orientation.
/// Callers must validate the model before using these indexed proof helpers.
pub(crate) fn cap_plane_is_proven(
    model: &Model,
    usage: &FaceUse,
    z: f64,
    upward: bool,
) -> Result<bool> {
    let face = &model.faces[usage.face];
    let surface = &face.surface;
    if planar_cap_z(surface) != Some(z) {
        return Ok(false);
    }
    let a = &surface.control_points[0][0];
    let b = &surface.control_points[1][0];
    let c = &surface.control_points[0][1];
    let orientation = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let physical_orientation = orientation * if usage.reversed { -1. } else { 1. };
    if if upward {
        physical_orientation <= 0.
    } else {
        physical_orientation >= 0.
    } {
        return Ok(false);
    }
    let domain = surface_domain(surface);
    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
        for coedge in &model.loops[wire].coedges {
            let edge = &model.edges[coedge.edge];
            if edge.degenerate || edge.curve.control_points.iter().any(|p| p[2] != z) {
                return Ok(false);
            }
            // Affine substitution in the complete rational pcurve control net
            // proves carrier agreement, including portions between samples.
            let mut mapped = coedge.pcurve.clone();
            for p in &mut mapped.control_points {
                let u = (p[0] - domain[0][0]) / (domain[0][1] - domain[0][0]);
                let v = (p[1] - domain[1][0]) / (domain[1][1] - domain[1][0]);
                *p = (0..3)
                    .map(|i| a[i] + u * (b[i] - a[i]) + v * (c[i] - a[i]))
                    .collect();
            }
            if !same_curve(&mapped, &directed_edge(model, coedge)?) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
pub(crate) fn side_is_proven(
    model: &Model,
    usage: &FaceUse,
    z_min: f64,
    z_max: f64,
) -> Result<bool> {
    use nurbs_core::surface::Axis;
    let face = &model.faces[usage.face];
    let surface = &face.surface;
    if !face.holes.is_empty() || model.loops[face.outer].coedges.len() != 4 {
        return Ok(false);
    }
    let extrusion_axis = (0..2).find(|&axis| {
        let (degree, count, periodic) = if axis == 0 {
            (
                surface.degree_u,
                surface.control_points.len(),
                surface.periodic_u,
            )
        } else {
            (
                surface.degree_v,
                surface.control_points[0].len(),
                surface.periodic_v,
            )
        };
        if degree != 1 || count != 2 || periodic {
            return false;
        }
        let profile_count = if axis == 0 {
            surface.control_points[0].len()
        } else {
            surface.control_points.len()
        };
        (0..profile_count).all(|i| {
            let (a, b, wa, wb) = if axis == 0 {
                (
                    &surface.control_points[0][i],
                    &surface.control_points[1][i],
                    surface.weights[0][i],
                    surface.weights[1][i],
                )
            } else {
                (
                    &surface.control_points[i][0],
                    &surface.control_points[i][1],
                    surface.weights[i][0],
                    surface.weights[i][1],
                )
            };
            a[0] == b[0]
                && a[1] == b[1]
                && wa == wb
                && ((a[2] == z_min && b[2] == z_max) || (a[2] == z_max && b[2] == z_min))
        })
    });
    let Some(axis) = extrusion_axis else {
        return Ok(false);
    };
    // Every extrusion column must use the same Z direction; otherwise the
    // control net contains a fold or twist rather than a prism side.
    let first_direction = surface.control_points[0][0][2];
    if if axis == 0 {
        surface.control_points[0]
            .iter()
            .any(|p| p[2] != first_direction)
    } else {
        surface
            .control_points
            .iter()
            .any(|r| r[0][2] != first_direction)
    } {
        return Ok(false);
    }
    let carrier_domain = surface_domain(surface);
    // STEP may retain a complete cylindrical carrier for a rectangular trim
    // spanning only part of its profile direction. Prove that trim rectangle
    // against exact restricted isocurves, not the whole carrier boundaries.
    let mut domain = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
    for coedge in &model.loops[face.outer].coedges {
        for p in &coedge.pcurve.control_points {
            for i in 0..2 {
                domain[i][0] = domain[i][0].min(p[i]);
                domain[i][1] = domain[i][1].max(p[i]);
            }
        }
    }
    if (0..2).any(|i| {
        domain[i][0] >= domain[i][1]
            || domain[i][0] < carrier_domain[i][0]
            || domain[i][1] > carrier_domain[i][1]
    }) || domain[axis] != carrier_domain[axis]
    {
        return Ok(false);
    }
    let mut boundaries = BTreeSet::new();
    let mut horizontal = 0;
    for coedge in &model.loops[face.outer].coedges {
        let pcurve = &coedge.pcurve;
        if pcurve.degree != 1
            || pcurve.control_points.len() != 2
            || pcurve.weights[0] != pcurve.weights[1]
            || pcurve.periodic
        {
            return Ok(false);
        }
        let a = &pcurve.control_points[0];
        let b = &pcurve.control_points[1];
        let constant = if a[0] == b[0] {
            0
        } else if a[1] == b[1] {
            1
        } else {
            return Ok(false);
        };
        let varying = 1 - constant;
        let side = if a[constant] == domain[constant][0] {
            0
        } else if a[constant] == domain[constant][1] {
            1
        } else {
            return Ok(false);
        };
        if !((a[varying] == domain[varying][0] && b[varying] == domain[varying][1])
            || (a[varying] == domain[varying][1] && b[varying] == domain[varying][0]))
        {
            return Ok(false);
        }
        if !boundaries.insert((constant, side)) {
            return Ok(false);
        }
        let mut iso = surface.iso(if constant == 0 { Axis::U } else { Axis::V }, a[constant])?;
        if domain[varying] != carrier_domain[varying] {
            iso = iso.trim(domain[varying][0], domain[varying][1])?;
        }
        if a[varying] > b[varying] {
            iso = iso.reverse()?;
        }
        if model.edges[coedge.edge].degenerate || !same_curve(&iso, &directed_edge(model, coedge)?)
        {
            return Ok(false);
        }
        if constant == axis {
            horizontal += 1;
        }
    }
    Ok(horizontal == 2 && boundaries.len() == 4)
}
/// Recognize only when every retained cap and side definition proves an XY
/// extrusion. Deformed control nets and merely similar display meshes do not
/// qualify. Equivalent but structurally different parameterizations may return
/// None; no geometric approximation is used to force a positive recognition.
pub fn recognize(model: &Model) -> Result<Option<ProfilePrism>> {
    model.validate()?;
    if model.vertices.is_empty()
        || model.bodies.is_empty()
        || model.bodies.len() != model.shells.len()
        || model.bodies.iter().any(|b| !b.inner_shells.is_empty())
    {
        return Ok(None);
    }
    let z_min = model
        .vertices
        .iter()
        .map(|v| v.point[2])
        .fold(f64::INFINITY, f64::min);
    let z_max = model
        .vertices
        .iter()
        .map(|v| v.point[2])
        .fold(f64::NEG_INFINITY, f64::max);
    if z_max <= z_min
        || model
            .vertices
            .iter()
            .any(|v| v.point[2] != z_min && v.point[2] != z_max)
    {
        return Ok(None);
    }
    let mut loops = Vec::new();
    let mut cap_edges = BTreeMap::<usize, usize>::new();
    let mut side_edges = BTreeMap::<usize, usize>::new();
    for shell in &model.shells {
        if !shell.closed {
            return Ok(None);
        }
        let mut bottom_count = 0;
        let mut top_count = 0;
        for usage in &shell.faces {
            let face = &model.faces[usage.face];
            if let Some(z) = planar_cap_z(&face.surface) {
                if !cap_is_proven(model, usage, z_min, z_max)? {
                    return Ok(None);
                }
                if z == z_min {
                    bottom_count += 1;
                } else {
                    top_count += 1;
                }
                for &wire in std::iter::once(&face.outer).chain(&face.holes) {
                    for c in &model.loops[wire].coedges {
                        *cap_edges.entry(c.edge).or_default() += 1;
                    }
                    if z == z_max {
                        let mut profile = Vec::new();
                        let mut coedges: Vec<_> = model.loops[wire].coedges.iter().collect();
                        if usage.reversed {
                            coedges.reverse();
                        }
                        for c in coedges {
                            let mut curve = directed_edge(model, c)?;
                            if usage.reversed {
                                curve = curve.reverse()?;
                            }
                            for p in &mut curve.control_points {
                                p.pop();
                            }
                            profile.push(curve);
                        }
                        loops.push(profile);
                    }
                }
            } else {
                if !side_is_proven(model, usage, z_min, z_max)? {
                    return Ok(None);
                }
                for c in &model.loops[face.outer].coedges {
                    *side_edges.entry(c.edge).or_default() += 1;
                }
            }
        }
        if bottom_count != 1 || top_count != 1 {
            return Ok(None);
        }
    }
    // Cap rims must each be shared with exactly one proven ruled side; other
    // side edges must join two sides. This also rejects hidden extra surfaces.
    for edge in 0..model.edges.len() {
        let caps = cap_edges.get(&edge).copied().unwrap_or(0);
        let sides = side_edges.get(&edge).copied().unwrap_or(0);
        if !((caps == 1 && sides == 1) || (caps == 0 && sides == 2)) {
            return Ok(None);
        }
    }
    Ok(Some(ProfilePrism {
        loops,
        z_min,
        z_max,
    }))
}

#[cfg(test)]
mod imported_cap_tests {
    use super::*;
    #[test]
#[cfg(feature = "codec")]
    fn scaled_retained_loft_preserves_holes_and_rational_sections() {
        let mut hole=crate::sketch::circle_wire(1.).unwrap();
        hole=hole.iter().rev().map(Curve::reverse).collect::<nurbs_core::Result<Vec<_>>>().unwrap();
        let loops=vec![crate::sketch::circle_wire(3.).unwrap(),hole];
        let before=value_codec::to_value(&loops).unwrap();
        let loft=loft_scaled(&loops,0.,10.,2.,[5.,7.]).unwrap();
        assert_eq!(loft.bodies.len(),1);
        assert_eq!(loft.faces.iter().filter(|face|face.holes.len()==1).count(),2);
        assert_eq!(value_codec::to_value(&loops).unwrap(),before);
        let side=&loft.faces[0].surface;
        let point=side.evaluate(0.37,0.5).unwrap().point;
        let source=loops[0][0].evaluate(0.37).unwrap().point;
        assert!((point[0]-(1.5*source[0]+2.5)).abs()<1e-12);
        assert!((point[1]-(1.5*source[1]+3.5)).abs()<1e-12);
        assert!((point[2]-5.).abs()<1e-12);
        assert!(side.weights.iter().flatten().any(|w| *w!=1.));
        let (step,_,_)=crate::step_interchange_v3::export_step_v9(&loft).unwrap();
        let (restored,_,_)=crate::step_interchange_v3::import_step_v9(&step).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.bodies.len(),1);
        assert_eq!(restored.faces.iter().filter(|face|face.holes.len()==1).count(),2);
        for scale in [0.,-1.,f64::NAN,f64::INFINITY] {
            assert!(loft_scaled(&loops,0.,10.,scale,[0.,0.]).is_err());
        }
        assert!(loft_scaled(&loops,0.,10.,2.,[1e6,0.]).is_err());
    }

    #[test]
    #[cfg(feature = "codec")]
    fn recognizes_trimmed_step_carriers_without_admitting_deformed_sides() {
        let model: Model = value_codec::from_str(include_str!(
            "../../../docs/qualification/cad-roadmap-2026-09-28/parts-history/imported-flange.json"
        ))
        .unwrap();
        let recognized = recognize(&model).unwrap().unwrap();
        assert_eq!(
            (recognized.z_min, recognized.z_max, recognized.loops.len()),
            (0., 7., 2)
        );
        let usage = model.shells[0]
            .faces
            .iter()
            .find(|u| model.faces[u.face].surface.control_points.len() == 9)
            .unwrap()
            .clone();
        assert!(side_is_proven(&model, &usage, 0., 7.).unwrap());
        let mut deformed = model.clone();
        deformed.faces[usage.face].surface.control_points[1][1][0] += 0.01;
        assert!(!side_is_proven(&deformed, &usage, 0., 7.).unwrap());
        let mut trimmed = model.clone();
        let wire = trimmed.faces[usage.face].outer;
        trimmed.loops[wire].coedges[0].pcurve.control_points[0][0] += 0.01;
        assert!(!side_is_proven(&trimmed, &usage, 0., 7.).unwrap());
        let top = model
            .faces
            .iter()
            .position(|f| planar_cap_z(&f.surface) == Some(7.))
            .unwrap();
        let result = crate::operations::push_planar_face(&model, top, 1.).unwrap();
        let volume = crate::analysis::mass_properties(&result, 1e-7, 200_000)
            .unwrap()
            .signed_volume_mm3;
        assert!((volume - 3000. * std::f64::consts::PI).abs() < 1e-5);
    }
}
