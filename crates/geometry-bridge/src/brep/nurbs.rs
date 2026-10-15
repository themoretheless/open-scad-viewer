use super::*;

/// A boundary-only triangulation connects boundary samples by chords that
/// are interior to this face. Along a curved authored edge the face across
/// it can pick the same chord between the same two samples, and the two
/// meshes then share an edge that is not authored: four triangles on one 3D
/// edge, no longer a manifold shell. Split every chord between two samples
/// of one authored edge at its midpoint with a private interior vertex;
/// chords lie inside the polygon, so the midpoint does. Samples of a
/// straight edge are collinear and never chorded, so planar faces keep
/// their boundary-only triangulation.
pub(super) fn split_boundary_chords(mesh: &mut FaceMesh, edges_of: &[[usize; 2]]) {
    let same_edge = |p: usize, q: usize| edges_of[p].iter().any(|e| edges_of[q].contains(e));
    loop {
        let mut incidence = BTreeMap::<[usize; 2], Vec<usize>>::new();
        for (index, t) in mesh.triangles.iter().enumerate() {
            for k in 0..3 {
                let (p, q) = (t[k], t[(k + 1) % 3]);
                incidence
                    .entry([p.min(q), p.max(q)])
                    .or_default()
                    .push(index);
            }
        }
        let chord = incidence.iter().find(|([p, q], tris)| {
            tris.len() == 2
                && mesh.shared[*p].is_some()
                && mesh.shared[*q].is_some()
                && same_edge(*p, *q)
        });
        let Some((&[p, q], tris)) = chord else {
            return;
        };
        let (first, second) = (tris[0].min(tris[1]), tris[0].max(tris[1]));
        let m = mesh.uv.len();
        mesh.uv.push([
            (mesh.uv[p][0] + mesh.uv[q][0]) / 2.,
            (mesh.uv[p][1] + mesh.uv[q][1]) / 2.,
        ]);
        mesh.shared.push(None);
        let mut replacement = Vec::with_capacity(4);
        for &index in &[first, second] {
            let t = mesh.triangles[index];
            let x = t.into_iter().find(|&v| v != p && v != q).unwrap();
            // Keep the winding: p -> q -> x becomes p -> m -> x, m -> q -> x.
            if (0..3).any(|k| t[k] == p && t[(k + 1) % 3] == q) {
                replacement.push([p, m, x]);
                replacement.push([m, q, x]);
            } else {
                replacement.push([q, m, x]);
                replacement.push([m, p, x]);
            }
        }
        mesh.triangles.swap_remove(second);
        mesh.triangles.swap_remove(first);
        mesh.triangles.extend(replacement);
    }
}
pub fn nurbs(model: &brep_core::Model, segments: usize) -> Result<Tessellation> {
    nurbs_with_freeform_note(model, segments, FreeformTessNote::SampledUncertified)
}

pub(super) fn nurbs_with_freeform_note(
    model: &brep_core::Model,
    segments: usize,
    freeform_note: FreeformTessNote,
) -> Result<Tessellation> {
    model.validate()?;
    if !(1..=32).contains(&segments) {
        return Err(input("B-rep tessellation segments must be 1..32"));
    }
    let mut registry = EdgeSamplingRegistry::new(model, segments)?;
    let mut face_ids = Vec::new();
    let mut closed_triangles = Vec::new();
    for shell in &model.shells {
        for use_ in &shell.faces {
            let face = &model.faces[use_.face];
            let outer = registry.sample_wire(model, face, face.outer)?;
            let holes = face
                .holes
                .iter()
                .map(|&wire| registry.sample_wire(model, face, wire))
                .collect::<Result<Vec<_>>>()?;
            let built = if is_affine_plane(&face.surface, model.tolerance_mm) {
                triangulate_boundary(&outer, &holes)?
            } else if let Some(rectangular) = rectangular_face(model, face, segments, &registry)? {
                rectangular
            } else if let Some(quarter) = quarter_disk_face(model, face, segments, &registry)? {
                quarter
            } else {
                clamp_interior_to_trims(
                    model,
                    face,
                    refine_trimmed_face(face, triangulate_boundary(&outer, &holes)?, segments)
                        .map_err(|error| {
                            input(format!("Face {} refinement: {}", use_.face, error.message))
                        })?,
                )?
            };
            registry.append(face, use_, built, &mut face_ids)?;
            closed_triangles.resize(face_ids.len(), shell.closed);
        }
    }
    registry.verify_boundary_uses(model, &face_ids)?;
    let topology_face_ids = face_ids
        .iter()
        .map(|&face| model.1.faces[face].to_string())
        .collect();
    let freeform_faces = model
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1);
    let note = if freeform_faces {
        freeform_note
    } else {
        FreeformTessNote::None
    };
    let mut result = finish_indexed_mode(
        registry.mesh,
        face_ids,
        Some(topology_face_ids),
        !model.shells.is_empty() && model.shells.iter().all(|s| s.closed),
        note,
    )?;
    result.closed_triangles = Some(closed_triangles);
    if let Some(certificate) = &mut result.certificate {
        certificate.shell_count = model.shells.len();
    }
    Ok(result)
}

/// Certified finite tessellation for independently recognized analytic shells.
/// Shared-edge registry incidence supplies no-crack/orientation proof; curved
/// deviation is selected from closed-form sphere/cone/torus/cylinder bounds.
pub fn certified_nurbs(
    model: &brep_core::Model,
    chord_tolerance_mm: f64,
    max_triangles: usize,
) -> Result<CertifiedTessellation> {
    use brep_core::predicate_evidence::{PredicateEvidence, compose_predicate_evidence};
    if !(chord_tolerance_mm.is_finite() && chord_tolerance_mm > 0.)
        || !(12..=20_000).contains(&max_triangles)
    {
        return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_OPTIONS_INVALID",
            "Chord tolerance must be positive and triangle budget 12..20000",
        ));
    }
    model.validate()?;
    let audit = brep_core::solid_audit::audit_solid(model)?;
    // Classify before adaptive search so unsupported geometry is a typed
    // refusal, distinct from a proved family exhausting the finite budget.
    brep_core::analysis::certified_tessellation_deviation(model, 1)?;
    let segments = (1..=32)
        .find(|segments| {
            brep_core::analysis::certified_tessellation_deviation(model, *segments)
                .is_ok_and(|deviation| deviation <= chord_tolerance_mm)
        })
        .ok_or_else(|| nurbs_core::Error::new(
            "BREP_TESSELLATION_BUDGET_EXHAUSTED",
            "Requested two-sided analytic deviation needs more than 32 subdivisions per patch, or a shell is outside the certified finite matrix",
        ))?;
    let deviation = brep_core::analysis::certified_tessellation_deviation(model, segments)?;
    let tessellation = nurbs(model, segments)?;
    if tessellation.built.mesh.indices.len() / 3 > max_triangles {
        return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_BUDGET_EXHAUSTED",
            "Certified tessellation exceeded its triangle budget",
        ));
    }
    let certificate = tessellation.certificate.as_ref().ok_or_else(|| {
        nurbs_core::Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Tessellation did not publish incidence coverage",
        )
    })?;
    let naming_complete = model.persistent_naming_complete()
        && model.1.faces.len() == model.faces.len()
        && model.1.edges.len() == model.edges.len();
    if !certificate.complete
        || !certificate.closed_shells
        || !tessellation.built.report.closed
        || !naming_complete
    {
        return Err(nurbs_core::Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Coverage, audit, ChangeSet, or naming evidence is incomplete",
        ));
    }
    let context = model.tolerance_context()?;
    let evidence = compose_predicate_evidence(
        &context,
        [
            // The analytic sagitta is recorded separately as the two-sided
            // tessellation enclosure. Predicate evidence binds exact authored
            // edge correspondence to the model tolerance context.
            PredicateEvidence::positional(&context, 0., chord_tolerance_mm)?,
            PredicateEvidence::topology_preservation(
                &context,
                "shared-edge identity, orientation, and no T-junction coverage",
                true,
            )?,
        ],
    )?;
    Ok(CertifiedTessellation {
        capability: CERTIFIED_TESSELLATION_CAPABILITY,
        context: context.spec_identity(),
        surface_to_mesh_deviation_mm: deviation,
        mesh_to_surface_deviation_mm: deviation,
        audit,
        evidence,
        change_set: model.1.change_set.clone(),
        naming_complete,
        max_triangles,
        subdivisions_per_patch: segments,
        tessellation,
    })
}

/// Certified freeform tessellation finite cell: equal-weight clamped Bezier
/// shells with Bernstein second-difference deviation (capability /1).
pub fn certified_freeform_nurbs(
    model: &brep_core::Model,
    chord_tolerance_mm: f64,
    max_triangles: usize,
) -> Result<CertifiedTessellation> {
    use brep_core::predicate_evidence::{PredicateEvidence, compose_predicate_evidence};
    if !(chord_tolerance_mm.is_finite() && chord_tolerance_mm > 0.)
        || !(12..=20_000).contains(&max_triangles)
    {
        return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_OPTIONS_INVALID",
            "Chord tolerance must be positive and triangle budget 12..20000",
        ));
    }
    model.validate()?;
    let audit = brep_core::solid_audit::audit_solid(model)?;
    brep_core::analysis::certified_freeform_tessellation_deviation(model, 1)?;
    let segments = (1..=32)
        .find(|segments| {
            brep_core::analysis::certified_freeform_tessellation_deviation(model, *segments)
                .is_ok_and(|deviation| deviation <= chord_tolerance_mm)
        })
        .ok_or_else(|| {
            nurbs_core::Error::new(
                "BREP_TESSELLATION_BUDGET_EXHAUSTED",
                "Requested freeform Bernstein deviation needs more than 32 subdivisions per patch",
            )
        })?;
    let deviation =
        brep_core::analysis::certified_freeform_tessellation_deviation(model, segments)?;
    let tessellation =
        nurbs_with_freeform_note(model, segments, FreeformTessNote::BernsteinCertified)?;
    if tessellation.built.mesh.indices.len() / 3 > max_triangles {
        return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_BUDGET_EXHAUSTED",
            "Certified freeform tessellation exceeded its triangle budget",
        ));
    }
    let certificate = tessellation.certificate.as_ref().ok_or_else(|| {
        nurbs_core::Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Tessellation did not publish incidence coverage",
        )
    })?;
    let naming_complete = model.persistent_naming_complete()
        && model.1.faces.len() == model.faces.len()
        && model.1.edges.len() == model.edges.len();
    if !certificate.complete
        || !certificate.closed_shells
        || !tessellation.built.report.closed
        || !naming_complete
    {
        return Err(nurbs_core::Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Coverage, audit, ChangeSet, or naming evidence is incomplete",
        ));
    }
    let context = model.tolerance_context()?;
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::positional(&context, 0., chord_tolerance_mm)?,
            PredicateEvidence::topology_preservation(
                &context,
                "shared-edge identity, orientation, and Bernstein freeform deviation",
                true,
            )?,
        ],
    )?;
    Ok(CertifiedTessellation {
        capability: FREEFORM_TESSELLATION_CAPABILITY,
        context: context.spec_identity(),
        surface_to_mesh_deviation_mm: deviation,
        mesh_to_surface_deviation_mm: deviation,
        audit,
        evidence,
        change_set: model.1.change_set.clone(),
        naming_complete,
        max_triangles,
        subdivisions_per_patch: segments,
        tessellation,
    })
}
pub(super) fn rectangular_face(
    model: &brep_core::Model,
    face: &brep_core::Face,
    _segments: usize,
    registry: &EdgeSamplingRegistry,
) -> Result<Option<FaceMesh>> {
    let coedges = &model.loops[face.outer].coedges;
    let corners = model.loop_uv(face.outer, 1)?;
    let min_u = corners
        .iter()
        .map(|point| point[0])
        .fold(f64::INFINITY, f64::min);
    let max_u = corners
        .iter()
        .map(|point| point[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let min_v = corners
        .iter()
        .map(|point| point[1])
        .fold(f64::INFINITY, f64::min);
    let max_v = corners
        .iter()
        .map(|point| point[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let rectangular = face.holes.is_empty()
        && corners.len() >= 4
        && coedges.iter().all(|c| {
            c.pcurve.degree == 1
                && c.pcurve.control_points.len() == 2
                && c.pcurve.weights.iter().all(|w| *w == c.pcurve.weights[0])
                && {
                    let a = &c.pcurve.control_points[0];
                    let b = &c.pcurve.control_points[1];
                    (a[0] == b[0] || a[1] == b[1])
                        && [a, b].iter().all(|p| {
                            p[0] == min_u || p[0] == max_u || p[1] == min_v || p[1] == max_v
                        })
                }
        })
        && max_u > min_u
        && max_v > min_v;
    if !rectangular {
        return Ok(None);
    }
    let horizontal = |v: f64| {
        coedges
            .iter()
            .filter(|coedge| {
                let points = &coedge.pcurve.control_points;
                points[0][1] == v && points[1][1] == v && !model.edges[coedge.edge].degenerate
            })
            .map(|coedge| registry.divisions(coedge))
            .sum::<usize>()
    };
    let vertical = |u: f64| {
        coedges
            .iter()
            .filter(|coedge| {
                let points = &coedge.pcurve.control_points;
                points[0][0] == u && points[1][0] == u && !model.edges[coedge.edge].degenerate
            })
            .map(|coedge| registry.divisions(coedge))
            .sum::<usize>()
    };
    let segments_u = horizontal(min_v).max(horizontal(max_v));
    let segments_v = vertical(min_u).max(vertical(max_u));
    if segments_u == 0 || segments_v == 0 {
        return Err(input("Rectangular patch has no ordinary boundary schedule"));
    }
    let poles: Vec<_> = coedges
        .iter()
        .filter(|c| model.edges[c.edge].degenerate)
        .map(|c| model.edges[c.edge].vertices[0])
        .collect();
    let mut boundary = BTreeMap::<(usize, usize), usize>::new();
    let mut pole_rows = BTreeMap::<usize, usize>::new();
    for coedge in coedges {
        let divisions = registry.divisions(coedge);
        let domain = coedge.pcurve.domain();
        for sample in 0..=divisions {
            let t = sample as f64 / divisions as f64;
            let uv = coedge
                .pcurve
                .evaluate(domain[0] + t * (domain[1] - domain[0]))?
                .point;
            let key = (
                ((uv[0] - min_u) / (max_u - min_u) * segments_u as f64).round() as usize,
                ((uv[1] - min_v) / (max_v - min_v) * segments_v as f64).round() as usize,
            );
            let position = registry.position(coedge, sample)?;
            boundary.insert(key, position);
            if model.edges[coedge.edge].degenerate {
                pole_rows.insert(key.1, position);
            }
        }
    }
    let mut out = FaceMesh {
        uv: vec![],
        shared: vec![],
        triangles: vec![],
    };
    for y in 0..=segments_v {
        for x in 0..=segments_u {
            let shared = boundary
                .get(&(x, y))
                .copied()
                .or_else(|| pole_rows.get(&y).copied());
            out.uv.push([
                min_u + (max_u - min_u) * x as f64 / segments_u as f64,
                min_v + (max_v - min_v) * y as f64 / segments_v as f64,
            ]);
            out.shared.push(shared);
        }
    }
    for y in 0..segments_v {
        for x in 0..segments_u {
            let a = y * (segments_u + 1) + x;
            let b = a + 1;
            let d = a + segments_u + 1;
            let c = d + 1;
            for triangle in [[a, b, c], [a, c, d]] {
                let repeated = (0..3).find_map(|i| {
                    let first = out.shared[triangle[i]];
                    first.filter(|_| first == out.shared[triangle[(i + 1) % 3]])
                });
                if let Some(vertex) = repeated {
                    if !poles.contains(&vertex) {
                        return Err(input(
                            "Sampling resolution collapses a nondegenerate boundary",
                        ));
                    }
                    continue;
                }
                out.triangles.push(triangle);
            }
        }
    }
    Ok(Some(out))
}
pub(super) fn quarter_disk_face(
    model: &brep_core::Model,
    face: &brep_core::Face,
    segments: usize,
    registry: &EdgeSamplingRegistry,
) -> Result<Option<FaceMesh>> {
    let coedges = &model.loops[face.outer].coedges;
    if !face.holes.is_empty() || coedges.len() != 3 {
        return Ok(None);
    }
    let a = &coedges[0].pcurve;
    let arc = &coedges[1].pcurve;
    let b = &coedges[2].pcurve;
    let bezier = |c: &Curve| {
        let [low, high] = c.domain();
        c.knots[..=c.degree].iter().all(|k| *k == low)
            && c.knots[c.control_points.len()..].iter().all(|k| *k == high)
    };
    let straight = |c: &Curve, points: &[Vec<f64>]| {
        bezier(c) && c.degree == 1 && c.control_points == points && c.weights[0] == c.weights[1]
    };
    let quarter = straight(a, &[vec![0., 0.], vec![1., 0.]])
        && straight(b, &[vec![0., 1.], vec![0., 0.]])
        && bezier(arc)
        && arc.degree == 2
        && arc.control_points == vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]]
        // The sphere's exact binary rational traversal has a different
        // parameter speed from the symmetric quarter arc. Both trace the
        // same quarter circle; evaluate the authored curve below so the
        // interior grid retains its shared edge sample schedule.
        && (arc.weights == vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
            || arc.weights == vec![1., 1., 2.]);
    if !quarter {
        return Ok(None);
    }
    if coedges.iter().any(|c| registry.divisions(c) != segments) {
        return Err(input("Quarter-disk edge schedules disagree"));
    }
    let mut out = FaceMesh {
        uv: vec![[0., 0.]],
        shared: vec![Some(registry.position(&coedges[0], 0)?)],
        triangles: vec![],
    };
    let [low, high] = arc.domain();
    for row in 1..=segments {
        for column in 0..=segments {
            let p = arc
                .evaluate(low + (high - low) * column as f64 / segments as f64)?
                .point;
            let radius = row as f64 / segments as f64;
            out.uv.push([p[0] * radius, p[1] * radius]);
            out.shared.push(if row == segments {
                Some(registry.position(&coedges[1], column)?)
            } else if column == 0 {
                Some(registry.position(&coedges[0], row)?)
            } else if column == segments {
                Some(registry.position(&coedges[2], segments - row)?)
            } else {
                None
            });
        }
    }
    for j in 0..segments {
        out.triangles.push([0, 1 + j, 2 + j]);
    }
    for row in 1..segments {
        for j in 0..segments {
            let a = 1 + (row - 1) * (segments + 1) + j;
            let b = a + 1;
            let d = a + segments + 1;
            let c = d + 1;
            out.triangles.extend([[a, d, c], [a, c, b]]);
        }
    }
    Ok(Some(out))
}
