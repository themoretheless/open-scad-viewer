//! One transaction from original supports to shared-root nonlinear face cuts.
use super::{Partition, Segment, invalid, unproven};
use crate::{Model, edge_partition, face_crossings};
use nurbs_core::{Result, interval_eval::Interval as I};
pub struct AutomaticPartition {
    /// Final model; nested counters and identity describe the final cut stage.
    /// Use `cells` and `original_face_identity` for the complete transaction.
    pub partition: Partition,
    pub endpoints: [usize; 2],
    pub original_face_identity: crate::TopoId,
    pub boundary_coverage: Vec<(usize, face_crossings::Report)>,
    /// Generation, every boundary query/split, fresh endpoint binding and cut.
    pub cells: usize,
}
/// Infer both end roots of a clipped ruled-chart section by inspecting
/// every original outer edge against the actual trimmed cutter face. Exactly
/// two retained roots with increasing original chart parameters are required.
/// Every partial model is private. Both material sectors remain present; this
/// does not remove material or claim full SS coverage, radius or solid validity.
pub fn partition_ruled_at_face(
    source: &Model,
    face: usize,
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<AutomaticPartition> {
    source.validate()?;
    cutter.validate()?;
    if !(8..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(invalid("Need bounded automatic section and root work"));
    }
    let support = source
        .faces
        .get(face)
        .ok_or_else(|| invalid("Missing original source face"))?;
    cutter
        .faces
        .get(cutter_face)
        .ok_or_else(|| invalid("Missing actual cutter face"))?;
    let edges = source.loops[support.outer]
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
            "Automatic endpoints need bounded distinct original outer edges",
        ));
    }
    let mut cells = 0;
    let mut model = source.clone();
    let mut coverage = Vec::new();
    let mut roots = Vec::new();
    for edge in edges {
        if cells == max_cells {
            return Err(unproven("Shared automatic boundary work exhausted"));
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
        }
        if roots.len() > 2 {
            return Err(unproven(
                "More than two retained boundary roots need branch ownership",
            ));
        }
        coverage.push((edge, split.coverage));
        model = split.model;
    }
    if roots.len() != 2 {
        return Err(unproven(
            "Section needs exactly two retained original boundary roots",
        ));
    }
    let (mut partition, endpoints, stage_cells) = partition_prebound(
        &model,
        face,
        cutter,
        cutter_face,
        &roots,
        tolerance_uv,
        max_cells - cells,
        max_exact,
    )?;
    cells += stage_cells;
    partition.model.refresh_change_set(&[source]);
    Ok(AutomaticPartition {
        partition,
        endpoints,
        original_face_identity: source.1.faces[face],
        boundary_coverage: coverage,
        cells,
    })
}

pub(super) fn partition_prebound(
    source: &Model,
    face: usize,
    cutter: &Model,
    cutter_face: usize,
    roots: &[(usize, [[f64; 2]; 3])],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<(Partition, [usize; 2], usize)> {
    if roots.len() != 2 || max_cells < 8 {
        return Err(unproven("Need two bound roots and remaining section work"));
    }
    let model = source;
    let support = &source.faces[face];
    let second = &cutter.faces[cutter_face];
    let mut cells = 0;
    let mut owned = Vec::new();
    for &(vertex, bounds) in roots {
        let mut squared = I::point(0.);
        for k in 0..3 {
            let difference = I::point(model.vertices[vertex].point[k])
                .sub(I::new(bounds[k][0], bounds[k][1])?)?;
            let upper = difference.lo.abs().max(difference.hi.abs());
            squared = squared.add(I::point(upper).mul(I::point(upper))?)?;
        }
        if squared.hi
            > I::point(source.tolerance_mm)
                .mul(I::point(source.tolerance_mm))?
                .lo
        {
            return Err(unproven(
                "Owned vertex is not bounded against its complete original root box",
            ));
        }
        let uses = model.loops[model.faces[face].outer]
            .coedges
            .iter()
            .filter(|c| model.edges[c.edge].vertices[usize::from(c.reversed)] == vertex)
            .collect::<Vec<_>>();
        if uses.len() != 1 {
            return Err(unproven(
                "Root needs unique original-chart boundary ownership",
            ));
        }
        let uv = &uses[0].pcurve.control_points[0];
        owned.push((vertex, [uv[0], uv[1]]));
    }
    let longitudinal = super::ruled_intersection::longitudinal_axis(&support.surface, &second.surface)?;
    owned.sort_by(|a, b| a.1[longitudinal].total_cmp(&b.1[longitudinal]));
    if owned[0].1[longitudinal] >= owned[1].1[longitudinal] || max_cells - cells < 4 {
        return Err(unproven(
            "Ruled branch needs increasing owned longitudinal endpoints",
        ));
    }
    let endpoints = [owned[0].0, owned[1].0];
    let uv_ends = [owned[0].1, owned[1].1];
    let candidate = super::generate_ruled_plane_cut_between(
        [&support.surface, &second.surface],
        uv_ends,
        endpoints.map(|v| model.vertices[v].point),
        source.tolerance_mm,
        max_cells - cells,
    )?;
    cells += candidate.cells;
    // Position on a carrier is not membership in the actual cutter face.
    // A positive rational pcurve lies in its control hull; qualify that whole
    // enclosure, including holes, before permitting any boundary mutation.
    let hull: [[f64; 2]; 2] = std::array::from_fn(|k| {
        [
            candidate.pcurves[1]
                .control_points
                .iter()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min),
            candidate.pcurves[1]
                .control_points
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max),
        ]
    });
    let domain = crate::face_domain::FaceDomain::new(cutter, cutter_face, tolerance_uv)?;
    if !domain.contains_closed_rectangle(hull) {
        if cells == max_cells {
            return Err(unproven("No remaining cutter membership work"));
        }
        let membership = domain.classify(hull, max_cells - cells)?;
        cells += membership.cells;
        if membership.location != nurbs_core::trim_domain::Location::Inside
            || membership.winding != Some(1)
        {
            return Err(unproven(
                "Complete generated section is not proven inside the actual cutter trim",
            ));
        }
    }
    let bound = super::bind_intersection_ends(
        &model,
        face,
        endpoints,
        &candidate,
        &second.surface,
        max_cells - cells,
    )?;
    cells += bound.cells;
    if cells == max_cells {
        return Err(unproven("No remaining nonlinear partition work"));
    }
    let partition = super::partition_with_budget(
        &model,
        face,
        endpoints,
        &[Segment {
            curve: bound.curve,
            pcurve: bound.pcurves[0].clone(),
        }],
        tolerance_uv,
        max_cells - cells,
        max_exact,
    )?;
    cells += partition.cells;
    Ok((partition, endpoints, cells))
}
