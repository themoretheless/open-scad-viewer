use super::*;

#[derive(Clone)]
pub struct Boundary {
    pub curve: Curve,
    /// Forward UV traversal of the face loop, regardless of edge reversal.
    pub pcurve: Curve,
    pub reversed: bool,
}
/// Partition candidates become usable contour edges only after all checks.
/// Source boundary identities and shell incidence still belong to the caller.
pub struct SplitBoundary {
    pub report: nurbs_core::boundary_partition::Report,
    pub boundaries: Option<Vec<Boundary>>,
}
pub fn split_boundary(
    context: &ToleranceContext,
    surface: &Surface,
    boundary: &Boundary,
    cuts: &[f64],
    tolerance_uv: f64,
    partition_cells: usize,
    agreement_cells: usize,
) -> Result<SplitBoundary> {
    let report = nurbs_core::boundary_partition::split(
        &boundary.curve,
        &boundary.pcurve,
        surface,
        boundary.reversed,
        cuts,
        context.spatial_bounds().on_mm,
        tolerance_uv,
        partition_cells,
        agreement_cells,
    )?;
    let candidates = report
        .pieces
        .iter()
        .map(|p| Boundary {
            curve: p.curve.clone(),
            pcurve: p.pcurve.clone(),
            reversed: p.reversed,
        })
        .collect::<Vec<_>>();
    let same = |a: [f64; 3], b: [f64; 3]| a.map(f64::to_bits) == b.map(f64::to_bits);
    let endpoints_preserved = oriented(boundary)
        .zip(candidates.first().and_then(oriented))
        .zip(candidates.last().and_then(oriented))
        .is_some_and(|((original, first), last)| {
            same(original[0], first[0]) && same(original[1], last[1])
        });
    let boundaries = (report.qualified && endpoints_preserved).then_some(candidates);
    Ok(SplitBoundary { report, boundaries })
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub pairs: usize,
    pub region_cells: usize,
    pub domain_cells: usize,
    pub agreement_cells: usize,
}
#[derive(Clone)]
pub struct QualifiedFace {
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    loops: Vec<Loop>,
    face: Face,
    context: ToleranceSpecIdentity,
}
impl QualifiedFace {
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    pub fn loops(&self) -> &[Loop] {
        &self.loops
    }
    pub fn face(&self) -> &Face {
        &self.face
    }
    pub fn context(&self) -> &ToleranceSpecIdentity {
        &self.context
    }
}
pub struct Report {
    pub region: trim_region_audit::Report,
    pub agreements: Vec<agreement::Report>,
    pub agreement_cells: usize,
    pub face: Option<QualifiedFace>,
    pub reason: &'static str,
}
fn endpoints(c: &Curve) -> Option<[[f64; 3]; 2]> {
    let n = c.control_points.len();
    let d = c.domain();
    if c.periodic
        || !c.knots[..=c.degree].iter().all(|k| *k == d[0])
        || !c.knots[n..].iter().all(|k| *k == d[1])
    {
        return None;
    }
    let p = |i| {
        let q: &Vec<f64> = &c.control_points[i];
        [q[0], q[1], q[2]]
    };
    Some([p(0), p(n - 1)])
}
fn oriented(edge: &Boundary) -> Option<[[f64; 3]; 2]> {
    endpoints(&edge.curve).map(|p| if edge.reversed { [p[1], p[0]] } else { p })
}
pub fn assemble(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report> {
    surface.validate()?;
    if !(1..=100000).contains(&limits.agreement_cells) {
        return Err(Error::new(
            "BREP_FACE_RECIPE_BUDGET",
            "Face agreement needs 1..100000 shared cells",
        ));
    }
    // Every input is checked before budget or region refusal can hide it.
    for edge in wires.iter().flatten() {
        edge.curve.validate()?;
        edge.pcurve.validate()?;
        if edge.curve.control_points[0].len() != 3 || edge.pcurve.control_points[0].len() != 2 {
            return Err(Error::new(
                "BREP_FACE_RECIPE_DIMENSION",
                "Face recipes need 3D world edges and 2D pcurves",
            ));
        }
    }
    let uv = wires
        .iter()
        .map(|wire| wire.iter().map(|e| e.pcurve.clone()).collect())
        .collect::<Vec<Vec<_>>>();
    let region = trim_region_audit::inspect(
        &uv,
        tolerance_uv,
        limits.pairs,
        limits.region_cells,
        limits.domain_cells,
    )?;
    let mut out = Report {
        region,
        agreements: vec![],
        agreement_cells: 0,
        face: None,
        reason: "face-region-unqualified",
    };
    if out.region.valid != Some(true) {
        return Ok(out);
    }
    if out.region.winding[0] != Some(1) || out.region.winding[1..].iter().any(|w| *w != Some(-1)) {
        out.reason = "face-region-orientation-invalid";
        return Ok(out);
    }
    for wire in wires {
        for i in 0..wire.len() {
            let (Some(a), Some(b)) = (oriented(&wire[i]), oriented(&wire[(i + 1) % wire.len()]))
            else {
                out.reason = "world-endpoint-identity-unproven";
                return Ok(out);
            };
            if a[1].map(f64::to_bits) != b[0].map(f64::to_bits) {
                out.reason = "world-joins-not-identical";
                return Ok(out);
            }
        }
    }
    for edge in wires.iter().flatten() {
        if out.agreement_cells == limits.agreement_cells {
            out.reason = "face-agreement-work-limit";
            return Ok(out);
        }
        let r = agreement::verify(
            &edge.curve,
            &edge.pcurve,
            surface,
            edge.reversed,
            context.spatial_bounds().on_mm,
            limits.agreement_cells - out.agreement_cells,
        )?;
        out.agreement_cells += r.cells;
        let status = r.status;
        out.agreements.push(r);
        if status != agreement::Status::WithinTolerance {
            out.reason = "face-source-agreement-unqualified";
            return Ok(out);
        }
    }
    let mut vertices = vec![];
    let mut edges = vec![];
    let mut loops = vec![];
    for wire in wires {
        let start = vertices.len();
        for edge in wire {
            vertices.push(Vertex {
                point: oriented(edge).unwrap()[0],
            });
        }
        let mut coedges = vec![];
        for (i, edge) in wire.iter().enumerate() {
            let ends = [start + i, start + (i + 1) % wire.len()];
            coedges.push(Coedge::new(
                edges.len(),
                edge.reversed,
                edge.pcurve.clone(),
            ));
            edges.push(Edge {
                degenerate: false,
                vertices: if edge.reversed {
                    [ends[1], ends[0]]
                } else {
                    ends
                },
                curve: edge.curve.clone(),
            });
        }
        loops.push(Loop { coedges });
    }
    let face = Face {
        surface: surface.clone(),
        outer: 0,
        holes: (1..loops.len()).collect(),
    };
    out.face = Some(QualifiedFace {
        vertices,
        edges,
        loops,
        face,
        context: context.spec_identity(),
    });
    out.reason = "face-fragment-qualified";
    Ok(out)
}

/// Replace a prepartitioned source arc, retaining all other boundary definitions.
/// Limits apply independently to original-face, contour and replacement-face audits.
pub struct ReplaceBoundary {
    pub original: Report,
    pub contour: Option<nurbs_core::offset_face_loops::Report>,
    pub replacement: Option<Report>,
}
pub fn replace_boundary_arc(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    loop_index: usize,
    arc_start: usize,
    arc_count: usize,
    contact: &Boundary,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<ReplaceBoundary> {
    replace_boundary_path(
        context,
        surface,
        wires,
        loop_index,
        arc_start,
        arc_count,
        std::slice::from_ref(contact),
        tolerance_uv,
        limits,
    )
}
/// Replace an original boundary arc by a piecewise world/pcurve contact path.
/// No joining curves, vertex welding or solid admission is performed.
pub fn replace_boundary_path(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    loop_index: usize,
    arc_start: usize,
    arc_count: usize,
    contacts: &[Boundary],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<ReplaceBoundary> {
    if contacts.is_empty() {
        return Err(Error::new(
            "BREP_FACE_RECIPE_CONTACT",
            "Replacement path must be nonempty",
        ));
    }
    // Validate the replacement even when the original cannot be qualified.
    for contact in contacts {
        contact.curve.validate()?;
        contact.pcurve.validate()?;
        if contact.curve.control_points[0].len() != 3 || contact.pcurve.control_points[0].len() != 2
        {
            return Err(Error::new(
                "BREP_FACE_RECIPE_DIMENSION",
                "Replacement needs a 3D edge and 2D pcurve",
            ));
        }
    }
    let original = assemble(context, surface, wires, tolerance_uv, limits)?;
    let mut out = ReplaceBoundary {
        original,
        contour: None,
        replacement: None,
    };
    if out.original.face.is_none() {
        return Ok(out);
    }
    let uv = wires
        .iter()
        .map(|wire| wire.iter().map(|e| e.pcurve.clone()).collect())
        .collect::<Vec<Vec<Curve>>>();
    let contour = nurbs_core::offset_face_loops::replace_boundary_path(
        &uv,
        loop_index,
        arc_start,
        arc_count,
        &contacts
            .iter()
            .map(|c| c.pcurve.clone())
            .collect::<Vec<_>>(),
        tolerance_uv,
        nurbs_core::offset_face_loops::Limits {
            pairs: limits.pairs,
            cells: limits.region_cells,
            domain_cells: limits.domain_cells,
        },
    )?;
    if !contour.region_subset_proven {
        out.contour = Some(contour);
        return Ok(out);
    }
    let mut replacement = wires
        .iter()
        .map(|wire| {
            wire.iter()
                .map(|e| Boundary {
                    curve: e.curve.clone(),
                    pcurve: e.pcurve.clone(),
                    reversed: e.reversed,
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    replacement[loop_index] = contour
        .origins
        .iter()
        .map(|origin| {
            let source = match origin {
                nurbs_core::offset_face_loops::Origin::Contact => &contacts[0],
                nurbs_core::offset_face_loops::Origin::ContactPiece { index } => &contacts[*index],
                nurbs_core::offset_face_loops::Origin::Kept {
                    loop_index,
                    curve_index,
                } => &wires[*loop_index][*curve_index],
                _ => unreachable!("Boundary-arc replacement creates no connectors"),
            };
            Boundary {
                curve: source.curve.clone(),
                pcurve: source.pcurve.clone(),
                reversed: source.reversed,
            }
        })
        .collect();
    out.replacement = Some(assemble(
        context,
        surface,
        &replacement,
        tolerance_uv,
        limits,
    )?);
    out.contour = Some(contour);
    Ok(out)
}

/// Result retains every fresh audit, including partition failures.
pub struct PartitionedReplacement {
    pub original: Report,
    pub splits: Vec<SplitBoundary>,
    pub replacement: Option<ReplaceBoundary>,
}
/// Split two distinct original edges at forward pcurve fractions, then replace
/// the cyclic arc between those cuts. Parameters are supplied, not solved here.
/// Limits apply separately to source audit, each partition and replacement.
pub fn partition_and_replace_path(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    loop_index: usize,
    start_edge: usize,
    start_fraction: f64,
    end_edge: usize,
    end_fraction: f64,
    contacts: &[Boundary],
    tolerance_uv: f64,
    partition_cells: usize,
    limits: Limits,
) -> Result<PartitionedReplacement> {
    if loop_index >= wires.len()
        || start_edge == end_edge
        || start_edge >= wires[loop_index].len()
        || end_edge >= wires[loop_index].len()
        || ![start_fraction, end_fraction]
            .iter()
            .all(|x| x.is_finite() && *x > 0. && *x < 1.)
        || contacts.is_empty()
    {
        return Err(Error::new(
            "BREP_FACE_RECIPE_PARTITION",
            "Choose distinct original edges and interior contact fractions",
        ));
    }
    // Validate every proposal before any early return caused by audit budgets.
    for c in contacts {
        c.curve.validate()?;
        c.pcurve.validate()?;
        if c.curve.control_points[0].len() != 3 || c.pcurve.control_points[0].len() != 2 {
            return Err(Error::new(
                "BREP_FACE_RECIPE_DIMENSION",
                "Replacement needs a 3D edge and 2D pcurve",
            ));
        }
    }
    let start = split_boundary(
        context,
        surface,
        &wires[loop_index][start_edge],
        &[start_fraction],
        tolerance_uv,
        partition_cells,
        limits.agreement_cells,
    )?;
    let end = split_boundary(
        context,
        surface,
        &wires[loop_index][end_edge],
        &[end_fraction],
        tolerance_uv,
        partition_cells,
        limits.agreement_cells,
    )?;
    let original = assemble(context, surface, wires, tolerance_uv, limits)?;
    let mut out = PartitionedReplacement {
        original,
        splits: vec![start, end],
        replacement: None,
    };
    if out.original.face.is_none() || out.splits.iter().any(|s| s.boundaries.is_none()) {
        return Ok(out);
    }
    let mut expanded = wires.to_vec();
    let mut wire = vec![];
    let mut begin = 0;
    let mut finish = 0;
    for (i, boundary) in wires[loop_index].iter().enumerate() {
        if i == start_edge {
            begin = wire.len() + 1;
            wire.extend(out.splits[0].boundaries.as_ref().unwrap().iter().cloned());
        } else if i == end_edge {
            finish = wire.len();
            wire.extend(out.splits[1].boundaries.as_ref().unwrap().iter().cloned());
        } else {
            wire.push(boundary.clone());
        }
    }
    let count = (finish + wire.len() - begin) % wire.len() + 1;
    expanded[loop_index] = wire;
    out.replacement = Some(replace_boundary_path(
        context,
        surface,
        &expanded,
        loop_index,
        begin,
        count,
        contacts,
        tolerance_uv,
        limits,
    )?);
    Ok(out)
}

pub struct BoundaryCrossing {
    pub loop_index: usize,
    pub boundary_index: usize,
    /// First parameter is the original boundary, second is the contact curve.
    pub report: nurbs_core::uv_curve_crossings::Report,
}
pub struct FaceCrossings {
    pub original: Report,
    pub crossings: Vec<BoundaryCrossing>,
    pub visited: usize,
    pub complete: bool,
    pub precision_proven: bool,
}
/// Search contact crossings against every original boundary of a freshly
/// qualified source face. Shared root budget covers all boundaries and holes.
/// This returns source-addressed enclosures; it does not choose rounded cuts.
pub fn locate_contact_boundaries(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    contact: &Curve,
    tolerance_uv: f64,
    limits: Limits,
    max_cells: usize,
    target_width: [f64; 2],
) -> Result<FaceCrossings> {
    use nurbs_core::uv_curve_crossings as crossings;
    contact.validate()?;
    if contact.control_points[0].len() != 2
        || contact.periodic
        || !(1..=100000).contains(&max_cells)
        || !target_width.iter().all(|x| x.is_finite() && *x > 0.)
    {
        return Err(Error::new(
            "BREP_FACE_RECIPE_CROSSINGS",
            "Choose a nonperiodic UV contact and bounded positive crossing precision/work",
        ));
    }
    for b in wires.iter().flatten() {
        b.pcurve.validate()?;
        if b.pcurve.periodic {
            return Err(Error::new(
                "BREP_FACE_RECIPE_CROSSINGS",
                "Original boundary crossing search requires nonperiodic pcurves",
            ));
        }
    }
    let original = assemble(context, surface, wires, tolerance_uv, limits)?;
    let mut out = FaceCrossings {
        original,
        crossings: vec![],
        visited: 0,
        complete: false,
        precision_proven: false,
    };
    if out.original.face.is_none() {
        return Ok(out);
    }
    out.complete = true;
    out.precision_proven = true;
    for (loop_index, wire) in wires.iter().enumerate() {
        for (boundary_index, b) in wire.iter().enumerate() {
            let report = if out.visited < max_cells {
                crossings::isolate_refined(
                    &b.pcurve,
                    contact,
                    max_cells - out.visited,
                    target_width,
                )?
            } else {
                crossings::Report {
                    cells: vec![crossings::Cell {
                        parameters: [b.pcurve.domain(), contact.domain()],
                        root: None,
                        state: crossings::State::Unresolved,
                    }],
                    visited: 0,
                    complete: false,
                    target_width: Some(target_width),
                    precision_proven: false,
                }
            };
            out.visited += report.visited;
            out.complete &= report.complete;
            out.precision_proven &= report.precision_proven;
            out.crossings.push(BoundaryCrossing {
                loop_index,
                boundary_index,
                report,
            });
        }
    }
    Ok(out)
}

#[cfg(feature = "codec")]
pub struct AddressedSourcePoint {
    pub loop_index: usize,
    pub boundary_index: usize,
    pub crossing_cell: usize,
    pub report: Option<crate::source_contact_point::Report>,
}
#[cfg(feature = "codec")]
pub struct SourcePointSearch {
    pub search: FaceCrossings,
    pub points: Vec<AddressedSourcePoint>,
    pub point_checks: usize,
    pub mapping_cells: usize,
    pub complete: bool,
}
