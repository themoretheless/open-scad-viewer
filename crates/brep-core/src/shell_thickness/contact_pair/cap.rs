//! Open faces on a qualified actual offset carrier. No closed body is inferred.
pub mod partition;
pub mod regions;
pub mod sewing;
use crate::{Coedge, Edge, Error, Face, FaceUse, Loop, Model, Result, Shell, Vertex};
use nurbs_core::{
    curve_partition_agreement::mapped,
    surface_offset::contact_curve::chain::region::compound::cap::Cap, trim_region_audit::nesting,
};
#[derive(Clone, Copy)]
pub struct Limits {
    pub orientation_cells: usize,
    pub boundary_cells: usize,
    pub nesting: nesting::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            orientation_cells: 100000,
            boundary_cells: 100000,
            nesting: Default::default(),
        }
    }
}
fn require(ok: bool, msg: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new("BREP_OFFSET_CAP_REFUSED", msg))
    }
}
pub struct Sheet {
    cap: Cap,
    model: Model,
    limits: Limits,
    orientation_cells: usize,
    boundary: crate::boundary_agreement::Report,
    partitions: Vec<partition::Record>,
    edge_roots: Vec<usize>,
}
impl Sheet {
    pub fn cap(&self) -> &Cap {
        &self.cap
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn orientation_cells(&self) -> usize {
        self.orientation_cells
    }
    pub fn boundary_proof(&self) -> &crate::boundary_agreement::Report {
        &self.boundary
    }
    pub fn reverify(&self) -> Result<()> {
        self.cap.reverify()?;
        let (mut m, _, _) = assemble(&self.cap, self.limits)?;
        let mut roots = (0..m.edges.len()).collect::<Vec<_>>();
        for record in &self.partitions {
            partition::replay_search(&m, record)?;
            let (next, _, _) = partition::apply(
                &self.cap,
                &m,
                &mut roots,
                record.edge,
                record.parameter,
                record.vertex,
                record.limits,
            )?;
            m = next;
        }
        require(
            roots == self.edge_roots,
            "Partition original-edge ownership differs",
        )?;
        require(
            m == self.model,
            "Actual cap sheet differs from original reconstruction",
        )
    }
}
fn assemble(cap: &Cap, l: Limits) -> Result<(Model, usize, crate::boundary_agreement::Report)> {
    require(
        (1..=100000).contains(&l.orientation_cells) && (1..=1000000).contains(&l.boundary_cells),
        "Choose bounded sheet work allowances",
    )?;
    let domain = cap.region().uv_domain();
    let mut m = Model::empty(cap.boundary_tolerance())?;
    let mut cells = 0;
    let mut oriented = vec![];
    for (index, (uv, world)) in domain
        .loops()
        .iter()
        .zip(cap.region().world_loops())
        .enumerate()
    {
        let base = m.vertices.len();
        let edges = m.edges.len();
        for (i, c) in world.iter().enumerate() {
            let p = &c.control_points[0];
            m.vertices.push(Vertex {
                point: [p[0], p[1], p[2]],
            });
            m.edges.push(Edge {
                degenerate: false,
                vertices: [base + i, base + (i + 1) % world.len()],
                curve: c.clone(),
            });
        }
        let mut coedges = vec![];
        let mut curves = vec![];
        for (i, reverse) in domain.coedges(index)? {
            let raw = &uv[i];
            let curve = if reverse { raw.reverse()? } else { raw.clone() };
            require(
                cells < l.orientation_cells,
                "Shared orientation budget exhausted",
            )?;
            let d = raw.domain();
            let proof = mapped::verify(
                raw,
                &curve,
                if reverse { [d[1], d[0]] } else { d },
                curve.domain(),
                cap.tolerance_uv(),
                l.orientation_cells - cells,
            )?;
            cells += proof.cells;
            require(
                proof.accepted,
                "Actual oriented pcurve does not preserve original UV correspondence",
            )?;
            curves.push(curve.clone());
            coedges.push(Coedge::new(edges + i, reverse, curve));
        }
        oriented.push(curves);
        m.loops.push(Loop { coedges });
    }
    let actual = nesting::build(&oriented, cap.tolerance_uv(), l.nesting)?;
    require(
        actual
            .nodes()
            .iter()
            .zip(domain.nodes())
            .all(|(a, b)| a.parent == b.parent && a.depth == b.depth && !a.reversed),
        "Actual oriented UV loops changed material nesting",
    )?;
    for c in actual.components() {
        m.faces.push(Face {
            surface: cap.surface().clone(),
            outer: c.outer,
            holes: c.holes.clone(),
        });
    }
    // UV islands are disconnected face components and own separate open shells.
    for face in 0..m.faces.len() {
        m.shells.push(Shell {
            faces: vec![FaceUse {
                face,
                reversed: false,
            }],
            closed: false,
        });
    }
    m.rebuild_topology_ids();
    m.validate()?;
    let boundary = crate::boundary_agreement::verify(&m, l.boundary_cells)?;
    require(
        boundary.complete,
        "Actual oriented cap boundary agreement remains unproved",
    )?;
    Ok((m, cells, boundary))
}
/// Creates one open BRep face per even-depth UV component, with its direct holes.
/// Raw rim curves stay owned by Cap; reversed pcurves receive fresh mapped-clock
/// proofs, a fresh complete nesting forest and continuous actual-carrier lifts.
pub fn build(cap: Cap, limits: Limits) -> Result<Sheet> {
    let (model, orientation_cells, boundary) = assemble(&cap, limits)?;
    let edge_roots = (0..model.edges.len()).collect();
    Ok(Sheet {
        cap,
        model,
        limits,
        orientation_cells,
        boundary,
        partitions: vec![],
        edge_roots,
    })
}
