//! Assembly of oriented Boolean patches into topologically closed shells.
use super::{
    MixedBrep, MixedEdge, MixedPcurve, MixedTopology, PairedBooleanPatches, Result, RootSewIndex,
    bounds_separated, carrier_bounds, unproved,
};
use brep_topology::{FaceUse, Shell};

impl PairedBooleanPatches {
    /// Integrate the signed volume of a shell bounded by two paired SS faces.
    /// Material orientation is preserved, so cavity shells yield negative flux.
    pub fn paired_shell_signed_volume(
        &self,
        shell: usize,
        tolerance_mm: f64,
        limits: super::RootBoundaryVolumeLimits,
    ) -> Result<super::RootBoundaryVolumeReport> {
        let model = self.sew_closed_shells(tolerance_mm)?;
        let topology = model.topology();
        let shell = topology
            .shells
            .get(shell)
            .ok_or_else(|| unproved("Boolean shell does not exist"))?;
        let mut faces: [FaceUse; 2] = shell
            .faces
            .clone()
            .try_into()
            .map_err(|_| unproved("Signed root volume needs two paired SS faces"))?;
        let first = &topology.faces[faces[0].face];
        if matches!(&topology.loops[first.outer].coedges[0].pcurve, MixedPcurve::Root(p) if p.support == 1)
        {
            faces.swap(0, 1);
        }
        super::volume::root_shell_volume(topology, &faces, limits)
    }

    /// Integrate signed flux over the selected closed shell's finite UV faces.
    pub fn shell_signed_volume(
        &self,
        shell: usize,
        tolerance_mm: f64,
        tolerance_uv: f64,
        limits: super::RootBoundaryVolumeLimits,
    ) -> Result<super::RootBoundaryVolumeReport> {
        if !tolerance_uv.is_finite() || tolerance_uv <= 0. {
            return Err(unproved("Shell flux needs positive finite UV tolerance"));
        }
        let model = self.sew_closed_shells(tolerance_mm)?;
        let topology = model.topology();
        let shell = topology
            .shells
            .get(shell)
            .ok_or_else(|| unproved("Boolean shell does not exist"))?;
        super::volume::shell_volume(topology, &shell.faces, Some(tolerance_uv), limits)
    }

    /// Compute complete ray parity against one selected shell. Crossing
    /// provenance indexes the face uses of that shell in sewing order.
    /// Interpreting parity as containment requires an embedded shell.
    pub fn classify_shell_point(
        &self,
        shell: usize,
        tolerance_mm: f64,
        point: [f64; 3],
        directions: &[[f64; 3]],
        limits: super::RootSolidPointLimits,
    ) -> Result<super::RootSolidPointReport> {
        let model = self.sew_closed_shells(tolerance_mm)?;
        let topology = model.topology();
        let shell = topology
            .shells
            .get(shell)
            .ok_or_else(|| unproved("Boolean shell does not exist"))?;
        super::volume::shell_point(topology, shell, point, directions, limits, true)
    }

    /// Determine whether the full source shell lies inside the target.
    /// Disjoint, connected shells have constant target parity along the source.
    pub fn shell_is_inside(
        &self,
        source: usize,
        target: usize,
        tolerance_mm: f64,
        directions: &[[f64; 3]],
        limits: super::RootMaterialBoxLimits,
    ) -> Result<bool> {
        if source == target {
            return Err(unproved("Shell containment needs two distinct shells"));
        }
        let model = self.sew_disjoint_shells(tolerance_mm)?;
        shell_is_inside(model.topology(), source, target, directions, limits)
    }

    /// XOR consists of two directed differences. Their interiors are
    /// disjoint by the Boolean truth table, but their boundaries may share an
    /// SS contour. Sew each material side before concatenating the two models.
    pub(super) fn assemble_symmetric_difference(
        self,
        tolerance_mm: f64,
        directions: &[[f64; 3]],
        containment: super::RootMaterialBoxLimits,
        volume: super::RootBoundaryVolumeLimits,
    ) -> Result<(MixedBrep, [f64; 2])> {
        let PairedBooleanPatches {
            layouts,
            regions,
            uncut,
        } = self;
        let mut halves = [
            Self {
                layouts: layouts.clone(),
                regions: [Vec::new(), Vec::new()],
                uncut: [Vec::new(), Vec::new()],
            },
            Self {
                layouts,
                regions: [Vec::new(), Vec::new()],
                uncut: [Vec::new(), Vec::new()],
            },
        ];
        for (side, (regions, uncut)) in regions.into_iter().zip(uncut).enumerate() {
            for region in regions {
                let reverse = region
                    .material()
                    .boolean_material()
                    .reverse_material_normal();
                halves[side ^ usize::from(reverse)].regions[side].push(region);
            }
            let topology = halves[0].layouts[side].model().topology();
            let mut original_normals = vec![None; topology.faces.len()];
            for use_ in topology.shells.iter().flat_map(|shell| &shell.faces) {
                original_normals[use_.face] = Some(use_.reversed);
            }
            for face in uncut {
                let original = original_normals[face.face]
                    .ok_or_else(|| unproved("XOR face has no original material orientation"))?;
                halves[side ^ usize::from(face.reversed != original)].uncut[side].push(face);
            }
        }
        // Split the tolerance between both differences and reserve room for rounding.
        let limit = super::RootBoundaryVolumeLimits {
            absolute_error: volume.absolute_error / 2.1,
            ..volume
        };
        let [first, second] = halves;
        let (a, [a_lo, a_hi]) =
            first.assemble_bodies(tolerance_mm, directions, containment, limit)?;
        let (b, [b_lo, b_hi]) =
            second.assemble_bodies(tolerance_mm, directions, containment, limit)?;
        combine_boolean_differences((a, [a_lo, a_hi]), (b, [b_lo, b_hi]), volume)
    }

    /// Assemble material bodies using the complete containment forest and the
    /// independently integrated signed flux of each oriented boundary.
    pub fn assemble_bodies(
        &self,
        tolerance_mm: f64,
        directions: &[[f64; 3]],
        containment: super::RootMaterialBoxLimits,
        volume: super::RootBoundaryVolumeLimits,
    ) -> Result<(MixedBrep, [f64; 2])> {
        let model = self.sew_closed_shells(tolerance_mm)?;
        admit_disjoint_boolean_shells(model, directions, containment, volume)
    }

    /// Sew closed shells whose boundaries are pairwise disjoint.
    ///
    /// The admitted operands account for same-source contacts. The full SS
    /// census accounts for contacts between the two cut supports, and the
    /// material box covers exclude any opposing contacts on uncut faces.
    /// With those covers, any remaining inter-shell contact must be represented
    /// by shared topology. Ordinary edges already connect their incident faces
    /// into one shell, so check for contacts that survive only at vertices.
    pub fn sew_disjoint_shells(&self, tolerance_mm: f64) -> Result<MixedBrep> {
        let model = self.sew_closed_shells(tolerance_mm)?;
        require_separate_shell_vertices(model.topology())?;
        Ok(model)
    }

    /// Sew the selected faces by original topological identity and exact SS
    /// edges. Returns validated closed shells with body roles still unassigned.
    pub fn sew_closed_shells(&self, tolerance_mm: f64) -> Result<MixedBrep> {
        let sources = (0..2).map(|side| {
            let source = self.layouts[side].model().topology();
            let cut_faces = self.regions[side].iter().map(|region| FaceUse {
                face: region.face(),
                reversed: region.material().boolean_material().reversed_surface(),
            });
            (source, cut_faces.chain(self.uncut[side].iter().cloned()))
        });
        sew_selected_shells(sources, tolerance_mm)
    }
}

pub(super) fn combine_boolean_differences(
    (a, [a_lo, a_hi]): (MixedBrep, [f64; 2]),
    (b, [b_lo, b_hi]): (MixedBrep, [f64; 2]),
    volume: super::RootBoundaryVolumeLimits,
) -> Result<(MixedBrep, [f64; 2])> {
    use nurbs_core::interval_eval::Interval;

    if a.topology().bodies.is_empty() {
        return Ok((b, [b_lo, b_hi]));
    }
    if b.topology().bodies.is_empty() {
        return Ok((a, [a_lo, a_hi]));
    }
    let total = Interval::new(a_lo, a_hi)?.add(Interval::new(b_lo, b_hi)?)?;
    if total.lo <= 0.
        || Interval::point(total.hi).sub(Interval::point(total.lo))?.hi > volume.absolute_error
    {
        return Err(unproved(
            "Boolean XOR material volume does not meet accuracy",
        ));
    }
    let (model, _) = a.concatenate(&b)?.into_parts();
    Ok((model, [total.lo, total.hi]))
}

/// Admit bodies after the caller has established an embedded boundary.
pub(super) fn admit_disjoint_boolean_shells(
    model: MixedBrep,
    directions: &[[f64; 3]],
    containment: super::RootMaterialBoxLimits,
    mut volume: super::RootBoundaryVolumeLimits,
) -> Result<(MixedBrep, [f64; 2])> {
    use brep_topology::Body;
    use nurbs_core::interval_eval::Interval;

    let mut topology = model.topology;
    require_separate_shell_vertices(&topology)?;
    let count = topology.shells.len();
    if count > 64 || !volume.absolute_error.is_finite() || volume.absolute_error <= 0. {
        return Err(unproved(
            "Invalid Boolean body shell count or volume accuracy",
        ));
    }
    if count == 0 {
        return Ok((MixedBrep::from_topology(topology)?, [0., 0.]));
    }
    let absolute_error = volume.absolute_error;
    if count > 1 {
        // Leave room for outward rounding when combining shell intervals.
        volume.absolute_error /= 2. * count as f64;
    }
    // A shell's bounded interior is also inside its boundary's control box.
    // A strict gap therefore rules out nesting without casting a ray.
    let bounds: Vec<_> = topology.shells.iter().map(|shell| {
        carrier_bounds(shell.faces.iter().map(|use_| &topology.faces[use_.face].surface))
    }).collect();
    let mut inside = vec![vec![false; count]; count];
    for (source, row) in inside.iter_mut().enumerate() {
        for (target, cell) in row.iter_mut().enumerate() {
            if source != target && !bounds_separated(&bounds[source], &bounds[target]) {
                *cell = shell_is_inside(&topology, source, target, directions, containment)?;
            }
        }
    }
    let (parents, depths) = super::volume::analyze_shell_nesting(&inside)?;

    let mut total = Interval::point(0.);
    for (shell, &depth) in topology.shells.iter().zip(&depths) {
        let report = super::volume::shell_volume(
            &topology,
            &shell.faces,
            Some(containment.point.tolerance_uv),
            volume,
        )?;
        let [lo, hi] = report
            .signed_bounds
            .filter(|_| report.accuracy_proven)
            .ok_or_else(|| unproved(&format!("Boolean body shell signed volume is unresolved: {report:?}")))?;
        if (depth % 2 == 0 && lo <= 0.) || (depth % 2 != 0 && hi >= 0.) {
            return Err(unproved(
                "Boolean shell material orientation disagrees with nesting",
            ));
        }
        total = total.add(Interval::new(lo, hi)?)?;
    }
    if total.lo <= 0.
        || Interval::point(total.hi).sub(Interval::point(total.lo))?.hi > absolute_error
    {
        return Err(unproved(
            "Boolean body positive volume or accuracy is unresolved",
        ));
    }

    for (outer_shell, &depth) in depths.iter().enumerate() {
        if depth % 2 == 0 {
            let inner_shells = parents
                .iter()
                .enumerate()
                .filter_map(|(i, &parent)| (parent == Some(outer_shell)).then_some(i))
                .collect();
            topology.bodies.push(Body {
                outer_shell,
                inner_shells,
            });
        }
    }
    Ok((MixedBrep::from_topology(topology)?, [total.lo, total.hi]))
}

/// Copy retained faces from one topology per operand into closed shells.
/// Rational indices are local to that topology. Exact SS curves can be shared
/// across operands when both faces reference the same root-edge definition.
pub(super) fn sew_selected_shells<'a>(
    sources: impl IntoIterator<Item = (&'a MixedTopology, impl IntoIterator<Item = FaceUse>)>,
    tolerance_mm: f64,
) -> Result<MixedBrep> {
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(unproved(
            "Boolean shell assembly needs positive finite tolerance",
        ));
    }
    let mut target = MixedTopology {
        vertices: Vec::new(),
        edges: Vec::new(),
        loops: Vec::new(),
        faces: Vec::new(),
        shells: Vec::new(),
        bodies: Vec::new(),
        tolerance_mm,
    };
    let mut face_uses = Vec::new();
    let mut root_index = RootSewIndex::default();
    for (source, selected_faces) in sources {
        // Original indices are shared within one operand only.
        let mut vertices = vec![None; source.vertices.len()];
        let mut edges = vec![None; source.edges.len()];
        for face_use in selected_faces {
            let mut face = source.faces[face_use.face].clone();
            for loop_id in std::iter::once(&mut face.outer).chain(&mut face.holes) {
                let mut wire = source.loops[*loop_id].clone();
                for coedge in &mut wire.coedges {
                    let old_id = coedge.edge;
                    let edge_id = if let Some(id) = edges[old_id] {
                        id
                    } else {
                        let old = &source.edges[old_id];
                        let id = match &old.curve {
                            MixedEdge::Root(root) => root_index.insert_edge(&mut target, root)?,
                            MixedEdge::Rational(_) => {
                                let mut edge = old.clone();
                                edge.vertices = old.vertices.map(|vertex| {
                                    *vertices[vertex].get_or_insert_with(|| {
                                        target.vertices.push(source.vertices[vertex].clone());
                                        target.vertices.len() - 1
                                    })
                                });
                                target.edges.push(edge);
                                target.edges.len() - 1
                            }
                        };
                        edges[old_id] = Some(id);
                        id
                    };
                    coedge.edge = edge_id;
                    if let MixedPcurve::Root(pcurve) = &mut coedge.pcurve {
                        pcurve.world_edge = edge_id;
                    }
                }
                *loop_id = target.loops.len();
                target.loops.push(wire);
            }
            face_uses.push(FaceUse {
                face: target.faces.len(),
                reversed: face_use.reversed,
            });
            target.faces.push(face);
        }
    }

    let mut incident = vec![Vec::new(); target.edges.len()];
    for face_use in &face_uses {
        let face = &target.faces[face_use.face];
        for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &target.loops[loop_id].coedges {
                if !target.edges[coedge.edge].degenerate {
                    incident[coedge.edge].push(face_use.face);
                }
            }
        }
    }
    let mut adjacent = vec![Vec::new(); face_uses.len()];
    for (edge, uses) in target.edges.iter().zip(incident) {
        if edge.degenerate {
            continue;
        }
        let [a, b] = uses.as_slice() else {
            return Err(unproved(
                "Boolean shell edge needs exactly two incident uses",
            ));
        };
        adjacent[*a].push(*b);
        adjacent[*b].push(*a);
    }
    let mut seen = vec![false; face_uses.len()];
    for first in 0..face_uses.len() {
        if seen[first] {
            continue;
        }
        let mut faces = Vec::new();
        let mut pending = vec![first];
        while let Some(face) = pending.pop() {
            if !std::mem::replace(&mut seen[face], true) {
                faces.push(face_uses[face].clone());
                pending.extend_from_slice(&adjacent[face]);
            }
        }
        target.shells.push(Shell {
            faces,
            closed: true,
        });
    }
    MixedBrep::from_topology(target)
}

/// All rational vertices preserve source identity, while the sewn SS edges
/// share exact root selectors across the two operands.
fn require_separate_shell_vertices(topology: &MixedTopology) -> Result<()> {
    let mut owner = vec![None; topology.vertices.len()];
    for (shell_id, shell) in topology.shells.iter().enumerate() {
        for use_ in &shell.faces {
            let face = &topology.faces[use_.face];
            for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for coedge in &topology.loops[loop_id].coedges {
                    for vertex in topology.edges[coedge.edge].vertices {
                        if owner[vertex]
                            .replace(shell_id)
                            .is_some_and(|other| other != shell_id)
                        {
                            return Err(unproved("Boolean shells touch at a shared vertex"));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn shell_is_inside(
    topology: &MixedTopology,
    source: usize,
    target: usize,
    directions: &[[f64; 3]],
    limits: super::RootMaterialBoxLimits,
) -> Result<bool> {
    let source = topology
        .shells
        .get(source)
        .ok_or_else(|| unproved("Containment source shell does not exist"))?;
    let target = topology
        .shells
        .get(target)
        .ok_or_else(|| unproved("Containment target shell does not exist"))?;
    let face = &topology.faces[source.faces[0].face];
    let coedge = &topology.loops[face.outer].coedges[0];
    let bounds = match &coedge.pcurve {
        MixedPcurve::Root(_) => {
            let edge = &topology.edges[coedge.edge];
            let super::MixedVertex::Root(point) = &topology.vertices[edge.vertices[0]].point else {
                return Err(unproved("Containment SS rim has no exact vertex"));
            };
            point.enclose(limits.point.geometry)?.world
        }
        MixedPcurve::Rational(pcurve) => {
            use nurbs_core::{
                curve_distance,
                interval_eval::{Interval, evaluate_surface_interval},
            };
            let range = coedge
                .trim()
                .map_or_else(|| pcurve.domain(), |trim| trim.pcurve_parameter);
            let parameter = range[0] * 0.5 + range[1] * 0.5;
            let span = (pcurve.degree..pcurve.control_points.len())
                .find(|&i| pcurve.knots[i] <= parameter && parameter < pcurve.knots[i + 1])
                .ok_or_else(|| unproved("Containment pcurve witness has no active span"))?;
            let uv = curve_distance::enclosure(pcurve, span, Interval::point(parameter))?;
            let surface = &face.surface;
            let u = Interval::new(
                uv[0].lo.max(surface.knots_u[surface.degree_u]),
                uv[0].hi.min(surface.knots_u[surface.control_points.len()]),
            )?;
            let v = Interval::new(
                uv[1].lo.max(surface.knots_v[surface.degree_v]),
                uv[1]
                    .hi
                    .min(surface.knots_v[surface.control_points[0].len()]),
            )?;
            if surface.periodic_u && (uv[0].lo < u.lo || uv[0].hi > u.hi)
                || surface.periodic_v && (uv[1].lo < v.lo || uv[1].hi > v.hi)
            {
                return Err(unproved("Containment witness crosses a periodic seam"));
            }
            let image: [Interval; 3] = evaluate_surface_interval(surface, u, v)?
                .try_into()
                .map_err(|_| unproved("Containment witness must be spatial"))?;
            image.map(|coordinate| [coordinate.lo, coordinate.hi])
        }
    };
    let (report, _, _) =
        super::volume::shell_material_box(topology, target, bounds, directions, limits, true)?;
    report
        .parity
        .ok_or_else(|| unproved("Shell containment witness parity unresolved"))
}
