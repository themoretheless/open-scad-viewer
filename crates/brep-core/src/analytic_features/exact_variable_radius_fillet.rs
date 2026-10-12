use super::*;

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
    let refusal = || refuse(
        "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
        "Variable-radius fillet requires one straight edge of an orthogonal planar cuboid",
    );
    if edges.len() != 1 || radii.len() != 1 || edges[0] >= model.edges.len() {
        return Err(refusal());
    }
    let selected = &model.edges[edges[0]];
    let a = model.vertices[selected.vertices[0]].point;
    let b = model.vertices[selected.vertices[1]].point;
    if is_axis_aligned_cuboid(model) && (a[0]-b[0]).abs() <= 1e-12
        && (a[1]-b[1]).abs() <= 1e-12 && (a[2]-b[2]).abs() > 1e-12 {
        return exact_axis_aligned_variable_radius_fillet(model, edges, radii);
    }
    let vertex = selected.vertices[0];
    let adjacent: Vec<_> = model.edges.iter().enumerate()
        .filter_map(|(i,e)| (i != edges[0] && e.vertices.contains(&vertex)).then_some(i))
        .collect();
    if adjacent.len() != 2 { return Err(refusal()); }
    let mut axes = [[0.;3];3];
    for (axis, edge_id) in [adjacent[0], adjacent[1], edges[0]].into_iter().enumerate() {
        let edge = &model.edges[edge_id];
        if edge.curve.degree != 1 { return Err(refusal()); }
        let other = edge.vertices.iter().copied().find(|&v|v != vertex).ok_or_else(refusal)?;
        let delta: [f64;3] = std::array::from_fn(|j|model.vertices[other].point[j]-a[j]);
        let length = delta[0].hypot(delta[1]).hypot(delta[2]);
        if !length.is_finite() || length <= 1e-12 { return Err(refusal()); }
        axes[axis] = delta.map(|v|v/length);
    }
    for i in 0..3 { for j in 0..i {
        if (0..3).map(|k|axes[i][k]*axes[j][k]).sum::<f64>().abs() > 1e-12 {
            return Err(refusal());
        }
    }}
    let mut to_local = [[0.;4];4];
    let mut to_world = [[0.;4];4];
    to_local[3][3]=1.;to_world[3][3]=1.;
    for i in 0..3 {
        for j in 0..3 { to_local[i][j]=axes[i][j];to_world[i][j]=axes[j][i]; }
        to_local[i][3]=-(0..3).map(|j|axes[i][j]*a[j]).sum::<f64>();
        to_world[i][3]=a[i];
    }
    // Recover a coordinate frame, then recognize and audit the entire source.
    // Three orthogonal incident edges alone do not qualify a cuboid.
    let local = crate::transform::affine(model,to_local)?;
    let result = exact_axis_aligned_variable_radius_fillet(&local,edges,radii)?;
    let world = crate::transform::affine(&result.model,to_world)?;
    certify_blend_result(world,EXACT_VARIABLE_RADIUS_FILLET_CAPABILITY,
        vec!["exact_linear_radius_law","rational_conical_fillet_face",
             "no_constant_radius_substitution","no_valence3_corner","rigid_cuboid_placement"],
        radii[0][0].max(radii[0][1]),"BREP_VARIABLE_RADIUS_FILLET_REFUSED")
}

fn exact_axis_aligned_variable_radius_fillet(
    model: &Model,
    edges: &[usize],
    radii: &[[f64; 2]],
) -> Result<AuditedFeatureResult> {
    audit_solid(model).map_err(|_| {
        refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Variable-radius fillet source must pass the global solid audit",
        )
    })?;
    if edges.len() != 1 || radii.len() != 1 {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "exact-variable-radius-fillet/1 admits exactly one vertical cuboid edge and one linear radius pair",
        ));
    }
    let [r0, r1] = radii[0];
    if !(r0.is_finite() && r1.is_finite() && r0 > 0. && r1 > 0.) {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Endpoint radii must be finite and positive",
        ));
    }
    if (r0 - r1).abs() <= model.tolerance_mm.max(1e-12) {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Constant radius must use the constant-radius fillet capabilities; refuse silent substitution",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "exact-variable-radius-fillet/1 admits axis-aligned planar cuboids only",
        ));
    }
    let edge = edges[0];
    if edge >= model.edges.len() {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Edge index out of range",
        ));
    }
    let edge_ref = &model.edges[edge];
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Curved edges are outside exact-variable-radius-fillet/1",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let vertical = dir[0].abs() <= 1e-12 && dir[1].abs() <= 1e-12 && dir[2].abs() > 1e-12;
    if !vertical {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "exact-variable-radius-fillet/1 admits vertical (+Z) cuboid edges only",
        ));
    }
    let (min, max) = model_bounds(model);
    let rmax = r0.max(r1);
    if max[0] - min[0] <= 2. * rmax || max[1] - min[1] <= 2. * rmax {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Cuboid extents too small for the endpoint radii",
        ));
    }
    let height = (a[2] - b[2]).abs();
    if !(height.is_finite() && height > rmax * 2.) {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
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
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
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
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
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
        "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
    )
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

fn exact_axis_aligned_valence3_corner_blend(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    audit_solid(model).map_err(|_| {
        refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Valence-3 corner blend source must pass the global solid audit",
        )
    })?;
    if !(radius.is_finite() && radius > 0.) {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Valence-3 radius must be finite and positive",
        ));
    }
    if edges.len() != 3
        || edges
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 3
    {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "exact-valence3-corner-blend/1 admits exactly three distinct concurrent cuboid edges",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "exact-valence3-corner-blend/1 admits axis-aligned planar cuboids only",
        ));
    }
    let (min, max) = model_bounds(model);
    if (0..3).any(|i| max[i] - min[i] <= 2. * radius) {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Cuboid extents too small for the valence-3 radius",
        ));
    }
    let mut corner_vertex = None;
    for &edge in edges {
        if edge >= model.edges.len() {
            return Err(refuse(
                "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                "Edge index out of range",
            ));
        }
        if model.edges[edge].curve.degree != 1 {
            return Err(refuse(
                "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                "Curved edges are outside exact-valence3-corner-blend/1",
            ));
        }
    }
    // Three distinct edges must share one cuboid corner.
    let sets: Vec<[usize; 2]> = edges.iter().map(|&e| model.edges[e].vertices).collect();
    for &v in &sets[0] {
        if sets[1].contains(&v) && sets[2].contains(&v) {
            corner_vertex = Some(v);
            break;
        }
    }
    let Some(vid) = corner_vertex else {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Selected edges do not meet at a single valence-3 vertex",
        ));
    };
    let p = model.vertices[vid].point;
    // Reflect each minimum coordinate to the maximum corner. This is an
    // isometry: radius and contact angles remain unchanged. affine() also
    // reverses shell uses for odd reflections and preserves topology IDs.
    let tolerance = model.tolerance_mm.max(1e-9);
    let mut placement = [[0.; 4]; 4];
    placement[3][3] = 1.;
    for i in 0..3 {
        if (p[i] - max[i]).abs() <= tolerance {
            placement[i][i] = 1.;
        } else if (p[i] - min[i]).abs() <= tolerance {
            placement[i][i] = -1.;
            placement[i][3] = min[i] + max[i];
        } else {
            return Err(refuse(
                "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                "Selected vertex must be a cuboid corner",
            ));
        }
    }
    let local_source = crate::transform::affine(model, placement)?;
    let local_result =
        crate::imprint_pipeline::valence3_cuboid_max_corner(&local_source, min, max, radius)
            .map_err(|error| {
                refuse(
                    "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                    &format!("Valence-3 authorship refused: {}", error.message),
                )
            })?;
    // The reflection is its own inverse. Certify the final world-space solid.
    let result = crate::transform::affine(&local_result, placement)?;
    let spheres = result
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 2)
        .count();
    let cylinders = result
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 1)
        .count();
    if spheres != 1 || cylinders < 3 {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Valence-3 result missing sphere octant or three cylinder faces",
        ));
    }
    certify_blend_result(
        result,
        EXACT_VALENCE3_CORNER_BLEND_CAPABILITY,
        vec![
            "exact_equal_radius_sphere_octant",
            "three_rational_cylinders",
            "plane_cylinder_sphere_network",
        ],
        radius,
        "BREP_VALENCE3_CORNER_BLEND_REFUSED",
    )
}

/// AF-01: cuboid single convex edge → cylindrical fillet via corner cutter − cylinder.
pub fn analytic_fillet(
    model: &Model,
    edge: usize,
    radius: f64,
) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
    if !(radius.is_finite() && radius > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet radius must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "analytic-fillet/1 admits axis-aligned planar cuboids only (AF-N1)",
        ));
    }
    if edge >= model.edges.len() {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Edge index out of range",
        ));
    }
    let edge_ref = &model.edges[edge];
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Curved edges are outside analytic-fillet/1 (AF-N1)",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = dir[0].hypot(dir[1]).hypot(dir[2]);
    if !(len.is_finite() && len > radius * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Edge too short for the requested fillet radius",
        ));
    }
    // Prefer a vertical (+Z) edge on the +X/+Y corner of an axis-aligned box.
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= radius * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Cuboid extents too small for fillet radius",
        ));
    }
    let vertical = dir[0].abs() <= 1e-12 && dir[1].abs() <= 1e-12 && dir[2].abs() > 1e-12;
    if !vertical {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "AF-01 walking slice admits vertical (+Z) cuboid edges only",
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
    let result =
        crate::imprint_pipeline::rounded_cuboid_vertical_edges(model, min, max, rounded, radius)?;
    result.validate()?;
    let has_cyl = result
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1);
    if !has_cyl {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet result missing cylindrical face; refuse faceted claim",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-fillet/1",
            complete: true,
            notes: vec!["af01_cylindrical_edge"],
        },
    ))
}

pub(super) fn vertical_edge_xy(model: &Model, edge: usize) -> Option<[f64; 2]> {
    let e = model.edges.get(edge)?;
    let a = model.vertices.get(e.vertices[0])?.point;
    let b = model.vertices.get(e.vertices[1])?.point;
    if (a[0] - b[0]).abs() > 1e-9 || (a[1] - b[1]).abs() > 1e-9 {
        return None;
    }
    Some([a[0], a[1]])
}

pub(super) fn remap_vertical_edge(model: &Model, xy: [f64; 2], tol: f64) -> Result<usize> {
    model
        .edges
        .iter()
        .enumerate()
        .find_map(|(i, e)| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            if (a[0] - b[0]).abs() > 1e-9 || (a[1] - b[1]).abs() > 1e-9 {
                return None;
            }
            let mx = 0.5 * (a[0] + b[0]);
            let my = 0.5 * (a[1] + b[1]);
            if (mx - xy[0]).hypot(my - xy[1]) <= tol {
                Some(i)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            refuse(
                "BREP_ANALYTIC_FILLET_REFUSED",
                "Fillet chain remapping lost a vertical edge; refuse silent nearest-edge",
            )
        })
}
