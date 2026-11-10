//! Whole original-pcurve G1 qualification of every authored round seam.
use crate::{Model, Result, watertight};
use nurbs_core::{surface_g1_pcurves, surface_join::Decision};
use std::sync::Arc;
pub mod material;
pub use material::QualifiedVolume;

/// Proof observations are bound to an immutable snapshot, not a caller's
/// subsequently mutable report model. This is local G1 and closed boundary
/// agreement, not global embedding or a reference-parity certificate.
pub struct Qualification {
    model: Model,
    edges: [usize; 3],
    all_edges: Vec<usize>,
    seams: Vec<surface_g1_pcurves::Report>,
    envelopes: Vec<(usize, nurbs_core::moving_envelope::Envelope)>,
    closure: watertight::QualifiedReport,
    cells: usize,
    exact_work: u64,
}

/// Immutable closed embedded boundary. Filled-material roles and cavities are
/// separate: this type does not promote a local audit to a solid-volume proof.
pub struct EmbeddedBoundary {
    qualification: Arc<Qualification>,
    report: crate::boundary_embedding::Report,
}
impl EmbeddedBoundary {
    pub fn model(&self) -> &Model {
        self.qualification.model()
    }
    pub fn qualification(&self) -> &Qualification {
        &self.qualification
    }
    pub fn report(&self) -> &crate::boundary_embedding::Report {
        &self.report
    }
}
impl std::fmt::Debug for Qualification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FullRoundSeams")
            .field("edges", &self.edges)
            .field("cells", &self.cells)
            .field("exact_work", &self.exact_work)
            .finish()
    }
}
impl Qualification {
    pub fn model(&self) -> &Model {
        &self.model
    }
    /// The two support contacts and crown; see all_edges for the complete chain.
    pub fn edges(&self) -> [usize; 3] {
        self.edges
    }
    /// Complete ordered longitudinal chain, including every internal arc span.
    pub fn all_edges(&self) -> &[usize] {
        &self.all_edges
    }
    pub fn seams(&self) -> &[surface_g1_pcurves::Report] {
        &self.seams
    }
    pub fn closure(&self) -> &watertight::QualifiedReport {
        &self.closure
    }
    /// Whole-surface distance and radial-normal certificates for EVERY authored
    /// circular side face, indexed in the immutable model.
    pub fn envelopes(&self) -> &[(usize, nurbs_core::moving_envelope::Envelope)] {
        &self.envelopes
    }
    pub fn work(&self) -> (usize, u64) {
        (self.cells, self.exact_work)
    }
    /// Fresh full-boundary proof on the immutable qualified construction.
    /// Budgets belong to the embedding stage and are explicit, independent of
    /// the earlier seam/radius/closure work. No caller-positive evidence is used.
    pub fn qualify_embedding(
        self: Arc<Self>,
        tolerance_uv: f64,
        limits: crate::boundary_embedding::Limits,
        max_linear_cells: usize,
    ) -> Result<EmbeddedBoundary> {
        let report = crate::boundary_embedding::inspect_with_linear(
            self.model(),
            tolerance_uv,
            limits,
            max_linear_cells,
        )?;
        if !report.proven {
            return Err(super::error(format!(
                "Full Round boundary embedding remains unproven: exact agreement={}, trims={}, face injectivity={}, all face pairs={}, pending={:?}",
                report.agreement.all_equal && report.agreement.all_joins_exact,
                report.trim.all_valid,
                report.intersections.faces.all_faces_injective,
                report.intersections.pairs.all_pairs_classified,
                report.intersections.pairs.next_pair,
            )));
        }
        Ok(EmbeddedBoundary {
            qualification: self,
            report,
        })
    }
}

pub(super) fn qualify(
    model: &Model,
    endpoints: [[[f64; 3]; 2]; 3],
    centers: [[f64; 3]; 2],
    radius: f64,
) -> Result<Arc<Qualification>> {
    qualify_chain(model, &endpoints, 1, centers, radius)
}

pub(super) fn qualify_chain(
    model: &Model,
    endpoints: &[[[f64; 3]; 2]],
    crown: usize,
    centers: [[f64; 3]; 2],
    radius: f64,
) -> Result<Arc<Qualification>> {
    if !(3..=255).contains(&endpoints.len()) || crown == 0 || crown >= endpoints.len() - 1 {
        return Err(super::error(
            "Full Round needs a bounded ordered arc chain with an internal crown",
        ));
    }
    const MAX_CELLS: usize = 100_000;
    const MAX_EXACT: u64 = 1_000_000;
    let near = |a: [f64; 3], b: [f64; 3]| {
        (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]) <= model.tolerance_mm
    };
    let mut edges = Vec::new();
    let mut seams = Vec::new();
    let mut cells = 0;
    let mut exact_work = 0;
    for pair in endpoints {
        let matching = model
            .edges
            .iter()
            .enumerate()
            .filter_map(|(id, edge)| {
                let [a, b] = edge.vertices.map(|v| model.vertices[v].point);
                ((near(a, pair[0]) && near(b, pair[1])) || (near(a, pair[1]) && near(b, pair[0])))
                    .then_some(id)
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(super::error(
                "Full Round seam must own exactly one authored edge",
            ));
        }
        let edge = matching[0];
        if edges.contains(&edge) {
            return Err(super::error("Full Round authored seams must be distinct"));
        }
        edges.push(edge);
    }
    let mut arc_faces = Vec::new();
    for pair in edges.windows(2) {
        let owners = model
            .faces
            .iter()
            .enumerate()
            .filter_map(|(id, face)| {
                pair.iter()
                    .all(|edge| {
                        std::iter::once(face.outer)
                            .chain(face.holes.iter().copied())
                            .any(|w| model.loops[w].coedges.iter().any(|c| c.edge == *edge))
                    })
                    .then_some(id)
            })
            .collect::<Vec<_>>();
        if owners.len() != 1 || arc_faces.contains(&owners[0]) {
            return Err(super::error(
                "Consecutive Full Round seams must own exactly one distinct authored side",
            ));
        }
        arc_faces.push(owners[0]);
    }
    for (i, &edge) in edges.iter().enumerate() {
        let mut owners = Vec::new();
        for face in &model.faces {
            for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for coedge in &model.loops[wire].coedges {
                    if coedge.edge == edge {
                        owners.push((face, coedge));
                    }
                }
            }
        }
        if owners.len() != 2 {
            return Err(super::error(
                "Full Round seam must have two actual face owners",
            ));
        }
        if cells >= MAX_CELLS || exact_work >= MAX_EXACT {
            return Err(super::error("Full Round seam proof allowance exhausted"));
        }
        let report = surface_g1_pcurves::inspect(
            &model.edges[edge].curve,
            [&owners[0].1.pcurve, &owners[1].1.pcurve],
            [&owners[0].0.surface, &owners[1].0.surface],
            [owners[0].1.reversed, owners[1].1.reversed],
            model.tolerance_mm,
            1e-5,
            (MAX_CELLS - cells) / (edges.len() + 1 - i),
            MAX_EXACT - exact_work,
        )?;
        cells += report.cells;
        exact_work += report.work();
        if !report.positions.proven
            || !report.regularity_proven
            || report.decision != Decision::WithinTolerance
        {
            return Err(super::error(
                "Full Round original-pcurve G1 seam remains unproven",
            ));
        }
        seams.push(report);
    }
    let center_curve =
        nurbs_core::curve::Curve::from_polyline(centers.map(|p| p.to_vec()).to_vec())?;
    let law = nurbs_core::curve::Curve::from_polyline(vec![vec![radius, 0.], vec![radius, 0.]])?;
    let mut envelopes = Vec::new();
    for (i, &face) in arc_faces.iter().enumerate() {
        if cells >= MAX_CELLS || exact_work >= MAX_EXACT {
            return Err(super::error("Full Round radius proof allowance exhausted"));
        }
        let surface = nurbs_core::surface_edit::transpose(&model.faces[face].surface)?;
        let mut proof = nurbs_core::moving_radius::qualify(
            &surface,
            &center_curve,
            &law,
            model.tolerance_mm,
            (MAX_CELLS - cells) / (arc_faces.len() + 1 - i),
            MAX_EXACT - exact_work,
        )?;
        cells += proof.cells;
        exact_work += proof.work;
        let certificate = proof.certificate.take().ok_or_else(|| {
            super::error("Full Round whole-side radius or radial-normal envelope remains unproven")
        })?;
        let allowance = (MAX_CELLS - cells) / (arc_faces.len() + 1 - i) / 3;
        if allowance == 0 || exact_work >= MAX_EXACT {
            return Err(super::error(
                "Full Round envelope proof allowance exhausted",
            ));
        }
        let mut envelope = nurbs_core::moving_envelope::qualify(
            certificate,
            nurbs_core::moving_envelope::Limits {
                max_sine_squared: 1e-10,
                cells: allowance,
                surface_spans: allowance,
                center_spans: allowance,
                radial_work: MAX_EXACT - exact_work,
            },
        )?;
        cells += envelope.cells + envelope.surface_spans + envelope.center_spans;
        exact_work += envelope.radial_work;
        let envelope = envelope.envelope.take().ok_or_else(|| {
            super::error("Full Round whole-side radial-normal envelope remains unproven")
        })?;
        envelopes.push((face, envelope));
    }
    if cells >= MAX_CELLS || exact_work >= MAX_EXACT {
        return Err(super::error("Full Round closure proof allowance exhausted"));
    }
    let closure = watertight::inspect_with_exact(model, MAX_CELLS - cells, MAX_EXACT - exact_work)?;
    cells += closure.closure.agreement.cells;
    exact_work += closure.exact.work;
    if closure.closure.status != watertight::Closure::WithinTolerance {
        return Err(super::error(
            "Full Round whole boundary agreement remains unproven",
        ));
    }
    Ok(Arc::new(Qualification {
        model: model.clone(),
        edges: [edges[0], edges[crown], *edges.last().unwrap()],
        all_edges: edges,
        seams,
        envelopes,
        closure,
        cells,
        exact_work,
    }))
}
