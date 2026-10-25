use super::*;

#[derive(Debug)]
pub struct SharedPatchEdge {
    /// One retained world curve, oriented as the source boundary.
    pub curve: nurbs_core::curve::Curve,
    pub source_pcurve: nurbs_core::curve::Curve,
    pub target_pcurve: nurbs_core::curve::Curve,
    pub target_reversed: bool,
    pub agreements: [Report; 2],
    pub verification_cells: usize,
}

fn map_trim_parameter(
    value: f64,
    target_domain: [f64; 2],
    source_domain: [f64; 2],
    scale: f64,
) -> Option<f64> {
    let x = if value == target_domain[0] {
        0.
    } else if value == target_domain[1] {
        1.
    } else {
        (value - target_domain[0]) / (target_domain[1] - target_domain[0])
    };
    let y = 1. - x;
    let t = if y == 0. {
        0.
    } else if y == 1. {
        1.
    } else {
        y / (scale * (1. - y) + y)
    };
    let mapped = if t == 0. {
        source_domain[0]
    } else if t == 1. {
        source_domain[1]
    } else {
        source_domain[0] + t * (source_domain[1] - source_domain[0])
    };
    (mapped.is_finite()
        && mapped >= source_domain[0]
        && mapped <= source_domain[1])
        .then_some(mapped)
}

/// Transfer authored target ends through the opposite retained source geometry.
/// Weight ratios only propose scales: a complete correspondence proof on the
/// original curves certifies every partial trim before its ends are admitted.
fn transfer_coedge_trim(
    target: &nurbs_core::curve::Curve,
    source: &nurbs_core::curve::Curve,
    trim: Option<brep_topology::CoedgeTrim>,
) -> Result<Option<brep_topology::CoedgeTrim>> {
    let Some(trim) = trim else {
        return Ok(None);
    };
    trim.validate()?;
    let target_domain = target.domain();
    let source_domain = source.domain();
    if trim.curve_parameter.iter().any(|&value| {
        !value.is_finite() || value < target_domain[0] || value > target_domain[1]
    }) {
        return Err(Error::new(
            "BREP_TRIM_DOMAIN",
            "Coedge trim interval must lie inside its original curve domain",
        ));
    }
    let mut scales: Vec<f64> = vec![1.];
    let a = &target.weights;
    let b = &source.weights;
    let ratio = b[b.len() - 1] * a[a.len() - 1] / (b[0] * a[0]);
    if ratio.is_finite() && ratio > 0. {
        for degree in 1..=target.degree.min(source.degree) {
            let scale = ratio.powf(1. / degree as f64);
            if (1e-12..=1e12).contains(&scale)
                && !scales.iter().any(|candidate| candidate.to_bits() == scale.to_bits())
            {
                scales.push(scale);
            }
        }
    }
    for scale in scales {
        let mapped_ends = trim.curve_parameter.map(|value| {
            map_trim_parameter(value, target_domain, source_domain, scale)
        });
        let (Some(first), Some(second)) = (mapped_ends[0], mapped_ends[1]) else {
            continue;
        };
        let curve_parameter = [first, second];
        let mapped = brep_topology::CoedgeTrim {
            curve_parameter,
            pcurve_parameter: trim.pcurve_parameter,
            periodic_lift: trim.periodic_lift,
        };
        if mapped.validate().is_err() {
            continue;
        }
        let proof = nurbs_core::periodic_trim::verify_projective_curve_correspondence(
            target,
            source,
            [scale, 1.],
            true,
            cad_predicates::MAX_WORK,
        )?;
        if proof.is_some() {
            return Ok(Some(mapped));
        }
    }
    Err(Error::new(
        "BREP_THREAD_PATCH_UNRESOLVED",
        "No certified correspondence for the retained boundary trim",
    ))
}

fn trim_nodes(
    curve: &nurbs_core::curve::Curve,
    pcurve: &nurbs_core::curve::Curve,
    surface: &nurbs_core::surface::Surface,
    trim: brep_topology::CoedgeTrim,
) -> Result<[[f64; 2]; 2]> {
    trim.validate()?;
    let domains = [curve.domain(), pcurve.domain()];
    let parameters = [trim.curve_parameter, trim.pcurve_parameter];
    for (parameter, domain) in parameters.iter().zip(domains) {
        if parameter
            .iter()
            .any(|value| *value < domain[0] || *value > domain[1])
        {
            return Err(Error::new(
                "BREP_TRIM_DOMAIN",
                "Coedge trim interval must lie inside its original curve domain",
            ));
        }
    }
    let fraction = |value: f64, domain: [f64; 2]| {
        if value == domain[0] {
            0.
        } else if value == domain[1] {
            1.
        } else {
            (value - domain[0]) / (domain[1] - domain[0])
        }
    };
    let nodes = std::array::from_fn(|end| {
        std::array::from_fn(|axis| fraction(parameters[axis][end], domains[axis]))
    });
    let uv = [
        pcurve.evaluate(trim.pcurve_parameter[0])?.point,
        pcurve.evaluate(trim.pcurve_parameter[1])?.point,
    ];
    let surface_domains = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    for end in 0..2 {
        for axis in 0..2 {
            let periodic = [surface.periodic_u, surface.periodic_v][axis];
            let domain = surface_domains[axis];
            let expected = if periodic {
                ((uv[end][axis] - domain[0]) / (domain[1] - domain[0])).floor() as i32
            } else {
                0
            };
            if trim.periodic_lift[end][axis] != expected {
                return Err(Error::new(
                    "BREP_TRIM_PERIODIC_LIFT",
                    "Coedge periodic lift does not match its authored pcurve endpoint",
                ));
            }
        }
    }
    Ok(nodes)
}

fn verify_coedge_use(
    curve: &nurbs_core::curve::Curve,
    pcurve: &nurbs_core::curve::Curve,
    surface: &nurbs_core::surface::Surface,
    trim: Option<brep_topology::CoedgeTrim>,
    reversed: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    use nurbs_core::curve_surface_agreement;
    if let Some(trim) = trim {
        let nodes = trim_nodes(curve, pcurve, surface, trim)?;
        let checked = curve_surface_agreement::verify_oriented_correspondence(
            curve, pcurve, surface, &nodes, reversed, tolerance, max_cells,
        )?;
        Ok(Report {
            status: checked.status,
            cells: checked.cells,
            witness: None,
            witness_distance: None,
        })
    } else {
        curve_surface_agreement::verify(curve, pcurve, surface, reversed, tolerance, max_cells)
    }
}

/// Sew two boundary uses within one open shell. Retains the left world curve;
/// exact opposite endpoints and continuous agreement on both faces are needed.
/// This does not certify surface embedding or turn the shell into a solid.
pub fn sew_patch_boundaries(
    model: &Model,
    left: [usize; 2],
    right: [usize; 2],
    max_verification_cells: usize,
) -> Result<(Model, SharedPatchEdge)> {
    sew_boundaries_impl(model, left, right, max_verification_cells, true)
}
pub(super) fn sew_boundaries_impl(
    model: &Model,
    left: [usize; 2],
    right: [usize; 2],
    max_verification_cells: usize,
    validate_input: bool,
) -> Result<(Model, SharedPatchEdge)> {
    if validate_input {
        model.validate()?;
    }
    let invalid = || {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Sew needs distinct forward boundary uses in one open shell",
        )
    };
    if model.shells.len() != 1 || model.shells[0].closed || !model.bodies.is_empty() {
        return Err(invalid());
    }
    let coedge = |address: [usize; 2]| {
        model
            .loops
            .get(address[0])
            .and_then(|l| l.coedges.get(address[1]))
            .ok_or_else(invalid)
    };
    let lc = coedge(left)?;
    let rc = coedge(right)?;
    if lc.edge == rc.edge || lc.reversed || rc.reversed {
        return Err(invalid());
    }
    for edge in [lc.edge, rc.edge] {
        if model
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .filter(|c| c.edge == edge)
            .count()
            != 1
        {
            return Err(invalid());
        }
    }
    let owner = |wire| {
        model
            .faces
            .iter()
            .find(|f| f.outer == wire)
            .ok_or_else(invalid)
    };
    let lf = owner(left[0])?;
    let rf = owner(right[0])?;
    let le = &model.edges[lc.edge];
    let re = &model.edges[rc.edge];
    let mut vertex_map: Vec<_> = (0..model.vertices.len()).collect();
    for i in 0..2 {
        let old = re.vertices[i];
        let new = le.vertices[1 - i];
        if model.vertices[old].point != model.vertices[new].point {
            return Err(invalid());
        }
        vertex_map[old] = new;
    }
    let provisional = nurbs_core::thread::MappedEdge {
        curve: le.curve.clone(),
        pcurve: lc.pcurve.clone(),
        agreement: Report {
            status: nurbs_core::curve_surface_agreement::Status::Unresolved,
            cells: 0,
            witness: None,
            witness_distance: None,
        },
        requested_world_tolerance: model.tolerance_mm,
    };
    let transferred_trim = transfer_coedge_trim(&re.curve, &le.curve, rc.trim())?;
    let proof = share_patch_edge_with_trims(
        &provisional,
        lc.trim(),
        &lf.surface,
        &rc.pcurve,
        transferred_trim,
        &rf.surface,
        true,
        model.tolerance_mm,
        max_verification_cells,
    )?;
    let mut result = model.clone();
    result.loops[right[0]].coedges[right[1]].edge = lc.edge;
    result.loops[right[0]].coedges[right[1]].reversed = true;
    if let Some(trim) = transferred_trim {
        let coedge = result.loops[right[0]].coedges[right[1]].clone();
        result.loops[right[0]].coedges[right[1]] = coedge.with_trim(trim)?;
    }
    result.edges.remove(rc.edge);
    for c in result.loops.iter_mut().flat_map(|l| &mut l.coedges) {
        if c.edge > rc.edge {
            c.edge -= 1;
        }
    }
    // Resolve endpoint unions before compacting; reject cycles rather than
    // silently assigning a different coordinate or breaking incidence.
    for i in 0..vertex_map.len() {
        let mut v = i;
        let mut steps = 0;
        while vertex_map[v] != v {
            v = vertex_map[v];
            steps += 1;
            if steps > vertex_map.len() {
                return Err(invalid());
            }
        }
        vertex_map[i] = v;
    }
    let mut compact = vec![usize::MAX; vertex_map.len()];
    let mut vertices = Vec::new();
    for (i, &root) in vertex_map.iter().enumerate() {
        if i == root {
            compact[i] = vertices.len();
            vertices.push(model.vertices[i].clone());
        }
    }
    for edge in &mut result.edges {
        edge.vertices = edge.vertices.map(|v| compact[vertex_map[v]]);
    }
    result.vertices = vertices;
    result.rebuild_topology_ids();
    if validate_input {
        result.inherit_topology_ids(&[model]);
    }
    if validate_input {
        result.validate()?;
    }
    Ok((result, proof))
}

/// Join two single-face patch sheets using one retained source edge. Opposite
/// loop traversal and exact endpoint equality are required; no vertex healing.
/// All source inputs remain immutable, including on failed verification.
pub fn join_patch_sheets(
    source: &Model,
    target: &Model,
    source_coedge: usize,
    target_coedge: usize,
    max_verification_cells: usize,
) -> Result<(Model, SharedPatchEdge)> {
    attach_patch_sheet(
        source,
        target,
        0,
        source_coedge,
        target_coedge,
        max_verification_cells,
    )
}

/// Attach a single-loop face to a boundary edge of an existing open shell.
/// Edges already used twice refuse; no third incident face is admitted.
pub fn attach_patch_sheet(
    source: &Model,
    target: &Model,
    source_loop: usize,
    source_coedge: usize,
    target_coedge: usize,
    max_verification_cells: usize,
) -> Result<(Model, SharedPatchEdge)> {
    attach_impl(
        source,
        target,
        source_loop,
        source_coedge,
        target_coedge,
        max_verification_cells,
        true,
    )
}
/// Attach a face along two boundaries atomically; the intermediate face fan
/// may be disconnected and is never returned or admitted as a Model.
pub fn attach_patch_sheet_two_edges(
    source: &Model,
    target: &Model,
    first: [[usize; 2]; 2],
    second: [[usize; 2]; 2],
    max_verification_cells: usize,
) -> Result<Model> {
    Ok(attach_two_edges_impl(source, target, first, second, max_verification_cells)?.0)
}
pub(super) fn attach_two_edges_impl(
    source: &Model,
    target: &Model,
    first: [[usize; 2]; 2],
    second: [[usize; 2]; 2],
    max_verification_cells: usize,
) -> Result<(Model, usize)> {
    if first[1][0] != 0 || second[1][0] != 0 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Target uses must belong to loop zero",
        ));
    }
    attach_patch_sheet_edges(
        source,
        target,
        &[(first[0], first[1][1]), (second[0], second[1][1])],
        max_verification_cells,
    )
}
/// Attach a complete single-loop face along 1..254 selected boundaries,
/// checking all seams under one shared budget before admitting the result.
pub fn attach_patch_sheet_edges(
    source: &Model,
    target: &Model,
    pairs: &[([usize; 2], usize)],
    max_verification_cells: usize,
) -> Result<(Model, usize)> {
    if pairs.is_empty() || pairs.len() > 254 || max_verification_cells > 100000 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Attach needs 1..254 distinct boundary pairs and at most 100000 cells",
        ));
    }
    let mut source_uses = std::collections::BTreeSet::new();
    let mut target_uses = std::collections::BTreeSet::new();
    for &(s, t) in pairs {
        if !source_uses.insert(s) || !target_uses.insert(t) {
            return Err(Error::new(
                "BREP_INVALID_THREAD_PATCH",
                "Repeated boundary correspondence",
            ));
        }
    }
    let (first_source, first_target) = pairs[0];
    let (mut result, proof) = attach_impl(
        source,
        target,
        first_source[0],
        first_source[1],
        first_target,
        max_verification_cells,
        false,
    )?;
    let target_loop = source.loops.len();
    let mut used = proof.verification_cells;
    for &(s, t) in &pairs[1..] {
        let (next, proof) = sew_boundaries_impl(
            &result,
            s,
            [target_loop, t],
            max_verification_cells - used,
            false,
        )?;
        used += proof.verification_cells;
        result = next;
    }
    result.validate()?;
    Ok((result, used))
}

pub(super) fn attach_impl(
    source: &Model,
    target: &Model,
    source_loop: usize,
    source_coedge: usize,
    target_coedge: usize,
    max_verification_cells: usize,
    validate_result: bool,
) -> Result<(Model, SharedPatchEdge)> {
    source.validate()?;
    target.validate()?;
    if target.faces.len() != 1 || target.loops.len() != 1 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Target must be a single-loop face",
        ));
    }
    for model in [source, target] {
        if model.shells.len() != 1
            || model.shells[0].closed
            || !model.bodies.is_empty()
            || model.faces.iter().any(|f| !f.holes.is_empty())
        {
            return Err(Error::new(
                "BREP_INVALID_THREAD_PATCH",
                "Join requires open shells without bodies or holes",
            ));
        }
    }
    let source_wire = source.loops.get(source_loop).ok_or_else(|| {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Source loop index out of range",
        )
    })?;
    let source_face = source
        .faces
        .iter()
        .find(|f| f.outer == source_loop)
        .ok_or_else(|| Error::new("BREP_INVALID_THREAD_PATCH", "Source loop has no face owner"))?;
    let sc = source_wire.coedges.get(source_coedge).ok_or_else(|| {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Source coedge index out of range",
        )
    })?;
    if source
        .loops
        .iter()
        .flat_map(|l| &l.coedges)
        .filter(|c| c.edge == sc.edge)
        .count()
        != 1
    {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Selected source edge is not a boundary edge",
        ));
    }
    let tc = target.loops[0].coedges.get(target_coedge).ok_or_else(|| {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Target coedge index out of range",
        )
    })?;
    if sc.reversed || tc.reversed {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Join expects forward source sheet coedges",
        ));
    }
    let se = &source.edges[sc.edge];
    let te = &target.edges[tc.edge];
    if source.vertices[se.vertices[0]].point != target.vertices[te.vertices[1]].point
        || source.vertices[se.vertices[1]].point != target.vertices[te.vertices[0]].point
    {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Join needs exact opposite boundary endpoints",
        ));
    }
    let tolerance = source.tolerance_mm.min(target.tolerance_mm);
    let provisional = nurbs_core::thread::MappedEdge {
        curve: se.curve.clone(),
        pcurve: sc.pcurve.clone(),
        agreement: Report {
            status: nurbs_core::curve_surface_agreement::Status::Unresolved,
            cells: 0,
            witness: None,
            witness_distance: None,
        },
        requested_world_tolerance: tolerance,
    };
    let transferred_trim = transfer_coedge_trim(&te.curve, &se.curve, tc.trim())?;
    let shared = share_patch_edge_with_trims(
        &provisional,
        sc.trim(),
        &source_face.surface,
        &tc.pcurve,
        transferred_trim,
        &target.faces[0].surface,
        true,
        tolerance,
        max_verification_cells,
    )?;
    let mut result = source.clone();
    result.tolerance_mm = tolerance;
    let mut vertices = vec![usize::MAX; target.vertices.len()];
    vertices[te.vertices[0]] = se.vertices[1];
    vertices[te.vertices[1]] = se.vertices[0];
    for (i, vertex) in target.vertices.iter().enumerate() {
        if vertices[i] == usize::MAX {
            let matches: Vec<_> = source
                .vertices
                .iter()
                .enumerate()
                .filter(|(_, v)| v.point == vertex.point)
                .map(|(i, _)| i)
                .collect();
            if matches.len() > 1 {
                return Err(Error::new(
                    "BREP_INVALID_THREAD_PATCH",
                    "Ambiguous exact vertex correspondence",
                ));
            }
            if let Some(&existing) = matches.first() {
                vertices[i] = existing;
            } else {
                vertices[i] = result.vertices.len();
                result.vertices.push(vertex.clone());
            }
        }
    }
    let mut edges = vec![usize::MAX; target.edges.len()];
    edges[tc.edge] = sc.edge;
    for (i, edge) in target.edges.iter().enumerate() {
        if i == tc.edge {
            continue;
        }
        edges[i] = result.edges.len();
        let mut edge = edge.clone();
        edge.vertices = edge.vertices.map(|v| vertices[v]);
        result.edges.push(edge);
    }
    let outer = result.loops.len();
    result.loops.push(Loop {
        coedges: target.loops[0]
            .coedges
            .iter()
            .map(|c| {
                let mut mapped = c.clone();
                mapped.edge = edges[c.edge];
                if c.edge == tc.edge {
                    mapped.reversed = true;
                    if let Some(trim) = transferred_trim {
                        mapped = mapped.with_trim(trim)?;
                    }
                }
                Ok(mapped)
            })
            .collect::<Result<Vec<_>>>()?,
    });
    let face = result.faces.len();
    result.faces.push(Face {
        surface: target.faces[0].surface.clone(),
        outer,
        holes: vec![],
    });
    result.shells[0].faces.push(FaceUse {
        face,
        reversed: false,
    });
    result.rebuild_topology_ids();
    if validate_result {
        result.inherit_topology_ids(&[source, target]);
    }
    if validate_result {
        result.validate()?;
    }
    Ok((result, shared))
}

/// Verify one immutable world edge on two original patch surfaces. Both uses
/// are checked anew; cached source agreement is not trusted. This constructs
/// geometric boundary evidence, not global sewing or shell orientation.
pub fn share_patch_edge(
    source: &nurbs_core::thread::MappedEdge,
    source_surface: &nurbs_core::surface::Surface,
    target_pcurve: &nurbs_core::curve::Curve,
    target_surface: &nurbs_core::surface::Surface,
    target_reversed: bool,
    tolerance: f64,
    max_verification_cells: usize,
) -> Result<SharedPatchEdge> {
    share_patch_edge_with_trims(
        source,
        None,
        source_surface,
        target_pcurve,
        None,
        target_surface,
        target_reversed,
        tolerance,
        max_verification_cells,
    )
}

fn share_patch_edge_with_trims(
    source: &nurbs_core::thread::MappedEdge,
    source_trim: Option<brep_topology::CoedgeTrim>,
    source_surface: &nurbs_core::surface::Surface,
    target_pcurve: &nurbs_core::curve::Curve,
    target_trim: Option<brep_topology::CoedgeTrim>,
    target_surface: &nurbs_core::surface::Surface,
    target_reversed: bool,
    tolerance: f64,
    max_verification_cells: usize,
) -> Result<SharedPatchEdge> {
    use nurbs_core::curve_surface_agreement::Status;
    if !(1e-10..=1e-2).contains(&tolerance) || max_verification_cells > 100000 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Shared boundary needs valid tolerance and at most 100000 cells",
        ));
    }
    if max_verification_cells == 0 {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Shared boundary verification budget exhausted",
        ));
    }
    let first = verify_coedge_use(
        &source.curve,
        &source.pcurve,
        source_surface,
        source_trim,
        false,
        tolerance,
        max_verification_cells,
    )?;
    if first.status != Status::WithinTolerance || first.cells == max_verification_cells {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Source boundary agreement unproved or budget exhausted",
        ));
    }
    // A transferred trim already expresses the authored target relation in the
    // opposite retained source parameterization; do not reverse those ends twice.
    let second = verify_coedge_use(
        &source.curve,
        target_pcurve,
        target_surface,
        target_trim,
        target_reversed && target_trim.is_none(),
        tolerance,
        max_verification_cells - first.cells,
    )?;
    if second.status != Status::WithinTolerance {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Target boundary agreement unproved",
        ));
    }
    let cells = first.cells + second.cells;
    Ok(SharedPatchEdge {
        curve: source.curve.clone(),
        source_pcurve: source.pcurve.clone(),
        target_pcurve: target_pcurve.clone(),
        target_reversed,
        agreements: [first, second],
        verification_cells: cells,
    })
}
