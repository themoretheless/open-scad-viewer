use super::*;

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
