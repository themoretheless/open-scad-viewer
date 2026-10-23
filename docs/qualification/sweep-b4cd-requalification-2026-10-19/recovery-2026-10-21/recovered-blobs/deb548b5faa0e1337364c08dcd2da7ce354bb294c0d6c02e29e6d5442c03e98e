//! Original boundary-root binding for an adaptively constructed curved/curved cut.
use super::{Partition, Segment, invalid, unproven};
use crate::{Model, edge_partition, face_crossings, face_domain::FaceDomain};
use nurbs_core::{Result, interval_eval::Interval as I, ss_intersection as ss};

pub struct AutomaticFiberPartition {
    pub partition: Partition,
    pub endpoints: [usize; 2],
    pub boundary_coverage: Vec<(usize, face_crossings::Report)>,
    pub cells: usize,
    pub evaluations: usize,
    pub root_error_upper_mm: f64,
    pub trace_pieces: usize,
    pub root_fibers: usize,
}
/// Infer two original boundary roots, certify one local original root fiber,
/// construct its complete trace, bind the actual saved endpoints, and reverify
/// every edited piece before atomically partitioning the source face. The
/// caller supplies a parameter search box, never a trace/positive certificate.
/// Free axis must belong to the first support; its interval is inferred from
/// actual owned chart endpoints. All sectors survive. Other root branches,
/// global SS coverage, material removal and corner construction remain separate.
pub fn partition_fiber_at_face(
    source: &Model,
    face: usize,
    cutter: &Model,
    cutter_face: usize,
    parameter_box: [[f64; 2]; 4],
    axis: ss::FiberAxis,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<AutomaticFiberPartition> {
    partition_fiber_impl(
        source,
        face,
        cutter,
        cutter_face,
        parameter_box,
        Some(axis),
        tolerance_uv,
        max_cells,
        max_exact,
    )
}

/// Propose a search box from original surface domains and owned boundary roots,
/// then certify the entire fiber. This still requires a single certifiable
/// root fiber; hidden branches and a complete surface-intersection census are
/// not implied. All root, construction and trim work shares one allowance.
pub fn partition_fiber_at_face_automatic(
    source: &Model,
    face: usize,
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<AutomaticFiberPartition> {
    source.validate()?;
    cutter.validate()?;
    let parameter_box = original_parameter_box(source, face, cutter, cutter_face)?;
    partition_fiber_impl(
        source,
        face,
        cutter,
        cutter_face,
        parameter_box,
        None,
        tolerance_uv,
        max_cells,
        max_exact,
    )
}

fn partition_fiber_impl(
    source: &Model,
    face: usize,
    cutter: &Model,
    cutter_face: usize,
    parameter_box: [[f64; 2]; 4],
    axis: Option<ss::FiberAxis>,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<AutomaticFiberPartition> {
    source.validate()?;
    cutter.validate()?;
    let _free = match axis {
        Some(ss::FiberAxis::FirstU) | None => 0,
        Some(ss::FiberAxis::FirstV) => 1,
        _ => {
            return Err(invalid(
                "Boundary-bound fiber needs a source-face free axis",
            ));
        }
    };
    if face >= source.faces.len()
        || cutter_face >= cutter.faces.len()
        || !(32..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || parameter_box
            .iter()
            .any(|d| !d[0].is_finite() || !d[1].is_finite() || d[0] >= d[1])
    {
        return Err(invalid("Need bounded original curved-cut inputs"));
    }
    let edges = source.loops[source.faces[face].outer]
        .coedges
        .iter()
        .map(|c| c.edge)
        .collect::<Vec<_>>();
    if !(3..=256).contains(&edges.len())
        || edges
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != edges.len()
    {
        return Err(unproven(
            "Need distinct original outer edges for root ownership",
        ));
    }
    let mut model = source.clone();
    let mut cells = 0;
    let mut boundary_coverage = Vec::new();
    let mut roots = Vec::new();
    let mut cutter_roots = Vec::new();
    for edge in edges {
        if cells >= max_cells {
            return Err(unproven("Curved-cut root work exhausted"));
        }
        let split = edge_partition::partition_all_at_face(
            &model,
            edge,
            cutter,
            cutter_face,
            tolerance_uv,
            max_cells - cells,
        )?;
        cells += split.cells;
        for (&index, &vertex) in split.coverage.eligible.iter().zip(&split.vertices) {
            roots.push((vertex, split.coverage.roots[index].root.position_bounds));
            let b = split.coverage.roots[index].root.parameter_box;
            cutter_roots.push([b[1], b[2]]);
        }
        if roots.len() > 2 {
            return Err(unproven("More than two roots need branch ownership"));
        }
        boundary_coverage.push((edge, split.coverage));
        model = split.model;
    }
    if roots.len() != 2 {
        return Err(unproven("Need exactly two retained boundary roots"));
    }
    let mut parameter_box = parameter_box;
    let axis = match axis {
        Some(axis) => axis,
        None => qualify_automatic_search(
            &model,
            face,
            cutter,
            cutter_face,
            &roots,
            &cutter_roots,
            &mut parameter_box,
            &mut cells,
            max_cells,
        )?,
    };
    let mut out = partition_fiber_prebound(
        &model,
        face,
        cutter,
        cutter_face,
        &roots,
        parameter_box,
        axis,
        tolerance_uv,
        max_cells - cells,
        max_exact,
        4096,
    )?;
    out.cells += cells;
    out.boundary_coverage = boundary_coverage;
    out.partition.model.refresh_change_set(&[source]);
    Ok(out)
}

pub(super) fn partition_fiber_prebound(
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
    let mut partition = super::partition_with_budget(
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

fn join_pieces(
    pieces: &[nurbs_core::curve::Curve],
    intervals: &[[f64; 2]],
) -> Result<nurbs_core::curve::Curve> {
    use nurbs_core::curve::Curve;
    if pieces.is_empty() || pieces.len() != intervals.len() {
        return Err(invalid("Missing compound branch pieces"));
    }
    let mut controls = pieces[0].control_points.clone();
    let mut knots = vec![intervals[0][0]; 4];
    for (i, piece) in pieces.iter().enumerate() {
        if piece.degree != 3
            || piece.control_points.len() != 4
            || piece.knots != vec![0., 0., 0., 0., 1., 1., 1., 1.]
            || piece.weights.iter().any(|&w| w != 1.)
        {
            return Err(unproven(
                "Compound branch needs original cubic unit-weight pieces",
            ));
        }
        if i > 0 {
            if pieces[i - 1].control_points.last() != piece.control_points.first()
                || intervals[i - 1][1] != intervals[i][0]
            {
                return Err(unproven("Compound branch endpoints are not shared"));
            }
            controls.extend_from_slice(&piece.control_points[1..]);
        }
        knots.extend(std::iter::repeat_n(
            intervals[i][1],
            if i + 1 == pieces.len() { 4 } else { 3 },
        ));
    }
    let out = Curve {
        degree: 3,
        knots,
        weights: vec![1.; controls.len()],
        control_points: controls,
        periodic: false,
    };
    out.validate()?;
    Ok(out)
}

pub(super) fn original_parameter_box(
    source: &Model,
    face: usize,
    cutter: &Model,
    cutter_face: usize,
) -> Result<[[f64; 2]; 4]> {
    let first = source
        .faces
        .get(face)
        .ok_or_else(|| invalid("Source face index is out of range"))?;
    let second = cutter
        .faces
        .get(cutter_face)
        .ok_or_else(|| invalid("Cutter face index is out of range"))?;
    let domains = |surface: &nurbs_core::surface::Surface| {
        (
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        )
    };
    let (u, v) = domains(&first.surface);
    let (s, t) = domains(&second.surface);
    Ok([u, v, s, t])
}

pub(super) fn automatic_search_chart(
    model: &Model,
    face: usize,
    roots: &[(usize, [[f64; 2]; 3])],
    cutter_roots: &[[[f64; 2]; 2]],
    parameter_box: &mut [[f64; 2]; 4],
    padding_fraction: f64,
    metric_scales: [f64; 4],
) -> Result<ss::FiberAxis> {
    if roots.len() != 2 || cutter_roots.len() != 2 {
        return Err(unproven("Automatic chart needs two owned original roots"));
    }
    let endpoints = roots
        .iter()
        .map(|&(vertex, _)| {
            let uses = model.loops[model.faces[face].outer]
                .coedges
                .iter()
                .filter(|c| model.edges[c.edge].vertices[usize::from(c.reversed)] == vertex)
                .collect::<Vec<_>>();
            if uses.len() != 1 {
                return Err(unproven(
                    "Automatic free axis needs unique original chart endpoints",
                ));
            }
            let p = &uses[0].pcurve.control_points[0];
            Ok([p[0], p[1]])
        })
        .collect::<Result<Vec<_>>>()?;
    let spans = [0, 1].map(|k| {
        (endpoints[1][k] - endpoints[0][k]).abs() / (parameter_box[k][1] - parameter_box[k][0])
    });
    // Endpoint data proposes a local branch neighborhood only. It is
    // never a whole-branch enclosure certificate: prebound construction
    // must freshly prove the entire resulting fiber and actual trace.
    for k in 0..4 {
        let original = parameter_box[k];
        let margin = (original[1] - original[0]) * padding_fraction * metric_scales[k];
        let (lo, hi) = if k < 2 {
            (
                endpoints[0][k].min(endpoints[1][k]),
                endpoints[0][k].max(endpoints[1][k]),
            )
        } else {
            (
                cutter_roots[0][k - 2][0].min(cutter_roots[1][k - 2][0]),
                cutter_roots[0][k - 2][1].max(cutter_roots[1][k - 2][1]),
            )
        };
        parameter_box[k] = [
            (lo - margin).max(original[0]),
            (hi + margin).min(original[1]),
        ];
    }
    let free = if spans[0] >= spans[1] { 0 } else { 1 };
    parameter_box[free] = [
        endpoints[0][free].min(endpoints[1][free]),
        endpoints[0][free].max(endpoints[1][free]),
    ];
    Ok(if free == 0 {
        ss::FiberAxis::FirstU
    } else {
        ss::FiberAxis::FirstV
    })
}

/// Failed proposal proofs consume the same allowance as successful ones. The
/// actual constructed and endpoint-bound trace is still checked independently.
pub(super) fn qualify_automatic_search(
    model: &Model,
    face: usize,
    cutter: &Model,
    cutter_face: usize,
    roots: &[(usize, [[f64; 2]; 3])],
    cutter_roots: &[[[f64; 2]; 2]],
    parameter_box: &mut [[f64; 2]; 4],
    cells: &mut usize,
    max_cells: usize,
) -> Result<ss::FiberAxis> {
    let original = *parameter_box;
    let sources = ss::FiberSources::new(
        &model.faces[face].surface,
        &cutter.faces[cutter_face].surface,
    )?;
    if max_cells.saturating_sub(*cells) < 2 {
        return Err(unproven("Automatic metric proposal work exhausted"));
    }
    *cells += 2;
    // UV spans on different supports can represent very different physical
    // lengths. These numerical tangent speeds only scale search proposals;
    // the unchanged original interval operator supplies every admission proof.
    let mut speeds = [1.; 4];
    for (support, surface) in [
        &model.faces[face].surface,
        &cutter.faces[cutter_face].surface,
    ]
    .iter()
    .enumerate()
    {
        let d = [original[2 * support], original[2 * support + 1]];
        if let Ok(evaluation) =
            surface.evaluate(0.5 * d[0][0] + 0.5 * d[0][1], 0.5 * d[1][0] + 0.5 * d[1][1])
        {
            if let Some((u, v)) = evaluation.first_derivatives() {
                for (k, tangent) in [u, v].iter().enumerate() {
                    let speed =
                        tangent.iter().map(|x| x * x).sum::<f64>().sqrt() * (d[k][1] - d[k][0]);
                    if speed.is_finite() && speed > 0. {
                        speeds[2 * support + k] = speed;
                    }
                }
            }
        }
    }
    let reference = speeds[0].min(speeds[1]);
    let metric_scales = speeds.map(|s| {
        let ratio = reference / s;
        if ratio.is_finite() && ratio > 0. {
            ratio
        } else {
            1.
        }
    });
    // Start locally to separate nearby branches; wider proposals handle trace
    // excursions and interval dependency. A proposal is never a certificate.
    let mut reasons = Vec::new();
    for attempt in 0..8 {
        *parameter_box = original;
        let axis = automatic_search_chart(
            model,
            face,
            roots,
            cutter_roots,
            parameter_box,
            (1usize << attempt) as f64 / 128.,
            metric_scales,
        )?;
        let free = if matches!(axis, ss::FiberAxis::FirstU) {
            0
        } else {
            1
        };
        let mut endpoint_box = original;
        automatic_search_chart(
            model,
            face,
            roots,
            cutter_roots,
            &mut endpoint_box,
            0.,
            metric_scales,
        )?;
        for recenter in 0..3 {
            let remaining = max_cells.saturating_sub(*cells);
            if remaining < 32 {
                return Err(unproven("Automatic chart search work exhausted"));
            }
            let proof = sources.certify_contracted(*parameter_box, axis, remaining / 8)?;
            *cells += proof.rectangles();
            let contracted = proof.domain();
            let image = proof.root_image();
            reasons.push(format!("{attempt}/{recenter}:{}", proof.reason()));
            if proof.into_fiber().is_some() {
                *parameter_box = contracted;
                return Ok(axis);
            }
            let Some(image) = image else {
                break;
            };
            // Recenter a PROPOSAL around a fresh original root enclosure, retain
            // endpoint seeds, and reprove the whole new box. This does not grant
            // root ownership or acceptance based on an inconclusive receipt.
            let mut next = contracted;
            let mut index = 0;
            for k in 0..4 {
                if k == free {
                    continue;
                }
                let [lo, hi] = image[index];
                index += 1;
                // Already included coordinates must not be enlarged merely
                // because another coordinate needs recentering. That would
                // reintroduce avoidable coupling in the original operator.
                if next[k][0] < lo && hi < next[k][1] {
                    continue;
                }
                let pad = (hi - lo) / 8.;
                next[k] = [
                    (lo - pad).min(endpoint_box[k][0]).max(original[k][0]),
                    (hi + pad).max(endpoint_box[k][1]).min(original[k][1]),
                ];
            }
            if next == *parameter_box || next.iter().any(|d| d[0] >= d[1]) {
                break;
            }
            *parameter_box = next;
        }
    }
    // A single uniform operator can be too coarse for an otherwise regular
    // curved branch. Reprove an ordered complete cover of its free interval.
    *parameter_box = original;
    let axis = automatic_search_chart(
        model,
        face,
        roots,
        cutter_roots,
        parameter_box,
        1. / 128.,
        metric_scales,
    )?;
    if qualify_fiber_cover(&sources, *parameter_box, axis, cells, max_cells).is_ok() {
        return Ok(axis);
    }
    Err(unproven(&format!(
        "Automatic original-surface fiber remains unresolved after bounded search: {}",
        reasons.join("; ")
    )))
}

fn qualify_fiber_cover(
    sources: &ss::FiberSources,
    domain: [[f64; 2]; 4],
    axis: ss::FiberAxis,
    cells: &mut usize,
    max_cells: usize,
) -> Result<Vec<ss::QualifiedFiber>> {
    let free = if matches!(axis, ss::FiberAxis::FirstU) {
        0
    } else {
        1
    };
    let mut pending = vec![(domain, 0)];
    let mut cover = Vec::new();
    while let Some((box4, depth)) = pending.pop() {
        let remaining = max_cells.saturating_sub(*cells);
        if remaining == 0 {
            return Err(unproven("Piecewise original-root work exhausted"));
        }
        let proof = match sources.certify_contracted(box4, axis, remaining) {
            Ok(proof) => proof,
            Err(error) => {
                *cells = max_cells;
                return Err(error);
            }
        };
        *cells += proof.rectangles();
        let excluded = proof.reason().contains("exclusion");
        if let Some(fiber) = proof.into_fiber() {
            cover.push(fiber);
            continue;
        }
        if excluded || depth == 8 || cover.len() + pending.len() >= 63 {
            return Err(unproven("No complete bounded original-root cover"));
        }
        let middle = box4[free][0] * 0.5 + box4[free][1] * 0.5;
        if !(box4[free][0] < middle && middle < box4[free][1]) {
            return Err(unproven("Original-root cover interval cannot split"));
        }
        let mut left = box4;
        let mut right = box4;
        left[free][1] = middle;
        right[free][0] = middle;
        pending.push((right, depth + 1));
        pending.push((left, depth + 1));
    }
    Ok(cover)
}
