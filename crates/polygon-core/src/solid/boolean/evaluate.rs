use super::*;

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
        let mut cmin = [f64::INFINITY; 3];
        let mut cmax = [f64::NEG_INFINITY; 3];
        for p in m.positions.as_chunks::<3>().0 {
            for k in 0..3 {
                cmin[k] = cmin[k].min(p[k]);
                cmax[k] = cmax[k].max(p[k]);
            }
        }
        std::array::from_fn::<_, 3, _>(|k| cmin[k] / 2. + cmax[k] / 2.)
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
fn bounds_touch(a: Option<(Point, Point)>, b: Option<(Point, Point)>, pad: f64) -> bool {
    match (a, b) {
        (Some((alo, ahi)), Some((blo, bhi))) => {
            (0..3).all(|k| alo[k] <= bhi[k] + pad && blo[k] <= ahi[k] + pad)
        }
        _ => false,
    }
}
/// Connectivity tolerance for operand bounds: relative to the overall extent
/// like the BSP tolerance, so a scaled model behaves like the original.
fn bounds_pad(boxes: &[Option<(Point, Point)>]) -> f64 {
    let extent = boxes
        .iter()
        .flatten()
        .flat_map(|(lo, hi)| (0..3).map(move |k| hi[k] - lo[k]))
        .fold(0., f64::max);
    default_tolerance() * 8. * extent.max(1.)
}
/// Groups of operands whose bounds are transitively connected, in first-index
/// order; every operand appears in exactly one group.
fn connected_groups(boxes: &[Option<(Point, Point)>], pad: f64) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..boxes.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
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
/// Borrowing variant of [`difference_many`]: avoids deep-cloning the cutters
/// at call sites that only hold references.
pub fn difference_many_refs(
    base: &Mesh,
    cutters: &[&Mesh],
    pairwise: &mut dyn FnMut(&Mesh, &Mesh) -> Result<Mesh>,
    batch: usize,
) -> Result<Mesh> {
    let batch = batch.max(1);
    let base_box = bounds(base);
    if base_box.is_none() {
        return Ok(crate::solid::primitives::empty());
    }
    let boxes: Vec<_> = cutters.iter().map(|m| bounds(m)).collect();
    let pad = bounds_pad(&[&[base_box][..], &boxes[..]].concat());
    let mut batches: Vec<Vec<usize>> = Vec::new();
    for (i, cutter_box) in boxes.iter().enumerate() {
        if !bounds_touch(base_box, *cutter_box, pad) {
            continue;
        }
        let open = batches.iter_mut().find(|members| {
            members.len() < batch
                && members
                    .iter()
                    .all(|&j| !bounds_touch(boxes[j], *cutter_box, pad))
        });
        match open {
            Some(members) => members.push(i),
            None => batches.push(vec![i]),
        }
    }
    let mut result = base.clone();
    for members in batches {
        let cutter = if members.len() == 1 {
            (*cutters[members[0]]).clone()
        } else {
            let parts: Vec<Mesh> = members.iter().map(|&j| (*cutters[j]).clone()).collect();
            crate::solid::primitives::join(&parts)?
        };
        result = pairwise(&result, &cutter)?;
        if result.indices.is_empty() {
            break;
        }
    }
    Ok(result)
}

/// Reconnect triangulated planar face boundaries after exact planar remeshing.
pub(crate) fn stitch_mesh(mesh: &Mesh, eps: f64) -> Result<Mesh> {
    let mut budget = Budget {
        stage: "stitch",
        work: 0,
        fragments: 0,
        options: Options::default(),
    };
    let p = polygons(mesh, eps, &mut budget)?;
    stitch(p, eps, &mut budget)
}


/// First disallowed contact at a relative tolerance, not an exact B-rep certificate.
/// Triangle indices refer to the original input. Degenerate inputs are refused.
#[derive(Clone, Debug)]
pub struct MeshIntersectionLocation {
    pub triangles: [usize; 2],
    pub point: Point,
    pub shared_vertices: usize,
    pub tolerance_mm: f64,
}
pub fn first_mesh_intersection(mesh: &Mesh, relative_tolerance: f64, max_work: usize) -> Result<Option<MeshIntersectionLocation>> {
    let (normalized,origin,extent)=intersection_mesh(mesh,relative_tolerance,max_work)?;
    let mut budget=Budget{stage:"mesh intersection inspection",work:0,fragments:0,
        options:Options{max_work,..Options::default()}};
    Ok(validation::first_geometry_failure(&normalized,relative_tolerance,&mut budget)?.map(|(a,b,p,shared)|MeshIntersectionLocation{
        triangles:[a,b],point:std::array::from_fn(|i|p[i]*extent+origin[i]),shared_vertices:shared,tolerance_mm:relative_tolerance*extent,
    }))
}

fn intersection_mesh(mesh:&Mesh,relative_tolerance:f64,max_work:usize)->Result<(Mesh,Point,f64)> {
    mesh.validate()?;
    if !(1e-12..=1e-5).contains(&relative_tolerance) || max_work==0 || max_work>8_000_000 {
        return Err(invalid("Invalid intersection tolerance or work budget"));
    }
    if mesh.indices.is_empty(){return Ok((mesh.clone(),[0.;3],1.));}
    let (lo,hi)=bounds(mesh).ok_or_else(||invalid("Mesh bounds unavailable"))?;
    let extent=(0..3).map(|i|hi[i]-lo[i]).fold(0.,f64::max);
    if !extent.is_finite() || extent<=0. {return Err(invalid("Degenerate mesh bounds"));}
    let origin:Point=std::array::from_fn(|i|lo[i]/2.+hi[i]/2.);
    let mut normalized=mesh.clone();
    for p in normalized.positions.chunks_exact_mut(3) {
        for i in 0..3 {p[i]=(p[i]-origin[i])/extent;}
    }
    for t in normalized.indices.chunks_exact(3) {
        let a=normalized.point(t[0])?;let b=normalized.point(t[1])?;let c=normalized.point(t[2])?;
        if norm(cross(sub(b,a),sub(c,a)))<=relative_tolerance*relative_tolerance {
            return Err(invalid("Degenerate triangle: repair it before intersection inspection"));
        }
    }
    Ok((normalized,origin,extent))
}

/// Every disallowed triangle pair, with explicit incomplete results at resource limits.
/// This checks the displayed triangle mesh at a relative tolerance, not the source B-rep.
#[derive(Clone, Debug)]
pub struct MeshIntersectionReport {
    pub contacts: Vec<MeshIntersectionLocation>,
    pub complete: bool,
    pub stop_reason: Option<&'static str>,
    pub work: usize,
}
pub fn mesh_intersections(mesh:&Mesh,relative_tolerance:f64,max_work:usize,max_contacts:usize)->Result<MeshIntersectionReport> {
    if max_contacts==0 || max_contacts>100_000 {return Err(invalid("Invalid intersection contact limit"));}
    let (normalized,origin,extent)=intersection_mesh(mesh,relative_tolerance,max_work)?;
    let mut budget=Budget {stage:"mesh intersection inspection",work:0,fragments:0,options:Options{max_work,..Options::default()}};
    let mut report=validation::geometry_failures(&normalized,relative_tolerance,&mut budget,Some(max_contacts))?;
    report.contacts.sort_by_key(|&(a,b,_,_)|(a,b));
    Ok(MeshIntersectionReport {
        contacts:report.contacts.into_iter().map(|(a,b,p,shared)|MeshIntersectionLocation {
            triangles:[a,b],point:std::array::from_fn(|i|p[i]*extent+origin[i]),shared_vertices:shared,tolerance_mm:relative_tolerance*extent,
        }).collect(),
        complete:report.complete,stop_reason:report.stop_reason,work:budget.work.min(max_work),
    })
}
