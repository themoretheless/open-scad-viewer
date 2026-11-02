use crate::{
    MAX_COEDGES, MAX_ENTITIES, MAX_FACES, Model, Report, TopoKind, boundary_agreement, curve_point,
    distance, invalid, require,
};
use nurbs_core::{Error, Result, curve::Curve, surface::{Surface, SurfaceSampler}};
use std::collections::{BTreeMap, BTreeSet};

// Preserve independent authored UV clocks during legacy sampled checks too.
// Geometric admission still requires the continuous/exact boundary owners.
fn coedge_uv(c: &crate::Coedge, world: &Curve, fraction: f64) -> Result<Vec<f64>> {
    if let Some(trim) = c.trim().filter(|t| t.curve_parameter == world.domain()) {
        // Subrange world trims also serve as diagnostic-only proof windows;
        // they do not replace the topology's complete loop (see BoundaryUse).
        trim.validate()?;
        let d = c.pcurve.domain();
        require(trim.pcurve_parameter.iter().all(|t| *t >= d[0] && *t <= d[1]),
            "Coedge UV trim lies outside its pcurve domain")?;
        let t = trim.pcurve_parameter[0] * (1. - fraction)
            + trim.pcurve_parameter[1] * fraction;
        c.pcurve.evaluate(t).map(|v| v.point)
    } else { curve_point(&c.pcurve, fraction) }
}

impl Model {
    pub fn loop_uv(&self, id: usize, segments: usize) -> Result<Vec<[f64; 2]>> {
        require((1..=64).contains(&segments), "Edge sampling must be 1..64")?;
        let wire = self.loops.get(id).ok_or_else(|| invalid("Unknown loop"))?;
        let mut uv = Vec::new();
        for c in &wire.coedges {
            for i in 0..segments {
                let p = coedge_uv(c, &self.edges[c.edge].curve, i as f64 / segments as f64)?;
                require(p.len() == 2, "pcurve must be 2D")?;
                uv.push([p[0], p[1]]);
            }
        }
        Ok(uv)
    }
    pub fn validate(&self) -> Result<Report> {
        self.validate_impl(true)
    }

    /// Validate every edge/pcurve lift over its complete authored interval.
    /// Unlike sampled validation, exhaustion is inconclusive rather than a
    /// successful admission. The model and its legacy report status are unchanged.
    pub fn validate_continuous(&self, max_cells: usize) -> Result<Report> {
        let mut report = self.validate_boundary_diagnostic_inputs()?;
        let agreement = boundary_agreement::verify(self, max_cells)?;
        for entry in &agreement.uses {
            match entry.status {
                nurbs_core::curve_surface_agreement::Status::WithinTolerance => {}
                nurbs_core::curve_surface_agreement::Status::Mismatch => {
                    return Err(Error::new(
                        "BREP_GEOMETRY_MISMATCH",
                        format!(
                            "Continuous edge/lift mismatch at face {}, coedge {}",
                            entry.face, entry.coedge
                        ),
                    ));
                }
                nurbs_core::curve_surface_agreement::Status::Unresolved => {
                    return Err(Error::new(
                        "BREP_GEOMETRY_UNRESOLVED",
                        format!(
                            "Continuous edge/lift agreement unresolved at face {}, coedge {}",
                            entry.face, entry.coedge
                        ),
                    ));
                }
            }
        }
        report.geometry_agreement = "continuous_with_tolerance";
        Ok(report)
    }

    /// Diagnostic-only admission. All topology, IDs, definitions, endpoint and
    /// UV-wire checks remain active; edge/lift agreement is checked separately
    /// over complete intervals. This does not admit the model to editing.
    pub(crate) fn validate_boundary_diagnostic_inputs(&self) -> Result<Report> {
        self.validate_impl(false)
    }

    fn validate_impl(&self, sampled_agreement: bool) -> Result<Report> {
        self.0.validate_topology().map_err(|e| Error {
            code: e.code,
            message: e.message,
        })?;
        let count = self.vertices.len()
            + self.edges.len()
            + self.loops.len()
            + self.faces.len()
            + self.shells.len()
            + self.bodies.len();
        let uses = self.loops.iter().map(|l| l.coedges.len()).sum::<usize>();
        // Gear bodies carry a few hundred faces (six per tooth, twice for a
        // herringbone); the ceiling stays a hard guard against runaway
        // authoring, not a modelling limit.
        if count > MAX_ENTITIES || uses > MAX_COEDGES || self.faces.len() > MAX_FACES {
            return Err(Error {
                code: "BREP_RESOURCE_LIMIT",
                message: format!(
                    "B-rep exceeds {MAX_ENTITIES} entities, {MAX_FACES} faces or {MAX_COEDGES} coedges"
                ),
            });
        }
        require(
            self.tolerance_mm.is_finite() && (1e-10..=1e-2).contains(&self.tolerance_mm),
            "Invalid B-rep tolerance (1e-10..1e-2 mm)",
        )?;
        let tol = self.tolerance_mm;
        require(
            self.1.vertices.len() == self.vertices.len()
                && self.1.edges.len() == self.edges.len()
                && self.1.loops.len() == self.loops.len()
                && self.1.faces.len() == self.faces.len()
                && self.1.shells.len() == self.shells.len()
                && self.1.bodies.len() == self.bodies.len(),
            "Topology ID table does not match entity counts",
        )?;
        let all_ids = self
            .1
            .vertices
            .iter()
            .chain(&self.1.edges)
            .chain(&self.1.loops)
            .chain(&self.1.faces)
            .chain(&self.1.shells)
            .chain(&self.1.bodies);
        let ids: BTreeSet<_> = all_ids.clone().collect();
        require(ids.len() == all_ids.count(), "Topology IDs must be unique")?;
        for (kind, ids) in self.identity_groups() {
            require(
                ids.iter().all(|id| id.kind() == kind),
                "Topology ID prefix does not match its entity kind",
            )?;
            require(
                ids.iter()
                    .all(|id| self.1.change_set.nodes.get(id) == Some(&kind)),
                "Authoritative change set is missing a topology entity",
            )?;
        }
        self.1.change_set.validate().map_err(|error| Error {
            code: error.code,
            message: error.message,
        })?;
        for (id, tolerance) in &self.1.local_tolerances_mm {
            require(
                matches!(id.kind(), TopoKind::Vertex | TopoKind::Edge) && ids.contains(id),
                "Local tolerance refers to an unknown or unsupported entity",
            )?;
            require(
                tolerance.is_finite() && (1e-10..=1e-2).contains(tolerance),
                "Invalid local edge/vertex tolerance",
            )?;
        }
        let mut vertex_used = vec![false; self.vertices.len()];
        for v in &self.vertices {
            require(
                v.point.iter().all(|x| x.is_finite() && x.abs() <= 1e6),
                "Invalid vertex coordinates",
            )?;
        }
        for e in &self.edges {
            e.curve.validate()?;
            require(
                e.curve.control_points[0].len() == 3,
                "Edge curve must be 3D",
            )?;
            if e.degenerate {
                let pole = self.vertices[e.vertices[0]].point;
                require(
                    e.curve
                        .control_points
                        .iter()
                        .all(|p| p.as_slice() == pole.as_slice()),
                    "Collapsed edge curve must be identically its pole vertex",
                )?;
            } else {
                require(
                    e.curve
                        .control_points
                        .iter()
                        .any(|p| *p != e.curve.control_points[0]),
                    "Constant edge requires an explicit collapsed-boundary marker",
                )?;
            }
            for (i, &v) in e.vertices.iter().enumerate() {
                let point = &self
                    .vertices
                    .get(v)
                    .ok_or_else(|| invalid("Edge references unknown vertex"))?
                    .point;
                vertex_used[v] = true;
                require(
                    distance(&curve_point(&e.curve, i as f64)?, point) <= tol,
                    "Edge endpoint does not match vertex",
                )?;
            }
        }
        require(vertex_used.iter().all(|v| *v), "Unused vertex")?;
        let mut loop_owner = vec![None; self.loops.len()];
        let mut edge_used = vec![false; self.edges.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            // Sampling the same immutable face must not revalidate its complete
            // control net for every coedge parameter.
            let surface = SurfaceSampler::new(&f.surface)?;
            for (li, &l) in std::iter::once(&f.outer).chain(&f.holes).enumerate() {
                let wire = self
                    .loops
                    .get(l)
                    .ok_or_else(|| invalid("Face references unknown loop"))?;
                require(
                    loop_owner[l].replace(fi).is_none(),
                    "Loop belongs to multiple faces or is repeated",
                )?;
                require(!wire.coedges.is_empty(), "Empty face loop")?;
                let mut previous: Option<usize> = None;
                let mut first = None;
                let mut previous_uv: Option<Vec<f64>> = None;
                let mut first_uv = None;
                for c in &wire.coedges {
                    let edge = self
                        .edges
                        .get(c.edge)
                        .ok_or_else(|| invalid("Coedge references unknown edge"))?;
                    edge_used[c.edge] = true;
                    c.pcurve.validate()?;
                    require(c.pcurve.control_points[0].len() == 2, "pcurve must be 2D")?;
                    if edge.degenerate && sampled_agreement {
                        validate_pole_boundary(
                            &f.surface,
                            &c.pcurve,
                            self.vertices[edge.vertices[0]].point,
                        )?;
                    }
                    let start = edge.vertices[usize::from(c.reversed)];
                    let end = edge.vertices[usize::from(!c.reversed)];
                    if let Some(p) = previous {
                        require(p == start, "Loop vertex chain is disconnected")?;
                    } else {
                        first = Some(start);
                    }
                    previous = Some(end);
                    let uv0 = coedge_uv(c, &edge.curve, 0.)?;
                    let uv1 = coedge_uv(c, &edge.curve, 1.)?;
                    if let Some(p) = &previous_uv {
                        require(distance(p, &uv0) <= 1e-9, "Loop UV chain is disconnected")?;
                    } else {
                        first_uv = Some(uv0);
                    }
                    previous_uv = Some(uv1);
                    // Diagnostic callers replace these samples with full-interval
                    // verification and retain per-use mismatch/unknown results.
                    if sampled_agreement {
                        for i in 0..=8 {
                            let t = i as f64 / 8.;
                            let uv = coedge_uv(c, &edge.curve, t)?;
                            let uv = if f.surface.periodic_u || f.surface.periodic_v {
                                nurbs_core::periodic_trim::wrapped_surface_parameters(&f.surface, [uv[0], uv[1]])?
                            } else { [uv[0], uv[1]] };
                            let p = surface.evaluate(uv[0], uv[1])?.point;
                            let q = curve_point(&edge.curve, if c.reversed { 1. - t } else { t })?;
                            require(
                                distance(&p, &q) <= tol,
                                "pcurve/surface and 3D edge disagree at sampled parameters",
                            )?;
                        }
                    }
                }
                require(previous == first, "Loop is not closed")?;
                require(
                    distance(previous_uv.as_ref().unwrap(), first_uv.as_ref().unwrap()) <= 1e-9,
                    "UV loop is not closed",
                )?;
                let uv = self.loop_uv(l, 8)?;
                let area = (0..uv.len())
                    .map(|i| {
                        let a = uv[i];
                        let b = uv[(i + 1) % uv.len()];
                        (a[0] - uv[0][0]) * (b[1] - uv[0][1])
                            - (b[0] - uv[0][0]) * (a[1] - uv[0][1])
                    })
                    .sum::<f64>();
                require(
                    if li == 0 { area > 0. } else { area < 0. },
                    "Outer UV loop must be CCW and hole loops CW",
                )?;
            }
        }
        require(loop_owner.iter().all(Option::is_some), "Unused loop")?;
        require(edge_used.iter().all(|v| *v), "Unused edge")?;
        let boundary = self
            .shells
            .iter()
            .map(|s| {
                let mut uses = BTreeMap::<usize, usize>::new();
                for u in &s.faces {
                    let f = &self.faces[u.face];
                    for &l in std::iter::once(&f.outer).chain(&f.holes) {
                        for c in &self.loops[l].coedges {
                            if !self.edges[c.edge].degenerate {
                                *uses.entry(c.edge).or_default() += 1;
                            }
                        }
                    }
                }
                uses.values().filter(|n| **n == 1).count()
            })
            .sum();
        Ok(Report {
            vertex_count: self.vertices.len(),
            edge_count: self.edges.len(),
            loop_count: self.loops.len(),
            face_count: self.faces.len(),
            shell_count: self.shells.len(),
            body_count: self.bodies.len(),
            boundary_edge_count: boundary,
            topology_valid: true,
            geometry_agreement: if sampled_agreement {
                "sampled_with_tolerance"
            } else {
                "not_checked"
            },
            solid_geometry_status: "not_certified",
        })
    }
}

mod pole_boundary;
pub(crate) use pole_boundary::validate_pole_boundary;
