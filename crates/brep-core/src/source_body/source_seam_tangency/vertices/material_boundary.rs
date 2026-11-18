//! Complete exact material boundary on one admitted original body.
use super::{Carrier, Class, FaceEvidence, Vertex, qualify_impl};
use crate::source_seam_tangency::{
    Collection, ExactSmoothLimits, material_orientation, qualify_exact_smooth_collection,
};
use crate::source_volume::{
    Body,
    regularity::{
        smooth::{SmoothFaces, StratifiedSmoothFaces},
        stratified::PoleEvidence,
    },
};
use nurbs_core::{Error, Result};

pub struct Limits {
    pub exact_seams: ExactSmoothLimits,
    pub material_orientation: material_orientation::Limits,
    pub vertices: super::Limits,
}

pub struct Projections<'a> {
    pub edges: &'a [usize],
    pub vertices: &'a [usize],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    ExactSeams,
    MaterialOrientation,
    VertexNeighborhoods,
    Complete,
}

pub struct Report<'a, E: Carrier> {
    pub certificate: Option<MaterialBoundary<'a, E>>,
    pub stage: Stage,
    pub reason: &'static str,
    pub uncertain_index: Option<usize>,
    pub seam_exact_work: u64,
    pub orientation_steps: usize,
    pub vertex_steps: usize,
}

/// A complete stratified boundary proof of the original material body.
/// Each smooth edge has exact jets and a consistent material side. Face and
/// vertex evidence limits the order of the resulting physical surface.
pub struct MaterialBoundary<'a, E: Carrier> {
    body: &'a Body<E>,
    edge_classes: &'a [Class],
    interior_order: usize,
    vertices: Vec<Vertex>,
}

impl<E: Carrier> MaterialBoundary<'_, E> {
    pub fn body(&self) -> &Body<E> {
        self.body
    }
    pub fn edge_classes(&self) -> &[Class] {
        self.edge_classes
    }
    pub fn interior_order(&self) -> usize {
        self.interior_order
    }
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }
    /// Minimum geometric order over face interiors, exact joins and physical
    /// vertex neighborhoods. A declared sharp stratum gives order zero.
    pub fn global_order(&self) -> usize {
        let edges = self.edge_classes.iter().map(|class| match class {
            Class::C0 => 0,
            Class::TangentPlanes => 1,
            Class::Curvature { .. } => 2,
        });
        let vertices = self.vertices.iter().map(|vertex| match vertex {
            Vertex::C0 { .. } => 0,
            Vertex::Smooth(neighborhood) => neighborhood.order(),
            Vertex::Pole { order, .. } => *order,
        });
        edges.chain(vertices).fold(self.interior_order, usize::min)
    }
}

pub fn qualify<'a, E: Carrier + 'a>(
    faces: &'a SmoothFaces<'_, '_, E>,
    seams: &'a Collection<'_, E>,
    projections: Projections<'_>,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a, E>> {
    qualify_material(faces, seams, projections, tolerance_uv, limits)
}

/// Include already qualified physical pole neighborhoods from this body.
pub fn qualify_stratified<'a, 'b, 's, E: Carrier + 'a, P: PoleEvidence<'b, 's, E>>(
    faces: &'a StratifiedSmoothFaces<'_, '_, 'b, 's, E, P>,
    seams: &'a Collection<'_, E>,
    projections: Projections<'_>,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a, E>> {
    qualify_material(faces, seams, projections, tolerance_uv, limits)
}

fn qualify_material<'a, E: Carrier + 'a>(
    faces: &'a dyn FaceEvidence<E>,
    seams: &'a Collection<'_, E>,
    projections: Projections<'_>,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a, E>> {
    if !std::ptr::eq(faces.body(), seams.body()) {
        return Err(Error::new(
            "BREP_SOURCE_MATERIAL_BOUNDARY",
            "Face and seam evidence must own the same original material body",
        ));
    }
    let exact = qualify_exact_smooth_collection(seams, projections.edges, limits.exact_seams)?;
    let mut out = Report {
        certificate: None,
        stage: Stage::ExactSeams,
        reason: exact.reason,
        uncertain_index: exact.uncertain_edge,
        seam_exact_work: exact.exact_work,
        orientation_steps: 0,
        vertex_steps: 0,
    };
    let Some(exact) = exact.certificate else {
        return Ok(out);
    };
    let oriented = material_orientation::qualify(&exact, limits.material_orientation)?;
    out.stage = Stage::MaterialOrientation;
    out.reason = oriented.reason;
    out.uncertain_index = oriented.uncertain_edge;
    out.orientation_steps = oriented.steps;
    let Some(oriented) = oriented.certificate else {
        return Ok(out);
    };
    let vertices = qualify_impl(
        faces,
        &oriented,
        projections.vertices,
        tolerance_uv,
        limits.vertices,
    )?;
    out.stage = Stage::VertexNeighborhoods;
    out.reason = vertices.reason;
    out.uncertain_index = vertices.uncertain_vertex;
    out.vertex_steps = vertices.steps;
    if let Some(atlas) = vertices.certificate {
        out.stage = Stage::Complete;
        out.certificate = Some(MaterialBoundary {
            body: faces.body(),
            edge_classes: seams.classes(),
            interior_order: faces.order(),
            vertices: atlas.vertices,
        });
    }
    Ok(out)
}
