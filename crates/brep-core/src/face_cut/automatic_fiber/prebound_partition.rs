//! Prebound partition: preserves the original arithmetic and proof budgets.
use super::*;

pub(crate) fn partition_fiber_prebound(
    source: &Model,
    face: usize,
    cutter: &Model,
    cutter_face: usize,
    roots: &[(usize, [[f64; 2]; 3])],
    parameter_box: [[f64; 2]; 4],
    axis: ss::FiberAxis,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
    max_evaluations: usize,
) -> Result<AutomaticFiberPartition> {
    let free = match axis {
        ss::FiberAxis::FirstU => 0,
        ss::FiberAxis::FirstV => 1,
        _ => {
            return Err(invalid(
                "Prebound cut requires a first-support free coordinate",
            ));
        }
    };
    if roots.len() != 2 || max_cells < 24 || max_evaluations == 0 {
        return Err(unproven("Need two shared roots and remaining fiber work"));
    }
    let model = source;
    let mut cells = 0;
    let boundary_coverage = Vec::new();
    let mut owned = Vec::new();
    for &(vertex, bounds) in roots {
        let mut distance = I::point(0.);
        for k in 0..3 {
            let d = I::point(model.vertices[vertex].point[k])
                .sub(I::new(bounds[k][0], bounds[k][1])?)?;
            let upper = d.lo.abs().max(d.hi.abs());
            distance = distance.add(I::point(upper).mul(I::point(upper))?)?;
        }
        if distance.hi
            > I::point(source.tolerance_mm)
                .mul(I::point(source.tolerance_mm))?
                .lo
        {
            return Err(unproven(
                "Saved root endpoint is not bounded against its original root box",
            ));
        }
        let uses = model.loops[model.faces[face].outer]
            .coedges
            .iter()
            .filter(|c| model.edges[c.edge].vertices[usize::from(c.reversed)] == vertex)
            .collect::<Vec<_>>();
        if uses.len() != 1 {
            return Err(unproven("Root needs a unique original chart endpoint"));
        }
        let uv = &uses[0].pcurve.control_points[0];
        owned.push((vertex, [uv[0], uv[1]]));
    }
    owned.sort_by(|a, b| a.1[free].total_cmp(&b.1[free]));
    let mut domain = parameter_box;
    domain[free] = [owned[0].1[free], owned[1].1[free]];
    if domain[free][0] >= domain[free][1] || max_cells.saturating_sub(cells) < 24 {
        return Err(unproven(
            "Need a noncollapsed original free interval and remaining cut work",
        ));
    }
    let sources = ss::FiberSources::new(
        &model.faces[face].surface,
        &cutter.faces[cutter_face].surface,
    )?;
    let cover_limit = cells + (max_cells - cells) / 4;
    let fibers = qualify_fiber_cover(&sources, domain, axis, &mut cells, cover_limit)?;
    let mut pieces = Vec::new();
    let mut compound_ranges = Vec::new();
    let mut evaluations = 0;
    for (index, fiber) in fibers.iter().enumerate() {
        let build = ss::construct_fiber_trace(
            fiber,
            ss::FiberTraceBuildLimits {
                tolerance_world: source.tolerance_mm / 4.,
                max_cells: (max_cells - cells) / (2 * (fibers.len() - index)),
                max_evaluations: (max_evaluations - evaluations) / (fibers.len() - index),
                ..Default::default()
            },
        )?;
        cells += build.cells;
        evaluations += build.evaluations;
        let trace = build
            .trace
            .ok_or_else(|| unproven("Whole original curved-cut trace is unresolved"))?;
        let start = pieces.len();
        pieces.extend(trace);
        if pieces.len() > 256 {
            return Err(unproven("Piecewise root trace exceeds 256 pieces"));
        }
        for chunk_start in (start..pieces.len()).step_by(85) {
            compound_ranges.push((chunk_start, (chunk_start + 85).min(pieces.len()), index));
        }
    }
    let count = pieces.len();
    let mut path: Vec<Segment> = Vec::new();
    let mut others: Vec<nurbs_core::curve::Curve> = Vec::new();
    let mut intervals = Vec::new();
    let mut root_error_upper_mm = 0_f64;
    let cutter_domain = FaceDomain::new(cutter, cutter_face, tolerance_uv)?;
    for (i, piece) in pieces.iter().enumerate() {
        let mut curve = piece.curve().clone();
        let mut pcurves = piece.pcurves().clone();
        if let (Some(previous), Some(other)) = (path.last(), others.last()) {
            curve.control_points[0] = previous.curve.control_points.last().unwrap().clone();
            pcurves[0].control_points[0] = previous.pcurve.control_points.last().unwrap().clone();
            pcurves[1].control_points[0] = other.control_points.last().unwrap().clone();
        }

        for (end, index) in [(0, 0), (1, count - 1)] {
            if i != index {
                continue;
            }
            let cv = if end == 0 {
                0
            } else {
                curve.control_points.len() - 1
            };
            let pc = if end == 0 {
                0
            } else {
                pcurves[0].control_points.len() - 1
            };
            curve.control_points[cv] = model.vertices[owned[end].0].point.to_vec();
            pcurves[0].control_points[pc] = owned[end].1.to_vec();
        }
        if max_cells.saturating_sub(cells) < 3 {
            return Err(unproven("Edited original-root trace work exhausted"));
        }
        let remaining = max_cells - cells;
        let fresh = ss::qualify_fiber_world_trace(
            piece.source(),
            &pcurves,
            &curve,
            ss::FiberWorldTraceLimits {
                tolerance_world: source.tolerance_mm / 2.,
                max_rectangles: remaining / 3,
                max_agreement_cells: remaining / 3,
            },
        )?;
        cells += fresh.rectangles() + fresh.agreement_cells();
        let fresh = fresh
            .into_trace()
            .ok_or_else(|| unproven("Bound endpoint edits fail original-root trace proof"))?;
        root_error_upper_mm = root_error_upper_mm.max(fresh.world_error_upper());
        let hull = std::array::from_fn(|k| {
            [
                pcurves[1]
                    .control_points
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::INFINITY, f64::min),
                pcurves[1]
                    .control_points
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max),
            ]
        });
        if !cutter_domain.contains_closed_rectangle(hull) {
            if cells >= max_cells {
                return Err(unproven("Cutter membership work exhausted"));
            }
            let r = cutter_domain.classify(hull, max_cells - cells)?;
            cells += r.cells;
            if r.location != nurbs_core::trim_domain::Location::Inside || r.winding != Some(1) {
                return Err(unproven(
                    "Curved section is not proven inside the actual cutter trim",
                ));
            }
        }
        others.push(pcurves[1].clone());
        intervals.push(piece.source().domain()[free]);
        path.push(Segment {
            curve,
            pcurve: pcurves[0].clone(),
        });
    }
    // Keep one topological edge per qualified root interval/chunk. Joining exact shared cubic
    // endpoints does not refit geometry; the ACTUAL saved compound curves must
    // nevertheless pass a fresh whole original-root proof after knot creation.
    let mut compound = Vec::new();
    for (start, end, fiber_index) in compound_ranges {
        let fiber = &fibers[fiber_index];
        let curve = join_pieces(
            &path[start..end]
                .iter()
                .map(|s| s.curve.clone())
                .collect::<Vec<_>>(),
            &intervals[start..end],
        )?;
        let pcurves = [
            join_pieces(
                &path[start..end]
                    .iter()
                    .map(|s| s.pcurve.clone())
                    .collect::<Vec<_>>(),
                &intervals[start..end],
            )?,
            join_pieces(&others[start..end], &intervals[start..end])?,
        ];
        if max_cells.saturating_sub(cells) < 4 {
            return Err(unproven("No remaining compound-root trace work"));
        }
        let mut chunk_domain = fiber.domain();
        chunk_domain[free] = [intervals[start][0], intervals[end - 1][1]];
        let chunk = fiber.source().certify(
            chunk_domain,
            axis,
            ss::FiberLimits {
                max_rectangles: (max_cells - cells) / 3,
                preconditioner: None,
            },
        )?;
        cells += chunk.report().rectangles;
        let chunk = chunk
            .into_fiber()
            .ok_or_else(|| unproven("Compound branch root operator is unproven"))?;
        let whole = ss::qualify_fiber_world_trace(
            &chunk,
            &pcurves,
            &curve,
            ss::FiberWorldTraceLimits {
                tolerance_world: source.tolerance_mm / 2.,
                max_rectangles: (max_cells - cells) / 3,
                max_agreement_cells: (max_cells - cells) / 3,
            },
        )?;
        cells += whole.rectangles() + whole.agreement_cells();
        let reason = whole.reason();
        let whole = whole.into_trace().ok_or_else(|| {
            unproven(&format!(
                "Saved compound branch fails whole original-root proof: {reason}"
            ))
        })?;
        root_error_upper_mm = root_error_upper_mm.max(whole.world_error_upper());
        compound.push(Segment {
            curve,
            pcurve: pcurves[0].clone(),
        });
    }
    let path = compound;
    if cells >= max_cells {
        return Err(unproven("No remaining curved-cut topology work"));
    }
    let endpoints = [owned[0].0, owned[1].0];
    let mut partition = super::super::partition_with_budget(
        &model,
        face,
        endpoints,
        &path,
        tolerance_uv,
        max_cells - cells,
        max_exact,
    )?;
    cells += partition.cells;
    partition.model.refresh_change_set(&[source]);
    Ok(AutomaticFiberPartition {
        partition,
        endpoints,
        boundary_coverage,
        cells,
        evaluations,
        root_error_upper_mm,
        trace_pieces: count,
        root_fibers: fibers.len(),
    })
}
