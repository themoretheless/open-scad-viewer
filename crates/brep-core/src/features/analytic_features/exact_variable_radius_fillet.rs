use super::*;

const EXACT_FILLET_REFUSED: &str = "BREP_EXACT_FILLET_REFUSED";
/// Edge-direction x-component above which the frame helper switches to +Y.
const PRISM_HELPER_SWITCH: f64 = 0.8;
/// Prism recognition tolerance as a multiple of the model tolerance.
const PRISM_TOLERANCE_FACTOR: f64 = 16.;

/// Orthonormal prism frame (w along the extrusion) and its measured caps.
struct PrismFrame {
    u: [f64; 3],
    v: [f64; 3],
    w: [f64; 3],
    local_vertices: Vec<[f64; 3]>,
    z0: f64,
    z1: f64,
    tol: f64,
}

impl PrismFrame {
    fn to_local(&self) -> [[f64; 4]; 4] {
        let (u, v, w) = (self.u, self.v, self.w);
        [
            [u[0], u[1], u[2], 0.],
            [v[0], v[1], v[2], 0.],
            [w[0], w[1], w[2], 0.],
            [0., 0., 0., 1.],
        ]
    }

    fn to_world(&self) -> [[f64; 4]; 4] {
        let (u, v, w) = (self.u, self.v, self.w);
        [
            [u[0], v[0], w[0], 0.],
            [u[1], v[1], w[1], 0.],
            [u[2], v[2], w[2], 0.],
            [0., 0., 0., 1.],
        ]
    }
}

pub(super) fn exact_prism_fillet(
    model: &Model,
    edges: &[usize],
    radius: f64,
    require_convex: bool,
) -> Result<AuditedFeatureResult> {
    audit_solid(model).map_err(|_| {
        refuse(
            EXACT_FILLET_REFUSED,
            "Fillet source must pass the global solid audit",
        )
    })?;
    if edges.is_empty()
        || !(radius.is_finite() && radius > 0.)
        || model.edges.iter().any(|edge| edge.curve.degree != 1)
        || model
            .faces
            .iter()
            .any(|face| face.surface.degree_u != 1 || face.surface.degree_v != 1)
    {
        return Err(refuse(
            EXACT_FILLET_REFUSED,
            "Exact prism fillet requires selected straight edges, planar faces, and positive constant radius",
        ));
    }
    let selected: std::collections::BTreeSet<_> = edges.iter().copied().collect();
    if selected.len() != edges.len() || selected.iter().any(|edge| *edge >= model.edges.len()) {
        return Err(refuse(
            EXACT_FILLET_REFUSED,
            "Selected fillet edges must be unique authored edges",
        ));
    }
    let frame = prism_frame(model, edges[0])?;
    let profile = matched_prism_profile(model, &frame, require_convex)?;
    let rounded = rounded_longitudinal_edges(model, &frame, &profile, &selected)?;
    let local_source = crate::transform::affine(model, frame.to_local())?;
    let local_result = crate::imprint_pipeline::rounded_convex_prism_edges(
        &local_source,
        &profile,
        &rounded,
        radius,
        frame.z0,
        frame.z1,
    )
    .map_err(|error| {
        refuse(
            EXACT_FILLET_REFUSED,
            &format!("Exact prism fillet feasibility refused: {}", error.message),
        )
    })?;
    let result = crate::transform::affine(&local_result, frame.to_world())?;
    certify_blend_result(
        result,
        if require_convex {
            EXACT_CONVEX_PRISM_FILLET_CAPABILITY
        } else {
            EXACT_SIMPLE_PRISM_FILLET_CAPABILITY
        },
        vec![
            "exact_positive_weight_circular_profile",
            "exact_cylindrical_longitudinal_patch",
            "mixed_selected_unselected_profile_vertices",
            "open_or_closed_profile_selection",
            "no_mesh_fallback",
        ],
        radius,
        EXACT_FILLET_REFUSED,
    )
}

/// Frame along the first selected edge; every vertex must lie on one of two caps.
fn prism_frame(model: &Model, first_edge: usize) -> Result<PrismFrame> {
    let collapsed = || refuse(EXACT_FILLET_REFUSED, "Selected fillet edge is collapsed");
    let first = &model.edges[first_edge];
    let p0 = model.vertices[first.vertices[0]].point;
    let p1 = model.vertices[first.vertices[1]].point;
    let w = unit3(sub3(p1, p0)).ok_or_else(collapsed)?;
    let helper = if w[0].abs() < PRISM_HELPER_SWITCH {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let u = unit3(cross3(helper, w)).ok_or_else(collapsed)?;
    let v = cross3(w, u);
    let local_vertices: Vec<_> = model
        .vertices
        .iter()
        .map(|vertex| {
            [
                dot3(vertex.point, u),
                dot3(vertex.point, v),
                dot3(vertex.point, w),
            ]
        })
        .collect();
    let z0 = local_vertices
        .iter()
        .map(|point| point[2])
        .fold(f64::INFINITY, f64::min);
    let z1 = local_vertices
        .iter()
        .map(|point| point[2])
        .fold(f64::NEG_INFINITY, f64::max);
    let tol = model.tolerance_mm * PRISM_TOLERANCE_FACTOR;
    if z1 - z0 <= tol
        || local_vertices
            .iter()
            .any(|point| (point[2] - z0).abs() > tol && (point[2] - z1).abs() > tol)
    {
        return Err(refuse(
            EXACT_FILLET_REFUSED,
            "Selected edges do not define a two-cap planar prism",
        ));
    }
    Ok(PrismFrame {
        u,
        v,
        w,
        local_vertices,
        z0,
        z1,
        tol,
    })
}

/// Bottom cap profile, checked against the top cap and (optionally) convexity.
fn matched_prism_profile(
    model: &Model,
    frame: &PrismFrame,
    require_convex: bool,
) -> Result<Vec<[f64; 2]>> {
    let tol = frame.tol;
    let profile = ordered_prism_cap_profile(model, &frame.local_vertices, frame.z0, tol)?;
    let top = ordered_prism_cap_profile(model, &frame.local_vertices, frame.z1, tol)?;
    if top.len() != profile.len()
        || top.iter().any(|a| {
            !profile
                .iter()
                .any(|b| (a[0] - b[0]).hypot(a[1] - b[1]) <= tol)
        })
    {
        return Err(refuse(
            EXACT_FILLET_REFUSED,
            "Prism caps must have matching projected boundaries",
        ));
    }
    if profile.len() < 3
        || require_convex
            && profile.iter().enumerate().any(|(i, a)| {
                let b = profile[(i + 1) % profile.len()];
                let c = profile[(i + 2) % profile.len()];
                (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]) <= tol
            })
    {
        return Err(refuse(
            EXACT_FILLET_REFUSED,
            "Prism profile must be strictly convex",
        ));
    }
    Ok(profile)
}

/// Per-profile-vertex flag: is its longitudinal edge selected.
fn rounded_longitudinal_edges(
    model: &Model,
    frame: &PrismFrame,
    profile: &[[f64; 2]],
    selected: &std::collections::BTreeSet<usize>,
) -> Result<Vec<bool>> {
    let (z0, z1, tol) = (frame.z0, frame.z1, frame.tol);
    let mut longitudinal = vec![None; profile.len()];
    for (edge_id, edge) in model.edges.iter().enumerate() {
        let [a, b] = edge.vertices.map(|vertex| frame.local_vertices[vertex]);
        let spans_caps = ((a[2] - z0).abs() <= tol && (b[2] - z1).abs() <= tol)
            || ((b[2] - z0).abs() <= tol && (a[2] - z1).abs() <= tol);
        if (a[0] - b[0]).hypot(a[1] - b[1]) > tol || !spans_caps {
            continue;
        }
        let index = profile
            .iter()
            .position(|p| (p[0] - a[0]).hypot(p[1] - a[1]) <= tol)
            .ok_or_else(|| {
                refuse(
                    EXACT_FILLET_REFUSED,
                    "Longitudinal edge does not correspond to the convex cap profile",
                )
            })?;
        longitudinal[index] = Some(edge_id);
    }
    let transition_refused = || {
        refuse(
            EXACT_FILLET_REFUSED,
            "Cap edges and valence-3 corner chains require an unavailable exact transition proof",
        )
    };
    let longitudinal: Vec<usize> = longitudinal
        .into_iter()
        .collect::<Option<_>>()
        .ok_or_else(transition_refused)?;
    if selected.iter().any(|edge| !longitudinal.contains(edge)) {
        return Err(transition_refused());
    }
    Ok(longitudinal
        .iter()
        .map(|edge| selected.contains(edge))
        .collect())
}

/// Exact linear cross-section radius law on one edge of an audited cuboid.
/// Endpoint radii must be positive, unequal (constant radius stays on
/// the constant-fillet capabilities), and within the profile collision bound.
/// Multi-edge / valence-3 networks remain typed-refused.
pub fn exact_variable_radius_fillet(
    model: &Model,
    edges: &[usize],
    radii: &[[f64; 2]],
) -> Result<AuditedFeatureResult> {
    model.validate()?;
    let refusal = || {
        refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Variable-radius fillet requires one straight edge of an orthogonal planar cuboid",
        )
    };
    if edges.len() != 1 || radii.len() != 1 || edges[0] >= model.edges.len() {
        return Err(refusal());
    }
    let selected = &model.edges[edges[0]];
    let a = model.vertices[selected.vertices[0]].point;
    let b = model.vertices[selected.vertices[1]].point;
    if is_axis_aligned_cuboid(model)
        && (a[0] - b[0]).abs() <= 1e-12
        && (a[1] - b[1]).abs() <= 1e-12
        && (a[2] - b[2]).abs() > 1e-12
    {
        return exact_axis_aligned_variable_radius_fillet(model, edges, radii);
    }
    let vertex = selected.vertices[0];
    let adjacent: Vec<_> = model
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| (i != edges[0] && e.vertices.contains(&vertex)).then_some(i))
        .collect();
    if adjacent.len() != 2 {
        return Err(refusal());
    }
    let mut axes = [[0.; 3]; 3];
    for (axis, edge_id) in [adjacent[0], adjacent[1], edges[0]].into_iter().enumerate() {
        let edge = &model.edges[edge_id];
        if edge.curve.degree != 1 {
            return Err(refusal());
        }
        let other = edge
            .vertices
            .iter()
            .copied()
            .find(|&v| v != vertex)
            .ok_or_else(refusal)?;
        let delta: [f64; 3] = std::array::from_fn(|j| model.vertices[other].point[j] - a[j]);
        let length = delta[0].hypot(delta[1]).hypot(delta[2]);
        if !length.is_finite() || length <= 1e-12 {
            return Err(refusal());
        }
        axes[axis] = delta.map(|v| v / length);
    }
    for i in 0..3 {
        for j in 0..i {
            if (0..3).map(|k| axes[i][k] * axes[j][k]).sum::<f64>().abs() > 1e-12 {
                return Err(refusal());
            }
        }
    }
    let mut to_local = [[0.; 4]; 4];
    let mut to_world = [[0.; 4]; 4];
    to_local[3][3] = 1.;
    to_world[3][3] = 1.;
    for i in 0..3 {
        for j in 0..3 {
            to_local[i][j] = axes[i][j];
            to_world[i][j] = axes[j][i];
        }
        to_local[i][3] = -(0..3).map(|j| axes[i][j] * a[j]).sum::<f64>();
        to_world[i][3] = a[i];
    }
    // Recover a coordinate frame, then recognize and audit the entire source.
    // Three orthogonal incident edges alone do not qualify a cuboid.
    let local = crate::transform::affine(model, to_local)?;
    let result = exact_axis_aligned_variable_radius_fillet(&local, edges, radii)?;
    let world = crate::transform::affine(&result.model, to_world)?;
    certify_blend_result(
        world,
        EXACT_VARIABLE_RADIUS_FILLET_CAPABILITY,
        vec![
            "exact_linear_radius_law",
            "rational_conical_fillet_face",
            "no_constant_radius_substitution",
            "no_valence3_corner",
            "rigid_cuboid_placement",
        ],
        radii[0][0].max(radii[0][1]),
        "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
    )
}

const VARIABLE_RADIUS_REFUSED: &str = "BREP_VARIABLE_RADIUS_FILLET_REFUSED";
/// Absolute coordinate tolerance for recognizing a vertical edge.
const VERTICAL_EDGE_TOLERANCE: f64 = 1e-12;

pub(super) fn exact_axis_aligned_variable_radius_fillet(
    model: &Model,
    edges: &[usize],
    radii: &[[f64; 2]],
) -> Result<AuditedFeatureResult> {
    let ([r0, r1], a, b) = vertical_variable_radius_edge(model, edges, radii)?;
    let (min, max) = model_bounds(model);
    let rmax = r0.max(r1);
    if max[0] - min[0] <= 2. * rmax || max[1] - min[1] <= 2. * rmax {
        return Err(refuse(
            VARIABLE_RADIUS_REFUSED,
            "Cuboid extents too small for the endpoint radii",
        ));
    }
    let height = (a[2] - b[2]).abs();
    if !(height.is_finite() && height > rmax * 2.) {
        return Err(refuse(
            VARIABLE_RADIUS_REFUSED,
            "Edge too short for the requested endpoint radii",
        ));
    }
    let x = (a[0] + b[0]) * 0.5;
    let y = (a[1] + b[1]) * 0.5;
    let at_max_x = (x - max[0]).abs() <= (x - min[0]).abs();
    let at_max_y = (y - max[1]).abs() <= (y - min[1]).abs();
    let corner = match (at_max_x, at_max_y) {
        (false, false) => 0,
        (true, false) => 1,
        (true, true) => 2,
        (false, true) => 3,
    };
    let mut rounded = [false; 4];
    rounded[corner] = true;
    // radii[0] applies at vertices[0], radii[1] at vertices[1]; map to z-order.
    let (radius_bottom, radius_top) = if a[2] <= b[2] { (r0, r1) } else { (r1, r0) };
    let result = crate::imprint_pipeline::variable_radius_cuboid_vertical_edges(
        model,
        min,
        max,
        rounded,
        radius_bottom,
        radius_top,
    )
    .map_err(|error| {
        refuse(
            VARIABLE_RADIUS_REFUSED,
            &format!(
                "Variable-radius fillet authorship refused: {}",
                error.message
            ),
        )
    })?;
    let has_cone = result.faces.iter().any(|face| {
        face.surface.degree_u > 1
            && face.surface.degree_v == 1
            && face.surface.control_points.iter().any(|row| {
                row.len() == 2
                    && (row[0][0] - row[1][0]).hypot(row[0][1] - row[1][1]) > model.tolerance_mm
            })
    });
    if !has_cone {
        return Err(refuse(
            VARIABLE_RADIUS_REFUSED,
            "Variable-radius result missing conical fillet face",
        ));
    }
    certify_blend_result(
        result,
        EXACT_VARIABLE_RADIUS_FILLET_CAPABILITY,
        vec![
            "exact_linear_radius_law",
            "rational_conical_fillet_face",
            "no_constant_radius_substitution",
            "no_valence3_corner",
        ],
        rmax,
        VARIABLE_RADIUS_REFUSED,
    )
}

/// Audits the source and radius law; returns the radii and the endpoints of
/// the single selected vertical cuboid edge.
fn vertical_variable_radius_edge(
    model: &Model,
    edges: &[usize],
    radii: &[[f64; 2]],
) -> Result<([f64; 2], [f64; 3], [f64; 3])> {
    audit_solid(model).map_err(|_| {
        refuse(
            VARIABLE_RADIUS_REFUSED,
            "Variable-radius fillet source must pass the global solid audit",
        )
    })?;
    let ([edge], [[r0, r1]]) = (edges, radii) else {
        return Err(refuse(
            VARIABLE_RADIUS_REFUSED,
            "exact-variable-radius-fillet/1 admits exactly one vertical cuboid edge and one linear radius pair",
        ));
    };
    let (r0, r1) = (*r0, *r1);
    if !(r0.is_finite() && r1.is_finite() && r0 > 0. && r1 > 0.) {
        return Err(refuse(
            VARIABLE_RADIUS_REFUSED,
            "Endpoint radii must be finite and positive",
        ));
    }
    if (r0 - r1).abs() <= model.tolerance_mm.max(1e-12) {
        return Err(refuse(
            VARIABLE_RADIUS_REFUSED,
            "Constant radius must use the constant-radius fillet capabilities; refuse silent substitution",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            VARIABLE_RADIUS_REFUSED,
            "exact-variable-radius-fillet/1 admits axis-aligned planar cuboids only",
        ));
    }
    let Some(edge_ref) = model.edges.get(*edge) else {
        return Err(refuse(VARIABLE_RADIUS_REFUSED, "Edge index out of range"));
    };
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            VARIABLE_RADIUS_REFUSED,
            "Curved edges are outside exact-variable-radius-fillet/1",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let vertical = dir[0].abs() <= VERTICAL_EDGE_TOLERANCE
        && dir[1].abs() <= VERTICAL_EDGE_TOLERANCE
        && dir[2].abs() > VERTICAL_EDGE_TOLERANCE;
    if !vertical {
        return Err(refuse(
            VARIABLE_RADIUS_REFUSED,
            "exact-variable-radius-fillet/1 admits vertical (+Z) cuboid edges only",
        ));
    }
    Ok(([r0, r1], a, b))
}

/// Exact equal-radius valence-3 corner blend: three concurrent cuboid edges at the
/// selected corner become rational quarter-cylinders joined by a stereographic spherical octant.
pub fn exact_valence3_corner_blend(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    if is_axis_aligned_cuboid(model) {
        return exact_axis_aligned_valence3_corner_blend(model, edges, radius);
    }
    model.validate()?;
    let refusal = || {
        refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Corner blend requires three distinct orthogonal edges of a planar cuboid",
        )
    };
    if edges.len() != 3
        || edges.iter().any(|&i| i >= model.edges.len())
        || edges
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 3
    {
        return Err(refusal());
    }
    let vertex = model.edges[edges[0]]
        .vertices
        .iter()
        .copied()
        .find(|v| edges.iter().all(|&e| model.edges[e].vertices.contains(v)))
        .ok_or_else(refusal)?;
    let origin = model.vertices[vertex].point;
    let mut axes = [[0.; 3]; 3];
    for (i, &edge) in edges.iter().enumerate() {
        let edge = &model.edges[edge];
        if edge.curve.degree != 1 {
            return Err(refusal());
        }
        let other = edge
            .vertices
            .iter()
            .copied()
            .find(|&v| v != vertex)
            .ok_or_else(refusal)?;
        let delta: [f64; 3] = std::array::from_fn(|j| model.vertices[other].point[j] - origin[j]);
        let length = delta[0].hypot(delta[1]).hypot(delta[2]);
        if !length.is_finite() || length <= 1e-12 {
            return Err(refusal());
        }
        axes[i] = delta.map(|v| v / length);
    }
    for i in 0..3 {
        for j in 0..i {
            let dot: f64 = (0..3).map(|k| axes[i][k] * axes[j][k]).sum();
            if dot.abs() > 1e-12 {
                return Err(refusal());
            }
        }
    }
    let mut to_local = [[0.; 4]; 4];
    let mut to_world = [[0.; 4]; 4];
    to_local[3][3] = 1.;
    to_world[3][3] = 1.;
    for i in 0..3 {
        for j in 0..3 {
            to_local[i][j] = axes[i][j];
            to_world[i][j] = axes[j][i];
        }
        to_local[i][3] = -(0..3).map(|j| axes[i][j] * origin[j]).sum::<f64>();
        to_world[i][3] = origin[i];
    }
    // Full cuboid recognition and audit run in the recovered orthonormal frame;
    // selected edges alone never establish the source's geometric class.
    let local = crate::transform::affine(model, to_local)?;
    let result = exact_axis_aligned_valence3_corner_blend(&local, edges, radius)?;
    let world = crate::transform::affine(&result.model, to_world)?;
    certify_blend_result(
        world,
        EXACT_VALENCE3_CORNER_BLEND_CAPABILITY,
        vec![
            "exact_equal_radius_sphere_octant",
            "three_rational_cylinders",
            "plane_cylinder_sphere_network",
            "rigid_cuboid_placement",
        ],
        radius,
        "BREP_VALENCE3_CORNER_BLEND_REFUSED",
    )
}
