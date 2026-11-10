//! Subdivide shared source edges once before cutting adjacent support faces.
use super::{invalid, unproven};
use crate::{Model, edge_partition, face_crossings};
use nurbs_core::Result;
use std::collections::{BTreeMap, BTreeSet};
pub struct SectionFace {
    pub source_face: usize,
    pub faces: [usize; 2],
    pub endpoints: [usize; 2],
    pub edges: Vec<usize>,
}
pub struct AutomaticNetwork {
    /// All sectors remain; no material has been removed or capped.
    pub model: Model,
    pub sections: Vec<SectionFace>,
    pub boundary_coverage: Vec<(usize, face_crossings::Report)>,
    /// Original edge, retained root ordinal and shared saved vertex.
    pub root_vertices: Vec<(usize, usize, usize)>,
    pub cells: usize,
    pub exact_work: u64,
}
/// One private transaction over adjacent original faces and one actual cutter.
/// The union of original outer edges is queried once. Every incident face uses
/// the SAME root vertex; roots are never rediscovered on rounded child edges.
/// Each requested face must have two retained roots and an admitted section.
pub fn partition_ruled_network(
    source: &Model,
    faces: &[usize],
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<AutomaticNetwork> {
    source.validate()?;
    cutter.validate()?;
    if !(1..=256).contains(&faces.len())
        || !(8..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || cutter_face >= cutter.faces.len()
    {
        return Err(invalid(
            "Need bounded original face network and actual cutter",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let mut by_face = Vec::new();
    for &face in faces {
        if face >= source.faces.len() || !seen.insert(face) {
            return Err(invalid("Section faces must be distinct original faces"));
        }
        let ring = &source.loops[source.faces[face].outer].coedges;
        let owned = ring.iter().map(|c| c.edge).collect::<BTreeSet<_>>();
        if !(3..=256).contains(&ring.len()) || owned.len() != ring.len() {
            return Err(unproven("Section needs distinct bounded outer edges"));
        }
        edges.extend(owned.iter().copied());
        by_face.push(owned);
    }
    if edges.len() > 4096 {
        return Err(invalid("Original section-edge union exceeds its bound"));
    }
    let mut model = source.clone();
    let mut cells = 0;
    let mut exact_work = 0;
    let mut root_vertices = Vec::new();
    let mut boundary_coverage = Vec::new();
    let mut roots = BTreeMap::new();
    for edge in edges {
        if cells == max_cells {
            return Err(unproven("Shared network boundary work exhausted"));
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
        let mut owned = Vec::new();
        for (ordinal, (&index, &vertex)) in split
            .coverage
            .eligible
            .iter()
            .zip(&split.vertices)
            .enumerate()
        {
            owned.push((vertex, split.coverage.roots[index].root.position_bounds));
            root_vertices.push((edge, ordinal, vertex));
        }
        roots.insert(edge, owned);
        boundary_coverage.push((edge, split.coverage));
        model = split.model;
    }
    let mut sections = Vec::new();
    for (&face, edges) in faces.iter().zip(by_face) {
        let owned = edges
            .iter()
            .flat_map(|edge| roots[edge].iter().copied())
            .collect::<Vec<_>>();
        let (cut, endpoints, stage_cells) = super::automatic_partition::partition_prebound(
            &model,
            face,
            cutter,
            cutter_face,
            &owned,
            tolerance_uv,
            max_cells - cells,
            max_exact - exact_work,
        )?;
        cells += stage_cells;
        exact_work += cut.exact_work;
        sections.push(SectionFace {
            source_face: face,
            faces: cut.faces,
            endpoints,
            edges: cut.edges,
        });
        model = cut.model;
    }
    model.refresh_change_set(&[source]);
    Ok(AutomaticNetwork {
        model,
        sections,
        boundary_coverage,
        root_vertices,
        cells,
        exact_work,
    })
}
