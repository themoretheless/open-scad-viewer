use super::*;

impl<'a> DirectBuilder<'a> {
    pub(super) fn vertex(&mut self, id: usize) -> Result<usize> {
        if let Some(v) = self.vertex_map.get(&id) {
            return Ok(*v);
        }
        let (ty, a) = call(self.entities, id)?;
        if ty != "VERTEX_POINT" || a.len() != 2 {
            return Err(refuse("EDGE_CURVE vertex is not VERTEX_POINT"));
        }
        let p = point(
            self.entities,
            one_ref(&a[1], "VERTEX_POINT geometry")?,
            3,
            self.scale,
        )?;
        let index = self.vertices.len();
        self.vertices.push(Vertex {
            point: [p[0], p[1], p[2]],
        });
        self.vertex_map.insert(id, index);
        Ok(index)
    }
    pub(super) fn edge(
        &mut self,
        id: usize,
        surface_id: usize,
        reversed: bool,
        surface: &Surface,
    ) -> Result<(usize, Curve)> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "EDGE_CURVE" || a.len() != 5 {
            return Err(refuse("ORIENTED_EDGE target is not EDGE_CURVE"));
        }
        let same_sense = boolean(&a[4], "EDGE_CURVE same_sense")?;
        let va = self.vertex(one_ref(&a[1], "EDGE_CURVE start")?)?;
        let vb = self.vertex(one_ref(&a[2], "EDGE_CURVE end")?)?;
        let geometry = one_ref(&a[3], "EDGE_CURVE geometry")?;
        let (gty, ga) = call(self.entities, geometry)?;
        if gty != "SURFACE_CURVE" && gty != "SEAM_CURVE" {
            return Err(refuse("EDGE_CURVE must use SURFACE_CURVE or SEAM_CURVE"));
        }
        if ga.len() < 3 {
            return Err(refuse("SURFACE_CURVE argument count mismatch"));
        }
        let mut c3 = if self.ap242_composition {
            curve_v5(
                self.entities,
                one_ref(&ga[1], "SURFACE_CURVE 3D curve")?,
                3,
                self.scale,
                self.interior_point_selectors,
            )?
        } else {
            curve(
                self.entities,
                one_ref(&ga[1], "SURFACE_CURVE 3D curve")?,
                3,
                self.scale,
            )?
        };
        let mut matching = Vec::new();
        for p in list(&ga[2], "SURFACE_CURVE pcurves")? {
            let (_, pa) = call(self.entities, one_ref(p, "SURFACE_CURVE pcurve")?)?;
            if pa.len() != 3 {
                return Err(refuse("Malformed PCURVE"));
            }
            if one_ref(&pa[1], "PCURVE surface")? == surface_id {
                let mut curve_id = one_ref(&pa[2], "PCURVE reference_to_curve")?;
                if self.ap242_composition {
                    let (reference_type, reference_args) = call(self.entities, curve_id)?;
                    if reference_type != "DEFINITIONAL_REPRESENTATION" || reference_args.len() != 3
                    {
                        return Err(refuse(
                            "AP242 PCURVE reference_to_curve must be a DEFINITIONAL_REPRESENTATION",
                        ));
                    }
                    let items = list(&reference_args[1], "DEFINITIONAL_REPRESENTATION items")?;
                    if items.len() != 1 {
                        return Err(refuse(
                            "PCURVE definitional representation must contain one curve",
                        ));
                    }
                    curve_id = one_ref(&items[0], "DEFINITIONAL_REPRESENTATION curve")?;
                }
                matching.push(if self.ap242_composition {
                    curve_v5(
                        self.entities,
                        curve_id,
                        2,
                        1.,
                        self.interior_point_selectors,
                    )?
                } else {
                    curve(self.entities, curve_id, 2, 1.)?
                });
            }
        }
        if matching.is_empty() {
            return Err(refuse("No PCURVE corresponds to ADVANCED_FACE surface"));
        }
        let mut pc = if gty == "SEAM_CURVE" || matching.len() > 1 {
            let use_index = self.seam_use.entry((id, surface_id)).or_insert(0);
            if *use_index >= matching.len() {
                return Err(refuse("Surface curve has fewer pcurves than oriented uses"));
            }
            let selected = matching[*use_index].clone();
            *use_index += 1;
            selected
        } else {
            matching.pop().unwrap()
        };
        if !same_sense {
            c3 = c3.reverse()?;
            pc = pc.reverse()?;
        }
        let curve_domain = c3.domain();
        let curve_start = c3.evaluate(curve_domain[0])?.point;
        let curve_end = c3.evaluate(curve_domain[1])?.point;
        let endpoint_error = |point: &[f64], vertex: &Vertex| {
            point
                .iter()
                .zip(vertex.point)
                .map(|(left, right)| (left - right) * (left - right))
                .sum::<f64>()
                .sqrt()
        };
        if endpoint_error(&curve_start, &self.vertices[va]) > 1e-7
            || endpoint_error(&curve_end, &self.vertices[vb]) > 1e-7
        {
            return Err(refuse(
                "EDGE_CURVE same_sense does not orient geometry from edge_start to edge_end",
            ));
        }
        // A closed edge may share its endpoint vertex while tracing a full
        // nonconstant curve. Only a constant control hull is a pole edge.
        let pole = self.vertices[va].point;
        let degenerate = va == vb && c3.control_points.iter().all(|point| {
            point.len() == 3 && point.iter().zip(pole).all(|(a, b)| *a == b)
        });
        verify_correspondence(
            &c3,
            &pc,
            surface,
            reversed,
            self.whole_domain_proofs,
            degenerate
                && self.interior_point_selectors
                && certify_periodic_surface(surface)?.is_none(),
        )
        .map_err(|e| {
            refuse(format!(
                "EDGE_CURVE #{id} on surface #{surface_id}: {}",
                e.message
            ))
        })?;
        if degenerate && !self.allow_degenerate {
            return Err(refuse(
                "Pole/degenerate edges are typed-refused before STEP /6",
            ));
        }
        if degenerate {
            let pole = self.vertices[va].point;
            if c3.control_points.iter().any(|point| {
                point.len() != 3 || point.iter().zip(pole).any(|(a, b)| (*a - b).abs() > 1e-9)
            }) {
                return Err(refuse(
                    "Degenerate EDGE_CURVE geometry is not identically its pole vertex",
                ));
            }
        }
        let index = if let Some(index) = self.edge_map.get(&id) {
            let prior = &self.edges[*index];
            if prior.vertices != [va, vb] {
                return Err(refuse("Shared EDGE_CURVE has inconsistent vertices"));
            }
            *index
        } else {
            let index = self.edges.len();
            self.edges.push(Edge {
                degenerate,
                vertices: [va, vb],
                curve: c3,
            });
            self.edge_map.insert(id, index);
            index
        };
        Ok((index, pc))
    }
    pub(super) fn loop_(&mut self, id: usize, surface_id: usize, surface: &Surface) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "EDGE_LOOP" || a.len() != 2 {
            return Err(refuse("FACE_BOUND target is not EDGE_LOOP"));
        }
        let mut coedges = Vec::new();
        for oriented in list(&a[1], "EDGE_LOOP members")? {
            let oid = one_ref(oriented, "EDGE_LOOP member")?;
            let (oty, oa) = call(self.entities, oid)?;
            if oty != "ORIENTED_EDGE" || oa.len() != 5 {
                return Err(refuse("EDGE_LOOP member is not ORIENTED_EDGE"));
            }
            let reversed = !boolean(&oa[4], "ORIENTED_EDGE orientation")?;
            let (edge, pcurve) = self.edge(
                one_ref(&oa[3], "ORIENTED_EDGE edge")?,
                surface_id,
                reversed,
                surface,
            )?;
            coedges.push(Coedge::new(
                edge,
                reversed,
                pcurve,
            ));
        }
        if coedges.is_empty() {
            return Err(refuse("EDGE_LOOP cannot be empty"));
        }
        let index = self.loops.len();
        self.loops.push(Loop { coedges });
        self.loop_entity.push(id);
        Ok(index)
    }
    pub(super) fn face(&mut self, id: usize) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "ADVANCED_FACE" || a.len() != 4 {
            return Err(refuse("CLOSED_SHELL member is not ADVANCED_FACE"));
        }
        let sid = one_ref(&a[2], "ADVANCED_FACE surface")?;
        let mut surface = if self.ap242_composition {
            surface_v5(self.entities, sid, self.scale)?
        } else if self.analytic_surfaces {
            surface_v4(self.entities, sid, self.scale)?
        } else {
            surface(self.entities, sid, self.scale)?
        };
        let mut outer = None;
        let mut holes = Vec::new();
        for bound in list(&a[1], "ADVANCED_FACE bounds")? {
            let bid = one_ref(bound, "ADVANCED_FACE bound")?;
            let (bty, ba) = call(self.entities, bid)?;
            if !matches!(bty, "FACE_OUTER_BOUND" | "FACE_BOUND") || ba.len() != 3 {
                return Err(refuse("ADVANCED_FACE bound type is unsupported"));
            }
            let loop_index = self.loop_(one_ref(&ba[1], "FACE_BOUND loop")?, sid, &surface)?;
            if !boolean(&ba[2], "FACE_BOUND orientation")? {
                for c in &mut self.loops[loop_index].coedges {
                    c.reversed = !c.reversed;
                    c.pcurve = c.pcurve.reverse()?;
                }
                self.loops[loop_index].coedges.reverse();
            }
            if bty == "FACE_OUTER_BOUND" {
                if outer.replace(loop_index).is_some() {
                    return Err(refuse("ADVANCED_FACE has multiple outer bounds"));
                }
            } else {
                holes.push(loop_index);
            }
        }
        let same_sense = boolean(&a[3], "ADVANCED_FACE same_sense")?;
        if !same_sense && self.ap242_composition {
            let sum = surface.knots_u[surface.degree_u]
                + surface.knots_u[surface.knots_u.len() - surface.degree_u - 1];
            surface.control_points.reverse();
            surface.weights.reverse();
            surface.knots_u.reverse();
            for knot in &mut surface.knots_u {
                *knot = sum - *knot
            }
            for loop_index in std::iter::once(outer.unwrap()).chain(holes.iter().copied()) {
                for coedge in &mut self.loops[loop_index].coedges {
                    for point in &mut coedge.pcurve.control_points {
                        point[0] = sum - point[0]
                    }
                }
            }
        }
        let index = self.faces.len();
        self.faces.push(Face {
            surface,
            outer: outer.ok_or_else(|| refuse("ADVANCED_FACE has no outer bound"))?,
            holes,
        });
        self.face_entity.push(id);
        Ok(index)
    }
    pub(super) fn shell(&mut self, id: usize) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        let closed = match ty {
            "CLOSED_SHELL" => true,
            "OPEN_SHELL" if self.allow_open_shells => false,
            _ => {
                return Err(refuse(
                    "Shell is outside the admitted CLOSED_SHELL/OPEN_SHELL subset",
                ));
            }
        };
        if a.len() != 2 {
            return Err(refuse("Shell argument count mismatch"));
        }
        let mut faces = Vec::new();
        for f in list(&a[1], "shell faces")? {
            let fid = one_ref(f, "shell face")?;
            let (_, fa) = call(self.entities, fid)?;
            faces.push(FaceUse {
                face: self.face(fid)?,
                reversed: !boolean(&fa[3], "ADVANCED_FACE sense")?,
            });
        }
        let index = self.shells.len();
        self.shells.push(Shell { faces, closed });
        self.shell_entity.push(id);
        Ok(index)
    }
    pub(super) fn body(&mut self, id: usize) -> Result<()> {
        let (ty, a) = call(self.entities, id)?;
        let (outer, inners) = match ty {
            "MANIFOLD_SOLID_BREP" if a.len() == 2 => (one_ref(&a[1], "solid shell")?, Vec::new()),
            "BREP_WITH_VOIDS" if a.len() == 3 => {
                let mut voids = Vec::new();
                for value in list(&a[2], "void shells")? {
                    let void_id = one_ref(value, "void shell")?;
                    if self.ap242_composition {
                        let (vty, va) = call(self.entities, void_id)?;
                        if vty != "ORIENTED_CLOSED_SHELL" || va.len() != 4 {
                            return Err(refuse(
                                "AP242 BREP_WITH_VOIDS requires ORIENTED_CLOSED_SHELL voids",
                            ));
                        }
                        let orientation = boolean(&va[3], "ORIENTED_CLOSED_SHELL orientation")?;
                        if orientation && (!self.allow_degenerate || self.whole_domain_proofs) {
                            return Err(refuse(
                                "Void ORIENTED_CLOSED_SHELL must oppose its CLOSED_SHELL",
                            ));
                        }
                        voids.push(one_ref(&va[2], "ORIENTED_CLOSED_SHELL element")?);
                    } else {
                        voids.push(void_id);
                    }
                }
                (one_ref(&a[1], "outer shell")?, voids)
            }
            _ => {
                return Err(refuse(
                    "Selected representation contains unsupported body root",
                ));
            }
        };
        let outer_shell = self.shell(outer)?;
        let inner_shells = inners
            .into_iter()
            .map(|s| self.shell(s))
            .collect::<Result<Vec<_>>>()?;
        self.bodies.push(Body {
            outer_shell,
            inner_shells,
        });
        self.body_entity.push(id);
        Ok(())
    }
    pub(super) fn root(&mut self, id: usize) -> Result<()> {
        let (ty, args) = call(self.entities, id)?;
        if ty == "SHELL_BASED_SURFACE_MODEL" && self.allow_open_shells {
            if args.len() != 2 {
                return Err(refuse("SHELL_BASED_SURFACE_MODEL argument count mismatch"));
            }
            for shell in list(&args[1], "SHELL_BASED_SURFACE_MODEL boundaries")? {
                let shell_id = one_ref(shell, "surface-model shell")?;
                if call(self.entities, shell_id)?.0 != "OPEN_SHELL" {
                    return Err(refuse(
                        "SHELL_BASED_SURFACE_MODEL requires OPEN_SHELL boundaries",
                    ));
                }
                self.shell(shell_id)?;
            }
            Ok(())
        } else {
            self.body(id)
        }
    }
}

pub(super) fn metadata_entity_map(builder: &DirectBuilder<'_>) -> Vec<(TopoKind, usize, usize)> {
    let mut out = Vec::new();
    out.extend(
        builder
            .vertex_map
            .iter()
            .map(|(entity, index)| (TopoKind::Vertex, *index, *entity)),
    );
    out.extend(
        builder
            .edge_map
            .iter()
            .map(|(entity, index)| (TopoKind::Edge, *index, *entity)),
    );
    out.extend(
        builder
            .loop_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Loop, i, *e)),
    );
    out.extend(
        builder
            .face_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Face, i, *e)),
    );
    out.extend(
        builder
            .shell_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Shell, i, *e)),
    );
    out.extend(
        builder
            .body_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Body, i, *e)),
    );
    out
}

pub(super) fn build_direct_roots(
    entities: &BTreeMap<usize, Entity>,
    roots: &[usize],
    scale: f64,
    options: DirectBuildOptions,
) -> Result<(Model, TopologyMapping)> {
    let DirectBuildOptions {
        analytic_surfaces,
        ap242_composition,
        allow_degenerate,
        whole_domain_proofs,
        interior_point_selectors,
        allow_open_shells,
    } = options;
    let mut builder = DirectBuilder {
        entities,
        scale,
        analytic_surfaces,
        ap242_composition,
        allow_degenerate,
        whole_domain_proofs,
        interior_point_selectors,
        allow_open_shells,
        vertices: vec![],
        vertex_map: BTreeMap::new(),
        edges: vec![],
        edge_map: BTreeMap::new(),
        seam_use: BTreeMap::new(),
        loops: vec![],
        loop_entity: vec![],
        faces: vec![],
        face_entity: vec![],
        shells: vec![],
        shell_entity: vec![],
        bodies: vec![],
        body_entity: vec![],
    };
    for root in roots {
        builder.root(*root)?
    }
    let map = metadata_entity_map(&builder);
    let topology = brep_topology::Model {
        vertices: builder.vertices,
        edges: builder.edges,
        loops: builder.loops,
        faces: builder.faces,
        shells: builder.shells,
        bodies: builder.bodies,
        tolerance_mm: 1e-7,
    };
    let mut model = Model(topology, TopologyIds::default());
    model.rebuild_topology_ids();
    model.validate()?;
    Ok((model, map))
}

pub(super) fn append_direct_model(
    target: &mut Model,
    source: Model,
    source_map: &[(TopoKind, usize, usize)],
    target_map: &mut Vec<(TopoKind, usize, usize)>,
) {
    let Model(mut topology, _) = source;
    let (vo, eo, lo, fo, so, bo) = (
        target.vertices.len(),
        target.edges.len(),
        target.loops.len(),
        target.faces.len(),
        target.shells.len(),
        target.bodies.len(),
    );
    for edge in &mut topology.edges {
        edge.vertices = [edge.vertices[0] + vo, edge.vertices[1] + vo]
    }
    for loop_ in &mut topology.loops {
        for coedge in &mut loop_.coedges {
            coedge.edge += eo
        }
    }
    for face in &mut topology.faces {
        face.outer += lo;
        for hole in &mut face.holes {
            *hole += lo
        }
    }
    for shell in &mut topology.shells {
        for face in &mut shell.faces {
            face.face += fo
        }
    }
    for body in &mut topology.bodies {
        body.outer_shell += so;
        for shell in &mut body.inner_shells {
            *shell += so
        }
    }
    target.0.vertices.extend(topology.vertices);
    target.0.edges.extend(topology.edges);
    target.0.loops.extend(topology.loops);
    target.0.faces.extend(topology.faces);
    target.0.shells.extend(topology.shells);
    target.0.bodies.extend(topology.bodies);
    for &(kind, index, entity) in source_map {
        let offset = match kind {
            TopoKind::Vertex => vo,
            TopoKind::Edge => eo,
            TopoKind::Loop => lo,
            TopoKind::Face => fo,
            TopoKind::Shell => so,
            TopoKind::Body => bo,
            TopoKind::ControlPoint => 0,
        };
        target_map.push((kind, index + offset, entity));
    }
}

/// Compose already validated document occurrences into one authoritative
/// bounded model and assign occurrence-local topology identities.
pub(super) fn compose_step_occurrences(models: &[Model], capability: &str) -> Result<Model> {
    if models.is_empty() {
        return Err(refuse(
            "STEP /7 composition requires at least one occurrence",
        ));
    }
    if models.len() > MAX_OCCURRENCES {
        return Err(refuse("STEP /7 composed occurrence count exceeds 256"));
    }
    let mut aggregate = Model::empty(1e-7)?;
    let mut ignored = Vec::new();
    for model in models {
        model.validate()?;
        append_direct_model(&mut aggregate, model.clone(), &[], &mut ignored);
    }
    aggregate.rebuild_topology_ids();
    let occurrence_ids = |kind: TopoKind, ids: &[TopoId]| {
        ids.iter()
            .enumerate()
            .map(|(index, id)| {
                TopoId::derive(
                    kind,
                    capability,
                    &format!("occurrence-entity-{index}"),
                    kind.as_str(),
                    id.to_string().as_bytes(),
                )
            })
            .collect::<Vec<_>>()
    };
    aggregate.1.vertices = occurrence_ids(TopoKind::Vertex, &aggregate.1.vertices);
    aggregate.1.edges = occurrence_ids(TopoKind::Edge, &aggregate.1.edges);
    aggregate.1.loops = occurrence_ids(TopoKind::Loop, &aggregate.1.loops);
    aggregate.1.faces = occurrence_ids(TopoKind::Face, &aggregate.1.faces);
    aggregate.1.shells = occurrence_ids(TopoKind::Shell, &aggregate.1.shells);
    aggregate.1.bodies = occurrence_ids(TopoKind::Body, &aggregate.1.bodies);
    aggregate.1.lineage.clear();
    aggregate.1.change_set = brep_topology::ChangeSet::default();
    for (kind, ids) in [
        (TopoKind::Vertex, &aggregate.1.vertices),
        (TopoKind::Edge, &aggregate.1.edges),
        (TopoKind::Loop, &aggregate.1.loops),
        (TopoKind::Face, &aggregate.1.faces),
        (TopoKind::Shell, &aggregate.1.shells),
        (TopoKind::Body, &aggregate.1.bodies),
    ] {
        for id in ids {
            aggregate.1.change_set.nodes.insert(*id, kind);
        }
    }
    aggregate.validate()?;
    Ok(aggregate)
}
pub fn compose_step_v7_occurrences(models: &[Model]) -> Result<Model> {
    compose_step_occurrences(models, STEP_INTERCHANGE_V7_CAPABILITY)
}
