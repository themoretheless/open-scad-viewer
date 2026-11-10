//! Explicit bounded boundary proposal fitting, before strict solid admission.
use super::*;
pub struct BoundaryAdjustment {
    pub edge: usize,
    pub source_face: usize,
    /// Fresh whole original-curve correspondence proved within this bound.
    pub displacement_upper_mm: f64,
}
pub struct FittedPatch {
    pub model: Model,
    pub adjustments: Vec<BoundaryAdjustment>,
    pub cells: usize,
    pub exact_work: u64,
    pub source_pcurves: Vec<Option<Curve>>,
}
/// Propose inward control adjustments only where rectangle inclusion cannot
/// qualify the original boundary. Exact shared endpoints and original supports
/// are retained. Fresh source/skin lifts and whole-original-curve deviation are
/// mandatory. This returns a proposal, not a membership or closed-solid claim;
/// pass its actual model to `attach_planned` for strict independent admission.
pub fn fit_boundaries(
    source: &Model,
    patch: &Model,
    vertices: &[usize],
    keep_vertices: &[usize],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<FittedPatch> {
    fit_boundaries_with_charts(
        source,
        patch,
        vertices,
        keep_vertices,
        &[],
        tolerance_uv,
        max_cells,
        max_exact,
    )
}
pub fn fit_boundaries_with_charts(
    source: &Model,
    patch: &Model,
    vertices: &[usize],
    keep_vertices: &[usize],
    charts: &[Option<Curve>],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<FittedPatch> {
    if !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || patch.faces.len() != 1
    {
        return Err(invalid(
            "Boundary fitting needs one patch face and bounded shared work",
        ));
    }
    let targets = plan_targets(source, patch, vertices, keep_vertices)?;
    if !charts.is_empty() && charts.len() != patch.edges.len() {
        return Err(invalid(
            "Need one optional source chart per transition edge",
        ));
    }
    let mut out = FittedPatch {
        model: patch.clone(),
        adjustments: Vec::new(),
        cells: 0,
        exact_work: 0,
        source_pcurves: if charts.is_empty() {
            vec![None; patch.edges.len()]
        } else {
            charts.to_vec()
        },
    };
    let deviation_limit = source.tolerance_mm * 0.125;
    for (edge, target) in targets.iter().enumerate() {
        if out.cells == max_cells {
            return Err(unproven("Boundary fitting work exhausted"));
        }
        let face = &source.faces[target.face];
        let boundary = &patch.edges[edge];
        let mut pc = if let Some(pc) = &out.source_pcurves[edge] {
            pc.clone()
        } else {
            let projection = curve_pullback::project(
                &boundary.curve,
                &face.surface,
                curve_pullback::Limits {
                    tolerance: source.tolerance_mm * 0.5,
                    max_segments: 256,
                    max_iterations: 32,
                    agreement_cells: max_cells - out.cells,
                },
            )?;
            out.cells += projection.agreement_cells;
            projection
                .pcurve
                .ok_or_else(|| unproven("Boundary fitting pullback unqualified"))?
        };
        pc.validate()?;
        if pc.control_points[0].len() != 2 {
            return Err(invalid("Fitting source chart must be two-dimensional"));
        }
        let outer = &source.loops[face.outer].coedges;
        for (k, &v) in boundary.vertices.iter().enumerate() {
            let c = outer
                .iter()
                .find(|c| source.edges[c.edge].vertices[usize::from(c.reversed)] == vertices[v])
                .ok_or_else(|| invalid("Fitting endpoint has no original loop authority"))?;
            let i = if k == 0 {
                0
            } else {
                pc.control_points.len() - 1
            };
            pc.control_points[i] = c.pcurve.control_points[0].clone();
        }
        let domain = FaceDomain::new(source, target.face, tolerance_uv)?;
        out.source_pcurves[edge] = Some(pc.clone());
        if inside(&pc, &domain, &mut out.cells, max_cells).is_ok() {
            continue;
        }
        if out.cells == max_cells {
            return Err(unproven("No boundary fitting proof work remains"));
        }
        let keep = outer
            .iter()
            .find(|c| source.edges[c.edge].vertices[usize::from(c.reversed)] == target.keep_vertex)
            .ok_or_else(|| invalid("Fitting retention vertex has no chart authority"))?
            .pcurve
            .control_points[0]
            .clone();
        let mut scale = 0_f64;
        for k in 0..3 {
            let lo = face
                .surface
                .control_points
                .iter()
                .flatten()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min);
            let hi = face
                .surface
                .control_points
                .iter()
                .flatten()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max);
            scale = scale.max(hi - lo);
        }
        if !scale.is_finite() || scale <= 0. {
            return Err(unproven("No finite fitting proposal scale"));
        }
        let fraction = (source.tolerance_mm / (64. * scale)).min(0.001);
        let original_pc = pc.clone();
        let last = pc.control_points.len() - 1;
        if last < 2 {
            return Err(unproven("No interior boundary controls to fit"));
        }
        for p in &mut pc.control_points[1..last] {
            for k in 0..2 {
                p[k] += fraction * (keep[k] - p[k]);
            }
        }
        if pc == original_pc {
            return Err(unproven(
                "Boundary fitting collapses at coordinate precision",
            ));
        }
        pc.validate()?;
        let endpoints = boundary.vertices.map(|v| patch.vertices[v].point);
        let lifted = curve_surface_agreement::lift(
            &pc,
            &face.surface,
            Some(endpoints),
            deviation_limit,
            max_cells - out.cells,
        )?;
        out.cells += lifted.cells;
        let curve = lifted
            .curve
            .ok_or_else(|| unproven("Fitted boundary lift unqualified"))?;
        if out.cells == max_cells {
            return Err(unproven("No original-boundary comparison work remains"));
        }
        let original = &boundary.curve;
        let carrier = nurbs_core::surface::Surface {
            degree_u: original.degree,
            degree_v: 1,
            knots_u: original.knots.clone(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: original
                .control_points
                .iter()
                .map(|p| vec![p.clone(), p.clone()])
                .collect(),
            weights: original.weights.iter().map(|&w| vec![w, w]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let d = original.domain();
        let map = Curve::from_polyline(vec![vec![d[0], 0.], vec![d[1], 0.]])?;
        let deviation = curve_surface_agreement::verify(
            &curve,
            &map,
            &carrier,
            false,
            deviation_limit,
            max_cells - out.cells,
        )?;
        out.cells += deviation.cells;
        if deviation.status != Status::WithinTolerance {
            return Err(unproven(
                "Fitted boundary exceeds original-curve deviation bound",
            ));
        }
        let skin_use = patch.loops[patch.faces[0].outer]
            .coedges
            .iter()
            .find(|c| c.edge == edge)
            .ok_or_else(|| invalid("Fitting edge has no patch-face use"))?;
        let positions = nurbs_core::surface_g2_pcurves::verify_positions(
            &curve,
            [&pc, &skin_use.pcurve],
            [&face.surface, &patch.faces[0].surface],
            [false, skin_use.reversed],
            source.tolerance_mm,
            max_cells - out.cells,
            max_exact - out.exact_work,
        )?;
        out.cells += positions.cells;
        out.exact_work += positions.exact_work;
        if !positions.proven {
            return Err(unproven(
                "Fitted boundary original source/skin lifts unqualified",
            ));
        }
        out.model.edges[edge].curve = curve;
        out.source_pcurves[edge] = Some(pc);
        out.adjustments.push(BoundaryAdjustment {
            edge,
            source_face: target.face,
            displacement_upper_mm: deviation_limit,
        });
    }
    out.model.validate()?;
    Ok(out)
}
