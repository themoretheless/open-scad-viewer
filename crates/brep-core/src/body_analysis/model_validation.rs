use crate::{
    MAX_COEDGES, MAX_ENTITIES, MAX_FACES, Model, Report, TopoKind, boundary_agreement, invalid,
    require,
};
use nurbs_core::{
    Error, Result,
    curve::{Curve, CurveSampler},
    surface::{Surface, SurfaceRefSampler},
};
use std::collections::HashSet;

#[inline]
fn distance2(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[inline]
fn distance3(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

// Preserve independent authored UV clocks during legacy sampled checks too.
// Geometric admission still requires the continuous/exact boundary owners.
fn resolve_coedge_uv_window(
    c: &crate::Coedge,
    pcurve: &CurveSampler<'_>,
    world_domain: [f64; 2],
) -> Result<Option<[f64; 2]>> {
    if let Some(trim) = c.trim().filter(|t| t.curve_parameter == world_domain) {
        // Subrange world trims also serve as diagnostic-only proof windows;
        // they do not replace the topology's complete loop (see BoundaryUse).
        trim.validate()?;
        let d = pcurve.domain();
        require(
            trim.pcurve_parameter.iter().all(|t| *t >= d[0] && *t <= d[1]),
            "Coedge UV trim lies outside its pcurve domain",
        )?;
        Ok(Some(trim.pcurve_parameter))
    } else {
        Ok(None)
    }
}

#[inline]
fn sample_pcurve_uv(
    pcurve: &CurveSampler<'_>,
    uv_window: Option<[f64; 2]>,
    fraction: f64,
) -> Result<[f64; 2]> {
    if let Some(w) = uv_window {
        let t = w[0] * (1. - fraction) + w[1] * fraction;
        pcurve.point2(t)
    } else {
        pcurve.point_at_fraction2(fraction)
    }
}

struct PreparedEdge<'a> {
    sampler: CurveSampler<'a>,
    endpoints: [[f64; 3]; 2],
}

struct CoedgeSampleCtx<'a> {
    raw_surface: &'a Surface,
    surface: &'a SurfaceRefSampler<'a>,
    surface_periodic: bool,
    coedge: &'a crate::Coedge,
    prepared_edge: &'a PreparedEdge<'a>,
    pcurve: &'a CurveSampler<'a>,
    window: Option<[f64; 2]>,
    uv0: [f64; 2],
    uv1: [f64; 2],
    tol: f64,
}

fn check_coedge_sampled_agreement(
    ctx: &CoedgeSampleCtx<'_>,
    uv_ring: &mut Vec<[f64; 2]>,
) -> Result<()> {
    let check_pair = |raw_uv: [f64; 2], q: [f64; 3]| -> Result<()> {
        let uv = if ctx.surface_periodic {
            nurbs_core::periodic_trim::wrapped_surface_parameters(ctx.raw_surface, raw_uv)?
        } else {
            raw_uv
        };
        let p = ctx.surface.point(uv[0], uv[1])?;
        require(
            distance3(p, q) <= ctx.tol,
            "pcurve/surface and 3D edge disagree at sampled parameters",
        )
    };
    let rev = ctx.coedge.reversed;
    uv_ring.push(ctx.uv0);
    check_pair(ctx.uv0, ctx.prepared_edge.endpoints[usize::from(rev)])?;
    for i in 1..8 {
        let t = i as f64 / 8.;
        let raw_uv = sample_pcurve_uv(ctx.pcurve, ctx.window, t)?;
        uv_ring.push(raw_uv);
        let edge_t = if rev { 1. - t } else { t };
        let q = ctx.prepared_edge.sampler.point_at_fraction3(edge_t)?;
        check_pair(raw_uv, q)?;
    }
    check_pair(ctx.uv1, ctx.prepared_edge.endpoints[usize::from(!rev)])
}

fn signed_uv_ring_area(uv: &[[f64; 2]]) -> f64 {
    (0..uv.len())
        .map(|i| {
            let a = uv[i];
            let b = uv[(i + 1) % uv.len()];
            (a[0] - uv[0][0]) * (b[1] - uv[0][1]) - (b[0] - uv[0][0]) * (a[1] - uv[0][1])
        })
        .sum::<f64>()
}

impl Model {
    pub fn loop_uv(&self, id: usize, segments: usize) -> Result<Vec<[f64; 2]>> {
        require((1..=64).contains(&segments), "Edge sampling must be 1..64")?;
        let wire = self.loops.get(id).ok_or_else(|| invalid("Unknown loop"))?;
        let mut uv = Vec::with_capacity(wire.coedges.len() * segments);
        for c in &wire.coedges {
            let pcurve = CurveSampler::new(&c.pcurve)?;
            require(pcurve.dimension() == 2, "pcurve must be 2D")?;
            let world_domain = self.edges[c.edge].curve.domain();
            let window = resolve_coedge_uv_window(c, &pcurve, world_domain)?;
            for i in 0..segments {
                uv.push(sample_pcurve_uv(&pcurve, window, i as f64 / segments as f64)?);
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
        self.validate_ids_and_tolerances(count)?;
        let prepared_edges = self.validate_vertices_and_edges(tol)?;
        self.validate_faces_and_loops(&prepared_edges, tol, sampled_agreement)?;
        let boundary = self.count_boundary_edges();
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

    fn validate_ids_and_tolerances(&self, count: usize) -> Result<()> {
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
        let mut ids = HashSet::with_capacity(count);
        for id in all_ids {
            require(ids.insert(id), "Topology IDs must be unique")?;
        }
        for (kind, group_ids) in self.identity_groups() {
            require(
                group_ids.iter().all(|id| id.kind() == kind),
                "Topology ID prefix does not match its entity kind",
            )?;
            require(
                group_ids
                    .iter()
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
        Ok(())
    }

    fn validate_vertices_and_edges(&self, tol: f64) -> Result<Vec<PreparedEdge<'_>>> {
        let mut vertex_used = vec![false; self.vertices.len()];
        for v in &self.vertices {
            require(
                v.point.iter().all(|x| x.is_finite() && x.abs() <= 1e6),
                "Invalid vertex coordinates",
            )?;
        }
        let mut prepared = Vec::with_capacity(self.edges.len());
        for e in &self.edges {
            let sampler = CurveSampler::new(&e.curve)?;
            require(sampler.dimension() == 3, "Edge curve must be 3D")?;
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
            let mut endpoints = [[0.; 3]; 2];
            for (i, &v) in e.vertices.iter().enumerate() {
                let point = self
                    .vertices
                    .get(v)
                    .ok_or_else(|| invalid("Edge references unknown vertex"))?
                    .point;
                vertex_used[v] = true;
                let ep = sampler.point_at_fraction3(i as f64)?;
                require(
                    distance3(ep, point) <= tol,
                    "Edge endpoint does not match vertex",
                )?;
                endpoints[i] = ep;
            }
            prepared.push(PreparedEdge { sampler, endpoints });
        }
        require(vertex_used.iter().all(|v| *v), "Unused vertex")?;
        Ok(prepared)
    }

    fn validate_faces_and_loops(
        &self,
        prepared_edges: &[PreparedEdge<'_>],
        tol: f64,
        sampled_agreement: bool,
    ) -> Result<()> {
        let mut loop_owner = vec![None; self.loops.len()];
        let mut edge_used = vec![false; self.edges.len()];
        let mut uv_ring = Vec::with_capacity(64);
        for (fi, f) in self.faces.iter().enumerate() {
            // Borrowed sampler validates the face control net once without cloning it.
            let surface = SurfaceRefSampler::new(&f.surface)?;
            let surface_periodic = f.surface.periodic_u || f.surface.periodic_v;
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
                uv_ring.clear();
                let mut previous: Option<usize> = None;
                let mut first = None;
                let mut previous_uv: Option<[f64; 2]> = None;
                let mut first_uv = None;
                for c in &wire.coedges {
                    let edge = self
                        .edges
                        .get(c.edge)
                        .ok_or_else(|| invalid("Coedge references unknown edge"))?;
                    let prepared_edge = &prepared_edges[c.edge];
                    edge_used[c.edge] = true;
                    let pcurve = CurveSampler::new(&c.pcurve)?;
                    require(pcurve.dimension() == 2, "pcurve must be 2D")?;
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
                    let window =
                        resolve_coedge_uv_window(c, &pcurve, prepared_edge.sampler.domain())?;
                    let uv0 = sample_pcurve_uv(&pcurve, window, 0.)?;
                    let uv1 = sample_pcurve_uv(&pcurve, window, 1.)?;
                    if let Some(p) = previous_uv {
                        require(distance2(p, uv0) <= 1e-9, "Loop UV chain is disconnected")?;
                    } else {
                        first_uv = Some(uv0);
                    }
                    previous_uv = Some(uv1);
                    if sampled_agreement {
                        check_coedge_sampled_agreement(
                            &CoedgeSampleCtx {
                                raw_surface: &f.surface,
                                surface: &surface,
                                surface_periodic,
                                coedge: c,
                                prepared_edge,
                                pcurve: &pcurve,
                                window,
                                uv0,
                                uv1,
                                tol,
                            },
                            &mut uv_ring,
                        )?;
                    }
                }
                require(previous == first, "Loop is not closed")?;
                let end_uv = previous_uv.unwrap();
                let start_uv = first_uv.unwrap();
                let uv_close = distance2(end_uv, start_uv) <= 1e-9;
                let periodic_close = if surface_periodic {
                    let wrapped =
                        nurbs_core::periodic_trim::wrapped_surface_parameters(&f.surface, end_uv)?;
                    distance2(wrapped, start_uv) <= 1e-9
                } else {
                    false
                };
                require(
                    uv_close || periodic_close,
                    "UV loop is not closed on its lifted chart",
                )?;
                let area = if sampled_agreement {
                    signed_uv_ring_area(&uv_ring)
                } else {
                    signed_uv_ring_area(&self.loop_uv(l, 8)?)
                };
                require(
                    if li == 0 { area > 0. } else { area < 0. },
                    "Outer UV loop must be CCW and hole loops CW",
                )?;
            }
        }
        require(loop_owner.iter().all(Option::is_some), "Unused loop")?;
        require(edge_used.iter().all(|v| *v), "Unused edge")?;
        Ok(())
    }

    fn count_boundary_edges(&self) -> usize {
        let mut uses = vec![0u32; self.edges.len()];
        let mut touched = Vec::new();
        let mut boundary = 0usize;
        for s in &self.shells {
            touched.clear();
            for u in &s.faces {
                let f = &self.faces[u.face];
                for &l in std::iter::once(&f.outer).chain(&f.holes) {
                    for c in &self.loops[l].coedges {
                        if !self.edges[c.edge].degenerate {
                            let slot = &mut uses[c.edge];
                            if *slot == 0 {
                                touched.push(c.edge);
                            }
                            *slot += 1;
                        }
                    }
                }
            }
            for &edge_idx in &touched {
                if uses[edge_idx] == 1 {
                    boundary += 1;
                }
                uses[edge_idx] = 0;
            }
        }
        boundary
    }
}

mod pole_boundary;
pub(crate) use pole_boundary::validate_pole_boundary;
