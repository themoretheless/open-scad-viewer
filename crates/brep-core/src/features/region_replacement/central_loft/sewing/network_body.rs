//! Closed INCIDENCE candidate for the actual central network and original
//! strips/stock. Closure is not embedding, volume, thickness or cap admission.
use super::*;
pub struct SewnCornerNetworkCandidate {
    model: Model,
    central_faces: Vec<usize>,
    strip_faces: [usize; 3],
    cap_faces: [usize; 3],
    closure: crate::watertight::QualifiedReport,
    trims: crate::face_domain::TrimAudit,
    final_jets: Vec<nurbs_core::surface_g2_pcurves::ExtrusionBoundaryReport>,
    pub cells: usize,
}
impl SewnCornerNetworkCandidate {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn final_jets(&self) -> &[nurbs_core::surface_g2_pcurves::ExtrusionBoundaryReport] {
        &self.final_jets
    }

    pub fn central_faces(&self) -> &[usize] {
        &self.central_faces
    }
    pub fn strip_faces(&self) -> [usize; 3] {
        self.strip_faces
    }
    pub fn cap_faces(&self) -> [usize; 3] {
        self.cap_faces
    }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport {
        &self.closure
    }
    pub fn trims(&self) -> &crate::face_domain::TrimAudit {
        &self.trims
    }
}
fn line(a: [f64; 3], b: [f64; 3]) -> Result<Curve> {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()])
}
/// Synchronize ONLY face-local UV endpoints of already shared topological
/// vertices on regular strip/plane charts. This is not 3D distance welding:
/// edits stay inside the existing UV tolerance and final whole-edge lifts
/// must still pass. Singular central pole charts are deliberately excluded.
fn own_uv_roots(m: &Model, wire: &mut crate::Loop, tolerance: f64) -> Result<()> {
    let mut roots = std::collections::BTreeMap::<usize, Vec<f64>>::new();
    for c in &mut wire.coedges {
        let e = &m.edges[c.edge];
        require(
            !e.degenerate
                && c.trim().is_none()
                && crate::rounded_patch_network::clamped_single(
                    &c.pcurve.knots,
                    c.pcurve.degree,
                    c.pcurve.control_points.len(),
                ),
            "UV root ownership needs regular full boundary uses",
        )?;
        for end in 0..2 {
            let vertex = e.vertices[end ^ usize::from(c.reversed)];
            let index = if end == 0 {
                0
            } else {
                c.pcurve.control_points.len() - 1
            };
            let proposed = c.pcurve.control_points[index].clone();
            let owned = roots.entry(vertex).or_insert_with(|| proposed.clone());
            let mut error = nurbs_core::interval_eval::Interval::point(0.);
            for k in 0..2 {
                let d = nurbs_core::interval_eval::Interval::point(proposed[k])
                    .sub(nurbs_core::interval_eval::Interval::point(owned[k]))?;
                error = error.add(nurbs_core::interval_eval::Interval::point(
                    d.lo.abs().max(d.hi.abs()),
                ))?;
            }
            require(
                error.hi <= tolerance,
                "Face-local roots disagree beyond the existing UV tolerance",
            )?;
            c.pcurve.control_points[index] = owned.clone();
        }
    }
    Ok(())
}
impl CentralLoftCandidate<'_, '_, '_> {
    /// Keep the actual stock/strip supports and attach each already split
    /// central edge ONCE. Root identities come from original endpoint ownership,
    /// never proximity welding. No Qualified/Solid result is produced here.
    pub fn sew_network_candidate(
        &self,
        patches: &[Surface],
        limits: CornerSewLimits,
    ) -> Result<SewnCornerNetworkCandidate> {
        require(
            (1000..=100000).contains(&limits.cells)
                && (1..=1000000).contains(&limits.exact_work)
                && (1..=100000).contains(&limits.trim_pairs)
                && (1..=1000000).contains(&limits.domain_cells),
            "Bound network body sewing",
        )?;
        let network =
            self.assemble_open_network(patches, limits.cells / 2, limits.exact_work / 2)?;
        let mut cells = network.cells;
        let source = self.body.source();
        let corner = self.strips.plan.vertex;
        let mut m = source.clone();
        m.shells.clear();
        m.bodies.clear();
        let cap = network.model();
        let vertex_offset = m.vertices.len();
        let edge_offset = m.edges.len();
        m.vertices.extend(cap.vertices.iter().cloned());
        for e in &cap.edges {
            m.edges.push(crate::Edge {
                curve: e.curve.clone(),
                vertices: e.vertices.map(|v| v + vertex_offset),
                degenerate: e.degenerate,
            });
        }
        let mut central_faces = Vec::new();
        for f in &cap.faces {
            let mut wire = cap.loops[f.outer].clone();
            for c in &mut wire.coedges {
                c.edge += edge_offset;
            }
            let outer = m.loops.len();
            m.loops.push(wire);
            central_faces.push(m.faces.len());
            m.faces.push(crate::Face {
                surface: f.surface.clone(),
                outer,
                holes: vec![],
            });
        }
        let mut extra = vec![Vec::new(); source.faces.len()];
        for binding in network
            .support_binding_proposals()
            .iter()
            .filter(|b| b.boundary % 2 == 1)
        {
            let faces = source
                .faces
                .iter()
                .enumerate()
                .filter(|(_, f)| f.surface == binding.support)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            require(
                faces.len() == 1,
                "Network connector lost its original stock face",
            )?;
            extra[faces[0]].push(binding.edge + edge_offset);
        }
        let mut strip_faces = [0; 3];
        let mut cap_faces = [0; 3];
        for (j, strip) in self.strips.strips.iter().enumerate() {
            let selected = &source.edges[strip.source_edge];
            require(
                selected.curve.degree == 1
                    && selected.curve.control_points.len() == 2
                    && selected.curve.weights[0] == selected.curve.weights[1],
                "Network stock retention currently needs original affine edges",
            )?;

            let other = *selected
                .vertices
                .iter()
                .find(|&&v| v != corner)
                .ok_or_else(|| refuse("Missing retained stock endpoint"))?;
            let axes = (0..3)
                .filter(|&k| source.vertices[corner].point[k] != source.vertices[other].point[k])
                .collect::<Vec<_>>();
            require(
                axes.len() == 1,
                "Network body currently needs stock-aligned original edges",
            )?;
            let axis = axes[0];
            let fixed = strip.cutters[1].faces[strip.cutter_faces[1]]
                .surface
                .control_points[0][0][axis];
            require(
                strip.cutters[1].faces[strip.cutter_faces[1]]
                    .surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[axis] == fixed)
                    && self.stock[axis][0] < fixed
                    && fixed < self.stock[axis][1],
                "Original far cutter must retain an interior stock-axis plane",
            )?;
            let mut sharp = source.vertices[corner].point;
            sharp[axis] = fixed;
            let sharp_id = topology::vertex(&mut m, sharp);
            let mut ends = selected.vertices;
            ends[usize::from(ends[1] == corner)] = sharp_id;
            m.edges[strip.source_edge] = crate::Edge {
                curve: line(m.vertices[ends[0]].point, m.vertices[ends[1]].point)?,
                vertices: ends,
                degenerate: false,
            };
            let near = usize::from(strip.end_order[0] != 0);
            let original = &strip.patch.ends[near].curve;
            let owners = self
                .curves
                .iter()
                .enumerate()
                .filter(|(i, c)| {
                    i % 2 == 0 && (*c == original || c.reverse().ok().as_ref() == Some(original))
                })
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            require(
                owners.len() == 1,
                "Original strip lost its central profile owner",
            )?;
            let boundary = owners[0];
            let pmodel = &strip.patch.model;
            pmodel.validate()?;
            require(
                pmodel.faces.len() == 1
                    && pmodel.faces[0].holes.is_empty()
                    && pmodel.edges.len() == 4
                    && pmodel.vertices.len() == 4,
                "Original strip must retain one owned face",
            )?;
            let near_edge = strip.patch.edges[if near == 0 { 0 } else { 2 }];
            let far_edge = strip.patch.edges[if near == 0 { 2 } else { 0 }];
            let bindings = network
                .support_binding_proposals()
                .iter()
                .filter(|b| b.boundary == boundary)
                .collect::<Vec<_>>();
            require(
                !bindings.is_empty()
                    && bindings
                        .iter()
                        .all(|b| b.support == pmodel.faces[0].surface),
                "Split central curve lost the actual original strip support",
            )?;
            let domain = self.curves[boundary].domain();
            let mut owned = [None; 2];
            for binding in &bindings {
                let use_ = cap.loops[cap.faces[binding.patch].outer]
                    .coedges
                    .iter()
                    .find(|c| c.edge == binding.edge)
                    .ok_or_else(|| refuse("Missing central boundary use"))?;
                let forward = use_.pcurve.control_points[0][0]
                    < use_.pcurve.control_points.last().unwrap()[0];
                let source_forward = self.pcurves[boundary].control_points[0][0]
                    < self.pcurves[boundary].control_points.last().unwrap()[0];
                let reverse = forward != source_forward;
                for end in 0..2 {
                    if binding.parameter[end] == domain[end] {
                        owned[end] = Some(
                            cap.edges[binding.edge].vertices[end ^ usize::from(reverse)]
                                + vertex_offset,
                        );
                    }
                }
            }
            let owned = [
                owned[0].ok_or_else(|| refuse("Unowned first strip root"))?,
                owned[1].ok_or_else(|| refuse("Unowned last strip root"))?,
            ];
            let original_edge = &pmodel.edges[near_edge].curve;
            require(
                self.curves[boundary] == *original_edge
                    || self.curves[boundary].reverse()?.eq(original_edge),
                "Original strip edge clock lost its source owner",
            )?;
            let reversed = self.curves[boundary] != *original_edge;
            let mut vertices = vec![None; pmodel.vertices.len()];
            for end in 0..2 {
                vertices[pmodel.edges[near_edge].vertices[end]] =
                    Some(owned[end ^ usize::from(reversed)]);
            }
            for (i, v) in pmodel.vertices.iter().enumerate() {
                if vertices[i].is_none() {
                    vertices[i] = Some(topology::vertex(&mut m, v.point));
                }
            }
            let vertices = vertices.into_iter().map(Option::unwrap).collect::<Vec<_>>();
            let mut edges = vec![None; pmodel.edges.len()];
            for (i, e) in pmodel.edges.iter().enumerate() {
                if i != near_edge {
                    edges[i] = Some(topology::edge(
                        &mut m,
                        e.curve.clone(),
                        e.vertices.map(|v| vertices[v]),
                        e.degenerate,
                    ));
                }
            }
            let mut uses = bindings
                .iter()
                .map(|b| (b.edge + edge_offset, b.pcurve.clone()))
                .collect::<Vec<_>>();
            for c in &pmodel.loops[pmodel.faces[0].outer].coedges {
                if c.edge != near_edge {
                    uses.push((
                        edges[c.edge].unwrap(),
                        if c.reversed {
                            c.pcurve.reverse()?
                        } else {
                            c.pcurve.clone()
                        },
                    ));
                }
            }
            let mut wire = topology::loop_from_edges(&m, uses)?;
            own_uv_roots(&m, &mut wire, self.options.tolerance_uv)?;
            topology::positive_loop(&mut wire, self.options.tolerance_uv, &mut cells, limits)
                .map_err(|e| {
                    crate::Error::new(
                        e.code,
                        format!(
                            "{}; strip={j}; UV={:?}",
                            e.message,
                            wire.coedges
                                .iter()
                                .map(|c| (
                                    c.edge,
                                    c.reversed,
                                    c.pcurve.control_points.first().cloned(),
                                    c.pcurve.control_points.last().cloned()
                                ))
                                .collect::<Vec<_>>()
                        ),
                    )
                })?;
            let outer = m.loops.len();
            m.loops.push(wire);
            strip_faces[j] = m.faces.len();
            m.faces.push(crate::Face {
                surface: pmodel.faces[0].surface.clone(),
                outer,
                holes: vec![],
            });
            for (side, index) in [strip.patch.edges[3], strip.patch.edges[1]]
                .into_iter()
                .enumerate()
            {
                extra[strip.support_faces[side]].push(edges[index].unwrap());
            }
            let far = edges[far_edge].unwrap();
            let far_vertices = m.edges[far].vertices;
            let mut cap_edges = vec![far];
            for side in 0..2 {
                let v = far_vertices[side];
                let boundary_curve = line(m.vertices[v].point, sharp)?;
                let id = topology::edge(&mut m, boundary_curve, [v, sharp_id], false);
                cap_edges.push(id);
                // Original contact identities, not the far curve's authored
                // traversal, determine which stock face owns this line.
                let contact = [strip.patch.edges[3], strip.patch.edges[1]]
                    .iter()
                    .position(|&e| {
                        pmodel.edges[e]
                            .vertices
                            .iter()
                            .any(|&v| vertices[v] == far_vertices[side])
                    })
                    .ok_or_else(|| refuse("Far root lost its contact owner"))?;
                extra[strip.support_faces[contact]].push(id);
            }
            let surface = planar::cap(self.stock, axis, fixed);
            let uses = cap_edges
                .into_iter()
                .map(|e| Ok((e, planar::project(&m.edges[e].curve, &surface)?)))
                .collect::<Result<Vec<_>>>()?;
            let mut wire = topology::loop_from_edges(&m, uses)?;
            own_uv_roots(&m, &mut wire, self.options.tolerance_uv)?;
            topology::positive_loop(&mut wire, self.options.tolerance_uv, &mut cells, limits)?;
            let outer = m.loops.len();
            m.loops.push(wire);
            cap_faces[j] = m.faces.len();
            m.faces.push(crate::Face {
                surface,
                outer,
                holes: vec![],
            });
        }
        for f in 0..source.faces.len() {
            let ids = source.loops[source.faces[f].outer]
                .coedges
                .iter()
                .map(|c| c.edge)
                .chain(extra[f].iter().copied())
                .collect::<Vec<_>>();
            let surface = &m.faces[f].surface;
            let uses = ids
                .into_iter()
                .map(|e| Ok((e, planar::project(&m.edges[e].curve, surface)?)))
                .collect::<Result<Vec<_>>>()?;
            let mut wire = topology::loop_from_edges(&m, uses)?;
            own_uv_roots(&m, &mut wire, self.options.tolerance_uv)?;
            topology::positive_loop(&mut wire, self.options.tolerance_uv, &mut cells, limits)?;
            let outer = m.faces[f].outer;
            m.loops[outer] = wire;
        }
        topology::compact_vertices(&mut m);
        let seed = source.shells[0]
            .faces
            .iter()
            .find(|u| u.face == 0)
            .ok_or_else(|| refuse("Missing original stock orientation"))?
            .reversed;
        topology::orient(&mut m, seed)?;
        m.inherit_topology_ids(&[source, cap]);
        m.validate()?;
        require(
            cells < limits.cells,
            "No actual network closure work remains",
        )?;
        let closure =
            crate::watertight::inspect_with_exact(&m, limits.cells - cells, limits.exact_work / 2)?;
        cells += closure.closure.agreement.cells;
        require(
            closure.closure.status == crate::watertight::Closure::WithinTolerance,
            &format!(
                "Actual network incidence or boundary lifts remain unproven: status={:?}, uses={:?}",
                closure.closure.status,
                closure
                    .closure
                    .agreement
                    .uses
                    .iter()
                    .filter(|u| u.status
                        != nurbs_core::curve_surface_agreement::Status::WithinTolerance)
                    .collect::<Vec<_>>()
            ),
        )?;
        m.shells[0].closed = true;
        m.bodies = vec![crate::Body {
            outer_shell: 0,
            inner_shells: vec![],
        }];
        m.inherit_topology_ids(&[source, cap]);
        m.validate()?;
        require(cells < limits.cells, "No final network trim work remains")?;
        let allowance = (limits.cells - cells) / 2;
        let trims = crate::face_domain::audit_trim_regions(
            &m,
            self.options.tolerance_uv,
            limits.trim_pairs,
            allowance,
            allowance.min(limits.domain_cells),
        )?;
        cells += trims.cells + trims.domain_cells;
        require(
            cells <= limits.cells && trims.all_valid,
            "Actual network face trims remain unproven",
        )?;
        let mut final_jets = Vec::new();
        let bindings = network.support_binding_proposals();
        for (i, binding) in bindings.iter().enumerate() {
            let target = central_faces[binding.patch];
            let edge = binding.edge + edge_offset;
            let owners = m
                .faces
                .iter()
                .enumerate()
                .filter(|(f, face)| {
                    *f != target
                        && face.surface == binding.support
                        && m.loops[face.outer].coedges.iter().any(|c| c.edge == edge)
                })
                .map(|(f, _)| f)
                .collect::<Vec<_>>();
            require(
                owners.len() == 1,
                "Final shared edge lost its original support use",
            )?;
            let source_face = owners[0];
            let forward_pc = |f: usize| -> Result<Curve> {
                let c = m.loops[m.faces[f].outer]
                    .coedges
                    .iter()
                    .find(|c| c.edge == edge)
                    .ok_or_else(|| refuse("Final edge use missing"))?;
                let mut pc = c.pcurve.clone();
                require(
                    c.trim().is_none()
                        && crate::rounded_patch_network::clamped_single(
                            &pc.knots,
                            pc.degree,
                            pc.control_points.len(),
                        ),
                    "Final G2 requires full owned Bezier clocks",
                )?;
                if c.reversed {
                    pc.control_points.reverse();
                    pc.weights.reverse();
                }
                Ok(pc)
            };
            let source_pc = forward_pc(source_face)?;
            let target_pc = forward_pc(target)?;
            require(cells < limits.cells, "No final network G2 work remains")?;
            let report = self.inspect_network_support_piece(
                &m.faces[target].surface,
                binding.boundary,
                &m.edges[edge].curve,
                &source_pc,
                &target_pc,
                (limits.cells - cells) / (bindings.len() - i),
                0,
            )?;
            cells += report.cells;
            require(
                report.positions.proven
                    && report.regularity_proven
                    && report.pending == 0
                    && report.decision == nurbs_core::surface_join::Decision::WithinTolerance,
                &format!(
                    "Final sewn G2 remains unproven: boundary={} patch={} normal={:?} tensor={:?}",
                    binding.boundary, binding.patch, report.normal, report.tensor
                ),
            )?;
            let face_sign = |face| {
                m.shells[0]
                    .faces
                    .iter()
                    .find(|f| f.face == face)
                    .unwrap()
                    .reversed
            };
            require(
                (face_sign(source_face) ^ face_sign(target))
                    == (report.orientation == nurbs_core::surface_g2::Orientation::Opposite),
                "Final sewn material orientation disagrees with geometric G2",
            )?;
            final_jets.push(report);
        }
        Ok(SewnCornerNetworkCandidate {
            final_jets,
            model: m,
            central_faces,
            strip_faces,
            cap_faces,
            closure,
            trims,
            cells,
        })
    }
}
