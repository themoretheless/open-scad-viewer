//! Actual boundary-attached removal from a checked existing Model. General
//! algebraic side walls remain native implicit ruled carriers, not fitted NURBS.
use super::*;
mod bounds;
mod topology;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PocketEdge {
    Source(usize),
    Top(u32),
    Bottom(u32),
    Vertical(u32),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PocketVertex {
    Source(usize),
    Top(u32),
    Bottom(u32),
}
fn edge_vertices(
    model: &Model,
    partition: &domains::DomainPartition,
    edge: &PocketEdge,
) -> Result<[PocketVertex; 2]> {
    match edge {
        PocketEdge::Source(id) => Ok(model
            .edges
            .get(*id)
            .ok_or_else(|| fail("pocket source edge is absent"))?
            .vertices
            .map(PocketVertex::Source)),
        PocketEdge::Top(id) | PocketEdge::Bottom(id) => {
            let e = partition
                .edges
                .get(*id as usize)
                .ok_or_else(|| fail("pocket algebraic edge is absent"))?;
            Ok(e.endpoints.map(|v| {
                if matches!(edge, PocketEdge::Top(_)) {
                    PocketVertex::Top(v)
                } else {
                    PocketVertex::Bottom(v)
                }
            }))
        }
        PocketEdge::Vertical(v) => {
            if *v as usize >= partition.vertices.len() {
                return Err(fail("pocket algebraic vertex is absent"));
            }
            Ok([PocketVertex::Top(*v), PocketVertex::Bottom(*v)])
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PocketEdgeUse {
    pub edge: PocketEdge,
    pub positive: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PocketFaceKind {
    Source {
        face: usize,
        reversed: bool,
    },
    /// Always intersect this predicate with the original authored face trims.
    Remainder {
        source_face: usize,
        regions: Vec<u32>,
        outside_triangle: bool,
        reversed: bool,
    },
    Floor {
        component: u32,
        regions: Vec<u32>,
        reversed: bool,
    },
    /// Exact ruled image (1-t)*original(u,v)+t*floor(u,v) on this CAD edge
    /// (root section or critical-fiber segment), t in [0,1]. Generally not
    /// a rational NURBS surface; no fitted replacement is stored.
    Wall {
        domain_edge: u32,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PocketFace {
    pub kind: PocketFaceKind,
    /// Outward shell traversal; source face reversal is already applied.
    pub loops: Vec<Vec<PocketEdgeUse>>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ModelPocketCertificate {
    pub version: u32,
    pub source_model: String,
    pub floor_source: String,
    pub source_face: usize,
    pub depth: u8,
    pub domain: DomainPolynomial,
    pub factors: Vec<DomainPolynomial>,
    pub window: [[ExactRatio; 2]; 2],
    pub normalized_parameter_maps: [[[ExactRatio; 3]; 2]; 2],
    pub partition: domains::DomainPartition,
    pub selected: Vec<u32>,
    pub components: Vec<Vec<u32>>,
    pub source_atlas: Certificate,
    pub source_nominal_volume: ExactRatio,
    pub face_window_winding: crate::profile_certificate::PointWindingCertificate,
    pub swept_bounds: [[ExactRatio; 2]; 3],
    pub faces: Vec<PocketFace>,
}
#[derive(Clone, Debug)]
pub struct AlgebraicModelPocket {
    original: Model,
    floor: Surface,
    powers: [[Bi; 4]; 2],
    certificate: ModelPocketCertificate,
}
impl AlgebraicModelPocket {
    /// The input Model is retained for source-backed face/edge geometry. It is
    /// NOT the result Model; consumers must apply this handle's native faces.
    pub fn source_model(&self) -> &Model {
        &self.original
    }
    pub fn floor_carrier(&self) -> &Surface {
        &self.floor
    }
    pub fn certificate(&self) -> &ModelPocketCertificate {
        &self.certificate
    }
    pub fn faces(&self) -> &[PocketFace] {
        &self.certificate.faces
    }
    pub fn edge_vertices(&self, edge: &PocketEdge) -> Result<[PocketVertex; 2]> {
        edge_vertices(&self.original, &self.certificate.partition, edge)
    }

    pub fn into_inputs(self) -> (Model, Surface) {
        (self.original, self.floor)
    }
    fn query(&self, u: &ExactRatio, v: &ExactRatio) -> Result<(Q, Q, DomainLocation)> {
        let u = roots::decode(u)?;
        let v = roots::decode(v)?;
        if u < Q::zero()
            || v < Q::zero()
            || &u + &v > Q::one()
            || [&u, &v]
                .iter()
                .any(|q| q.numer().bits() > 512 || q.denom().bits() > 512)
        {
            return Err(fail(
                "pocket query outside common triangle or rational budget",
            ));
        }
        let location = domains::locate(&self.certificate.partition, &u, &v)?;
        Ok((u, v, location))
    }
    fn belongs(&self, cells: &[u32], location: &DomainLocation) -> bool {
        let p = &self.certificate.partition;
        match location {
            DomainLocation::Face(f) => cells.contains(f),
            DomainLocation::Edge(e) => cells.iter().any(|f| {
                p.faces[*f as usize]
                    .boundary
                    .iter()
                    .any(|x| x.unsigned_abs() - 1 == *e)
            }),
            DomainLocation::Vertex(v) => cells.iter().any(|f| {
                p.faces[*f as usize]
                    .boundary
                    .iter()
                    .any(|x| p.edges[x.unsigned_abs() as usize - 1].endpoints.contains(v))
            }),
        }
    }
    fn point(&self, carrier: usize, u: &Q, v: &Q) -> Result<P> {
        let h = &self.powers[carrier];
        let w = at(&h[3], u, v);
        if w <= Q::zero() {
            return Err(fail("pocket query has an unproved denominator"));
        }
        let p = std::array::from_fn(|i| at(&h[i], u, v) / &w);
        guard(&p)?;
        Ok(p)
    }
    pub fn evaluate_floor_rational(
        &self,
        component: u32,
        u: &ExactRatio,
        v: &ExactRatio,
    ) -> Result<[ExactRatio; 3]> {
        let (u, v, location) = self.query(u, v)?;
        let cells = self
            .certificate
            .components
            .get(component as usize)
            .ok_or_else(|| fail("pocket floor component is absent"))?;
        if !self.belongs(cells, &location) {
            return Err(fail("query is outside the selected pocket floor"));
        }
        Ok(self.point(1, &u, &v)?.each_ref().map(roots::ratio))
    }
    /// Local retained-source query within the proved guard window only.
    pub fn evaluate_retained_rational(
        &self,
        u: &ExactRatio,
        v: &ExactRatio,
    ) -> Result<[ExactRatio; 3]> {
        let (u, v, location) = self.query(u, v)?;
        let w = self
            .certificate
            .window
            .each_ref()
            .map(|r| r.each_ref().map(roots::decode));
        if u < w[0][0].clone()?
            || u > w[0][1].clone()?
            || v < w[1][0].clone()?
            || v > w[1][1].clone()?
        {
            return Err(fail("retained query outside the proved source-face window"));
        }
        let partition = &self.certificate.partition;
        let selected = &self.certificate.selected;
        let incident = match location {
            DomainLocation::Face(f) => vec![f],
            DomainLocation::Edge(e) => partition
                .faces
                .iter()
                .enumerate()
                .filter(|(_, f)| f.boundary.iter().any(|x| x.unsigned_abs() - 1 == e))
                .map(|(f, _)| f as u32)
                .collect(),
            DomainLocation::Vertex(v) => partition
                .faces
                .iter()
                .enumerate()
                .filter(|(_, f)| {
                    f.boundary.iter().any(|x| {
                        partition.edges[x.unsigned_abs() as usize - 1]
                            .endpoints
                            .contains(&v)
                    })
                })
                .map(|(f, _)| f as u32)
                .collect(),
        };
        if !incident.is_empty() && incident.iter().all(|f| selected.contains(f)) {
            return Err(fail("the queried original-face region was removed"));
        }
        Ok(self.point(0, &u, &v)?.each_ref().map(roots::ratio))
    }
    pub fn evaluate_wall_rational(
        &self,
        edge: u32,
        u: &ExactRatio,
        v: &ExactRatio,
        t: &ExactRatio,
    ) -> Result<[ExactRatio; 3]> {
        let (u, v, location) = self.query(u, v)?;
        let t = roots::decode(t)?;
        if t < Q::zero() || t > Q::one() || t.numer().bits() > 512 || t.denom().bits() > 512 {
            return Err(fail(
                "pocket wall interpolation parameter is outside budget",
            ));
        }
        if !self
            .certificate
            .faces
            .iter()
            .any(|f| f.kind == PocketFaceKind::Wall { domain_edge: edge })
        {
            return Err(fail("pocket wall edge is absent"));
        }
        let admitted = match location {
            DomainLocation::Edge(e) => e == edge,
            DomainLocation::Vertex(v) => self.certificate.partition.edges[edge as usize]
                .endpoints
                .contains(&v),
            _ => false,
        };
        if !admitted {
            return Err(fail("wall query is not on its exact algebraic edge"));
        }
        let a = self.point(0, &u, &v)?;
        let b = self.point(1, &u, &v)?;
        let p = std::array::from_fn(|i| &a[i] + &t * (&b[i] - &a[i]));
        guard(&p)?;
        Ok(p.each_ref().map(roots::ratio))
    }
}
/// Remove arbitrary selected two-dimensional CAD cells inside a proved window,
/// replacing the original face region with an independent floor and exact
/// algebraic ruled walls. Singular D is allowed when the selected union has
/// closed nonbranching boundary and every native edge pairs consistently.
/// Currently source face and floor need affine XY graph projections; other
/// original faces may be arbitrary checked NURBS, including nongraph carriers.
pub fn certify_model_pocket(
    model: &Model,
    source_face: usize,
    floor: &Surface,
    domain: &DomainPolynomial,
    factors: &[DomainPolynomial],
    window: [[ExactRatio; 2]; 2],
    selection: Option<&[u32]>,
    depth: u8,
) -> Result<AlgebraicModelPocket> {
    if source_face >= model.faces.len() || depth > 3 || factors.len() > 11 {
        return Err(fail("pocket source index/depth/factor budget exceeded"));
    }
    let (source_atlas, _) = checked_charts(model, depth)?;
    let volume = super::super::difference::connected_outer_volume(model, &source_atlas)?;
    let uses = model.shells[model.bodies[0].outer_shell]
        .faces
        .iter()
        .filter(|u| u.face == source_face)
        .collect::<Vec<_>>();
    if uses.len() != 1 {
        return Err(fail("pocket target needs one original shell face use"));
    }
    let mut work = Work::default();
    let upper = &model.faces[source_face].surface;
    let aligned = align(upper, floor, &mut work)?;
    let orientation = (&aligned.xy[0][1] * &aligned.xy[1][2]
        - &aligned.xy[0][2] * &aligned.xy[1][1])
        .is_negative();
    if uses[0].reversed != orientation {
        return Err(fail("pocket target must face outward along positive Z"));
    }
    let win: [[Q; 2]; 2] = [
        [roots::decode(&window[0][0])?, roots::decode(&window[0][1])?],
        [roots::decode(&window[1][0])?, roots::decode(&window[1][1])?],
    ];
    if win
        .iter()
        .any(|r| r[0] <= Q::zero() || r[0] >= r[1] || r[1] >= Q::one())
        || &win[0][1] + &win[1][1] >= Q::one()
    {
        return Err(fail(
            "pocket guard window must be strictly inside the common triangle",
        ));
    }
    let d = decode(domain)?;
    if d.is_empty() {
        return Err(fail("pocket domain must be a nonzero source polynomial"));
    }
    let mut geometry_factors = if factors.is_empty() {
        vec![domain.clone()]
    } else {
        let mut product_d = vec![vec![Q::one()]];
        for f in factors {
            product_d = product(&product_d, &decode(f)?, &mut work)?;
        }
        if product_d != d {
            return Err(fail(
                "pocket factor witness does not equal its complete domain polynomial",
            ));
        }
        factors.to_vec()
    };
    if d.len() == 1 && d[0].len() == 1 {
        geometry_factors.clear();
    }
    let guard_u = vec![vec![
        -&win[0][0] * &win[0][1],
        &win[0][0] + &win[0][1],
        -Q::one(),
    ]];
    let guard_v = vec![
        vec![-&win[1][0] * &win[1][1]],
        vec![&win[1][0] + &win[1][1]],
        vec![-Q::one()],
    ];
    geometry_factors.extend([encode(&guard_u), encode(&guard_v)]);
    let partition = domains::partition_cut_factors(
        &[domain.clone(), encode(&guard_u), encode(&guard_v)],
        &geometry_factors,
    )?;
    let mut selected = selection.map(|s| s.to_vec()).unwrap_or_else(|| {
        partition
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| f.signs[0] == 1 && f.signs[1] == 1 && f.signs[2] == 1)
            .map(|(i, _)| i as u32)
            .collect()
    });
    selected.sort();
    if selected.iter().any(|f| {
        partition
            .faces
            .get(*f as usize)
            .is_none_or(|f| f.signs[1] != 1 || f.signs[2] != 1)
    }) {
        return Err(fail("selected CAD cell leaves the proved pocket window"));
    }
    let (template, _, components) = super::topology::with_side_boundaries(
        &partition,
        &selected,
        orientation,
        aligned.lower_determinant.is_negative(),
    )?;
    if components.is_empty() {
        return Err(fail("pocket removal needs a nonempty selected domain"));
    }
    let a = &aligned.powers[0];
    let b = &aligned.powers[1];
    let gap = sum(
        &product(&a[2], &b[3], &mut work)?,
        &scale(&product(&b[2], &a[3], &mut work)?, &-Q::one()),
    )?;
    if !polynomial::positive(&bounds::restrict_box(&gap, &win, &mut work)?, &mut work)? {
        return Err(fail("pocket floor touches or crosses the original face"));
    }
    let face_window_winding = bounds::face_contains_window(model, source_face, &win, &mut work)?;
    let swept = bounds::swept_bounds(&aligned.powers, &win, &mut work)?;
    bounds::other_faces_avoid(model, source_face, &swept, &mut work)?;
    let faces = topology::assemble(
        model,
        source_face,
        &partition,
        &selected,
        &template,
        &components,
        aligned.lower_determinant.is_negative(),
    )?;
    let certificate = ModelPocketCertificate {
        version: 1,
        source_model: source_id(&Source::Brep(model.clone()))?,
        floor_source: source(floor)?,
        source_face,
        depth,
        domain: domain.clone(),
        factors: factors.to_vec(),
        window,
        normalized_parameter_maps: aligned
            .maps
            .each_ref()
            .map(|m| m.each_ref().map(|a| a.each_ref().map(roots::ratio))),
        partition,
        selected,
        components,
        source_atlas,
        source_nominal_volume: roots::ratio(&volume),
        face_window_winding,
        swept_bounds: swept.each_ref().map(|r| r.each_ref().map(roots::ratio)),
        faces,
    };
    Ok(AlgebraicModelPocket {
        original: model.clone(),
        floor: floor.clone(),
        powers: aligned.powers,
        certificate,
    })
}
pub fn verify_model_pocket(
    model: &Model,
    floor: &Surface,
    certificate: &ModelPocketCertificate,
) -> Result<AlgebraicModelPocket> {
    if certificate.version != 1
        || certificate.source_model != source_id(&Source::Brep(model.clone()))?
        || certificate.floor_source != source(floor)?
    {
        return Err(fail("model pocket source binding mismatch"));
    }
    let expected = certify_model_pocket(
        model,
        certificate.source_face,
        floor,
        &certificate.domain,
        &certificate.factors,
        certificate.window.clone(),
        Some(&certificate.selected),
        certificate.depth,
    )?;
    if expected.certificate != *certificate {
        return Err(fail(
            "model pocket geometry, regions or cap sewing certificate mismatch",
        ));
    }
    Ok(expected)
}
#[cfg(test)]
mod tests;

mod ordinary;
pub use ordinary::{
    certify_pocket_boolean_model, certify_pocket_boolean_model_with_arcs,
    verify_pocket_boolean_model, BoundaryArcWitness, CertifiedPocketModel, PocketModelCertificate,
};

pub use ordinary::{
    certify_framed_pocket_boolean_model, certify_model_pocket_batch,
    certify_oriented_pocket_boolean_model, verify_framed_pocket_boolean_model,
    verify_model_pocket_batch, CertifiedFramedPocketModel, CertifiedPocketModelBatch,
    FramedPocketModelCertificate, ModelPocketRequest, PocketCoordinateFrame,
    PocketModelBatchCertificate,
};

mod box_boolean;
pub use box_boolean::{CertifiedGraphBoxBoolean, GraphBoxBooleanCertificate, certify_graph_box_boolean, verify_graph_box_boolean};
pub(crate) use box_boolean::try_graph_box_boolean;
