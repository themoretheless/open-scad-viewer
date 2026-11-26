//! Actual original rectangular stock, independent of surface UV axis order.
//! Six exact support planes plus fresh closed embedding/volume admission.
use crate::{Error, Model, Result, source_body_restore};
use std::collections::BTreeSet;
use nurbs_core::ss_intersection as ss;
use crate::root_model::{QualifiedRootShellPositiveFlux, RootPeriodicBandLimits, unproved};
pub struct Limits {
    pub operations: usize,
    pub body: source_body_restore::Limits,
}
pub struct QualifiedStockBox<'a> {
    source: &'a Model,
    limits: &'a Limits,
    bounds: [[f64; 2]; 3],
    planes: [usize; 6],
    body: crate::source_volume::Body,
    operations: usize,
}
impl QualifiedStockBox<'_> {
    pub fn source(&self) -> &Model {
        self.source
    }
    pub fn bounds(&self) -> [[f64; 2]; 3] {
        self.bounds
    }
    /// xmin,xmax,ymin,ymax,zmin,zmax original face identities.
    pub fn planes(&self) -> [usize; 6] {
        self.planes
    }
    pub fn body(&self) -> &crate::source_volume::Body {
        &self.body
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify(self.source, self.limits)?;
        require(
            fresh.bounds == self.bounds
                && fresh.planes == self.planes
                && fresh.operations == self.operations,
            "Original stock box ownership changed",
        )
    }
}
fn error(s: &str) -> Error {
    Error::new("BREP_SOURCE_BOX_UNPROVEN", s)
}
fn require(ok: bool, s: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(error(s)) }
}
struct Work {
    used: usize,
    max: usize,
}
impl Work {
    fn charge(&mut self, n: usize) -> Result<()> {
        require(
            n <= self.max - self.used,
            "Stock box structural work exhausted",
        )?;
        self.used += n;
        Ok(())
    }
}
pub fn qualify<'a>(source: &'a Model, limits: &'a Limits) -> Result<QualifiedStockBox<'a>> {
    require(
        (1..=1000000).contains(&limits.operations),
        "Bound original stock work",
    )?;
    let mut work = Work {
        used: 0,
        max: limits.operations,
    };
    source.validate()?;
    require(
        source.faces.len() == 6
            && source.loops.len() == 6
            && source.edges.len() == 12
            && source.vertices.len() == 8
            && source.shells.len() == 1
            && source.bodies.len() == 1
            && source.bodies[0].outer_shell == 0
            && source.bodies[0].inner_shells.is_empty(),
        "Need one six-face original box without cavities",
    )?;
    require(
        source.shells[0].faces.len() == 6
            && source.shells[0]
                .faces
                .iter()
                .map(|f| f.face)
                .collect::<BTreeSet<_>>()
                == (0..6).collect(),
        "Actual stock shell must own every original face once",
    )?;
    require(
        source
            .edges
            .iter()
            .flat_map(|e| e.vertices)
            .collect::<BTreeSet<_>>()
            .len()
            == 8,
        "Stock has an unused or missing actual corner",
    )?;
    require(
        source.faces.iter().all(|f| f.holes.is_empty())
            && source
                .faces
                .iter()
                .map(|f| f.outer)
                .collect::<BTreeSet<_>>()
                .len()
                == 6
            && source
                .loops
                .iter()
                .all(|l| l.coedges.len() == 4 && l.coedges.iter().all(|c| c.trim().is_none())),
        "Need six complete original natural rectangles",
    )?;
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for v in &source.vertices {
        work.charge(3)?;
        for k in 0..3 {
            bounds[k][0] = bounds[k][0].min(v.point[k]);
            bounds[k][1] = bounds[k][1].max(v.point[k]);
        }
    }
    require(
        bounds.iter().all(|b| b[0] < b[1]),
        "Original stock extent is degenerate",
    )?;
    let mut corners = BTreeSet::new();
    for v in &source.vertices {
        let mut id = 0;
        for k in 0..3 {
            work.charge(1)?;
            if v.point[k] == bounds[k][1] {
                id |= 1 << k;
            } else {
                require(
                    v.point[k] == bounds[k][0],
                    "Vertex is not an exact original box corner",
                )?;
            }
        }
        require(corners.insert(id), "Repeated original stock corner")?;
    }
    for e in &source.edges {
        work.charge(1)?;
        e.curve.validate()?;
        let c = &e.curve;
        let n = c.control_points.len();
        let d = c.domain();
        require(
            !e.degenerate
                && !c.periodic
                && c.knots[..=c.degree].iter().all(|&t| t == d[0])
                && c.knots[n..].iter().all(|&t| t == d[1]),
            "Need complete clamped original stock edges",
        )?;
        require(
            c.control_points[0].as_slice() == source.vertices[e.vertices[0]].point.as_slice()
                && c.control_points[n - 1].as_slice()
                    == source.vertices[e.vertices[1]].point.as_slice(),
            "Stock vertex differs from literal original edge endpoint",
        )?;
    }
    let mut planes = [usize::MAX; 6];
    for (f, face) in source.faces.iter().enumerate() {
        face.surface.validate()?;
        let mut matches = Vec::new();
        for side in 0..6 {
            let mut yes = true;
            for p in face.surface.control_points.iter().flatten() {
                work.charge(1)?;
                yes &= p[side / 2] == bounds[side / 2][side % 2];
            }
            if yes {
                matches.push(side);
            }
        }
        require(
            matches.len() == 1,
            "Support is not one exact original box plane",
        )?;
        let side = matches[0];
        require(planes[side] == usize::MAX, "Repeated stock support plane")?;
        planes[side] = f;
    }
    require(
        planes.iter().all(|&f| f != usize::MAX),
        "Missing original stock plane",
    )?;
    for f in &source.faces {
        for c in &source.loops[f.outer].coedges {
            require(
                work.used < work.max,
                "No stock boundary identity work remains",
            )?;
            let identity = nurbs_core::curve_surface_agreement::verify_exact(
                &source.edges[c.edge].curve,
                &c.pcurve,
                &f.surface,
                c.reversed,
                (work.max - work.used).min(1000000) as u64,
            )?
            .ok_or_else(|| error("Original stock boundary identity is unproven"))?;
            work.charge(identity.work_used as usize)?;
            require(
                identity.outcome == cad_predicates::BezierIdentity::Equal,
                "Original stock boundary does not exactly meet its support",
            )?;
        }
    }
    let incidence = crate::source_support_shell::prepare(
        source,
        crate::source_support_shell::Limits {
            tolerance_uv: limits.body.embedding.tolerance_uv,
            faces: 6,
            uses: 24,
            regions: limits.body.shell.regions.region,
            exact_work: limits.body.shell.exact_work,
        },
    )?;
    let shell = incidence.shell.ok_or_else(|| error(incidence.reason))?;
    require(
        shell.regions().is_some_and(|r| {
            r.iter()
                .all(|r| r.whole_chart_material() && r.chart_winding() == 1)
        }),
        "Stock requires complete original rectangular charts",
    )?;
    let l = &limits.body.embedding;
    let geometry = crate::source_shell_geometry::qualify(
        shell,
        crate::source_shell_geometry::Limits {
            tolerance_uv: l.tolerance_uv,
            corners: l.corners,
            spans: l.spans,
            linear_cells: l.linear_cells,
            pairs: l.pairs,
            exact_work: l.exact_work,
            driver_cells: l.driver_cells,
        },
    )?;
    let geometry = geometry.geometry.ok_or_else(|| error(geometry.reason))?;
    let l = &limits.body.volume;
    let volume = crate::source_volume::qualify(
        geometry,
        crate::source_volume::Limits {
            axis: l.axis,
            origin: l.origin,
            absolute_error: l.absolute_error,
            tolerance_uv: l.tolerance_uv,
            cells: l.cells,
            spans: l.spans,
            domain_cells: l.domain_cells,
        },
    )?;
    let body = volume.body.ok_or_else(|| error(volume.reason))?;
    require(
        !body.reverse_orientation(),
        "Actual original stock face senses are not outward",
    )?;
    Ok(QualifiedStockBox {
        source,
        limits,
        bounds,
        planes,
        body,
        operations: work.used,
    })
}

#[cfg(test)]
mod tests;

impl QualifiedRootShellPositiveFlux {
    /// Intersect a complete periodic wall with a stock box whose two axial
    /// faces cut it strictly inside its original caps. The wall control hull
    /// must lie strictly between the other four stock faces.
    pub fn intersect_periodic_stock_band(
        source: &crate::closed_solid::QualifiedClosedSolid,
        stock: &QualifiedStockBox<'_>,
        section_limits: ss::PeriodicSSBuildLimits,
        mut limits: RootPeriodicBandLimits,
    ) -> Result<Self> {
        let model = source.model();
        if model.faces.len() != 3
            || model.shells.len() != 1
            || model.bodies.len() != 1
            || model.faces.iter().any(|face| !face.holes.is_empty())
        {
            return Err(unproved(
                "Periodic stock intersection needs a full wall and two caps",
            ));
        }
        let mut walls = model
            .faces
            .iter()
            .enumerate()
            .filter(|(_, face)| face.surface.periodic_u || face.surface.periodic_v);
        let (face, wall) = walls
            .next()
            .filter(|_| walls.next().is_none())
            .ok_or_else(|| unproved("Periodic stock intersection needs one periodic wall"))?;
        let wall = &wall.surface;
        let axis = match (wall.periodic_u, wall.periodic_v) {
            (true, false) => 0,
            (false, true) => 1,
            _ => {
                return Err(unproved(
                    "Stock intersection needs exactly one periodic axis",
                ));
            }
        };
        let natural = crate::boolean_robustness::surface_uv_domains(wall);
        if crate::face_domain::FaceDomain::new(model, face, 1e-8)?.literal_rectangle()
            != Some(natural)
        {
            return Err(unproved(
                "The original wall does not cover its complete periodic rectangle",
            ));
        }
        let carrier = nurbs_core::surface_injectivity::certify_periodic_extrusion(
            wall,
            axis,
            section_limits.max_pairs,
            section_limits.max_cells,
        )?;
        let axial = carrier
            .axial_coordinate
            .filter(|_| carrier.proven && carrier.regular_interior)
            .ok_or_else(|| unproved("The periodic wall lacks a proved axial section family"))?;
        let pole = |row: usize| {
            if axis == 0 {
                &wall.control_points[0][row]
            } else {
                &wall.control_points[row][0]
            }
        };
        let mut ends = [pole(0)[axial], pole(1)[axial]];
        ends.sort_by(f64::total_cmp);
        let [low, high] = stock.bounds()[axial];
        if !(ends[0] < low && high < ends[1]) {
            return Err(unproved(
                "Both axial stock faces must cut strictly inside the original wall",
            ));
        }
        let other_faces = model
            .faces
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != face)
            .map(|(_, cap)| &cap.surface)
            .collect::<Vec<_>>();
        if !ends.into_iter().all(|end| {
            other_faces
                .iter()
                .any(|cap| cap.control_points.iter().flatten().all(|p| p[axial] == end))
        }) {
            return Err(unproved(
                "Both original caps must be on the wall's axial ends",
            ));
        }
        if !wall.control_points.iter().flatten().all(|p| {
            (0..3)
                .all(|k| k == axial || (stock.bounds()[k][0] < p[k] && p[k] < stock.bounds()[k][1]))
        }) {
            return Err(unproved(
                "A lateral stock face may intersect the periodic wall",
            ));
        }
        let section = |end: usize| {
            let plane = &stock.source().faces[stock.planes()[2 * axial + end]].surface;
            let pair = ss::FiberSources::new(wall, plane)?;
            ss::qualify_periodic_plane_ss(&pair, section_limits)
        };
        let lower = section(0)?;
        let upper = section(1)?;
        let tolerance = model.tolerance_mm.max(stock.source().tolerance_mm);
        limits.flux.axis = axial;
        limits.flux.origin = low.midpoint(high);
        Self::qualify_periodic_band([&lower, &upper], tolerance, limits)
    }
}
