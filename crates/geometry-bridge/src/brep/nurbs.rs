use super::*;

pub fn nurbs(model: &brep_core::Model, segments: usize) -> Result<Tessellation> {
    nurbs_with_freeform_note(model, segments, FreeformTessNote::SampledUncertified)
}

pub(super) fn nurbs_with_freeform_note(
    model: &brep_core::Model,
    segments: usize,
    freeform_note: FreeformTessNote,
) -> Result<Tessellation> {
    nurbs_with_schedule(model, segments, freeform_note, true)
}
pub(super) fn nurbs_with_schedule(
    model: &brep_core::Model,
    segments: usize,
    freeform_note: FreeformTessNote,
    reduce_linear_rulings: bool,
) -> Result<Tessellation> {
    model.validate()?;
    if !(1..=32).contains(&segments) {
        return Err(input("B-rep tessellation segments must be 1..32"));
    }
    let mut registry = EdgeSamplingRegistry::new(model, segments, reduce_linear_rulings)?;
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
            registry.append(model, use_, built, &mut face_ids, reduce_linear_rulings)?;
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

/// Certified finite tessellation. Shared-edge registry incidence supplies
/// no-crack/orientation proof; curved deviation is selected from closed-form
/// sphere/cone/torus/cylinder bounds, or for general NURBS faces (variable
/// weights, lofts, curved sweeps) from per-cell bounds on the emitted mesh.
pub fn certified_nurbs(
    model: &brep_core::Model,
    chord_tolerance_mm: f64,
    max_triangles: usize,
) -> Result<CertifiedTessellation> {
    use brep_core::predicate_evidence::{compose_predicate_evidence, PredicateEvidence};
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
    // Classify before reporting budget exhaustion so unsupported geometry is a
    // typed refusal. A count refuses only when every count refuses: general
    // cells may straddle a C0 knot at coarse grids yet certify on finer ones.
    let mut first_error = None;
    let mut certified_any = false;
    let mut chosen = None;
    for segments in 1..=32 {
        match certified_deviation(model, segments) {
            Ok((deviation, _)) => {
                certified_any = true;
                if deviation <= chord_tolerance_mm {
                    chosen = Some(segments);
                    break;
                }
            }
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    let segments = match (chosen, certified_any, first_error) {
        (Some(segments), _, _) => segments,
        (None, false, Some(error)) => return Err(error),
        _ => return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_BUDGET_EXHAUSTED",
            "Requested two-sided deviation needs more than 32 subdivisions per patch, or a shell is outside the certified finite matrix",
        )),
    };
    let (deviation, capability) = certified_deviation(model, segments)?;
    let tessellation =
        nurbs_with_schedule(model, segments, FreeformTessNote::SampledUncertified, false)?;
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
        capability,
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
    use brep_core::predicate_evidence::{compose_predicate_evidence, PredicateEvidence};
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
    let mut pole_columns = BTreeMap::<usize, usize>::new();
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
                // Quotient charts may collapse either parameter direction.
                // Only the grid line owned by this edge maps to its pole.
                if coedge.pcurve.control_points[0][0] == coedge.pcurve.control_points[1][0] {
                    pole_columns.insert(key.0, position);
                } else {
                    pole_rows.insert(key.1, position);
                }
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
                .or_else(|| pole_rows.get(&y).copied())
                .or_else(|| pole_columns.get(&x).copied());
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

/// Display levels for the viewer LOD selector: one tessellation per requested
/// subdivision count, each paired with its certified two-sided deviation.
/// Counts must be strictly increasing within 1..=32. A model outside the
/// certified analytic families and the general full-chart NURBS cell is a
/// typed refusal; a model whose bounds do not
/// strictly decrease (all-planar bodies have zero deviation at every count)
/// returns `None`, because one level already displays it exactly.
pub fn certified_display_levels(
    model: &brep_core::Model,
    segments: &[usize],
) -> Result<Option<Vec<(usize, f64, Tessellation)>>> {
    if !(2..=3).contains(&segments.len())
        || segments.windows(2).any(|pair| pair[0] >= pair[1])
        || segments.iter().any(|count| !(1..=32).contains(count))
    {
        return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_OPTIONS_INVALID",
            "Display levels need two or three strictly increasing subdivision counts in 1..32",
        ));
    }
    model.validate()?;
    let bounds = segments
        .iter()
        .map(|&count| certified_deviation(model, count).map(|(bound, _)| bound))
        .collect::<Result<Vec<_>>>()?;
    if bounds.windows(2).any(|pair| pair[0] <= pair[1]) {
        return Ok(None);
    }
    segments
        .iter()
        .zip(bounds)
        .map(|(&count, bound)| {
            Ok((
                count,
                bound,
                nurbs_with_schedule(model, count, FreeformTessNote::SampledUncertified, false)?,
            ))
        })
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

pub struct PeriodicTrimSheetDisplay {
    pub sheet: brep_core::feature_ops::periodic_trim_sheet::PeriodicTrimSheetReport,
    pub tessellation: Option<Tessellation>,
}

/// Build exact trimmed NURBS sheets and sample them with the existing
/// B-rep tessellator. Each face keeps its identity in the display mesh.
pub fn tessellate_exact_periodic_trim_sheet(
    source: &nurbs_core::surface::Surface,
    contours: &[&[[f64; 2]]],
    tolerance_mm: f64,
    max_work: u64,
    segments: usize,
) -> Result<PeriodicTrimSheetDisplay> {
    if !(1..=32).contains(&segments) {
        return Err(input("B-rep tessellation segments must be 1..32"));
    }
    let sheet = brep_core::feature_ops::periodic_trim_sheet::build_exact_periodic_trim_sheet(
        source,
        contours,
        tolerance_mm,
        max_work,
    )?;
    let tessellation = sheet
        .model
        .as_ref()
        .map(|model| nurbs(model, segments))
        .transpose()?;
    Ok(PeriodicTrimSheetDisplay {
        sheet,
        tessellation,
    })
}

/// Decode a periodic trim sheet request and serialize the exact B-rep or its
/// refusal stage for the shared native and WASM JSON interface.
pub fn exact_periodic_trim_sheet_request(
    request: &value_codec::Value,
) -> Result<value_codec::Value> {
    periodic_trim_sheet_response(request, PeriodicTrimBindings::None)
}

/// Build an exact sheet with the original contour, segment and interval of each edge.
pub fn exact_periodic_trim_sheet_bindings_request(
    request: &value_codec::Value,
) -> Result<value_codec::Value> {
    periodic_trim_sheet_response(request, PeriodicTrimBindings::Oriented)
}

/// Serialize directed parameters in caller sample order as exact rational strings.
pub fn exact_periodic_trim_sheet_input_bindings_request(
    request: &value_codec::Value,
) -> Result<value_codec::Value> {
    periodic_trim_sheet_response(request, PeriodicTrimBindings::Input)
}

enum PeriodicTrimBindings {
    None,
    Oriented,
    Input,
}

fn periodic_trim_sheet_response(
    request: &value_codec::Value,
    bindings: PeriodicTrimBindings,
) -> Result<value_codec::Value> {
    use crate::field;
    use brep_core::feature_ops::periodic_trim_sheet::{
        build_exact_periodic_trim_sheet, build_exact_periodic_trim_sheet_with_edge_bindings,
        build_exact_periodic_trim_sheet_with_input_bindings,
    };
    use nurbs_core::editing::exact_periodic_surface::PeriodicTrimEdgeRefusal;
    use value_codec::json;

    let source: nurbs_core::surface::Surface = field(request, "surface")?;
    let contours: Vec<Vec<[f64; 2]>> = field(request, "contours")?;
    let tolerance_mm: f64 = field(request, "toleranceMm")?;
    let max_work: u64 = field(request, "maxWork")?;
    if !(1..=1_000_000).contains(&max_work) {
        return Err(input("Periodic trim sheet maxWork must be 1..=1000000"));
    }

    let contours = contours.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let (report, bindings) = match bindings {
        PeriodicTrimBindings::Oriented => {
            let result = build_exact_periodic_trim_sheet_with_edge_bindings(
                &source,
                &contours,
                tolerance_mm,
                max_work,
            )?;
            let bindings = result
                .edge_bindings
                .into_iter()
                .map(|binding| {
                    json!({
                        "edgeId": binding.edge_id,
                        "contour": binding.source.contour,
                        "segment": binding.source.segment,
                        "interval": binding.source.interval,
                    })
                })
                .collect::<Vec<_>>();
            (result.bound_sheet.sheet, Some(bindings))
        }
        PeriodicTrimBindings::Input => {
            let result = build_exact_periodic_trim_sheet_with_input_bindings(
                &source,
                &contours,
                tolerance_mm,
                max_work,
            )?;
            let bindings = result
                .edge_bindings
                .into_iter()
                .map(|binding| {
                    json!({
                        "edgeId": binding.edge_id,
                        "contour": binding.source.contour,
                        "segment": binding.source.segment,
                        "parameters": binding.source.parameters.map(|t| json!({
                            "numerator": t.numer().to_string(),
                            "denominator": t.denom().to_string(),
                        })),
                    })
                })
                .collect::<Vec<_>>();
            (result.bound_sheet.sheet, Some(bindings))
        }
        PeriodicTrimBindings::None => (
            build_exact_periodic_trim_sheet(&source, &contours, tolerance_mm, max_work)?,
            None,
        ),
    };
    let refusal = report.refusal.map(|reason| match reason {
        PeriodicTrimEdgeRefusal::Patch(_) => "patch",
        PeriodicTrimEdgeRefusal::Lift(_) => "lift",
        PeriodicTrimEdgeRefusal::Restrict(_) => "restrict",
    });
    let mut response = json!({
        "model": report.model,
        "workUsed": report.work_used,
        "refusalStage": refusal,
    });
    if let Some(bindings) = bindings {
        response["edgeBindings"] = json!(bindings);
    }
    Ok(response)
}

/// Audit the joins of one original contour using a shared build and audit budget.
pub fn exact_periodic_trim_contour_audit_request(
    request: &value_codec::Value,
) -> Result<value_codec::Value> {
    use crate::field;
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimTangentJoin, build_exact_periodic_surface_trim_input_edges,
    };
    use value_codec::json;

    let source: nurbs_core::surface::Surface = field(request, "surface")?;
    let contours: Vec<Vec<[f64; 2]>> = field(request, "contours")?;
    let contour: usize = field(request, "contour")?;
    let max_work: u64 = field(request, "maxWork")?;
    if contour >= contours.len() {
        return Err(input("Periodic trim audit contour index is out of range"));
    }
    let contours = contours.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let edges = build_exact_periodic_surface_trim_input_edges(&source, &contours, max_work)?;
    let build_work = edges.geometry.work_used;
    if edges.geometry.patches.is_none() {
        return Ok(json!({"joins": null, "workUsed": build_work, "refusalStage": "build"}));
    }
    if build_work == max_work {
        return Ok(json!({"joins": null, "workUsed": build_work, "refusalStage": "audit"}));
    }
    let audit = edges.audit_input_contour_joins(contour, max_work - build_work)?;
    let joins = audit.joins.map(|joins| {
        joins
            .into_iter()
            .map(|join| {
                let kind = match join.join {
                    PeriodicTrimTangentJoin::OneSided => "oneSided",
                    PeriodicTrimTangentJoin::Stationary => "stationary",
                    PeriodicTrimTangentJoin::C1 => "c1",
                    PeriodicTrimTangentJoin::G1 => "g1",
                    PeriodicTrimTangentJoin::Reversal => "reversal",
                    PeriodicTrimTangentJoin::Corner => "corner",
                };
                json!({
                    "segment": join.segment,
                    "parameter": {
                        "numerator": join.parameter.numer().to_string(),
                        "denominator": join.parameter.denom().to_string(),
                    },
                    "kind": kind,
                })
            })
            .collect::<Vec<_>>()
    });
    let refusal_stage = joins.is_none().then_some("audit");
    Ok(json!({
        "joins": joins,
        "workUsed": build_work + audit.work_used,
        "refusalStage": refusal_stage,
    }))
}
