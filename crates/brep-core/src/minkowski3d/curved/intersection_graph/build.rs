use super::*;
/// Every original face pair must either separate over its WHOLE actual trim,
/// or possess complete original-root/physical-arc coverage and a complete exact
/// trim-boundary event census. Shared roots use original edge identities and
/// strict curve-parameter ranks; no world-distance welding occurs.
/// Current sufficient proofs refuse singular/coincident strata, mixed-clock
/// physical components, seam/seam coincidences and unresolved event order.
pub fn certify_nurbs_intersection_graph(
    a: &Model,
    b: &Model,
    limits: NurbsGraphLimits,
) -> Result<QualifiedNurbsIntersectionGraph> {
    a.validate()?;
    b.validate()?;
    if limits.source_depth > 3
        || !(1..=1000000).contains(&limits.max_cells)
        || !(1..=1000000).contains(&limits.max_rectangles)
        || limits.max_exact_work > 1000000
        || !(1..=4096).contains(&limits.max_roots)
        || [a, b].iter().any(|m| !(1..=16).contains(&m.faces.len()))
    {
        return Err(fail(
            "intersection graph needs bounded nonempty source Models and proof allowances",
        ));
    }
    for model in [a, b] {
        let mut shell_uses = vec![0usize; model.shells.len()];
        let mut face_uses = vec![0usize; model.faces.len()];
        for body in &model.bodies {
            for shell in std::iter::once(body.outer_shell).chain(body.inner_shells.iter().copied())
            {
                shell_uses[shell] += 1;
            }
        }
        for shell in &model.shells {
            for u in &shell.faces {
                face_uses[u.face] += 1;
            }
        }
        if model.bodies.is_empty()
            || !model.persistent_naming_complete()
            || model.shells.iter().any(|s| !s.closed)
            || shell_uses.iter().any(|&n| n != 1)
            || face_uses.iter().any(|&n| n != 1)
        {
            return Err(fail(
                "intersection graph requires complete original identities and uniquely owned closed boundary shells/faces",
            ));
        }
    }
    atlas::boolean::source_boundary(a)?;
    atlas::boolean::source_boundary(b)?;
    let atlases = [
        certify_source_atlas(a, limits.source_depth)?,
        certify_source_atlas(b, limits.source_depth)?,
    ];
    let face_bounds = [bounds::model(a)?, bounds::model(b)?];
    let mut build = Build {
        models: [a, b],
        limits,
        work: NurbsGraphWork::default(),
        queries: BTreeMap::new(),
    };
    let mut graphs = Vec::new();
    for fa in 0..a.faces.len() {
        for fb in 0..b.faces.len() {
            graphs.push(pairs::construct(
                &mut build,
                [fa, fb],
                &face_bounds[0][fa],
                &face_bounds[1][fb],
            )?);
        }
    }
    let (vertices, arcs, cycles) = pairs::join(&graphs)?;
    let certificate = NurbsIntersectionGraphCertificate {
        version: 1,
        sources: [
            source_id(&Source::Brep(a.clone()))?,
            source_id(&Source::Brep(b.clone()))?,
        ],
        limits,
        atlases,
        face_bounds,
        pairs: graphs,
        vertices,
        arcs,
        cycles,
        work: build.work,
    };
    Ok(QualifiedNurbsIntersectionGraph { certificate })
}
pub fn verify_nurbs_intersection_graph(
    a: &Model,
    b: &Model,
    certificate: &NurbsIntersectionGraphCertificate,
) -> Result<QualifiedNurbsIntersectionGraph> {
    if certificate.version != 1 {
        return Err(fail("intersection graph certificate version mismatch"));
    }
    if a.faces.is_empty()
        || b.faces.is_empty()
        || certificate.pairs.len() != a.faces.len().saturating_mul(b.faces.len())
        || certificate
            .pairs
            .iter()
            .enumerate()
            .any(|(i, p)| p.faces != [i / b.faces.len(), i % b.faces.len()])
        || certificate.sources
            != [
                source_id(&Source::Brep(a.clone()))?,
                source_id(&Source::Brep(b.clone()))?,
            ]
    {
        return Err(fail(
            "intersection graph omitted original face pairs or changed its source binding",
        ));
    }
    // Rejection-only structural preflight. These caller fields acquire no
    // geometry authority: the full original-source reconstruction still follows.
    let (vertices, arcs, cycles) = pairs::join(&certificate.pairs)?;
    if vertices != certificate.vertices || arcs != certificate.arcs || cycles != certificate.cycles
    {
        return Err(fail("intersection graph seam/cycle census is inconsistent"));
    }
    let fresh = certify_nurbs_intersection_graph(a, b, certificate.limits)?;
    if fresh.certificate != *certificate {
        return Err(fail(
            "intersection coverage, trim partition, original seam identity or cycle census changed",
        ));
    }
    Ok(fresh)
}
