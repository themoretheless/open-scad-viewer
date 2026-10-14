use super::*;

pub(super) fn stitch(polys: Vec<Polygon>, eps: f64, budget: &mut Budget) -> Result<Mesh> {
    let mut vertices = Vertices::new(eps);
    let mut faces = Vec::new();
    for p in polys {
        let mut face: Vec<usize> = p.vertices.into_iter().map(|v| vertices.insert(v)).collect();
        face.dedup();
        if face.len() > 1 && face.first() == face.last() {
            face.pop();
        }
        if face.len() >= 3 {
            faces.push(face);
        }
    }
    // All fragment vertices participate in edge splitting, including those
    // introduced by an adjacent face. This removes BSP T junctions.
    // Split candidates must lie within the eps-expanded bbox of the edge
    // (projection inside the segment plus perpendicular distance <= eps
    // implies per-axis containment), so a uniform grid with cells no smaller
    // than the longest edge span finds them in O(1) cell lookups per edge.
    // This replaces three sorted sweeps over all vertices (3 * O(n log n)
    // plus three index arrays). The eps-sized `Vertices.cells` cannot serve
    // here: walking it along an edge costs O((length / eps)^3) cell visits.
    let mut span = eps;
    for face in &faces {
        for i in 0..face.len() {
            let (a, b) = (face[i], face[(i + 1) % face.len()]);
            if a == b {
                continue;
            }
            let (p, q) = (vertices.points[a], vertices.points[b]);
            for axis in 0..3 {
                span = span.max((p[axis] - q[axis]).abs());
            }
        }
    }
    // Buckets are filled in ascending vertex order, but the per-edge sequence
    // is fixed by the explicit sort below, so neither hash ordering nor cell
    // traversal order can affect the output.
    let mut grid = FxHashMap::<[i64; 3], Vec<usize>>::default();
    for (id, p) in vertices.points.iter().enumerate() {
        grid.entry(p.map(|v| (v / span).floor() as i64))
            .or_default()
            .push(id);
    }
    // Lookup-only map: output order is fixed by face/vertex traversal, so the
    // hasher cannot affect determinism.
    let mut edges = FxHashMap::<(usize, usize), Vec<usize>>::default();
    let mut indices = Vec::new();
    for face in faces {
        let mut boundary = Vec::new();
        for i in 0..face.len() {
            let a = face[i];
            let b = face[(i + 1) % face.len()];
            if a == b {
                continue;
            }
            let key = (a.min(b), a.max(b));
            let sequence = match edges.entry(key) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let p = vertices.points[key.0];
                    let q = vertices.points[key.1];
                    let direction = sub(q, p);
                    let length = norm(direction);
                    if length <= eps {
                        return Err(numeric("Collapsed Boolean edge"));
                    }
                    let mut along = vec![(0., key.0), (1., key.1)];
                    // Cells overlapping the eps-expanded edge bbox: with a
                    // cell size of at least the longest edge span this is a
                    // constant number of cells per axis. Every point passing
                    // the precise filter below lies in one of these cells.
                    let low: [f64; 3] =
                        std::array::from_fn(|axis| (p[axis].min(q[axis]) - eps) / span);
                    let high: [f64; 3] =
                        std::array::from_fn(|axis| (p[axis].max(q[axis]) + eps) / span);
                    for cx in low[0].floor() as i64..=high[0].floor() as i64 {
                        for cy in low[1].floor() as i64..=high[1].floor() as i64 {
                            for cz in low[2].floor() as i64..=high[2].floor() as i64 {
                                let Some(cell) = grid.get(&[cx, cy, cz]) else {
                                    continue;
                                };
                                for &id in cell {
                                    budget.tick(1)?;
                                    if id == key.0 || id == key.1 {
                                        continue;
                                    }
                                    let delta = sub(vertices.points[id], p);
                                    let t = dot(delta, direction) / (length * length);
                                    if t > eps / length
                                        && t < 1. - eps / length
                                        && norm(sub(delta, scale(direction, t))) <= eps
                                    {
                                        along.push((t, id));
                                    }
                                }
                            }
                        }
                    }
                    // Parameter order along the edge; the index tiebreak keeps
                    // the sequence deterministic when two candidates share a
                    // bitwise-equal parameter.
                    along.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                    entry.insert(along.into_iter().map(|(_, i)| i).collect())
                }
            };
            if a == key.0 {
                boundary.extend_from_slice(&sequence[..sequence.len() - 1]);
            } else {
                boundary.extend(sequence.iter().rev().take(sequence.len() - 1));
            }
        }
        if boundary.len() < 3 {
            continue;
        }
        if boundary.len() == 3 {
            indices.extend(&boundary);
        } else {
            let center = std::array::from_fn(|a| {
                boundary.iter().map(|i| vertices.points[*i][a]).sum::<f64>() / boundary.len() as f64
            });
            // Do not re-use a boundary vertex as a centroid of a thin polygon.
            let id = vertices.points.len();
            vertices.points.push(center);
            for i in 0..boundary.len() {
                indices.extend([id, boundary[i], boundary[(i + 1) % boundary.len()]]);
            }
        }
        if indices.len() / 3 > budget.options.max_output_triangles {
            return Err(limit());
        }
    }
    // Canonical triangle accounting removes coincident zero-thickness faces.
    let mut unique = BTreeMap::<[usize; 3], ([usize; 3], i32)>::new();
    for t in indices.as_chunks::<3>().0 {
        let t = [t[0], t[1], t[2]];
        let mut key = t;
        key.sort();
        let inversions =
            usize::from(t[0] > t[1]) + usize::from(t[0] > t[2]) + usize::from(t[1] > t[2]);
        let sign = if inversions % 2 == 0 { 1 } else { -1 };
        let entry = unique.entry(key).or_insert((t, 0));
        entry.1 += sign;
    }
    let mut out = Mesh {
        positions: vec![],
        indices: vec![],
        uv: None,
    };
    let mut remap = BTreeMap::new();
    for (_, (mut t, count)) in unique {
        if count == 0 {
            continue;
        }
        let inversions =
            usize::from(t[0] > t[1]) + usize::from(t[0] > t[2]) + usize::from(t[1] > t[2]);
        if (inversions % 2 == 0) != (count > 0) {
            t.swap(1, 2);
        }
        for old in t {
            let id = *remap.entry(old).or_insert_with(|| {
                let id = out.positions.len() / 3;
                out.positions.extend(vertices.points[old]);
                id
            });
            out.indices.push(id);
        }
    }
    Ok(out)
}
/// Union, intersection or A-minus-B of closed oriented solid meshes. Empty
/// meshes are valid identities. UVs are dropped because CSG creates new faces.
pub fn boolean(a: &Mesh, b: &Mesh, operation: Operation, options: &Options) -> Result<BuiltMesh> {
    a.validate()?;
    b.validate()?;
    if !(1e-12..=1e-5).contains(&options.relative_tolerance)
        || options.max_work == 0
        || options.max_work > 8_000_000
        || options.max_fragments == 0
        || options.max_fragments > 100_000
        || options.max_output_triangles == 0
        || options.max_output_triangles > MAX_TRIANGLES
    {
        return Err(error(
            "POLYGON_BOOLEAN_INVALID_OPTIONS",
            "Invalid Boolean tolerance or work/fragment/output limits",
        ));
    }
    let input_triangles = [a.indices.len() / 3, b.indices.len() / 3];
    // Inputs above the BSP admission cap can still take an exact fast path
    // (separated, nested, identical or empty operands). Those results are
    // bounded by their inputs, so only the BSP path keeps the cap; the
    // tolerance-budgeted intersection audits are skipped above it and the
    // report says so through `self_intersection_status`.
    let audited = input_triangles.iter().sum::<usize>() <= BSP_INPUT_TRIANGLES;
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for mesh in [a, b] {
        for i in &mesh.indices {
            let p = mesh.point(*i)?;
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
    }
    let empty = a.indices.is_empty() && b.indices.is_empty();
    let origin = if empty {
        [0.; 3]
    } else {
        std::array::from_fn(|i| min[i] / 2. + max[i] / 2.)
    };
    let extent = if empty {
        1.
    } else {
        (0..3).map(|i| max[i] - min[i]).fold(0., f64::max)
    };
    if !extent.is_finite() || extent <= 0. {
        return Err(numeric("Solid bounds cannot be normalized"));
    }
    let a = normalized(a, origin, extent)?;
    let b = normalized(b, origin, extent)?;
    validate_solid(&a)?;
    validate_solid(&b)?;
    let eps = options.relative_tolerance;
    let mut budget = Budget {
        stage: "input validation",
        work: 0,
        fragments: 0,
        options: options.clone(),
    };
    if audited {
        for (index, mesh) in [&a, &b].into_iter().enumerate() {
            validation::geometry(mesh, eps, &mut budget)
                .map_err(|e| error(e.code, format!("Input {index}: {}", e.message)))?;
            validation::orientation(mesh, eps, &mut budget)?;
        }
    }
    // Exact mesh identity avoids unnecessary splitting of densely sampled surfaces.
    // Validation above still applies: identity cannot admit malformed solids.
    let contains = |outer: &Mesh, inner: &Mesh| -> Result<bool> {
        if outer.indices.is_empty() || inner.indices.is_empty() || outer.indices.len() / 3 > 128 {
            return Ok(false);
        }
        for t in outer.indices.as_chunks::<3>().0 {
            let p = outer.point(t[0])?;
            let n = cross(sub(outer.point(t[1])?, p), sub(outer.point(t[2])?, p));
            let tolerance = eps * norm(n);
            if outer
                .positions
                .as_chunks::<3>()
                .0
                .iter()
                .any(|v| dot(n, sub([v[0], v[1], v[2]], p)) > tolerance)
                || inner
                    .positions
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .any(|v| dot(n, sub([v[0], v[1], v[2]], p)) >= -tolerance)
            {
                return Ok(false);
            }
        }
        Ok(true)
    };
    let center = |m: &Mesh| {
        std::array::from_fn::<_, 3, _>(|k| {
            (m.positions
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min)
                + m.positions
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max))
                / 2.
        })
    };
    let axis = sub(center(&b), center(&a));
    let separated = !a.indices.is_empty()
        && !b.indices.is_empty()
        && a.positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| dot(axis, [p[0], p[1], p[2]]))
            .fold(f64::NEG_INFINITY, f64::max)
            + eps * norm(axis)
            < b.positions
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| dot(axis, [p[0], p[1], p[2]]))
                .fold(f64::INFINITY, f64::min);
    // `fast` marks results assembled from whole input surfaces without BSP
    // clipping; their size is bounded by the inputs and they carry no new
    // seams, so the output cap and coplanar simplification do not apply.
    let mut fast = true;
    let mut result = if separated {
        match operation {
            Operation::Union => crate::solid::primitives::join(&[a, b])?,
            Operation::Difference => a,
            Operation::Intersection => crate::solid::primitives::empty(),
        }
    } else if contains(&a, &b)? {
        match operation {
            Operation::Union => a,
            Operation::Intersection => b,
            Operation::Difference => {
                let mut cavity = b;
                cavity.reverse_winding();
                crate::solid::primitives::join(&[a, cavity])?
            }
        }
    } else if contains(&b, &a)? {
        match operation {
            Operation::Union => b,
            Operation::Intersection => a,
            Operation::Difference => crate::solid::primitives::empty(),
        }
    } else if a.positions == b.positions && a.indices == b.indices {
        match operation {
            Operation::Union | Operation::Intersection => a,
            Operation::Difference => Mesh {
                positions: vec![],
                indices: vec![],
                uv: None,
            },
        }
    } else if a.indices.is_empty() || b.indices.is_empty() {
        budget.stage = "clipping";
        match operation {
            Operation::Union => {
                if a.indices.is_empty() {
                    b
                } else {
                    a
                }
            }
            Operation::Difference => a,
            Operation::Intersection => Mesh {
                positions: vec![],
                indices: vec![],
                uv: None,
            },
        }
    } else {
        if !audited {
            return Err(limit());
        }
        fast = false;
        budget.stage = "BSP construction";
        let mut a = Bsp::build(polygons(&a, eps, &mut budget)?, eps, &mut budget)
            .map_err(|e| error(e.code, format!("Build A: {}", e.message)))?;
        let mut b = Bsp::build(polygons(&b, eps, &mut budget)?, eps, &mut budget)
            .map_err(|e| error(e.code, format!("Build B: {}", e.message)))?;
        budget.stage = "clipping";
        match operation {
            Operation::Union => {
                a.clip_to(&b, eps, &mut budget)?;
                b.clip_to(&a, eps, &mut budget)?;
                b.invert();
                b.clip_to(&a, eps, &mut budget)?;
                b.invert();
            }
            Operation::Difference => {
                a.invert();
                a.clip_to(&b, eps, &mut budget)?;
                b.clip_to(&a, eps, &mut budget)?;
                b.invert();
                b.clip_to(&a, eps, &mut budget)?;
                b.invert();
            }
            Operation::Intersection => {
                a.invert();
                b.clip_to(&a, eps, &mut budget)?;
                b.invert();
                a.clip_to(&b, eps, &mut budget)?;
                b.clip_to(&a, eps, &mut budget)?;
            }
        }
        let mut polys = a.polygons();
        polys.extend(b.polygons());
        if operation != Operation::Union {
            for p in &mut polys {
                p.flip();
            }
        }
        budget.stage = "stitch";
        stitch(polys, eps, &mut budget)
            .map_err(|e| error(e.code, format!("Stitch: {}", e.message)))?
    };
    if !fast && result.indices.len() / 3 > options.max_output_triangles {
        return Err(limit());
    }
    // Remove internal triangulation seams before the expensive intersection
    // audit. The audit still checks the final surface, including its topology.
    // Audited fast-path results keep the simplification so their bytes match
    // the pre-fast-path output; above the cap it is skipped because it groups
    // faces per plane and a large curved surface has one plane per triangle.
    if (!fast || audited) && !result.indices.is_empty() {
        // Coplanar retriangulation is an optimization; difficult multi-hole caps
        // may fail its polygon bridge heuristic. Keep the stitched triangles in
        // that case and run the same topology/intersection/orientation audits.
        if let Ok(simplified) = crate::solid::primitives::simplify(&result) {
            result = simplified;
        }
    }
    validate_solid(&result).map_err(|e| {
        error(
            "POLYGON_BOOLEAN_NON_MANIFOLD_RESULT",
            format!(
                "Boolean result is not a valid solid at this tolerance: {}",
                e.message
            ),
        )
    })?;
    if audited {
        budget.stage = "result validation";
        validation::geometry(&result, eps, &mut budget).map_err(|e| {
            if e.code == "POLYGON_BOOLEAN_RESOURCE_LIMIT" {
                e
            } else {
                error("POLYGON_BOOLEAN_NON_MANIFOLD_RESULT", e.message)
            }
        })?;
        validation::orientation(&result, eps, &mut budget)?;
    }
    for p in result.positions.as_chunks_mut::<3>().0 {
        for axis in 0..3 {
            p[axis] = origin[axis] + p[axis] * extent;
        }
    }
    let mut report = result.inspect()?;
    if !result.indices.is_empty() && !report.closed {
        return Err(numeric(
            "World-coordinate precision cannot represent the Boolean result",
        ));
    }
    report.construction = Construction::Boolean;
    report.self_intersection_status = if audited {
        "checked_with_tolerance".into()
    } else {
        "not_checked".into()
    };
    report.boolean = Some(BooleanReport {
        operation,
        tolerance_mm: eps * extent,
        work: budget.work,
        fragments: budget.fragments,
        input_triangles,
    });
    Ok(BuiltMesh {
        mesh: result,
        report,
    })
}
/// Axis-aligned bounds of the vertices a mesh's triangles reference.
pub fn bounds(mesh: &Mesh) -> Option<(Point, Point)> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for &i in &mesh.indices {
        let p = mesh.point(i).ok()?;
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    (!mesh.indices.is_empty()).then_some((min, max))
}
/// Bounds that touch or overlap within `pad` are treated as connected, so face
/// contact is never mistaken for separation.
pub(super) fn bounds_touch(a: Option<(Point, Point)>, b: Option<(Point, Point)>, pad: f64) -> bool {
    match (a, b) {
        (Some((alo, ahi)), Some((blo, bhi))) => {
            (0..3).all(|k| alo[k] <= bhi[k] + pad && blo[k] <= ahi[k] + pad)
        }
        _ => false,
    }
}
/// Connectivity tolerance for operand bounds: relative to the overall extent
/// like the BSP tolerance, so a scaled model behaves like the original.
pub(super) fn bounds_pad(boxes: &[Option<(Point, Point)>]) -> f64 {
    let extent = boxes
        .iter()
        .flatten()
        .flat_map(|(lo, hi)| (0..3).map(move |k| hi[k] - lo[k]))
        .fold(0., f64::max);
    default_tolerance() * 8. * extent.max(1.)
}
/// Groups of operands whose bounds are transitively connected, in first-index
/// order; every operand appears in exactly one group.
pub(super) fn connected_groups(boxes: &[Option<(Point, Point)>], pad: f64) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..boxes.len()).collect();
    pub(super) fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            if bounds_touch(boxes[i], boxes[j], pad) {
                let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                if ri != rj {
                    parent[rj.max(ri)] = rj.min(ri);
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut slot = vec![usize::MAX; boxes.len()];
    for i in 0..boxes.len() {
        let r = root(&mut parent, i);
        if slot[r] == usize::MAX {
            slot[r] = groups.len();
            groups.push(Vec::new());
        }
        groups[slot[r]].push(i);
    }
    groups
}
/// Union of many solids. Operands whose bounds are separated never meet in
/// CSG: each bound-connected group is folded through `pairwise` in operand
/// order and the group results are joined, which is exact for disjoint
/// surfaces. A single operand is returned unchanged.
pub fn union_many(
    meshes: &[Mesh],
    pairwise: &mut dyn FnMut(&Mesh, &Mesh) -> Result<Mesh>,
) -> Result<Mesh> {
    union_many_refs(&meshes.iter().collect::<Vec<_>>(), pairwise)
}
/// Borrowing variant of [`union_many`]: call sites that only hold references
/// avoid deep-cloning every operand's positions/indices at the boundary.
pub fn union_many_refs(
    meshes: &[&Mesh],
    pairwise: &mut dyn FnMut(&Mesh, &Mesh) -> Result<Mesh>,
) -> Result<Mesh> {
    let boxes: Vec<_> = meshes.iter().map(|m| bounds(m)).collect();
    let pad = bounds_pad(&boxes);
    let mut parts = Vec::new();
    for group in connected_groups(&boxes, pad) {
        let mut folded: Option<Mesh> = None;
        for i in group {
            folded = Some(match folded {
                None => (*meshes[i]).clone(),
                Some(m) => pairwise(&m, meshes[i])?,
            });
        }
        if let Some(m) = folded {
            parts.push(m);
        }
    }
    match parts.len() {
        0 => Ok(crate::solid::primitives::empty()),
        1 => Ok(parts.pop().unwrap()),
        _ => crate::solid::primitives::join(&parts),
    }
}
/// `base` minus every cutter, as A \ (B1 ∪ B2 …) = ((A \ B1) \ B2) …. Cutters
/// whose bounds miss the base are skipped; the rest are subtracted in batches
/// of at most `batch` mutually separated cutters, joined without CSG. Small
/// batches keep every BSP step local instead of clipping the whole base
/// against one large joined cutter.
pub fn difference_many(
    base: &Mesh,
    cutters: &[Mesh],
    pairwise: &mut dyn FnMut(&Mesh, &Mesh) -> Result<Mesh>,
    batch: usize,
) -> Result<Mesh> {
    difference_many_refs(base, &cutters.iter().collect::<Vec<_>>(), pairwise, batch)
}
