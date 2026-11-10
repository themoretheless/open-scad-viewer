//! Shared original-root ownership across an adaptively constructed cut network.
use super::{SectionFace, invalid, unproven};
use crate::{Model, edge_partition, face_crossings};
use nurbs_core::{Result, ss_intersection::FiberAxis};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy)]
pub struct FiberFaceRequest {
    pub face: usize,
    pub parameter_box: [[f64; 2]; 4],
    pub axis: FiberAxis,
}
pub struct FiberCapRequest {
    pub section: FiberFaceRequest,
    pub remove_uv: [f64; 2],
}
pub struct AutomaticFiberCapRequest {
    pub face: usize,
    /// Original face UV point selecting the material sector to remove.
    pub remove_uv: [f64; 2],
}
pub struct FiberCap {
    pub cap: crate::region_replacement::SurfaceCap,
    /// Original source edge, root ordinal, vertex in the FINAL compacted body.
    pub root_vertices: Vec<(usize, usize, usize)>,
    pub cells: usize,
    pub exact_work: u64,
    pub evaluations: usize,
    pub root_error_upper_mm: f64,
}
/// One private transaction from shared original roots through curved face
/// sections and explicit material-side intent to an owned closed cap. No
/// partially cut model escapes on refusal. The actual cap/retained-face seam
/// can be sharp; no prescribed-radius or G2 corner claim is made.
pub fn trim_fiber_and_cap(
    source: &Model,
    requests: &[FiberCapRequest],
    remove_uncut: &[usize],
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<FiberCap> {
    trim_fiber_and_cap_impl(
        source,
        requests,
        remove_uncut,
        cutter,
        cutter_face,
        tolerance_uv,
        max_cells,
        max_exact,
        false,
    )
}

/// Infer every section chart from shared original roots, then select material
/// by the caller's original-face seeds and close atomically with the cutter.
pub fn trim_fiber_and_cap_automatic(
    source: &Model,
    requests: &[AutomaticFiberCapRequest],
    remove_uncut: &[usize],
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<FiberCap> {
    source.validate()?;
    cutter.validate()?;
    if !(1..=256).contains(&requests.len()) {
        return Err(invalid("Need 1..256 automatic section requests"));
    }
    let requests = requests
        .iter()
        .map(|r| {
            Ok(FiberCapRequest {
                section: FiberFaceRequest {
                    face: r.face,
                    parameter_box: super::automatic_fiber::original_parameter_box(
                        source,
                        r.face,
                        cutter,
                        cutter_face,
                    )?,
                    axis: FiberAxis::FirstU,
                },
                remove_uv: r.remove_uv,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    trim_fiber_and_cap_impl(
        source,
        &requests,
        remove_uncut,
        cutter,
        cutter_face,
        tolerance_uv,
        max_cells,
        max_exact,
        true,
    )
}

fn trim_fiber_and_cap_impl(
    source: &Model,
    requests: &[FiberCapRequest],
    remove_uncut: &[usize],
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
    automatic: bool,
) -> Result<FiberCap> {
    source.validate()?;
    let mut seen = BTreeSet::new();
    if requests.len() + remove_uncut.len() > 256
        || requests
            .iter()
            .any(|r| !seen.insert(r.section.face) || r.remove_uv.iter().any(|v| !v.is_finite()))
        || remove_uncut
            .iter()
            .any(|&f| f >= source.faces.len() || !seen.insert(f))
    {
        return Err(invalid(
            "Need distinct original sections and finite material-side seeds",
        ));
    }
    let sections = requests.iter().map(|r| r.section).collect::<Vec<_>>();
    let network = partition_fiber_network_impl(
        source,
        &sections,
        cutter,
        cutter_face,
        tolerance_uv,
        max_cells,
        max_exact,
        automatic,
    )?;
    let mut cells = network.cells;
    let mut selected = remove_uncut.to_vec();
    for (request, section) in requests.iter().zip(&network.sections) {
        let mut owner = None;
        for face in section.faces {
            if cells >= max_cells {
                return Err(unproven("Shared curved-cap selection work exhausted"));
            }
            let r = crate::face_domain::FaceDomain::new(&network.model, face, tolerance_uv)?
                .classify(request.remove_uv.map(|v| [v, v]), max_cells - cells)?;
            cells += r.cells;
            match r.location {
                nurbs_core::trim_domain::Location::Inside if r.winding == Some(1) => {
                    if owner.replace(face).is_some() {
                        return Err(unproven("Ambiguous removal-sector ownership"));
                    }
                }
                nurbs_core::trim_domain::Location::Outside => {}
                _ => {
                    return Err(unproven(
                        "Removal seed is on or unresolved near the curved section",
                    ));
                }
            }
        }
        selected.push(owner.ok_or_else(|| unproven("Removal seed owns no curved-cut sector"))?);
    }
    if max_cells.saturating_sub(cells) < 8 {
        return Err(unproven("No remaining curved-cap closure work"));
    }
    let cap = crate::region_replacement::cap_region(
        &network.model,
        &selected,
        &cutter.faces[cutter_face].surface,
        tolerance_uv,
        max_cells - cells,
        max_exact - network.exact_work,
    )?;
    cells += cap.cells;
    let exact_work = network.exact_work + cap.replacement.exact_work;
    let root_vertices = network
        .root_vertices
        .iter()
        .map(|&(edge, ordinal, vertex)| {
            cap.replacement.source_vertices[vertex]
                .map(|v| (edge, ordinal, v))
                .ok_or_else(|| unproven("Closed cap discarded an owned original root vertex"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(FiberCap {
        cap,
        root_vertices,
        cells,
        exact_work,
        evaluations: network.evaluations,
        root_error_upper_mm: network.root_error_upper_mm,
    })
}
pub struct FiberNetwork {
    /// All sectors remain. Region replacement is a separate private operation.
    pub model: Model,
    pub sections: Vec<SectionFace>,
    pub root_vertices: Vec<(usize, usize, usize)>,
    pub boundary_coverage: Vec<(usize, face_crossings::Report)>,
    pub cells: usize,
    pub exact_work: u64,
    pub evaluations: usize,
    pub root_error_upper_mm: f64,
}
/// Query every original outer edge ONCE against the actual cutter. Neighboring
/// faces reuse the same original root boxes and vertices, not new queries on
/// rounded descendant edges. All curved traces, bound endpoint edits and trim
/// partitions are freshly admitted under one shared work allowance. This does
/// not census hidden SS branches or remove material/close a body.
pub fn partition_fiber_network(
    source: &Model,
    requests: &[FiberFaceRequest],
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<FiberNetwork> {
    partition_fiber_network_impl(
        source,
        requests,
        cutter,
        cutter_face,
        tolerance_uv,
        max_cells,
        max_exact,
        false,
    )
}

/// Select local search charts from shared original roots, without asking the
/// caller for parameter boxes or free axes. This does not census hidden branches.
pub fn partition_fiber_network_automatic(
    source: &Model,
    faces: &[usize],
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<FiberNetwork> {
    source.validate()?;
    cutter.validate()?;
    if !(1..=256).contains(&faces.len()) {
        return Err(invalid("Need 1..256 distinct original faces"));
    }
    let requests = faces
        .iter()
        .map(|&face| {
            Ok(FiberFaceRequest {
                face,
                parameter_box: super::automatic_fiber::original_parameter_box(
                    source,
                    face,
                    cutter,
                    cutter_face,
                )?,
                axis: FiberAxis::FirstU,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    partition_fiber_network_impl(
        source,
        &requests,
        cutter,
        cutter_face,
        tolerance_uv,
        max_cells,
        max_exact,
        true,
    )
}

fn partition_fiber_network_impl(
    source: &Model,
    requests: &[FiberFaceRequest],
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
    automatic: bool,
) -> Result<FiberNetwork> {
    source.validate()?;
    cutter.validate()?;
    if !(1..=256).contains(&requests.len())
        || cutter_face >= cutter.faces.len()
        || !(32..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(invalid("Need bounded original curved-cut network inputs"));
    }
    let mut seen = BTreeSet::new();
    let mut union = BTreeSet::new();
    let mut by_face = Vec::new();
    for request in requests {
        if request.face >= source.faces.len()
            || !seen.insert(request.face)
            || !matches!(request.axis, FiberAxis::FirstU | FiberAxis::FirstV)
            || request
                .parameter_box
                .iter()
                .any(|d| !d[0].is_finite() || !d[1].is_finite() || d[0] >= d[1])
        {
            return Err(invalid(
                "Need distinct original faces and bounded first-support root charts",
            ));
        }
        let ring = &source.loops[source.faces[request.face].outer].coedges;
        let edges = ring.iter().map(|c| c.edge).collect::<BTreeSet<_>>();
        if !(3..=256).contains(&ring.len()) || ring.len() != edges.len() {
            return Err(unproven(
                "Curved-cut network needs distinct bounded original outer edges",
            ));
        }
        union.extend(edges.iter().copied());
        by_face.push(edges);
    }
    if union.len() > 4096 {
        return Err(invalid("Original root edge union exceeds its limit"));
    }
    let mut model = source.clone();
    let mut cells = 0;
    let mut exact_work = 0;
    let mut evaluations = 0;
    let mut root_vertices = Vec::new();
    let mut boundary_coverage = Vec::new();
    let mut roots = BTreeMap::new();
    let mut cutter_roots = BTreeMap::new();
    for edge in union {
        if cells >= max_cells {
            return Err(unproven("Shared original network root work exhausted"));
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
        let mut cutter_owned = Vec::new();
        for (ordinal, (&index, &vertex)) in split
            .coverage
            .eligible
            .iter()
            .zip(&split.vertices)
            .enumerate()
        {
            owned.push((vertex, split.coverage.roots[index].root.position_bounds));
            let b = split.coverage.roots[index].root.parameter_box;
            cutter_owned.push([b[1], b[2]]);
            root_vertices.push((edge, ordinal, vertex));
        }
        roots.insert(edge, owned);
        cutter_roots.insert(edge, cutter_owned);
        boundary_coverage.push((edge, split.coverage));
        model = split.model;
    }
    let mut sections = Vec::new();
    let mut root_error_upper_mm = 0_f64;
    for (index, (request, edges)) in requests.iter().zip(by_face).enumerate() {
        let owned = edges
            .iter()
            .flat_map(|e| roots[e].iter().copied())
            .collect::<Vec<_>>();
        let mut parameter_box = request.parameter_box;
        let axis = if automatic {
            let cutter_owned = edges
                .iter()
                .flat_map(|e| cutter_roots[e].iter().copied())
                .collect::<Vec<_>>();
            super::automatic_fiber::qualify_automatic_search(
                &model,
                request.face,
                cutter,
                cutter_face,
                &owned,
                &cutter_owned,
                &mut parameter_box,
                &mut cells,
                max_cells,
            )?
        } else {
            request.axis
        };
        let out = super::automatic_fiber::partition_fiber_prebound(
            &model,
            request.face,
            cutter,
            cutter_face,
            &owned,
            parameter_box,
            axis,
            tolerance_uv,
            max_cells - cells,
            (max_exact - exact_work) / (requests.len() - index) as u64,
            4096 - evaluations,
        )
        .map_err(|error| {
            nurbs_core::Error::new(error.code, format!(
                "Curved-cut face {} with search {:?}: {}",
                request.face, parameter_box, error.message
            ))
        })?;
        cells += out.cells;
        exact_work += out.partition.exact_work;
        evaluations += out.evaluations;
        root_error_upper_mm = root_error_upper_mm.max(out.root_error_upper_mm);
        sections.push(SectionFace {
            source_face: request.face,
            faces: out.partition.faces,
            endpoints: out.endpoints,
            edges: out.partition.edges,
        });
        model = out.partition.model;
    }
    model.refresh_change_set(&[source]);
    Ok(FiberNetwork {
        model,
        sections,
        root_vertices,
        boundary_coverage,
        cells,
        exact_work,
        evaluations,
        root_error_upper_mm,
    })
}
