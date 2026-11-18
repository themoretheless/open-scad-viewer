//! Actual cap face sectors. UV paths propose cuts; whole actual carrier lifts,
//! both trim sectors and all final coedges qualify them. No volume is inferred.
pub mod automatic;
mod closed;
pub mod sewing;
use super::{Sheet, require};
use crate::{Model, Result, TopoId, face_cut, face_domain};
use nurbs_core::{curve::Curve, surface_offset::contact_curve::lift};
#[derive(Clone, Copy)]
pub struct Limits {
    pub fitting: lift::FitLimits,
    pub cut_cells: usize,
    pub exact_work: u64,
    pub boundary_cells: usize,
    pub trim_pairs: usize,
    pub trim_cells: usize,
    pub trim_domain_cells: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            fitting: Default::default(),
            cut_cells: 100000,
            exact_work: 1000000,
            boundary_cells: 100000,
            trim_pairs: 100000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
        }
    }
}
pub struct Record {
    pub parent: TopoId,
    pub faces: Vec<usize>,
    pub identities: Vec<TopoId>,
    pub edges: Vec<usize>,
    pub cells: usize,
    pub exact_work: u64,
    pub fitting: Vec<lift::FitReport>,
    pub boundary: crate::boundary_agreement::Report,
    pub trim: face_domain::TrimAudit,
    face: usize,
    endpoints: [usize; 2],
    uv: Vec<Curve>,
    world: Option<Vec<Curve>>,
    closed: bool,
    automatic: Option<Box<automatic::Input>>,
    pub intersection: Option<automatic::Evidence>,
    limits: Limits,
}
impl Record {
    pub fn cutter(&self) -> Option<&Regions> {
        self.automatic.as_ref().map(|i| &i.cutter)
    }
}
pub struct Regions {
    source: Sheet,
    model: Model,
    cuts: Vec<Record>,
    provenance_nodes: usize,
    provenance_depth: usize,
}
impl Regions {
    pub fn source(&self) -> &Sheet {
        &self.source
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn cuts(&self) -> &[Record] {
        &self.cuts
    }
    /// Cuts between existing outer-loop vertices. Every UV segment retains its
    /// supplied clock/poles/weights; the actual world lift is generated and proved.
    pub fn split_uv(
        mut self,
        face: usize,
        endpoints: [usize; 2],
        uv: &[Curve],
        limits: Limits,
    ) -> Result<Self> {
        require(self.cuts.len() < 64, "At most 64 retained cap face cuts")?;
        let (model, record) = apply(&self.source, &self.model, face, endpoints, uv, limits)?;
        self.model = model;
        self.cuts.push(record);
        Ok(self)
    }
    /// Authored actual world cuts are projected into the original face chart.
    /// Projection proposes UV curves; complete saved-curve and trim proofs admit.
    pub fn split_world(
        mut self,
        face: usize,
        endpoints: [usize; 2],
        curves: &[Curve],
        limits: Limits,
    ) -> Result<Self> {
        require(self.cuts.len() < 64, "At most 64 retained cap face cuts")?;
        let (model, record) =
            apply_world(&self.source, &self.model, face, endpoints, curves, limits)?;
        self.model = model;
        self.cuts.push(record);
        Ok(self)
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let mut model = self.source.model().clone();
        let mut nodes = 1usize;
        let mut depth = 1usize;
        for c in &self.cuts {
            let (next, record) = if let Some(input) = &c.automatic {
                input.cutter.reverify()?;
                nodes += input.cutter.provenance_nodes;
                depth = depth.max(input.cutter.provenance_depth + 1);
                automatic::apply(&self.source, &model, input)?
            } else if c.closed {
                closed::apply(&self.source, &model, c.face, &c.uv, c.limits)?
            } else if let Some(curves) = &c.world {
                apply_world(&self.source, &model, c.face, c.endpoints, curves, c.limits)?
            } else {
                apply(&self.source, &model, c.face, c.endpoints, &c.uv, c.limits)?
            };
            require(
                record.parent == c.parent
                    && record.closed == c.closed
                    && record.identities == c.identities
                    && record.faces == c.faces
                    && record.edges == c.edges
                    && automatic::same(c.intersection.as_ref(), record.intersection.as_ref()),
                "Retained cap face cut provenance differs on replay",
            )?;
            model = next;
        }
        require(
            nodes == self.provenance_nodes && depth == self.provenance_depth,
            "Automatic cap provenance ownership differs on replay",
        )?;
        require(
            model == self.model,
            "Retained cap sectors differ from complete original reconstruction",
        )
    }
    /// Strict local UV sector selection. Every face remains present; deleting
    /// sectors requires replacement boundaries and separate closed-solid admission.
    pub fn select<'a>(&'a self, seeds: &[[f64; 2]], max_cells: usize) -> Result<Selection<'a>> {
        let (retained, discard, owners, cells) = classify(
            &self.model,
            self.source.cap().tolerance_uv(),
            seeds,
            max_cells,
        )?;
        Ok(Selection {
            regions: self,
            seeds: seeds.to_vec(),
            max_cells,
            retained,
            discard,
            owners,
            cells,
        })
    }
}
pub struct Selection<'a> {
    regions: &'a Regions,
    seeds: Vec<[f64; 2]>,
    max_cells: usize,
    retained: Vec<usize>,
    discard: Vec<usize>,
    owners: Vec<usize>,
    cells: usize,
}
impl Selection<'_> {
    pub fn retained_faces(&self) -> &[usize] {
        &self.retained
    }
    pub fn discard_faces(&self) -> &[usize] {
        &self.discard
    }
    pub fn seed_faces(&self) -> &[usize] {
        &self.owners
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn reverify(&self) -> Result<()> {
        self.regions.reverify()?;
        let (a, b, c, _) = classify(
            self.regions.model(),
            self.regions.source.cap().tolerance_uv(),
            &self.seeds,
            self.max_cells,
        )?;
        require(
            a == self.retained && b == self.discard && c == self.owners,
            "Original UV material-sector selection differs on replay",
        )
    }
}
/// Sealed source ownership is retained unchanged for complete reconstruction.
pub fn build(source: Sheet) -> Regions {
    let model = source.model().clone();
    Regions {
        source,
        model,
        cuts: vec![],
        provenance_nodes: 1,
        provenance_depth: 1,
    }
}
/// Literal UV of a uniquely owned existing outer vertex. No sampled projection
/// or nearest-point binding is performed. Also usable for new sector endpoints.
pub fn vertex_uv(model: &Model, face: usize, vertex: usize) -> Result<[f64; 2]> {
    model.validate()?;
    let f = model
        .faces
        .get(face)
        .ok_or_else(|| crate::Error::new("BREP_OFFSET_CAP_REFUSED", "Unknown cap sector face"))?;
    let mut uv = None;
    for c in &model.loops[f.outer].coedges {
        let e = &model.edges[c.edge];
        if e.vertices[usize::from(c.reversed)] == vertex {
            require(
                uv.is_none(),
                "Cap cut vertex has ambiguous outer-loop ownership",
            )?;
            let p = &c.pcurve.control_points[0];
            uv = Some([p[0], p[1]]);
        }
    }
    uv.ok_or_else(|| {
        crate::Error::new(
            "BREP_OFFSET_CAP_REFUSED",
            "Cap cut endpoint is not an owned outer-loop vertex",
        )
    })
}
fn validate(l: Limits) -> Result<()> {
    let f = l.fitting;
    require(
        (1..=16).contains(&f.max_degree)
            && (1..=100000).contains(&f.evaluations)
            && (1..=10000000).contains(&f.coefficient_work)
            && (1..=100000).contains(&f.proof_cells)
            && (1..=100000).contains(&f.spans)
            && (1..=1000000).contains(&l.cut_cells)
            && l.exact_work <= 1000000
            && (1..=1000000).contains(&l.boundary_cells)
            && (1..=100000).contains(&l.trim_pairs)
            && (1..=100000).contains(&l.trim_cells)
            && (1..=1000000).contains(&l.trim_domain_cells),
        "Choose bounded cap sector fitting/cut/boundary/trim allowances",
    )
}
fn clamped(c: &Curve) -> bool {
    let d = c.domain();
    !c.periodic
        && c.knots[..=c.degree].iter().all(|k| *k == d[0])
        && c.knots[c.control_points.len()..].iter().all(|k| *k == d[1])
}
fn apply(
    source: &Sheet,
    model: &Model,
    face: usize,
    endpoints: [usize; 2],
    uv: &[Curve],
    limits: Limits,
) -> Result<(Model, Record)> {
    validate(limits)?;
    require(
        (1..=64).contains(&uv.len()) && endpoints[0] != endpoints[1],
        "Choose 1..64 UV cut segments and distinct existing endpoints",
    )?;
    // Validate all definitions before entering any fitting budget.
    for c in uv {
        c.validate()?;
        require(
            c.control_points[0].len() == 2 && clamped(c) && c.weights.iter().all(|w| *w > 0.),
            "Cap cut needs positive clamped nonperiodic 2D curves",
        )?;
    }
    let ends = [
        vertex_uv(model, face, endpoints[0])?,
        vertex_uv(model, face, endpoints[1])?,
    ];
    require(
        uv[0].control_points[0].as_slice() == ends[0]
            && uv.last().unwrap().control_points.last().unwrap().as_slice() == ends[1],
        "Authored UV cut must retain literal owned endpoint coordinates",
    )?;
    for w in uv.windows(2) {
        require(
            w[0].control_points.last() == Some(&w[1].control_points[0]),
            "Authored UV cut segments need literal common endpoints",
        )?;
    }
    let carrier = &model.faces[face].surface;
    require(
        carrier == source.cap().surface(),
        "Cap sector carrier differs from sealed original actual surface",
    )?;
    let mut vertices = vec![model.vertices[endpoints[0]].point];
    for c in uv.iter().skip(1) {
        let p = &c.control_points[0];
        let q = carrier.evaluate(p[0], p[1])?.point;
        vertices.push([q[0], q[1], q[2]]);
    }
    vertices.push(model.vertices[endpoints[1]].point);
    let mut fitting = vec![];
    let mut path = vec![];
    for (i, c) in uv.iter().enumerate() {
        // Zero offset fits the ACTUAL carrier; no ideal-source approximation receipt
        // stands in for cut agreement on this surface.
        let fit = lift::fit(
            c,
            carrier,
            0.,
            model.tolerance_mm,
            Some([vertices[i], vertices[i + 1]]),
            limits.fitting,
        )?;
        let curve = fit
            .curve
            .as_ref()
            .ok_or_else(|| {
                crate::Error::new(
                    "BREP_OFFSET_CAP_REFUSED",
                    "Actual cap cut lift remains unproved",
                )
            })?
            .clone();
        path.push(face_cut::Segment {
            curve,
            pcurve: c.clone(),
        });
        fitting.push(fit);
    }
    let result = face_cut::partition_with_budget(
        model,
        face,
        endpoints,
        &path,
        source.cap().tolerance_uv(),
        limits.cut_cells,
        limits.exact_work,
    )?;
    finish(
        source,
        face,
        endpoints,
        CapCurves {
            uv: uv.to_vec(),
            world: None,
            fitting,
        },
        limits,
        result,
    )
}
fn apply_world(
    source: &Sheet,
    model: &Model,
    face: usize,
    endpoints: [usize; 2],
    curves: &[Curve],
    limits: Limits,
) -> Result<(Model, Record)> {
    validate(limits)?;
    require(
        (1..=64).contains(&curves.len()),
        "Choose 1..64 actual world cut segments",
    )?;
    for c in curves {
        c.validate()?;
        require(
            c.control_points[0].len() == 3 && clamped(c) && c.weights.iter().all(|w| *w > 0.),
            "Actual cap world cuts need positive clamped nonperiodic 3D curves",
        )?;
    }
    for vertex in endpoints {
        vertex_uv(model, face, vertex)?;
    }
    require(
        model.faces[face].surface == *source.cap().surface(),
        "Cap sector carrier differs from sealed actual surface",
    )?;
    let result = face_cut::projected_with_budget(
        model,
        face,
        endpoints,
        curves,
        source.cap().tolerance_uv(),
        limits.cut_cells.min(100000),
        limits.exact_work,
    )?;
    finish(
        source,
        face,
        endpoints,
        CapCurves {
            uv: vec![],
            world: Some(curves.to_vec()),
            fitting: vec![],
        },
        limits,
        result,
    )
}
fn finish(
    source: &Sheet,
    face: usize,
    endpoints: [usize; 2],
    curves: CapCurves,
    limits: Limits,
    result: face_cut::Partition,
) -> Result<(Model, Record)> {
    finish_model(
        source,
        face,
        endpoints,
        curves,
        limits,
        CutPartition {
            model: result.model,
            parent: result.source_identity,
            faces: result.faces.to_vec(),
            identities: result.face_identities.to_vec(),
            edges: result.edges,
            cells: result.cells,
            exact_work: result.exact_work,
        },
    )
}
/// Cap boundary curves in UV and, when lifted, in world space, with their fitting reports.
struct CapCurves {
    uv: Vec<Curve>,
    world: Option<Vec<Curve>>,
    fitting: Vec<lift::FitReport>,
}
/// A cut cap face: the resulting model, its source identity, the new faces with identities,
/// the cut edges and the work spent.
struct CutPartition {
    model: Model,
    parent: TopoId,
    faces: Vec<usize>,
    identities: Vec<TopoId>,
    edges: Vec<usize>,
    cells: usize,
    exact_work: u64,
}
fn finish_model(
    source: &Sheet,
    face: usize,
    endpoints: [usize; 2],
    curves: CapCurves,
    limits: Limits,
    partition: CutPartition,
) -> Result<(Model, Record)> {
    let CapCurves { uv, world, fitting } = curves;
    let CutPartition { model, parent, faces, identities, edges, cells, exact_work } = partition;
    let boundary = crate::boundary_agreement::verify(&model, limits.boundary_cells)?;
    require(
        boundary.complete,
        "Actual retained cap sector boundary lifts remain unproved",
    )?;
    let trim = face_domain::audit_trim_regions(
        &model,
        source.cap().tolerance_uv(),
        limits.trim_pairs,
        limits.trim_cells,
        limits.trim_domain_cells,
    )?;
    require(
        trim.all_valid,
        "Actual retained cap sector trim regions remain unproved",
    )?;
    require(
        model
            .faces
            .iter()
            .all(|f| f.surface == *source.cap().surface()),
        "Cap sector construction altered its original actual carrier",
    )?;
    let record = Record {
        parent,
        faces,
        identities,
        edges,
        cells,
        exact_work,
        fitting,
        boundary,
        trim,
        face,
        endpoints,
        uv,
        world,
        closed: false,
        automatic: None,
        intersection: None,
        limits,
    };
    Ok((model, record))
}
fn classify(
    model: &Model,
    tol: f64,
    seeds: &[[f64; 2]],
    max: usize,
) -> Result<(Vec<usize>, Vec<usize>, Vec<usize>, usize)> {
    require(
        (1..=256).contains(&seeds.len())
            && seeds.iter().flatten().all(|x| x.is_finite())
            && (1..=1000000).contains(&max),
        "Choose finite UV sector witnesses and bounded winding work",
    )?;
    let domains = (0..model.faces.len())
        .map(|f| face_domain::FaceDomain::new(model, f, tol))
        .collect::<Result<Vec<_>>>()?;
    let mut retained = std::collections::BTreeSet::new();
    let mut owners = vec![];
    let mut cells = 0;
    for p in seeds {
        let mut owner = None;
        for (f, d) in domains.iter().enumerate() {
            require(
                cells < max,
                "Shared cap material-sector winding allowance exhausted",
            )?;
            let r = d.classify(p.map(|x| [x, x]), (max - cells).min(100000))?;
            cells += r.cells;
            match r.location {
                nurbs_core::trim_domain::Location::Inside if r.winding == Some(1) => {
                    require(
                        owner.replace(f).is_none(),
                        "UV material witness lies in overlapping cap sectors",
                    )?;
                }
                nurbs_core::trim_domain::Location::Outside => {}
                _ => require(
                    false,
                    "UV material witness lies on a boundary or remains unresolved",
                )?,
            }
        }
        let face = owner.ok_or_else(|| {
            crate::Error::new(
                "BREP_OFFSET_CAP_REFUSED",
                "UV material witness lies outside all retained cap sectors",
            )
        })?;
        retained.insert(face);
        owners.push(face);
    }
    let discard = (0..model.faces.len())
        .filter(|f| !retained.contains(f))
        .collect();
    Ok((retained.into_iter().collect(), discard, owners, cells))
}
